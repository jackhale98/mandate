//! Conversations for the client (DEC-093; M6_ENGINE_CONTRACTS §3.9): the
//! talk panel (`TalkView`: the topics the player can raise with someone,
//! each with its odds where it is contested, its cost and whether it can be
//! asked now, with the reason when not), and an exchange as it was spoken
//! (`ExchangeView`: the line keys and their arguments in the speaker's
//! voice, what the player learned, the leads it opened, how the
//! relationship moved). Also the canvass preview (`CanvassView`): the
//! worries the player knows of in a neighbourhood, with the match the
//! engine would apply (DEC-091 §1.6).
//!
//! Facts and Fluent keys only (DEC-054). Every check, every odds figure and
//! every match comes from the engine's own functions (`mandate_sim::talk`,
//! `actions::canvass_match`), the same ones the commands use: the odds shown
//! are the odds rolled (DEC-073). Knowledge gates what is listed (subjects
//! the player knows of; worries as far as they know them), never the odds
//! or the rules (DEC-091). Views never write.
//!
//! **The order is the engine's** (M6 note 26): a topic's subjects come with
//! the ones the person could be expected to know first (`likely`), judged
//! from public facts only (`talk::expect`); the client draws the rest
//! behind "Something else…". Nothing here reads what the person secretly
//! knows or is willing to say, so the panel never gives the answer away.
//! Pairings that make no sense (`talk::expect::nonsense`) are left out.

use crate::characters::{PersonRefView, opinion_band, person_ref_known, trust_band};
use crate::knowledge::{Gate, KnowledgeLevelDto, KnowledgeView, LeadView, SubjectDto};
use crate::places::{LocationView, RouteView, location_view};
use crate::views::DateView;
use mandate_content::conversation::{AboutKind, TalkMechanic};
use mandate_content::{ContentBundle, TierKey};
use mandate_sim::characters::{self, CharacterId, ambient};
use mandate_sim::commands::EngineError;
use mandate_sim::places::{self, Location};
use mandate_sim::player::knowledge::{Level, Subject};
use mandate_sim::policy::PolicyStatus;
use mandate_sim::talk::{self, AboutRef, Exchange, ExchangeId, TalkTarget};
use mandate_sim::world::{Jurisdiction, MunicipalityId, NeighborhoodId, PlaceId};
use mandate_sim::{Engine, State};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

// --- Commands -----------------------------------------------------------------------

/// Whom the player talks to: someone stored (`person`, a character id), or
/// a passer-by where the player is (`slot`; they become a character, as
/// with `Meet`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TalkTargetDto {
    Person { person: u32 },
    Ambient { slot: u8 },
}

impl TalkTargetDto {
    pub fn to_sim(self) -> TalkTarget {
        match self {
            TalkTargetDto::Person { person } => TalkTarget::Person {
                person: CharacterId(person),
            },
            TalkTargetDto::Ambient { slot } => TalkTarget::Ambient { slot },
        }
    }
}

/// What a conversation is about, with plain ids (mirrors the engine's
/// `AboutRef`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AboutRefDto {
    Place {
        place: u32,
    },
    Person {
        person: u32,
    },
    Hood {
        hood: u32,
    },
    Bill {
        policy: u32,
    },
    Template {
        template: String,
    },
    Proposal {
        proposal: u32,
    },
    Rival,
    /// The whole jurisdiction the conversation is in: its tier (`local` |
    /// `regional` | `national`) and id (0 for the nation).
    Jurisdiction {
        tier: String,
        id: u32,
    },
}

/// A jurisdiction from its tier word and raw id (`None`: no such tier).
fn jurisdiction_of(tier: &str, id: u32) -> Option<Jurisdiction> {
    let t = match tier {
        "local" => TierKey::Local,
        "regional" => TierKey::Regional,
        "national" => TierKey::National,
        _ => return None,
    };
    Some(Jurisdiction::of(t, id))
}

/// A jurisdiction's tier word (`local` | `regional` | `national`).
fn tier_word(at: Jurisdiction) -> &'static str {
    match at.tier() {
        TierKey::Local => "local",
        TierKey::Regional => "regional",
        TierKey::National => "national",
    }
}

/// How a neighbourhood that has its town's name is named to the player: as
/// the town centre (`ui-hood-centre`, with `$centre_town`; M6 note 19). The
/// stored name stays. `None` for every other neighbourhood.
pub fn hood_centre_label(
    st: &State,
    hood: NeighborhoodId,
) -> Option<(String, BTreeMap<String, String>)> {
    let town = mandate_sim::talk::context::centre_town(st, hood)?;
    let args = BTreeMap::from([("centre_town".to_string(), town.to_string())]);
    Some((
        mandate_sim::talk::context::HOOD_CENTRE_KEY.to_string(),
        args,
    ))
}

/// The town of the player's focus (DEC-098: the jurisdictions simulated in
/// detail around them; their home town in M6).
fn focus_town(st: &State) -> Option<MunicipalityId> {
    st.focus().into_iter().find_map(|j| j.municipality())
}

