//! Answering decisions (the Commands phase, OVERVIEW §4.2). The only path by
//! which a decision changes the world: the player's answer, or the default
//! option when the deadline passes (the player's inaction). Every effect is
//! a real engine change (DEC-073), recorded for the aftermath.

use super::{Actor, Applied, DecisionId, DecisionStatus, FactKind, Resolution, RunEnd, person_of};
use crate::actions::ActionResult;
use crate::characters::{self, MemoryKind, Subject};
use crate::commands::EngineError;
use crate::events::EventKind;
use crate::player::knowledge::{self, Level};
use crate::politics::Vote;
use crate::spine::{Cause, PlayerAct};
use crate::state::State;
use crate::world::NeighborhoodId;
use mandate_content::{ContentBundle, EffectDef, RiderKind, Scope, VoteChoice};
use mandate_core::Money;
use mandate_core::math::clamp;

fn bad(msg: &str) -> EngineError {
    EngineError::InvalidCommand(msg.into())
}

fn memory_kind(key: &str) -> Option<MemoryKind> {
    use MemoryKind::*;
    [
        BackedMyBill,
        OpposedMyBill,
        PromisedFavour,
        KeptPromise,
        BrokePromise,
        BrokeRanks,
        LobbiedMe,
        PressedMe,
        DefeatedMe,
        LostToMe,
        VetoedMyBill,
        SignedMyBill,
        Met,
        Scandal,
        StoodWithMe,
        StoodAgainstMe,
        IgnoredMe,
        BlamedMe,
    ]
    .into_iter()
    .find(|k| k.key() == key)
}

/// Neighbourhoods an approval or awareness effect reaches.
fn hoods(state: &State, scope: Scope, hood: Option<u32>) -> Vec<usize> {
    if let Some(h) = hood {
        return vec![h as usize];
    }
    let Some(pl) = state.player.as_ref() else {
        return Vec::new();
    };
    match scope {
        Scope::Ward => crate::policy::ward_hoods(&state.world, pl.ward),
        Scope::Town => state.world.hoods_of(pl.municipality).collect(),
    }
}

/// Whether the player could take an option now (time and money).
pub fn affordable(state: &State, effects: &[EffectDef]) -> Result<(), &'static str> {
    let Some(pl) = state.player.as_ref() else {
        return Err("no player");
    };
    let slots: u32 = effects
        .iter()
        .map(|e| match e {
            EffectDef::Slots { n } => *n as u32,
            _ => 0,
        })
        .sum();
    if slots > pl.slots_left() as u32 {
        return Err("not enough time left this turn");
    }
    let spend: f64 = effects
        .iter()
        .map(|e| match e {
            EffectDef::Money { amount } if *amount < 0.0 => -*amount,
            _ => 0.0,
        })
        .sum();
    if spend > pl.money.units() as f64 {
        return Err("not enough campaign funds");
    }
    Ok(())
}

/// The grants a knowledge definition gives for this decision.
fn knowledge_grants(
    state: &State,
    content: &ContentBundle,
    def: &mandate_content::SubjectDef,
    level: Level,
    cast: &[super::CastEntry],
    here: Option<NeighborhoodId>,
) -> Vec<knowledge::Grant> {
    let r = crate::scheduler::Reads::of_state(state, content);
    let v = super::cast::View::new(&r, &state.politics);
    super::learn::grants(&v, def, level, cast, here)
}

/// Where a decision happens, for subjects named without a role: its
/// scene's neighbourhood, else the player's home neighbourhood.
fn decision_hood(state: &State, place: Option<crate::places::Location>) -> Option<NeighborhoodId> {
    place
        .and_then(|l| crate::places::hood_of(state, l))
        .or_else(|| crate::places::home_neighborhood(state))
}

/// The root of an answer: the player's choice, or letting it pass.
fn answer_cause(by_default: bool) -> Cause {
    Cause::Player(if by_default {
        PlayerAct::LetPass
    } else {
        PlayerAct::Answer
    })
}

