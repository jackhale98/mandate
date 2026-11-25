//! The other side (EXCHANGE_SCENES §4): where a prompt comes from, the
//! deck, and the round's draw.
//!
//! **Prompts are true.** Each source reads one owner's state and gives
//! zero or more attacks: a true thing about the player, sized by a real
//! quantity, anchored on the event the spine keeps for it. An attack whose
//! event the log has forgotten is gone. A clean record leaves the personal
//! sources empty, and the round falls through to a question.
//!
//! **Nothing here reads what the player knows.** The world does not wait
//! for the player (DEC-091): a hidden fact needs the *speaker* to be one
//! of its knowers, never the player's ledger.
//!
//! **Every draw is keyed** (`Stream::Misc`, the arcs' own tags), on the
//! run, the beat and the round: never on a list position, so the view may
//! ask as often as it likes and no stream advances.

use super::audience;
use super::state::{AnswerHeld, Attack, Group, GroupLean, PromptNow, Subject, What};
use crate::arcs::{ArcRun, BeatRef};
use crate::characters::{self, CharacterId, MemoryKind};
use crate::events::EventLog;
use crate::narrative::cast::View;
use crate::policy::{EndReason, PolicyInstance, PolicyStatus};
use crate::spine::Thread;
use mandate_content::condition::Worry;
use mandate_content::exchange::{ArcExchangeDef, ExchangeParams, MoveKind, SourceKind, Speaker};
use mandate_content::{ContentBundle, PolicyTemplate};
use mandate_core::Stream;

/// Keyed-draw tags (`Stream::Misc`; never reused), beside the arcs'
/// `KEY_CLOCK` and `KEY_MOVE`.
pub(crate) const KEY_PROMPT: u64 = 0xA2C5_0003;
pub(crate) const KEY_PROMPT_HOLDS: u64 = 0xA2C5_0004;
pub(crate) const KEY_VARIANT: u64 = 0xA2C5_0005;
pub(crate) const KEY_LINE_HOLDS: u64 = 0xA2C5_0006;
pub(crate) const KEY_SAY: u64 = 0xA2C5_0007;

/// A keyed draw of a scene: the run, the beat, the round, and whatever
/// else names the thing drawn. No stream advances.
pub(crate) fn keyed(
    seed: u64,
    tag: u64,
    run: &ArcRun,
    beat: &BeatRef,
    round: u8,
    extra: u64,
) -> mandate_core::SimRng {
    mandate_core::rng::keyed(
        seed,
        Stream::Misc,
        &[
            tag,
            u64::from(run.id.0),
            mandate_core::hash::derive_u64(&beat.key(), &[]),
            u64::from(round),
            extra,
        ],
    )
}

/// The worry a policy template is about: the concern whose attitudes it
/// lines up with most (by either sign: a rates rise is about taxes), if
/// at least `worry_match`. Data, not a table of names.
pub fn worry_of_template(
    content: &ContentBundle,
    p: &ExchangeParams,
    t: &PolicyTemplate,
) -> Option<String> {
    let norm = |v: &[f64]| mandate_core::math::sqrt(v.iter().map(|x| x * x).sum::<f64>());
    let nt = norm(&t.signature_weights);
    if nt <= 0.0 {
        return None;
    }
    let mut best: Option<(f64, &str)> = None;
    for c in &content.concerns {
        let nc = norm(&c.attitude_weights);
        if nc <= 0.0 {
            continue;
        }
        let dot: f64 = c
            .attitude_weights
            .iter()
            .zip(&t.signature_weights)
            .map(|(a, b)| a * b)
            .sum();
        let cos = (dot / (nc * nt)).abs();
        if cos >= p.worry_match && best.is_none_or(|b| cos > b.0) {
            best = Some((cos, c.id.as_str()));
        }
    }
    best.map(|b| b.1.to_string())
}

/// The worry a bill or a law is about.
pub fn worry_of(v: &View<'_>, p: &ExchangeParams, inst: &PolicyInstance) -> Option<String> {
    let t = v.content().policies.get(&inst.template)?;
    worry_of_template(v.content(), p, t)
}

