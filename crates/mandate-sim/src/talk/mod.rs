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
//! **A miss is free** (M6 note 26; `talk_params.miss_free`): on a topic that
//! informs, a reply that is only the topic's safe answer (the person does
//! not know, or will not say) costs no time. The time must still be there
//! to ask (so what is offered never shows who knows), and where the person
//! knows who would know, the safe answer's lead says so (`LeadToDef`).
//!
//! **Asking again** the same week gives another line (the keyed draw and
//! the repetition guard) and teaches nothing new unless the relationship
//! changed (the speaker's discretion rose); a mechanic does not run twice.
//!
//! Determinism: line draws are keyed on the dialogue stream and never
//! advance it; contested mechanics roll on the player stream like
//! lobbying. No prose: Fluent keys and facts (DEC-054).

pub mod context;
pub mod expect;
pub mod hooks;
pub mod open;
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
use crate::world::{Jurisdiction, NeighborhoodId, PlaceId};
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
    Place {
        place: PlaceId,
    },
    Person {
        person: CharacterId,
    },
    Hood {
        hood: NeighborhoodId,
    },
    Bill {
        policy: PolicyInstanceId,
    },
    Template {
        template: ContentKey,
    },
    Proposal {
        proposal: FamilyInstanceId,
    },
    Rival,
    /// The whole jurisdiction the conversation is in (M6 note 19).
    Jurisdiction {
        at: Jurisdiction,
    },
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
            AboutRef::Jurisdiction { .. } => AboutKind::Jurisdiction,
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

