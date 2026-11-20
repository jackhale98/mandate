//! Conversations (DEC-093; M6_ENGINE_CONTRACTS §3): the player talks to
//! someone here, on a topic, sent as one command (`PlayerCommand::Talk`).
//! The engine composes the reply from content lines (`reply`) under the
//! shared condition type (`context`), and applies what it does at once
//! through the owning domains (the command phase, OVERVIEW §4.3): the
//! ledger learns with the person as the source (`Effect::Learn`, capped at
//! what they will tell), leads open (`Effect::Lead`), relationships move
//! (`characters`), and the topic's mechanic runs through the existing
//! lobbying, approval and favour paths.
//!
//! **Time:** a conversation costs half a slot (two fill one; late at night
//! it uses the late slot, where lobbying late is allowed). "Say hello"
//! (`Meet`) stays free.
//!
//! **Asking again** the same week gives another line (the keyed draw and
//! the repetition guard) and teaches nothing new unless the relationship
//! changed (the speaker's discretion rose); a mechanic does not run twice.
//!
//! Determinism: line draws are keyed on the dialogue stream and never
//! advance it; contested mechanics roll on the player stream like
//! lobbying. No prose: Fluent keys and facts (DEC-054).

pub mod context;
pub mod reply;
pub mod state;

pub use state::{Exchange, ExchangeId, SpokenLine, TalkState, UsedLine};

use crate::actions::ActionResult;
use crate::characters::knows::{self, Openness};
use crate::characters::{self, CharacterId, MemoryKind};
use crate::commands::EngineError;
use crate::places::{Location, Offer};
use crate::player::knowledge::{self, FamilyInstanceId, Lead, LeadTarget, Level, Source, Subject};
use crate::policy::{PolicyInstanceId, PolicyStatus};
use crate::politics::PoliticianId;
use crate::scheduler::Effect;
use crate::state::State;
use crate::world::{NeighborhoodId, PlaceId};
use context::{Args, Ctx};
use mandate_content::conversation::{
    AboutKind, LeadToDef, TalkCost, TalkEffectDef, TalkMechanic, TopicDef,
};
use mandate_content::{ContentBundle, ContentKey, TierKey};
use mandate_core::math::{clamp, sigmoid};
use mandate_core::{SimDate, Stream};
use serde::{Deserialize, Serialize};

/// Whom the player talks to: someone stored, or a passer-by (promoted to a
/// Named character, as `Meet` does).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TalkTarget {
    Person { person: CharacterId },
    Ambient { slot: u8 },
}

/// What a conversation is about, for topics that take a subject.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AboutRef {
    Place { place: PlaceId },
    Person { person: CharacterId },
    Hood { hood: NeighborhoodId },
    Bill { policy: PolicyInstanceId },
    Template { template: ContentKey },
    Proposal { proposal: FamilyInstanceId },
    Rival,
}

impl AboutRef {
    pub fn kind(&self) -> AboutKind {
        match self {
            AboutRef::Place { .. } => AboutKind::Place,
            AboutRef::Person { .. } => AboutKind::Person,
            AboutRef::Hood { .. } => AboutKind::Hood,
            AboutRef::Bill { .. } => AboutKind::Bill,
            AboutRef::Template { .. } => AboutKind::Template,
            AboutRef::Proposal { .. } => AboutKind::Proposal,
            AboutRef::Rival => AboutKind::Rival,
        }
    }
}

/// The week of a date (the repetition guard's period).
pub fn week_of(d: SimDate) -> u32 {
    d.0 / 7
}

fn bad(msg: &str) -> EngineError {
    EngineError::InvalidCommand(msg.into())
}

/// The stored people the player can talk to where they are (not
/// themselves): whoever's daily routine has them here now (DEC-031).
pub fn present(state: &State, content: &ContentBundle) -> Vec<CharacterId> {
    let Some(pl) = state.player.as_ref() else {
        return Vec::new();
    };
    let me = characters::player_character(state);
    characters::ambient::stored_at(state, content, pl.location)
        .into_iter()
        .filter(|c| Some(*c) != me)
        .collect()
}

/// Whether the player can talk to this person now (they are here).
pub fn can_talk(state: &State, content: &ContentBundle, person: CharacterId) -> bool {
    let Some(pl) = state.player.as_ref() else {
        return false;
    };
    Some(person) != characters::player_character(state)
        && characters::routines::whereabouts(state, content, person) == Some(pl.location)
}