/// What a law has delivered against what a worry needs (§3.1, `record`):
/// its ledger's readings for the worry ÷ the worry's full amount for a
/// place that size, × its progress. The worry it answers best, and how
/// far (0..1). `None` when its ledger shows nothing on any worry.
pub fn delivery(v: &View<'_>, p: &ExchangeParams, inst: &PolicyInstance) -> Option<(String, f64)> {
    if !inst.active() {
        return None;
    }
    let world = v.r.world;
    let residents: f64 = inst
        .at()
        .hoods(world)
        .iter()
        .map(|h| v.r.population().hood_total(*h) as f64)
        .sum();
    let mut best: Option<(String, f64)> = None;
    for w in &p.worries {
        let mut share = 0.0;
        for r in &w.readings {
            let read: f64 = if r.key.ends_with('_') {
                inst.ledger
                    .iter()
                    .filter(|(k, _)| k.starts_with(&r.key))
                    .map(|(_, x)| *x)
                    .sum()
            } else {
                inst.ledger.get(&r.key).copied().unwrap_or(0.0)
            };
            let full = if r.per_thousand {
                r.full * (residents / 1000.0).max(1.0)
            } else {
                r.full
            };
            if full > 0.0 {
                share += read / full;
            }
        }
        let s = share.clamp(0.0, 1.0) * f64::from(inst.progress).clamp(0.0, 1.0);
        if s > 0.0 && best.as_ref().is_none_or(|b| s > b.1) {
            best = Some((w.concern.clone(), s));
        }
    }
    best
}

/// Who is across from the player, as the deck reads them.
#[derive(Clone, Debug, PartialEq)]
pub struct Opponent {
    pub id: Option<CharacterId>,
    /// The mean of their oratory and intrigue.
    pub skill: f64,
    pub oratory: f64,
    pub boldness: f64,
    pub integrity: f64,
    pub warmth: f64,
    /// A grudge: a trait in `deck.grudge_traits`, or a memory of the
    /// player against them.
    pub grudge: bool,
}

impl Opponent {
    /// Skilled enough to read how residents feel and to pick their best.
    pub fn sharp(&self, p: &ExchangeParams) -> bool {
        self.skill >= p.deck.sharp
    }

    /// How they read a crowd: polls for the weak, truth for the sharp
    /// (DEC-030: better information is the only advantage).
    pub fn reads(&self, p: &ExchangeParams) -> Worry {
        if self.sharp(p) {
            Worry::Felt
        } else {
            Worry::Measured
        }
    }
}

pub fn opponent(v: &View<'_>, p: &ExchangeParams, run: &ArcRun, def: &ArcExchangeDef) -> Opponent {
    let id = run.person(&def.sides.other);
    let profile = id.and_then(|c| characters::profile(v.pol, v.r.world, c));
    let Some(x) = profile else {
        // The floor (a meeting's organiser may be uncast): no character.
        return Opponent {
            id,
            skill: 0.5,
            oratory: 0.5,
            boldness: 0.5,
            integrity: 0.5,
            warmth: 0.5,
            grudge: false,
        };
    };
    let against = v.me().is_some_and(|me| {
        characters::salient_memories(v.pol, x.id, Some(me), v.date(), 3)
            .iter()
            .map(|m| m.sentiment)
            .sum::<f32>()
            < 0.0
    });
    Opponent {
        id,
        skill: f64::from(x.competencies.oratory + x.competencies.intrigue) / 2.0,
        oratory: f64::from(x.competencies.oratory),
        boldness: f64::from(x.personality.boldness),
        integrity: f64::from(x.personality.integrity),
        warmth: f64::from(x.personality.warmth),
        grudge: against || x.traits.iter().any(|t| p.deck.grudge_traits.contains(t)),
    }
}

/// An event the log still keeps.
fn kept(log: &EventLog, id: Option<crate::events::EventId>) -> Option<crate::events::EventId> {
    id.filter(|e| log.get(*e).is_some())
}

/// The laws and bills the player sponsored.
fn sponsored<'a>(v: &'a View<'_>) -> impl Iterator<Item = &'a PolicyInstance> {
    let me = v.player().map(|p| p.politician);
    v.r.policy()
        .instances
        .iter()
        .filter(move |i| me.is_some() && i.sponsor == me)
}