#[allow(clippy::too_many_arguments)]
fn apply_effect(
    state: &mut State,
    content: &ContentBundle,
    id: DecisionId,
    cast: &[super::CastEntry],
    about: Option<super::About>,
    here: Option<NeighborhoodId>,
    e: &EffectDef,
    by_default: bool,
) -> Applied {
    let date = state.calendar.date;
    let me = characters::player_character(state);
    let me_pol = state.player.as_ref().map(|p| p.politician);
    let subject = Some(Subject::Decision { id: id.0 });
    let cause = answer_cause(by_default);
    let skip = |what: &str| Applied::Skipped {
        effect: what.into(),
    };
    match e {
        EffectDef::Opinion {
            role,
            delta,
            memory,
        } => {
            let (Some(p), Some(me)) = (person_of(cast, role), me) else {
                return skip("opinion");
            };
            characters::touch(&mut state.politics, content, state.meta.seed, p, date);
            let applied = match memory.as_deref().and_then(memory_kind) {
                Some(kind) => characters::remember(
                    &mut state.politics,
                    content,
                    date,
                    p,
                    me,
                    kind,
                    subject,
                    Some(*delta),
                ),
                None => characters::change_opinion(&mut state.politics, content, p, me, *delta),
            };
            Applied::Opinion {
                person: p,
                delta: applied,
            }
        }
        EffectDef::Approval {
            scope,
            classes,
            neighborhood,
            delta,
        } => {
            let hood = neighborhood
                .as_deref()
                .and_then(|r| match super::actor_of(cast, r) {
                    Some(Actor::Neighborhood { id }) => Some(id),
                    _ => None,
                });
            let (mut acc, mut w) = (0.0, 0.0);
            for h in hoods(state, *scope, hood) {
                let Some(hp) = state.population.hoods.get_mut(h) else {
                    continue;
                };
                for (ci, c) in hp.cohorts.iter_mut().enumerate() {
                    let class = crate::population::class_of(ci) as u8;
                    if !classes.is_empty() && !classes.contains(&class) {
                        continue;
                    }
                    // Room-scaled like every approval change (canvassing, votes).
                    let room = 1.0 - (c.player_approval as f64).abs();
                    let before = c.player_approval;
                    c.player_approval =
                        clamp(c.player_approval as f64 + delta * room, -1.0, 1.0) as f32;
                    acc += (c.player_approval - before) as f64 * c.count as f64;
                    w += c.count as f64;
                }
            }
            Applied::Approval {
                scope: if hood.is_some() {
                    "hood".into()
                } else {
                    match scope {
                        Scope::Ward => "ward".into(),
                        Scope::Town => "town".into(),
                    }
                },
                neighborhood: hood,
                classes: classes.clone(),
                delta: (acc / w.max(1.0)) as f32,
            }
        }
        EffectDef::Awareness { scope, delta } => {
            let (mut acc, mut w) = (0.0, 0.0);
            for h in hoods(state, *scope, None) {
                let Some(hp) = state.population.hoods.get_mut(h) else {
                    continue;
                };
                for c in hp.cohorts.iter_mut() {
                    let aw = c.player_awareness as f64;
                    let step = if *delta > 0.0 {
                        delta * (1.0 - aw)
                    } else {
                        *delta
                    };
                    c.player_awareness = clamp(aw + step, 0.0, 1.0) as f32;
                    acc += (c.player_awareness as f64 - aw) * c.count as f64;
                    w += c.count as f64;
                }
            }
            Applied::Awareness {
                scope: match scope {
                    Scope::Ward => "ward".into(),
                    Scope::Town => "town".into(),
                },
                delta: (acc / w.max(1.0)) as f32,
            }
        }
        EffectDef::Money { amount } => {
            let Some(pl) = state.player.as_mut() else {
                return skip("money");
            };
            let before = pl.money;
            let after = (pl.money.units() as f64 + amount).max(0.0);
            pl.money = Money::from_units(after.round() as i64);
            Applied::Money {
                amount: (pl.money - before).units() as f64,
            }
        }
        EffectDef::Health { delta } => track(state, |t| &mut t.health, *delta)
            .map(|d| Applied::Health { delta: d })
            .unwrap_or_else(|| skip("health")),
        EffectDef::Standing { delta } => track(state, |t| &mut t.standing, *delta)
            .map(|d| Applied::Standing { delta: d })
            .unwrap_or_else(|| skip("standing")),
        EffectDef::LegalJeopardy { delta } => track(state, |t| &mut t.legal_jeopardy, *delta)
            .map(|d| Applied::LegalJeopardy { delta: d })
            .unwrap_or_else(|| skip("legal_jeopardy")),
        EffectDef::Slots { n } => {
            // Defaults never cost time (validated); answers check it first.
            if by_default {
                return skip("slots");
            }
            if let Some(pl) = state.player.as_mut() {
                pl.slots_used = pl.slots_used.saturating_add(*n);
            }
            Applied::Slots { n: *n }
        }
        EffectDef::Record { delta } => {
            let Some(me) = me_pol else {
                return skip("record");
            };
            let before = state.politics.get(me).map(|p| p.record).unwrap_or(0.0);
            if *delta >= 0.0 {
                crate::politics::add_record(&mut state.politics, me, *delta as f32);
            } else if let Some(p) = state.politics.get_mut(me) {
                p.record = (p.record + *delta as f32).max(0.0);
            }
            let after = state.politics.get(me).map(|p| p.record).unwrap_or(0.0);
            Applied::Record {
                delta: after - before,
            }
        }
        EffectDef::Favour {
            role,
            owed_to_player,
        } => {
            let (Some(p), Some(me)) = (person_of(cast, role), me) else {
                return skip("favour");
            };
            let (debtor, creditor) = if *owed_to_player { (p, me) } else { (me, p) };
            characters::owe(
                &mut state.politics,
                debtor,
                creditor,
                "favour-decision",
                subject,
                date,
            );
            if !owed_to_player {
                characters::remember(
                    &mut state.politics,
                    content,
                    date,
                    p,
                    me,
                    MemoryKind::PromisedFavour,
                    subject,
                    Some(0.0),
                );
            }
            Applied::Favour {
                person: p,
                owed_to_player: *owed_to_player,
            }
        }
        EffectDef::Repay { role } => {
            let (Some(p), Some(me)) = (person_of(cast, role), me) else {
                return skip("repay");
            };
            match characters::repay(&mut state.politics, me, p) {
                Some(f) => {
                    characters::remember(
                        &mut state.politics,
                        content,
                        date,
                        p,
                        me,
                        MemoryKind::KeptPromise,
                        f.subject.or(subject),
                        None,
                    );
                    Applied::Repaid { person: p }
                }
                None => skip("repay"),
            }
        }
        EffectDef::Vote { role, vote } => {
            let Some(Actor::Policy { id: bill }) = super::actor_of(cast, role) else {
                return skip("vote");
            };
            let r = crate::scheduler::Reads::of_state(state, content);
            let v = super::cast::View::new(&r, &state.politics);
            if !v.can_vote(bill) {
                return skip("vote");
            }
            let sign = match vote {
                VoteChoice::Yes => 1,
                VoteChoice::No => -1,
                VoteChoice::Abstain => 0,
                VoteChoice::WithParty => v.party_line(bill) as i32,
                VoteChoice::WithWard => {
                    let l = v.ward_lean(bill);
                    if l > 0.0 {
                        1
                    } else if l < 0.0 {
                        -1
                    } else {
                        0
                    }
                }
            };
            let v = match sign {
                1 => Vote::Yes,
                -1 => Vote::No,
                _ => Vote::Abstain,
            };
            match crate::actions::declare_vote(state, content, &bill, &v) {
                Ok(_) => Applied::Vote {
                    policy: bill,
                    vote: match v {
                        Vote::Yes => "yes",
                        Vote::No => "no",
                        Vote::Abstain => "abstain",
                    }
                    .into(),
                },
                Err(_) => skip("vote"),
            }
        }
        EffectDef::Rider {
            role,
            rider,
            ward_of,
        } => {
            let Some(Actor::Policy { id: bill }) = super::actor_of(cast, role) else {
                return skip("rider");
            };
            let ward = match ward_of {
                Some(r) => person_of(cast, r).and_then(|c| {
                    state
                        .politics
                        .politicians
                        .iter()
                        .find(|p| p.character == c)
                        .and_then(|p| p.role.constituency())
                }),
                None => state.player.as_ref().map(|p| p.ward),
            };
            let Some(ward) = ward else {
                return skip("rider");
            };
            let Some(p) = state
                .policy
                .instances
                .get_mut(bill as usize)
                .filter(|p| p.status == crate::policy::PolicyStatus::UnderConsideration)
            else {
                return skip("rider");
            };
            use crate::policy::Clause;
            let clause = match rider {
                RiderKind::WardProject => Clause::Earmark { ward },
                RiderKind::Sunset => Clause::Sunset { years: 4 },
            };
            let present = p.clauses.iter().any(|c| match (c, &clause) {
                (Clause::Earmark { ward: a }, Clause::Earmark { ward: b }) => a == b,
                (Clause::Sunset { .. }, Clause::Sunset { .. }) => true,
                _ => false,
            });
            if present {
                return skip("rider");
            }
            p.clauses.push(clause);
            Applied::Rider {
                policy: bill,
                rider: match rider {
                    RiderKind::WardProject => "ward_project",
                    RiderKind::Sunset => "sunset",
                }
                .into(),
            }
        }
        EffectDef::CallIn { role, bill } => {
            let (Some(p), Some(me), Some(me_pol)) = (person_of(cast, role), me, me_pol) else {
                return skip("call_in");
            };
            let Some(Actor::Policy { id: policy }) = super::actor_of(cast, bill) else {
                return skip("call_in");
            };
            let Some(debtor) = state
                .politics
                .politicians
                .iter()
                .find(|x| x.character == p && x.in_office())
                .map(|x| x.id)
            else {
                return skip("call_in");
            };
            if characters::repay(&mut state.politics, p, me).is_none() {
                return skip("call_in");
            }
            // The player's side of the bill: their own bill, else their
            // declared vote, else yes.
            let own = state
                .policy
                .instances
                .get(policy as usize)
                .is_some_and(|x| x.sponsor == Some(me_pol));
            let side = if own {
                1.0
            } else {
                match state.politics.declared_votes.get(&(me_pol.0, policy)) {
                    Some(v) if *v < 0 => -1.0,
                    _ => 1.0,
                }
            };
            *state.politics.sway.entry((debtor.0, policy)).or_default() +=
                (side * content.actions.lobby.trade_gain) as f32;
            characters::remember(
                &mut state.politics,
                content,
                date,
                p,
                me,
                MemoryKind::PressedMe,
                Some(Subject::Policy { id: policy }),
                None,
            );
            Applied::CalledIn { person: p, policy }
        }
        EffectDef::Draft { template, params } => {
            match crate::actions::draft(state, content, template, params) {
                Ok(id) => Applied::Drafted { policy: id },
                Err(_) => skip("draft"),
            }
        }
        EffectDef::Hidden { with_role, .. } => {
            let (Some(spec), Some(me)) = (e.hidden_spec(), me_pol) else {
                return skip("hidden");
            };
            let risk = crate::media::risk_within_horizon(
                content,
                &crate::media::exposure(state, content, &spec, me),
            );
            let actors = with_role
                .as_deref()
                .and_then(|r| super::actor_of(cast, r))
                .into_iter()
                .collect();
            crate::media::hide(state, content, &spec, me, actors, cause);
            Applied::Hidden {
                kind: spec.kind,
                risk: risk as f32,
            }
        }
        EffectDef::ScandalResponse { response } => match about {
            Some(super::About::Scandal { id }) => {
                crate::media::respond(state, content, id, *response);
                Applied::Responded {
                    response: response.key().into(),
                }
            }
            _ => skip("scandal_response"),
        },
        EffectDef::LegalResponse { response, role } => match about {
            Some(super::About::Case { id }) => {
                let ally = role.as_deref().and_then(|r| person_of(cast, r));
                crate::justice::respond(state, content, id, *response, ally);
                Applied::Responded {
                    response: response.key().into(),
                }
            }
            _ => skip("legal_response"),
        },
        // Knowledge (DEC-091): the player domain's ledger, written by the
        // answer (the Commands phase), as every option effect is.
        EffectDef::Learn { subject, level } => {
            let grants = knowledge_grants(state, content, subject, *level, cast, here);
            learned(state, subject.key(), *level, grants, id)
        }
        EffectDef::RevealPlace { role, kinds } => {
            let def = mandate_content::SubjectDef::Place {
                role: role.clone(),
                kinds: kinds.clone(),
            };
            // Without a role, a reveal is anywhere in town, not just here.
            let at = if role.is_some() { here } else { None };
            let grants = knowledge_grants(state, content, &def, Level::Known, cast, at);
            learned(state, "place", Level::Known, grants, id)
        }
    }
}