/// Whether talking late at night is possible here (where lobbying late is).
pub fn late_ok(state: &State, content: &ContentBundle) -> bool {
    let Some(pl) = state.player.as_ref() else {
        return false;
    };
    crate::places::locations_for(
        state,
        content,
        Offer::Action {
            key: "lobby",
            late_night: true,
        },
        None,
    )
    .contains(&pl.location)
}

/// What a conversation costs, in half slots (1: half a slot).
pub fn cost_halves(content: &ContentBundle) -> u32 {
    match content.talk.cost {
        TalkCost::HalfSlot => 1,
        TalkCost::Slot => 2,
    }
}

/// Whether the player has the time for a conversation now.
fn check_cost(state: &State, content: &ContentBundle, late_night: bool) -> Result<(), EngineError> {
    let pl = state.player.as_ref().ok_or_else(|| bad("no player"))?;
    if late_night {
        if pl.late_night_used {
            return Err(bad("late night already used this turn"));
        }
        return Ok(());
    }
    let half_left =
        content.talk.cost == TalkCost::HalfSlot && pl.talk.half_slot == Some(state.calendar.turn);
    if !half_left && pl.slots_left() < 1 {
        return Err(bad("not enough time left this turn"));
    }
    Ok(())
}

/// Pay for a conversation (checked first by `check_cost`).
fn pay(state: &mut State, content: &ContentBundle, late_night: bool) {
    let turn = state.calendar.turn;
    let health = content.actions.late_night_health as f32;
    let Some(pl) = state.player.as_mut() else {
        return;
    };
    if late_night {
        pl.late_night_used = true;
        pl.tracks.health = (pl.tracks.health - health).max(0.0);
        return;
    }
    match content.talk.cost {
        TalkCost::HalfSlot if pl.talk.half_slot == Some(turn) => pl.talk.half_slot = None,
        TalkCost::HalfSlot => {
            pl.slots_used += 1;
            pl.talk.half_slot = Some(turn);
        }
        TalkCost::Slot => pl.slots_used += 1,
    }
}

/// Is the subject a fair one to raise in this topic? (Exists, in the
/// player's town, and a person the player has at least heard of.)
fn check_about(
    state: &State,
    content: &ContentBundle,
    topic: &TopicDef,
    about: Option<&AboutRef>,
) -> Result<(), EngineError> {
    let pl = state.player.as_ref().ok_or_else(|| bad("no player"))?;
    let town = pl.municipality;
    let Some(a) = about else {
        if topic.about_required {
            return Err(bad("this topic needs a subject"));
        }
        return Ok(());
    };
    if !topic.about.contains(&a.kind()) {
        return Err(bad("that is not something to raise on this topic"));
    }
    let ok = match a {
        AboutRef::Place { place } => state
            .world
            .places
            .get(place.index())
            .is_some_and(|p| p.municipality == town),
        AboutRef::Person { person } => {
            Some(*person) != characters::player_character(state)
                && state.politics.characters.get(*person).is_some()
                && knowledge::knows(state, &Subject::Person { person: *person }) >= Level::Heard
        }
        AboutRef::Hood { hood } => state
            .world
            .neighborhoods
            .get(hood.index())
            .is_some_and(|h| h.municipality == town),
        AboutRef::Bill { policy } => state
            .policy
            .instances
            .get(policy.index())
            .is_some_and(|p| p.jurisdiction == town.0 && p.status != PolicyStatus::Ended),
        AboutRef::Template { template } => content
            .policies
            .get(template)
            .is_some_and(|t| t.tier == TierKey::Local),
        // Policy families (F-series) bring proposals.
        AboutRef::Proposal { .. } => false,
        AboutRef::Rival => state
            .politics
            .characters
            .towns
            .get(town.index())
            .is_some_and(|t| t.rival.is_some()),
    };
    if ok {
        Ok(())
    } else {
        Err(bad("you know of nothing like that in your town"))
    }
}