impl AboutRefDto {
    pub fn to_sim(&self) -> AboutRef {
        use mandate_sim::player::knowledge::FamilyInstanceId;
        use mandate_sim::policy::PolicyInstanceId;
        match self {
            AboutRefDto::Place { place } => AboutRef::Place {
                place: PlaceId(*place),
            },
            AboutRefDto::Person { person } => AboutRef::Person {
                person: CharacterId(*person),
            },
            AboutRefDto::Hood { hood } => AboutRef::Hood {
                hood: NeighborhoodId(*hood),
            },
            AboutRefDto::Bill { policy } => AboutRef::Bill {
                policy: PolicyInstanceId(*policy),
            },
            AboutRefDto::Template { template } => AboutRef::Template {
                template: template.clone(),
            },
            AboutRefDto::Proposal { proposal } => AboutRef::Proposal {
                proposal: FamilyInstanceId(*proposal),
            },
            AboutRefDto::Rival => AboutRef::Rival,
            // A tier that does not exist names the nation's whole: refused
            // by the command's own checks unless the player is there.
            AboutRefDto::Jurisdiction { tier, id } => AboutRef::Jurisdiction {
                at: jurisdiction_of(tier, *id).unwrap_or(Jurisdiction::Nation),
            },
        }
    }

    pub fn of(a: &AboutRef) -> Self {
        match a {
            AboutRef::Place { place } => AboutRefDto::Place { place: place.0 },
            AboutRef::Person { person } => AboutRefDto::Person { person: person.0 },
            AboutRef::Hood { hood } => AboutRefDto::Hood { hood: hood.0 },
            AboutRef::Bill { policy } => AboutRefDto::Bill { policy: policy.0 },
            AboutRef::Template { template } => AboutRefDto::Template {
                template: template.clone(),
            },
            AboutRef::Proposal { proposal } => AboutRefDto::Proposal {
                proposal: proposal.0,
            },
            AboutRef::Rival => AboutRefDto::Rival,
            AboutRef::Jurisdiction { at } => AboutRefDto::Jurisdiction {
                tier: tier_word(*at).into(),
                id: at.raw(),
            },
        }
    }
}

// --- The talk panel ----------------------------------------------------------------

/// A subject the player can raise on a topic: only things they know of
/// (DEC-091). Named by `label` (a name) or, where that is empty, by
/// `label_key` (a Fluent key: a policy's or a place kind's name). With
/// `label_args`, `label_key` names it (with those arguments) in place of
/// `label`: the town centre ("Larkowick town centre"; `label` keeps the
/// stored name).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AboutChoiceView {
    pub about: AboutRefDto,
    /// `place` | `person` | `hood` | `bill` | `template` | `rival` |
    /// `jurisdiction`.
    pub kind: String,
    pub label: String,
    pub label_key: Option<String>,
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_args: Option<BTreeMap<String, String>>,
    /// The odds of a contested topic on this subject (DEC-073): the
    /// engine's, the ones the roll uses. `None` where nothing is rolled.
    pub odds: Option<f64>,
    pub available: bool,
    /// Why not, as a Fluent key (`talk-why-…`), when not available.
    pub unavailable_key: Option<String>,
    /// The person could be expected to know this, judged from public facts
    /// (their occupation, where they live and work, their post, their
    /// bills, their trade's issue): listed first. The others are listed
    /// after, for "Something else…". Never from what they secretly know.
    pub likely: bool,
}

/// A topic the player can raise with this person.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TopicView {
    /// Topic content id (`topic.who_runs`): the `Talk` command's `topic`.
    pub id: String,
    /// The button's Fluent key.
    pub name_key: String,
    /// `inform` | `pitch` | `ask_support` | `favour` | `small_talk` |
    /// `open` (the open question: the person says what is on their mind;
    /// it takes no subject) | `choice` (a system's own).
    pub mechanic: String,
    /// It needs a subject (`about`); otherwise one is optional.
    pub about_required: bool,
    /// The subjects it can be about, as far as the player knows of them:
    /// the `likely` ones first (the most expected first), then the rest.
    pub about: Vec<AboutChoiceView>,
    /// Asked with no subject: the odds, where something is rolled.
    pub odds: Option<f64>,
    /// Asked with no subject: can it be asked now? (A topic that needs a
    /// subject is never available without one.)
    pub available: bool,
    pub unavailable_key: Option<String>,
    /// Asked of this person already this week: another line, nothing new
    /// unless the relationship changed (and no second roll).
    pub asked_this_week: bool,
    /// A system's own choice (talk hooks, S4; `mechanic` `choice`): the
    /// arguments of `name_key` and `say_key` (`*_key` arguments are keys).
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_args: Option<BTreeMap<String, String>>,
    /// A choice's line for the player (what they say), with `name_args`.
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub say_key: Option<String>,
    /// A choice's costs, shown before it is chosen (DEC-073): Fluent keys
    /// and their arguments (`lobby-cost-*`).
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub costs: Option<Vec<crate::office::ReasonView>>,
    /// The bill a choice argues (lobbying, M6 note 29): the panel groups
    /// the arguments on one bill under it.
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bill: Option<u32>,
    /// It can be chosen without walking to them (lobbying from the list,
    /// DEC-051).
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote: Option<bool>,
}

