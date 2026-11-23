//! What a speaker knows, and what they will tell the player (DEC-093;
//! M6_ENGINE_CONTRACTS §3.4).
//!
//! **NPCs keep no ledgers** (DEC-091 §1.10). What someone knows is derived
//! from the content rules (`talk_params.speaker_knows`: who knows what, to
//! what level) and the live state, so nothing is stored and nothing is
//! added to saves.
//!
//! **Discretion** caps what they tell: their opinion of the player sets a
//! band (`talk_params.discretion`), the friendliest tell the most only with
//! enough trust, traits shift the cap by whole levels (schemers and the
//! cautious one more guarded; the vain one more open when flattered), and a
//! favour they owe the player, called in, opens them up by one. They tell
//! `min(what they know, the cap)`.

use super::CharacterId;
use crate::player::knowledge::{Level, Subject};
use crate::state::State;
use crate::world::MunicipalityId;
use mandate_content::ContentBundle;
use mandate_content::conversation::{KnownSubject, KnowsWho};

/// A speaker's role for topics and fallback lines: their occupation id, or
/// their post's role word from the constitution (a body's `member_key`, an
/// office's id; DEC-098). `None`: a politician out of office.
pub fn role_key(state: &State, content: &ContentBundle, id: CharacterId) -> Option<String> {
    let c = state.politics.characters.get(id)?;
    if let Some(p) = c.politician.and_then(|p| state.politics.get(p)) {
        let constitution = content.constitutions.get(&state.meta.constitution)?;
        return crate::politics::role_word(constitution, p).map(str::to_string);
    }
    c.named.as_deref().map(|n| n.occupation.clone())
}

/// What a name tag, list row or card says someone is (the one label rule).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RoleLabel {
    /// Holds a post: the constitution's role word (`councillor`, ...).
    Post(String),
    /// Stands in a running race.
    Candidate,
    /// Out of office: their trade, as an occupation's name key.
    Trade(String),
    /// Out of office with no trade: only ever the player's own standing.
    OutOfOffice,
}

/// A politician out of office has no occupation in state (and saves do not
/// change), so their trade is derived: stable for a character, drawn from the
/// non-leader occupations that share their pet issue when any do.
pub fn trade_of<'a>(
    state: &State,
    content: &'a ContentBundle,
    id: CharacterId,
) -> Option<&'a mandate_content::OccupationDef> {
    let c = state.politics.characters.get(id)?;
    if let Some(n) = c.named.as_deref() {
        return content.occupation(&n.occupation);
    }
    let mut pool: Vec<&_> = content
        .occupations
        .iter()
        .filter(|o| !o.leader && o.expertise.as_deref() == Some(c.expertise.as_str()))
        .collect();
    if pool.is_empty() {
        pool = content.occupations.iter().filter(|o| !o.leader).collect();
    }
    if pool.is_empty() {
        return None;
    }
    let h = (u64::from(id.0) + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    pool.get(((h >> 33) as usize) % pool.len()).copied()
}

/// The single rule for labelling a person: their post, else a running
/// race, else their trade. "Out of office" is left for the player's own
/// standing, never a passer-by's (owner note, 2026-11-21).
pub fn role_label(state: &State, content: &ContentBundle, id: CharacterId) -> RoleLabel {
    let Some(c) = state.politics.characters.get(id) else {
        return RoleLabel::OutOfOffice;
    };
    if let Some(p) = c.politician.and_then(|p| state.politics.get(p)) {
        let word = content
            .constitutions
            .get(&state.meta.constitution)
            .and_then(|k| crate::politics::role_word(k, p));
        if let Some(w) = word {
            return RoleLabel::Post(w.to_string());
        }
        if super::is_candidate(&state.politics, p.id) {
            return RoleLabel::Candidate;
        }
        if p.is_player {
            return RoleLabel::OutOfOffice;
        }
    }
    match trade_of(state, content, id) {
        Some(o) => RoleLabel::Trade(o.name_key.clone()),
        None => RoleLabel::OutOfOffice,
    }
}

/// The town a subject is about (`None`: no town's local knowledge covers it).
fn subject_town(state: &State, s: &Subject) -> Option<MunicipalityId> {
    match s {
        Subject::TownConcerns { town }
        | Subject::Broker { town }
        | Subject::RivalPlans { town }
        | Subject::PolicySupport { town, .. }
        | Subject::Situational { town, .. }
        | Subject::Layer { town, .. }
        | Subject::Sector { town, .. } => Some(*town),
        Subject::Site { place } | Subject::Service { place } => state
            .world
            .places
            .get(place.index())
            .map(|p| p.municipality),
        Subject::HoodConcerns { hood } => state
            .world
            .neighborhoods
            .get(hood.index())
            .map(|h| h.municipality),
        Subject::Person { person } => {
            super::municipality_of(&state.politics, &state.world, *person).map(MunicipalityId)
        }
        Subject::Place { place } => state
            .world
            .places
            .get(place.index())
            .map(|p| p.municipality),
        Subject::Proposal { .. } | Subject::Secret { .. } => None,
    }
}

