//! The content workbench's explanations (owner, 2026-11-19): for a content
//! item, whether it could fire right now and why, as a tree of the
//! conditions it needs, each checked by the **real evaluator**, with the
//! values that evaluator reads. Read-only: nothing here writes state or
//! draws from a stream, so it is safe in views, the lab and tests.
//!
//! **Three condition languages, three adapters.** The narrative
//! `Condition`, `SituationCondition` (policies and conversations) and
//! `HeardCondition` (overheard talk) each implement [`Explainer`]: the
//! verdict comes from the language's own evaluator (`cast::holds`,
//! `policy::situation_holds` / `talk::context::Ctx::holds`,
//! `overheard::Ctx::holds`); the adapter only adds what was read, for
//! display. When the languages are unified (DEC-098 §5, task U1), one
//! adapter replaces the three and the item explanations below stay.
//!
//! **What "could fire now" means** per kind (the gates each kind's system
//! applies before its conditions, then the conditions):
//! - dilemma: its route (spotlight, request, storyline stage, raised),
//!   cooldown, not already pending, then casting and triggers exactly as the
//!   director's `evaluate` does. The director's pacing (budgets, spacing,
//!   relevance) is shown but does not decide the verdict;
//! - situational policy: its unlock (any condition) in the player's town;
//! - storyline: the next step (start, or the current stage's advance);
//! - overheard line: it holds at some place in town where people talk;
//! - topic, line, voice: it holds for some speaker in town (place and
//!   cost aside);
//! - scene: a dilemma that presents it could fire, and its place resolves.

use crate::characters::{self, CharacterId};
use crate::narrative::cast::{self, View};
use crate::narrative::{Actor, CastEntry};
use crate::places::Location;
use crate::scheduler::Reads;
use crate::state::State;
use crate::talk::AboutRef;
use crate::talk::context::{Args, Ctx as TalkCtx};
use crate::world::{MunicipalityId, NeighborhoodId};
use mandate_content::conversation::{AboutKind, ConcernOf, TalkPart};
use mandate_content::{
    Condition, ContentBundle, DilemmaCategory, DilemmaDef, HeardCondition, RaisedBy,
    SituationCondition, WaterFeature,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// --- The tree ------------------------------------------------------------------------

/// One node of an explanation: a gate, a role, a group, or a condition.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Node {
    /// `all` | `any` | `condition` | `gate` | `role` | `place` | `speaker` |
    /// `stage` | `info` (shown, never part of the verdict).
    pub kind: String,
    /// The condition type (`track`, `crisis`) or the gate's name.
    pub label: String,
    /// The parameters as content writes them (`track=legal_jeopardy, min=60`).
    pub detail: String,
    /// The real evaluator's verdict (an `info` node's is informative only).
    pub ok: bool,
    /// What was read, as (name, value).
    pub values: Vec<(String, String)>,
    pub children: Vec<Node>,
}

impl Node {
    pub fn new(kind: &str, label: &str, detail: impl Into<String>, ok: bool) -> Self {
        Node {
            kind: kind.into(),
            label: label.into(),
            detail: detail.into(),
            ok,
            values: Vec::new(),
            children: Vec::new(),
        }
    }

    pub fn with(mut self, name: &str, value: impl ToString) -> Self {
        self.values.push((name.into(), value.to_string()));
        self
    }

    pub fn group(kind: &str, label: &str, children: Vec<Node>) -> Self {
        let ok = children.iter().filter(|c| c.kind != "info").all(|c| c.ok);
        Node {
            kind: kind.into(),
            label: label.into(),
            detail: String::new(),
            ok,
            values: Vec::new(),
            children,
        }
    }

    /// The first failing node, depth first (the "why not").
    pub fn first_failure(&self) -> Option<&Node> {
        if self.ok || self.kind == "info" {
            return None;
        }
        self.children
            .iter()
            .filter(|c| c.kind != "info")
            .find_map(|c| c.first_failure())
            .or(Some(self))
    }
}

/// The adapter each condition language implements (DEC-098 §5: U1 makes
/// this one adapter). `holds` must be the language's own evaluator.
pub trait Explainer {
    type Cond: Serialize;
    /// The real evaluator's verdict on one condition.
    fn holds(&self, c: &Self::Cond) -> bool;
    /// What the evaluator reads for this condition, for display.
    fn observe(&self, c: &Self::Cond) -> Vec<(String, String)>;
    /// The alternatives of an `any` condition.
    fn any_of<'c>(&self, c: &'c Self::Cond) -> Option<&'c [Self::Cond]>;
}

/// A condition's type and its parameters, as content writes them.
pub fn describe<C: Serialize>(c: &C) -> (String, String) {
    let v = serde_json::to_value(c).unwrap_or_default();
    let label = v
        .get("type")
        .and_then(|t| t.as_str())
        .unwrap_or("?")
        .to_string();
    let detail = match v {
        serde_json::Value::Object(m) => m
            .into_iter()
            .filter(|(k, _)| k != "type" && k != "of")
            .map(|(k, x)| match x {
                serde_json::Value::String(s) => format!("{k}={s}"),
                other => format!("{k}={other}"),
            })
            .collect::<Vec<_>>()
            .join(", "),
        _ => String::new(),
    };
    (label, detail)
}

/// Explain one condition (and an `any`'s alternatives).
pub fn explain_one<E: Explainer>(e: &E, c: &E::Cond) -> Node {
    let (label, detail) = describe(c);
    let children: Vec<Node> = e
        .any_of(c)
        .map(|of| of.iter().map(|x| explain_one(e, x)).collect())
        .unwrap_or_default();
    Node {
        kind: if e.any_of(c).is_some() {
            "any".into()
        } else {
            "condition".into()
        },
        label,
        detail,
        ok: e.holds(c),
        values: e.observe(c),
        children,
    }
}

/// Explain a list of conditions that must all hold (every language's
/// `when`, `triggers`, `start` and `advance` are conjunctions).
pub fn explain_all<E: Explainer>(e: &E, label: &str, cs: &[E::Cond]) -> Node {
    Node::group("all", label, cs.iter().map(|c| explain_one(e, c)).collect())
}

fn num(v: f64) -> String {
    let r = (v * 1000.0).round() / 1000.0;
    format!("{r}")
}

fn name(st: &State, c: CharacterId) -> String {
    characters::profile(&st.politics, &st.world, c)
        .map(|p| p.full_name())
        .unwrap_or_else(|| format!("#{}", c.0))
}

/// The player's town as the evaluators read it (`View::town`: R1 moves this
/// to the player's home or focus; nothing here reads the player's town
/// directly).
fn home(v: &View<'_>) -> Option<u32> {
    v.player().map(|p| p.home().0)
}

// --- Adapter 1: narrative conditions --------------------------------------------------

/// Narrative `Condition` with a cast (dilemmas, goals, storylines, scenes).
pub struct NarrativeExplainer<'a, 'v> {
    pub v: &'a View<'v>,
    pub cast: &'a [CastEntry],
}

impl Explainer for NarrativeExplainer<'_, '_> {
    type Cond = Condition;

    fn holds(&self, c: &Condition) -> bool {
        cast::holds(self.v, c, self.cast)
    }