/// The talk panel for someone (§3.9): who they are, whether they are here
/// (else where to find them), what a conversation costs, and the topics.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TalkView {
    /// Gated as everywhere (DEC-091): a stranger shows only their role.
    pub person: PersonRefView,
    /// They are where the player is: topics can be raised now.
    pub here: bool,
    /// Where they can be found (auto-routing, DEC-079: `go`), when not here.
    /// `None`: nowhere the player can walk to now.
    pub route: Option<RouteView>,
    /// How they feel about the player and how far they trust them (never
    /// gated: about the player).
    pub opinion_band: String,
    pub trust_band: String,
    /// What a conversation costs, in half slots (1: half a slot).
    pub cost_halves: u32,
    /// A conversation earlier this turn left half a slot: the next one uses it.
    pub half_slot_left: bool,
    pub slots_left: u32,
    /// The view is for a late-night conversation (the late slot).
    pub late_night: bool,
    /// A late-night conversation is possible here now.
    pub late_night_possible: bool,
    /// No time left for a conversation now (no slot, no half slot; late at
    /// night, the late slot used).
    pub out_of_time: bool,
    pub topics: Vec<TopicView>,
    /// What they say as the conversation opens, before any topic is chosen
    /// (M6 note 19; `talk::greeting`): the opener the first reply would have
    /// started with, which that reply then leaves out. `None` once they
    /// have talked with the player today, or when they are not here.
    pub greeting: Option<SpokenLineView>,
    /// The speaker's voice (`talk_voices` id), when one holds.
    pub voice: Option<String>,
}

/// The reason key for a refused conversation. The engine's checks are the
/// authority (`talk::check`); this only names them for the panel.
fn why(e: &EngineError) -> &'static str {
    let EngineError::InvalidCommand(m) = e else {
        return "talk-why-other";
    };
    let m = m.as_str();
    if m.contains("not enough time") {
        "talk-why-time"
    } else if m.contains("late night already used") {
        "talk-why-late-used"
    } else if m.contains("meet after hours instead") {
        "talk-why-day"
    } else if m.contains("late at night") {
        "talk-why-late-here"
    } else if m.contains("needs a subject") {
        "talk-why-needs-subject"
    } else if m.contains("not the person to ask") {
        "talk-why-role"
    } else if m.contains("not talked about here") {
        "talk-why-place"
    } else if m.contains("not something to ask them now") {
        "talk-why-not-now"
    } else if m.contains("owe them too much") {
        "talk-why-owe"
    } else if m.contains("where it is done") {
        "talk-why-verb-place"
    } else if m.contains("not here to talk") {
        "talk-why-away"
    } else if m.contains("nothing like that") || m.contains("not something to raise") {
        "talk-why-about"
    } else {
        "talk-why-other"
    }
}

/// A topic is not offered to this person here and now (their role, the
/// place, its conditions): it is left out of the panel, not shown greyed.
fn not_offered(key: &str) -> bool {
    matches!(key, "talk-why-role" | "talk-why-place" | "talk-why-not-now")
}

/// Where someone can be talked to now: where their daily routine has them
/// (`characters::routines`, DEC-031), when that is somewhere the player can
/// go. "Go to them" routes there from every view (DEC-051).
pub fn talk_location(engine: &Engine, id: CharacterId) -> Option<Location> {
    let st = engine.state();
    st.player.as_ref()?;
    characters::routines::whereabouts(st, engine.content(), id)
        .filter(|loc| places::validate_move(st, *loc).is_ok())
}

