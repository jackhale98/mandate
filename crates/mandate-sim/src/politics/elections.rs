//! Elections (DEC-028), polls (DEC-029), and their research-informed
//! refinements (DEC-071).
//!
//! The pipeline per local election:
//! 1. **Open** (campaign start): candidacy and party nomination per seat;
//!    hidden race shocks and a national swing are drawn from keyed streams.
//! 2. **Campaign** (weekly): abstract NPC campaigning; persuasion decays.
//!    Pollsters publish noisy estimates (never the truth).
//! 3. **Election day**: per-cohort logit vote with separate turnout; plurality
//!    tabulation; seats change hands; a factor decomposition explains the result.

use super::parties::PartyId;
use super::{Council, PoliticianId, PoliticsState, Role, mean_attitudes, new_politician};
use crate::events::EventKind;
use crate::population::{ATTITUDES, Cohort, PopulationState};
use crate::scheduler::{Outs, Reads};
use crate::world::{ConstituencyId, MunicipalityId, Tier, World};
use mandate_content::{ElectionCalendarDef, ElectionTiming, PoliticsParams, TierKey};
use mandate_core::math::{clamp, exp};
use mandate_core::{SimDate, SimRng, Stream};
use serde::{Deserialize, Serialize};

/// Results are kept for this many past elections per seat (save size).
const KEEP_ELECTIONS: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Seat {
    Council {
        municipality: u32,
        ward: ConstituencyId,
    },
    Mayor {
        municipality: u32,
    },
}