    fn any_of<'c>(&self, _: &'c Condition) -> Option<&'c [Condition]> {
        None
    }

    fn observe(&self, c: &Condition) -> Vec<(String, String)> {
        let v = self.v;
        let st_pol = v.pol;
        let me = v.me();
        let who = |role: &str| crate::narrative::person_of(self.cast, role);
        let town = v.town_character();
        let mut out: Vec<(String, String)> = Vec::new();
        let mut put = |k: &str, x: String| out.push((k.into(), x));
        if let Some(r) = c.roles().first()
            && let Some(a) = crate::narrative::actor_of(self.cast, r)
        {
            put(r, actor_label(v, a));
        }
        match c {
            Condition::Crisis { kind, .. } => put(
                "severity",
                town.and_then(|t| t.crises.iter().find(|x| &x.kind == kind))
                    .map(|x| num(x.severity as f64))
                    .unwrap_or_else(|| "no such crisis".into()),
            ),
            Condition::LiveCrisis { .. } => put(
                "live crisis",
                town.and_then(|t| t.crisis())
                    .map(|x| format!("{} {}", x.kind, num(x.severity as f64)))
                    .unwrap_or_else(|| "none".into()),
            ),
            Condition::Indicator { indicator, .. } => {
                if let Some(m) = home(v) {
                    put(
                        "value",
                        num(crate::concerns::indicator(v.r, m as usize, *indicator)),
                    );
                }
            }
            Condition::Concern { .. } => put(
                "priorities",
                town.map(|t| t.priorities.join(", ")).unwrap_or_default(),
            ),
            Condition::Industry { .. } => put(
                "industry",
                town.and_then(|t| t.industry.as_deref())
                    .and_then(|s| v.content().sector(s))
                    .map(|s| format!("{} ({:?})", s.id, s.driver))
                    .unwrap_or_else(|| "none".into()),
            ),
            Condition::ElectionWithin { .. } => put(
                "days to election",
                v.next_election()
                    .map(|d| v.date().days_until(d).to_string())
                    .unwrap_or_else(|| "none".into()),
            ),
            Condition::Campaigning { .. } => put("campaigning", v.campaigning().to_string()),
            // The player's post as its role word in the run's constitution.
            Condition::Office { .. } => put(
                "post",
                v.player()
                    .and_then(|pl| st_pol.get(pl.politician))
                    .map(|p| {
                        v.content()
                            .constitutions
                            .get(&v.r.meta.constitution)
                            .and_then(|c| crate::politics::role_word(c, p))
                            .unwrap_or("out of office")
                            .to_string()
                    })
                    .unwrap_or_else(|| "no player".into()),
            ),
            Condition::Track { track, .. } => put(
                "value",
                track_value(v, *track)
                    .map(num)
                    .unwrap_or_else(|| "-".into()),
            ),
            Condition::Money { .. } => put(
                "funds",
                v.player()
                    .map(|p| p.money.units().to_string())
                    .unwrap_or_default(),
            ),
            Condition::Opinion { role, .. } => {
                if let (Some(p), Some(me)) = (who(role), me) {
                    put("opinion", num(characters::opinion(st_pol, p, me) as f64));
                }
            }
            Condition::Favour { role, .. } => {
                if let (Some(p), Some(me)) = (who(role), me) {
                    put(
                        "player owes",
                        characters::favours_owed(st_pol, me, p).to_string(),
                    );
                    put(
                        "owed to player",
                        characters::favours_owed(st_pol, p, me).to_string(),
                    );
                }
            }
            Condition::Remembers { role, .. } => {
                if let (Some(p), Some(me)) = (who(role), me) {
                    let kinds: Vec<&str> = st_pol
                        .characters
                        .get(p)
                        .map(|c| {
                            c.memories
                                .iter()
                                .filter(|m| m.actor == me)
                                .map(|m| m.kind.key())
                                .collect()
                        })
                        .unwrap_or_default();
                    put("memories of player", kinds.join(", "));
                }
            }
            Condition::Personality { role, axis, .. } => {
                if let Some(p) = who(role).and_then(|p| characters::profile(st_pol, v.r.world, p)) {
                    let x = &p.personality;
                    let val = match axis.as_str() {
                        "ambition" => x.ambition,
                        "integrity" => x.integrity,
                        "warmth" => x.warmth,
                        "boldness" => x.boldness,
                        _ => x.loyalty,
                    };
                    put(axis, num(val as f64));
                }
            }
            Condition::Trait { role, .. } => put(
                "traits",
                who(role)
                    .and_then(|p| st_pol.characters.get(p))
                    .map(|c| c.traits.join(", "))
                    .unwrap_or_default(),
            ),
            Condition::LawInForce { templates, .. } => {
                let n = home(v)
                    .map(|m| {
                        v.r.policy()
                            .in_municipality(m)
                            .filter(|p| {
                                matches!(
                                    p.status,
                                    crate::policy::PolicyStatus::Enacted
                                        | crate::policy::PolicyStatus::InEffect
                                ) && templates.contains(&p.template)
                            })
                            .count()
                    })
                    .unwrap_or(0);
                put("laws in force", n.to_string());
            }
            Condition::PartyLineAgainstWard { role } => {
                if let Some(Actor::Policy { id }) = crate::narrative::actor_of(self.cast, role) {
                    put("party line", v.party_line(id).to_string());
                    put("ward lean", num(v.ward_lean(id)));
                }
            }
            Condition::CanVote { role } => {
                if let Some(Actor::Policy { id }) = crate::narrative::actor_of(self.cast, role) {
                    put("can vote", v.can_vote(id).to_string());
                }
            }
            Condition::Goal { role, .. } => put(
                "goals",
                who(role)
                    .map(|p| {
                        st_pol
                            .characters
                            .goals
                            .iter()
                            .filter(|g| g.holder == p)
                            .map(|g| g.goal.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_default(),
            ),
            Condition::MonthsInPlay { .. } => {
                let began = v.play_began();
                let months = if v.date() >= began {
                    began.days_until(v.date()) / 30
                } else {
                    0
                };
                put("months in play", months.to_string());
            }
            Condition::Knows { .. } => {}
        }
        out
    }
}

/// A track's value as `cast::holds` reads it (display only; the verdict is
/// the evaluator's).
fn track_value(v: &View<'_>, t: mandate_content::TrackKey) -> Option<f64> {
    use mandate_content::TrackKey as K;
    let pl = v.player()?;
    let x = &pl.tracks;
    Some(match t {
        K::Viability => x.viability as f64,
        K::LegalJeopardy => x.legal_jeopardy as f64,
        K::Threat => x.threat as f64,
        K::Standing => x.standing as f64,
        K::Health => x.health as f64,
        K::Mandate => pl.mandate as f64,
        K::Record => v.pol.get(pl.politician).map(|p| p.record as f64)? * 100.0,
    })
}

/// A cast actor as the workbench shows it.
pub fn actor_label(v: &View<'_>, a: Actor) -> String {
    match a {
        Actor::Person { id } => characters::profile(v.pol, v.r.world, id)
            .map(|p| p.full_name())
            .unwrap_or_else(|| format!("person #{}", id.0)),
        Actor::Neighborhood { id } => {
            v.r.world
                .neighborhoods
                .get(id as usize)
                .map(|h| h.name.clone())
                .unwrap_or_else(|| format!("neighbourhood #{id}"))
        }
        Actor::Policy { id } => {
            v.r.policy()
                .instances
                .get(id as usize)
                .map(|p| format!("{} #{id}", p.template))
                .unwrap_or_else(|| format!("policy #{id}"))
        }
    }
}

// --- Adapter 2: situation conditions --------------------------------------------------

/// Where a `SituationCondition` is read: a town's situation (a policy's
/// unlock) or a conversation (its slots).
pub enum SituationExplainer<'a> {
    Town {
        st: &'a State,
        content: &'a ContentBundle,
        m: u32,
    },
    Talk(&'a TalkCtx<'a>),
}

impl SituationExplainer<'_> {
    fn st(&self) -> &State {
        match self {
            SituationExplainer::Town { st, .. } => st,
            SituationExplainer::Talk(c) => c.st,
        }
    }

    fn content(&self) -> &ContentBundle {
        match self {
            SituationExplainer::Town { content, .. } => content,
            SituationExplainer::Talk(c) => c.content,
        }
    }

    fn town(&self) -> u32 {
        match self {
            SituationExplainer::Town { m, .. } => *m,
            SituationExplainer::Talk(c) => c.town.0,
        }
    }
}