/// The subjects the player knows of for each kind a topic can be about.
fn choices(engine: &Engine, kinds: &[AboutKind]) -> Vec<AboutChoiceView> {
    let st = engine.state();
    let content = engine.content();
    let gate = Gate::of(engine);
    let Some(pl) = st.player.as_ref() else {
        return Vec::new();
    };
    let Some(town) = focus_town(st) else {
        return Vec::new();
    };
    let me = characters::player_character(st);
    let mut out = Vec::new();
    let mut push = |about: AboutRef, label: String, label_key: Option<String>| {
        out.push(AboutChoiceView {
            kind: about.kind().key().into(),
            about: AboutRefDto::of(&about),
            label,
            label_key,
            label_args: None,
            odds: None,
            available: false,
            unavailable_key: None,
            likely: false,
        });
    };
    for k in kinds {
        match k {
            AboutKind::Jurisdiction => {
                for at in talk::jurisdictions_here(st) {
                    let key = format!("talk-whole-{}", tier_word(at));
                    push(AboutRef::Jurisdiction { at }, String::new(), Some(key));
                }
            }
            AboutKind::Hood => {
                // The neighbourhoods of the player's focus (DEC-098).
                let mut hoods: Vec<usize> = st
                    .focus()
                    .into_iter()
                    .flat_map(|j| j.hoods(&st.world))
                    .collect();
                hoods.dedup();
                for h in hoods {
                    if let Some(n) = st.world.neighborhoods.get(h) {
                        push(AboutRef::Hood { hood: n.id }, n.name.clone(), None);
                    }
                }
            }
            AboutKind::Place => {
                for p in st.world.places.iter().filter(|p| {
                    p.municipality == town
                        && p.is_open()
                        && crate::knowledge::place_listed(&gate, st, p.id)
                }) {
                    push(
                        AboutRef::Place { place: p.id },
                        p.name.clone(),
                        Some(format!("place-{}", p.kind.key())),
                    );
                }
            }
            AboutKind::Person => {
                // People the player has at least heard of (the ledger), in
                // town, not the speaker, the player or the rival (who has a
                // kind of their own).
                let rival = st
                    .politics
                    .characters
                    .towns
                    .get(town.index())
                    .and_then(|t| t.rival);
                for s in pl.knowledge.entries.keys() {
                    let Subject::Person { person } = s else {
                        continue;
                    };
                    if Some(*person) == me || Some(*person) == rival {
                        continue;
                    }
                    if gate.person(*person) < Level::Heard {
                        continue;
                    }
                    let Some(p) = characters::profile(&st.politics, &st.world, *person) else {
                        continue;
                    };
                    if p.municipality.is_some_and(|m| m != town.0) {
                        continue;
                    }
                    let role = crate::characters::person_ref(engine, *person)
                        .map(|r| r.role_key)
                        .filter(|r| !r.is_empty());
                    push(AboutRef::Person { person: *person }, p.full_name(), role);
                }
            }
            AboutKind::Bill => {
                for b in st
                    .policy
                    .in_municipality(town.0)
                    .filter(|b| b.status != PolicyStatus::Ended)
                {
                    if let Some(t) = mandate_sim::policy::resolve(content, &st.policy, b) {
                        push(
                            AboutRef::Bill { policy: b.id },
                            String::new(),
                            Some(t.name_key.clone()),
                        );
                    }
                }
            }
            AboutKind::Template => {
                // The standard menu is never gated; situational policies
                // once heard of (DEC-091).
                for t in content
                    .policies
                    .values()
                    .filter(|t| t.tier == TierKey::Local && !t.retired)
                {
                    if t.unlock.is_some()
                        && gate.level(&Subject::Situational {
                            town,
                            template: t.id.clone(),
                        }) < Level::Heard
                    {
                        continue;
                    }
                    push(
                        AboutRef::Template {
                            template: t.id.clone(),
                        },
                        String::new(),
                        Some(t.name_key.clone()),
                    );
                }
            }
            AboutKind::Rival => {
                // The rival's identity is never gated (DEC-091).
                if let Some(r) = st
                    .politics
                    .characters
                    .towns
                    .get(town.index())
                    .and_then(|t| t.rival)
                    && let Some(p) = characters::profile(&st.politics, &st.world, r)
                {
                    push(AboutRef::Rival, p.full_name(), Some("ui-your-rival".into()));
                }
            }
            // Policy families (F-series) bring proposals.
            AboutKind::Proposal => {}
        }
    }
    // The centre is named as the town centre.
    for c in &mut out {
        if let AboutRefDto::Hood { hood } = c.about
            && let Some((key, args)) = hood_centre_label(st, NeighborhoodId(hood))
        {
            c.label_key = Some(key);
            c.label_args = Some(args);
        }
    }
    out
}

/// The talk panel (`QueryDto::Talk`) for someone stored or a passer-by
/// where the player is. `late_night`: the panel for a conversation in the
/// late slot. `None` for the player, an unknown id, nobody in that slot, or
/// no game.
///
/// A passer-by is met as the conversation starts (the `Talk` command
/// promotes them, as `Meet` does): their panel is worked out on a copy of
/// the state where that has happened, so it says exactly what the command
/// will do. Views never write: the copy is dropped.
pub fn talk_view(engine: &Engine, to: TalkTargetDto, late_night: bool) -> Option<TalkView> {
    match to {
        TalkTargetDto::Person { person } => person_talk_view(engine, person, late_night),
        TalkTargetDto::Ambient { slot } => {
            let st = engine.state();
            let content = engine.content();
            let loc = st.player.as_ref()?.location;
            let mut trial = st.clone();
            let id = ambient::promote(&mut trial, content, loc, slot)?;
            if let Some(me) = characters::player_character(&trial) {
                let date = trial.calendar.date;
                characters::remember(
                    &mut trial.politics,
                    content,
                    date,
                    id,
                    me,
                    characters::MemoryKind::Met,
                    None,
                    None,
                );
            }
            let met = Engine::from_state(trial, engine.content_arc()).ok()?;
            let mut v = person_talk_view(&met, id.0, late_night)?;
            v.person.id = None;
            v.person.ambient_slot = Some(slot);
            v.person.tier = "ambient".into();
            Some(v)
        }
    }
}

