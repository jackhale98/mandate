//! Offers (ENGAGEMENT §3.2): the weekly draw at a sliding rate, and the
//! doorstep and conversation asks that a canvass or a talk brings. A
//! doorstep ask comes from a household the canvass knocked on (DEC-114
//! §3.4), whose speaker the canvass promoted when its roll came up
//! (`doorstep_due`), else from a resident of the neighbourhood.
//!
//! The week's chance of an offer is
//! `min(1, new_per_month × 12 / 52 × intent_by_open(open) × intent_by_quiet(days since the last offer))`:
//! a player with nothing open is asked sooner, a busy one is left alone.
//! Each template whose triggers hold, whose `min_career_days` has passed and
//! whose asker can be cast is a candidate, weighted by its `weight` × the
//! asker's opinion of the player × the asker's history (callbacks); one is
//! drawn with a keyed draw (`Stream::Asks`, keyed by week). Asks are aimed
//! at the player: whether one exists never depends on what the player knows
//! (DEC-091; validation refuses `knows` in triggers).

use super::{Ask, AskId, AskState, AskStatus, Asker, Deed, History, Need, StatementNeed};
use crate::characters::{self, CharacterId};
use crate::narrative::cast::{self, View};
use crate::narrative::{Actor, CastEntry, actor_of, person_of};
use crate::policy::PolicyStatus;
use crate::scheduler::{Outs, Reads};
use crate::world::NeighborhoodId;
use mandate_content::asks::{
    AskDef, AskFrom, BillsDef, NeedDef, NeedVote, OnBehalf, ROLE_ASKER, ROLE_ASKER_HOOD,
    StatementDef, interpolate,
};
use mandate_content::{Cast, RoleDef};
use mandate_core::{SimDate, Stream};

/// One castable offer.
#[derive(Clone, Debug)]
pub struct Candidate {
    pub template: String,
    pub asker: Asker,
    pub cast: Vec<CastEntry>,
    pub needs: Vec<Need>,
    pub weight: f64,
    pub due: SimDate,
    pub history: History,
}

/// The week's chance of an offer (pure; the curves of ENGAGEMENT §3.2).
pub fn chance(a: &AskState, r: &Reads<'_>) -> f64 {
    let Some(pl) = r.player else {
        return 0.0;
    };
    let p = &r.content.ask_params;
    let open = a.open_asks();
    if open >= p.max_open as usize {
        return 0.0;
    }
    let campaign = characters::is_candidate(r.politics(), pl.politician);
    let base = if campaign {
        p.new_per_month_campaign
    } else {
        p.new_per_month
    };
    let quiet = match a.last_offer {
        Some(d) => d.days_until(r.date),
        None => super::career_days(r),
    };
    (base * 12.0 / 52.0
        * interpolate(&p.intent_by_open, open as f64)
        * interpolate(&p.intent_by_quiet, quiet as f64))
    .clamp(0.0, 1.0)
}

/// Asks domain, Director phase, weekly: maybe one offer at the sliding rate.
pub fn weekly(a: &mut AskState, r: &Reads<'_>, out: &mut Outs<'_>) {
    let Some(pl) = r.player else {
        return;
    };
    if !pl.status.active() || r.content.asks.is_empty() {
        return;
    }
    let p = chance(a, r);
    if p <= 0.0 {
        return;
    }
    let draw = mandate_core::rng::keyed(
        r.meta.seed,
        Stream::Asks,
        &[super::KEY_OFFER, r.date.0 as u64],
    )
    .next_f64();
    if draw >= p {
        return;
    }
    let cands = candidates(a, r, &[AskFrom::Week, AskFrom::TalkedAbout]);
    let weights: Vec<f64> = cands.iter().map(|c| c.weight).collect();
    let pick = mandate_core::rng::keyed(
        r.meta.seed,
        Stream::Asks,
        &[super::KEY_PICK, r.date.0 as u64],
    )
    .weighted_index(&weights);
    if let Some(i) = pick {
        offer(a, r, out, cands[i].clone());
    }
}

