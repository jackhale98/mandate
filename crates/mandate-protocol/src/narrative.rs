//! Decisions, scenes, and the story so far for the client (DEC-033, DEC-077
//! §4, DEC-035, DEC-079): facts and Fluent keys only. The engine decides what
//! the player sees: a scene's conversation only where it happens.

use crate::characters::{MemoryView, PersonRefView, person_ref};
use crate::places::{LocationView, location_view};
use crate::views::{DateView, EventView};
use mandate_content::{ContentBundle, DilemmaCategory, EffectDef, Scope, VoteChoice};
use mandate_sim::Engine;
use mandate_sim::characters::{self, MemoryKind};
use mandate_sim::narrative::{
    Actor, Applied, CastEntry, Chapter, Decision, DecisionId, DecisionStatus, FactKind, Reason,
    Weight, resolve,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

/// One line of a trade-off or an aftermath: a Fluent key with arguments.
/// Arguments named `*_key` are Fluent keys themselves.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EffectLineView {
    pub key: String,
    pub args: BTreeMap<String, String>,
    /// `cost` | `gain` | `mixed`.
    pub tone: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DecisionOptionView {
    pub id: String,
    pub label_key: String,
    pub default: bool,
    pub costs: Vec<EffectLineView>,
    pub gains: Vec<EffectLineView>,
    /// Why it cannot be chosen now (Fluent key), if it cannot.
    pub unavailable_key: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct BeatView {
    /// The speaker's name (or empty for the player).
    pub speaker: String,
    pub key: String,
    /// The memory the speaker quotes (rendered first, passed as `memory`).
    pub quote: Option<MemoryView>,
}

/// Where a decision's scene happens, and the conversation if the player is there.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SceneView {
    pub place: Option<LocationView>,
    pub here: bool,
    /// Only where it happens (DEC-079: going there adds context).
    pub beats: Vec<BeatView>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AftermathView {
    pub option: String,
    pub label_key: String,
    pub key: String,
    pub by_default: bool,
    pub date: DateView,
    pub lines: Vec<EffectLineView>,
    /// The event of the decision being settled, to ask "Why?" about
    /// (`QueryDto::Why`); none once the log no longer keeps it.
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event: Option<u32>,
}

/// A decision (a dilemma, a request, or an arc's decision beat).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DecisionView {
    pub id: u32,
    pub template: String,
    pub title_key: String,
    pub situation_key: String,
    /// `climax` | `significant` | `request`.
    pub kind: String,
    /// Fluent arguments: the cast, the town, the live crisis, the player.
    pub args: BTreeMap<String, String>,
    pub people: Vec<PersonRefView>,
    pub opened: DateView,
    pub deadline: DateView,
    pub days_left: u32,
    /// `pending` | `answered` | `defaulted` | `lapsed`.
    pub status: String,
    pub options: Vec<DecisionOptionView>,
    pub default_option: String,
    /// Why this came up (Fluent keys with arguments).
    pub reasons: Vec<EffectLineView>,
    pub scene: SceneView,
    /// The arc whose beat it is (its title key).
    pub storyline_key: Option<String>,
    pub aftermath: Option<AftermathView>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ChapterView {
    pub index: u32,
    /// `session` | `campaign`.
    pub kind: String,
    pub start: DateView,
    pub end: DateView,
    /// 0..1.
    pub progress: f64,
    pub climaxes: u32,
    pub significant: u32,
    pub minor: u32,
    pub peak_tension: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct StorylineView {
    /// The arc run (`QueryDto::Arc`): its sheet reopens from the story so
    /// far (DEC-113, A3).
    pub arc: u32,
    pub title_key: String,
    /// The stages reached, as story lines (Fluent keys with the cast).
    pub stages: Vec<EventView>,
    /// `ended` | `removed`, or none while it runs.
    pub ended: Option<String>,
    pub args: BTreeMap<String, String>,
}

/// The story so far: chapter and tension, the arcs, and what happened.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct StoryView {
    pub profile_key: String,
    pub chapter: Option<ChapterView>,
    pub chapters: Vec<ChapterView>,
    /// 0..1.
    pub tension: f64,
    pub storylines: Vec<StorylineView>,
    /// Spotlit facts, newest first.
    pub entries: Vec<EventView>,
    /// Decisions taken (or let pass), newest first.
    pub past: Vec<DecisionView>,
}

fn put(args: &mut BTreeMap<String, String>, k: &str, v: String) {
    args.insert(k.to_string(), v);
}

/// Fluent arguments for a cast: `$<role>` names (a bill or law by its
/// policy name), `$<role>_role` a person's role, plus town, crisis, player.
pub fn cast_args(engine: &Engine, cast: &[CastEntry]) -> BTreeMap<String, String> {
    let st = engine.state();
    let content = engine.content();
    let mut a = BTreeMap::new();
    for e in cast {
        match e.actor {
            Actor::Person { id } => {
                if let Some(p) = person_ref(engine, id) {
                    put(&mut a, &e.role, p.name);
                    put(&mut a, &format!("{}_role_key", e.role), p.role_key);
                }
            }
            Actor::Neighborhood { id } => {
                if let Some(h) = st.world.neighborhoods.get(id as usize) {
                    put(&mut a, &e.role, h.name.clone());
                }
            }
            Actor::Policy { id } => {
                if let Some(t) = st
                    .policy
                    .instances
                    .get(id as usize)
                    .and_then(|p| mandate_sim::policy::resolve(content, &st.policy, p))
                {
                    put(&mut a, &format!("{}_key", e.role), t.name_key.clone());
                }
            }
        }
    }
    if let Some(pl) = st.player.as_ref() {
        if let Some(m) = st.world.municipalities.get(pl.municipality.index()) {
            put(&mut a, "town", m.name.clone());
        }
        if let Some(p) = st.politics.get(pl.politician) {
            put(&mut a, "player", p.full_name());
        }
        if let Some(c) = st
            .politics
            .characters
            .towns
            .get(pl.municipality.index())
            .and_then(|t| t.crisis())
            .and_then(|c| content.crises.iter().find(|d| d.id == c.kind))
        {
            put(&mut a, "crisis_key", c.name_key.clone());
        }
    }
    a
}

/// Fluent arguments for an ask or a promise (DEC-097 §2; the seam for E7's
/// `AskView`): its cast's (`cast_args`), `$<role>_occupation` for a person
/// (their occupation id without its prefix, for variants), `history` (how
/// the asker's last ask ended), `days` (to the answer or the deadline),
/// `place` (where a visit is done), `answer_key` (a promise's chosen bill),
/// `concern_key` (a statement's worry), and `constituency_key` with
/// `number` (a promise's voters).
pub fn ask_args(engine: &Engine, a: &mandate_sim::asks::Ask) -> BTreeMap<String, String> {
    use mandate_sim::asks::{Asker, Need, StatementNeed};
    let st = engine.state();
    let content = engine.content();
    let mut args = cast_args(engine, &a.cast);
    for e in &a.cast {
        if let Actor::Person { id } = e.actor
            && let Some(n) = st
                .politics
                .characters
                .get(id)
                .and_then(|c| c.named.as_deref())
        {
            let occ = n.occupation.trim_start_matches("occupation.").to_string();
            put(&mut args, &format!("{}_occupation", e.role), occ);
        }
    }
    put(&mut args, "history", a.history.key().into());
    let until = if a.offered() { a.respond_by } else { a.due };
    put(
        &mut args,
        "days",
        st.calendar.date.days_until(until).to_string(),
    );
    // The crisis mid-sentence: `ask-crisis-<kind>` ("the job losses").
    if let Some(k) = args.get("crisis_key").cloned() {
        let kind = content
            .crises
            .iter()
            .find(|c| c.name_key == k)
            .map(|c| c.id.trim_start_matches("crisis.").to_string());
        let key = kind.map(|k| format!("ask-crisis-{k}"));
        // A crisis a pack added without its phrase keeps its label.
        if let Some(key) = key.filter(|key| {
            let line = format!("{key} =");
            content
                .locales
                .get("en")
                .is_some_and(|files| files.values().any(|t| t.contains(&line)))
        }) {
            put(&mut args, "crisis_key", key);
        }
    }
    for n in &a.needs {
        match n {
            Need::Visit { place } => {
                if let Some(p) = st.world.places.get(place.index()) {
                    if p.name.is_empty() {
                        // A place with no name of its own: `ask-place-<kind>`.
                        put(
                            &mut args,
                            "place_key",
                            format!("ask-place-{}", p.kind.key()),
                        );
                    } else {
                        put(&mut args, "place", p.name.clone());
                    }
                }
            }
            Need::Statement {
                about: StatementNeed::Concern { concern },
            } => {
                if let Some(c) = content.concerns.iter().find(|c| &c.id == concern) {
                    put(&mut args, "concern_key", c.name_key.clone());
                }
            }
            _ => {}
        }
    }
    if let Some(t) = a.answer.as_deref().and_then(|t| content.policies.get(t)) {
        put(&mut args, "answer_key", t.name_key.clone());
    }
    match a.asker {
        Asker::Constituency { id } => {
            put(&mut args, "constituency_key", "ask-constituency".into());
            if let Some(c) = st.world.constituencies.get(id.index()) {
                put(&mut args, "number", c.number.to_string());
            }
        }
        Asker::Electorate { at } => {
            // Tier-generic (DEC-098): the voters of the jurisdiction the race is for.
            let j = crate::post::jurisdiction_dto(&engine.state().world, at);
            put(&mut args, "constituency_key", "ask-electorate".into());
            put(&mut args, "electorate", j.name);
            put(&mut args, "electorate_tier", j.tier);
        }
        _ => {}
    }
    args
}

/// Signed, with one decimal for small non-zero changes (so +0.5 is not "+0").
fn signed(v: f64, digits: usize) -> String {
    let digits = if v != 0.0 && v.abs() < 0.95 {
        digits.max(1)
    } else {
        digits
    };
    signed_exact(v, digits)
}

fn signed_exact(v: f64, digits: usize) -> String {
    if v > 0.0 {
        format!("+{v:.digits$}")
    } else if v < 0.0 {
        format!("−{:.digits$}", -v)
    } else {
        format!("{v:.digits$}")
    }
}

fn who_key(scope: &str, classes: &[u8]) -> String {
    if let [c] = classes {
        return format!("effect-who-class-{c}");
    }
    match scope {
        "town" => "effect-who-town",
        "hood" => "effect-who-hood",
        _ => "effect-who-ward",
    }
    .into()
}

fn line(key: &str, args: Vec<(&str, String)>, tone: &str) -> EffectLineView {
    EffectLineView {
        key: key.into(),
        args: args.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
        tone: tone.into(),
    }
}

fn approval_line(
    key: &str,
    scope: &str,
    hood: Option<String>,
    classes: &[u8],
    delta: f64,
) -> EffectLineView {
    let tone = if delta < 0.0 { "cost" } else { "gain" };
    let delta = signed(delta * 100.0, 0);
    if let Some(h) = hood {
        return line(
            "effect-approval-hood",
            vec![("delta", delta), ("hood", h)],
            tone,
        );
    }
    line(
        key,
        vec![("delta", delta), ("who_key", who_key(scope, classes))],
        tone,
    )
}

/// The policy name key of a bill or law.
fn bill_key(engine: &Engine, id: u32) -> String {
    engine
        .state()
        .policy
        .instances
        .get(id as usize)
        .and_then(|p| mandate_sim::policy::resolve(engine.content(), &engine.state().policy, p))
        .map(|t| t.name_key.clone())
        .unwrap_or_default()
}

fn name_of(engine: &Engine, id: characters::CharacterId) -> String {
    person_ref(engine, id).map(|p| p.name).unwrap_or_default()
}

/// The trade-off lines of an option, before it is chosen (DEC-073).
fn preview(engine: &Engine, d: &Decision, e: &EffectDef) -> EffectLineView {
    let st = engine.state();
    let content = engine.content();
    let person = |role: &str| mandate_sim::narrative::person_of(&d.cast, role);
    let tone = |cost: bool| if cost { "cost" } else { "gain" };
    match e {
        EffectDef::Opinion { role, delta, .. } => line(
            "effect-opinion",
            vec![
                (
                    "name",
                    person(role).map(|p| name_of(engine, p)).unwrap_or_default(),
                ),
                ("delta", signed(*delta, 0)),
            ],
            tone(*delta < 0.0),
        ),
        EffectDef::Approval {
            scope,
            classes,
            neighborhood,
            delta,
        } => {
            let hood =
                neighborhood.as_deref().and_then(|r| {
                    match mandate_sim::narrative::actor_of(&d.cast, r) {
                        Some(Actor::Neighborhood { id }) => st
                            .world
                            .neighborhoods
                            .get(id as usize)
                            .map(|h| h.name.clone()),
                        _ => None,
                    }
                });
            let scope = if hood.is_some() {
                "hood"
            } else if *scope == Scope::Town {
                "town"
            } else {
                "ward"
            };
            approval_line("effect-approval", scope, hood, classes, *delta)
        }
        EffectDef::Awareness { scope, delta } => line(
            "effect-awareness",
            vec![
                ("delta", signed(delta * 100.0, 0)),
                (
                    "who_key",
                    who_key(
                        if *scope == Scope::Town {
                            "town"
                        } else {
                            "ward"
                        },
                        &[],
                    ),
                ),
            ],
            tone(*delta < 0.0),
        ),
        EffectDef::Money { amount } => line(
            "effect-money",
            vec![("delta", signed(*amount, 0))],
            tone(*amount < 0.0),
        ),
        EffectDef::Health { delta } => line(
            "effect-health",
            vec![("delta", signed(*delta, 0))],
            tone(*delta < 0.0),
        ),
        EffectDef::Slots { n } => line("effect-slots", vec![("n", n.to_string())], "cost"),
        EffectDef::Standing { delta } => line(
            "effect-standing",
            vec![("delta", signed(*delta, 0))],
            tone(*delta < 0.0),
        ),
        EffectDef::LegalJeopardy { delta } => line(
            "effect-legal",
            vec![("delta", signed(*delta, 0))],
            tone(*delta > 0.0),
        ),
        EffectDef::Record { delta } => line(
            "effect-record",
            vec![("delta", signed(delta * 100.0, 0))],
            tone(*delta < 0.0),
        ),
        EffectDef::Favour {
            role,
            owed_to_player,
        } => line(
            if *owed_to_player {
                "effect-favour-owed-to-you"
            } else {
                "effect-favour-owed"
            },
            vec![(
                "name",
                person(role).map(|p| name_of(engine, p)).unwrap_or_default(),
            )],
            tone(!owed_to_player),
        ),
        EffectDef::Repay { role } => line(
            "effect-repaid",
            vec![(
                "name",
                person(role).map(|p| name_of(engine, p)).unwrap_or_default(),
            )],
            "mixed",
        ),
        EffectDef::Vote { role, vote } => {
            let bill = match mandate_sim::narrative::actor_of(&d.cast, role) {
                Some(Actor::Policy { id }) => st
                    .policy
                    .instances
                    .get(id as usize)
                    .and_then(|p| mandate_sim::policy::resolve(content, &st.policy, p))
                    .map(|t| t.name_key.clone())
                    .unwrap_or_default(),
                _ => String::new(),
            };
            let v = match vote {
                VoteChoice::Yes => "vote-yes",
                VoteChoice::No => "vote-no",
                VoteChoice::Abstain => "vote-abstain",
                VoteChoice::WithParty => "vote-with-party",
                VoteChoice::WithWard => "vote-with-ward",
            };
            line(
                "effect-vote",
                vec![("bill_key", bill), ("vote_key", v.into())],
                "mixed",
            )
        }
        EffectDef::Rider {
            role,
            rider,
            ward_of,
        } => {
            let bill = match mandate_sim::narrative::actor_of(&d.cast, role) {
                Some(Actor::Policy { id }) => bill_key(engine, id),
                _ => String::new(),
            };
            let whose = ward_of
                .as_deref()
                .and_then(person)
                .map(|p| name_of(engine, p));
            match (rider, whose) {
                (mandate_content::RiderKind::WardProject, Some(name)) => line(
                    "effect-rider-ward-project-for",
                    vec![("bill_key", bill), ("name", name)],
                    "mixed",
                ),
                (mandate_content::RiderKind::WardProject, None) => line(
                    "effect-rider-ward-project",
                    vec![("bill_key", bill)],
                    "mixed",
                ),
                (mandate_content::RiderKind::Sunset, _) => {
                    line("effect-rider-sunset", vec![("bill_key", bill)], "mixed")
                }
            }
        }
        EffectDef::CallIn { role, bill } => {
            let bill = match mandate_sim::narrative::actor_of(&d.cast, bill) {
                Some(Actor::Policy { id }) => bill_key(engine, id),
                _ => String::new(),
            };
            line(
                "effect-call-in",
                vec![
                    (
                        "name",
                        person(role).map(|p| name_of(engine, p)).unwrap_or_default(),
                    ),
                    ("bill_key", bill),
                ],
                "mixed",
            )
        }
        EffectDef::Draft { template, .. } => {
            match content.family(template) {
                // A family: its open proposal in the player's focus, named with
                // its slots' arguments (DEC-092 §2.19).
                Some(f) => {
                    let focus = st.focus();
                    let inst =
                        st.policy.families.instances.iter().find(|i| {
                            i.is_open() && &i.family == template && focus.contains(&i.at)
                        });
                    let mut v = line(
                        "effect-draft",
                        vec![("bill_key", f.name_key.clone())],
                        "gain",
                    );
                    // The slots' names as the player may see them.
                    if let Some((_, args)) = inst.and_then(|i| crate::proposals::title(engine, i)) {
                        for (k, a) in args {
                            v.args.entry(k).or_insert(a);
                        }
                    }
                    v
                }
                None => line(
                    "effect-draft",
                    vec![(
                        "bill_key",
                        content
                            .policies
                            .get(template)
                            .map(|t| t.name_key.clone())
                            .unwrap_or_default(),
                    )],
                    "gain",
                ),
            }
        }
        // DEC-073: the risk is shown before the player commits, as a coarse band.
        EffectDef::Hidden { severity, .. } => {
            let (Some(spec), Some(pl)) = (e.hidden_spec(), st.player.as_ref()) else {
                return line("effect-hidden", vec![], "cost");
            };
            let x = mandate_sim::media::exposure(st, content, &spec, pl.politician);
            let p = mandate_sim::media::risk_within_horizon(content, &x);
            line(
                "effect-hidden",
                vec![
                    (
                        "risk_key",
                        format!("risk-{}", mandate_sim::media::band(content, p)),
                    ),
                    ("severity_key", format!("severity-{}", severity.key())),
                ],
                "cost",
            )
        }
        EffectDef::ScandalResponse { response } => line(
            &format!("effect-respond-{}", response.key().replace('_', "-")),
            vec![],
            "mixed",
        ),
        EffectDef::LegalResponse { response, .. } => line(
            &format!("effect-legal-{}", response.key().replace('_', "-")),
            vec![],
            "mixed",
        ),
        // Knowledge (DEC-091): what kind of thing the option teaches.
        EffectDef::Learn { subject, .. } => line(
            "effect-learn",
            vec![(
                "what_key",
                format!("knows-{}", subject.key().replace('_', "-")),
            )],
            "gain",
        ),
        EffectDef::RevealPlace { .. } => line(
            "effect-learn",
            vec![("what_key", "knows-place".into())],
            "gain",
        ),
        // Cleaner air, paid for in an industry's orders (audit fix 8).
        EffectDef::Inspection { output, months, .. } => line(
            "effect-inspection",
            vec![
                ("output", format!("{:.0}", output * 100.0)),
                ("months", months.to_string()),
            ],
            "mixed",
        ),
    }
}

/// What an answered decision did (facts for the aftermath card).
fn applied_line(engine: &Engine, a: &Applied) -> EffectLineView {
    let st = engine.state();
    let content = engine.content();
    let tone = |cost: bool| if cost { "cost" } else { "gain" };
    match a {
        Applied::Opinion { person, delta } => line(
            "effect-opinion",
            vec![
                ("name", name_of(engine, *person)),
                ("delta", signed(*delta as f64, 0)),
            ],
            tone(*delta < 0.0),
        ),
        Applied::Approval {
            scope,
            neighborhood,
            classes,
            delta,
        } => approval_line(
            "effect-approval",
            scope,
            neighborhood.and_then(|h| {
                st.world
                    .neighborhoods
                    .get(h as usize)
                    .map(|x| x.name.clone())
            }),
            classes,
            *delta as f64,
        ),
        Applied::Awareness { scope, delta } => line(
            "effect-awareness",
            vec![
                ("delta", signed(*delta as f64 * 100.0, 0)),
                ("who_key", who_key(scope, &[])),
            ],
            tone(*delta < 0.0),
        ),
        Applied::Money { amount } => line(
            "effect-money",
            vec![("delta", signed(*amount, 0))],
            tone(*amount < 0.0),
        ),
        Applied::Health { delta } => line(
            "effect-health",
            vec![("delta", signed(*delta as f64, 0))],
            tone(*delta < 0.0),
        ),
        Applied::Slots { n } => line("effect-slots", vec![("n", n.to_string())], "cost"),
        Applied::Standing { delta } => line(
            "effect-standing",
            vec![("delta", signed(*delta as f64, 0))],
            tone(*delta < 0.0),
        ),
        Applied::LegalJeopardy { delta } => line(
            "effect-legal",
            vec![("delta", signed(*delta as f64, 0))],
            tone(*delta > 0.0),
        ),
        Applied::Record { delta } => line(
            "effect-record",
            vec![("delta", signed(*delta as f64 * 100.0, 0))],
            tone(*delta < 0.0),
        ),
        Applied::Favour {
            person,
            owed_to_player,
        } => line(
            if *owed_to_player {
                "effect-favour-owed-to-you"
            } else {
                "effect-favour-owed"
            },
            vec![("name", name_of(engine, *person))],
            tone(!owed_to_player),
        ),
        Applied::Repaid { person } => line(
            "effect-repaid",
            vec![("name", name_of(engine, *person))],
            "mixed",
        ),
        Applied::Vote { policy, vote } => line(
            "effect-vote",
            vec![
                (
                    "bill_key",
                    st.policy
                        .instances
                        .get(*policy as usize)
                        .and_then(|p| mandate_sim::policy::resolve(content, &st.policy, p))
                        .map(|t| t.name_key.clone())
                        .unwrap_or_default(),
                ),
                ("vote_key", format!("vote-{vote}")),
            ],
            "mixed",
        ),
        Applied::Rider { policy, rider } => line(
            &format!("effect-rider-{}", rider.replace('_', "-")),
            vec![("bill_key", bill_key(engine, *policy))],
            "mixed",
        ),
        Applied::CalledIn { person, policy } => line(
            "effect-call-in",
            vec![
                ("name", name_of(engine, *person)),
                ("bill_key", bill_key(engine, *policy)),
            ],
            "mixed",
        ),
        Applied::Drafted { policy } => line(
            "effect-draft",
            vec![(
                "bill_key",
                st.policy
                    .instances
                    .get(*policy as usize)
                    .and_then(|p| mandate_sim::policy::resolve(content, &st.policy, p))
                    .map(|t| t.name_key.clone())
                    .unwrap_or_default(),
            )],
            "gain",
        ),
        Applied::Hidden { risk, .. } => line(
            "effect-hidden-done",
            vec![(
                "risk_key",
                format!("risk-{}", mandate_sim::media::band(content, *risk as f64)),
            )],
            "cost",
        ),
        Applied::Responded { response } => line(
            "effect-responded",
            vec![(
                "response_key",
                format!("response-{}", response.replace('_', "-")),
            )],
            "mixed",
        ),
        Applied::Skipped { effect } => {
            line("effect-skipped", vec![("what", effect.clone())], "mixed")
        }
        // Knowledge (DEC-091): facts only; the notebook (K2/K5) shows what.
        Applied::Learned { subject, n, .. } => line(
            "effect-learned",
            vec![
                ("what_key", format!("knows-{}", subject.replace('_', "-"))),
                ("n", n.to_string()),
            ],
            "gain",
        ),
    }
}

fn reason_line(engine: &Engine, d: &Decision, r: &Reason) -> Option<EffectLineView> {
    let content = engine.content();
    let town = cast_args(engine, &d.cast)
        .get("town")
        .cloned()
        .unwrap_or_default();
    Some(match r {
        Reason::Crisis { kind, .. } => line(
            "why-crisis",
            vec![
                (
                    "crisis_key",
                    content
                        .crises
                        .iter()
                        .find(|c| &c.id == kind)?
                        .name_key
                        .clone(),
                ),
                ("town", town),
            ],
            "mixed",
        ),
        Reason::Concern { concern } => line(
            "why-concern",
            vec![(
                "concern_key",
                content
                    .concerns
                    .iter()
                    .find(|c| &c.id == concern)?
                    .name_key
                    .clone(),
            )],
            "mixed",
        ),
        Reason::Election { .. } => line("why-election", vec![], "mixed"),
        Reason::Arc { arc } => line(
            "why-storyline",
            vec![("title_key", content.arc(arc)?.title_key.clone())],
            "mixed",
        ),
        Reason::Memory { person } => line(
            "why-memory",
            vec![("name", name_of(engine, *person))],
            "mixed",
        ),
        Reason::Goal { person, goal } => line(
            "why-goal",
            vec![
                ("name", name_of(engine, *person)),
                ("goal_key", content.goal(goal)?.name_key.clone()),
            ],
            "mixed",
        ),
        Reason::Ward => line("why-ward", vec![], "mixed"),
        Reason::Pressure { track } => line(
            "why-pressure",
            vec![("track_key", format!("track-{track}"))],
            "mixed",
        ),
        Reason::Opportunity => return None,
    })
}

fn weight_key(w: Weight, c: DilemmaCategory) -> &'static str {
    match (w, c) {
        (Weight::Climax, _) => "climax",
        (_, DilemmaCategory::Request) => "request",
        _ => "significant",
    }
}

fn scene_view(engine: &Engine, d: &Decision) -> SceneView {
    let st = engine.state();
    let content = engine.content();
    let place = d.place.map(|l| location_view(st, l));
    let here = d.place.is_some() && st.player.as_ref().map(|p| p.location) == d.place;
    let mut beats = Vec::new();
    let me = characters::player_character(st);
    if here
        && let Some(sc) = content
            .dilemma(&d.template)
            .and_then(|t| t.scene.as_deref())
            .and_then(|s| content.scene(s))
    {
        for b in &sc.beats {
            let speaker = mandate_sim::narrative::person_of(&d.cast, &b.speaker);
            let name = speaker.map(|p| name_of(engine, p)).unwrap_or_default();
            let (key, quote) = match (&b.quote_fallback, speaker, me) {
                (Some(fallback), Some(p), Some(me)) => {
                    // Quoted memories ("You promised the bridge in '31").
                    match characters::salient_memories(
                        &st.politics,
                        p,
                        Some(me),
                        st.calendar.date,
                        1,
                    )
                    .into_iter()
                    .next()
                    {
                        Some(m) => (
                            b.line.clone(),
                            Some(MemoryView {
                                key: MemoryKind::fluent_key(m.kind),
                                subject: crate::characters::subject_view(engine, m.subject),
                                year: m.date.calendar().year,
                                tone: crate::characters::tone(m.sentiment),
                            }),
                        ),
                        None => (fallback.clone(), None),
                    }
                }
                (Some(fallback), _, _) => (fallback.clone(), None),
                (None, _, _) => (b.line.clone(), None),
            };
            beats.push(BeatView {
                speaker: name,
                key,
                quote,
            });
        }
    }
    SceneView { place, here, beats }
}

fn status_key(s: DecisionStatus) -> &'static str {
    match s {
        DecisionStatus::Pending => "pending",
        DecisionStatus::Answered => "answered",
        DecisionStatus::Defaulted => "defaulted",
        DecisionStatus::Lapsed => "lapsed",
    }
}