/// Teach grants from a decision. The aftermath records how many subjects
/// are now at the level (ones already known count: the effect held).
fn learned(
    state: &mut State,
    subject: &str,
    level: Level,
    grants: Vec<knowledge::Grant>,
    id: DecisionId,
) -> Applied {
    if grants.is_empty() || state.player.is_none() {
        return Applied::Skipped {
            effect: "learn".into(),
        };
    }
    let n = grants.len() as u32;
    knowledge::teach(state, grants, &knowledge::Source::Decision { decision: id });
    Applied::Learned {
        subject: subject.into(),
        level: level.key().into(),
        n,
    }
}

fn track(
    state: &mut State,
    f: impl Fn(&mut crate::player::Tracks) -> &mut f32,
    delta: f64,
) -> Option<f32> {
    let pl = state.player.as_mut()?;
    let t = f(&mut pl.tracks);
    let before = *t;
    *t = clamp(*t as f64 + delta, 0.0, 100.0) as f32;
    Some(*t - before)
}

/// Answer a pending decision (the player's answer, or the default when the
/// deadline passes). Applies the option's effects, records the aftermath,
/// and moves its storyline on.
pub fn answer(
    state: &mut State,
    content: &ContentBundle,
    id: DecisionId,
    option: &str,
    by_default: bool,
) -> Result<ActionResult, EngineError> {
    let date = state.calendar.date;
    let d = state
        .narrative
        .decision(id)
        .filter(|d| d.status == DecisionStatus::Pending)
        .cloned()
        .ok_or_else(|| bad("no such decision is waiting"))?;
    let def = content
        .dilemma(&d.template)
        .ok_or_else(|| bad("that decision no longer exists"))?;
    let opt = def
        .options
        .iter()
        .find(|o| o.id == option)
        .ok_or_else(|| bad("no such option"))?;
    if !by_default {
        affordable(state, &opt.effects).map_err(bad)?;
    }
    let here = decision_hood(state, d.place);
    let applied: Vec<Applied> = opt
        .effects
        .iter()
        .map(|e| apply_effect(state, content, id, &d.cast, d.about, here, e, by_default))
        .collect();
    if !by_default {
        // Answering is meeting (DEC-091, §1.3): every cast person `known`,
        // plus what the scene teaches. A decision let pass teaches only its
        // default option's own effects.
        let mut grants: Vec<knowledge::Grant> = {
            let r = crate::scheduler::Reads::of_state(state, content);
            let v = super::cast::View::new(&r, &state.politics);
            super::learn::cast_people(&v, &d.cast)
                .into_iter()
                .map(|person| knowledge::Grant {
                    subject: knowledge::Subject::Person { person },
                    level: Level::Known,
                })
                .collect()
        };
        if let Some(sc) = def.scene.as_deref().and_then(|s| content.scene(s)) {
            for t in &sc.teaches {
                grants.extend(knowledge_grants(
                    state, content, &t.subject, t.level, &d.cast, here,
                ));
            }
        }
        knowledge::teach(state, grants, &knowledge::Source::Decision { decision: id });
    }
    let n = &mut state.narrative;
    if let Some(x) = n.decisions.iter_mut().find(|x| x.id == id) {
        x.status = if by_default {
            DecisionStatus::Defaulted
        } else {
            DecisionStatus::Answered
        };
        x.resolution = Some(Resolution {
            option: option.into(),
            date,
            by_default,
            applied,
        });
    }
    n.log(
        date,
        FactKind::DecisionResolved {
            decision: id,
            option: option.into(),
            by_default,
        },
        d.cast.iter().map(|e| e.actor).collect(),
        true,
        (d.relevance / 3.0).clamp(0.0, 1.0),
    );
    // The storyline moves to the branch the answer chose.
    if let Some(rid) = d.storyline
        && let Some(i) = n.storylines.iter().position(|r| r.id == rid)
        && let Some(sdef) = content.storyline(&n.storylines[i].template)
    {
        let r = &mut n.storylines[i];
        let cur = r.stage as usize;
        let next = sdef
            .stages
            .get(cur)
            .and_then(|st| st.branches.get(option))
            .and_then(|k| sdef.stages.iter().position(|s| &s.id == k))
            .unwrap_or(cur + 1);
        r.stage = next as u32;
        r.entered = false;
        r.since = date;
        r.decision = None;
        if next >= sdef.stages.len() {
            super::director::end_run(n, content, i, date, RunEnd::Ended);
        }
    }
    let cause = answer_cause(by_default);
    state.events.record(
        date,
        EventKind::DecisionResolved {
            decision: id.0,
            template: d.template.clone(),
            option: option.into(),
            by_default,
        },
        cause,
    );
    let mut res = ActionResult {
        outcome_key: opt.aftermath_key.clone(),
        ..Default::default()
    };
    res.args.insert("decision".into(), id.0.to_string());
    Ok(res)
}