impl Seat {
    pub fn municipality(self) -> u32 {
        match self {
            Seat::Council { municipality, .. } | Seat::Mayor { municipality } => municipality,
        }
    }
    pub fn is_executive(self) -> bool {
        matches!(self, Seat::Mayor { .. })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Candidate {
    pub politician: PoliticianId,
    pub party: Option<PartyId>,
    /// Per race neighbourhood (aligned with `Race::hoods`), 0..1.
    pub awareness: Vec<f32>,
    /// Awareness when the campaign opened (for the "campaign" factor).
    pub awareness_start: Vec<f32>,
    /// Decaying persuasion from contact (utility units).
    pub persuasion: Vec<f32>,
    /// Turnout mobilisation from contact (0..1).
    pub mobilisation: Vec<f32>,
    /// Recent contact count (diminishing returns; decays).
    pub contacts: Vec<f32>,
    pub spent: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Race {
    pub seat: Seat,
    pub opened: SimDate,
    pub election_day: SimDate,
    pub hoods: Vec<u32>,
    pub incumbent: Option<PoliticianId>,
    pub candidates: Vec<Candidate>,
    pub partisan: bool,
    /// Hidden per-candidate shock (utility units), revealed on election day.
    pub shock: Vec<f32>,
    /// Hidden standard-normal draw for the race's shared polling error.
    pub poll_bias_z: Vec<f32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CandidateResult {
    pub politician: PoliticianId,
    pub party: Option<PartyId>,
    pub votes: u64,
    pub share: f32,
    /// Final poll average share (before the result), if polled.
    pub final_poll: Option<f32>,
}

/// One factor's contribution to the winner's margin over the runner-up (share points 0..1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FactorShare {
    pub factor: String,
    pub margin: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RaceResult {
    pub seat: Seat,
    pub date: SimDate,
    pub candidates: Vec<CandidateResult>,
    pub winner: PoliticianId,
    pub incumbent: Option<PoliticianId>,
    pub turnout: f32,
    pub decomposition: Vec<FactorShare>,
    /// Winner's share per neighbourhood (hood id, share).
    pub hood_winner_share: Vec<(u32, f32)>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Poll {
    /// Index into `ElectionState::races`.
    pub race: u32,
    pub date: SimDate,
    pub pollster: u8,
    /// Share per candidate (aligned with the race's candidates), among decided voters.
    pub shares: Vec<f32>,
    pub undecided: f32,
    pub sample: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ElectionState {
    /// Races of the current campaign (empty between campaigns).
    pub races: Vec<Race>,
    pub polls: Vec<Poll>,
    /// National party swing for the current cycle (utility units, hidden).
    pub national_swing: Vec<f32>,
    /// Per pollster, per party house effect (share units).
    pub house_effects: Vec<Vec<f32>>,
    pub results: Vec<RaceResult>,
    /// Turnout per neighbourhood at the last election (population reads it for habit).
    pub last_hood_turnout: Vec<(u32, f32)>,
    pub last_election: Option<SimDate>,
    /// Campaign window of the current/next local election.
    pub campaign_open: Option<SimDate>,
    pub next_day: Option<SimDate>,
}

impl ElectionState {
    pub fn races_in(&self, municipality: u32) -> impl Iterator<Item = (usize, &Race)> {
        self.races
            .iter()
            .enumerate()
            .filter(move |(_, r)| r.seat.municipality() == municipality)
    }

    pub fn latest_result(&self, seat: Seat) -> Option<&RaceResult> {
        self.results.iter().rev().find(|r| r.seat == seat)
    }
}

// --- Calendar ------------------------------------------------------------------

/// The local tier's election calendar and term, if elections are configured.
pub fn local_calendar<'a>(r: &Reads<'a>) -> Option<(&'a ElectionCalendarDef, u32)> {
    let c = r.content.constitutions.get(&r.meta.constitution)?;
    let t = c.tier(TierKey::Local)?;
    let term = t
        .bodies
        .first()
        .map(|b| b.term_years as u32)
        .unwrap_or(4)
        .max(1);
    t.elections.as_ref().map(|e| (e, term))
}

/// First election day strictly after `after`.
pub fn next_election_day(cal: &ElectionCalendarDef, term: u32, after: SimDate) -> SimDate {
    let mut y = after.calendar().year;
    loop {
        if y % term == cal.cycle_offset as u32 % term {
            let d = SimDate::from_ymd(y, cal.month as u32, cal.day as u32);
            if d > after {
                return d;
            }
        }
        y += 1;
    }
}

// --- Daily system --------------------------------------------------------------

/// Politics domain, Elections phase, daily: open campaigns, campaign weekly,
/// publish polls, and hold elections on their day.
pub fn daily(st: &mut PoliticsState, r: &Reads<'_>, out: &mut Outs<'_>) {
    let Some((cal, term)) = local_calendar(r) else {
        return;
    };
    let params = &r.content.politics;
    let today = r.date;
    // Schedule.
    if st.elections.next_day.is_none_or(|d| d < today) {
        let day = next_election_day(cal, term, SimDate(today.0.saturating_sub(1)));
        st.elections.next_day = Some(day);
        st.elections.campaign_open = Some(day.sub_days(cal.campaign_weeks as u32 * 7));
    }
    let (Some(day), Some(open)) = (st.elections.next_day, st.elections.campaign_open) else {
        return;
    };
    // Opens on its day, or late (a game starting or loading inside the window).
    if today >= open && today < day && st.elections.races.is_empty() {
        open_campaign(st, r, out, cal, day);
    }
    if !st.elections.races.is_empty() && today < day {
        let since = st
            .elections
            .races
            .first()
            .map(|r| r.opened)
            .unwrap_or(open)
            .days_until(today);
        if since.is_multiple_of(7) {
            campaign_week(st, r, out);
        }
        if since.is_multiple_of(params.polls.every_days.max(1) as u32) && since > 0 {
            publish_polls(st, r, out, day);
        }
    }
    if today == day && !st.elections.races.is_empty() {
        hold(st, r, out, cal);
    }
}

trait DateExt {
    fn sub_days(self, d: u32) -> SimDate;
}
impl DateExt for SimDate {
    fn sub_days(self, d: u32) -> SimDate {
        SimDate(self.0.saturating_sub(d))
    }
}

// --- Opening: candidacy and nomination ------------------------------------------

fn names(r: &Reads<'_>) -> (Vec<String>, Vec<String>) {
    let g = r
        .content
        .name_pools
        .get("names.given")
        .map(|p| p.prefixes.clone())
        .unwrap_or_default();
    let f = r
        .content
        .name_pools
        .get("names.family")
        .map(|p| p.prefixes.clone())
        .unwrap_or_default();
    (g, f)
}

fn age(st: &PoliticsState, id: PoliticianId, year: u32) -> u32 {
    st.get(id)
        .map(|p| year.saturating_sub(p.birth_year))
        .unwrap_or(50)
}

/// Party support in a constituency: turnout-weighted attachment shares.
fn party_strength(pop: &PopulationState, hoods: &[u32], parties: usize) -> Vec<f64> {
    let mut acc = vec![0.0; parties];
    let mut total = 0.0;
    for &h in hoods {
        for c in &pop.hoods[h as usize].cohorts {
            let w = c.count as f64 * c.turnout as f64;
            total += w;
            for (p, a) in c.party_id.iter().enumerate().take(parties) {
                acc[p] += *a as f64 * w;
            }
        }
    }
    acc.iter().map(|a| a / total.max(1.0)).collect()
}

fn open_campaign(
    st: &mut PoliticsState,
    r: &Reads<'_>,
    out: &mut Outs<'_>,
    cal: &ElectionCalendarDef,
    day: SimDate,
) {
    let params = &r.content.politics;
    let pop = r.population();
    let world = r.world;
    let year = r.date.calendar().year;
    let seed = r.meta.seed;
    let np = st.parties.len();
    let (given, family) = names(r);
    let term_limit = r
        .content
        .constitutions
        .get(&r.meta.constitution)
        .and_then(|c| c.tier(TierKey::Local))
        .and_then(|t| t.offices.first())
        .and_then(|o| o.term_limit);
    // National swing and pollster house effects for this cycle.
    let mut cycle_rng = mandate_core::rng::keyed(seed, Stream::Elections, &[day.0 as u64, 0]);
    st.elections.national_swing = (0..np)
        .map(|_| cycle_rng.normal(0.0, params.vote.national_swing_sd) as f32)
        .collect();
    st.elections.house_effects = (0..params.polls.pollsters.len())
        .map(|_| {
            (0..np)
                .map(|_| cycle_rng.normal(0.0, params.polls.house_sd) as f32)
                .collect()
        })
        .collect();
    st.elections.polls.clear();
    let player = r.player.map(|p| p.politician);
    let filing = r.player.and_then(|p| p.filing);
    // A party will not renominate an officeholder it has lost patience with.
    let renominated = r
        .player
        .is_none_or(|p| p.tracks.standing as f64 >= params.vote.renomination_standing);
    if !renominated && !matches!(filing, Some(Filing::Retire)) {
        out.events
            .record(r.date, EventKind::PlayerDeselected, vec![]);
    }

    let mut races = Vec::new();
    for (mi, mu) in world.municipalities.iter().enumerate() {
        let m = mi as u32;
        let Some(council) = st.councils.get(mi).cloned() else {
            continue;
        };
        let all_hoods: Vec<u32> = mu.neighborhoods.iter().map(|h| h.0).collect();
        let mut rng =
            mandate_core::rng::keyed(seed, Stream::Elections, &[day.0 as u64, 1 + m as u64]);
        // --- Mayor first: an open mayoralty draws ambitious councillors.
        let mayor = council.mayor;
        let mayor_runs = mayor.is_some_and(|id| {
            let p = st.get(id);
            let limited = term_limit.is_some_and(|l| p.is_some_and(|p| p.terms_in_office >= l));
            if Some(id) == player {
                return matches!(filing, Some(Filing::Mayor) | None) && !limited && renominated;
            }
            !limited && age(st, id, year) < 74 && rng.chance(0.85)
        });
        let strength = party_strength(pop, &all_hoods, np);
        let mut mayor_candidates: Vec<PoliticianId> = Vec::new();
        let mut leaving_council: Vec<PoliticianId> = Vec::new();
        for pi in 0..np {
            let pid = PartyId(pi as u32);
            if pi >= 2 && strength[pi] < params.vote.minor_party_threshold {
                continue;
            }
            if mayor_runs
                && mayor
                    .and_then(|id| st.get(id))
                    .is_some_and(|p| p.party == Some(pid))
            {
                if let Some(id) = mayor {
                    mayor_candidates.push(id);
                }
                continue;
            }
            // The player's party nominates the player when they file (Standing gate).
            if filing == Some(Filing::Mayor)
                && player
                    .and_then(|id| st.get(id))
                    .is_some_and(|p| p.party == Some(pid) && p.municipality() == m)
                && r.player.is_some_and(|p| p.tracks.standing >= 40.0)
            {
                if let Some(id) = player {
                    mayor_candidates.push(id);
                    if council.members.contains(&id) {
                        leaving_council.push(id);
                    }
                }
                continue;
            }
            // An ambitious councillor of this party, else a fresh candidate.
            let pick = council
                .members
                .iter()
                .copied()
                .filter(|id| Some(*id) != player)
                .filter(|id| {
                    st.get(*id)
                        .is_some_and(|p| p.party == Some(pid) && p.ambition > 0.6)
                })
                .max_by(|a, b| {
                    let s = |id: &PoliticianId| {
                        st.get(*id).map(|p| p.ambition + p.oratory).unwrap_or(0.0)
                    };
                    s(a).total_cmp(&s(b))
                })
                .filter(|_| rng.chance(0.5));
            let id = match pick {
                Some(id) => {
                    leaving_council.push(id);
                    id
                }
                None => recruit(st, &mut rng, &given, &family, pop, &all_hoods, pid, m, year),
            };
            mayor_candidates.push(id);
        }
        if !mayor_candidates.is_empty() {
            races.push(new_race(
                st,
                r,
                Seat::Mayor { municipality: m },
                all_hoods.clone(),
                mayor,
                mayor_candidates.clone(),
                cal,
                day,
                &mut rng,
            ));
        }
        // --- Council wards.
        for (wi, ward) in world
            .constituencies
            .iter()
            .filter(|c| c.tier == Tier::Local && c.jurisdiction == m)
            .enumerate()
        {
            let hoods: Vec<u32> = ward.neighborhoods.iter().map(|h| h.0).collect();
            let sitting = council.members.get(wi).copied();
            let runs = sitting.is_some_and(|id| {
                if leaving_council.contains(&id) {
                    return false;
                }
                if Some(id) == player {
                    return matches!(filing, Some(Filing::Council) | None) && renominated;
                }
                age(st, id, year) < 76 && rng.chance(0.82)
            });
            let strength = party_strength(pop, &hoods, np);
            let mut cands = Vec::new();
            for pi in 0..np {
                let pid = PartyId(pi as u32);
                if pi >= 2 && strength[pi] < params.vote.minor_party_threshold {
                    continue;
                }
                if runs
                    && sitting
                        .and_then(|id| st.get(id))
                        .is_some_and(|p| p.party == Some(pid))
                {
                    if let Some(id) = sitting {
                        cands.push(id);
                    }
                    continue;
                }
                // A player out of office who filed for their old ward is their
                // party's nominee there.
                if filing == Some(Filing::Council)
                    && r.player.is_some_and(|p| p.ward == ward.id)
                    && player
                        .and_then(|id| st.get(id))
                        .is_some_and(|p| !p.in_office() && p.party == Some(pid))
                {
                    if let Some(id) = player {
                        cands.push(id);
                    }
                    continue;
                }
                // A former officeholder of this party living here, else a recruit.
                let former = st
                    .politicians
                    .iter()
                    .filter(|p| {
                        matches!(p.role, Role::Private { municipality } if municipality == m)
                            && p.party == Some(pid)
                            && !p.is_player
                            && hoods.contains(&p.home)
                            && year.saturating_sub(p.birth_year) < 70
                            && p.ambition > 0.5
                    })
                    .map(|p| p.id)
                    .find(|id| !mayor_candidates.contains(id));
                let id = match former {
                    Some(id) if rng.chance(0.6) => id,
                    _ => recruit(st, &mut rng, &given, &family, pop, &hoods, pid, m, year),
                };
                cands.push(id);
            }
            races.push(new_race(
                st,
                r,
                Seat::Council {
                    municipality: m,
                    ward: ward.id,
                },
                hoods,
                sitting,
                cands,
                cal,
                day,
                &mut rng,
            ));
        }
    }
    st.elections.races = races;
    out.events.record(
        r.date,
        EventKind::CampaignOpened { election_day: day },
        vec![],
    );
}

#[allow(clippy::too_many_arguments)]
fn recruit(
    st: &mut PoliticsState,
    rng: &mut SimRng,
    given: &[String],
    family: &[String],
    pop: &PopulationState,
    hoods: &[u32],
    party: PartyId,
    municipality: u32,
    year: u32,
) -> PoliticianId {
    let hs: Vec<usize> = hoods.iter().map(|h| *h as usize).collect();
    let local = mean_attitudes(pop, &hs);
    let platform = st.party(party).map(|p| p.platform).unwrap_or(local);
    let mut att = [0.0f32; ATTITUDES];
    for k in 0..ATTITUDES {
        att[k] = clamp(
            0.55 * platform[k] as f64 + 0.45 * local[k] as f64 + rng.normal(0.0, 0.12),
            -1.0,
            1.0,
        ) as f32;
    }
    let home = *rng.pick(hoods).unwrap_or(&0);
    let id = new_politician(
        st,
        rng,
        given,
        family,
        Role::Private { municipality },
        att,
        Some(party),
        home,
        year,
    );
    if let Some(p) = st.get_mut(id) {
        p.birth_year = year.saturating_sub(rng.range_i64(26, 62) as u32);
    }
    id
}

#[allow(clippy::too_many_arguments)]
fn new_race(
    st: &PoliticsState,
    r: &Reads<'_>,
    seat: Seat,
    hoods: Vec<u32>,
    incumbent: Option<PoliticianId>,
    candidates: Vec<PoliticianId>,
    cal: &ElectionCalendarDef,
    day: SimDate,
    rng: &mut SimRng,
) -> Race {
    let params = &r.content.politics;
    let pop = r.population();
    let player = r.player.map(|p| p.politician);
    let cands = candidates
        .iter()
        .map(|id| {
            let p = st.get(*id);
            let in_office_here =
                p.is_some_and(|p| p.in_office() && p.municipality() == seat.municipality());
            let is_incumbent = Some(*id) == incumbent;
            let awareness: Vec<f32> = hoods
                .iter()
                .map(|h| {
                    if Some(*id) == player {
                        let cs = &pop.hoods[*h as usize].cohorts;
                        let (s, n) = cs.iter().fold((0.0f64, 0.0f64), |(s, n), c| {
                            (
                                s + c.player_awareness as f64 * c.count as f64,
                                n + c.count as f64,
                            )
                        });
                        let own = (s / n.max(1.0)) as f32;
                        return if is_incumbent {
                            own.max(params.vote.incumbent_awareness as f32 * 0.8)
                        } else {
                            own.max(params.vote.challenger_awareness as f32)
                        };
                    }
                    let base = if is_incumbent {
                        params.vote.incumbent_awareness
                    } else if in_office_here {
                        params.vote.incumbent_awareness * 0.6
                    } else {
                        params.vote.challenger_awareness
                    };
                    let home = p.is_some_and(|p| p.home == *h);
                    (base + if home { 0.15 } else { 0.0 }).min(0.95) as f32
                })
                .collect();
            Candidate {
                politician: *id,
                party: p.and_then(|p| p.party),
                awareness_start: awareness.clone(),
                awareness,
                persuasion: vec![0.0; hoods.len()],
                mobilisation: vec![0.0; hoods.len()],
                contacts: vec![0.0; hoods.len()],
                spent: 0.0,
            }
        })
        .collect::<Vec<_>>();
    let n = cands.len();
    Race {
        seat,
        opened: r.date,
        election_day: day,
        hoods,
        incumbent,
        candidates: cands,
        partisan: cal.partisan_ballot,
        shock: (0..n)
            .map(|_| rng.normal(0.0, params.vote.race_shock_sd) as f32)
            .collect(),
        poll_bias_z: (0..n).map(|_| rng.normal(0.0, 1.0) as f32).collect(),
    }
}

// --- Campaign ------------------------------------------------------------------

/// Weekly: persuasion and contact decay; abstract campaigning for NPC candidates.
fn campaign_week(st: &mut PoliticsState, r: &Reads<'_>, out: &mut Outs<'_>) {
    let params = &r.content.politics.campaign;
    let decay = exp(-7.0 * core_ln2() / params.persuasion_half_life_days.max(1.0)) as f32;
    let player = r.player.map(|p| p.politician);
    let home = r.player.map(|p| p.municipality.0);
    let pop = r.population();
    let rng = out.rng.get(Stream::Elections);
    let PoliticsState {
        elections,
        politicians,
        ..
    } = st;
    for race in elections.races.iter_mut() {
        for c in race.candidates.iter_mut() {
            for v in c.persuasion.iter_mut() {
                *v *= decay;
            }
            for v in c.contacts.iter_mut() {
                *v *= 0.8;
            }
            // The player campaigns with commands; NPCs in the player's town use
            // the full AI (npc.rs). Everyone else is abstracted here.
            if Some(c.politician) == player || Some(race.seat.municipality()) == home {
                continue;
            }
            // NPC campaign effort: contacts weighted toward populous neighbourhoods
            // they are least known in (DEC-030: the same contact rules as the player).
            let skill = politicians
                .get(c.politician.index())
                .map(|p| 0.5 + p.oratory as f64)
                .unwrap_or(1.0);
            let effort = params.npc_contacts_per_week * skill;
            let mut whole = effort.floor() as u32;
            if rng.chance(effort - effort.floor()) {
                whole += 1;
            }
            for _ in 0..whole {
                let weights: Vec<f64> = race
                    .hoods
                    .iter()
                    .enumerate()
                    .map(|(i, h)| {
                        let adults = pop.hoods[*h as usize].adults() as f64;
                        adults * (1.05 - c.awareness[i] as f64) / (1.0 + c.contacts[i] as f64)
                    })
                    .collect();
                if let Some(i) = rng.weighted_index(&weights) {
                    contact(c, i, params, 1.0);
                }
            }
        }
    }
}

fn core_ln2() -> f64 {
    core::f64::consts::LN_2
}

/// Apply one campaign contact (canvass-equivalent) to a neighbourhood, with
/// diminishing returns. `scale` lets town halls and ads reuse the rule.
pub fn contact(c: &mut Candidate, i: usize, params: &mandate_content::CampaignParams, scale: f64) {
    let factor = mandate_core::math::pow(params.diminishing, c.contacts[i] as f64) * scale;
    c.awareness[i] = clamp(
        c.awareness[i] as f64
            + params.awareness_per_canvass * factor * (1.0 - c.awareness[i] as f64),
        0.0,
        1.0,
    ) as f32;
    c.persuasion[i] += (params.persuasion_per_canvass * factor) as f32;
    c.mobilisation[i] = clamp(
        c.mobilisation[i] as f64 + params.mobilisation_per_canvass * factor,
        0.0,
        1.0,
    ) as f32;
    c.contacts[i] += 1.0;
}

/// One ad buy: broad, shallow reach (money instead of time). Raises name
/// recognition everywhere in the race, persuades a little, mobilises nobody,
/// and saturates within a turn (`nth` = buys already made this turn).
pub fn advertise(c: &mut Candidate, params: &mandate_content::CampaignParams, nth: u8) {
    let f = mandate_core::math::pow(0.7, nth as f64);
    for i in 0..c.awareness.len() {
        c.awareness[i] = clamp(
            c.awareness[i] as f64 + params.awareness_per_ad * f * (1.0 - c.awareness[i] as f64),
            0.0,
            1.0,
        ) as f32;
        c.persuasion[i] += (0.25 * params.persuasion_per_canvass * f) as f32;
    }
    c.spent += params.ad_cost as f64;
}

// --- Vote model ------------------------------------------------------------------

/// Which utility terms to include (all for the real count; one off for decomposition).
#[derive(Clone, Copy)]
struct Terms {
    issues: bool,
    party: bool,
    valence: bool,
    record: bool,
    campaign: bool,
    shock: bool,
    /// Fraction of the hidden race shock visible (polls see part of it late on).
    shock_visible: f64,
}

const ALL: Terms = Terms {
    issues: true,
    party: true,
    valence: true,
    record: true,
    campaign: true,
    shock: true,
    shock_visible: 1.0,
};

/// Context shared by every cohort in a race.
struct RaceCtx<'a> {
    race: &'a Race,
    st: &'a PoliticsState,
    params: &'a PoliticsParams,
    pop: &'a PopulationState,
    player: Option<PoliticianId>,
    mean: [f32; ATTITUDES],
    beta: f64,
    turnout_mult: f64,
    media: f64,
    retro: f64,
    /// Local viability per candidate (party strength relative to the strongest).
    viability: Vec<f64>,
}

/// Mean attitude standard deviation across a set of neighbourhoods (polarisation).
fn polarisation(pop: &PopulationState, hoods: &[u32], mean: &[f32; ATTITUDES]) -> f64 {
    let mut acc = 0.0;
    let mut w = 0.0;
    for &h in hoods {
        for c in &pop.hoods[h as usize].cohorts {
            let n = c.count as f64;
            for k in 0..ATTITUDES {
                let d = c.attitudes[k] as f64 - mean[k] as f64;
                acc += d * d * n;
            }
            w += n * ATTITUDES as f64;
        }
    }
    mandate_core::math::sqrt(acc / w.max(1.0))
}

fn race_ctx<'a>(st: &'a PoliticsState, r: &'a Reads<'_>, race: &'a Race) -> RaceCtx<'a> {
    let params = &r.content.politics;
    let pop = r.population();
    let hs: Vec<usize> = race.hoods.iter().map(|h| *h as usize).collect();
    let mean = mean_attitudes(pop, &hs);
    let pol = polarisation(pop, &race.hoods, &mean);
    let v = &params.vote;
    let t = clamp(
        (pol - v.polarization_low) / (v.polarization_high - v.polarization_low).max(1e-6),
        0.0,
        1.0,
    );
    let beta = v.beta_low + (v.beta_high - v.beta_low) * t;
    let timing = local_calendar(r).map(|(c, _)| c.timing);
    let turnout_mult = match timing {
        Some(ElectionTiming::Concurrent) => params.turnout.concurrent,
        _ => params.turnout.standalone,
    };
    // Media weight: towns that dominate their region are judged on their record.
    let m = race.seat.municipality() as usize;
    let world = r.world;
    let region_pop: u64 = world
        .municipalities
        .get(m)
        .and_then(|mu| world.regions.get(mu.region.index()))
        .map(|re| {
            re.municipalities
                .iter()
                .map(|x| pop.municipality_total(world, *x))
                .sum()
        })
        .unwrap_or(1);
    let own = pop.municipality_total(world, MunicipalityId(m as u32));
    let media = 0.5 + 0.5 * own as f64 / region_pop.max(1) as f64;
    // Benchmarked retrospective: local unemployment change against national (DEC-071 §2).
    let econ = r.economy();
    let retro = econ
        .munis
        .get(m)
        .map(|e| {
            let du = match (
                e.history.last(),
                e.history.get(e.history.len().saturating_sub(13)),
            ) {
                (Some(a), Some(b)) => (a.unemployment - b.unemployment) as f64,
                _ => 0.0,
            };
            let h = &econ.national.history;
            let dn = match (h.last(), h.get(h.len().saturating_sub(13))) {
                (Some(a), Some(b)) => (a.unemployment - b.unemployment) as f64,
                _ => 0.0,
            };
            clamp(-(du - dn) * 10.0, -1.0, 1.0)
        })
        .unwrap_or(0.0);
    let strength = party_strength(pop, &race.hoods, st.parties.len());
    let top = strength.iter().cloned().fold(0.0, f64::max).max(1e-6);
    let viability = race
        .candidates
        .iter()
        .map(|c| match c.party.and_then(|p| strength.get(p.index())) {
            Some(s) => (s / top).min(1.0),
            None => 0.3,
        })
        .collect();
    RaceCtx {
        race,
        st,
        params,
        pop,
        viability,
        player: r.player.map(|p| p.politician),
        mean,
        beta,
        turnout_mult,
        media,
        retro,
    }
}

impl RaceCtx<'_> {
    /// Utility of each candidate for one cohort in race neighbourhood `i`.
    fn utilities(&self, i: usize, c: &Cohort, terms: Terms, swing: &[f32]) -> Vec<f64> {
        let v = &self.params.vote;
        let h = self.race.hoods[i];
        self.race
            .candidates
            .iter()
            .enumerate()
            .map(|(k, cand)| {
                let Some(p) = self.st.get(cand.politician) else {
                    return 0.0;
                };
                let party = cand.party.and_then(|id| self.st.party(id));
                // Perceived position: the person, coloured by the party label.
                let mut pos = p.attitudes;
                if self.race.partisan
                    && let Some(pt) = party
                {
                    for a in 0..ATTITUDES {
                        pos[a] = 0.6 * p.attitudes[a] + 0.4 * pt.platform[a];
                    }
                }
                let mut prox = 0.0;
                let mut dir = 0.0;
                for a in 0..ATTITUDES {
                    let x = c.attitudes[a] as f64;
                    let q = pos[a] as f64;
                    prox -= (x - q) * (x - q);
                    dir += (x - self.mean[a] as f64) * (q - self.mean[a] as f64);
                }
                let issue = ((1.0 - self.beta) * prox / ATTITUDES as f64 * 4.0
                    + self.beta * dir / ATTITUDES as f64 * 4.0)
                    * v.issue_weight;
                let pid_share = cand
                    .party
                    .and_then(|id| c.party_id.get(id.index()))
                    .copied()
                    .unwrap_or(0.0) as f64;
                let partisan = if self.race.partisan {
                    pid_share * v.party_weight
                } else {
                    0.0
                };
                let quality = v.valence_weight
                    * (0.5 * (p.oratory as f64 - 0.5) + 0.5 * (p.integrity as f64 - 0.5)
                        - p.scandal as f64)
                    + if p.home == h { v.home_ward_bonus } else { 0.0 }
                    // An incumbent's good record helps with voters who know them.
                    + if Some(cand.politician) == self.race.incumbent {
                        v.record_bonus * (p.record as f64 - v.record_baseline).max(0.0)
                    } else {
                        0.0
                    };
                // A thin record is attacked in the campaign: every voter hears.
                let absentee = if Some(cand.politician) == self.race.incumbent {
                    v.record_penalty * (p.record as f64 - v.record_baseline).min(0.0)
                } else {
                    0.0
                };
                // Record: incumbents (and the governing party's nominee, at half weight).
                let incumbent = Some(cand.politician) == self.race.incumbent;
                let gov_party = self
                    .race
                    .incumbent
                    .and_then(|id| self.st.get(id))
                    .and_then(|ip| ip.party);
                let record_weight = if incumbent {
                    1.0
                } else if cand.party.is_some() && cand.party == gov_party {
                    0.4
                } else {
                    0.0
                };
                let exec = if self.race.seat.is_executive() {
                    v.executive_multiplier
                } else {
                    1.0
                };
                let mut record = record_weight
                    * v.retrospective_weight
                    * 0.1
                    * exec
                    * self.media
                    * ((c.gov_approval as f64 - 0.5) + 0.5 * self.retro);
                if record < 0.0 && self.race.partisan {
                    record *= 1.0 - v.partisan_discount * pid_share / 0.7;
                }
                if Some(cand.politician) == self.player {
                    record += c.player_approval as f64 * 1.2;
                }
                let aw = if terms.campaign {
                    cand.awareness[i] as f64
                } else {
                    cand.awareness_start[i] as f64
                };
                let aw = if Some(cand.politician) == self.player {
                    aw.max(if terms.campaign {
                        c.player_awareness as f64
                    } else {
                        0.0
                    })
                } else {
                    aw
                };
                let persuasion = if terms.campaign {
                    cand.persuasion[i] as f64
                } else {
                    0.0
                };
                let known = (if terms.issues { issue } else { 0.0 })
                    + (if terms.valence { quality } else { 0.0 })
                    + (if terms.record { record } else { 0.0 })
                    + persuasion;
                let shock = if terms.shock {
                    self.race.shock[k] as f64 * terms.shock_visible
                        + cand
                            .party
                            .and_then(|id| swing.get(id.index()))
                            .copied()
                            .unwrap_or(0.0) as f64
                } else {
                    0.0
                };
                let wasted = v.strategic_weight * (1.0 - self.viability[k]).powi(2);
                aw * known
                    + if terms.party { partisan - wasted } else { 0.0 }
                    + if terms.record { absentee } else { 0.0 }
                    + shock
            })
            .collect()
    }

    fn turnout(&self, i: usize, c: &Cohort, terms: Terms) -> f64 {
        let t = &self.params.turnout;
        let mob: f64 = if terms.campaign {
            self.race
                .candidates
                .iter()
                .map(|cd| cd.mobilisation[i] as f64)
                .fold(0.0, f64::max)
        } else {
            0.0
        };
        clamp(
            (1.0 - t.habit) * c.turnout as f64 * self.turnout_mult
                + t.habit * c.last_turnout as f64
                + mob * t.mobilisation_max,
            0.02,
            0.95,
        )
    }

    /// Expected votes per candidate and total turnout votes; optional per-hood shares.
    fn tally(
        &self,
        terms: Terms,
        swing: &[f32],
        per_hood: bool,
    ) -> (Vec<f64>, f64, f64, Vec<Vec<f64>>) {
        let n = self.race.candidates.len();
        let mut votes = vec![0.0; n];
        let mut voters = 0.0;
        let mut adults = 0.0;
        let mut hood_votes = Vec::new();
        for (i, h) in self.race.hoods.iter().enumerate() {
            let mut hv = vec![0.0; n];
            for c in &self.pop.hoods[*h as usize].cohorts {
                if c.count == 0 {
                    continue;
                }
                let t = self.turnout(i, c, terms);
                let cast = c.count as f64 * t;
                adults += c.count as f64;
                voters += cast;
                let u = self.utilities(i, c, terms, swing);
                let max = u.iter().cloned().fold(f64::MIN, f64::max);
                let e: Vec<f64> = u.iter().map(|x| exp(x - max)).collect();
                let s: f64 = e.iter().sum::<f64>().max(1e-12);
                for k in 0..n {
                    let v = cast * e[k] / s;
                    votes[k] += v;
                    hv[k] += v;
                }
            }
            if per_hood {
                hood_votes.push(hv);
            }
        }
        (votes, voters, adults, hood_votes)
    }
}

fn shares(v: &[f64]) -> Vec<f64> {
    let s: f64 = v.iter().sum::<f64>().max(1e-12);
    v.iter().map(|x| x / s).collect()
}

/// Expected vote shares now (the model truth without the hidden shock): used by
/// the NPC AI and lab tools, never shown to the player (DEC-029).
pub fn expected_shares(st: &PoliticsState, r: &Reads<'_>, race: &Race) -> Vec<f64> {
    let ctx = race_ctx(st, r, race);
    let terms = Terms {
        shock: false,
        shock_visible: 0.0,
        ..ALL
    };
    shares(&ctx.tally(terms, &st.elections.national_swing, false).0)
}

// --- Polls -----------------------------------------------------------------------

fn publish_polls(st: &mut PoliticsState, r: &Reads<'_>, out: &mut Outs<'_>, day: SimDate) {
    if r.calendar.phase == crate::state::GamePhase::BurnIn {
        return;
    }
    let params = &r.content.politics.polls;
    let home = r.player.map(|p| p.municipality.0);
    let rng = out.rng.get(Stream::Elections);
    let mut new_polls = Vec::new();
    for (ri, race) in st.elections.races.iter().enumerate() {
        // Pollsters cover mayoral races everywhere and council races in the player's town.
        if !race.seat.is_executive() && Some(race.seat.municipality()) != home {
            continue;
        }
        let ctx = race_ctx(st, r, race);
        let total_days = race.opened.days_until(day).max(1) as f64;
        let left = r.date.days_until(day) as f64;
        let progress = 1.0 - left / total_days;
        let terms = Terms {
            shock_visible: 0.6 * progress,
            ..ALL
        };
        let (votes, _, _, _) = ctx.tally(terms, &st.elections.national_swing, false);
        let truth = shares(&votes);
        let undecided: f64 = {
            let mut acc = 0.0;
            let mut w = 0.0;
            for (i, h) in race.hoods.iter().enumerate() {
                let n = r.population().hoods[*h as usize].adults() as f64;
                let known = race
                    .candidates
                    .iter()
                    .map(|c| c.awareness[i] as f64)
                    .fold(0.0, f64::max);
                acc += n * (1.0 - known) * 0.4;
                w += n;
            }
            clamp(acc / w.max(1.0), 0.02, 0.4)
        };
        let shared = params.shared_sd * (1.0 + (params.early_multiplier - 1.0) * (1.0 - progress));
        let pollster = (rng.below(params.pollsters.len().max(1) as u64)) as u8;
        let house = st.elections.house_effects.get(pollster as usize);
        let n = params.sample_size as f64;
        let mut s: Vec<f64> = truth
            .iter()
            .enumerate()
            .map(|(k, p)| {
                let party = race.candidates[k].party;
                let he = party
                    .and_then(|id| house.and_then(|h| h.get(id.index())))
                    .copied()
                    .unwrap_or(0.0) as f64;
                let sampling = mandate_core::math::sqrt(p * (1.0 - p) / n.max(1.0));
                (p + race.poll_bias_z[k] as f64 * shared + he + rng.normal(0.0, sampling))
                    .max(0.005)
            })
            .collect();
        let sum: f64 = s.iter().sum();
        for v in s.iter_mut() {
            *v /= sum;
        }
        new_polls.push(Poll {
            race: ri as u32,
            date: r.date,
            pollster,
            shares: s.iter().map(|v| *v as f32).collect(),
            undecided: undecided as f32,
            sample: params.sample_size,
        });
    }
    if !new_polls.is_empty() {
        out.events.record(
            r.date,
            EventKind::PollsPublished {
                count: new_polls.len() as u32,
            },
            vec![],
        );
    }
    st.elections.polls.extend(new_polls);
}

/// Average of the latest poll from each pollster for a race.
pub fn poll_average(st: &ElectionState, race: usize) -> Option<Vec<f32>> {
    let mut latest: Vec<&Poll> = Vec::new();
    for p in st.polls.iter().rev().filter(|p| p.race as usize == race) {
        if !latest.iter().any(|q| q.pollster == p.pollster) {
            latest.push(p);
        }
    }
    let first = latest.first()?;
    let mut avg = vec![0.0f32; first.shares.len()];
    for p in &latest {
        for (a, s) in avg.iter_mut().zip(&p.shares) {
            *a += s / latest.len() as f32;
        }
    }
    Some(avg)
}

// --- Election day ------------------------------------------------------------------

const FACTORS: [&str; 6] = ["issues", "party", "valence", "record", "campaign", "shock"];

fn hold(st: &mut PoliticsState, r: &Reads<'_>, out: &mut Outs<'_>, _cal: &ElectionCalendarDef) {
    let races = std::mem::take(&mut st.elections.races);
    let swing = st.elections.national_swing.clone();
    let player = r.player.map(|p| p.politician);
    let mut results = Vec::new();
    let mut hood_turnout: Vec<(u32, f32)> = Vec::new();
    for (ri, race) in races.iter().enumerate() {
        let ctx = race_ctx(st, r, race);
        let (votes, voters, adults, hood_votes) = ctx.tally(ALL, &swing, true);
        let sh = shares(&votes);
        let mut order: Vec<usize> = (0..votes.len()).collect();
        order.sort_by(|a, b| votes[*b].total_cmp(&votes[*a]));
        let Some(&w) = order.first() else { continue };
        let runner = order.get(1).copied();
        // Factor decomposition of the winner's margin over the runner-up.
        // History (burn-in) results need no explanation screen: skip the cost.
        let explain = r.calendar.phase != crate::state::GamePhase::BurnIn;
        let decomposition = runner
            .filter(|_| explain)
            .map(|ru| {
                let margin = |v: &[f64]| {
                    let s = shares(v);
                    (s[w] - s[ru]) as f32
                };
                let full = margin(&votes);
                FACTORS
                    .iter()
                    .map(|f| {
                        let mut t = ALL;
                        match *f {
                            "issues" => t.issues = false,
                            "party" => t.party = false,
                            "valence" => t.valence = false,
                            "record" => t.record = false,
                            "campaign" => t.campaign = false,
                            _ => t.shock = false,
                        }
                        let without = ctx.tally(t, &swing, false).0;
                        FactorShare {
                            factor: (*f).to_string(),
                            margin: full - margin(&without),
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();
        let final_poll = poll_average(&st.elections, ri);
        let hood_winner_share = race
            .hoods
            .iter()
            .zip(&hood_votes)
            .map(|(h, hv)| (*h, shares(hv)[w] as f32))
            .collect();
        for (i, h) in race.hoods.iter().enumerate() {
            if race.seat.is_executive() {
                let t: f64 = ctx.pop.hoods[*h as usize]
                    .cohorts
                    .iter()
                    .map(|c| ctx.turnout(i, c, ALL) * c.count as f64)
                    .sum::<f64>()
                    / ctx.pop.hoods[*h as usize].adults().max(1) as f64;
                hood_turnout.push((*h, t as f32));
            }
        }
        results.push(RaceResult {
            seat: race.seat,
            date: r.date,
            candidates: race
                .candidates
                .iter()
                .enumerate()
                .map(|(k, c)| CandidateResult {
                    politician: c.politician,
                    party: c.party,
                    votes: votes[k].round() as u64,
                    share: sh[k] as f32,
                    final_poll: final_poll.as_ref().and_then(|p| p.get(k)).copied(),
                })
                .collect(),
            winner: race.candidates[w].politician,
            incumbent: race.incumbent,
            turnout: (voters / adults.max(1.0)) as f32,
            decomposition,
            hood_winner_share,
        });
    }
    // Seats change hands: councils first, then mayors.
    let mut winners_council: Vec<(u32, ConstituencyId, PoliticianId)> = Vec::new();
    let mut winners_mayor: Vec<(u32, PoliticianId)> = Vec::new();
    let mut ran: Vec<PoliticianId> = Vec::new();
    for res in &results {
        for c in &res.candidates {
            ran.push(c.politician);
        }
        match res.seat {
            Seat::Council { municipality, ward } => {
                winners_council.push((municipality, ward, res.winner))
            }
            Seat::Mayor { municipality } => winners_mayor.push((municipality, res.winner)),
        }
    }
    let ward_index = |world: &World, m: u32, ward: ConstituencyId| {
        world
            .constituencies
            .iter()
            .filter(|c| c.tier == Tier::Local && c.jurisdiction == m)
            .position(|c| c.id == ward)
    };
    let mut new_holders: Vec<PoliticianId> = Vec::new();
    for (m, ward, id) in &winners_council {
        if let (Some(council), Some(wi)) = (
            st.councils.get_mut(*m as usize),
            ward_index(r.world, *m, *ward),
        ) && let Some(slot) = council.members.get_mut(wi)
        {
            *slot = *id;
        }
        set_role(
            st,
            *id,
            Role::Councillor {
                municipality: *m,
                ward: *ward,
            },
        );
        new_holders.push(*id);
    }
    for (m, id) in &winners_mayor {
        if let Some(council) = st.councils.get_mut(*m as usize) {
            council.mayor = Some(*id);
        }
        set_role(st, *id, Role::Mayor { municipality: *m });
        new_holders.push(*id);
    }
    // Anyone whose claimed seat now belongs to someone else leaves office.
    let councils: Vec<Council> = st.councils.clone();
    for p in st.politicians.iter_mut() {
        let holds = match p.role {
            Role::Councillor { municipality, .. } => councils
                .get(municipality as usize)
                .is_some_and(|c| c.members.contains(&p.id)),
            Role::Mayor { municipality } => councils
                .get(municipality as usize)
                .is_some_and(|c| c.mayor == Some(p.id)),
            Role::Private { .. } => true,
        };
        if !holds {
            p.role = Role::Private {
                municipality: p.municipality(),
            };
            p.terms_in_office = 0;
        }
    }
    // Re-elected incumbents add a term; newcomers start their first.
    for res in &results {
        if let Some(p) = st.get_mut(res.winner) {
            if res.incumbent == Some(res.winner) {
                p.terms_in_office = p.terms_in_office.saturating_add(1);
            } else {
                p.terms_in_office = 1;
            }
        }
    }
    // Events: the player's town in detail, other towns' mayoral upsets.
    let home = r.player.map(|p| p.municipality.0);
    for res in &results {
        let m = res.seat.municipality();
        let defeated = res
            .incumbent
            .is_some_and(|i| i != res.winner && res.candidates.iter().any(|c| c.politician == i));
        let mine = Some(m) == home;
        if mine || (res.seat.is_executive() && defeated) {
            out.events.record(
                r.date,
                EventKind::RaceDecided {
                    municipality: m,
                    mayor: res.seat.is_executive(),
                    winner: res.winner.0,
                    incumbent_defeated: defeated,
                    player_won: player == Some(res.winner),
                    player_ran: player
                        .is_some_and(|p| res.candidates.iter().any(|c| c.politician == p)),
                },
                vec![],
            );
        }
    }
    out.events.record(
        r.date,
        EventKind::ElectionHeld {
            races: results.len() as u32,
        },
        vec![],
    );
    // Keep the last few cycles only.
    st.elections.results.extend(results);
    let dates: Vec<SimDate> = {
        let mut d: Vec<SimDate> = st.elections.results.iter().map(|x| x.date).collect();
        d.dedup();
        d
    };
    if dates.len() > KEEP_ELECTIONS {
        let cutoff = dates[dates.len() - KEEP_ELECTIONS];
        st.elections.results.retain(|x| x.date >= cutoff);
    }
    st.elections.last_hood_turnout = hood_turnout;
    st.elections.last_election = Some(r.date);
    st.elections.polls.clear();
    st.elections.next_day = None;
    st.elections.campaign_open = None;
    let _ = new_holders;
}

fn set_role(st: &mut PoliticsState, id: PoliticianId, role: Role) {
    if let Some(p) = st.get_mut(id) {
        p.role = role;
    }
}

/// Monthly: records fade (8%), and NPC officeholders' records drift toward
/// their own diligence. The player has no baseline: their record is earned.
pub fn monthly(st: &mut PoliticsState, _r: &Reads<'_>, _out: &mut Outs<'_>) {
    for p in st.politicians.iter_mut() {
        p.record *= 0.92;
        if p.in_office() && !p.is_player {
            let diligence = 0.5 * p.integrity + 0.5 * p.ambition;
            p.record = (p.record + 0.08 * diligence).min(1.0);
        }
    }
}

/// Yearly upkeep: time in office, scandal decay, and war chests.
pub fn yearly(st: &mut PoliticsState, _r: &Reads<'_>, _out: &mut Outs<'_>) {
    for p in st.politicians.iter_mut() {
        if p.in_office() {
            p.years_in_office = p.years_in_office.saturating_add(1);
            p.funds += 2_000.0 + 3_000.0 * p.oratory as f64;
        }
        p.scandal *= 0.6;
    }
}

/// The player's intention for the next local election.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Filing {
    Council,
    Mayor,
    /// Sit this election out.
    Retire,
}
