//! Where and as whom the player begins (DEC-098, DEC-101): the one start
//! rule. World generation seats the player with it in the default town
//! (`mandate-worldgen`), and an opening the draft chose (`arcs::opening`)
//! seats them with it in the opening's town. Both go through
//! [`choose_seat`] and [`take_seat`], so a start is the same wherever it
//! is made.
//!
//! The post comes from the constitution by id (a seat in the legislature
//! of the first career act's tier); no office or body is named here.

use super::{PlayerState, Tracks};
use crate::places::Location;
use crate::politics::{PoliticianId, PoliticsState, holder_of, members_of};
use crate::population::PopulationState;
use crate::world::{ConstituencyId, Jurisdiction, MunicipalityId, NeighborhoodId, World};
use mandate_content::{ConstitutionDef, ContentBundle, TierKey};

/// What the player starts as (EXTENSIBILITY_AUDIT §3.9): a constitution
/// post, by id, and which of its seats.
#[derive(Clone, Debug, PartialEq)]
pub struct Start {
    pub post: StartPost,
    pub constituency: Selector,
}

/// The post a start names, by constitution id.
#[derive(Clone, Debug, PartialEq)]
pub enum StartPost {
    /// A seat in this body.
    Body(String),
    /// This office.
    Office(String),
}

/// Which seat a start takes, by the last result's margin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Selector {
    Marginal,
    Safe,
    Any,
}

impl Start {
    /// The entry post: a seat in the legislature of the constitution's
    /// first career act's tier (the local one when the act names none), in
    /// a marginal district.
    pub fn sitting_member(c: &ConstitutionDef) -> Option<Start> {
        let tier = c
            .career
            .acts
            .first()
            .and_then(|a| a.tier)
            .unwrap_or(TierKey::Local);
        let body = crate::politics::legislature(c, tier)?;
        Some(Start {
            post: StartPost::Body(body.id.clone()),
            constituency: Selector::Marginal,
        })
    }

    /// The constitution's entry start for a state's rules.
    pub fn entry(state: &crate::State, content: &ContentBundle) -> Option<Start> {
        content
            .constitutions
            .get(&state.meta.constitution)
            .and_then(Start::sitting_member)
    }
}

/// The district a politician sits for, else the one their home is in.
fn district_of(
    world: &World,
    politics: &PoliticsState,
    town: MunicipalityId,
    id: PoliticianId,
) -> Option<ConstituencyId> {
    let p = politics.get(id)?;
    let home = NeighborhoodId(p.home);
    p.role.constituency().or_else(|| {
        Jurisdiction::town(town)
            .constituencies(world)
            .find(|c| c.neighborhoods.contains(&home))
            .map(|c| c.id)
    })
}

/// The seat a start takes in a town: among the post's holders there (the
/// members of the body, or the office's holder), those sitting for the
/// district that holds `near` if any do, else all of them; of those, the
/// member of a major party by the selector's margin (the first election
/// is winnable and losable: drama, DEC-059), else the middle one.
pub fn choose_seat(
    world: &World,
    politics: &PoliticsState,
    start: &Start,
    town: MunicipalityId,
    near: Option<NeighborhoodId>,
) -> Option<PoliticianId> {
    let at = Jurisdiction::town(town);
    let margin_of = |id: PoliticianId| -> f32 {
        politics
            .elections
            .results
            .iter()
            .rev()
            .find(|r| r.winner == id)
            .map(|r| {
                let mut s: Vec<f32> = r.candidates.iter().map(|c| c.share).collect();
                s.sort_by(|a, b| b.total_cmp(a));
                s[0] - s.get(1).copied().unwrap_or(0.0)
            })
            .unwrap_or(0.5)
    };
    let all: Vec<PoliticianId> = match &start.post {
        StartPost::Body(id) => members_of(politics, id, at).to_vec(),
        StartPost::Office(id) => holder_of(politics, id, at).into_iter().collect(),
    };
    let members: Vec<PoliticianId> = match near {
        Some(h) => {
            let theirs: Vec<PoliticianId> = all
                .iter()
                .copied()
                .filter(|id| {
                    district_of(world, politics, town, *id)
                        .and_then(|c| world.constituencies.get(c.index()))
                        .is_some_and(|c| c.neighborhoods.contains(&h))
                })
                .collect();
            if theirs.is_empty() { all } else { theirs }
        }
        None => all,
    };
    let major = |id: &PoliticianId| {
        politics
            .get(*id)
            .is_some_and(|p| p.party.is_some_and(|x| x.0 < 2))
    };
    let by_margin = |a: &PoliticianId, b: &PoliticianId| margin_of(*a).total_cmp(&margin_of(*b));
    let picked = match start.constituency {
        Selector::Marginal => members.iter().copied().filter(major).min_by(by_margin),
        Selector::Safe => members.iter().copied().filter(major).max_by(by_margin),
        Selector::Any => None,
    };
    picked.or_else(|| members.get(members.len() / 2).copied())
}