impl Explainer for SituationExplainer<'_> {
    type Cond = SituationCondition;

    fn holds(&self, c: &SituationCondition) -> bool {
        match self {
            SituationExplainer::Town { st, content, m } => {
                crate::policy::situation_holds(st, content, c, *m)
            }
            SituationExplainer::Talk(ctx) => ctx.holds(c, &mut Args::new()),
        }
    }

    fn any_of<'c>(&self, c: &'c SituationCondition) -> Option<&'c [SituationCondition]> {
        match c {
            SituationCondition::Any { of } => Some(of),
            _ => None,
        }
    }

    fn observe(&self, c: &SituationCondition) -> Vec<(String, String)> {
        use SituationCondition as U;
        let st = self.st();
        let content = self.content();
        let m = self.town();
        let pol = &st.politics;
        let town = pol.characters.towns.get(m as usize);
        let mut out: Vec<(String, String)> = Vec::new();
        let mut put = |k: &str, x: String| out.push((k.into(), x));
        let ctx = match self {
            SituationExplainer::Talk(c) => Some(*c),
            SituationExplainer::Town { .. } => None,
        };
        // The person in a slot (conversations only).
        let slot_person = |slot: &str| ctx.and_then(|c| c.person(slot));
        match c {
            U::Crisis { kind, .. } => put(
                "severity",
                town.and_then(|t| t.crises.iter().find(|x| &x.kind == kind))
                    .map(|x| num(x.severity as f64))
                    .unwrap_or_else(|| "no such crisis".into()),
            ),
            U::Indicator { indicator, .. } => {
                let r = Reads::of_state(st, content);
                put(
                    "value",
                    num(crate::concerns::indicator(&r, m as usize, *indicator)),
                );
            }
            U::PlantClosure { .. } => put(
                "last closure",
                st.events
                    .iter()
                    .rev()
                    .find(|e| {
                        matches!(e.kind, crate::events::EventKind::PlantClosure { municipality, .. } if municipality == m)
                    })
                    .map(|e| format!("{} days ago", e.date.days_until(st.calendar.date)))
                    .unwrap_or_else(|| "none on record".into()),
            ),
            U::Disaster { .. } => put(
                "months since disaster",
                st.economy
                    .munis
                    .get(m as usize)
                    .map(|e| e.months_since_disaster.to_string())
                    .unwrap_or_default(),
            ),
            U::Occupation { slot, .. } => put(
                slot,
                slot_person(slot)
                    .map(|p| {
                        format!(
                            "{} ({})",
                            name(st, p),
                            characters::knows::role_key(st, content, p).unwrap_or_else(|| "-".into())
                        )
                    })
                    .unwrap_or_else(|| "nobody".into()),
            ),
            U::Trait { slot, .. } => put(
                "traits",
                slot_person(slot)
                    .and_then(|p| pol.characters.get(p))
                    .map(|c| c.traits.join(", "))
                    .unwrap_or_default(),
            ),
            U::Personality { slot, axis, .. } => {
                if let Some(p) = slot_person(slot).and_then(|p| characters::profile(pol, &st.world, p))
                {
                    let x = &p.personality;
                    let val = match axis.as_str() {
                        "ambition" => Some(x.ambition),
                        "integrity" => Some(x.integrity),
                        "warmth" => Some(x.warmth),
                        "boldness" => Some(x.boldness),
                        "loyalty" => Some(x.loyalty),
                        _ => None,
                    };
                    put(
                        axis,
                        val.map(|v| num(v as f64))
                            .unwrap_or_else(|| "unknown axis".into()),
                    );
                }
            }
            U::Goal { slot, .. } => put(
                "goals",
                slot_person(slot)
                    .map(|p| {
                        pol.characters
                            .goals
                            .iter()
                            .filter(|g| g.holder == p)
                            .map(|g| g.goal.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_default(),
            ),
            U::Opinion { slot, .. } => {
                if let (Some(p), Some(cx)) = (slot_person(slot), ctx) {
                    put("opinion", num(characters::opinion(pol, p, cx.me) as f64));
                }
            }
            U::Trust { slot, .. } => {
                if let (Some(p), Some(cx)) = (slot_person(slot), ctx) {
                    put(
                        "trust",
                        num(characters::trust(pol, content, p, cx.me) as f64),
                    );
                }
            }
            U::Remembers { slot, .. } => {
                if let (Some(p), Some(cx)) = (slot_person(slot), ctx) {
                    let kinds: Vec<&str> = pol
                        .characters
                        .get(p)
                        .map(|c| {
                            c.memories
                                .iter()
                                .filter(|x| x.actor == cx.me)
                                .map(|x| x.kind.key())
                                .collect()
                        })
                        .unwrap_or_default();
                    put("memories of player", kinds.join(", "));
                }
            }
            U::Favour { slot, .. } => {
                if let (Some(p), Some(cx)) = (slot_person(slot), ctx) {
                    put(
                        "player owes",
                        characters::favours_owed(pol, cx.me, p).to_string(),
                    );
                    put(
                        "owed to player",
                        characters::favours_owed(pol, p, cx.me).to_string(),
                    );
                }
            }
            U::IsBroker { slot, .. } => {
                put(slot, slot_person(slot).map(|p| name(st, p)).unwrap_or_default());
                put(
                    "broker",
                    town.and_then(|t| t.broker)
                        .map(|p| name(st, p))
                        .unwrap_or_else(|| "none".into()),
                );
            }
            U::IsRival { slot, .. } => {
                put(slot, slot_person(slot).map(|p| name(st, p)).unwrap_or_default());
                put(
                    "rival",
                    town.and_then(|t| t.rival)
                        .map(|p| name(st, p))
                        .unwrap_or_else(|| "none".into()),
                );
            }
            U::Tells { subject, .. } => {
                if let Some(cx) = ctx {
                    put(
                        "would tell",
                        cx.subject(subject)
                            .map(|s| cx.told(&s).key().to_string())
                            .unwrap_or_else(|| "no such subject here".into()),
                    );
                    put("discretion cap", cx.cap.key().into());
                }
            }
            U::PlayerKnows { subject, .. } => {
                if let Some(cx) = ctx {
                    put(
                        "player knows",
                        cx.subject(subject)
                            .map(|s| crate::player::knowledge::knows(st, &s).key().to_string())
                            .unwrap_or_else(|| "no such subject here".into()),
                    );
                }
            }
            U::Any { .. } => {}
            U::Voice { .. } => {
                if let Some(cx) = ctx {
                    put(
                        "speaker's voice",
                        crate::talk::reply::voice(cx)
                            .map(|v| v.id.clone())
                            .unwrap_or_else(|| "none".into()),
                    );
                }
            }
            U::Concern { of, .. } => {
                // What the speaker's home neighbourhood feels most (the
                // reading `Ctx::top_concern` makes; display only).
                let top = match (of, ctx) {
                    (ConcernOf::Speaker, Some(cx)) => cx
                        .hood(mandate_content::conversation::HoodOf::SpeakerHome)
                        .and_then(|h| st.world.neighborhoods.get(h.index()).map(|n| (h, n)))
                        .and_then(|(h, n)| {
                            crate::concerns::felt_concerns(
                                &st.population,
                                content,
                                n.municipality.index(),
                                &[h.index()],
                            )
                            .first()
                            .filter(|(_, l)| *l > 0.0)
                            .map(|(i, _)| *i)
                        }),
                    (ConcernOf::Speaker, None) => None,
                    (ConcernOf::Town, _) => {
                        crate::concerns::top_concerns(&st.population, m as usize)
                            .first()
                            .filter(|(_, l)| *l > 0.0)
                            .map(|(i, _)| *i)
                    }
                };
                put(
                    "top concern",
                    top.and_then(|i| content.concerns.get(i))
                        .map(|c| c.id.clone())
                        .unwrap_or_else(|| "none".into()),
                );
            }
        }
        out
    }
}

// --- Adapter 3: overheard conditions ---------------------------------------------------

/// `HeardCondition` at a neighbourhood (where a line is heard).
pub struct HeardExplainer<'a> {
    ctx: crate::overheard::Ctx<'a>,
}

impl<'a> HeardExplainer<'a> {
    /// At a location in a town (`None`: the location has no neighbourhood).
    pub fn at(st: &'a State, content: &'a ContentBundle, loc: Location) -> Option<Self> {
        let h = crate::places::hood_of(st, loc)?;
        let n = st.world.neighborhoods.get(h.index())?;
        Some(HeardExplainer {
            ctx: crate::overheard::Ctx {
                st,
                content,
                m: n.municipality.0,
                hood: h.index(),
            },
        })
    }

    /// The arguments every condition that holds provides (the line's own
    /// arguments, as the draw gives them), with `town` and `hood`.
    pub fn args(&self, cs: &[HeardCondition]) -> (BTreeMap<String, String>, Option<u32>) {
        let mut args = BTreeMap::new();
        let mut story = None;
        for c in cs {
            self.ctx.holds(c, &mut args, &mut story);
        }
        let st = self.ctx.st;
        if let Some(m) = st.world.municipalities.get(self.ctx.m as usize) {
            args.insert("town".into(), m.name.clone());
        }
        if let Some(n) = st.world.neighborhoods.get(self.ctx.hood) {
            args.insert("hood".into(), n.name.clone());
        }
        (args, story)
    }
}

impl Explainer for HeardExplainer<'_> {
    type Cond = HeardCondition;

    fn holds(&self, c: &HeardCondition) -> bool {
        self.ctx
            .holds(c, &mut BTreeMap::new(), &mut Default::default())
    }

    fn any_of<'c>(&self, c: &'c HeardCondition) -> Option<&'c [HeardCondition]> {
        match c {
            HeardCondition::Any { of } => Some(of),
            _ => None,
        }
    }

    fn observe(&self, c: &HeardCondition) -> Vec<(String, String)> {
        let st = self.ctx.st;
        let m = self.ctx.m;
        let town = st.politics.characters.towns.get(m as usize);
        let mu = st.world.municipalities.get(m as usize);
        let mut out: Vec<(String, String)> = Vec::new();
        let mut put = |k: &str, x: String| out.push((k.into(), x));
        match c {
            HeardCondition::Water { feature } => {
                let waterfront = st.world.neighborhoods.get(self.ctx.hood).is_some_and(|n| {
                    n.kind == crate::world::NeighborhoodKind::Waterfront
                });
                put("river", mu.is_some_and(|x| x.on_river).to_string());
                put("coast", mu.is_some_and(|x| x.coastal).to_string());
                if matches!(feature, WaterFeature::Any) {
                    put("waterfront", waterfront.to_string());
                }
            }
            HeardCondition::Dry => {
                put("river", mu.is_some_and(|x| x.on_river).to_string());
                put("coast", mu.is_some_and(|x| x.coastal).to_string());
            }
            HeardCondition::Crisis { .. } => put(
                "crises",
                town.map(|t| {
                    t.crises
                        .iter()
                        .map(|x| format!("{} {}", x.kind, num(x.severity as f64)))
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default(),
            ),
            HeardCondition::PlantClosure { .. } => put(
                "last closure",
                st.events
                    .iter()
                    .rev()
                    .find(|e| {
                        matches!(e.kind, crate::events::EventKind::PlantClosure { municipality, .. } if municipality == m)
                    })
                    .map(|e| format!("{} days ago", e.date.days_until(st.calendar.date)))
                    .unwrap_or_else(|| "none on record".into()),
            ),
            HeardCondition::Disaster { .. } => put(
                "months since disaster",
                st.economy
                    .munis
                    .get(m as usize)
                    .map(|e| e.months_since_disaster.to_string())
                    .unwrap_or_default(),
            ),
            HeardCondition::Industry => put(
                "industry",
                town.and_then(|t| t.industry.clone())
                    .unwrap_or_else(|| "none".into()),
            ),
            HeardCondition::Broker => put(
                "broker",
                town.and_then(|t| t.broker)
                    .map(|p| name(st, p))
                    .unwrap_or_else(|| "none".into()),
            ),
            HeardCondition::Rival | HeardCondition::RivalRecord { .. } => {
                let rival = town.and_then(|t| t.rival);
                put(
                    "rival",
                    rival.map(|p| name(st, p)).unwrap_or_else(|| "none".into()),
                );
                if matches!(c, HeardCondition::RivalRecord { .. })
                    && let Some(p) = rival.and_then(|r| {
                        st.politics.politicians.iter().find(|p| p.character == r)
                    })
                {
                    put("record", num(p.record as f64));
                }
            }
            HeardCondition::PlayerAwareness { .. } | HeardCondition::PlayerApproval { .. } => {
                // Residents' mean awareness and approval of the player here
                // (as the evaluator weighs them; display only).
                if let Some(cs) = st.population.hoods.get(self.ctx.hood).map(|h| &h.cohorts) {
                    let n: f64 = cs.iter().map(|c| c.count as f64).sum::<f64>().max(1.0);
                    let mean = |f: &dyn Fn(&crate::population::Cohort) -> f64| {
                        cs.iter().map(|c| f(c) * c.count as f64).sum::<f64>() / n
                    };
                    put("awareness", num(mean(&|c| c.player_awareness as f64)));
                    put("approval", num(mean(&|c| c.player_approval as f64)));
                }
            }
            HeardCondition::Scandal { .. } => put(
                "running scandals in town",
                st.media
                    .scandals
                    .iter()
                    .filter(|x| x.running() && x.town == m)
                    .count()
                    .to_string(),
            ),
            HeardCondition::Headline => put(
                "latest story",
                st.media
                    .stories
                    .iter()
                    .rev()
                    .find(|s| s.town == m)
                    .map(|s| format!("{} days ago", s.date.days_until(st.calendar.date)))
                    .unwrap_or_else(|| "none".into()),
            ),
            HeardCondition::ElectionWithin { .. } => put(
                "days to election",
                st.politics
                    .elections
                    .next_day
                    .filter(|d| *d >= st.calendar.date)
                    .map(|d| st.calendar.date.days_until(d).to_string())
                    .unwrap_or_else(|| "none scheduled".into()),
            ),
            HeardCondition::Any { .. } => {}
        }
        out
    }
}

// --- Items ------------------------------------------------------------------------------

/// The kinds of content item the workbench explains. Asks, secrets and
/// curiosities join when their registries exist.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    Dilemma,
    Situational,
    Storyline,
    Overheard,
    Topic,
    Line,
    Voice,
    Scene,
}