/// Doorstep and conversation asks from the day's new deeds.
pub(crate) fn from_deeds(
    a: &mut AskState,
    r: &Reads<'_>,
    out: &mut Outs<'_>,
    deeds: &[(u32, super::DeedRecord)],
) {
    let Some(pl) = r.player else {
        return;
    };
    if !pl.status.active() {
        return;
    }
    let p = &r.content.ask_params;
    for (event, d) in deeds {
        if a.open_asks() >= p.max_open as usize {
            return;
        }
        let (from, person, hood) = match &d.deed {
            Deed::Canvass { hood } => (AskFrom::Doorstep, None, Some(*hood)),
            Deed::Talk { person, .. } => (AskFrom::Talk, Some(*person), None),
            _ => continue,
        };
        if doorstep_roll(r.meta.seed, *event) >= p.doorstep_chance {
            continue;
        }
        let pol = r.politics();
        let v = View::new(r, pol);
        let mut found: Option<Candidate> = None;
        for t in r.content.asks.iter().filter(|t| t.comes_from(from)) {
            if !eligible(a, r, t) {
                continue;
            }
            let people: Vec<CharacterId> = match (person, hood) {
                (Some(x), _) => vec![x],
                // A household knocked on today first (DEC-114 §3.4: its
                // speaker was promoted as the street finished), then the
                // neighbourhood's residents.
                (None, Some(h)) => {
                    let mut v2 = speakers(r, h, d.on);
                    for x in residents(&v, &t.asker, h) {
                        if !v2.contains(&x) {
                            v2.push(x);
                        }
                    }
                    v2
                }
                _ => Vec::new(),
            };
            for x in people {
                let preset = vec![CastEntry {
                    role: ROLE_ASKER.into(),
                    actor: Actor::Person { id: x },
                }];
                if let Some(c) = cast_one(a, r, &v, t, None, &preset) {
                    found = Some(c);
                    break;
                }
            }
            if found.is_some() {
                break;
            }
        }
        if let Some(c) = found {
            offer(a, r, out, c);
        }
    }
}

/// The doorstep roll of a deed (one a canvass or a talk, keyed by its
/// event): below `doorstep_chance`, the deed brings an ask.
pub fn doorstep_roll(seed: u64, event: u32) -> f64 {
    mandate_core::rng::keyed(seed, Stream::Asks, &[super::KEY_DOORSTEP, event as u64]).next_f64()
}

/// Whether a canvass's deed will bring a doorstep ask, as [`from_deeds`]
/// decides it (an active player with room for an ask, the roll): the
/// canvass then promotes the speaker of a household it knocked on
/// (`actions::ground`), whom `from_deeds` casts first.
pub fn doorstep_due(state: &crate::State, content: &mandate_content::ContentBundle, event: u32) -> bool {
    let p = &content.ask_params;
    state.player.as_ref().is_some_and(|pl| pl.status.active())
        && !content.asks.is_empty()
        && state.asks.open_asks() < p.max_open as usize
        && doorstep_roll(state.meta.seed, event) < p.doorstep_chance
}

/// The speakers of the households knocked on in `hood` on the canvass's
/// day (promoted to Named residents), in the order of their records.
fn speakers(r: &Reads<'_>, hood: NeighborhoodId, on: SimDate) -> Vec<CharacterId> {
    r.politics()
        .characters
        .households
        .records
        .iter()
        .filter(|x| x.hood == hood && x.on == on)
        .filter_map(|x| x.speaker)
        .collect()
}

/// Named people (not politicians) who could be the asker and live in `hood`.
fn residents(v: &View<'_>, asker: &Cast, hood: NeighborhoodId) -> Vec<CharacterId> {
    cast::candidates(v, asker, &[], None)
        .into_iter()
        .filter_map(|a| match a {
            Actor::Person { id } => Some(id),
            _ => None,
        })
        .filter(|id| {
            v.pol
                .characters
                .get(*id)
                .and_then(|c| c.named.as_deref())
                .is_some_and(|n| n.home == hood.0)
        })
        .collect()
}