/// Whether a `who` rule describes the speaker (in town `town`).
fn is_who(
    state: &State,
    content: &ContentBundle,
    speaker: CharacterId,
    who: KnowsWho,
    town: MunicipalityId,
) -> bool {
    let Some(c) = state.politics.characters.get(speaker) else {
        return false;
    };
    let named = c.named.as_deref();
    let occupation = named.and_then(|n| content.occupation(&n.occupation));
    let tc = state.politics.characters.towns.get(town.index());
    match who {
        KnowsWho::AnyoneLocal => true,
        KnowsWho::Leader => occupation.is_some_and(|o| o.leader),
        KnowsWho::StakeOverlap => {
            let Some(o) = occupation else {
                return false;
            };
            let driver = tc
                .and_then(|t| t.industry.as_deref())
                .and_then(|s| content.sector(s))
                .map(|s| s.driver);
            let crisis = tc.and_then(|t| t.crisis()).map(|x| x.kind.as_str());
            driver.is_some_and(|d| o.stake.drivers.contains(&d))
                || crisis.is_some_and(|k| o.stake.crises.iter().any(|x| x == k))
        }
        KnowsWho::Broker => tc.and_then(|t| t.broker) == Some(speaker),
        KnowsWho::Politician => c.politician.is_some(),
        // Sponsors arrive with policy families (F-series): nobody yet.
        KnowsWho::Sponsor => false,
        KnowsWho::Workplace => named.is_some_and(|n| n.place.is_some()),
    }
}

/// Whether a rule's subject covers a ledger subject, for this speaker.
fn covers(state: &State, speaker: CharacterId, what: KnownSubject, s: &Subject) -> bool {
    let ch = &state.politics.characters;
    match (what, s) {
        (KnownSubject::TownConcerns, Subject::TownConcerns { .. })
        | (KnownSubject::Broker, Subject::Broker { .. })
        | (KnownSubject::RivalPlans, Subject::RivalPlans { .. })
        | (KnownSubject::PolicySupport, Subject::PolicySupport { .. }) => true,
        (KnownSubject::HomeConcerns, Subject::HoodConcerns { hood }) => {
            super::profile(&state.politics, &state.world, speaker).is_some_and(|p| p.home == hood.0)
        }
        (KnownSubject::Council, Subject::Person { person }) => ch
            .get(*person)
            .and_then(|c| c.politician)
            .and_then(|p| state.politics.get(p))
            .is_some_and(|p| p.in_office()),
        (KnownSubject::WorkplacePeople, Subject::Person { person }) => {
            let place = |id: CharacterId| ch.get(id).and_then(|c| c.named.as_deref()?.place);
            *person != speaker && place(speaker).is_some() && place(speaker) == place(*person)
        }
        (KnownSubject::Themselves, Subject::Person { person }) => *person == speaker,
        // Proposals arrive with policy families (F-series).
        (KnownSubject::SponsoredProposals | KnownSubject::WorkplaceProposals, _) => false,
        _ => false,
    }
}

/// What a speaker knows of a subject (derived from `talk_params.speaker_knows`;
/// never stored). Local knowledge only: a subject of another town is unknown.
pub fn speaker_knows(
    state: &State,
    content: &ContentBundle,
    speaker: CharacterId,
    s: &Subject,
) -> Level {
    let Some(town) = subject_town(state, s) else {
        return Level::Unknown;
    };
    if super::municipality_of(&state.politics, &state.world, speaker) != Some(town.0) {
        return Level::Unknown;
    }
    content
        .talk
        .speaker_knows
        .iter()
        .filter(|r| covers(state, speaker, r.subject, s))
        .filter(|r| is_who(state, content, speaker, r.who, town))
        .map(|r| r.level)
        .max()
        .unwrap_or_default()
}

/// What shifts a speaker's discretion beyond the relationship.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Openness {
    /// The player made small talk with them earlier this week.
    pub flattered: bool,
    /// A favour they owe the player is being called in.
    pub favour_called: bool,
}

fn shift(l: Level, by: i32) -> Level {
    let i = Level::ALL.iter().position(|x| *x == l).unwrap_or(0) as i32;
    Level::ALL[(i + by).clamp(0, Level::ALL.len() as i32 - 1) as usize]
}

/// The most a speaker tells the player (§3.4): the opinion band's level
/// (the friendliest's `top` only with trust; else one less), shifted by
/// traits, flattery and a called-in favour.
pub fn discretion_cap(
    state: &State,
    content: &ContentBundle,
    speaker: CharacterId,
    player: CharacterId,
    open: Openness,
) -> Level {
    let t = &content.talk;
    let d = &t.discretion;
    let op = super::opinion(&state.politics, speaker, player) as f64;
    let trust = super::trust(&state.politics, content, speaker, player) as f64;
    let base = match d.bands.iter().find(|b| op < b.below) {
        Some(b) => b.tells,
        None if trust >= d.trust_for_detailed => d.top,
        None => shift(d.top, -1),
    };
    let traits = state
        .politics
        .characters
        .get(speaker)
        .map(|c| c.traits.as_slice())
        .unwrap_or(&[]);
    let mut by: i32 = traits
        .iter()
        .filter_map(|x| t.trait_discretion.get(x))
        .map(|v| *v as i32)
        .sum();
    if open.flattered {
        by += traits
            .iter()
            .filter_map(|x| t.flattered_discretion.get(x))
            .map(|v| *v as i32)
            .sum::<i32>();
    }
    if open.favour_called {
        by += t.favour_discretion as i32;
    }
    shift(base, by)
}

/// What a speaker would tell the player of a subject: what they know,
/// capped by their discretion.
pub fn tells(
    state: &State,
    content: &ContentBundle,
    speaker: CharacterId,
    s: &Subject,
    cap: Level,
) -> Level {
    speaker_knows(state, content, speaker, s).min(cap)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shifting_clamps_to_the_levels() {
        assert_eq!(shift(Level::Known, -1), Level::Heard);
        assert_eq!(shift(Level::Heard, -3), Level::Unknown);
        assert_eq!(shift(Level::Known, 2), Level::Detailed);
        assert_eq!(shift(Level::Detailed, 0), Level::Detailed);
    }
}