/// What a topic's mechanic will do, decided before anything is spent.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Mech {
    None,
    SmallTalk,
    /// Sway a colleague on a bill (the lobbying formula).
    Pitch {
        target: PoliticianId,
        policy: PolicyInstanceId,
    },
    PitchNoted,
    /// A colleague asked to back a bill (the lobbying odds).
    Back {
        target: PoliticianId,
        policy: PolicyInstanceId,
    },
    /// Anyone else asked for a public endorsement (of the player or a bill).
    Endorse {
        policy: Option<PolicyInstanceId>,
    },
    CallIn {
        debtor: PoliticianId,
        policy: PolicyInstanceId,
    },
    Offer {
        target: PoliticianId,
        policy: PolicyInstanceId,
    },
    FavourNone,
}

/// A colleague: the holder of a post whose jurisdiction is in the player's
/// focus (not the player; DEC-098).
fn colleague(state: &State, c: CharacterId) -> Option<PoliticianId> {
    let focus = state.focus();
    let p = state
        .politics
        .characters
        .get(c)?
        .politician
        .and_then(|p| state.politics.get(p))?;
    (!p.is_player && p.role.post().is_some_and(|post| focus.contains(&post.at()))).then_some(p.id)
}

fn plan(
    state: &State,
    topic: &TopicDef,
    speaker: CharacterId,
    me: CharacterId,
    about: Option<&AboutRef>,
) -> Result<Mech, EngineError> {
    let bill = match about {
        Some(AboutRef::Bill { policy }) => Some(*policy),
        _ => None,
    };
    let peer = colleague(state, speaker);
    Ok(match topic.mechanic {
        TalkMechanic::Inform => Mech::None,
        TalkMechanic::SmallTalk => Mech::SmallTalk,
        TalkMechanic::Pitch => match (peer, bill) {
            (Some(target), Some(policy)) => Mech::Pitch { target, policy },
            _ => Mech::PitchNoted,
        },
        TalkMechanic::AskSupport => match (peer, bill) {
            (Some(target), Some(policy)) => Mech::Back { target, policy },
            _ => Mech::Endorse { policy: bill },
        },
        TalkMechanic::Favour => match (peer, bill) {
            (Some(p), Some(policy))
                if characters::favours_owed(&state.politics, speaker, me) > 0 =>
            {
                Mech::CallIn { debtor: p, policy }
            }
            (Some(target), Some(policy)) => {
                if characters::favours_owed(&state.politics, me, speaker) as i32
                    >= crate::actions::MAX_FAVOR_DEBT
                {
                    return Err(bad("you already owe them too much"));
                }
                Mech::Offer { target, policy }
            }
            _ => Mech::FavourNone,
        },
    })
}

/// Odds that someone who is not a colleague agrees to speak up (§3.2
/// `ask_support`): by their opinion of and trust in the player.
pub fn support_odds(state: &State, content: &ContentBundle, speaker: CharacterId) -> f64 {
    let Some(me) = characters::player_character(state) else {
        return 0.0;
    };
    let s = &content.talk.support;
    let op = characters::opinion(&state.politics, speaker, me) as f64;
    let tr = characters::trust(&state.politics, content, speaker, me) as f64;
    clamp(
        sigmoid(s.base + op / 100.0 * s.per_opinion + (tr - 0.5) * s.per_trust),
        s.min,
        s.max,
    )
}

/// The odds a contested topic shows before the player asks (DEC-073):
/// pitches and asks for support. `None` for the others.
pub fn odds(
    state: &State,
    content: &ContentBundle,
    speaker: CharacterId,
    topic: &str,
    about: Option<&AboutRef>,
    late_night: bool,
) -> Option<f64> {
    let t = content.talk_topic(topic)?;
    let me = characters::player_character(state)?;
    match plan(state, t, speaker, me, about).ok()? {
        Mech::Pitch { target, .. } | Mech::Back { target, .. } => Some(crate::actions::lobby_odds(
            state, content, target, late_night,
        )),
        Mech::Endorse { .. } => Some(support_odds(state, content, speaker)),
        _ => None,
    }
}

/// Whether the player asked this person about this topic this week.
pub fn asked_this_week(state: &State, speaker: CharacterId, topic: &str) -> bool {
    let Some(pl) = state.player.as_ref() else {
        return false;
    };
    pl.talk
        .this_week(speaker, week_of(state.calendar.date))
        .any(|e| e.topic == topic)
}