/// Not cooling down, old enough in the career.
fn eligible(a: &AskState, r: &Reads<'_>, t: &AskDef) -> bool {
    super::career_days(r) >= t.min_career_days
        && a.cooldowns.get(&t.id).is_none_or(|d| *d <= r.date)
}

/// Every castable offer this week from these sources.
pub fn candidates(a: &AskState, r: &Reads<'_>, from: &[AskFrom]) -> Vec<Candidate> {
    let pol = r.politics();
    let v = View::new(r, pol);
    let mut out = Vec::new();
    let talked = talked_about(a, r);
    for t in &r.content.asks {
        if !eligible(a, r, t) {
            continue;
        }
        let mut presets: Vec<(Option<CharacterId>, Vec<CastEntry>)> = Vec::new();
        if from.contains(&AskFrom::Week) && t.comes_from(AskFrom::Week) {
            if matches!(t.asker, Cast::GoalHolder) {
                for g in &pol.characters.goals {
                    let names = r
                        .content
                        .goals
                        .iter()
                        .any(|d| d.id == g.goal && d.ask.as_deref() == Some(t.id.as_str()));
                    if names {
                        presets.push((Some(g.holder), Vec::new()));
                    }
                }
            } else {
                presets.push((None, Vec::new()));
            }
        }
        if from.contains(&AskFrom::TalkedAbout) && t.comes_from(AskFrom::TalkedAbout) {
            for p in &talked {
                presets.push((
                    None,
                    vec![CastEntry {
                        role: ROLE_ASKER.into(),
                        actor: Actor::Person { id: *p },
                    }],
                ));
            }
        }
        for (holder, preset) in presets {
            if let Some(c) = cast_one(a, r, &v, t, holder, &preset) {
                out.push(c);
            }
        }
    }
    out
}

/// People the player asked around about lately (deeds), oldest first.
fn talked_about(a: &AskState, r: &Reads<'_>) -> Vec<CharacterId> {
    let from = SimDate(
        r.date
            .0
            .saturating_sub(r.content.ask_params.talked_about_days),
    );
    let mut out = Vec::new();
    for d in a.deeds_since(from) {
        if let Deed::Talk { about: Some(p), .. } = d.deed
            && !out.contains(&p)
        {
            out.push(p);
        }
    }
    out
}

/// The asker's home neighbourhood: a Named person's home, a politician's.
pub fn home_hood(v: &View<'_>, id: CharacterId) -> Option<u32> {
    let c = v.pol.characters.get(id)?;
    if let Some(n) = c.named.as_deref() {
        return Some(n.home);
    }
    c.politician.and_then(|p| v.pol.get(p)).map(|p| p.home)
}

/// The cast with the asker's neighbourhood added.
fn with_hood(v: &View<'_>, cast: &[CastEntry]) -> Vec<CastEntry> {
    let mut out = cast.to_vec();
    if let Some(p) = person_of(cast, ROLE_ASKER)
        && let Some(h) = home_hood(v, p)
        && !out.iter().any(|c| c.role == ROLE_ASKER_HOOD)
    {
        out.push(CastEntry {
            role: ROLE_ASKER_HOOD.into(),
            actor: Actor::Neighborhood { id: h },
        });
    }
    out
}

/// Who the ask is from.
fn asker_of(r: &Reads<'_>, t: &AskDef, person: CharacterId) -> Asker {
    if t.on_behalf == OnBehalf::Party
        && let Some(party) = r
            .player
            .and_then(|pl| r.politics().get(pl.politician))
            .and_then(|p| p.party)
    {
        return Asker::Party { id: party };
    }
    Asker::Person { id: person }
}

/// Is this asker free to ask (no open ask of theirs, not cooling down)?
fn asker_free(a: &AskState, asker: Asker, date: SimDate) -> bool {
    !a.open().any(|x| x.asker == asker)
        && !a
            .asker_cooldowns
            .iter()
            .any(|(x, until)| *x == asker && *until > date)
}