fn person_talk_view(engine: &Engine, person: u32, late_night: bool) -> Option<TalkView> {
    let st = engine.state();
    let content = engine.content();
    let pl = st.player.as_ref()?;
    let id = CharacterId(person);
    let me = characters::player_character(st)?;
    if id == me {
        return None;
    }
    characters::profile(&st.politics, &st.world, id)?;
    let gate = Gate::of(engine);
    let mut person_view = person_ref_known(engine, &gate, id)?;
    let here = talk::can_talk(st, content, id);
    person_view.can_talk = here;
    let route = if here {
        None
    } else {
        talk_location(engine, id).map(|loc| RouteView {
            action: "go".into(),
            late_night: false,
            neighborhood: places::hood_of(st, loc).map(|h| h.0),
            to: Some(location_view(st, loc)),
            here: false,
        })
    };
    let greeting = if here {
        talk::greeting(st, content, id).map(|g| SpokenLineView {
            part: g.part.key().into(),
            key: g.key,
            args: g.args.into_iter().collect(),
        })
    } else {
        None
    };
    let voice = talk_voice(st, content, id);
    let turn = st.calendar.turn;
    let half_slot_left = pl.talk.half_slot == Some(turn) && talk::cost_halves(content) == 1;
    let late_possible = here && !pl.late_night_used && talk::late_ok(st, content);
    let late = late_night && late_possible;
    let out_of_time = if late_night {
        pl.late_night_used
    } else {
        pl.slots_left() == 0 && !half_slot_left
    };
    let role = characters::knows::role_key(st, content, id);
    // The conversation as the order reads it: public facts only (no
    // discretion: `cap` is not read by `talk::expect`).
    let ctx = mandate_sim::talk::context::Ctx {
        st,
        content,
        town: pl.home(),
        me,
        speaker: id,
        about: None,
        here: pl.location,
        cap: Level::Unknown,
    };
    let mut topics = Vec::new();
    for t in &content.talk_topics {
        // Who it is offered to does not depend on where they stand.
        if !t.offered.roles.is_empty()
            && !role.as_ref().is_some_and(|r| t.offered.roles.contains(r))
        {
            continue;
        }
        let asked = talk::asked_this_week(st, id, &t.id);
        // The open question takes no subject: the engine chooses.
        let mut about = if t.mechanic == TalkMechanic::Open {
            Vec::new()
        } else {
            choices(engine, &t.about)
        };
        // Pairings that make no sense are never listed (by rule), and the
        // rest are ordered by what public facts say this person would know.
        about.retain(|c| !talk::expect::nonsense(&ctx, t, Some(&c.about.to_sim())));
        let mut ranked: Vec<(Level, AboutChoiceView)> = about
            .into_iter()
            .map(|mut c| {
                let about = c.about.to_sim();
                c.likely = talk::expect::likely(&ctx, t, &about);
                let level = if c.likely {
                    talk::expect::expectation(&ctx, t, &about)
                } else {
                    Level::Unknown
                };
                (level, c)
            })
            .collect();
        ranked.sort_by_key(|(l, _)| std::cmp::Reverse(*l));
        let mut about: Vec<AboutChoiceView> = ranked.into_iter().map(|(_, c)| c).collect();
        let judge = |a: Option<&AboutRef>| -> (Option<f64>, bool, Option<&'static str>) {
            if !here {
                return (None, false, Some("talk-why-away"));
            }
            match talk::check(st, content, id, &t.id, a, late) {
                Ok(()) => (talk::odds(st, content, id, &t.id, a, late), true, None),
                Err(e) => (None, false, Some(why(&e))),
            }
        };
        let (odds, available, reason) = if !here {
            (None, false, Some("talk-why-away"))
        } else if t.about_required {
            (None, false, Some("talk-why-needs-subject"))
        } else {
            judge(None)
        };
        if reason.is_some_and(not_offered) {
            continue;
        }
        for c in &mut about {
            let (o, ok, r) = judge(Some(&c.about.to_sim()));
            c.odds = o;
            c.available = ok;
            c.unavailable_key = r.map(String::from);
        }
        // A subject the engine would never take here (it is not talked about
        // on this topic, or not with them) is left out.
        about.retain(|c| !c.unavailable_key.as_deref().is_some_and(not_offered));
        if t.about_required && about.is_empty() {
            // Nothing the player knows of to raise: the topic waits.
            continue;
        }
        if t.about_required && here && about.iter().all(|c| !c.available) {
            // Every subject refused for the same reason (no time): say so on
            // the topic itself.
            let shared = about[0].unavailable_key.clone();
            if about.iter().all(|c| c.unavailable_key == shared) {
                topics.push(TopicView {
                    id: t.id.clone(),
                    name_key: t.name_key.clone(),
                    mechanic: t.mechanic.key().into(),
                    about_required: true,
                    about,
                    odds: None,
                    available: false,
                    unavailable_key: shared,
                    asked_this_week: asked,
                    name_args: None,
                    say_key: None,
                    costs: None,
                    bill: None,
                    remote: None,
                });
                continue;
            }
        }
        topics.push(TopicView {
            id: t.id.clone(),
            name_key: t.name_key.clone(),
            mechanic: t.mechanic.key().into(),
            about_required: t.about_required,
            about,
            odds,
            available,
            unavailable_key: reason.map(String::from),
            asked_this_week: asked,
            name_args: None,
            say_key: None,
            costs: None,
            bill: None,
            remote: None,
        });
    }
    // Systems' own choices (talk hooks, S4) come first: an ask's "Put in a
    // word" is why the player came. Checked as the command checks them.
    let mut choices: Vec<TopicView> = talk::hooks::offered(st, content, id)
        .into_iter()
        .map(|c| {
            // Lobbying needs no walk (DEC-051): it is checked as sent, with
            // the late flag of the view (a day's argument is refused late).
            let reason = if !here && !c.remote {
                Some("talk-why-away")
            } else {
                talk::check_choice(st, content, id, &c.id, late)
                    .err()
                    .map(|e| why(&e))
            };
            TopicView {
                id: c.id.clone(),
                name_key: c.label_key.clone(),
                mechanic: "choice".into(),
                about_required: false,
                about: Vec::new(),
                odds: c.odds,
                available: reason.is_none(),
                unavailable_key: reason.map(String::from),
                asked_this_week: false,
                name_args: Some(c.args.iter().cloned().collect()),
                say_key: Some(c.say_key.clone()),
                costs: (!c.costs.is_empty()).then(|| {
                    c.costs
                        .iter()
                        .map(|(key, args)| crate::office::ReasonView {
                            key: key.clone(),
                            args: args.iter().cloned().collect(),
                        })
                        .collect()
                }),
                bill: c.bill,
                remote: c.remote.then_some(true),
            }
        })
        .collect();
    choices.append(&mut topics);
    let topics = choices;
    Some(TalkView {
        person: person_view,
        here,
        route,
        opinion_band: opinion_band(characters::opinion(&st.politics, id, me) as f64),
        trust_band: trust_band(characters::trust(&st.politics, content, id, me) as f64),
        cost_halves: talk::cost_halves(content),
        half_slot_left,
        slots_left: pl.slots_left() as u32,
        late_night: late,
        late_night_possible: late_possible,
        out_of_time,
        topics,
        greeting,
        voice,
    })
}