/// The jurisdictions a conversation where the player stands is "in" (the
/// whole-town subject): those of the player's focus that contain where they
/// are (DEC-098; the town in M6), widest last.
pub fn jurisdictions_here(state: &State) -> Vec<Jurisdiction> {
    let Some(pl) = state.player.as_ref() else {
        return Vec::new();
    };
    let Some(hood) = crate::places::hood_of(state, pl.location) else {
        return Vec::new();
    };
    state
        .focus()
        .into_iter()
        .filter(|j| j.contains_hood(&state.world, hood))
        .collect()
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
    let town = pl.home();
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
        AboutRef::Jurisdiction { at } => jurisdictions_here(state).contains(at),
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
    PitchNoted,
    /// Anyone but a colleague on a bill asked for a public endorsement (of
    /// the player or a bill).
    Endorse {
        policy: Option<PolicyInstanceId>,
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
    about: Option<&AboutRef>,
) -> Result<Mech, EngineError> {
    let bill = match about {
        Some(AboutRef::Bill { policy }) => Some(*policy),
        _ => None,
    };
    // A colleague is lobbied on a bill in conversation's own choices
    // (`actions::lobbying`, M6 note 29), not through these topics.
    let lobbied = colleague(state, speaker).is_some() && bill.is_some();
    if lobbied
        && matches!(
            topic.mechanic,
            TalkMechanic::Pitch | TalkMechanic::AskSupport | TalkMechanic::Favour
        )
    {
        return Err(bad("that is not something to ask them now: argue the bill"));
    }
    Ok(match topic.mechanic {
        TalkMechanic::Inform | TalkMechanic::Open => Mech::None,
        TalkMechanic::SmallTalk => Mech::SmallTalk,
        TalkMechanic::Pitch => Mech::PitchNoted,
        TalkMechanic::AskSupport => Mech::Endorse { policy: bill },
        TalkMechanic::Favour => Mech::FavourNone,
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
/// asks for support. `None` for the others. (Lobbying a colleague on a
/// bill shows each argument's own: `actions::lobbying`.)
pub fn odds(
    state: &State,
    content: &ContentBundle,
    speaker: CharacterId,
    topic: &str,
    about: Option<&AboutRef>,
    _late_night: bool,
) -> Option<f64> {
    let t = content.talk_topic(topic)?;
    match plan(state, t, speaker, about).ok()? {
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

/// Whether a conversation with this person opens now (M6 note 19): the
/// player has not talked with them today. Derived from the exchange log
/// (every conversation, a hook's choice too, is logged the day it is had),
/// so no state is kept for it. While it holds, the talk view carries the
/// greeting and the reply says no opener of its own.
pub fn opens_now(state: &State, speaker: CharacterId) -> bool {
    let Some(pl) = state.player.as_ref() else {
        return false;
    };
    let today = state.calendar.date;
    !pl.talk
        .log
        .iter()
        .any(|e| e.speaker == speaker && e.on == today)
}

/// Small talk with this person this week (flattery, §3.4).
fn small_talks_this_week(state: &State, content: &ContentBundle, speaker: CharacterId) -> usize {
    let Some(pl) = state.player.as_ref() else {
        return 0;
    };
    pl.talk
        .this_week(speaker, week_of(state.calendar.date))
        .filter(|e| {
            content
                .talk_topic(&e.topic)
                .is_some_and(|t| t.mechanic == TalkMechanic::SmallTalk)
        })
        .count()
}

/// The greeting (M6 note 19): what the person says as the conversation
/// opens, before any topic is chosen. The opener rule (`reply::opener`: the
/// openers that hold, the repetition guard, the keyed draw), with no topic
/// and nothing asked about. `None` once they have talked with the player
/// today (`opens_now`), or when no opener holds. Read-only.
pub fn greeting(
    state: &State,
    content: &ContentBundle,
    speaker: CharacterId,
) -> Option<SpokenLine> {
    if !opens_now(state, speaker) {
        return None;
    }
    let pl = state.player.as_ref()?;
    let me = characters::player_character(state)?;
    if speaker == me {
        return None;
    }
    let week = week_of(state.calendar.date);
    let open = Openness {
        flattered: small_talks_this_week(state, content, speaker) > 0,
        favour_called: false,
    };
    let ctx = Ctx {
        st: state,
        content,
        town: pl.home(),
        me,
        speaker,
        about: None,
        here: pl.location,
        cap: knows::discretion_cap(state, content, speaker, me, open),
    };
    let n = pl.talk.this_week(speaker, week).count() as u32;
    let c = reply::opener(&ctx, None, pl.talk.used_by(speaker), week, n)?;
    let mut args = context_args(&ctx);
    args.extend(c.args);
    Some(SpokenLine {
        part: c.part,
        line: c.line.id.clone(),
        key: c.line.key.clone(),
        args: args.into_iter().collect(),
    })
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
    /// The greeting the conversation opened with (the talk view's), when it
    /// opens now: remembered as a line the speaker used.
    greeting: Option<SpokenLine>,
    teach: Vec<(Subject, Level)>,
    leads: Vec<Lead>,
    effects: Vec<TalkEffectDef>,
    small_talks: usize,
    /// What it was about: the player's subject, or, for the open question,
    /// what the engine chose.
    about: Option<AboutRef>,
    /// The reply was only the topic's safe answer on a topic that informs:
    /// no time is charged (`talk_params.miss_free`).
    miss: bool,
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
    // The open question: the engine chooses what it is about.
    let asks_open = topic.mechanic == TalkMechanic::Open;
    if asks_open && about.is_some() {
        return Err(bad("that is not something to raise on this topic"));
    }
    let picked = if asks_open {
        open::mind(state, content, speaker, me, pl.home(), &topic.id)
    } else {
        None
    };
    let about = match &picked {
        Some(m) => m.about.as_ref(),
        None => about,
    };
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
    let mechanic = plan(state, topic, speaker, about)?;
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
    let small_talks = small_talks_this_week(state, content, speaker);
    let open = Openness {
        flattered: small_talks > 0,
        favour_called: false,
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
        town: pl.home(),
        me,
        speaker,
        about,
        here: pl.location,
        cap,
    };
    if !ctx.all_hold(&o.when, &mut Args::new()) || expect::nonsense(&ctx, topic, about) {
        return Err(bad("that is not something to ask them now"));
    }
    // The first conversation of the day opened with the greeting: no
    // opener again. Later ones open as the greeting would.
    let greeting = greeting(state, content, speaker);
    let opening = if opens_now(state, speaker) {
        reply::Opening::Greeted
    } else {
        reply::Opening::Opener {
            n: talk.this_week(speaker, week).count() as u32,
        }
    };
    let composed = reply::compose_with(
        &ctx,
        topic,
        talk.used_by(speaker),
        week,
        n,
        opening,
        asks_open && picked.is_none(),
    );
    let substance = composed
        .parts
        .iter()
        .find(|c| c.part == mandate_content::conversation::TalkPart::Substance);
    let miss = content.talk.miss_free
        && topic.mechanic.informs()
        && (composed.fell_back || substance.is_none_or(|c| c.line.miss));
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
        // A line gated on what they would tell without saying it carries
        // no name (nothing the player was not told is in an exchange).
        let withheld = mandate_content::conversation::withheld_args(c.line);
        args.retain(|k, _| !withheld.contains(k));
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
            && let Some(to) = lead_target(&ctx, &l.to, &subject, l.level)
        {
            // A lead to a person names them: the player has heard of them.
            if let LeadTarget::Person { person } = &to {
                teach.push((Subject::Person { person: *person }, Level::Heard));
            }
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
        greeting,
        teach,
        leads,
        effects,
        small_talks,
        about: about.cloned(),
        miss,
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
    ctx.role_args(&mut a);
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
            if let Some(h) = crate::places::hood_of(st, ctx.here) {
                context::hood_arg(st, h, "here", &mut a);
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
        Some(AboutRef::Hood { hood }) => context::hood_arg(st, *hood, "about", &mut a),
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
        Some(AboutRef::Jurisdiction { at }) => {
            let name = match at {
                Jurisdiction::Municipality { id } => {
                    st.world.municipalities.get(id.index()).map(|m| &m.name)
                }
                Jurisdiction::Region { id } => st.world.regions.get(id.index()).map(|r| &r.name),
                Jurisdiction::Nation => None,
            };
            if let Some(n) = name {
                a.insert("about".into(), n.clone());
            }
        }
        Some(AboutRef::Proposal { .. }) | None => {}
    }
    a
}

fn lead_target(
    ctx: &Ctx<'_>,
    to: &LeadToDef,
    subject: &Subject,
    level: Level,
) -> Option<LeadTarget> {
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
        // Whoever the speaker knows who would know, by public facts.
        LeadToDef::WhoKnows => {
            expect::who_knows(ctx, subject, level).map(|person| LeadTarget::Person { person })
        }
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
    if hooks::is_choice(topic) {
        return choose(state, content, speaker, topic, late_night);
    }
    let mut p = prepare(state, content, speaker, topic, about, late_night)?;
    if p.parts.is_empty() {
        // Never an empty box: nothing to say is said (M6 note 12).
        p.parts.push(nothing_to_say(state, speaker));
    }
    let date = state.calendar.date;
    let me = p.me;
    let before = characters::opinion(&state.politics, speaker, me);
    // Who is here as the conversation starts is noted (S1: last seen).
    knowledge::note_sightings(state, content);
    if !p.miss {
        pay(state, content, late_night);
    }
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
    let (outcome, outcome_args) = if p.miss {
        (Some(MISS.to_string()), Vec::new())
    } else if p.repeat && p.mechanic != Mech::SmallTalk {
        (None, Vec::new())
    } else {
        mechanic(state, content, &p)
    };
    let after = characters::opinion(&state.politics, speaker, me);
    let place = match state.player.as_ref().map(|x| x.location) {
        Some(Location::Place { place }) => Some(place),
        _ => None,
    };
    let lines: Vec<ContentKey> = p
        .greeting
        .iter()
        .chain(p.parts.iter())
        .map(|s| s.line.clone())
        .collect();
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
            about: p.about.clone(),
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

/// The outcome of a miss: they could not help, and it cost no time.
pub const MISS: &str = "outcome-talk-miss";

/// The Fluent key of "they have nothing to say about that".
pub const NOTHING_TO_SAY: &str = "talk-nothing-to-say";

/// The line said when a conversation would otherwise say nothing.
fn nothing_to_say(state: &State, speaker: CharacterId) -> SpokenLine {
    let mut args: Vec<(String, String)> = Vec::new();
    if let Some(p) = characters::profile(&state.politics, &state.world, speaker) {
        args.push(("speaker".into(), p.full_name()));
    }
    SpokenLine {
        part: mandate_content::conversation::TalkPart::Substance,
        line: NOTHING_TO_SAY.into(),
        key: NOTHING_TO_SAY.into(),
        args,
    }
}

/// Every check a hook's choice makes, without changing anything (the talk
/// panel's "available"): they are here, it is offered now, and the time.
pub fn check_choice(
    state: &State,
    content: &ContentBundle,
    speaker: CharacterId,
    id: &str,
    late_night: bool,
) -> Result<(&'static hooks::Hook, hooks::Choice), EngineError> {
    let me = characters::player_character(state).ok_or_else(|| bad("no player"))?;
    if speaker == me {
        return Err(bad("they are not here to talk to"));
    }
    let found = hooks::find(state, content, speaker, id)
        .ok_or_else(|| bad("that is not something to ask them now"))?;
    // A remote choice (lobbying a colleague) needs no walk (DEC-051).
    if !found.1.remote && !can_talk(state, content, speaker) {
        return Err(bad("they are not here to talk to"));
    }
    match found.1.cost {
        hooks::ChoiceCost::Conversation => {
            if late_night && !late_ok(state, content) {
                return Err(bad(
                    "late at night, people talk only where private meetings are held",
                ));
            }
            check_cost(state, content, late_night)?;
        }
        hooks::ChoiceCost::Day => {
            if late_night {
                return Err(bad("that is for the day: meet after hours instead"));
            }
            check_cost(state, content, false)?;
        }
        hooks::ChoiceCost::LateSlot => check_cost(state, content, true)?,
        hooks::ChoiceCost::Free => {
            if late_night && !late_ok(state, content) {
                return Err(bad(
                    "late at night, people talk only where private meetings are held",
                ));
            }
        }
    }
    Ok(found)
}

/// A hook's choice (S4, [`hooks`]): the checks and the cost of a
/// conversation, the reply by its conditions, the owner's effect, and the
/// exchange in the log. Fails without changing anything.
fn choose(
    state: &mut State,
    content: &ContentBundle,
    speaker: CharacterId,
    id: &str,
    late_night: bool,
) -> Result<ActionResult, EngineError> {
    let date = state.calendar.date;
    let pl = state.player.as_ref().ok_or_else(|| bad("no player"))?;
    let me = characters::player_character(state).ok_or_else(|| bad("no player"))?;
    let (hook, choice) = check_choice(state, content, speaker, id, late_night)?;
    let greeted = greeting(state, content, speaker);
    let open = Openness {
        flattered: false,
        favour_called: false,
    };
    let cap = knows::discretion_cap(state, content, speaker, me, open);
    let ctx = Ctx {
        st: state,
        content,
        town: pl.home(),
        me,
        speaker,
        about: None,
        here: pl.location,
        cap,
    };
    let mut args = context_args(&ctx);
    args.extend(choice.args.iter().cloned());
    let (reply, part) = match hooks::reply(&ctx, &choice) {
        Some((r, extra)) => {
            let mut a = args.clone();
            a.extend(extra);
            let line = SpokenLine {
                part: mandate_content::conversation::TalkPart::Substance,
                line: r.key.clone(),
                key: r.key.clone(),
                args: a.into_iter().collect(),
            };
            (r.clone(), line)
        }
        // No reply holds: nothing to say, and nothing agreed.
        None => (
            hooks::Reply {
                key: NOTHING_TO_SAY.into(),
                when: Vec::new(),
                agrees: false,
            },
            nothing_to_say(state, speaker),
        ),
    };
    let before = characters::opinion(&state.politics, speaker, me);
    knowledge::note_sightings(state, content);
    match choice.cost {
        hooks::ChoiceCost::Conversation => pay(state, content, late_night),
        hooks::ChoiceCost::Day => pay(state, content, false),
        hooks::ChoiceCost::LateSlot => pay(state, content, true),
        hooks::ChoiceCost::Free => {}
    }
    // Talking to someone is meeting them properly.
    let mut learned = Vec::new();
    if let Some(pl) = state.player.as_mut() {
        let subject = Subject::Person { person: speaker };
        let e = Effect::Learn {
            subject: subject.clone(),
            level: Level::Known,
            source: Source::Talk {
                person: speaker,
                topic: choice.id.clone(),
            },
        };
        if knowledge::apply_effect(&mut pl.knowledge, e, date, content) {
            learned.push((subject, Level::Known));
        }
    }
    // What it does: the owner's code.
    let done = (hook.chosen)(state, content, &choice, speaker, &reply);
    let after = characters::opinion(&state.politics, speaker, me);
    let name = characters::profile(&state.politics, &state.world, speaker)
        .map(|x| x.full_name())
        .unwrap_or_default();
    let place = match state.player.as_ref().map(|x| x.location) {
        Some(Location::Place { place }) => Some(place),
        _ => None,
    };
    let t = &content.talk;
    let Some(pl) = state.player.as_mut() else {
        return Err(bad("no player"));
    };
    let id: ExchangeId = pl.talk.ids.alloc();
    if let Some(g) = &greeted {
        pl.talk.record_used(
            speaker,
            std::slice::from_ref(&g.line),
            date,
            t.memory_lines as usize,
        );
    }
    pl.talk.push(
        Exchange {
            id,
            on: date,
            speaker,
            place,
            topic: choice.id.clone(),
            about: None,
            parts: vec![part],
            learned,
            leads: Vec::new(),
            cap,
            repeat: false,
            opinion: after - before,
            outcome: Some(done.outcome_key.clone()),
            outcome_args: done.args.clone(),
        },
        t.log_cap as usize,
    );
    let mut res = ActionResult {
        created: Some(id.0),
        outcome_key: done.outcome_key,
        ..Default::default()
    };
    res.args.insert("name".into(), name);
    res.args.extend(done.args);
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
) -> (Option<String>, Vec<(String, String)>) {
    let date = state.calendar.date;
    let (speaker, me) = (p.speaker, p.me);
    let fx = &content.actions.lobby;
    let s = &content.talk.support;
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