/// Cast one template (with a preset asker, or a goal's holder) so that its
/// asker is free, its triggers hold and its needs can be met.
fn cast_one(
    a: &AskState,
    r: &Reads<'_>,
    v: &View<'_>,
    t: &AskDef,
    holder: Option<CharacterId>,
    preset: &[CastEntry],
) -> Option<Candidate> {
    let me = v.me();
    let taken = a.faults_asked();
    let asker_role = RoleDef {
        id: ROLE_ASKER.into(),
        cast: t.asker.clone(),
    };
    let roles: Vec<RoleDef> = std::iter::once(asker_role.clone())
        .chain(t.roles.iter().cloned())
        .collect();
    let ok = |c: &[CastEntry]| -> bool {
        let Some(person) = person_of(c, ROLE_ASKER) else {
            return false;
        };
        if Some(person) == me || !asker_free(a, asker_of(r, t, person), r.date) {
            return false;
        }
        // A preset asker (a doorstep, a talk) must be one the cast allows.
        if preset.iter().any(|p| p.role == ROLE_ASKER)
            && !cast::candidates(v, &t.asker, &[], holder).contains(&Actor::Person { id: person })
        {
            return false;
        }
        let full = with_hood(v, c);
        t.triggers.iter().all(|x| cast::holds(v, x, &full))
            && needs_for(r, v, &t.needs, &full, None, &taken).is_some()
            && !done_lately(a, r, t, &full)
    };
    let cast = cast::cast_search(v, &roles, preset, holder, 4, &ok)?;
    let full = with_hood(v, &cast);
    let person = person_of(&full, ROLE_ASKER)?;
    let needs = needs_for(r, v, &t.needs, &full, None, &taken)?;
    let due = due_of(r, t, &needs)?;
    let asker = asker_of(r, t, person);
    let history = a.history_of(asker);
    let p = &r.content.ask_params;
    let opinion = me
        .map(|m| characters::opinion(v.pol, person, m) as f64)
        .unwrap_or(0.0);
    let callback = match history {
        History::Kept => p.asker_kept_mult,
        History::Broken => p.asker_broken_mult,
        History::None => 1.0,
    };
    let weight = t.weight * (0.5 + (opinion + 100.0) / 200.0).clamp(0.5, 1.5) * callback;
    Some(Candidate {
        template: t.id.clone(),
        asker,
        cast: full,
        needs,
        weight,
        due,
        history,
    })
}

/// The player did what the first need asks lately (`fresh_days`).
fn done_lately(a: &AskState, r: &Reads<'_>, t: &AskDef, cast: &[CastEntry]) -> bool {
    let Some(days) = t.fresh_days else {
        return false;
    };
    let from = SimDate(r.date.0.saturating_sub(days));
    let hood_of = |role: &str| match actor_of(cast, role) {
        Some(Actor::Neighborhood { id }) => Some(NeighborhoodId(id)),
        _ => None,
    };
    let Some(first) = t.needs.first() else {
        return false;
    };
    a.deeds_since(from).any(|d| match (first, &d.deed) {
        (
            NeedDef::TownHall {
                role: Some(role), ..
            },
            Deed::TownHall { hood },
        ) => hood_of(role) == Some(*hood),
        (NeedDef::Canvass { role, .. }, Deed::Canvass { hood }) => hood_of(role) == Some(*hood),
        (NeedDef::Talk { role, .. }, Deed::Talk { person, .. }) => {
            person_of(cast, role) == Some(*person)
        }
        (NeedDef::Statement { .. }, Deed::Statement { .. }) => true,
        _ => false,
    })
}

