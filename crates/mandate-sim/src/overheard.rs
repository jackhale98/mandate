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
//!
//! Reading draws nothing new. The one writer is [`overhear`]: when the
//! player arrives at a place where people talk, each drawn line's
//! `teaches` subjects are raised to `heard` in the player's knowledge
//! ledger (DEC-091; the player's command writing the player's state).

use crate::places::{Location, hood_of};
use crate::state::State;
use mandate_content::{Condition, ContentBundle, ContentKey, OverheardDef};
use mandate_core::Stream;
use std::collections::BTreeMap;

/// Lines drawn per visit.
pub const LINES: usize = 3;

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

/// Every line that holds at a location, with its weight and arguments.
fn candidates(st: &State, content: &ContentBundle, loc: Location) -> Vec<(f64, HeardLine)> {
    let Some(h) = hood_of(st, loc) else {
        return Vec::new();
    };
    let Some(n) = st.world.neighborhoods.get(h.index()) else {
        return Vec::new();
    };
    let m = n.municipality.0;
    let r = crate::scheduler::Reads::of_state(st, content);
    let v = crate::narrative::cast::View::new(&r, &st.politics);
    let cx = crate::condition::Ctx::at_location(&v, st, loc);
    // The neighbourhood's strongest three concerns, by how strongly felt.
    let felt: Vec<(usize, f64)> = crate::concerns::top(
        &crate::concerns::ranked(
            &st.population,
            content,
            &st.world,
            crate::concerns::Scope::Neighbourhood(h),
            mandate_content::condition::Worry::Felt,
        ),
        3,
    )
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
        cx.story.set(None);
        if !crate::condition::all_hold(&cx, &def.when, &mut args) {
            continue;
        }
        let story = cx.story.get();
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

/// Draw `n` lines for a week. Each line gets its own keyed score for the
/// week (weighted sampling without replacement, Efraimidis–Spirakis: the
/// largest `ln(u) / weight` win), so a line's draw does not depend on which
/// other lines hold, and last week's picks can be recomputed. Lines are taken
/// best first, one per concern while others are left; `avoid` lines (last
/// week's) only when nothing else is left.
fn draw(
    st: &State,
    content: &ContentBundle,
    loc: Location,
    week: u64,
    n: usize,
    avoid: &[ContentKey],
) -> Vec<HeardLine> {
    let pool = candidates(st, content, loc);
    let (m, h) = hood_of(st, loc)
        .and_then(|h| {
            st.world
                .neighborhoods
                .get(h.index())
                .map(|x| (x.municipality.0, h.0))
        })
        .unwrap_or((0, 0));
    let mut scored: Vec<(f64, HeardLine)> = pool
        .into_iter()
        .filter(|(w, _)| *w > 0.0)
        .map(|(w, l)| {
            let index = content
                .overheard
                .iter()
                .position(|d| d.id == l.id)
                .unwrap_or(usize::MAX) as u64;
            let mut rng = st.rng.keyed(
                Stream::Dialogue,
                &[
                    0x0E4D_4EAD,
                    m as u64,
                    h as u64,
                    week,
                    location_key(loc),
                    index,
                ],
            );
            let u = rng.next_f64().max(1e-12);
            (mandate_core::math::ln(u) / w, l)
        })
        .collect();
    // Best first; ties by registry order (the sort is stable).
    scored.sort_by(|a, b| b.0.total_cmp(&a.0));
    let mut out: Vec<HeardLine> = Vec::new();
    let mut concerns: Vec<&ContentKey> = Vec::new();
    // Pass 1: fresh lines, one per concern; pass 2: fresh lines; pass 3: any.
    for pass in 0..3 {
        for (_, l) in &scored {
            if out.len() >= n {
                break;
            }
            if out.iter().any(|x| x.id == l.id) {
                continue;
            }
            if pass < 2 && avoid.contains(&l.id) {
                continue;
            }
            let c = concern_of(content, &l.id);
            if pass == 0 && c.is_some_and(|c| concerns.contains(&c)) {
                continue;
            }
            if let Some(c) = c {
                concerns.push(c);
            }
            out.push(l.clone());
        }
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

/// What this week's lines at a place let the player hear of (DEC-091,
/// §1.3): each drawn line's `teaches` subjects, with the line, in draw
/// order. Lines have no cast: a neighbourhood or a place is where the line
/// is heard. Read-only and draws nothing new (the week's draw is keyed).
pub fn teachings(
    st: &State,
    content: &ContentBundle,
    loc: Location,
) -> Vec<(ContentKey, crate::player::knowledge::Subject)> {
    let here = hood_of(st, loc);
    let drawn = lines(st, content, loc);
    let r = crate::scheduler::Reads::of_state(st, content);
    let v = crate::narrative::cast::View::new(&r, &st.politics);
    let mut out = Vec::new();
    for l in drawn {
        let Some(def) = content.overheard.iter().find(|d| d.id == l.id) else {
            continue;
        };
        for t in &def.teaches {
            for s in crate::narrative::learn::subjects(&v, t, &[], here) {
                out.push((l.id.clone(), s));
            }
        }
    }
    out
}

/// The player overhears this week's talk at the place they arrived at:
/// every subject a drawn line teaches is raised to `heard` (the player's
/// command writes the player's ledger). Returns how many subjects changed.
pub fn overhear(state: &mut State, content: &ContentBundle, loc: Location) -> usize {
    let Location::Place { place } = loc else {
        return 0;
    };
    let mut n = 0;
    for (line, subject) in teachings(state, content, loc) {
        let src = crate::player::knowledge::Source::Overheard { place, line };
        if crate::player::knowledge::teach_one(
            state,
            subject,
            crate::player::knowledge::Level::Heard,
            src,
        ) {
            n += 1;
        }
    }
    n
}

/// The Fluent key of a sector as residents name it (a noun phrase such as
/// "the factories"), else its name.
pub fn talk_key(s: &mandate_content::SectorDef) -> String {
    s.talk_key.clone().unwrap_or_else(|| s.name_key.clone())
}

/// Every line that holds at a location this week, in registry order
/// (before the draw): for tests, the lab, and content authors.
pub fn available(st: &State, content: &ContentBundle, loc: Location) -> Vec<HeardLine> {
    candidates(st, content, loc)
        .into_iter()
        .map(|(_, l)| l)
        .collect()
}

/// Does a single condition hold at a location (the one evaluator, with
/// the location's neighbourhood as `here`)?
pub fn condition_holds(st: &State, content: &ContentBundle, loc: Location, c: &Condition) -> bool {
    if hood_of(st, loc).is_none() {
        return false;
    }
    crate::condition::on_state(st, content, |v| {
        let cx = crate::condition::Ctx::at_location(v, st, loc);
        crate::condition::holds(&cx, c, &mut crate::condition::Args::new())
    })
}