/// The votes someone has declared: (policy, +1 or −1).
pub fn declared(v: &View<'_>, who: crate::politics::PoliticianId) -> Vec<(u32, i8)> {
    v.pol
        .declared_votes
        .range((who.0, 0)..=(who.0, u32::MAX))
        .filter(|(_, vote)| **vote != 0)
        .map(|((_, policy), vote)| (*policy, *vote))
        .collect()
}

/// How far the crowd group that leans hardest against a declared vote
/// leans, with that group: `None` when no group leans against it. The
/// lean is today's (voters judge with today's attitudes).
pub fn against_lean(
    v: &View<'_>,
    groups: &[GroupLean],
    policy: u32,
    vote: i8,
) -> Option<(usize, f64)> {
    let inst = v.r.policy().instances.get(policy as usize)?;
    let t = v.content().policies.get(&inst.template)?;
    let pop = v.r.population();
    let mut best: Option<(usize, f64)> = None;
    for (gi, g) in groups.iter().enumerate() {
        if !g.group.crowd() {
            continue;
        }
        let hoods: Vec<usize> = audience::hoods_of(v.r.world, &g.group)
            .into_iter()
            .filter(|h| *h < pop.hoods.len())
            .collect();
        if hoods.is_empty() {
            continue;
        }
        let against = -f64::from(vote) * crate::policy::public_support(t, pop, &hoods);
        if against > 0.0 && best.is_none_or(|b| against > b.1) {
            best = Some((gi, against));
        }
    }
    best
}

fn gravity(p: &ExchangeParams, g: mandate_content::Gravity) -> f64 {
    match g {
        mandate_content::Gravity::Ethics => p.size.gravity_ethics,
        mandate_content::Gravity::Misconduct => p.size.gravity_misconduct,
        mandate_content::Gravity::Crime => p.size.gravity_crime,
    }
}

/// The chance a claim held on this much evidence stands up: the player's
/// own rule (`hold`), read by thirds.
pub fn holds_on(p: &ExchangeParams, evidence: f64) -> f64 {
    if evidence >= 2.0 / 3.0 {
        p.hold.detailed
    } else if evidence >= 1.0 / 3.0 {
        p.hold.known
    } else {
        p.hold.heard
    }
}