/// The speaker's voice (`talk::reply::voice`) in this conversation.
fn talk_voice(st: &State, content: &ContentBundle, speaker: CharacterId) -> Option<String> {
    let pl = st.player.as_ref()?;
    let me = characters::player_character(st)?;
    let ctx = mandate_sim::talk::context::Ctx {
        st,
        content,
        town: pl.home(),
        me,
        speaker,
        about: None,
        here: pl.location,
        cap: Level::Unknown,
    };
    mandate_sim::talk::reply::voice(&ctx).map(|v| v.id.clone())
}

// --- An exchange ---------------------------------------------------------------------

/// One part of a reply as spoken: a Fluent key and its arguments (names and
/// numbers captured when it was said; `…_key` arguments are Fluent keys).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SpokenLineView {
    /// `opener` | `quote` | `substance` | `closer`.
    pub part: String,
    pub key: String,
    pub args: BTreeMap<String, String>,
}

/// Something the player learned in a conversation, as the notebook names it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LearnedItemView {
    pub subject: SubjectDto,
    /// Fluent key `notebook-<subject type>`.
    pub title_key: String,
    pub args: BTreeMap<String, String>,
    /// The level reached in this conversation.
    pub level: KnowledgeLevelDto,
}

/// A conversation the player had (`QueryDto::Exchange`; the `Talk`
/// command's `created`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ExchangeView {
    pub id: u32,
    pub date: DateView,
    pub speaker: PersonRefView,
    /// Where it happened, if at a place.
    pub place: Option<LocationView>,
    /// Topic content id, and its Fluent name key.
    pub topic: String,
    pub topic_key: String,
    /// What it was about: a name (`about_label`) or a Fluent key. With
    /// `about_label_args`, the key (with them) names it in place of the
    /// name (the town centre).
    pub about_label: Option<String>,
    pub about_label_key: Option<String>,
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub about_label_args: Option<BTreeMap<String, String>>,
    /// The reply, in order (opener, quote, substance, closer).
    pub parts: Vec<SpokenLineView>,
    /// What the player learned from it (only what was new).
    pub learned: Vec<LearnedItemView>,
    /// Leads it opened (where to go, whom to ask).
    pub leads: Vec<LeadView>,
    /// `warmer` | `cooler`, when the speaker's opinion of the player moved.
    pub relationship: Option<String>,
    /// The result in a line (`outcome-talk…`) and its arguments (`name`).
    pub outcome_key: String,
    pub outcome_args: BTreeMap<String, String>,
    /// Asked the same thing of them earlier that week.
    pub repeat: bool,
    /// The Why under the result (an agenda verb had in conversation,
    /// DEC-114): a Fluent key that takes `outcome_args`.
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why_key: Option<String>,
}