/// Every check the command makes, without changing anything (the talk
/// panel's "available"). For a passer-by, ask after `Meet`.
pub fn check(
    state: &State,
    content: &ContentBundle,
    speaker: CharacterId,
    topic: &str,
    about: Option<&AboutRef>,
    late_night: bool,
) -> Result<(), EngineError> {
    prepare(state, content, speaker, topic, about, late_night).map(|_| ())
}

/// What a conversation will say and do, worked out before anything changes.
struct Prepared {
    speaker: CharacterId,
    me: CharacterId,
    topic: ContentKey,
    mechanic: Mech,
    repeat: bool,
    learn: bool,
    cap: Level,
    parts: Vec<SpokenLine>,
    teach: Vec<(Subject, Level)>,
    leads: Vec<Lead>,
    effects: Vec<TalkEffectDef>,
    small_talks: usize,
    /// The odds shown before asking (`odds`), fixed before anything
    /// changes: the roll uses these, not odds recomputed after the reply
    /// taught or warmed anything (DEC-073: the odds shown are the odds
    /// rolled).
    odds: Option<f64>,
}

fn prepare(
    state: &State,
    content: &ContentBundle,
    speaker: CharacterId,
    topic_id: &str,
    about: Option<&AboutRef>,
    late_night: bool,
) -> Result<Prepared, EngineError> {
    let date = state.calendar.date;
    let week = week_of(date);
    let pl = state.player.as_ref().ok_or_else(|| bad("no player"))?;
    let me = characters::player_character(state).ok_or_else(|| bad("no player"))?;
    let topic = content
        .talk_topic(topic_id)
        .ok_or_else(|| bad("unknown topic"))?;
    if speaker == me || !can_talk(state, content, speaker) {
        return Err(bad("they are not here to talk to"));
    }
    if late_night && !late_ok(state, content) {
        return Err(bad(
            "late at night, people talk only where private meetings are held",
        ));
    }
    check_about(state, content, topic, about)?;
    let o = &topic.offered;
    if !o.roles.is_empty()
        && !knows::role_key(state, content, speaker).is_some_and(|r| o.roles.contains(&r))
    {
        return Err(bad("they are not the person to ask about that"));
    }
    if !o.places.is_empty()
        && !o
            .places
            .iter()
            .any(|k| k == crate::places::kind_key(state, pl.location))
    {
        return Err(bad("that is not talked about here"));
    }
    check_cost(state, content, late_night)?;
    let mechanic = plan(state, topic, speaker, me, about)?;
    // This week with this person: asking again, and flattery.
    let talk = &pl.talk;
    let earlier: Vec<&Exchange> = talk
        .this_week(speaker, week)
        .filter(|e| e.topic == topic.id)
        .collect();
    let n = earlier.len() as u32;
    let same: Vec<&&Exchange> = earlier
        .iter()
        .filter(|e| e.about.as_ref() == about)
        .collect();
    let repeat = !same.is_empty();
    let small_talks = talk
        .this_week(speaker, week)
        .filter(|e| {
            content
                .talk_topic(&e.topic)
                .is_some_and(|t| t.mechanic == TalkMechanic::SmallTalk)
        })
        .count();
    let open = Openness {
        flattered: small_talks > 0,
        favour_called: matches!(mechanic, Mech::CallIn { .. }),
    };
    let cap = knows::discretion_cap(state, content, speaker, me, open);
    let learn = match same.iter().map(|e| e.cap).max() {
        None => true,
        // The relationship changed since: they may tell more.
        Some(prev) => cap > prev,
    };
    let ctx = Ctx {
        st: state,
        content,
        town: pl.municipality,
        me,
        speaker,
        about,
        here: pl.location,
        cap,
    };
    if !ctx.all_hold(&o.when, &mut Args::new()) {
        return Err(bad("that is not something to ask them now"));
    }
    let composed = reply::compose(&ctx, topic, talk.used_by(speaker), week, n);
    let base = context_args(&ctx);
    let source = Source::Talk {
        person: speaker,
        topic: topic.id.clone(),
    };
    let mut parts = Vec::new();
    let mut teach: Vec<(Subject, Level)> = Vec::new();
    let mut leads = Vec::new();
    let mut effects = Vec::new();
    for c in &composed.parts {
        let mut args = base.clone();
        args.extend(c.args.clone());
        parts.push(SpokenLine {
            part: c.part,
            line: c.line.id.clone(),
            key: c.line.key.clone(),
            args: args.into_iter().collect(),
        });
        // What it teaches, capped at what the speaker will tell.
        for t in &c.line.teaches {
            let Some(s) = ctx.subject(&t.subject) else {
                continue;
            };
            let level = t.level.min(ctx.told(&s));
            if level > Level::Unknown {
                teach.push((s, level));
            }
        }
        if let Some(l) = &c.line.lead
            && let Some(subject) = ctx.subject(&l.subject)
            && let Some(to) = lead_target(&ctx, &l.to)
        {
            leads.push(Lead {
                subject,
                level: l.level,
                key: l.key.clone(),
                to,
                since: date,
                source: source.clone(),
            });
        }
        effects.extend(c.line.effects.iter().cloned());
    }
    // Talking to someone is meeting them properly.
    teach.push((Subject::Person { person: speaker }, Level::Known));
    Ok(Prepared {
        speaker,
        me,
        topic: topic.id.clone(),
        mechanic,
        repeat,
        learn,
        cap,
        parts,
        teach,
        leads,
        effects,
        small_talks,
        odds: odds(state, content, speaker, &topic.id, about, late_night),
    })
}

