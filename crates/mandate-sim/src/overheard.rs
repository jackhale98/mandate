//! Overheard talk (DEC-079): what residents say at a diner, market, or union
//! hall. Read-only. Each week every such place draws a few lines from the
//! content registry (`overheard.jsonc`) that hold for its neighbourhood:
//! lines about the neighbourhood's strongest concerns, and opinions and news
//! grounded in facts (the town's water, its crisis, a plant that closed, its
//! defining industry, the power broker, the rival, a running scandal, a
//! recent story, what residents think of the player).
//!
//! The draw is keyed by (town, neighbourhood, week, place) on the dialogue
//! stream (DEC-041), so two places in one town differ, the same place
//! changes week to week, and last week's lines are avoided where others
//! hold. No prose: Fluent keys and facts only (DEC-054).

use crate::events::EventKind;
use crate::places::{Location, hood_of};
use crate::state::State;
use crate::world::NeighborhoodKind;
use mandate_content::{
    ContentBundle, ContentKey, HeardCondition, HeardSubject, OverheardDef, WaterFeature,
};
use mandate_core::Stream;
use std::collections::BTreeMap;

/// Lines drawn per visit.
pub const LINES: usize = 3;
/// How far back a story counts as news (days).
const NEWS_DAYS: u32 = 56;
/// Share of residents who must know the player before they have an opinion.
const KNOWN: f64 = 0.2;

/// One line as residents say it: a Fluent key and its arguments (names and
/// facts; arguments named `*_key` are Fluent keys). `story` is a media story
/// the line retells (its headline is added by the protocol).
#[derive(Clone, Debug, PartialEq)]
pub struct HeardLine {
    pub id: ContentKey,
    pub key: String,
    pub args: BTreeMap<String, String>,
    pub story: Option<u32>,
}

/// A stable number for a location (part of the draw's key).
fn location_key(loc: Location) -> u64 {
    match loc {
        Location::Home => 0,
        Location::Street { neighborhood } => (1 << 32) | neighborhood.0 as u64,
        Location::Place { place } => (2 << 32) | place.0 as u64,
    }
}

struct Ctx<'a> {
    st: &'a State,
    content: &'a ContentBundle,
    m: u32,
    hood: usize,
}

fn in_range(v: f64, min: Option<f64>, max: Option<f64>) -> bool {
    min.is_none_or(|a| v >= a) && max.is_none_or(|z| v <= z)
}