/// Seat the player as `seat` in `town`: the politician becomes the
/// player's (the start's stats: 38 years old in `year`, a first term three
/// years in, a modest record), the voters of their district (of the whole
/// town, for an office) know them a little, and a new [`PlayerState`] is
/// returned with the start's resources. Its ledger is empty: it is seeded
/// once the player is in the state (`knowledge::seed`).
pub fn take_seat(
    world: &World,
    politics: &mut PoliticsState,
    population: &mut PopulationState,
    content: &ContentBundle,
    seat: PoliticianId,
    town: MunicipalityId,
    year: u32,
) -> Option<PlayerState> {
    let ward = district_of(world, politics, town, seat)?;
    let me = politics.get_mut(seat)?;
    me.is_player = true;
    me.oratory = 0.6;
    me.negotiation = 0.6;
    me.birth_year = year - 38;
    me.years_in_office = 3;
    me.terms_in_office = 1;
    me.scandal = 0.0;
    // A modest first-term record: the player must build their own.
    me.record = 0.3;
    let known: Vec<NeighborhoodId> = match me.role.post() {
        Some(p) if p.is_office() => world
            .hoods_of(town)
            .map(|h| NeighborhoodId(h as u32))
            .collect(),
        _ => world
            .constituencies
            .get(ward.index())
            .map(|c| c.neighborhoods.clone())
            .unwrap_or_default(),
    };
    for h in known {
        if let Some(hood) = population.hoods.get_mut(h.index()) {
            for c in hood.cohorts.iter_mut() {
                c.player_awareness = 0.3;
                c.player_approval = 0.1;
            }
        }
    }
    Some(PlayerState {
        politician: seat,
        municipality: town,
        ward,
        standard_slots: content.tuning.standard_slots,
        slots_used: 0,
        late_night_used: false,
        mandate: 50.0,
        money: mandate_core::Money::from_units(5_000),
        staff: 1,
        tracks: Tracks {
            viability: 50.0,
            legal_jeopardy: 0.0,
            threat: 5.0,
            standing: 60.0,
            health: 85.0,
        },
        actions: Vec::new(),
        filing: None,
        ads_this_turn: 0,
        at_events: Vec::new(),
        last_election_seen: None,
        location: Location::Home,
        status: Default::default(),
        // Seeded from a start profile when play begins (DEC-091).
        knowledge: Default::default(),
        // Nobody has talked with the player yet (DEC-093).
        talk: Default::default(),
        // No doorsteps knocked yet (DEC-097 §1).
        ground: Default::default(),
    })
}

/// Seat the player afresh in `town` (an opening the draft chose): the
/// start's seat there ([`choose_seat`], for the district holding `near`
/// if one does), taken by the start rule ([`take_seat`]). Whoever the
/// player was before is an ordinary politician again, and what the voters
/// knew of them is forgotten; the new player's ledger is seeded from the
/// same start profile and option, their policy decks dealt, their day
/// chosen.
pub fn seat_in(
    state: &mut crate::State,
    content: &ContentBundle,
    town: MunicipalityId,
    near: Option<NeighborhoodId>,
) -> Result<(), String> {
    let start = Start::entry(state, content).ok_or("the constitution has no entry post")?;
    let seat = choose_seat(&state.world, &state.politics, &start, town, near)
        .ok_or("no seat to take in the town")?;
    let year = state.calendar.date.calendar().year;
    let old = state.player.as_ref().ok_or("no player")?;
    let (profile, local) = match &old.knowledge.origin {
        super::knowledge::LedgerOrigin::LocalOption { profile } => (Some(profile.clone()), true),
        super::knowledge::LedgerOrigin::Profile { profile } => (Some(profile.clone()), false),
        _ => (None, false),
    };
    let lab = old.knowledge.lab_omniscient;
    let was = old.politician;
    if let Some(p) = state.politics.get_mut(was) {
        p.is_player = false;
    }
    for hood in state.population.hoods.iter_mut() {
        for c in hood.cohorts.iter_mut() {
            c.player_awareness = 0.0;
            c.player_approval = 0.0;
        }
    }
    let mut pl = take_seat(
        &state.world,
        &mut state.politics,
        &mut state.population,
        content,
        seat,
        town,
        year,
    )
    .ok_or("the seat has no district")?;
    pl.knowledge.lab_omniscient = lab;
    state.player = Some(pl);
    super::knowledge::seed(state, content, profile.as_deref(), local)?;
    crate::policy::families::on_load(state, content);
    state.calendar.day = crate::characters::calendar::choose_day(state, content);
    Ok(())
}