/// `$speaker`, `$player`, `$town`, `$here`, and `$about` / `$about_key`.
/// A place without a proper name is named by its kind, as a Fluent key
/// (`here_key`, `about_key`: "the union hall"; `Ctx::place_arg`).
pub fn context_args(ctx: &Ctx<'_>) -> Args {
    let st = ctx.st;
    let mut a = Args::new();
    a.insert("speaker".into(), ctx.name_of(ctx.speaker));
    a.insert("player".into(), ctx.name_of(ctx.me));
    if let Some(m) = st.world.municipalities.get(ctx.town.index()) {
        a.insert("town".into(), m.name.clone());
    }
    match ctx.here {
        Location::Place { place } => {
            if let Some(p) = st.world.places.get(place.index()) {
                ctx.place_arg(p, "here", &mut a);
            }
        }
        Location::Street { .. } | Location::Home => {
            if let Some(h) = crate::places::hood_of(st, ctx.here)
                .and_then(|h| st.world.neighborhoods.get(h.index()))
            {
                a.insert("here".into(), h.name.clone());
            }
        }
    }
    match ctx.about {
        Some(AboutRef::Place { place }) => {
            if let Some(p) = st.world.places.get(place.index()) {
                ctx.place_arg(p, "about", &mut a);
            }
        }
        Some(AboutRef::Person { person }) => {
            a.insert("about".into(), ctx.name_of(*person));
        }
        Some(AboutRef::Rival) => {
            if let Some(r) = ctx.rival() {
                a.insert("about".into(), ctx.name_of(r));
            }
        }
        Some(AboutRef::Hood { hood }) => {
            if let Some(h) = st.world.neighborhoods.get(hood.index()) {
                a.insert("about".into(), h.name.clone());
            }
        }
        Some(AboutRef::Bill { policy }) => {
            if let Some(t) = st
                .policy
                .instances
                .get(policy.index())
                .and_then(|p| crate::policy::resolve(ctx.content, &st.policy, p))
            {
                a.insert("about_key".into(), t.name_key.clone());
            }
        }
        Some(AboutRef::Template { template }) => {
            if let Some(t) = ctx.content.policies.get(template) {
                a.insert("about_key".into(), t.name_key.clone());
            }
        }
        Some(AboutRef::Proposal { .. }) | None => {}
    }
    a
}

fn lead_target(ctx: &Ctx<'_>, to: &LeadToDef) -> Option<LeadTarget> {
    match to {
        LeadToDef::PlaceKind { kind } => Some(LeadTarget::PlaceKind {
            town: ctx.town,
            kind: kind.clone(),
        }),
        LeadToDef::Person { slot } => ctx
            .person(slot)
            .filter(|p| *p != ctx.me)
            .map(|person| LeadTarget::Person { person }),
        LeadToDef::Streets { of } => ctx.hood(*of).map(|hood| LeadTarget::Streets { hood }),
    }
}