/// The deadline: drawn in the template's range (its lower half in a term
/// after a re-election, ENGAGEMENT §4.7), or the vote on the bill.
fn due_of(r: &Reads<'_>, t: &AskDef, needs: &[Need]) -> Option<SimDate> {
    match t.deadline_days {
        Some(range) => {
            let [lo, hi] = deadline_range(range, second_term(r));
            let key = mandate_core::hash::derive_u64(&t.id, &[]);
            let d = mandate_core::rng::keyed(
                r.meta.seed,
                Stream::Asks,
                &[super::KEY_DEADLINE, r.date.0 as u64, key],
            )
            .range_i64(lo as i64, hi as i64);
            Some(r.date.add_days(d as u32))
        }
        None => {
            let policy = needs.iter().find_map(|n| match n {
                Need::Vote { policy, .. } => Some(*policy),
                _ => None,
            })?;
            let on = r.policy().instances.get(policy as usize)?.next_stage_on?;
            // Too close to the vote to ask in time.
            (on >= r.date.add_days(7)).then_some(on)
        }
    }
}

/// The days an ask's deadline is drawn from: its range, or its lower half
/// in a term after a re-election (ENGAGEMENT §4.7: expectations rise for
/// incumbents).
pub fn deadline_range([lo, hi]: [u32; 2], second_term: bool) -> [u32; 2] {
    if second_term {
        [lo, lo + (hi - lo) / 2]
    } else {
        [lo, hi]
    }
}

/// A term after a re-election: the player has served more than one term.
pub fn second_term(r: &Reads<'_>) -> bool {
    r.player
        .and_then(|pl| r.politics().get(pl.politician))
        .is_some_and(|p| p.in_office() && p.terms_in_office >= 2)
}

fn bills(r: &Reads<'_>, b: &BillsDef, answer: Option<&str>) -> Option<Vec<String>> {
    let out = match b {
        BillsDef::Templates { templates } => templates.clone(),
        BillsDef::CrisisAnswer => {
            let kind = live_crisis(r)?;
            super::resolve::crisis_answers(r.content, &kind)
        }
        BillsDef::ConcernAnswer { concern } => super::resolve::concern_answers(r.content, concern),
        BillsDef::Chosen => vec![answer?.to_string()],
    };
    (!out.is_empty()).then_some(out)
}

/// The most severe live crisis of the player's home town.
pub fn live_crisis(r: &Reads<'_>) -> Option<String> {
    let pl = r.player?;
    r.politics()
        .characters
        .towns
        .get(pl.home().index())
        .and_then(|t| t.crisis())
        .filter(|c| c.severity > 0.0)
        .map(|c| c.kind.clone())
}

/// Resolve a template's needs with a cast (and a promise's answer and
/// race neighbourhoods); `None` if one cannot be met now.
pub(crate) fn needs(
    r: &Reads<'_>,
    v: &View<'_>,
    defs: &[NeedDef],
    cast: &[CastEntry],
    promise: Option<(&[u32], Option<&str>)>,
) -> Option<Vec<Need>> {
    needs_for(r, v, defs, cast, promise, &[])
}