/// Apply the default of every decision whose deadline falls before `until`
/// (the end of the turn being ended): the player let it pass.
pub fn resolve_due(state: &mut State, content: &ContentBundle, until: mandate_core::SimDate) {
    if !state.narrative.resolve_defaults {
        // Lab: decisions expire without consequence (the spotlight audit).
        let date = state.calendar.date;
        let n = &mut state.narrative;
        let mut runs = Vec::new();
        for d in n.decisions.iter_mut() {
            if d.status == DecisionStatus::Pending && d.deadline <= until {
                d.status = DecisionStatus::Lapsed;
                d.resolution = Some(Resolution {
                    option: String::new(),
                    date,
                    by_default: true,
                    applied: Vec::new(),
                });
                runs.extend(d.storyline);
            }
        }
        for r in n.storylines.iter_mut().filter(|r| runs.contains(&r.id)) {
            r.decision = None;
            r.entered = false;
        }
        return;
    }
    let due: Vec<(DecisionId, String)> = state
        .narrative
        .pending()
        .filter(|d| d.deadline <= until)
        .filter_map(|d| {
            let def = content.dilemma(&d.template)?;
            let o = def.options.iter().find(|o| o.default)?;
            Some((d.id, o.id.clone()))
        })
        .collect();
    for (id, opt) in due {
        let _ = answer(state, content, id, &opt, true);
    }
}