/// `PlayerCommand::Talk`: talk to someone here on a topic. Returns the new
/// exchange's id in `created`. Fails without changing anything.
pub fn apply(
    state: &mut State,
    content: &ContentBundle,
    to: &TalkTarget,
    topic: &str,
    about: Option<&AboutRef>,
    late_night: bool,
) -> Result<ActionResult, EngineError> {
    match to {
        TalkTarget::Person { person } => talk_to(state, content, *person, topic, about, late_night),
        TalkTarget::Ambient { slot } => {
            // Meeting a passer-by promotes them (as `Meet`); try it on a
            // copy so a refused conversation changes nothing.
            let loc = state
                .player
                .as_ref()
                .map(|p| p.location)
                .ok_or_else(|| bad("no player"))?;
            let mut trial = state.clone();
            let id = characters::ambient::promote(&mut trial, content, loc, *slot)
                .ok_or_else(|| bad("nobody like that is here now"))?;
            if let Some(me) = characters::player_character(&trial) {
                let date = trial.calendar.date;
                characters::remember(
                    &mut trial.politics,
                    content,
                    date,
                    id,
                    me,
                    MemoryKind::Met,
                    None,
                    None,
                );
            }
            let res = talk_to(&mut trial, content, id, topic, about, late_night)?;
            *state = trial;
            Ok(res)
        }
    }
}

fn talk_to(
    state: &mut State,
    content: &ContentBundle,
    speaker: CharacterId,
    topic: &str,
    about: Option<&AboutRef>,
    late_night: bool,
) -> Result<ActionResult, EngineError> {
    let p = prepare(state, content, speaker, topic, about, late_night)?;
    let date = state.calendar.date;
    let me = p.me;
    let before = characters::opinion(&state.politics, speaker, me);
    // Who is here as the conversation starts is noted (S1: last seen).
    knowledge::note_sightings(state, content);
    pay(state, content, late_night);
    // What it teaches and where it points: the player's ledger.
    let mut learned = Vec::new();
    let mut leads = Vec::new();
    if let Some(pl) = state.player.as_mut() {
        let source = Source::Talk {
            person: speaker,
            topic: p.topic.clone(),
        };
        for (subject, level) in &p.teach {
            if !p.learn && *subject != (Subject::Person { person: speaker }) {
                continue;
            }
            let e = Effect::Learn {
                subject: subject.clone(),
                level: *level,
                source: source.clone(),
            };
            if knowledge::apply_effect(&mut pl.knowledge, e, date, content) {
                learned.push((subject.clone(), *level));
            }
        }
        if p.learn {
            for lead in p.leads.iter().cloned() {
                if knowledge::apply_effect(
                    &mut pl.knowledge,
                    Effect::Lead { lead: lead.clone() },
                    date,
                    content,
                ) {
                    leads.push(lead);
                }
            }
        }
    }
    // The relationship: the lines' effects, once.
    if !p.repeat {
        for e in &p.effects {
            match e {
                TalkEffectDef::Opinion { delta } => {
                    characters::change_opinion(&mut state.politics, content, speaker, me, *delta);
                }
                TalkEffectDef::Remember { kind } => {
                    if let Some(k) = MemoryKind::from_key(kind) {
                        characters::remember(
                            &mut state.politics,
                            content,
                            date,
                            speaker,
                            me,
                            k,
                            None,
                            None,
                        );
                    }
                }
            }
        }
    }
    let name = characters::profile(&state.politics, &state.world, speaker)
        .map(|x| x.full_name())
        .unwrap_or_default();
    // The topic's mechanic, once a week per subject (small talk by its cap).
    let (outcome, outcome_args) = if p.repeat && p.mechanic != Mech::SmallTalk {
        (None, Vec::new())
    } else {
        mechanic(state, content, &p, late_night)
    };
    let after = characters::opinion(&state.politics, speaker, me);
    let place = match state.player.as_ref().map(|x| x.location) {
        Some(Location::Place { place }) => Some(place),
        _ => None,
    };
    let lines: Vec<ContentKey> = p.parts.iter().map(|s| s.line.clone()).collect();
    let t = &content.talk;
    let Some(pl) = state.player.as_mut() else {
        return Err(bad("no player"));
    };
    let id: ExchangeId = pl.talk.ids.alloc();
    pl.talk
        .record_used(speaker, &lines, date, t.memory_lines as usize);
    let key = match (&outcome, p.repeat) {
        (Some(k), _) => k.clone(),
        (None, true) => "outcome-talk-again".to_string(),
        (None, false) => "outcome-talk".to_string(),
    };
    pl.talk.push(
        Exchange {
            id,
            on: date,
            speaker,
            place,
            topic: p.topic.clone(),
            about: about.cloned(),
            parts: p.parts.clone(),
            learned,
            leads,
            cap: p.cap,
            repeat: p.repeat,
            opinion: after - before,
            outcome: outcome.clone(),
            outcome_args: outcome_args.clone(),
        },
        t.log_cap as usize,
    );
    let mut res = ActionResult {
        created: Some(id.0),
        outcome_key: key,
        ..Default::default()
    };
    res.args.insert("name".into(), name);
    res.args.extend(outcome_args);
    pl.log(
        date,
        "action-talk",
        &res.outcome_key,
        res.args
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
    );
    Ok(res)
}