/// [`needs`], with the faults open asks are about already (`taken`).
fn needs_for(
    r: &Reads<'_>,
    v: &View<'_>,
    defs: &[NeedDef],
    cast: &[CastEntry],
    promise: Option<(&[u32], Option<&str>)>,
    taken: &[crate::upkeep::FaultId],
) -> Option<Vec<Need>> {
    let person = |role: &str| person_of(cast, role);
    // A neighbourhood role, or a person's (where they live).
    let hood = |role: &str| match actor_of(cast, role) {
        Some(Actor::Neighborhood { id }) => Some(NeighborhoodId(id)),
        Some(Actor::Person { id }) => home_hood(v, id).map(NeighborhoodId),
        _ => None,
    };
    let bill = |role: &str| match actor_of(cast, role) {
        Some(Actor::Policy { id }) => Some(id),
        _ => None,
    };
    let answer = promise.and_then(|p| p.1);
    let me = r.player.map(|p| p.politician);
    let mut out = Vec::new();
    for d in defs {
        out.push(match d {
            NeedDef::Vote { role, vote } => {
                let policy = bill(role)?;
                let p = r.policy().instances.get(policy as usize)?;
                if p.status != PolicyStatus::UnderConsideration || !v.can_vote(policy) {
                    return None;
                }
                if me.is_some_and(|m| r.politics().declared_votes.contains_key(&(m.0, policy))) {
                    return None;
                }
                let vote = match vote {
                    NeedVote::Yes => crate::politics::Vote::Yes,
                    NeedVote::No => crate::politics::Vote::No,
                    NeedVote::WithParty => match v.party_line(policy) {
                        1 => crate::politics::Vote::Yes,
                        -1 => crate::politics::Vote::No,
                        _ => return None,
                    },
                };
                Need::Vote { policy, vote }
            }
            NeedDef::AgainstAll { tag } => Need::VoteAgainstAll { tag: tag.clone() },
            NeedDef::Statement { about } => Need::Statement {
                about: match about {
                    StatementDef::Crisis => StatementNeed::Crisis {
                        kind: live_crisis(r)?,
                    },
                    StatementDef::Concern { concern } => StatementNeed::Concern {
                        concern: concern.clone(),
                    },
                    StatementDef::Bill { role } => StatementNeed::Bill {
                        policy: bill(role)?,
                    },
                    StatementDef::Any => StatementNeed::Any,
                },
            },
            NeedDef::TownHall { role, constituency } => {
                let hoods: Vec<NeighborhoodId> = if *constituency {
                    match promise {
                        Some((h, _)) => h.iter().map(|x| NeighborhoodId(*x)).collect(),
                        None => {
                            let c = super::effects::player_constituency(r)?;
                            r.world.constituencies.get(c.index())?.neighborhoods.clone()
                        }
                    }
                } else {
                    vec![hood(role.as_deref()?)?]
                };
                if hoods.is_empty() {
                    return None;
                }
                Need::TownHall { hoods }
            }
            NeedDef::Canvass { role, or_town_hall } => Need::Canvass {
                hood: hood(role)?,
                or_town_hall: *or_town_hall,
            },
            NeedDef::Visit { kinds, role } => {
                let pl = r.player?;
                let near = role.as_deref().and_then(hood);
                let mut places: Vec<&crate::world::Place> = r
                    .world
                    .places
                    .iter()
                    .filter(|p| p.municipality == pl.home() && p.is_open())
                    .filter(|p| kinds.iter().any(|k| k == p.kind.key()))
                    .collect();
                places.sort_by_key(|p| (Some(p.neighborhood) != near, p.id));
                Need::Visit {
                    place: places.first()?.id,
                }
            }
            NeedDef::Talk { role, .. } => Need::Talk {
                person: person(role)?,
            },
            NeedDef::Introduce { bills: b } => Need::Introduce {
                templates: bills(r, b, answer)?,
            },
            NeedDef::Enacted {
                bills: b,
                sponsor_player,
            } => Need::Enacted {
                templates: bills(r, b, answer)?,
                sponsor_player: *sponsor_player,
            },
            NeedDef::Casework => Need::Casework,
            NeedDef::Fixed { role } => Need::Fixed {
                fault: fault_near(r, hood(role)?, taken)?,
            },
            NeedDef::NoSurfacedHiddenFact => Need::NoSurfacedHiddenFact,
            NeedDef::Quiet { role } => Need::Quiet {
                person: person(role)?,
            },
        });
    }
    Some(out)
}

/// The fault an ask is cast on near someone's door (DEC-114): the oldest
/// fault waiting in the queue on the neighbourhood's streets that no open
/// ask is about already (`taken`).
fn fault_near(
    r: &Reads<'_>,
    hood: NeighborhoodId,
    taken: &[crate::upkeep::FaultId],
) -> Option<crate::upkeep::FaultId> {
    r.upkeep()?
        .in_hood(hood)
        .filter(|f| f.open() && !taken.contains(&f.id))
        .min_by_key(|f| (f.opened, f.id))
        .map(|f| f.id)
}