impl ItemKind {
    pub const ALL: [ItemKind; 8] = [
        ItemKind::Dilemma,
        ItemKind::Situational,
        ItemKind::Storyline,
        ItemKind::Overheard,
        ItemKind::Topic,
        ItemKind::Line,
        ItemKind::Voice,
        ItemKind::Scene,
    ];

    pub fn key(self) -> &'static str {
        match self {
            ItemKind::Dilemma => "dilemma",
            ItemKind::Situational => "situational",
            ItemKind::Storyline => "storyline",
            ItemKind::Overheard => "overheard",
            ItemKind::Topic => "topic",
            ItemKind::Line => "line",
            ItemKind::Voice => "voice",
            ItemKind::Scene => "scene",
        }
    }

    pub fn parse(s: &str) -> Option<ItemKind> {
        ItemKind::ALL.into_iter().find(|k| k.key() == s)
    }
}

/// A text the item shows, with the arguments it would be given now.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Text {
    /// `title` | `situation` | `line` | `stage:<id>` | `beat:<speaker>` | `reason`.
    pub slot: String,
    pub key: String,
    pub args: BTreeMap<String, String>,
}

/// Where to look when an item depends on a speaker or a place.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Focus {
    pub speaker: Option<CharacterId>,
    pub place: Option<crate::world::PlaceId>,
}

/// Why an item could or could not fire now.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Explanation {
    pub kind: ItemKind,
    pub id: String,
    pub could_fire: bool,
    pub tree: Node,
    /// The cast the texts are rendered with: the one that satisfied the
    /// conditions, else (`forced`) every role filled by its first
    /// candidate regardless of conditions; empty if a role has nobody.
    pub cast: Vec<CastEntry>,
    pub forced: bool,
    /// Texts whose arguments the engine already knows (lines); dilemma,
    /// storyline and scene texts take the cast's arguments (the protocol's).
    pub texts: Vec<Text>,
    /// The speaker or place the explanation was made for, if any.
    pub speaker: Option<CharacterId>,
    pub place: Option<crate::world::PlaceId>,
    /// The subject a conversation item was explained with (a representative
    /// one of each kind its topic takes; the first for which it holds).
    pub about: Option<AboutRef>,
}

impl Explanation {
    fn new(kind: ItemKind, id: &str, tree: Node) -> Self {
        Explanation {
            kind,
            id: id.into(),
            could_fire: tree.ok,
            tree,
            cast: Vec::new(),
            forced: false,
            texts: Vec::new(),
            speaker: None,
            place: None,
            about: None,
        }
    }
}

/// Explain one item now. `None`: no such item, or no player to explain for.
pub fn explain(
    st: &State,
    content: &ContentBundle,
    kind: ItemKind,
    id: &str,
    focus: Focus,
) -> Option<Explanation> {
    st.player.as_ref()?;
    let r = Reads::of_state(st, content);
    let v = View::new(&r, &st.politics);
    match kind {
        ItemKind::Dilemma => dilemma(st, &v, content.dilemma(id)?),
        ItemKind::Situational => situational(st, &v, content, id),
        ItemKind::Storyline => storyline(st, &v, content, id),
        ItemKind::Overheard => overheard(st, &v, content, id, focus),
        ItemKind::Topic => topic(st, &v, content, id, focus),
        ItemKind::Line => line(st, &v, content, id, focus),
        ItemKind::Voice => voice(st, &v, content, id, focus),
        ItemKind::Scene => scene(st, &v, content, id),
    }
}