/// Run a topic's mechanic through the existing paths. Returns the outcome
/// key (if the mechanic did something) and its arguments.
fn mechanic(
    state: &mut State,
    content: &ContentBundle,
    p: &Prepared,
    late_night: bool,
) -> (Option<String>, Vec<(String, String)>) {
    let date = state.calendar.date;
    let (speaker, me) = (p.speaker, p.me);
    let fx = &content.actions.lobby;
    let s = &content.talk.support;
    let negotiation = state
        .player
        .as_ref()
        .and_then(|x| state.politics.get(x.politician))
        .map(|x| x.negotiation as f64)
        .unwrap_or(0.5);
    let subject = |policy: PolicyInstanceId| Some(characters::Subject::Policy { id: policy.0 });
    let key = |k: &str| (Some(format!("outcome-talk-{k}")), Vec::new());
    match p.mechanic {
        Mech::None => (None, Vec::new()),
        Mech::PitchNoted => key("pitch-noted"),
        Mech::FavourNone => key("favour-none"),
        Mech::SmallTalk => {
            if p.small_talks < content.talk.small_talk_weekly_cap as usize {
                characters::remember(
                    &mut state.politics,
                    content,
                    date,
                    speaker,
                    me,
                    MemoryKind::Met,
                    None,
                    Some(content.talk.small_talk_opinion),
                );
            }
            key("small-talk")
        }
        Mech::Pitch { target, policy } => {
            let odds = p
                .odds
                .unwrap_or_else(|| crate::actions::lobby_odds(state, content, target, late_night));
            let lp = &content.council.lobby;
            let rng = state.rng.get(Stream::Player);
            let success = rng.chance(odds);
            let gain = if success {
                rng.range_f64(lp.gain[0], lp.gain[1]) * (0.5 + negotiation)
            } else {
                lp.fail_gain
            } * content.talk.pitch.sway_share;
            *state.politics.sway.entry((target.0, policy.0)).or_default() += gain as f32;
            characters::remember(
                &mut state.politics,
                content,
                date,
                speaker,
                me,
                if success {
                    MemoryKind::LobbiedMe
                } else {
                    MemoryKind::PressedMe
                },
                subject(policy),
                Some(if success {
                    fx.opinion_success
                } else {
                    fx.opinion_fail
                }),
            );
            if late_night {
                late_deal(state, content, speaker);
            }
            key(if success {
                "pitch-success"
            } else {
                "pitch-fail"
            })
        }
        Mech::Back { target, policy } => {
            let odds = p
                .odds
                .unwrap_or_else(|| crate::actions::lobby_odds(state, content, target, late_night));
            let success = state.rng.get(Stream::Player).chance(odds);
            if !success {
                refused(state, content, speaker, me, subject(policy));
                return key("support-refused");
            }
            *state.politics.sway.entry((target.0, policy.0)).or_default() += s.sway as f32;
            characters::remember(
                &mut state.politics,
                content,
                date,
                speaker,
                me,
                MemoryKind::LobbiedMe,
                subject(policy),
                Some(fx.opinion_success),
            );
            if late_night {
                late_deal(state, content, speaker);
            }
            ask_in_return(state, content, speaker, me, subject(policy));
            key("support-backed")
        }
        Mech::Endorse { policy } => {
            let odds = p
                .odds
                .unwrap_or_else(|| support_odds(state, content, speaker));
            let success = state.rng.get(Stream::Player).chance(odds);
            let subj = policy.and_then(subject);
            if !success {
                refused(state, content, speaker, me, subj);
                return key("support-refused");
            }
            // A public endorsement: their neighbours hear it (V2 brings
            // follower weights by community, class and occupation).
            if let Some(home) =
                characters::profile(&state.politics, &state.world, speaker).map(|x| x.home as usize)
                && let Some(h) = state.population.hoods.get_mut(home)
            {
                for c in h.cohorts.iter_mut() {
                    let room = 1.0 - (c.player_approval as f64).abs();
                    c.player_approval = clamp(
                        c.player_approval as f64 + s.endorse_approval * room,
                        -1.0,
                        1.0,
                    ) as f32;
                }
            }
            if let Some(id) = policy
                && let Some(x) = state.policy.instances.get_mut(id.index())
            {
                x.awareness = clamp(x.awareness as f64 + s.bill_awareness, 0.0, 1.0) as f32;
            }
            characters::remember(
                &mut state.politics,
                content,
                date,
                speaker,
                me,
                MemoryKind::LobbiedMe,
                subj,
                Some(fx.opinion_success),
            );
            if ask_in_return(state, content, speaker, me, subj) {
                key("support-endorsed-favour")
            } else {
                key("support-endorsed")
            }
        }
        Mech::CallIn { debtor, policy } => {
            if characters::repay(&mut state.politics, speaker, me).is_none() {
                return key("favour-none");
            }
            // The player's side of the bill: their own, else their declared
            // vote, else yes (as the `call_in` effect of decisions).
            let pl = state.player.as_ref().map(|x| x.politician);
            let own = state
                .policy
                .instances
                .get(policy.index())
                .is_some_and(|x| x.sponsor.is_some() && x.sponsor == pl);
            let side = if own {
                1.0
            } else {
                match pl.and_then(|x| state.politics.declared_votes.get(&(x.0, policy.0))) {
                    Some(v) if *v < 0 => -1.0,
                    _ => 1.0,
                }
            };
            *state.politics.sway.entry((debtor.0, policy.0)).or_default() +=
                (side * fx.trade_gain) as f32;
            characters::remember(
                &mut state.politics,
                content,
                date,
                speaker,
                me,
                MemoryKind::PressedMe,
                subject(policy),
                None,
            );
            key("favour-called-in")
        }
        Mech::Offer { target, policy } => {
            *state.politics.sway.entry((target.0, policy.0)).or_default() +=
                (fx.trade_gain * content.talk.favour.sway_share) as f32;
            characters::owe(
                &mut state.politics,
                me,
                speaker,
                "favour-bill",
                subject(policy),
                date,
            );
            characters::remember(
                &mut state.politics,
                content,
                date,
                speaker,
                me,
                MemoryKind::PromisedFavour,
                subject(policy),
                Some(fx.opinion_success),
            );
            key("favour-offered")
        }
    }
}