fn unavailable(st: &mandate_sim::State, effects: &[EffectDef]) -> Option<String> {
    match resolve::affordable(st, effects) {
        Ok(()) => None,
        Err(e) if e.contains("time") => Some("ui-not-enough-time".into()),
        Err(_) => Some("ui-not-enough-money".into()),
    }
}

/// A decision as the player sees it.
pub fn decision_view(engine: &Engine, d: &Decision) -> Option<DecisionView> {
    let st = engine.state();
    let content: &ContentBundle = engine.content();
    let def = content.dilemma(&d.template)?;
    let pending = d.status == DecisionStatus::Pending;
    let options = def
        .options
        .iter()
        .map(|o| {
            let (mut costs, mut gains) = (Vec::new(), Vec::new());
            for e in &o.effects {
                // A response to a story or a case: what it buys and what it risks.
                let response = match e {
                    EffectDef::ScandalResponse { response } => Some(format!(
                        "effect-respond-{}",
                        response.key().replace('_', "-")
                    )),
                    EffectDef::LegalResponse { response, .. } => {
                        Some(format!("effect-legal-{}", response.key().replace('_', "-")))
                    }
                    _ => None,
                };
                if let Some(k) = response {
                    gains.push(line(&format!("{k}-gain"), vec![], "gain"));
                    costs.push(line(&format!("{k}-cost"), vec![], "cost"));
                    continue;
                }
                let l = preview(engine, d, e);
                match e.valence() {
                    (true, false) => costs.push(l),
                    (false, true) => gains.push(l),
                    _ => {
                        costs.push(l.clone());
                        gains.push(l);
                    }
                }
            }
            // The tone of a one-sided effect follows its sign.
            costs.retain(|l| l.tone != "gain");
            gains.retain(|l| l.tone != "cost");
            DecisionOptionView {
                id: o.id.clone(),
                label_key: o.label_key.clone(),
                default: o.default,
                costs,
                gains,
                unavailable_key: if pending && !o.default {
                    unavailable(st, &o.effects)
                } else {
                    None
                },
            }
        })
        .collect();
    let people = d
        .cast
        .iter()
        .filter_map(|e| match e.actor {
            Actor::Person { id } => person_ref(engine, id),
            _ => None,
        })
        .collect();
    let aftermath = d.resolution.as_ref().filter(|_| !pending).and_then(|r| {
        let o = def.options.iter().find(|o| o.id == r.option)?;
        Some(AftermathView {
            option: o.id.clone(),
            label_key: o.label_key.clone(),
            key: o.aftermath_key.clone(),
            by_default: r.by_default,
            date: DateView::of(r.date),
            lines: r.applied.iter().map(|a| applied_line(engine, a)).collect(),
            event: crate::why::thread_event(
                &st.events,
                mandate_sim::spine::Thread::Decision(d.id.0),
                |k| matches!(k, mandate_sim::events::EventKind::DecisionResolved { .. }),
            ),
        })
    });
    let mut args = cast_args(engine, &d.cast);
    // A raised decision names what came out and who ran it.
    let about = match d.about {
        Some(mandate_sim::narrative::About::Scandal { id }) => st
            .media
            .scandal(id)
            .map(|s| (s.kind.clone(), s.severity, s.outlet)),
        Some(mandate_sim::narrative::About::Case { id }) => st
            .justice
            .case(id)
            .map(|c| (c.kind.clone(), c.severity, None)),
        None => None,
    };
    if let Some((kind, sev, outlet)) = about {
        args.insert(
            "fact_key".into(),
            format!("fact-{}", kind.replace('_', "-")),
        );
        args.insert("severity_key".into(), format!("severity-{}", sev.key()));
        if let Some(o) = outlet.and_then(|o| st.media.outlet(o)) {
            let (k, place) = mandate_sim::media::outlet_name(content, o);
            args.insert("outlet_key".into(), k);
            args.insert("outlet_place".into(), place);
        }
    }
    Some(DecisionView {
        id: d.id.0,
        template: d.template.clone(),
        title_key: def.title_key.clone(),
        situation_key: def.situation_key.clone(),
        kind: weight_key(d.weight, def.category).into(),
        args,
        people,
        opened: DateView::of(d.opened),
        deadline: DateView::of(d.deadline),
        days_left: st.calendar.date.days_until(d.deadline),
        status: status_key(d.status).into(),
        options,
        default_option: def
            .options
            .iter()
            .find(|o| o.default)
            .map(|o| o.id.clone())
            .unwrap_or_default(),
        reasons: d
            .reasons
            .iter()
            .filter_map(|r| reason_line(engine, d, r))
            .collect(),
        scene: scene_view(engine, d),
        // The arc whose beat it is (storylines are arcs, DEC-109).
        storyline_key: d.arc.as_ref().and_then(|a| {
            let template = match st.arcs.run(a.run) {
                Some(r) => &r.template,
                None => &st.arcs.past.iter().find(|p| p.id == a.run)?.template,
            };
            content.arc(template).map(|s| s.title_key.clone())
        }),
        aftermath,
    })
}