/// What a subject is called, for a choice or an exchange's "about".
fn about_label(engine: &Engine, a: &AboutRef) -> (Option<String>, Option<String>) {
    let st = engine.state();
    let content = engine.content();
    let gate = Gate::of(engine);
    match a {
        AboutRef::Place { place } => st
            .world
            .places
            .get(place.index())
            .map(|p| {
                (
                    Some(p.name.clone()),
                    Some(format!("place-{}", p.kind.key())),
                )
            })
            .unwrap_or((None, None)),
        AboutRef::Person { person } => {
            let name = characters::profile(&st.politics, &st.world, *person)
                .map(|p| gate.name(*person, p.full_name()));
            let role = crate::characters::person_ref(engine, *person).map(|r| r.role_key);
            (name, role)
        }
        AboutRef::Hood { hood } => (
            st.world
                .neighborhoods
                .get(hood.index())
                .map(|h| h.name.clone()),
            hood_centre_label(st, *hood).map(|(k, _)| k),
        ),
        AboutRef::Jurisdiction { at } => (None, Some(format!("talk-whole-{}", tier_word(*at)))),
        AboutRef::Bill { policy } => (
            None,
            st.policy
                .instances
                .get(policy.index())
                .and_then(|p| mandate_sim::policy::resolve(content, &st.policy, p))
                .map(|t| t.name_key.clone()),
        ),
        AboutRef::Template { template } => (
            None,
            content.policies.get(template).map(|t| t.name_key.clone()),
        ),
        AboutRef::Proposal { .. } => (None, None),
        AboutRef::Rival => (
            focus_town(st)
                .and_then(|town| st.politics.characters.towns.get(town.index()))
                .and_then(|t| t.rival)
                .and_then(|r| characters::profile(&st.politics, &st.world, r))
                .map(|p| p.full_name()),
            Some("ui-your-rival".into()),
        ),
    }
}

fn exchange_view_of(engine: &Engine, e: &Exchange) -> Option<ExchangeView> {
    let st = engine.state();
    let content = engine.content();
    let gate = Gate::of(engine);
    let speaker = person_ref_known(engine, &gate, e.speaker)?;
    let topic_key = content
        .talk_topic(&e.topic)
        .map(|t| t.name_key.clone())
        .unwrap_or_default();
    let (about_label, about_label_key) = e
        .about
        .as_ref()
        .map(|a| about_label(engine, a))
        .unwrap_or((None, None));
    let mut outcome_args: BTreeMap<String, String> = e.outcome_args.iter().cloned().collect();
    outcome_args.insert("name".into(), speaker.name.clone());
    let outcome_key = match (&e.outcome, e.repeat) {
        (Some(k), _) => k.clone(),
        (None, true) => "outcome-talk-again".into(),
        (None, false) => "outcome-talk".into(),
    };
    Some(ExchangeView {
        id: e.id.0,
        date: DateView::of(e.on),
        place: e
            .place
            .map(|place| location_view(st, Location::Place { place })),
        speaker,
        topic: e.topic.clone(),
        topic_key,
        about_label_args: match &e.about {
            Some(AboutRef::Hood { hood }) => hood_centre_label(st, *hood).map(|(_, a)| a),
            _ => None,
        },
        about_label,
        about_label_key,
        parts: e
            .parts
            .iter()
            .map(|p| SpokenLineView {
                part: p.part.key().into(),
                key: p.key.clone(),
                args: p.args.iter().cloned().collect(),
            })
            .collect(),
        learned: e
            .learned
            .iter()
            .map(|(s, level)| LearnedItemView {
                subject: SubjectDto::of(s),
                title_key: format!("notebook-{}", s.key().replace('_', "-")),
                args: crate::knowledge::subject_args(&gate, s, *level),
                level: (*level).into(),
            })
            .collect(),
        leads: e
            .leads
            .iter()
            .map(|l| crate::knowledge::lead_view(&gate, l))
            .collect(),
        relationship: if e.opinion >= 0.5 {
            Some("warmer".into())
        } else if e.opinion <= -0.5 {
            Some("cooler".into())
        } else {
            None
        },
        why_key: mandate_sim::actions::verbs::why_key(&e.topic, &outcome_key),
        outcome_key,
        outcome_args,
        repeat: e.repeat,
    })
}