/// A refusal: they remember being pressed.
fn refused(
    state: &mut State,
    content: &ContentBundle,
    speaker: CharacterId,
    me: CharacterId,
    subject: Option<characters::Subject>,
) {
    let date = state.calendar.date;
    characters::remember(
        &mut state.politics,
        content,
        date,
        speaker,
        me,
        MemoryKind::PressedMe,
        subject,
        Some(content.talk.support.refused_opinion),
    );
}

/// Someone who agreed without much warmth asks a favour in return (the
/// player owes them one). Returns whether they did.
fn ask_in_return(
    state: &mut State,
    content: &ContentBundle,
    speaker: CharacterId,
    me: CharacterId,
    subject: Option<characters::Subject>,
) -> bool {
    let op = characters::opinion(&state.politics, speaker, me) as f64;
    if op >= content.talk.support.favour_below {
        return false;
    }
    let date = state.calendar.date;
    characters::owe(
        &mut state.politics,
        me,
        speaker,
        "favour-support",
        subject,
        date,
    );
    true
}

/// A late-night pitch is a back-room deal (as late lobbying, DEC-034).
fn late_deal(state: &mut State, content: &ContentBundle, speaker: CharacterId) {
    let legal = content.actions.lobby.late_night_legal as f32;
    let Some(me) = state.player.as_mut().map(|p| {
        p.tracks.legal_jeopardy = (p.tracks.legal_jeopardy + legal).min(100.0);
        p.politician
    }) else {
        return;
    };
    crate::media::hide(
        state,
        content,
        &content.media.late_night_deal,
        me,
        vec![crate::narrative::Actor::Person { id: speaker }],
    );
}