impl Ctx<'_> {
    fn politician_of(
        &self,
        c: crate::characters::CharacterId,
    ) -> Option<&crate::politics::Politician> {
        self.st
            .politics
            .politicians
            .iter()
            .find(|p| p.character == c)
    }

    fn character_name(&self, c: crate::characters::CharacterId) -> Option<String> {
        crate::characters::profile(&self.st.politics, &self.st.world, c).map(|x| x.full_name())
    }

    fn town_character(&self) -> Option<&crate::characters::TownCharacter> {
        self.st.politics.characters.towns.get(self.m as usize)
    }

    /// The player's mean awareness and approval in the neighbourhood.
    fn player_standing(&self) -> Option<(f64, f64)> {
        self.st.player.as_ref()?;
        let cs = &self.st.population.hoods.get(self.hood)?.cohorts;
        let n: f64 = cs.iter().map(|c| c.count as f64).sum();
        if n <= 0.0 {
            return None;
        }
        let aw = cs
            .iter()
            .map(|c| c.player_awareness as f64 * c.count as f64)
            .sum::<f64>()
            / n;
        let ap = cs
            .iter()
            .map(|c| c.player_approval as f64 * c.count as f64)
            .sum::<f64>()
            / n;
        Some((aw, ap))
    }

    fn player_name(&self) -> Option<String> {
        let pl = self.st.player.as_ref()?;
        self.st.politics.get(pl.politician).map(|p| p.full_name())
    }

    /// Does a condition hold? Adds its arguments when it does.
    fn holds(
        &self,
        c: &HeardCondition,
        args: &mut BTreeMap<String, String>,
        story: &mut Option<u32>,
    ) -> bool {
        let st = self.st;
        let today = st.calendar.date;
        let mu = st.world.municipalities.get(self.m as usize);
        let waterfront = st
            .world
            .neighborhoods
            .get(self.hood)
            .is_some_and(|n| n.kind == NeighborhoodKind::Waterfront);
        let river = mu.is_some_and(|x| x.on_river);
        let coast = mu.is_some_and(|x| x.coastal);
        match c {
            HeardCondition::Water { feature } => match feature {
                WaterFeature::River => river,
                WaterFeature::Coast => coast,
                WaterFeature::Any => river || coast || waterfront,
            },
            HeardCondition::Dry => !(river || coast || waterfront),
            HeardCondition::Crisis { kind } => {
                let Some(x) = self.town_character().and_then(|t| {
                    t.crises
                        .iter()
                        .filter(|x| kind.as_ref().is_none_or(|k| &x.kind == k))
                        .max_by(|a, b| a.severity.total_cmp(&b.severity))
                }) else {
                    return false;
                };
                if let Some(d) = self.content.crises.iter().find(|d| d.id == x.kind) {
                    args.insert("crisis_key".into(), d.name_key.clone());
                }
                true
            }
            HeardCondition::PlantClosure { within_months } => {
                let days = *within_months as u32 * 30;
                let found = st
                    .events
                    .iter()
                    .rev()
                    .take_while(|e| e.date.days_until(today) <= days)
                    .find_map(|e| match &e.kind {
                        EventKind::PlantClosure {
                            municipality,
                            sector,
                            jobs,
                        } if *municipality == self.m => Some((sector.clone(), *jobs)),
                        _ => None,
                    });
                let Some((sector, jobs)) = found else {
                    return false;
                };
                if let Some(s) = self.content.sector(&sector) {
                    args.insert("industry_key".into(), talk_key(&s.name_key));
                }
                args.insert("jobs".into(), jobs.to_string());
                true
            }
            HeardCondition::Disaster { within_months } => st
                .economy
                .munis
                .get(self.m as usize)
                .is_some_and(|e| e.months_since_disaster < *within_months),
            HeardCondition::Industry => {
                let Some(s) = self
                    .town_character()
                    .and_then(|t| t.industry.as_deref())
                    .and_then(|s| self.content.sector(s))
                else {
                    return false;
                };
                args.insert("industry_key".into(), talk_key(&s.name_key));
                true
            }
            HeardCondition::Broker => {
                let Some(n) = self
                    .town_character()
                    .and_then(|t| t.broker)
                    .and_then(|b| self.character_name(b))
                else {
                    return false;
                };
                args.insert("broker".into(), n);
                true
            }
            HeardCondition::Rival => {
                let Some(n) = self
                    .town_character()
                    .and_then(|t| t.rival)
                    .and_then(|r| self.character_name(r))
                else {
                    return false;
                };
                args.insert("rival".into(), n);
                true
            }
            HeardCondition::RivalRecord { min, max } => {
                let Some((name, record)) =
                    self.town_character().and_then(|t| t.rival).and_then(|r| {
                        let p = self.politician_of(r)?;
                        Some((p.full_name(), p.record as f64))
                    })
                else {
                    return false;
                };
                if !in_range(record, *min, *max) {
                    return false;
                }
                args.insert("rival".into(), name);
                true
            }
            HeardCondition::PlayerAwareness { min, max } => {
                let (Some((aw, _)), Some(name)) = (self.player_standing(), self.player_name())
                else {
                    return false;
                };
                if !in_range(aw, *min, *max) {
                    return false;
                }
                args.insert("player".into(), name);
                true
            }
            HeardCondition::PlayerApproval { min, max } => {
                let (Some((aw, ap)), Some(name)) = (self.player_standing(), self.player_name())
                else {
                    return false;
                };
                if aw < KNOWN || !in_range(ap, *min, *max) {
                    return false;
                }
                args.insert("player".into(), name);
                true
            }
            HeardCondition::Scandal { about } => {
                let me = st.player.as_ref().map(|p| p.politician);
                let rival = self
                    .town_character()
                    .and_then(|t| t.rival)
                    .and_then(|r| self.politician_of(r))
                    .map(|p| p.id);
                let Some(x) = st
                    .media
                    .scandals
                    .iter()
                    .rev()
                    .filter(|x| x.running() && x.town == self.m)
                    .find(|x| match about {
                        HeardSubject::Anyone => true,
                        HeardSubject::Player => Some(x.subject) == me,
                        HeardSubject::Rival => Some(x.subject) == rival,
                    })
                else {
                    return false;
                };
                let Some(name) = st.politics.get(x.subject).map(|p| p.full_name()) else {
                    return false;
                };
                args.insert("subject".into(), name);
                args.insert(
                    "fact_key".into(),
                    format!("fact-{}", x.kind.replace('_', "-")),
                );
                true
            }
            HeardCondition::Headline => {
                let Some(s) = st.media.stories.iter().rev().find(|s| {
                    s.town == self.m
                        && s.date.days_until(today) <= NEWS_DAYS
                        && st
                            .media
                            .outlet(s.outlet)
                            .is_some_and(|o| o.covers(&st.world, self.m))
                }) else {
                    return false;
                };
                *story = Some(s.id);
                true
            }
            HeardCondition::ElectionWithin { days } => st
                .politics
                .elections
                .next_day
                .is_some_and(|d| d >= today && today.days_until(d) <= *days),
            HeardCondition::Any { of } => of.iter().any(|x| self.holds(x, args, story)),
        }
    }
}

