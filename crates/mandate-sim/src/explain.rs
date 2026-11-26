//! The content workbench's explanations (owner, 2026-11-19): for a content
//! item, whether it could fire right now and why, as a tree of the
//! conditions it needs, each checked by the **real evaluator**, with the
//! values that evaluator reads. Read-only: nothing here writes state or
//! draws from a stream, so it is safe in views, the lab and tests.
//!
//! **One condition language, one adapter** (DEC-098 §5, task U1): every
//! condition is explained by [`CondExplainer`], whose verdict is the one
//! evaluator's (`crate::condition::holds`) in the context the item's use
//! binds (a cast, a town's situation, a place, a conversation); the adapter
//! only adds what was read, for display, one arm per variant.
//!
//! **What "could fire now" means** per kind (the gates each kind's system
//! applies before its conditions, then the conditions):
//! - dilemma: its route (spotlight, request, an arc's beat, raised),
//!   cooldown, not already pending, then casting and triggers exactly as the
//!   director's `evaluate` does. The director's pacing (budgets, spacing,
//!   relevance) is shown but does not decide the verdict;
//! - situational policy: its unlock (any condition) in the player's town;
//! - overheard line: it holds at some place in town where people talk;
//! - topic, line, voice: it holds for some speaker in town (place and
//!   cost aside);
//! - scene: a dilemma that presents it could fire, and its place resolves.

use crate::characters::{self, CharacterId};
use crate::condition;
use crate::narrative::cast::{self, View};
use crate::narrative::{Actor, CastEntry};
use crate::places::Location;
use crate::scheduler::Reads;
use crate::state::State;
use crate::talk::AboutRef;
use crate::talk::context::{Args, Ctx as TalkCtx};
use crate::world::{MunicipalityId, NeighborhoodId};
use mandate_content::condition::{CondTier, Worry};
use mandate_content::conversation::{AboutKind, ConcernOf, TalkPart};
use mandate_content::{
    Condition, ContentBundle, DilemmaCategory, DilemmaDef, RaisedBy, WaterFeature,
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

/// The adapter a condition language implements (one since U1:
/// [`CondExplainer`]). `holds` must be the language's own evaluator.
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

// --- The one adapter ------------------------------------------------------------------

/// The one condition language through its one evaluator
/// (`crate::condition::holds`), in the context its use binds: a cast
/// (dilemmas, goals, scenes), a town's situation (a policy's
/// unlock), a place (an overheard line), or a conversation.
pub struct CondExplainer<'a> {
    pub cx: &'a condition::Ctx<'a>,
}

impl Explainer for CondExplainer<'_> {
    type Cond = Condition;

    fn holds(&self, c: &Condition) -> bool {
        condition::holds(self.cx, c, &mut Args::new())
    }

    fn any_of<'c>(&self, c: &'c Condition) -> Option<&'c [Condition]> {
        match c {
            Condition::Any { of } => Some(of),
            _ => None,
        }
    }

    /// What the evaluator reads for each variant (one table; display only,
    /// the verdict is the evaluator's).
    fn observe(&self, c: &Condition) -> Vec<(String, String)> {
        observe(self.cx, c)
    }
}

