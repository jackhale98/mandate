//! Town character (DEC-077 §1): what a town is like, derived from the
//! simulation each month and stored: its defining industry, a live local
//! crisis (from the crisis registry, with hysteresis and a severity trend),
//! a power broker, a named rival, and its local priorities (its top
//! concerns). Nothing here is authored per run.

use super::CharacterId;
use crate::player::PlayerState;
use crate::politics::elections::{Filing, Seat};
use crate::politics::{PoliticianId, PoliticsState, Role};
use crate::scheduler::Reads;
use crate::world::World;
use mandate_content::{ContentKey, SectorDriver};
use mandate_core::SimDate;
use serde::{Deserialize, Serialize};

/// A crisis a town is living through.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActiveCrisis {
    /// Crisis kind content id.
    pub kind: ContentKey,
    /// 0..1, this month.
    pub severity: f32,
    pub since: SimDate,
    /// Monthly severity, oldest first (the trend).
    pub history: Vec<f32>,
}

impl ActiveCrisis {
    /// Severity change over the kept history (positive = worsening).
    pub fn trend(&self) -> f32 {
        match (self.history.first(), self.history.last()) {
            (Some(a), Some(b)) => b - a,
            _ => 0.0,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TownCharacter {
    /// The sector that defines the town (content id): the largest relative
    /// to its national share, public employment aside.
    pub industry: Option<ContentKey>,
    /// Its share of the town's jobs.
    pub industry_share: f32,
    /// Crises in progress, in registry order.
    pub crises: Vec<ActiveCrisis>,
    pub broker: Option<CharacterId>,
    pub rival: Option<CharacterId>,
    /// The town's top concerns (content ids), strongest first.
    pub priorities: Vec<ContentKey>,
}

impl TownCharacter {
    /// The live crisis: the most severe in progress (ties: registry order).
    pub fn crisis(&self) -> Option<&ActiveCrisis> {
        self.crises
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.severity.total_cmp(&b.1.severity).then(b.0.cmp(&a.0)))
            .map(|(_, c)| c)
    }
}

/// Every town's character this month, carrying crisis state forward from
/// `prev` (hysteresis and the trend).
pub fn compute_all_from(
    pol: &PoliticsState,
    r: &Reads<'_>,
    prev: &[TownCharacter],
) -> Vec<TownCharacter> {
    let world = r.world;
    let content = r.content;
    let econ = r.economy();
    let n = world.municipalities.len();
    // National jobs by sector, for distinctiveness.
    let mut national = vec![0.0f64; content.sectors.len()];
    for mu in &econ.munis {
        for (k, s) in mu.sectors.iter().enumerate() {
            if let Some(x) = national.get_mut(k) {
                *x += s.jobs;
            }
        }
    }
    let total_national: f64 = national.iter().sum::<f64>().max(1.0);
    // Who lives and works in each town (one pass).
    let mut officeholders: Vec<Vec<PoliticianId>> = vec![Vec::new(); n];
    for p in &pol.politicians {
        if p.in_office()
            && !p.is_player
            && let Some(v) = officeholders.get_mut(p.municipality() as usize)
        {
            v.push(p.id);
        }
    }
    let mut leaders: Vec<Vec<CharacterId>> = vec![Vec::new(); n];
    for c in &pol.characters.people {
        let Some(nc) = c.named.as_deref() else {
            continue;
        };
        if !content.occupation(&nc.occupation).is_some_and(|o| o.leader) {
            continue;
        }
        let m = nc
            .place
            .and_then(|p| world.places.get(p.index()))
            .map(|p| p.municipality.0)
            .or_else(|| {
                world
                    .neighborhoods
                    .get(nc.home as usize)
                    .map(|h| h.municipality.0)
            });
        if let Some(v) = m.and_then(|m| leaders.get_mut(m as usize)) {
            v.push(c.id);
        }
    }
    let k = &content.characters.town;
    (0..n)
        .map(|m| {
            let old = prev.get(m);
            // Defining industry: share × location quotient.
            let mut industry: Option<(usize, f64, f64)> = None;
            if let Some(mu) = econ.munis.get(m) {
                let total = mu.total_jobs().max(1.0);
                for (i, (s, def)) in mu.sectors.iter().zip(&content.sectors).enumerate() {
                    if def.driver == SectorDriver::Public || s.jobs <= 0.0 {
                        continue;
                    }
                    let share = s.jobs / total;
                    let nat = national.get(i).copied().unwrap_or(0.0) / total_national;
                    let score = share * share / nat.max(1e-6);
                    if industry.is_none_or(|b| score > b.1) {
                        industry = Some((i, score, share));
                    }
                }
            }
            // Crises, with hysteresis.
            let mut crises = Vec::new();
            for def in &content.crises {
                let v = crate::concerns::indicator(r, m, def.indicator);
                let was = old.and_then(|t| t.crises.iter().find(|c| c.kind == def.id));
                let on = if was.is_some() {
                    v >= def.end
                } else {
                    v >= def.begin
                };
                if !on {
                    continue;
                }
                let sev = def.severity(v) as f32;
                let mut history = was.map(|c| c.history.clone()).unwrap_or_default();
                history.push(sev);
                let keep = k.trend_months.max(1) as usize;
                if history.len() > keep {
                    history.drain(..history.len() - keep);
                }
                crises.push(ActiveCrisis {
                    kind: def.id.clone(),
                    severity: sev,
                    since: was.map(|c| c.since).unwrap_or(r.date),
                    history,
                });
            }
            let mut t = TownCharacter {
                industry: industry
                    .and_then(|(i, _, _)| content.sectors.get(i))
                    .map(|s| s.id.clone()),
                industry_share: industry.map(|(_, _, s)| s as f32).unwrap_or(0.0),
                crises,
                broker: None,
                rival: rival(pol, world, r.player, m),
                priorities: crate::concerns::top_concerns(r.population(), m)
                    .into_iter()
                    .filter(|(_, l)| *l > 0.0)
                    .take(3)
                    .filter_map(|(i, _)| content.concerns.get(i).map(|c| c.id.clone()))
                    .collect(),
            };
            t.broker = broker(pol, r, &t, &officeholders[m], &leaders[m]);
            t
        })
        .collect()
}

pub fn compute_all(pol: &PoliticsState, r: &Reads<'_>) -> Vec<TownCharacter> {
    compute_all_from(pol, r, &pol.characters.towns)
}

/// The power broker: the organisation leader with the most weight and stake
/// in the defining industry or the live crisis, or else the most senior
/// officeholder (the mayor counts double).
fn broker(
    pol: &PoliticsState,
    r: &Reads<'_>,
    t: &TownCharacter,
    officeholders: &[PoliticianId],
    leaders: &[CharacterId],
) -> Option<CharacterId> {
    let content = r.content;
    let k = &content.characters.town;
    let driver = t
        .industry
        .as_deref()
        .and_then(|s| content.sector(s))
        .map(|s| s.driver);
    let crisis = t.crisis().map(|c| c.kind.as_str());
    let mut best: Option<(CharacterId, f64)> = None;
    let mut consider = |id: CharacterId, score: f64| {
        if best.is_none_or(|b| score > b.1 || (score == b.1 && id < b.0)) {
            best = Some((id, score));
        }
    };
    for &id in leaders {
        let Some(o) = pol
            .characters
            .get(id)
            .and_then(|c| c.named.as_deref())
            .and_then(|n| content.occupation(&n.occupation))
        else {
            continue;
        };
        let stake = driver.is_some_and(|d| o.stake.drivers.contains(&d)) as u8 as f64
            + crisis.is_some_and(|c| o.stake.crises.iter().any(|x| x == c)) as u8 as f64;
        if stake > 0.0 {
            consider(id, o.influence + k.broker_stake * stake);
        }
    }
    for &pid in officeholders {
        let Some(p) = pol.get(pid) else { continue };
        let mayor = matches!(p.role, Role::Mayor { .. });
        let score = k.broker_seniority * p.years_in_office as f64 * if mayor { 2.0 } else { 1.0 }
            + if mayor { 0.3 } else { 0.0 };
        consider(p.character, score);
    }
    best.map(|b| b.0)
}

/// The seat a town's leading figure will want next: the player's (by their
/// filing) in their own town; elsewhere, the mayoralty.
fn wanted_seat(
    pol: &PoliticsState,
    player: Option<&PlayerState>,
    m: usize,
) -> Option<(Option<PoliticianId>, Seat)> {
    let m32 = m as u32;
    if let Some(pl) = player
        && pl.municipality.index() == m
    {
        let me = pol.get(pl.politician)?;
        let seat = match (pl.filing, me.role) {
            (Some(Filing::Retire), _) => return None,
            (Some(Filing::Mayor), _) | (None, Role::Mayor { .. }) => {
                Seat::Mayor { municipality: m32 }
            }
            _ => Seat::Council {
                municipality: m32,
                ward: pl.ward,
            },
        };
        return Some((Some(me.id), seat));
    }
    let mayor = pol.councils.get(m).and_then(|c| c.mayor);
    Some((mayor, Seat::Mayor { municipality: m32 }))
}

/// The named rival (DEC-077 §1): whoever competes with the town's leading
/// figure (the player, in their town) for the next seat they will want — the
/// strongest opponent in a running race, else the seat's holder, else the
/// runner-up there last time, else the most ambitious officeholder of
/// another party.
pub fn rival(
    pol: &PoliticsState,
    world: &World,
    player: Option<&PlayerState>,
    m: usize,
) -> Option<CharacterId> {
    let _ = world;
    let (me, seat) = wanted_seat(pol, player, m)?;
    let char_of = |p: PoliticianId| pol.get(p).map(|x| x.character);
    let not_me = |p: &PoliticianId| Some(*p) != me;
    if let Some((ri, race)) = pol
        .elections
        .races
        .iter()
        .enumerate()
        .find(|(_, r)| r.seat == seat)
    {
        let polls = crate::politics::elections::poll_average(&pol.elections, ri);
        let best = race
            .candidates
            .iter()
            .enumerate()
            .filter(|(_, c)| not_me(&c.politician))
            .max_by(|a, b| {
                let s = |i: usize| {
                    polls
                        .as_ref()
                        .and_then(|p| p.get(i))
                        .copied()
                        .unwrap_or(0.0)
                };
                s(a.0).total_cmp(&s(b.0)).then(b.0.cmp(&a.0))
            })
            .map(|(_, c)| c.politician);
        if let Some(p) = best {
            return char_of(p);
        }
    }
    let holder = match seat {
        Seat::Mayor { .. } => pol.councils.get(m).and_then(|c| c.mayor),
        Seat::Council { ward, .. } => pol
            .politicians
            .iter()
            .find(|p| matches!(p.role, Role::Councillor { ward: w, .. } if w == ward))
            .map(|p| p.id),
    };
    if let Some(h) = holder.filter(not_me) {
        return char_of(h);
    }
    if let Some(res) = pol.elections.latest_result(seat) {
        let mut cands: Vec<_> = res
            .candidates
            .iter()
            .filter(|c| not_me(&c.politician))
            .collect();
        cands.sort_by(|a, b| b.share.total_cmp(&a.share));
        if let Some(c) = cands.first() {
            return char_of(c.politician);
        }
    }
    let my_party = me.and_then(|p| pol.get(p)).and_then(|p| p.party);
    pol.politicians
        .iter()
        .filter(|p| p.in_office() && p.municipality() as usize == m && not_me(&p.id))
        .filter(|p| my_party.is_none() || p.party != my_party)
        .max_by(|a, b| a.ambition.total_cmp(&b.ambition).then(b.id.cmp(&a.id)))
        .map(|p| p.character)
}