/// Asks domain, Director phase, daily: asks other domains asked for. An
/// arc's ask beat (DEC-109, `Effect::OfferAsk`) is offered from the run's
/// cast person with the run's cast, if it can be kept and they have no
/// ask open; the matter is new, so their cooldown does not hold it back.
/// A request that cannot be met is dropped (the run asks again).
pub fn requests(a: &mut AskState, r: &Reads<'_>, out: &mut Outs<'_>) {
    use crate::scheduler::Effect;
    let reqs = out.take(|e| matches!(e, Effect::OfferAsk { .. }));
    if reqs.is_empty() || !r.player.is_some_and(|pl| pl.status.active()) {
        return;
    }
    let pol = r.politics();
    let v = View::new(r, pol);
    for q in reqs {
        let Effect::OfferAsk {
            template,
            asker,
            cast,
            arc,
        } = q.effect
        else {
            continue;
        };
        let Some(t) = r.content.ask(&template) else {
            continue;
        };
        if a.asks.iter().any(|x| x.arc.as_ref() == Some(&arc))
            || a.open_asks() >= r.content.ask_params.max_open as usize
        {
            continue;
        }
        let mut preset = vec![CastEntry {
            role: ROLE_ASKER.into(),
            actor: Actor::Person { id: asker },
        }];
        for role in &t.roles {
            if let Some(e) = cast.iter().find(|e| e.role == role.id) {
                preset.push(e.clone());
            }
        }
        let who = asker_of(r, t, asker);
        a.asker_cooldowns.retain(|(x, _)| *x != who);
        let Some(c) = cast_one(a, r, &v, t, None, &preset) else {
            continue;
        };
        let before = a.asks.len();
        offer_caused(a, r, out, c, Some(q.cause));
        if a.asks.len() > before
            && let Some(x) = a.asks.last_mut()
        {
            x.arc = Some(arc);
        }
    }
}

/// Make an offer.
fn offer(a: &mut AskState, r: &Reads<'_>, out: &mut Outs<'_>, c: Candidate) {
    offer_caused(a, r, out, c, None);
}

/// Make an offer; `prompted`: what led someone to ask, beyond the
/// template's trigger (an arc's stage).
fn offer_caused(
    a: &mut AskState,
    r: &Reads<'_>,
    out: &mut Outs<'_>,
    c: Candidate,
    prompted: Option<crate::spine::Cause>,
) {
    let Some(t) = r.content.ask(&c.template) else {
        return;
    };
    let p = &r.content.ask_params;
    let id: AskId = a.ids.alloc();
    let respond_by = r.date.add_days(p.respond_days).min(c.due);
    a.asks.push(Ask {
        id,
        template: c.template.clone(),
        promise: false,
        asker: c.asker,
        cast: c.cast,
        needs: c.needs,
        step: 0,
        done: Vec::new(),
        public: t.public,
        big: t.big,
        opened: r.date,
        respond_by,
        due: c.due,
        status: AskStatus::Offered,
        history: c.history,
        race: None,
        answer: None,
        tries: 0,
        arc: None,
    });
    a.last_offer = Some(r.date);
    a.cooldowns
        .insert(c.template.clone(), r.date.add_days(t.cooldown_days));
    a.asker_cooldowns.retain(|(_, until)| *until > r.date);
    a.asker_cooldowns
        .push((c.asker, r.date.add_days(p.per_asker_cooldown_days)));
    a.tally.offered += 1;
    // Someone chose to ask (a person), by the template's trigger.
    let mut cause =
        crate::spine::Causes::from(crate::spine::Cause::Process(crate::spine::Process::Asks));
    if let Asker::Person { id } = c.asker {
        cause = cause.and(crate::spine::Cause::Actor(id));
    }
    if let Some(p) = prompted {
        cause = cause.and(p);
    }
    out.events.record(
        r.date,
        crate::events::EventKind::AskOffered {
            ask: id.0,
            template: c.template,
            big: t.big,
        },
        cause,
    );
}