/// Every item's id, by kind, in registry order.
pub fn items(content: &ContentBundle) -> Vec<(ItemKind, String)> {
    let mut out = Vec::new();
    out.extend(
        content
            .dilemmas
            .iter()
            .map(|d| (ItemKind::Dilemma, d.id.clone())),
    );
    out.extend(
        content
            .policies
            .values()
            .filter(|p| p.unlock.is_some())
            .map(|p| (ItemKind::Situational, p.id.clone())),
    );
    out.extend(
        content
            .storylines
            .iter()
            .map(|d| (ItemKind::Storyline, d.id.clone())),
    );
    out.extend(
        content
            .overheard
            .iter()
            .map(|d| (ItemKind::Overheard, d.id.clone())),
    );
    out.extend(
        content
            .talk_topics
            .iter()
            .map(|d| (ItemKind::Topic, d.id.clone())),
    );
    out.extend(
        content
            .talk_lines
            .iter()
            .map(|d| (ItemKind::Line, d.id.clone())),
    );
    out.extend(
        content
            .talk_voices
            .iter()
            .map(|d| (ItemKind::Voice, d.id.clone())),
    );
    out.extend(
        content
            .scenes
            .iter()
            .map(|d| (ItemKind::Scene, d.id.clone())),
    );
    out
}

// --- Dilemmas -----------------------------------------------------------------------

/// Roles as cast (or the first role nobody can fill).
fn roles_node(v: &View<'_>, def_roles: &[mandate_content::RoleDef], cast: &[CastEntry]) -> Node {
    let mut kids = Vec::new();
    for r in def_roles {
        let (label, _) = describe(&r.cast);
        let node = match crate::narrative::actor_of(cast, &r.id) {
            Some(a) => Node::new("role", &r.id, label, true).with("cast", actor_label(v, a)),
            None => Node::new("role", &r.id, label, false).with("cast", "nobody fits"),
        };
        kids.push(node);
    }
    Node::group("all", "cast", kids)
}

/// Roles filled one by one regardless of conditions (the director's first
/// try), stopping at the first role nobody can fill.
fn forced_cast(
    v: &View<'_>,
    roles: &[mandate_content::RoleDef],
    preset: &[CastEntry],
    holder: Option<CharacterId>,
) -> Vec<CastEntry> {
    let mut out: Vec<CastEntry> = Vec::new();
    for r in roles {
        if let Some(p) = preset.iter().find(|p| p.role == r.id) {
            out.push(p.clone());
            continue;
        }
        let Some(a) = cast::candidates(v, &r.cast, &out, holder)
            .into_iter()
            .find(|a| !out.iter().any(|c| c.actor == *a))
        else {
            break;
        };
        out.push(CastEntry {
            role: r.id.clone(),
            actor: a,
        });
    }
    for p in preset {
        if !out.iter().any(|c| c.role == p.role) {
            out.push(p.clone());
        }
    }
    out
}

/// Cast and triggers as the director evaluates them (`director::evaluate`),
/// with each trigger explained against the cast it found, or, when it
/// found none, against the forced cast. Returns the node, the cast, whether
/// it was forced, and the relevance when castable.
fn cast_and_triggers(
    v: &View<'_>,
    def: &DilemmaDef,
    preset: &[CastEntry],
    holder: Option<CharacterId>,
) -> (Node, Vec<CastEntry>, bool, Option<f64>) {
    let n = v.r.narrative();
    let found = crate::narrative::director::evaluate(v, n, def, preset, holder, &[]);
    let (cast, forced, relevance) = match &found {
        Some(c) => (c.cast.clone(), false, Some(c.relevance)),
        None => (forced_cast(v, &def.roles, preset, holder), true, None),
    };
    let roles = roles_node(v, &def.roles, &cast);
    let e = NarrativeExplainer { v, cast: &cast };
    let mut triggers = explain_all(&e, "triggers", &def.triggers);
    if forced && roles.ok && triggers.ok {
        // Every trigger holds for the first candidates but the search found
        // no cast: cannot happen (its first try is this cast); kept honest.
        triggers.ok = false;
    }
    let mut kids = vec![roles, triggers];
    if forced {
        kids.push(Node::new(
            "info",
            "search",
            "no cast satisfies the triggers (up to 6 candidates per role tried); shown with the first candidates",
            false,
        ));
    }
    let mut node = Node::group("all", "cast and triggers", kids);
    node.ok = found.is_some();
    (node, cast, forced, relevance)
}