/// Everything one source gives about the player now.
#[allow(clippy::too_many_lines)]
pub fn attacks(
    v: &View<'_>,
    log: &EventLog,
    p: &ExchangeParams,
    run: &ArcRun,
    groups: &[GroupLean],
    opp: &Opponent,
    source: SourceKind,
) -> Vec<Attack> {
    let Some(pl) = v.player() else {
        return Vec::new();
    };
    let me = pl.politician;
    let content = v.content();
    let today = v.date();
    let flat = |source, what, size: f64, anchor| Attack {
        source,
        subject: Subject::of(what, None),
        size: size.clamp(0.0, 1.0) as f32,
        anchor,
        p: 1.0,
        group: None,
    };
    match source {
        SourceKind::BrokenPromise => {
            v.r.asks()
                .asks
                .iter()
                .filter(|a| a.promise || a.public)
                .filter_map(|a| {
                    let crate::asks::AskStatus::Broken { on } = a.status else {
                        return None;
                    };
                    // The town forgets it when the log does.
                    let anchor = kept(log, log.head(Thread::Ask(a.id.0)))?;
                    let days = f64::from(on.days_until(today.max(on)));
                    let recency =
                        mandate_core::math::pow(0.5, days / f64::from(p.size.recency_days.max(1)));
                    let concern = content.promise(&a.template).and_then(|t| match &t.answers {
                        mandate_content::asks::PromiseAnswers::Concern { concern } => {
                            Some(concern.clone())
                        }
                        _ => None,
                    });
                    Some(Attack {
                        subject: Subject::of(What::Promise { ask: a.id.0 }, concern),
                        ..flat(source, What::None, p.size.promise * recency, Some(anchor))
                    })
                })
                .collect()
        }
        SourceKind::VoteAgainstLean => declared(v, me)
            .into_iter()
            .filter_map(|(policy, vote)| {
                let anchor = kept(log, log.head(Thread::Bill(policy)))?;
                let (gi, against) = against_lean(v, groups, policy, vote)?;
                if against < p.size.lean_line {
                    return None;
                }
                let inst = v.r.policy().instances.get(policy as usize)?;
                Some(Attack {
                    subject: Subject::of(What::Bill { policy }, worry_of(v, p, inst)),
                    group: Some(groups[gi].group.key()),
                    ..flat(source, What::None, against, Some(anchor))
                })
            })
            .collect(),
        SourceKind::FailedLaw => sponsored(v)
            .filter_map(|i| {
                let size = match (i.status, i.end_reason) {
                    (
                        PolicyStatus::Ended,
                        Some(EndReason::Defeated | EndReason::Vetoed | EndReason::Withdrawn),
                    ) => 1.0,
                    (PolicyStatus::InEffect, _) if i.progress >= 0.5 => {
                        let d = delivery(v, p, i).map_or(0.0, |x| x.1);
                        if d >= p.size.failed_line {
                            return None;
                        }
                        1.0 - d
                    }
                    _ => return None,
                };
                let anchor = kept(log, log.head(Thread::Bill(i.id.0)))?;
                Some(Attack {
                    subject: Subject::of(What::Law { policy: i.id.0 }, worry_of(v, p, i)),
                    ..flat(source, What::None, size, Some(anchor))
                })
            })
            .collect(),
        SourceKind::ThinRecord => {
            let record = v.pol.get(me).map_or(0.0, |x| f64::from(x.record));
            if record < p.size.thin_line {
                vec![flat(source, What::Record, 1.0 - record, None)]
            } else {
                Vec::new()
            }
        }
        // What was let pass in this arc, by the simulation's own record:
        // an ask of the run left to lapse, a cast person who remembers
        // being ignored (a decision's default leaves that memory). Never
        // the decisions themselves: which were presented is the director's
        // timing, and the spotlight changes no outcome (DEC-033 B).
        SourceKind::Absent => {
            let lapsed =
                v.r.asks()
                    .asks
                    .iter()
                    .filter(|a| a.arc.as_ref().is_some_and(|x| x.run == run.id))
                    .filter(|a| {
                        matches!(
                            a.status,
                            crate::asks::AskStatus::Declined {
                                unanswered: true,
                                ..
                            }
                        )
                    })
                    .count();
            let ignored = match (opp.id, v.me()) {
                (Some(o), Some(my)) => v.pol.characters.get(o).is_some_and(|c| {
                    c.memories
                        .iter()
                        .any(|m| m.actor == my && m.kind == MemoryKind::IgnoredMe)
                }),
                _ => false,
            };
            let n = lapsed + usize::from(ignored);
            if n == 0 {
                Vec::new()
            } else {
                vec![flat(
                    source,
                    What::Arc,
                    p.size.absent * n as f64,
                    kept(log, log.head(Thread::Arc(run.id.0))),
                )]
            }
        }
        SourceKind::Scandal => {
            v.r.media()
                .running_about(me)
                .map(|s| {
                    flat(
                        source,
                        What::Scandal { id: s.id },
                        f64::from(s.heat),
                        kept(log, log.head(Thread::Scandal(s.id))),
                    )
                })
                .collect()
        }
        SourceKind::HiddenFact => {
            let Some(speaker) = opp.id else {
                return Vec::new();
            };
            let media = v.r.media();
            v.r.narrative()
                .hidden
                .iter()
                .filter(|h| h.subject == Some(me) && !media.discovered.contains(&h.id))
                .filter_map(|h| {
                    let held = media
                        .knowledge
                        .iter()
                        .filter(|k| {
                            k.hidden == h.id
                                && k.knower == crate::media::Knower::Person { id: speaker }
                        })
                        .map(|k| f64::from(k.evidence))
                        .fold(None, |a: Option<f64>, e| Some(a.map_or(e, |x| x.max(e))))?;
                    // Character (§4.2): integrity keeps a weak claim out;
                    // only the bold take the chance.
                    if held < p.deck.weak_evidence
                        && (opp.integrity >= p.deck.integrity_bar || opp.boldness < p.deck.bold_bar)
                    {
                        return None;
                    }
                    Some(Attack {
                        p: holds_on(p, held) as f32,
                        ..flat(
                            source,
                            What::Hidden { id: h.id },
                            gravity(p, h.severity) * held,
                            kept(log, log.head(Thread::Hidden(h.id))),
                        )
                    })
                })
                .collect()
        }
        SourceKind::GoverningRecord => {
            let mine = v.pol.get(me).and_then(|x| x.party);
            let governs = mine.is_some()
                && v.pol
                    .offices
                    .iter()
                    .filter(|o| o.at == run.at)
                    .filter_map(|o| o.holder)
                    .any(|h| v.pol.get(h).and_then(|x| x.party) == mine);
            if !governs {
                return Vec::new();
            }
            let pop = v.r.population();
            let mut worst: Option<(usize, f64)> = None;
            for (gi, g) in groups.iter().enumerate() {
                let (mut acc, mut n) = (0.0, 0.0);
                for h in audience::hoods_of(v.r.world, &g.group) {
                    for c in pop
                        .hoods
                        .get(h)
                        .map(|x| x.cohorts.as_slice())
                        .unwrap_or(&[])
                    {
                        acc += f64::from(c.count) * f64::from(c.gov_approval);
                        n += f64::from(c.count);
                    }
                }
                if n > 0.0 && worst.is_none_or(|w| acc / n < w.1) {
                    worst = Some((gi, acc / n));
                }
            }
            match worst {
                Some((gi, a)) if a < p.size.governing_line => vec![Attack {
                    group: Some(groups[gi].group.key()),
                    ..flat(
                        source,
                        What::Government,
                        (p.size.governing_line - a) / p.size.governing_line.max(1e-6),
                        None,
                    )
                }],
                _ => Vec::new(),
            }
        }
        SourceKind::TopWorry => vec![question(v, p, groups, &[])],
        // Not built yet (`SourceKind::built`): an arc that lists one is
        // never raised.
        _ => Vec::new(),
    }
}