/// Pending decisions, soonest deadline first.
pub fn pending_decisions(engine: &Engine) -> Vec<DecisionView> {
    let mut v: Vec<&Decision> = engine.state().narrative.pending().collect();
    v.sort_by_key(|d| (d.deadline, d.id));
    v.into_iter()
        .filter_map(|d| decision_view(engine, d))
        .collect()
}

pub fn decision_by_id(engine: &Engine, id: u32) -> Option<DecisionView> {
    let d = engine.state().narrative.decision(DecisionId(id))?;
    decision_view(engine, d)
}

fn chapter_view(c: &Chapter, on: mandate_core::SimDate) -> ChapterView {
    ChapterView {
        index: c.index,
        kind: c.kind.key().into(),
        start: DateView::of(c.start),
        end: DateView::of(c.closed.unwrap_or(c.end)),
        progress: if c.closed.is_some() {
            1.0
        } else {
            c.progress(on)
        },
        climaxes: c.climaxes as u32,
        significant: c.significant as u32,
        minor: c.minor as u32,
        peak_tension: c.peak_tension as f64,
    }
}

/// A fact as a story line (Fluent key and arguments), if it is one the
/// player sees.
pub fn fact_event(engine: &Engine, f: &mandate_sim::narrative::Fact) -> Option<EventView> {
    let st = engine.state();
    let content = engine.content();
    let mut args = BTreeMap::new();
    let town = st
        .player
        .as_ref()
        .and_then(|p| st.world.municipalities.get(p.municipality.index()))
        .map(|m| m.name.clone())
        .unwrap_or_default();
    let key: String = match &f.kind {
        FactKind::CrisisBegan { kind } | FactKind::CrisisWorse { kind } => {
            let c = content.crises.iter().find(|c| &c.id == kind)?;
            put(&mut args, "crisis_key", c.name_key.clone());
            put(&mut args, "town", town);
            if matches!(f.kind, FactKind::CrisisBegan { .. }) {
                "event-development-crisis".into()
            } else {
                "event-development-crisis-worse".into()
            }
        }
        FactKind::GoalTaken { person, goal } => {
            let p = person_ref(engine, *person)?;
            put(&mut args, "name", p.name);
            put(&mut args, "role_key", p.role_key);
            put(&mut args, "goal_key", content.goal(goal)?.name_key.clone());
            "event-development-goal".into()
        }
        FactKind::RivalNamed { person } => {
            put(&mut args, "name", name_of(engine, *person));
            "event-development-rival".into()
        }
        FactKind::DecisionOpened { decision } => {
            let d = st.narrative.decision(*decision)?;
            put(
                &mut args,
                "title_key",
                content.dilemma(&d.template)?.title_key.clone(),
            );
            "event-decision-opened".into()
        }
        FactKind::DecisionResolved {
            decision,
            by_default,
            ..
        } => {
            let d = st.narrative.decision(*decision)?;
            put(
                &mut args,
                "title_key",
                content.dilemma(&d.template)?.title_key.clone(),
            );
            if *by_default {
                "event-decision-defaulted".into()
            } else {
                "event-decision-answered".into()
            }
        }
        FactKind::TrackWarning { track, .. } => {
            put(&mut args, "track_key", format!("track-{track}"));
            "event-track-warning".into()
        }
        FactKind::ChapterBegan { kind, .. } => {
            put(
                &mut args,
                "kind_key",
                format!("chapter-kind-{}", kind.key()),
            );
            "event-chapter-began".into()
        }
        FactKind::TownEvent { event } => {
            let e = st.events.get(*event)?;
            let v = crate::views::event_view(e, engine);
            return Some(EventView {
                id: f.id,
                date: v.date,
                text_key: v.text_key,
                args: v.args,
            });
        }
        FactKind::DecisionLapsed { .. } | FactKind::Discoverable { .. } => return None,
    };
    Some(EventView {
        id: f.id,
        date: DateView::of(f.date),
        text_key: key,
        args,
    })
}