fn dilemma(st: &State, v: &View<'_>, def: &DilemmaDef) -> Option<Explanation> {
    let content = v.content();
    let n = &st.narrative;
    let date = v.date();
    let d = &content.director;
    let mut gates = Vec::new();
    let active = st.calendar.phase != crate::state::GamePhase::BurnIn
        && st.player.as_ref().is_some_and(|p| p.status.active());
    gates.push(Node::new(
        "gate",
        "in play",
        "the director runs once play begins",
        active,
    ));
    let mut preset: Vec<CastEntry> = Vec::new();
    let mut holders: Vec<Option<CharacterId>> = vec![None];
    // The route by which the dilemma reaches the player.
    if def.category == DilemmaCategory::Request {
        let goals: Vec<&mandate_content::GoalDef> = content
            .goals
            .iter()
            .filter(|g| g.request.as_deref() == Some(def.id.as_str()))
            .collect();
        let hs: Vec<CharacterId> = st
            .politics
            .characters
            .goals
            .iter()
            .filter(|g| goals.iter().any(|x| x.id == g.goal))
            .map(|g| g.holder)
            .collect();
        let ids: Vec<&str> = goals.iter().map(|g| g.id.as_str()).collect();
        gates.push(
            Node::new(
                "gate",
                "route",
                format!("a request, raised by goals: {}", ids.join(", ")),
                !hs.is_empty(),
            )
            .with(
                "holders",
                hs.iter()
                    .map(|h| name(st, *h))
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
        );
        holders = hs.into_iter().map(Some).collect();
    } else if def.storyline_only {
        let stage = n.active_runs().find_map(|run| {
            let sd = content.storyline(&run.template)?;
            let s = sd.stages.get(run.stage as usize)?;
            (s.decision.as_deref() == Some(def.id.as_str())).then(|| (run, s.id.clone()))
        });
        let lines: Vec<&str> = content
            .storylines
            .iter()
            .filter(|s| {
                s.stages
                    .iter()
                    .any(|x| x.decision.as_deref() == Some(def.id.as_str()))
            })
            .map(|s| s.id.as_str())
            .collect();
        let mut g = Node::new(
            "gate",
            "route",
            format!("a storyline stage of: {}", lines.join(", ")),
            stage.is_some(),
        );
        if let Some((run, sid)) = &stage {
            g = g.with("at stage", format!("{} / {sid}", run.template));
            preset = run.cast.clone();
        }
        gates.push(g);
    } else if let Some(by) = def.raised_by {
        let me = st.player.as_ref().map(|p| p.politician);
        let open = match by {
            RaisedBy::Scandal => st
                .media
                .scandals
                .iter()
                .any(|s| Some(s.subject) == me && s.running() && s.response.is_none()),
            RaisedBy::Investigation => st.justice.cases.iter().any(|c| {
                Some(c.subject) == me
                    && c.stage == crate::justice::Stage::Investigation
                    && c.response.is_none()
            }),
        };
        gates.push(Node::new(
            "gate",
            "route",
            format!("raised by a {by:?} about the player (never budgeted)").to_lowercase(),
            open,
        ));
    } else {
        let staged = n.active_runs().any(|r| {
            content.storyline(&r.template).is_some_and(|s| {
                s.stages
                    .iter()
                    .any(|x| x.decision.as_deref() == Some(def.id.as_str()))
            })
        });
        gates.push(Node::new(
            "gate",
            "route",
            "the director's spotlight (not a stage of a running storyline)",
            !staged,
        ));
    }
    if def.raised_by.is_none() {
        let until = n.cooldowns.get(&def.id).copied().filter(|x| *x > date);
        let mut g = Node::new(
            "gate",
            "cooldown",
            format!("{} days after it last came up", def.cooldown_days),
            until.is_none(),
        );
        if let Some(u) = until {
            g = g.with("days left", date.days_until(u));
        }
        gates.push(g);
    }
    let pending = n.pending().any(|p| p.template == def.id);
    gates.push(Node::new(
        "gate",
        "not pending",
        "not already waiting for an answer",
        !pending,
    ));
    // Casting and triggers, for each goal holder in turn (requests).
    let mut best: Option<(Node, Vec<CastEntry>, bool, Option<f64>)> = None;
    for h in holders.iter().copied().chain(
        // A request with no holder still shows its cast and triggers.
        holders.is_empty().then_some(None),
    ) {
        let r = cast_and_triggers(v, def, &preset, h);
        let ok = r.0.ok;
        if best.is_none() || ok {
            best = Some(r);
        }
        if ok {
            break;
        }
    }
    let (ct, cast, forced, relevance) = best?;
    let mut pacing = Vec::new();
    if let Some(rel) = relevance {
        pacing.push(
            Node::new(
                "info",
                "relevance",
                format!(
                    "the spotlight needs {} (a chapter's climax does not)",
                    num(d.min_relevance)
                ),
                rel >= d.min_relevance,
            )
            .with("relevance", num(rel)),
        );
    }
    let pend = n.pending().count();
    pacing.push(
        Node::new(
            "info",
            "room",
            format!("at most {} decisions waiting", d.max_pending),
            pend < d.max_pending as usize,
        )
        .with("waiting", pend),
    );
    if let Some(ch) = n.chapter.as_ref() {
        pacing.push(
            Node::new(
                "info",
                "chapter",
                format!("{:?}", ch.kind).to_lowercase(),
                true,
            )
            .with("progress", num(ch.progress(date)))
            .with("significant so far", ch.significant)
            .with("minor so far", ch.minor),
        );
    }
    let mut kids = vec![Node::group("all", "gates", gates), ct];
    kids.push(Node {
        kind: "info".into(),
        label: "pacing".into(),
        detail: "the director's budgets and spacing decide when, not whether".into(),
        ok: true,
        values: Vec::new(),
        children: pacing,
    });
    let tree = Node::group("all", &def.id, kids);
    let mut x = Explanation::new(ItemKind::Dilemma, &def.id, tree);
    x.cast = cast;
    x.forced = forced;
    Some(x)
}

// --- Situational policies ----------------------------------------------------------------

fn situational(st: &State, v: &View<'_>, content: &ContentBundle, id: &str) -> Option<Explanation> {
    let t = content.policies.get(id)?;
    let u = t.unlock.as_ref()?;
    let m = home(v)?;
    let e = SituationExplainer::Town { st, content, m };
    let mut any = Node::group(
        "any",
        "unlock (any)",
        u.any.iter().map(|c| explain_one(&e, c)).collect(),
    );
    // The policy domain's verdict.
    any.ok = crate::policy::unlocked(st, content, t, m);
    let tree = Node::group("all", id, vec![any]);
    let mut x = Explanation::new(ItemKind::Situational, id, tree);
    x.texts.push(Text {
        slot: "title".into(),
        key: t.name_key.clone(),
        args: BTreeMap::new(),
    });
    x.texts.push(Text {
        slot: "reason".into(),
        key: u.reason_key.clone(),
        args: BTreeMap::new(),
    });
    Some(x)
}

// --- Storylines -------------------------------------------------------------------------

fn storyline(st: &State, v: &View<'_>, content: &ContentBundle, id: &str) -> Option<Explanation> {
    let def = content.storyline(id)?;
    let n = &st.narrative;
    let date = v.date();
    let run = n.active_runs().find(|r| r.template == def.id);
    let mut kids = Vec::new();
    let (cast, forced);
    match run {
        Some(run) => {
            kids.push(
                Node::new("gate", "running", "a run of this storyline is active", true)
                    .with("since", run.started.days_until(date).to_string() + " days")
                    .with(
                        "stages done",
                        run.stages
                            .iter()
                            .map(|(s, _)| s.as_str())
                            .collect::<Vec<_>>()
                            .join(", "),
                    ),
            );
            cast = run.cast.clone();
            forced = false;
            let still: Vec<Node> = def
                .roles
                .iter()
                .map(|role| {
                    let a = crate::narrative::actor_of(&run.cast, &role.id);
                    let ok = a.is_some_and(|a| cast::still_cast(v, role, a, &run.cast, None));
                    Node::new("role", &role.id, describe(&role.cast).0, ok).with(
                        "cast",
                        a.map(|a| actor_label(v, a))
                            .unwrap_or_else(|| "nobody".into()),
                    )
                })
                .collect();
            kids.push(Node::group(
                "all",
                "cast still fits (else the arc fades)",
                still,
            ));
            match def.stages.get(run.stage as usize) {
                Some(stage) => {
                    let ready = run.since.add_days(stage.wait_days);
                    let mut g = Node::new(
                        "gate",
                        "waited",
                        format!("{} days after the previous stage", stage.wait_days),
                        date >= ready,
                    );
                    if date < ready {
                        g = g.with("days left", date.days_until(ready));
                    }
                    let mut sk = vec![g];
                    if let Some(t) = stage.timeout_days {
                        sk.push(
                            Node::new(
                                "gate",
                                "not timed out",
                                format!("fades {t} days after it is ready"),
                                date <= ready.add_days(t),
                            )
                            .with("waiting", run.decision.is_some() || run.entered),
                        );
                    }
                    let e = NarrativeExplainer { v, cast: &run.cast };
                    sk.push(explain_all(&e, "advance", &stage.advance));
                    if let Some(dk) = &stage.decision
                        && let Some(ddef) = content.dilemma(dk)
                    {
                        let (ct, ..) = cast_and_triggers(v, ddef, &run.cast, None);
                        let mut ct = ct;
                        ct.label = format!("decision {dk}");
                        sk.push(ct);
                    }
                    let mut node = Node::group("stage", &format!("stage {}", stage.id), sk);
                    if run.decision.is_some() {
                        node.children.push(Node::new(
                            "info",
                            "decision open",
                            "this stage's decision is waiting for an answer",
                            true,
                        ));
                    }
                    kids.push(node);
                }
                None => kids.push(Node::new("gate", "stage", "past the last stage", false)),
            }
        }
        None => {
            let until = n.cooldowns.get(&def.id).copied().filter(|x| *x > date);
            let mut g = Node::new(
                "gate",
                "cooldown",
                format!("{} days after it last ended", def.cooldown_days),
                until.is_none(),
            );
            if let Some(u) = until {
                g = g.with("days left", date.days_until(u));
            }
            kids.push(g);
            let cap = content.director.storylines_active as usize;
            let running = n.active_runs().count();
            kids.push(
                Node::new(
                    "gate",
                    "room",
                    format!("at most {cap} storylines at once"),
                    running < cap,
                )
                .with("running", running),
            );
            let follow = n.follow_ups.iter().any(|f| f.0 == def.id);
            if follow {
                kids.push(Node::new(
                    "info",
                    "follow-up",
                    "queued as a follow-up (starts with the earlier arc's cast)",
                    true,
                ));
            }
            match cast::cast_roles(v, &def.roles, &[], None) {
                Some(c) => {
                    kids.push(roles_node(v, &def.roles, &c));
                    let e = NarrativeExplainer { v, cast: &c };
                    kids.push(explain_all(&e, "start", &def.start));
                    cast = c;
                    forced = false;
                }
                None => {
                    let c = forced_cast(v, &def.roles, &[], None);
                    kids.push(roles_node(v, &def.roles, &c));
                    let e = NarrativeExplainer { v, cast: &c };
                    let mut s = explain_all(&e, "start", &def.start);
                    s.ok = false;
                    kids.push(s);
                    cast = c;
                    forced = true;
                }
            }
        }
    }
    let tree = Node::group("all", &def.id, kids);
    let mut x = Explanation::new(ItemKind::Storyline, &def.id, tree);
    x.cast = cast;
    x.forced = forced;
    Some(x)
}

// --- Overheard lines ---------------------------------------------------------------------

/// Places in the player's town where people talk (the `overheard` service).
pub fn talk_places(st: &State, content: &ContentBundle) -> Vec<crate::world::PlaceId> {
    let r = Reads::of_state(st, content);
    let v = View::new(&r, &st.politics);
    let Some(m) = home(&v) else {
        return Vec::new();
    };
    st.world
        .places
        .iter()
        .filter(|p| p.municipality.0 == m)
        .filter(|p| {
            crate::places::services_at(st, content, Location::Place { place: p.id })
                .contains(&"overheard")
        })
        .map(|p| p.id)
        .collect()
}

fn overheard(
    st: &State,
    _v: &View<'_>,
    content: &ContentBundle,
    id: &str,
    focus: Focus,
) -> Option<Explanation> {
    let def = content.overheard.iter().find(|d| d.id == id)?;
    let places = talk_places(st, content);
    let mut kids = Vec::new();
    let mut first_ok: Option<crate::world::PlaceId> = None;
    for p in &places {
        let loc = Location::Place { place: *p };
        let Some(e) = HeardExplainer::at(st, content, loc) else {
            continue;
        };
        // The overheard system's verdict: the line is among those that hold.
        let holds = crate::overheard::available(st, content, loc)
            .iter()
            .any(|l| l.id == def.id);
        let drawn = crate::overheard::lines(st, content, loc)
            .iter()
            .any(|l| l.id == def.id);
        let mut pk = Vec::new();
        if let Some(c) = &def.concern {
            let felt: Vec<String> = crate::concerns::felt_concerns(
                &st.population,
                content,
                e.ctx.m as usize,
                &[e.ctx.hood],
            )
            .into_iter()
            .filter(|(_, w)| *w > 0.0)
            .take(3)
            .filter_map(|(i, _)| content.concerns.get(i).map(|x| x.id.clone()))
            .collect();
            pk.push(
                Node::new(
                    "gate",
                    "concern",
                    format!("{c} among the neighbourhood's top three"),
                    felt.contains(c),
                )
                .with("top three", felt.join(", ")),
            );
        }
        pk.push(explain_all(&e, "when", &def.when));
        pk.push(Node::new(
            "info",
            "drawn this week",
            "the week's draw (three lines per place)",
            drawn,
        ));
        let label = st
            .world
            .places
            .get(p.index())
            .map(|x| {
                if x.name.is_empty() {
                    format!("{} #{}", x.kind.key(), p.0)
                } else {
                    x.name.clone()
                }
            })
            .unwrap_or_default();
        let mut node = Node::group("place", &label, pk);
        node.ok = holds;
        if holds && first_ok.is_none() {
            first_ok = Some(*p);
        }
        kids.push(node);
    }
    let chosen = focus
        .place
        .filter(|p| places.contains(p))
        .or(first_ok)
        .or(places.first().copied());
    let mut any = Node::group("any", "a place in town where people talk", kids);
    any.ok = any.children.iter().any(|c| c.ok);
    if places.is_empty() {
        any.detail = "no place in town offers overheard talk".into();
    }
    let tree = Node::group("all", id, vec![any]);
    let mut x = Explanation::new(ItemKind::Overheard, id, tree);
    if let Some(p) = chosen
        && let Some(e) = HeardExplainer::at(st, content, Location::Place { place: p })
    {
        let (args, _) = e.args(&def.when);
        x.texts.push(Text {
            slot: "line".into(),
            key: def.key.clone(),
            args,
        });
        x.place = Some(p);
    }
    Some(x)
}

// --- Conversations ------------------------------------------------------------------------

/// Everyone in the player's town the player could talk to: officeholders,
/// then Named people (not the player).
pub fn speakers(st: &State, content: &ContentBundle) -> Vec<CharacterId> {
    let r = Reads::of_state(st, content);
    let v = View::new(&r, &st.politics);
    let Some(m) = home(&v) else {
        return Vec::new();
    };
    let me = v.me();
    let mut out: Vec<CharacterId> = v
        .officeholders()
        .into_iter()
        .filter_map(|p| st.politics.get(p).map(|x| x.character))
        .collect();
    for c in &st.politics.characters.people {
        if c.named.is_some()
            && Some(c.id) != me
            && !out.contains(&c.id)
            && characters::municipality_of(&st.politics, &st.world, c.id) == Some(m)
        {
            out.push(c.id);
        }
    }
    out
}

/// A representative subject of one kind: the rival, a person (the broker,
/// else the rival), a bill before the council, a template, the player's
/// home neighbourhood, a place in town.
fn sample_kind(st: &State, content: &ContentBundle, m: u32, k: AboutKind) -> Option<AboutRef> {
    let town = st.politics.characters.towns.get(m as usize);
    match k {
        AboutKind::Rival => town.and_then(|t| t.rival).map(|_| AboutRef::Rival),
        AboutKind::Person => town
            .and_then(|t| t.broker.or(t.rival))
            .map(|person| AboutRef::Person { person }),
        AboutKind::Bill => st
            .policy
            .in_municipality(m)
            .find(|p| p.status == crate::policy::PolicyStatus::UnderConsideration)
            .map(|p| AboutRef::Bill { policy: p.id }),
        AboutKind::Template => content
            .policies
            .values()
            .next()
            .map(|t| AboutRef::Template {
                template: t.id.clone(),
            }),
        AboutKind::Hood => crate::places::home_neighborhood(st).map(|hood| AboutRef::Hood { hood }),
        AboutKind::Place => st
            .world
            .places
            .iter()
            .find(|p| p.municipality.0 == m)
            .map(|p| AboutRef::Place { place: p.id }),
        AboutKind::Proposal => None,
    }
}

/// The subjects to try a topic with: none (when the topic does not need
/// one), then a representative subject of each kind it takes that exists in
/// town.
pub fn sample_abouts(
    st: &State,
    content: &ContentBundle,
    kinds: &[AboutKind],
    required: bool,
) -> Vec<Option<AboutRef>> {
    let r = Reads::of_state(st, content);
    let v = View::new(&r, &st.politics);
    let mut out: Vec<Option<AboutRef>> = Vec::new();
    if !required || kinds.is_empty() {
        out.push(None);
    }
    if let Some(m) = home(&v) {
        for k in kinds {
            if let Some(a) = sample_kind(st, content, m, *k)
                && !out.contains(&Some(a.clone()))
            {
                out.push(Some(a));
            }
        }
    }
    out
}

/// A conversation's context with a speaker (no small talk, no favour
/// called in: the discretion of a plain conversation).
pub fn talk_ctx<'a>(
    st: &'a State,
    content: &'a ContentBundle,
    speaker: CharacterId,
    about: Option<&'a AboutRef>,
) -> Option<TalkCtx<'a>> {
    let pl = st.player.as_ref()?;
    let me = characters::player_character(st)?;
    let r = Reads::of_state(st, content);
    let v = View::new(&r, &st.politics);
    let town = MunicipalityId(home(&v)?);
    let cap = characters::knows::discretion_cap(st, content, speaker, me, Default::default());
    Some(TalkCtx {
        st,
        content,
        town,
        me,
        speaker,
        about,
        here: pl.location,
        cap,
    })
}