/// The room's question (§4.5): the audience's strongest worry, as its
/// residents feel it, that has not been asked already in this scene (the
/// same question is not put twice while the room has another). Not an
/// attack; it always exists.
pub fn question(
    v: &View<'_>,
    p: &ExchangeParams,
    groups: &[GroupLean],
    asked: &[String],
) -> Attack {
    let mut hoods: Vec<usize> = groups
        .iter()
        .flat_map(|g| audience::hoods_of(v.r.world, &g.group))
        .collect();
    hoods.sort_unstable();
    hoods.dedup();
    let concern = (!hoods.is_empty())
        .then(|| {
            let ranking = crate::concerns::ranked(
                v.r.population(),
                v.content(),
                v.r.world,
                crate::concerns::Scope::District(&hoods),
                Worry::Felt,
            );
            let ids: Vec<String> = crate::concerns::top(&ranking, usize::MAX)
                .filter_map(|(i, _)| v.content().concerns.get(i).map(|c| c.id.clone()))
                .collect();
            let key = |c: &String| format!("{}:{c}", SourceKind::TopWorry.key());
            ids.iter()
                .find(|c| !asked.contains(&key(c)))
                .or(ids.first())
                .cloned()
        })
        .flatten();
    Attack {
        source: SourceKind::TopWorry,
        subject: Subject::of(What::Worry, concern),
        size: p.size.question as f32,
        anchor: None,
        p: 1.0,
        group: None,
    }
}

/// How hard an attack bites by weight, as the other side reads the room.
fn weighted_bite(
    v: &View<'_>,
    p: &ExchangeParams,
    groups: &[GroupLean],
    opp: &Opponent,
    a: &Attack,
) -> f64 {
    let by = opp.reads(p);
    let bite: f64 = groups
        .iter()
        .map(|g| {
            let fit = match &g.group {
                Group::Member { .. } | Group::Person { .. } => p.member_fit.base,
                crowd => audience::crowd_fit(v, p, crowd, a.subject.concern.as_deref(), by),
            };
            f64::from(g.weight) * fit
        })
        .sum::<f64>()
        * f64::from(a.size)
        * f64::from(a.p);
    // Character shapes what they reach for (§4.2).
    let mut w = 1.0;
    if a.source.personal() {
        w += p.deck.bold_personal * opp.boldness;
        if opp.grudge {
            w += p.deck.grudge_personal;
        }
    }
    if a.source == SourceKind::TopWorry {
        w += p.deck.warm_question * opp.warmth;
    }
    bite * w
}