/// The arcs as the story sheet lists them, newest first (storylines are
/// arcs, DEC-109; until the arc view of task A3 this is their surface):
/// the runs in progress, then the last that ended, each with the stages
/// it reached as the event log holds them (a stage's story so far, an
/// exit's how it ended), with the facts as they were.
fn arc_stories(engine: &Engine) -> Vec<StorylineView> {
    use mandate_sim::events::EventKind;
    let st = engine.state();
    let content = engine.content();
    let live = st
        .arcs
        .runs
        .iter()
        .rev()
        .map(|r| (r.id, &r.template, None::<&str>));
    let ended = st
        .arcs
        .past
        .iter()
        .rev()
        .map(|p| (p.id, &p.template, Some(p.exit.as_str())));
    live.chain(ended)
        .take(12)
        .filter_map(|(id, template, exit)| {
            let def = content.arc(template)?;
            let stages: Vec<EventView> = st
                .events
                .iter()
                .filter(|e| {
                    matches!(&e.kind,
                        EventKind::ArcStage { arc, .. } | EventKind::ArcEnded { arc, .. }
                            if *arc == id.0)
                })
                .map(|e| crate::views::event_view(e, engine))
                .collect();
            let args = stages.last().map(|e| e.args.clone()).unwrap_or_default();
            Some(StorylineView {
                arc: id.0,
                title_key: def.title_key.clone(),
                stages,
                ended: exit.map(|x| {
                    if x == mandate_sim::arcs::EXIT_REMOVED {
                        "removed"
                    } else {
                        "ended"
                    }
                    .into()
                }),
                args,
            })
        })
        .collect()
}

/// The story so far.
pub fn story_view(engine: &Engine) -> StoryView {
    let st = engine.state();
    let content = engine.content();
    let n = &st.narrative;
    let on = st.calendar.date;
    let storylines = arc_stories(engine);
    StoryView {
        profile_key: n
            .profile(content)
            .map(|p| p.name_key.clone())
            .unwrap_or_default(),
        chapter: n.chapter.as_ref().map(|c| chapter_view(c, on)),
        chapters: n
            .chapters
            .iter()
            .rev()
            .take(12)
            .map(|c| chapter_view(c, on))
            .collect(),
        tension: n.tension as f64,
        storylines,
        entries: n
            .facts
            .iter()
            .rev()
            .filter(|f| f.spotlit)
            .filter_map(|f| fact_event(engine, f))
            .take(40)
            .collect(),
        past: n
            .decisions
            .iter()
            .rev()
            .filter(|d| d.status != DecisionStatus::Pending && d.status != DecisionStatus::Lapsed)
            .take(20)
            .filter_map(|d| decision_view(engine, d))
            .collect(),
    }
}