fn speaker_label(st: &State, content: &ContentBundle, s: CharacterId) -> String {
    format!(
        "{} ({})",
        name(st, s),
        characters::knows::role_key(st, content, s).unwrap_or_else(|| "out of office".into())
    )
}

/// A topic for one speaker: offered to their role, and its `when`.
fn topic_node(ctx: &TalkCtx<'_>, t: &mandate_content::conversation::TopicDef) -> Node {
    let st = ctx.st;
    let role = characters::knows::role_key(st, ctx.content, ctx.speaker);
    let o = &t.offered;
    let mut kids = Vec::new();
    kids.push(
        Node::new(
            "gate",
            "offered to",
            if o.roles.is_empty() {
                "anyone".to_string()
            } else {
                o.roles.join(", ")
            },
            o.roles.is_empty() || role.as_ref().is_some_and(|r| o.roles.contains(r)),
        )
        .with("role", role.clone().unwrap_or_else(|| "none".into())),
    );
    if !o.places.is_empty() {
        kids.push(Node::new(
            "info",
            "places",
            format!("raised only at: {}", o.places.join(", ")),
            true,
        ));
    }
    let e = SituationExplainer::Talk(ctx);
    kids.push(explain_all(&e, "when", &o.when));
    Node::group(
        "speaker",
        &speaker_label(st, ctx.content, ctx.speaker),
        kids,
    )
}