/// The sources a scene lists in any round, in the order first listed.
fn listed(def: &ArcExchangeDef) -> Vec<SourceKind> {
    let mut out: Vec<SourceKind> = Vec::new();
    for r in &def.prompts {
        for s in &r.from {
            if !out.contains(s) {
                out.push(*s);
            }
        }
    }
    out
}

/// The sources a `prep` mark the run holds keeps the other side off.
pub fn barred(run: &ArcRun, def: &ArcExchangeDef) -> Vec<SourceKind> {
    def.prep
        .iter()
        .filter(|x| run.has_mark(&x.mark))
        .flat_map(|x| x.bars.iter().copied())
        .collect()
}

/// The other side's deck as the scene begins (§4.2): every attack every
/// source of the scene gives, ranked by weighted bite as they read the
/// room and shaped by character; skill sets how many they hold. The
/// question is never in it: it is always there to ask.
pub fn deck(
    v: &View<'_>,
    log: &EventLog,
    p: &ExchangeParams,
    run: &ArcRun,
    def: &ArcExchangeDef,
    groups: &[GroupLean],
    opp: &Opponent,
) -> Vec<Attack> {
    let bars = barred(run, def);
    let mut all: Vec<(f64, Attack)> = listed(def)
        .into_iter()
        .filter(|s| !s.always() && !bars.contains(s))
        .flat_map(|s| attacks(v, log, p, run, groups, opp, s))
        .filter(|a| a.size > 0.0)
        .map(|a| (weighted_bite(v, p, groups, opp, &a), a))
        .collect();
    all.sort_by(|x, y| y.0.total_cmp(&x.0).then_with(|| x.1.key().cmp(&y.1.key())));
    let n = (p.deck.base + (p.deck.per_skill * opp.skill).round()).max(1.0) as usize;
    all.into_iter().take(n).map(|x| x.1).collect()
}

/// A full term in a post: the years of the post the player is standing
/// for, read from the constitution.
fn full_term(v: &View<'_>) -> u32 {
    let post = audience::my_race(v).map(|r| r.0);
    post.and_then(|post| {
        let c = v.content().constitutions.get(&v.r.meta.constitution)?;
        crate::politics::elections::calendar_for(c, post).map(|x| x.1)
    })
    .unwrap_or(4)
}

/// The laws the player may speak for (§3.1, `record`): in force, and
/// theirs (sponsored: whole; only declared for: `voted_share`). With the
/// worry each answers best and how far.
pub fn laws(v: &View<'_>, p: &ExchangeParams) -> Vec<(u32, String, f64)> {
    let Some(pl) = v.player() else {
        return Vec::new();
    };
    let me = pl.politician;
    let votes = declared(v, me);
    v.r.policy()
        .instances
        .iter()
        .filter_map(|i| {
            let share = if i.sponsor == Some(me) {
                1.0
            } else if votes
                .iter()
                .any(|(policy, vote)| *policy == i.id.0 && *vote > 0)
            {
                p.voted_share
            } else {
                return None;
            };
            let (concern, d) = delivery(v, p, i)?;
            Some((i.id.0, concern, d * share))
        })
        .filter(|x| x.2 > 0.005)
        .collect()
}