/// What the evaluator reads for a condition, as (name, value): one arm per
/// variant.
fn observe(cx: &condition::Ctx<'_>, c: &Condition) -> Vec<(String, String)> {
    use Condition as C;
    let v = cx.v;
    let pol = v.pol;
    let content = v.content();
    let world = v.r.world;
    let today = v.date();
    let me = cx.me();
    let m = cx.town_id().map(|t| t.0);
    let town = cx.town_character();
    let mu = m.and_then(|m| world.municipalities.get(m as usize));
    let pname = |p: CharacterId| {
        characters::profile(pol, world, p)
            .map(|x| x.full_name())
            .unwrap_or_else(|| format!("#{}", p.0))
    };
    let mut out: Vec<(String, String)> = Vec::new();
    let mut put = |k: &str, x: String| out.push((k.into(), x));
    if c.tier() == CondTier::Local && m.is_none() && !matches!(c, C::Any { .. }) {
        put(
            "jurisdiction",
            "not a town (local-only: never holds)".into(),
        );
        return out;
    }
    // The actor in the slot the condition names.
    if let Some(r) = c.slots().first() {
        if let Some(a) = crate::narrative::actor_of(cx.cast, r) {
            put(r, actor_label(v, a));
        } else if let Some(p) = cx.talk.and_then(|t| t.person(r)) {
            let role = condition::role_key(cx, p).unwrap_or_else(|| "-".into());
            put(r, format!("{} ({role})", pname(p)));
        } else {
            put(r, "nobody".into());
        }
    }
    let who = |slot: &str| cx.person(slot);
    match c {
        C::Crisis { .. } => put(
            "crises",
            town.map(|t| {
                t.crises
                    .iter()
                    .map(|x| format!("{} {}", x.kind, num(x.severity as f64)))
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "none".into()),
        ),
        C::Indicator { indicator, .. } => {
            if let Some(m) = m {
                put(
                    "value",
                    num(crate::concerns::indicator(v.r, m as usize, *indicator)),
                );
            }
        }
        C::Concern { .. } => {
            if let Some(w) = c.worry() {
                let ranked: Vec<String> = condition::ranked_worries(cx, w.of, w.by)
                    .into_iter()
                    .take(w.top as usize)
                    .filter_map(|i| content.concerns.get(i).map(|x| x.id.clone()))
                    .collect();
                let by = match w.by {
                    Worry::Felt => "felt",
                    Worry::Measured => "measured",
                };
                let of = match w.of {
                    ConcernOf::Town => "town",
                    ConcernOf::Speaker => "speaker's home",
                    ConcernOf::Here => "here",
                };
                put(
                    &format!("top {} ({by}, {of})", w.top),
                    if ranked.is_empty() {
                        "none".into()
                    } else {
                        ranked.join(", ")
                    },
                );
            }
        }
        C::Industry { .. } => put(
            "industry",
            town.and_then(|t| t.industry.as_deref())
                .and_then(|s| content.sector(s))
                .map(|s| format!("{} ({:?})", s.id, s.driver))
                .unwrap_or_else(|| "none".into()),
        ),
        C::PlantClosure { .. } => {
            let last = cx.st.map(|st| {
                st.events
                    .iter()
                    .rev()
                    .find(|e| {
                        matches!(e.kind, crate::events::EventKind::PlantClosure { municipality, .. } if Some(municipality) == m)
                    })
                    .map(|e| format!("{} days ago", e.date.days_until(today)))
                    .unwrap_or_else(|| "none on record".into())
            });
            put(
                "last closure",
                last.unwrap_or_else(|| "the event log is not read here".into()),
            );
        }
        C::Disaster { .. } => put(
            "months since disaster",
            m.and_then(|m| v.r.economy().munis.get(m as usize))
                .map(|e| e.months_since_disaster.to_string())
                .unwrap_or_default(),
        ),
        C::LawInForce { templates, .. } => {
            let n = m
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
        C::Water { .. } | C::Dry => {
            put("river", mu.is_some_and(|x| x.on_river).to_string());
            put("coast", mu.is_some_and(|x| x.coastal).to_string());
            if matches!(
                c,
                C::Dry
                    | C::Water {
                        feature: WaterFeature::Any
                    }
            ) {
                let waterfront = cx
                    .here_hood()
                    .and_then(|h| world.neighborhoods.get(h.index()))
                    .is_some_and(|n| n.kind == crate::world::NeighborhoodKind::Waterfront);
                put("waterfront", waterfront.to_string());
            }
        }
        C::Broker | C::IsBroker { .. } => put(
            "broker",
            town.and_then(|t| t.broker)
                .map(pname)
                .unwrap_or_else(|| "none".into()),
        ),
        C::Rival | C::RivalRecord { .. } | C::IsRival { .. } => {
            let rival = town.and_then(|t| t.rival);
            put("rival", rival.map(pname).unwrap_or_else(|| "none".into()));
            if matches!(c, C::RivalRecord { .. })
                && let Some(p) =
                    rival.and_then(|r| pol.politicians.iter().find(|p| p.character == r))
            {
                put("record", num(p.record as f64));
            }
        }
        C::Scandal { .. } => put(
            "running scandals in town",
            v.r.media()
                .scandals
                .iter()
                .filter(|x| x.running() && Some(x.town) == m)
                .count()
                .to_string(),
        ),
        C::Headline => put(
            "latest story",
            v.r.media()
                .stories
                .iter()
                .rev()
                .find(|s| Some(s.town) == m)
                .map(|s| format!("{} days ago", s.date.days_until(today)))
                .unwrap_or_else(|| "none".into()),
        ),
        C::PlayerAwareness { .. } | C::PlayerApproval { .. } => {
            // Residents' mean awareness and approval of the player here.
            match condition::standing(cx) {
                Some((aw, ap)) => {
                    put("awareness", num(aw));
                    put("approval", num(ap));
                }
                None => put("residents here", "none".into()),
            }
        }
        C::Shedding { .. } => {
            let found = cx.st.zip(cx.here).and_then(|(st, here)| {
                let Location::Place { place } = here else {
                    return None;
                };
                let p = st.world.places.get(place.index())?;
                let s = content.place_kind(p.kind.key())?.sector.clone()?;
                let x = crate::visits::shedding(st, content, p.municipality, &s);
                Some(format!("{s} (shedding {})", num(x)))
            });
            put("industry here", found.unwrap_or_else(|| "none".into()));
        }
        C::Present { .. } => {
            let here: Vec<String> = cx
                .st
                .zip(cx.here)
                .map(|(st, loc)| {
                    crate::visits::present(st, content, loc)
                        .into_iter()
                        .map(|p| {
                            format!(
                                "{} ({})",
                                pname(p),
                                crate::visits::occupation_of(st, p).unwrap_or("-")
                            )
                        })
                        .collect()
                })
                .unwrap_or_default();
            put(
                "here now",
                if here.is_empty() {
                    "nobody".into()
                } else {
                    here.join(", ")
                },
            );
        }
        C::DayPart { .. } => put(
            "day part",
            cx.part
                .or_else(|| cx.st.map(crate::characters::routines::day_part))
                .map(|p| format!("{p:?}"))
                .unwrap_or_else(|| "the clock is not read here".into()),
        ),
        C::Phase { .. } => put(
            "campaigns running",
            cx.at
                .map(|at| {
                    pol.elections
                        .races
                        .iter()
                        .filter(|r| r.seat.at() == at && r.election_day >= today)
                        .count()
                        .to_string()
                })
                .unwrap_or_else(|| "no jurisdiction".into()),
        ),
        C::ElectionWithin { .. } => put(
            "days to election",
            cx.election_day()
                .filter(|d| *d >= today)
                .map(|d| today.days_until(d).to_string())
                .unwrap_or_else(|| "none".into()),
        ),
        C::MonthsInPlay { .. } => {
            let began = v.play_began();
            let months = if today >= began {
                began.days_until(today) / 30
            } else {
                0
            };
            put("months in play", months.to_string());
        }
        C::Campaigning { .. } => put("campaigning", v.campaigning().to_string()),
        // The player's post as its role word in the run's constitution.
        C::Office { .. } => put(
            "post",
            v.player()
                .and_then(|pl| pol.get(pl.politician))
                .map(|p| {
                    content
                        .constitutions
                        .get(&v.r.meta.constitution)
                        .and_then(|c| crate::politics::role_word(c, p))
                        .unwrap_or("out of office")
                        .to_string()
                })
                .unwrap_or_else(|| "no player".into()),
        ),
        C::HoldsPower { .. } => {
            let con = content.constitutions.get(&v.r.meta.constitution);
            let powers: Vec<String> = match (v.player(), cx.at, con) {
                (Some(pl), Some(at), Some(con)) => crate::politics::posts_of(pol, pl.politician)
                    .into_iter()
                    .filter(|p| p.at() == at)
                    .flat_map(|p| crate::politics::powers_of(con, p).to_vec())
                    .map(|p| format!("{p:?}"))
                    .collect(),
                _ => Vec::new(),
            };
            put(
                "powers here",
                if powers.is_empty() {
                    "none".into()
                } else {
                    powers.join(", ")
                },
            );
        }
        C::Track { track, .. } => put(
            "value",
            track_value(v, *track)
                .map(num)
                .unwrap_or_else(|| "-".into()),
        ),
        C::Money { .. } => put(
            "funds",
            v.player()
                .map(|p| p.money.units().to_string())
                .unwrap_or_default(),
        ),
        // E2 (ENGAGEMENT §4.4): the newest matching answer, and the
        // player's laws.
        C::Answered { .. } => {
            let found = cast::answered_records(v, c);
            put(
                "last answered",
                found
                    .first()
                    .map(|a| format!("{} ({} days ago)", a.option, a.date.days_until(today)))
                    .unwrap_or_else(|| "never in the window".into()),
            );
        }
        C::SponsoredLaw { .. } => {
            let mine = v.player().map(|p| p.politician);
            let laws =
                v.r.policy()
                    .instances
                    .iter()
                    .filter(|p| mine.is_some() && p.sponsor == mine)
                    .count();
            put("player's bills and laws", laws.to_string());
        }
        C::Knows { .. } | C::Occupation { .. } | C::Any { .. } => {}
        C::PlayerKnows { subject, .. } => {
            if let Some(t) = cx.talk {
                put(
                    "player knows",
                    t.subject(subject)
                        .map(|s| crate::player::knowledge::knows(t.st, &s).key().to_string())
                        .unwrap_or_else(|| "no such subject here".into()),
                );
            }
        }
        C::Trait { slot, .. } => put(
            "traits",
            who(slot)
                .and_then(|p| pol.characters.get(p))
                .map(|c| c.traits.join(", "))
                .unwrap_or_default(),
        ),
        C::Personality { slot, axis, .. } => {
            if let Some(p) = who(slot).and_then(|p| characters::profile(pol, world, p)) {
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
        C::Goal { slot, .. } => put(
            "goals",
            who(slot)
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
        C::Opinion { slot, .. } => {
            if let (Some(p), Some(me)) = (who(slot), me) {
                put("opinion", num(characters::opinion(pol, p, me) as f64));
            }
        }
        C::Trust { slot, .. } => {
            if let (Some(p), Some(me)) = (who(slot), me) {
                put("trust", num(characters::trust(pol, content, p, me) as f64));
            }
        }
        // A word, never a count (DEC-114 §5).
        C::Familiar { slot, .. } => {
            if let (Some(p), Some(me)) = (who(slot), me) {
                let band = characters::familiar::band(pol, content, p, me);
                put("familiar", band.key().unwrap_or("familiar-stranger").into());
            }
        }
        C::Remembers { slot, .. } => {
            if let (Some(p), Some(me)) = (who(slot), me) {
                let kinds: Vec<&str> = pol
                    .characters
                    .get(p)
                    .map(|c| {
                        c.memories
                            .iter()
                            .filter(|x| x.actor == me)
                            .map(|x| x.kind.key())
                            .collect()
                    })
                    .unwrap_or_default();
                put("memories of player", kinds.join(", "));
            }
        }
        C::Favour { slot, .. } => {
            if let (Some(p), Some(me)) = (who(slot), me) {
                put(
                    "player owes",
                    characters::favours_owed(pol, me, p).to_string(),
                );
                put(
                    "owed to player",
                    characters::favours_owed(pol, p, me).to_string(),
                );
            }
        }
        C::BillCosts { .. } => {
            let mut a = condition::Args::new();
            if condition::holds(cx, c, &mut a) {
                for (k, x) in a {
                    put(&k, x);
                }
            } else {
                put("bill", "none of theirs that a yes would cost".into());
            }
        }
        C::Met { slot, .. } => {
            if let (Some(p), Some(me)) = (who(slot), me) {
                let talked = v.player().is_some_and(|pl| pl.talk.used.contains_key(&p));
                let first = pol.characters.get(p).and_then(|ch| {
                    ch.memories
                        .iter()
                        .filter(|x| x.actor == me && x.kind == characters::MemoryKind::Met)
                        .map(|x| x.date)
                        .min()
                });
                put("talked with the player", talked.to_string());
                put(
                    "first met",
                    first
                        .map(|d| format!("{} days ago", d.days_until(today)))
                        .unwrap_or_else(|| "never".into()),
                );
            }
        }
        C::PartyLineAgainstWard { slot } => {
            if let Some(Actor::Policy { id }) = crate::narrative::actor_of(cx.cast, slot) {
                put("party line", v.party_line(id).to_string());
                put("ward lean", num(v.ward_lean(id)));
            }
        }
        C::CanVote { slot } => {
            if let Some(Actor::Policy { id }) = crate::narrative::actor_of(cx.cast, slot) {
                put("can vote", v.can_vote(id).to_string());
            }
        }
        C::Tells { subject, .. } => {
            if let Some(t) = cx.talk {
                put(
                    "would tell",
                    t.subject(subject)
                        .map(|s| t.told(&s).key().to_string())
                        .unwrap_or_else(|| "no such subject here".into()),
                );
                put("discretion cap", t.cap.key().into());
            }
        }
        C::Voice { .. } => {
            if let Some(t) = cx.talk {
                put(
                    "speaker's voice",
                    crate::talk::reply::voice(t)
                        .map(|v| v.id.clone())
                        .unwrap_or_else(|| "none".into()),
                );
            }
        }
        C::About { .. } => {
            if let Some(t) = cx.talk {
                put(
                    "about",
                    t.about
                        .map(|a| match a {
                            crate::talk::AboutRef::Place { place } => world
                                .places
                                .get(place.index())
                                .map(|p| format!("place: {}", p.kind.key()))
                                .unwrap_or_else(|| "place".into()),
                            other => other.kind().key().to_string(),
                        })
                        .unwrap_or_else(|| "nothing".into()),
                );
                put(
                    "speaker's workplace",
                    t.workplace(t.speaker)
                        .map(|p| p.kind.key().to_string())
                        .unwrap_or_else(|| "none".into()),
                );
            }
        }
        // A policy family's bound slots (the families' own "Why?" comes
        // with their views, F3).
        C::HoodConcern { .. }
        | C::HoodHazard { .. }
        | C::SectorDecline { .. }
        | C::PlaceOpen { .. }
        | C::PlaceClosed { .. } => {}
        // An arc run (DEC-109): its stage, marks and heat.
        C::Arc { arc, .. } => put(
            "runs",
            v.r.arcs()
                .map(|a| {
                    a.runs
                        .iter()
                        .filter(|r| &r.template == arc)
                        .map(|r| r.stage.clone())
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default(),
        ),
        C::ArcMark { .. }
        | C::ArcHeat { .. }
        | C::ArcDays { .. }
        | C::Displaced { .. }
        | C::Recovered { .. }
        | C::ArcLastTurn { .. } => {
            if let Some(r) = cx.arc {
                put("stage", r.stage.clone());
                put("marks", r.marks.join(", "));
                put("heat", num(f64::from(r.heat)));
                put("last turn", r.before.clone().unwrap_or_default());
            } else {
                put("arc", "no run here (never holds)".into());
            }
        }
        C::BudgetYear { .. } => {
            if let Some(at) = cx.at {
                let left = v
                    .date()
                    .days_until(crate::finance::budget_year_turns(at, v.date()));
                put("days to the budget year's turn", left.to_string());
            }
        }
        C::BudgetBalance { .. } | C::BodySupport { .. } => {
            let mut a = condition::Args::new();
            let unbounded = match c {
                C::BudgetBalance { .. } => C::BudgetBalance {
                    min: None,
                    max: None,
                },
                C::BodySupport { power, opinion, .. } => C::BodySupport {
                    power: *power,
                    opinion: *opinion,
                    min: None,
                    max: None,
                },
                _ => c.clone(),
            };
            if condition::holds(cx, &unbounded, &mut a) {
                for (k, x) in a {
                    put(&k, x);
                }
            }
        }
        C::LastRace { .. } | C::SponsorsBill { .. } | C::Proposal { .. } => {}
        // The plant as it stands (any state, no date asked).
        C::Plant { slot, .. } => {
            let mut a = condition::Args::new();
            let any = C::Plant {
                slot: slot.clone(),
                status: Vec::new(),
                closes_within_days: None,
            };
            if condition::holds(cx, &any, &mut a) {
                for k in ["stem", "jobs", "closes_day", "closes_year"] {
                    if let Some(x) = a.get(k) {
                        put(k, x.clone());
                    }
                }
            }
        }
        C::BodyMajority { power, .. } => {
            if let Some(at) = cx.at {
                let n = crate::arcs::cast::power_holders(v, at, *power).len();
                put("holders of the power", n.to_string());
            }
        }
    }
    out
}

/// The arguments the conditions that hold provide (the line's own
/// arguments, as the overheard draw gives them), with `town` and `hood`,
/// and the story a `headline` found.
pub fn heard_args(
    cx: &condition::Ctx<'_>,
    cs: &[Condition],
) -> (BTreeMap<String, String>, Option<u32>) {
    let mut args = BTreeMap::new();
    cx.story.set(None);
    for c in cs {
        condition::holds(cx, c, &mut args);
    }
    let world = cx.v.r.world;
    if let Some(m) = cx
        .town_id()
        .and_then(|m| world.municipalities.get(m.index()))
    {
        args.insert("town".into(), m.name.clone());
    }
    if let Some(n) = cx
        .here_hood()
        .and_then(|h| world.neighborhoods.get(h.index()))
    {
        args.insert("hood".into(), n.name.clone());
    }
    (args, cx.story.get())
}

/// A track's value as the evaluator reads it (display only; the verdict is
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

// --- Items ------------------------------------------------------------------------------

/// The kinds of content item the workbench explains. Asks, secrets and
/// curiosities join when their registries exist.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    Dilemma,
    Situational,
    Overheard,
    Topic,
    Line,
    Voice,
    Scene,
}

impl ItemKind {
    pub const ALL: [ItemKind; 7] = [
        ItemKind::Dilemma,
        ItemKind::Situational,
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
    let cx = condition::Ctx::narrative(v, &cast);
    let e = CondExplainer { cx: &cx };
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
    } else if def.arc_only || staged_by_live_run(st, content, &def.id) {
        // An arc's decision beat: the run asks for it at its stage.
        let stage = st.arcs.runs.iter().find_map(|run| {
            let s = content.arc(&run.template)?.stage(&run.stage)?;
            names_dilemma(s, &def.id).then(|| (run, s.id.clone()))
        });
        let arcs: Vec<&str> = content
            .arcs
            .iter()
            .filter(|a| a.stages.iter().any(|s| names_dilemma(s, &def.id)))
            .map(|a| a.id.as_str())
            .collect();
        let mut g = Node::new(
            "gate",
            "route",
            format!("a decision beat of: {}", arcs.join(", ")),
            stage.is_some(),
        );
        if let Some((run, sid)) = &stage {
            g = g.with("at stage", format!("{} / {sid}", run.template));
            preset = run.narrative_cast();
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
        gates.push(Node::new(
            "gate",
            "route",
            "the director's spotlight (not a beat of a running arc)",
            true,
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
    let cx = condition::Ctx::town(v, st, MunicipalityId(m));
    let e = CondExplainer { cx: &cx };
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

// --- Arcs' decision beats ---------------------------------------------------------------

/// A stage has a decision beat of this dilemma.
fn names_dilemma(s: &mandate_content::arcs::StageDef, dilemma: &str) -> bool {
    s.beats.iter().any(|b| {
        matches!(&b.kind, mandate_content::arcs::BeatKind::Decision { dilemma: d, .. } if d == dilemma)
    })
}

/// A live run's template names the dilemma as a decision beat (the
/// spotlight leaves it to the run).
fn staged_by_live_run(st: &State, content: &ContentBundle, dilemma: &str) -> bool {
    st.arcs.runs.iter().any(|r| {
        content
            .arc(&r.template)
            .is_some_and(|a| a.stages.iter().any(|s| names_dilemma(s, dilemma)))
    })
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
    v: &View<'_>,
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
        let Some(hood) = crate::places::hood_of(st, loc) else {
            continue;
        };
        let cx = condition::Ctx::at_location(v, st, loc);
        let e = CondExplainer { cx: &cx };
        // The overheard system's verdict: the line is among those that hold.
        let holds = crate::overheard::available(st, content, loc)
            .iter()
            .any(|l| l.id == def.id);
        let drawn = crate::overheard::lines(st, content, loc)
            .iter()
            .any(|l| l.id == def.id);
        let mut pk = Vec::new();
        if let Some(c) = &def.concern {
            let felt: Vec<String> = crate::concerns::top(
                &crate::concerns::ranked(
                    &st.population,
                    content,
                    &st.world,
                    crate::concerns::Scope::Neighbourhood(hood),
                    Worry::Felt,
                ),
                3,
            )
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
        && crate::places::hood_of(st, Location::Place { place: p }).is_some()
    {
        let cx = condition::Ctx::at_location(v, st, Location::Place { place: p });
        let (args, _) = heard_args(&cx, &def.when);
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
        AboutKind::Jurisdiction => Some(AboutRef::Jurisdiction {
            at: crate::world::Jurisdiction::town(crate::world::MunicipalityId(m)),
        }),
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

/// `sample_abouts`, plus, when the topic takes a place, the speaker's own
/// workplace (asked about from inside, a reply of its own).
pub fn sample_abouts_for(
    st: &State,
    content: &ContentBundle,
    speaker: CharacterId,
    kinds: &[AboutKind],
    required: bool,
) -> Vec<Option<AboutRef>> {
    let mut out = sample_abouts(st, content, kinds, required);
    if kinds.contains(&AboutKind::Place)
        && let Some(cx) = talk_ctx(st, content, speaker, None)
        && let Some(p) = cx.workplace(speaker)
    {
        let a = Some(AboutRef::Place { place: p.id });
        if !out.contains(&a) {
            out.push(a);
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

/// A conversation's conditions explained, with the conversation bound.
fn talk_when(ctx: &TalkCtx<'_>, cs: &[Condition]) -> Node {
    condition::on_state(ctx.st, ctx.content, |v| {
        let cx = condition::Ctx::talk(v, ctx);
        explain_all(&CondExplainer { cx: &cx }, "when", cs)
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
    kids.push(talk_when(ctx, &o.when));
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
        kids.push(talk_when(ctx, &l.when));
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
        let when = talk_when(ctx, &vd.when);
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