/// A conversation by id (`QueryDto::Exchange`), while it is in the log.
pub fn exchange_view(engine: &Engine, id: u32) -> Option<ExchangeView> {
    let pl = engine.state().player.as_ref()?;
    let e = pl.talk.exchange(ExchangeId(id))?;
    exchange_view_of(engine, e)
}

/// The exchange a notebook entry learned from a conversation came from: the
/// newest with that person, on that topic and day, that taught it.
pub fn exchange_for(
    st: &State,
    person: CharacterId,
    topic: &str,
    on: mandate_core::SimDate,
    subject: &Subject,
) -> Option<u32> {
    let pl = st.player.as_ref()?;
    pl.talk
        .log
        .iter()
        .rev()
        .find(|e| {
            e.speaker == person
                && e.topic == topic
                && e.on == on
                && e.learned.iter().any(|(s, _)| s == subject)
        })
        .map(|e| e.id.0)
}

// --- Canvassing on a worry (DEC-091 §1.6) ---------------------------------------------

/// A worry the player can raise on the doorsteps of a neighbourhood.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CanvassWorryView {
    /// Concern content id: the canvass action's `issue`.
    pub id: String,
    pub name_key: String,
    /// What the player knows it from: `hood` (this neighbourhood's worries)
    /// or `town` (the town's).
    pub source: String,
    /// The neighbourhood's strongest worry, as far as the player knows.
    pub top: bool,
    /// How well it fits here, in a word (`strong` | `fair` | `weak`), once
    /// the player knows the neighbourhood's worries (`known` and up).
    pub fit_band: Option<String>,
    /// The engine's multiplier on the canvass's approval gain
    /// (`actions::canvass_match`, the command's own), shown once the player
    /// knows the neighbourhood's worries in detail.
    pub fit: Option<f64>,
}

/// The canvass preview for a neighbourhood (`QueryDto::Canvass`): the
/// worries the player knows of (DEC-091: never one they don't), each with
/// the match the engine would apply as far as they know the place.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CanvassView {
    pub neighborhood: u32,
    pub name: String,
    pub worries: Vec<CanvassWorryView>,
    /// What the player knows of this neighbourhood's worries (and the lead).
    pub knowledge: KnowledgeView,
}

/// A canvass fit in words, on the thresholds the canvass outcome uses
/// (`knowledge.effects.canvass`: halfway to the ceiling, halfway to the floor).
pub fn fit_band(content: &ContentBundle, fit: f64) -> &'static str {
    let k = &content.knowledge.effects.canvass;
    if fit > 1.0 && fit >= 1.0 + 0.5 * (k.ceiling - 1.0) {
        "strong"
    } else if fit < 1.0 && fit <= 1.0 - 0.5 * (1.0 - k.floor) {
        "weak"
    } else {
        "fair"
    }
}

pub fn canvass_view(engine: &Engine, neighborhood: u32) -> Option<CanvassView> {
    let st = engine.state();
    let content = engine.content();
    st.player.as_ref()?;
    let town = focus_town(st)?;
    let hood = NeighborhoodId(neighborhood);
    let n = st.world.neighborhoods.get(hood.index())?;
    if n.municipality != town {
        return None;
    }
    let gate = Gate::of(engine);
    let hood_subject = Subject::HoodConcerns { hood };
    let hood_level = gate.level(&hood_subject);
    let town_level = gate.town_concerns(town);
    let mine = crate::knowledge::gate_concerns(
        hood_level,
        crate::office::concerns_for(
            st,
            content,
            mandate_sim::concerns::Scope::Neighbourhood(hood),
        ),
        false,
        usize::MAX,
    );
    let theirs = crate::knowledge::gate_concerns(
        town_level,
        crate::office::concerns_for(st, content, mandate_sim::concerns::Scope::town(town)),
        false,
        usize::MAX,
    );
    let mut worries: Vec<CanvassWorryView> = Vec::new();
    for (i, c) in mine.iter().enumerate() {
        worries.push(CanvassWorryView {
            id: c.id.clone(),
            name_key: c.name_key.clone(),
            source: "hood".into(),
            top: i == 0,
            fit_band: None,
            fit: None,
        });
    }
    for c in theirs {
        if worries.iter().any(|w| w.id == c.id) {
            continue;
        }
        worries.push(CanvassWorryView {
            id: c.id,
            name_key: c.name_key,
            source: "town".into(),
            top: false,
            fit_band: None,
            fit: None,
        });
    }
    if hood_level >= Level::Known {
        for w in &mut worries {
            let fit = mandate_sim::actions::canvass_match(st, content, hood, Some(&w.id));
            w.fit_band = Some(fit_band(content, fit).into());
            if hood_level >= Level::Detailed {
                w.fit = Some(fit);
            }
        }
    }
    Some(CanvassView {
        neighborhood,
        name: n.name.clone(),
        worries,
        knowledge: gate.view(&hood_subject),
    })
}