/// The answers the other side holds to the player's lines (§4.3), and how
/// many they may give in a scene.
pub fn answers(
    v: &View<'_>,
    log: &EventLog,
    p: &ExchangeParams,
    run: &ArcRun,
    def: &ArcExchangeDef,
    groups: &[GroupLean],
    opp: &Opponent,
) -> (Vec<AnswerHeld>, u8) {
    let mut out = Vec::new();
    let Some(pl) = v.player() else {
        return (out, 0);
    };
    let held = |kind: MoveKind, what: Option<What>| AnswerHeld {
        kind,
        what,
        size: p.answer(kind) as f32,
    };
    for kind in &def.moves {
        if !kind.answerable() || p.answer(*kind) <= 0.0 {
            continue;
        }
        match kind {
            // The group a law hurt: its declared grievance.
            MoveKind::Record => {
                for (policy, _, _) in laws(v, p) {
                    let hurts = v
                        .r
                        .policy()
                        .instances
                        .get(policy as usize)
                        .and_then(|i| v.content().policies.get(&i.template))
                        .is_some_and(|t| {
                            t.side_effects.iter().any(|s| {
                                matches!(s, mandate_content::SideEffectDef::Grievance { per_unit, .. }
                                    if *per_unit > 0.0)
                            })
                        });
                    if hurts {
                        out.push(held(*kind, Some(What::Law { policy })));
                    }
                }
            }
            // "Another promise": one broken and not owned.
            MoveKind::Promise => {
                if v.r.asks().broken_public_promises() > 0 {
                    out.push(held(*kind, None));
                }
            }
            // The same fault in the player.
            MoveKind::Turn => {
                let same = [
                    SourceKind::VoteAgainstLean,
                    SourceKind::ThinRecord,
                    SourceKind::Scandal,
                ]
                .into_iter()
                .any(|s| !attacks(v, log, p, run, groups, opp, s).is_empty());
                if same {
                    out.push(held(*kind, None));
                }
            }
            // "You have had N years."
            MoveKind::Act => {
                let years = v
                    .pol
                    .get(pl.politician)
                    .map_or(0, |x| u32::from(x.years_in_office));
                if years >= full_term(v) {
                    out.push(held(*kind, None));
                }
            }
            // The backer's own standing where they have no tie.
            MoveKind::Second => {
                let backer = def.sides.second.as_deref().and_then(|r| run.person(r));
                if let Some(b) = backer {
                    let ties = audience::ties(v, p, groups, b);
                    let heaviest = groups
                        .iter()
                        .enumerate()
                        .max_by(|x, y| x.1.weight.total_cmp(&y.1.weight).then(y.0.cmp(&x.0)))
                        .map(|x| x.0);
                    if heaviest.is_some_and(|g| ties[g].value < p.fit.top) {
                        out.push(held(*kind, None));
                    }
                }
            }
            _ => {}
        }
    }
    let left = (p.deck.answers_base + (p.deck.answers_per_skill * opp.oratory).round()).max(0.0);
    (out, left.min(20.0) as u8)
}

/// The round's prompt (§4.2): the first of the round's sources that gives
/// an attack the deck holds and that has not been retired; the best of
/// them, or one of the best few for a weaker opponent, by a keyed draw.
/// A round with no source left asks the room's question.
#[allow(clippy::too_many_arguments)]
pub fn draw(
    v: &View<'_>,
    p: &ExchangeParams,
    run: &ArcRun,
    beat: &BeatRef,
    def: &ArcExchangeDef,
    groups: &[GroupLean],
    deck: &[Attack],
    retired: &[String],
    opp: &Opponent,
    round: u8,
) -> PromptNow {
    let seed = v.r.meta.seed;
    let rdef = def.round(round);
    let chair = def.sides.chair.as_deref().and_then(|r| run.person(r));
    let by = match rdef.map(|r| r.by) {
        Some(Speaker::Chair) if chair.is_some() => Speaker::Chair,
        _ => Speaker::Other,
    };
    let bars = barred(run, def);
    let mut attack: Option<Attack> = None;
    for s in rdef.map(|r| r.from.as_slice()).unwrap_or(&[]) {
        if bars.contains(s) {
            continue;
        }
        if s.always() {
            attack = Some(question(v, p, groups, retired));
            break;
        }
        let cands: Vec<&Attack> = deck
            .iter()
            .filter(|a| a.source == *s && !retired.contains(&a.key()))
            .collect();
        if cands.is_empty() {
            continue;
        }
        // A skilled opponent takes their best; a weaker one, one of their
        // best few.
        let k = if opp.sharp(p) {
            1
        } else {
            usize::from(p.deck.pick_top).min(cands.len()).max(1)
        };
        let i = if k > 1 {
            keyed(seed, KEY_PROMPT, run, beat, round, 0).range_i64(0, k as i64 - 1) as usize
        } else {
            0
        };
        attack = Some(cands[i.min(cands.len() - 1)].clone());
        break;
    }
    let attack = attack.unwrap_or_else(|| question(v, p, groups, retired));
    let stood = attack.p >= 1.0
        || keyed(seed, KEY_PROMPT_HOLDS, run, beat, round, 0).chance(f64::from(attack.p));
    let variants = i64::from(p.prompt_variants.max(1));
    let variant = keyed(seed, KEY_VARIANT, run, beat, round, 0).range_i64(1, variants) as u8;
    PromptNow {
        attack,
        by,
        variant,
        stood,
    }
}