/// Try each speaker in town, each with every subject the topic takes; the
/// explanation is for the focused speaker, else the first for whom it
/// holds, else the first (with the first subject that holds for them).
fn per_speaker(
    st: &State,
    content: &ContentBundle,
    focus: Focus,
    about_kinds: &[AboutKind],
    about_required: bool,
    f: &dyn Fn(&TalkCtx<'_>) -> (Node, Vec<Text>),
) -> Talked {
    let all = speakers(st, content);
    let mut abouts = sample_abouts(st, content, about_kinds, about_required);
    // A topic that needs a subject nobody in town offers cannot be raised;
    // its conditions are still shown, without one.
    let no_subject = abouts.is_empty();
    if no_subject {
        abouts.push(None);
    }
    type Pick = (Node, CharacterId, Vec<Text>, Option<AboutRef>);
    let mut chosen: Option<Pick> = None;
    let mut fallback: Option<Pick> = None;
    let mut holds = 0;
    for s in &all {
        let mut mine: Option<Pick> = None;
        for about in &abouts {
            let Some(ctx) = talk_ctx(st, content, *s, about.as_ref()) else {
                continue;
            };
            let (node, texts) = f(&ctx);
            if mine.as_ref().is_none_or(|m| !m.0.ok && node.ok) {
                mine = Some((node, *s, texts, about.clone()));
            }
        }
        let Some(mine) = mine else { continue };
        if mine.0.ok {
            holds += 1;
        }
        if focus.speaker == Some(*s) {
            chosen = Some(mine);
        } else if fallback.as_ref().is_none_or(|fb| !fb.0.ok && mine.0.ok) {
            fallback = Some(mine);
        }
    }
    let pick = chosen.or(fallback);
    let mut kids = Vec::new();
    let (node, speaker, texts, about) = match pick {
        Some((n, s, t, a)) => (n, Some(s), t, a),
        None => (
            Node::new("speaker", "nobody", "no one in town to talk to", false),
            None,
            Vec::new(),
            None,
        ),
    };
    if let Some(a) = &about {
        kids.push(Node::new("info", "about", describe(a).0, true).with("subject", describe(a).1));
    }
    if no_subject {
        let kinds: Vec<&str> = about_kinds.iter().map(|k| k.key()).collect();
        kids.push(Node::new(
            "gate",
            "subject",
            format!(
                "needs a {} to be about; none in town now",
                kinds.join(" or ")
            ),
            false,
        ));
        holds = 0;
    }
    kids.push(node);
    let mut tree = Node::group("all", "speaker", kids);
    tree.label = format!("holds for {holds} of {} speakers in town", all.len());
    Talked {
        tree,
        speaker,
        texts,
        about,
        holds,
    }
}

/// What `per_speaker` found.
struct Talked {
    tree: Node,
    speaker: Option<CharacterId>,
    texts: Vec<Text>,
    about: Option<AboutRef>,
    holds: usize,
}

fn topic(
    st: &State,
    _v: &View<'_>,
    content: &ContentBundle,
    id: &str,
    focus: Focus,
) -> Option<Explanation> {
    let t = content.talk_topic(id)?;
    let r = per_speaker(st, content, focus, &t.about, t.about_required, &|ctx| {
        (topic_node(ctx, t), Vec::new())
    });
    let mut tree = Node::group("all", id, vec![r.tree]);
    tree.ok = r.holds > 0;
    let mut x = Explanation::new(ItemKind::Topic, id, tree);
    x.speaker = r.speaker;
    x.about = r.about;
    x.texts.push(Text {
        slot: "title".into(),
        key: t.name_key.clone(),
        args: BTreeMap::new(),
    });
    Some(x)
}

fn line(
    st: &State,
    _v: &View<'_>,
    content: &ContentBundle,
    id: &str,
    focus: Focus,
) -> Option<Explanation> {
    let l = content.talk_lines.iter().find(|x| x.id == id)?;
    let topic = l.topic.as_deref().and_then(|k| content.talk_topic(k));
    let about_kinds: Vec<AboutKind> = topic.map(|t| t.about.clone()).unwrap_or_default();
    let f = |ctx: &TalkCtx<'_>| -> (Node, Vec<Text>) {
        let mut kids = Vec::new();
        // The part's gate: a substance line answers its topic (offered to
        // the speaker); a closer is said in its voice.
        match l.part {
            TalkPart::Substance | TalkPart::Opener => {
                if let Some(t) = topic {
                    let mut n = topic_node(ctx, t);
                    n.kind = "gate".into();
                    n.label = format!("topic {} offered", t.id);
                    kids.push(n);
                }
            }
            TalkPart::Closer => {
                let voice = crate::talk::reply::voice(ctx);
                kids.push(
                    Node::new(
                        "gate",
                        "voice",
                        format!("said in {}", l.voice.clone().unwrap_or_default()),
                        voice.is_some_and(|v| Some(&v.id) == l.voice.as_ref()),
                    )
                    .with(
                        "speaker's voice",
                        voice.map(|v| v.id.clone()).unwrap_or_else(|| "none".into()),
                    ),
                );
            }
            TalkPart::Quote => {}
        }
        if l.fallback {
            kids.push(Node::new(
                "info",
                "fallback",
                "said only when no other substance line of the topic holds",
                true,
            ));
        }
        let e = SituationExplainer::Talk(ctx);
        kids.push(explain_all(&e, "when", &l.when));
        // The reply's own reading: every condition holds (with its arguments).
        let mut args = crate::talk::context_args(ctx);
        let holds = ctx.all_hold(&l.when, &mut args);
        let mut node = Node::group(
            "speaker",
            &speaker_label(ctx.st, ctx.content, ctx.speaker),
            kids,
        );
        node.ok = node.ok && holds;
        let texts = vec![Text {
            slot: "line".into(),
            key: l.key.clone(),
            args: args.into_iter().collect(),
        }];
        (node, texts)
    };
    let required = topic.is_some_and(|t| t.about_required);
    let r = per_speaker(st, content, focus, &about_kinds, required, &f);
    let mut tree = Node::group("all", id, vec![r.tree]);
    tree.ok = r.holds > 0;
    let mut x = Explanation::new(ItemKind::Line, id, tree);
    x.speaker = r.speaker;
    x.texts = r.texts;
    x.about = r.about;
    Some(x)
}

fn voice(
    st: &State,
    _v: &View<'_>,
    content: &ContentBundle,
    id: &str,
    focus: Focus,
) -> Option<Explanation> {
    let vd = content.talk_voice(id)?;
    let f = |ctx: &TalkCtx<'_>| -> (Node, Vec<Text>) {
        let e = SituationExplainer::Talk(ctx);
        let when = explain_all(&e, "when", &vd.when);
        let actual = crate::talk::reply::voice(ctx);
        let is = actual.is_some_and(|v| v.id == vd.id);
        let mut node = Node::group(
            "speaker",
            &speaker_label(ctx.st, ctx.content, ctx.speaker),
            vec![
                when,
                Node::new(
                    "gate",
                    "highest priority",
                    format!("priority {}: no higher voice holds", vd.priority),
                    is,
                )
                .with(
                    "speaker's voice",
                    actual
                        .map(|v| v.id.clone())
                        .unwrap_or_else(|| "none".into()),
                ),
            ],
        );
        node.ok = is;
        (node, Vec::new())
    };
    let r = per_speaker(st, content, focus, &[], false, &f);
    let mut tree = Node::group("all", id, vec![r.tree]);
    tree.ok = r.holds > 0;
    let mut x = Explanation::new(ItemKind::Voice, id, tree);
    x.speaker = r.speaker;
    Some(x)
}

// --- Scenes -----------------------------------------------------------------------------

fn scene(st: &State, v: &View<'_>, content: &ContentBundle, id: &str) -> Option<Explanation> {
    let sc = content.scene(id)?;
    let mut kids = Vec::new();
    let mut best: Option<(bool, Vec<CastEntry>, bool)> = None;
    for d in content
        .dilemmas
        .iter()
        .filter(|d| d.scene.as_deref() == Some(id))
    {
        let Some(dx) = dilemma(st, v, d) else {
            continue;
        };
        let place = crate::narrative::director::scene_place(v, &sc.place, &dx.cast);
        let mut node = Node::group(
            "all",
            &format!("presented by {}", d.id),
            vec![
                Node::new("gate", "dilemma could fire", d.id.clone(), dx.could_fire),
                Node::new("place", "place", describe(&sc.place).1, place.is_some()).with(
                    "resolves to",
                    place
                        .map(|p| describe(&p).1)
                        .unwrap_or_else(|| "nowhere in town".into()),
                ),
            ],
        );
        node.ok = dx.could_fire && place.is_some();
        if best.as_ref().is_none_or(|b| !b.0 && node.ok) {
            best = Some((node.ok, dx.cast.clone(), dx.forced));
        }
        kids.push(node);
    }
    if kids.is_empty() {
        kids.push(Node::new(
            "gate",
            "presented",
            "no dilemma presents this scene",
            false,
        ));
    }
    let mut any = Node::group("any", "a dilemma that presents it", kids);
    any.ok = any.children.iter().any(|c| c.ok);
    let tree = Node::group("all", id, vec![any]);
    let mut x = Explanation::new(ItemKind::Scene, id, tree);
    if let Some((_, cast, forced)) = best {
        x.cast = cast;
        x.forced = forced;
    }
    Some(x)
}

// --- Speakers and places as the workbench lists them -----------------------------------

/// A speaker's name and role key (`councillor`, an occupation id).
pub fn speaker_info(
    st: &State,
    content: &ContentBundle,
    s: CharacterId,
) -> (String, Option<String>) {
    (name(st, s), characters::knows::role_key(st, content, s))
}

/// A neighbourhood's index for a place (where an overheard line is heard).
pub fn place_hood(st: &State, p: crate::world::PlaceId) -> Option<NeighborhoodId> {
    st.world.places.get(p.index()).map(|x| x.neighborhood)
}