/// Every line that holds at a location, with its weight and arguments.
fn candidates(st: &State, content: &ContentBundle, loc: Location) -> Vec<(f64, HeardLine)> {
    let Some(h) = hood_of(st, loc) else {
        return Vec::new();
    };
    let Some(n) = st.world.neighborhoods.get(h.index()) else {
        return Vec::new();
    };
    let m = n.municipality.0;
    let ctx = Ctx {
        st,
        content,
        m,
        hood: h.index(),
    };
    // The neighbourhood's strongest three concerns, by how strongly felt.
    let felt: Vec<(usize, f64)> =
        crate::concerns::felt_concerns(&st.population, content, m as usize, &[h.index()])
            .into_iter()
            .filter(|(_, w)| *w > 0.0)
            .take(3)
            .collect();
    const RANK: [f64; 3] = [1.0, 0.7, 0.5];
    let town = st
        .world
        .municipalities
        .get(m as usize)
        .map(|x| x.name.clone())
        .unwrap_or_default();
    let mut out = Vec::new();
    for def in &content.overheard {
        let factor = match &def.concern {
            Some(c) => {
                let Some((rank, (_, w))) = felt
                    .iter()
                    .enumerate()
                    .find(|(_, (i, _))| content.concerns.get(*i).is_some_and(|x| &x.id == c))
                else {
                    continue;
                };
                // A strongly felt concern is talked about more.
                RANK[rank] * (0.4 + w.min(1.5))
            }
            None => 0.6,
        };
        let mut args = BTreeMap::new();
        let mut story = None;
        if !def.when.iter().all(|c| ctx.holds(c, &mut args, &mut story)) {
            continue;
        }
        args.insert("town".into(), town.clone());
        args.insert("hood".into(), n.name.clone());
        out.push((
            def.weight * factor,
            HeardLine {
                id: def.id.clone(),
                key: def.key.clone(),
                args,
                story,
            },
        ));
    }
    out
}

fn concern_of<'a>(content: &'a ContentBundle, id: &str) -> Option<&'a ContentKey> {
    content
        .overheard
        .iter()
        .find(|d| d.id == id)
        .and_then(|d: &OverheardDef| d.concern.as_ref())
}

/// Draw `n` lines for a week (keyed; `avoid` lines are much less likely).
fn draw(
    st: &State,
    content: &ContentBundle,
    loc: Location,
    week: u64,
    n: usize,
    avoid: &[ContentKey],
) -> Vec<HeardLine> {
    let mut pool = candidates(st, content, loc);
    for (w, l) in pool.iter_mut() {
        if avoid.contains(&l.id) {
            *w *= 0.05;
        }
    }
    let (m, h) = hood_of(st, loc)
        .and_then(|h| {
            st.world
                .neighborhoods
                .get(h.index())
                .map(|x| (x.municipality.0, h.0))
        })
        .unwrap_or((0, 0));
    let mut rng = st.rng.keyed(
        Stream::Dialogue,
        &[0x0E4D_4EAD, m as u64, h as u64, week, location_key(loc)],
    );
    let mut out: Vec<HeardLine> = Vec::new();
    while out.len() < n && !pool.is_empty() {
        let total: f64 = pool.iter().map(|(w, _)| *w).sum();
        if total <= 0.0 {
            break;
        }
        let mut x = rng.range_f64(0.0, total);
        let mut pick = pool.len() - 1;
        for (i, (w, _)) in pool.iter().enumerate() {
            if x < *w {
                pick = i;
                break;
            }
            x -= w;
        }
        let (_, line) = pool.remove(pick);
        // Variety: a second line on the same concern is less likely.
        if let Some(c) = concern_of(content, &line.id) {
            for (w, l) in pool.iter_mut() {
                if concern_of(content, &l.id) == Some(c) {
                    *w *= 0.35;
                }
            }
        }
        out.push(line);
    }
    out
}

/// What people at a location are saying this week (at most `LINES`).
pub fn lines(st: &State, content: &ContentBundle, loc: Location) -> Vec<HeardLine> {
    let week = (st.calendar.date.0 / 7) as u64;
    let last: Vec<ContentKey> = if week > 0 {
        draw(st, content, loc, week - 1, LINES, &[])
            .into_iter()
            .map(|l| l.id)
            .collect()
    } else {
        Vec::new()
    };
    draw(st, content, loc, week, LINES, &last)
}

/// The Fluent key of a sector as residents name it (`talk-sector-...`, a
/// noun phrase such as "the factories").
pub fn talk_key(sector_name_key: &str) -> String {
    format!("talk-{sector_name_key}")
}
