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
//!
//! **Officeholders and workers** (M6 note 19): someone holding a post knows
//! every neighbourhood of its jurisdiction by report (the `officeholder`
//! rules: the letters and the casework tell them the complaint plainly, not
//! the street-by-street detail a resident has), and someone who works in a
//! neighbourhood hears what is said there (`works_there`). The levels are
//! content; who counts is read from the constitution's posts, never by name.

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

/// Where someone works, in town `town`: their own place (a named
/// character's), else where their post sits (DEC-098). `None` for someone
/// with neither.
pub fn workplace(
    state: &State,
    c: CharacterId,
    town: MunicipalityId,
) -> Option<&crate::world::Place> {
    let ch = state.politics.characters.get(c)?;
    if let Some(p) = ch
        .named
        .as_deref()
        .and_then(|n| n.place)
        .and_then(|p| state.world.places.get(p.index()))
    {
        return Some(p);
    }
    let kind = ch
        .politician
        .and_then(|p| state.politics.get(p))
        .and_then(|p| p.role.post())
        .and_then(|post| crate::politics::workplace_of(&state.politics, post))?;
    state
        .world
        .places
        .iter()
        .find(|p| p.municipality == town && p.kind == kind)
}

/// The jurisdiction of the post someone holds (`None`: no post).
fn post_at(state: &State, c: CharacterId) -> Option<crate::world::Jurisdiction> {
    state
        .politics
        .characters
        .get(c)?
        .politician
        .and_then(|p| state.politics.get(p))
        .and_then(|p| p.role.post())
        .map(|post| post.at())
}

/// Whether someone holds a post whose jurisdiction (and so its districts)
/// covers this neighbourhood (DEC-098).
pub fn holds_post_over(state: &State, c: CharacterId, hood: crate::world::NeighborhoodId) -> bool {
    post_at(state, c).is_some_and(|at| at.contains_hood(&state.world, hood))
}

/// Whether someone holds a post whose jurisdiction covers this town.
fn holds_post_over_town(state: &State, c: CharacterId, town: MunicipalityId) -> bool {
    use crate::world::Jurisdiction as J;
    match post_at(state, c) {
        Some(J::Municipality { id }) => id == town,
        Some(J::Region { id }) => state
            .world
            .municipalities
            .get(town.index())
            .is_some_and(|m| m.region == id),
        Some(J::Nation) => true,
        None => false,
    }
}

/// Whether someone works in this neighbourhood (their workplace is there).
pub fn works_in(state: &State, c: CharacterId, hood: crate::world::NeighborhoodId) -> bool {
    let Some(town) = state
        .world
        .neighborhoods
        .get(hood.index())
        .map(|h| h.municipality)
    else {
        return false;
    };
    workplace(state, c, town).is_some_and(|p| p.neighborhood == hood)
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

/// Whether a `who` rule describes the speaker (in town `town`, on subject `s`).
fn is_who(
    state: &State,
    content: &ContentBundle,
    speaker: CharacterId,
    who: KnowsWho,
    town: MunicipalityId,
    s: &Subject,
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
        KnowsWho::Officeholder => match s {
            Subject::HoodConcerns { hood } => holds_post_over(state, speaker, *hood),
            _ => holds_post_over_town(state, speaker, town),
        },
        KnowsWho::WorksThere => match s {
            Subject::HoodConcerns { hood } => works_in(state, speaker, *hood),
            _ => false,
        },
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
        (KnownSubject::HoodConcerns, Subject::HoodConcerns { .. }) => true,
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
/// never stored). Local knowledge only: a subject of another town is unknown,
/// except to someone holding a post over it (the `officeholder` rules).
pub fn speaker_knows(
    state: &State,
    content: &ContentBundle,
    speaker: CharacterId,
    s: &Subject,
) -> Level {
    knows_by(state, content, speaker, s, false)
}

/// A `who` rule that cannot be seen from outside: who the power broker is
/// is itself something to find out. Every other rule reads public facts (an
/// occupation, a post, a workplace, a home).
fn hidden(who: KnowsWho) -> bool {
    matches!(who, KnowsWho::Broker)
}

/// What public facts say a person would know of a subject (M6 note 26): the
/// same rules as [`speaker_knows`], without those nobody could see from
/// outside ([`hidden`]) and before any discretion. This orders the talk
/// panel, so the panel never shows what someone secretly knows, or what
/// they are willing to say.
pub fn expected_knows(
    state: &State,
    content: &ContentBundle,
    speaker: CharacterId,
    s: &Subject,
) -> Level {
    knows_by(state, content, speaker, s, true)
}

fn knows_by(
    state: &State,
    content: &ContentBundle,
    speaker: CharacterId,
    s: &Subject,
    public_only: bool,
) -> Level {
    let Some(town) = subject_town(state, s) else {
        return Level::Unknown;
    };
    let local = super::municipality_of(&state.politics, &state.world, speaker) == Some(town.0);
    content
        .talk
        .speaker_knows
        .iter()
        .filter(|r| !(public_only && hidden(r.who)))
        .filter(|r| local || r.who == KnowsWho::Officeholder)
        .filter(|r| covers(state, speaker, r.subject, s))
        .filter(|r| is_who(state, content, speaker, r.who, town, s))
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
