//! The player's lines (EXCHANGE_SCENES §3): every line the state supports
//! this round, the four or five shown, and the one locked line.
//!
//! **Lines are earned.** Each kind has a requirement in real state and a
//! strength computed from real quantities; only the safe move has neither.
//!
//! **Not a checklist** (DEC-091, binding on this file):
//! - a line that knowledge opens is a different line with its own cost
//!   and its own audience (`heard` names one group; `turn` makes an enemy
//!   and may not stand; `proof` may not stand), never a plain line made
//!   stronger for everyone: what the player knows changes no number of a
//!   line they could say anyway;
//! - second-hand knowledge gives the line with its risk shown (`hold`),
//!   not no line;
//! - the lines of the unprepared (`record`, `own`, `promise`, `offer`,
//!   `pass`) need nothing learned;
//! - the locked line names a route, one a round, and never what was
//!   missed.
//!
//! The choice of which lines to show ranks by truth (§3.3): what the
//! player knows changes what the preview can tell them, never the list
//! beyond the requirements.

use super::attacks;
use super::audience;
use super::landing::{self, Known, LineIn, RoundIn};
use super::state::{ArcExchangeState, Group, Subject, What};
use crate::arcs::ArcRun;
use crate::characters::CharacterId;
use crate::narrative::cast::View;
use crate::player::knowledge::{self, Subject as Learned};
use crate::state::State;
use crate::world::{Jurisdiction, NeighborhoodId};
use mandate_content::ContentBundle;
use mandate_content::exchange::{ArcExchangeDef, ExchangeParams, MoveKind, PrepRemoves};
use mandate_content::knowledge::Level;

/// A cost or a side effect of a line, as a key with its arguments. Always
/// exact: what an action does is never gated by what the player knows.
#[derive(Clone, Debug, PartialEq)]
pub struct Note {
    pub key: String,
    pub args: Vec<(String, String)>,
}

fn note(key: &str, args: &[(&str, String)]) -> Note {
    Note {
        key: key.to_string(),
        args: args.iter().map(|(k, v)| (k.to_string(), v.clone())).collect(),
    }
}

/// What saying a line does besides moving the room: requests to the
/// owning systems, and the run's own changes.
#[derive(Clone, Debug, PartialEq)]
pub enum Deed {
    /// A real promise in the asks system.
    Promise {
        template: String,
        answer: Option<String>,
    },
    /// A draft exists.
    Draft { template: String },
    /// The player's Standing moves.
    Standing { delta: f64 },
    /// The other side remembers being pressed.
    Pressed { person: CharacterId },
    /// A hidden fact is out: the media open it as a story.
    Surface { hidden: u32 },
    /// A slot of the player's time.
    Slot,
    /// The prompt's attack cannot be used again in this scene.
    Retire,
    /// The room's temper falls.
    Cool { by: f64 },
}

/// What the player can know of the other side's answer to a line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnswerSeen {
    /// The kind cannot be answered.
    Never,
    /// They have none.
    No,
    /// They have one.
    Yes,
    /// The player does not know them well enough to say.
    Maybe,
}

/// One line the player may say this round.
#[derive(Clone, Debug, PartialEq)]
pub struct Opt {
    /// Stable (`record:law.17`, `heard:hood.4`, `pass`): never a place in
    /// a list.
    pub id: String,
    pub kind: MoveKind,
    pub subject: Subject,
    /// The group a targeted line names.
    pub target: Option<usize>,
    pub line: LineIn,
    /// A `prep` mark added it.
    pub prep: bool,
    pub notes: Vec<Note>,
    pub deeds: Vec<Deed>,
    /// Arguments of its label and its sentence.
    pub args: Vec<(String, String)>,
    pub answer_seen: AnswerSeen,
}

/// The line the scene allows and the player cannot say, with a route.
#[derive(Clone, Debug, PartialEq)]
pub struct Locked {
    pub kind: MoveKind,
    /// `exchange-locked-<reason>`.
    pub key: String,
    pub args: Vec<(String, String)>,
}

/// The round as the formula sees it: the prompt's size and fit, the room.
pub fn round_in(
    v: &View<'_>,
    p: &ExchangeParams,
    run: &ArcRun,
    def: &ArcExchangeDef,
    xs: &ArcExchangeState,
) -> Option<RoundIn> {
    let prompt = xs.prompt.as_ref()?;
    let oratory = v
        .player()
        .and_then(|pl| v.pol.get(pl.politician))
        .map_or(0.5, |me| f64::from(me.oratory));
    Some(RoundIn {
        size: f64::from(prompt.attack.size),
        cold: cold(p, run, def, xs.round),
        voice: p.voice_base + p.voice_span * oratory,
        temper: temper(run, def),
        stood: prompt.stood,
        fit: audience::fit(
            v,
            p,
            &xs.groups,
            None,
            prompt.attack.subject.concern.as_deref(),
        ),
        lean: xs.groups.iter().map(|g| f64::from(g.lean)).collect(),
    })
}

/// `cold` in the first round unless the player knew what was coming.
pub fn cold(p: &ExchangeParams, run: &ArcRun, def: &ArcExchangeDef, round: u8) -> f64 {
    let warned = def
        .prep
        .iter()
        .any(|x| x.removes == Some(PrepRemoves::Cold) && run.has_mark(&x.mark));
    if round == 1 && !warned { p.cold } else { 1.0 }
}

/// `1 − drag × heat` where the scene has a temper (the run's own heat).
pub fn temper(run: &ArcRun, def: &ArcExchangeDef) -> f64 {
    match &def.temper {
        Some(t) => (1.0 - f64::from(t.drag) * f64::from(run.heat)).clamp(0.0, 1.0),
        None => 1.0,
    }
}

/// How well the player knows a hidden fact.
fn secret_known(v: &View<'_>, hidden: u32) -> Level {
    knowledge::level_of(v.player(), &Learned::Secret { hidden })
}

fn hold(p: &ExchangeParams, level: Level) -> f64 {
    match level {
        Level::Unknown => 0.0,
        Level::Heard => p.hold.heard,
        Level::Known => p.hold.known,
        Level::Detailed => p.hold.detailed,
    }
}

fn concern_key(content: &ContentBundle, id: &str) -> String {
    content
        .concerns
        .iter()
        .find(|c| c.id == id)
        .map(|c| c.name_key.clone())
        .unwrap_or_default()
}

/// The share of the holders of a power at a jurisdiction (the player
/// aside) who would back them: of their party, thinking well of them, or
/// owing them a favour (the reading the `body_support` condition takes).
fn backing(v: &View<'_>, at: Jurisdiction, power: mandate_content::Power) -> f64 {
    let (Some(pl), Some(me)) = (v.player(), v.me()) else {
        return 0.0;
    };
    let mine = v.pol.get(pl.politician).and_then(|x| x.party);
    let others: Vec<&crate::politics::Politician> = crate::arcs::cast::power_holders(v, at, power)
        .into_iter()
        .filter(|id| *id != pl.politician)
        .filter_map(|id| v.pol.get(id))
        .collect();
    if others.is_empty() {
        return 0.0;
    }
    let n = others
        .iter()
        .filter(|x| {
            (mine.is_some() && x.party == mine)
                || f64::from(crate::characters::opinion(v.pol, x.character, me)) >= 10.0
                || crate::characters::favours_owed(v.pol, x.character, me) > 0
        })
        .count();
    n as f64 / others.len() as f64
}

/// Every line the state supports this round (§3.3, step 1), in the
/// table's order then by id. The first is never "the right answer".
#[allow(clippy::too_many_lines)]
pub fn options(
    st: &State,
    content: &ContentBundle,
    p: &ExchangeParams,
    run: &ArcRun,
    arc: &mandate_content::arcs::ArcDef,
    def: &ArcExchangeDef,
    xs: &ArcExchangeState,
) -> Vec<Opt> {
    let Some(prompt) = xs.prompt.as_ref() else {
        return Vec::new();
    };
    let menu = if def.moves.contains(&MoveKind::Promise) {
        crate::asks::commands::promise_menu(st, content)
    } else {
        Vec::new()
    };
    crate::condition::on_state(st, content, |v| {
        let Some(pl) = v.player() else {
            return Vec::new();
        };
        let opp = attacks::opponent(v, p, run, def);
        let groups = &xs.groups;
        let n = groups.len();
        let room = vec![1.0; n];
        let used_kind =
            |k: MoveKind| xs.used.iter().any(|u| u.split(':').next() == Some(k.key()));
        let repeat = |k: MoveKind| if used_kind(k) { p.repeat } else { 1.0 };
        let other_known = opp
            .id
            .map_or(Level::Unknown, |o| audience::person_known(v, o));
        let today = v.date();
        let mut out: Vec<Opt> = Vec::new();
        let mut push = |mut o: Opt, immune: bool| {
            if xs.used.contains(&o.id) {
                return;
            }
            // What the other side will answer (§4.3), and what the player
            // can know of it (§5.7).
            let held = (o.kind.answerable() && xs.answers_left > 0 && !immune)
                .then(|| {
                    xs.answers
                        .iter()
                        .find(|a| {
                            a.kind == o.kind
                                && a.what.as_ref().is_none_or(|w| *w == o.subject.what)
                        })
                        .map(|a| f64::from(a.size))
                })
                .flatten()
                .unwrap_or(0.0);
            let seen_well = other_known >= Level::Known;
            o.answer_seen = if !o.kind.answerable() {
                AnswerSeen::Never
            } else if !seen_well {
                AnswerSeen::Maybe
            } else if held > 0.0 {
                AnswerSeen::Yes
            } else {
                AnswerSeen::No
            };
            o.line.answer = if !o.kind.answerable() || seen_well {
                Known::exact(held)
            } else {
                Known::between(held, 0.0, p.answer(o.kind))
            };
            o.line.answers = o.kind.always_answers()
                || o.subject.same(&prompt.attack.subject)
                || (o.kind == MoveKind::Proof
                    && (prompt.attack.source.fault() || prompt.attack.source.deniable()));
            out.push(o);
        };
        let blank = |id: String, kind: MoveKind, subject: Subject, strength: f64| Opt {
            id,
            kind,
            line: LineIn {
                kind,
                strength: strength.clamp(0.0, 1.0),
                p: 1.0,
                answers: false,
                answer: Known::exact(0.0),
                fit: audience::fit(v, p, groups, Some(kind), subject.concern.as_deref()),
                reach: room.clone(),
            },
            subject,
            target: None,
            prep: false,
            notes: Vec::new(),
            deeds: Vec::new(),
            args: Vec::new(),
            answer_seen: AnswerSeen::Never,
        };
        for kind in MoveKind::ALL {
            if !def.moves.contains(&kind) {
                continue;
            }
            match kind {
                MoveKind::Record => {
                    for (policy, concern, s) in attacks::laws(v, p) {
                        let inst = v.r.policy().instances.get(policy as usize);
                        let subject =
                            Subject::of(What::Law { policy }, Some(concern.clone()));
                        let mut o = blank(
                            format!("record:law.{policy}"),
                            kind,
                            subject,
                            s * repeat(kind),
                        );
                        // A law's own area decides whom it reaches.
                        if let Some(area) = inst.and_then(|i| i.area()) {
                            for (gi, g) in groups.iter().enumerate() {
                                let hoods = audience::hoods_of(v.r.world, &g.group);
                                if g.group.crowd()
                                    && !area.iter().any(|h| hoods.contains(&h.index()))
                                {
                                    o.line.reach[gi] = p.spill;
                                }
                            }
                        }
                        let law_key = inst
                            .and_then(|i| content.policies.get(&i.template))
                            .map(|t| t.name_key.clone())
                            .unwrap_or_default();
                        o.args = vec![
                            ("law_key".into(), law_key),
                            ("worry_key".into(), concern_key(content, &concern)),
                        ];
                        push(o, false);
                    }
                }
                MoveKind::Heard => {
                    for (gi, g) in groups.iter().enumerate() {
                        if !matches!(g.group, Group::Area { .. }) {
                            continue;
                        }
                        // Second-hand knowledge gives the line with its
                        // risk shown, not no line.
                        let level = audience::worries_known(v, &g.group);
                        if level == Level::Unknown {
                            continue;
                        }
                        let Some((concern, felt)) = audience::top_worry(v, &g.group) else {
                            continue;
                        };
                        let visits = audience::hoods_of(v.r.world, &g.group)
                            .into_iter()
                            .map(|h| {
                                pl.ground.visits_now(
                                    NeighborhoodId(h as u32),
                                    today,
                                    p.visits_keep,
                                )
                            })
                            .fold(0.0, f64::max);
                        let presence = (visits / p.visits_full.max(1e-6))
                            .clamp(p.heard_floor, 1.0);
                        let key = g.group.key();
                        let mut o = blank(
                            format!("heard:{key}"),
                            kind,
                            Subject::of(What::Group { key }, Some(concern.clone())),
                            felt * presence * repeat(kind),
                        );
                        o.target = Some(gi);
                        o.line.reach = (0..n)
                            .map(|i| if i == gi { 1.0 } else { p.spill })
                            .collect();
                        if level < Level::Known {
                            o.line.p = hold(p, level);
                            o.notes.push(note("exchange-risk-secondhand", &[]));
                        }
                        if def.temper.is_some() {
                            o.deeds.push(Deed::Cool {
                                by: p.temper.cool_heard,
                            });
                        }
                        o.args = vec![
                            (
                                "group".into(),
                                audience::name(v, &g.group).unwrap_or_default(),
                            ),
                            ("worry_key".into(), concern_key(content, &concern)),
                        ];
                        push(o, false);
                    }
                }
                MoveKind::Turn => {
                    // A public fault still has to be looked up: someone the
                    // player has only heard of gives the line with a risk.
                    let Some(other) = opp.id.filter(|_| other_known > Level::Unknown) else {
                        continue;
                    };
                    let public_p = if other_known >= Level::Known {
                        1.0
                    } else {
                        hold(p, other_known)
                    };
                    let their = v.pol.characters.get(other).and_then(|c| c.politician);
                    let name = crate::characters::profile(v.pol, v.r.world, other)
                        .map(|x| x.full_name())
                        .unwrap_or_default();
                    let mut faults: Vec<(String, Subject, f64, f64, Option<u32>)> = Vec::new();
                    if let Some(their) = their {
                        for (policy, vote) in attacks::declared(v, their) {
                            let Some((_, against)) =
                                attacks::against_lean(v, groups, policy, vote)
                            else {
                                continue;
                            };
                            if against < p.size.lean_line {
                                continue;
                            }
                            let concern = v
                                .r
                                .policy()
                                .instances
                                .get(policy as usize)
                                .and_then(|i| attacks::worry_of(v, p, i));
                            faults.push((
                                format!("turn:vote.{policy}"),
                                Subject::of(What::Bill { policy }, concern),
                                against * p.public_share,
                                public_p,
                                None,
                            ));
                        }
                        let record = v.pol.get(their).map_or(1.0, |x| f64::from(x.record));
                        if record < p.size.thin_line {
                            faults.push((
                                "turn:record".into(),
                                Subject::of(What::Record, None),
                                (1.0 - record) * p.public_share,
                                public_p,
                                None,
                            ));
                        }
                        for s in v.r.media().running_about(their) {
                            faults.push((
                                format!("turn:story.{}", s.id),
                                Subject::of(What::Scandal { id: s.id }, None),
                                f64::from(s.heat) * p.public_share,
                                public_p,
                                None,
                            ));
                        }
                        // A secret may be aired in public, never across a
                        // private table (DEC-112, the owner's answer 2).
                        if def.table.is_none() {
                            let media = v.r.media();
                            for h in v.r.narrative().hidden.iter().filter(|h| {
                                h.subject == Some(their) && !media.discovered.contains(&h.id)
                            }) {
                                let level = secret_known(v, h.id);
                                if level == Level::Unknown {
                                    continue;
                                }
                                let size = match h.severity {
                                    mandate_content::Gravity::Ethics => p.size.gravity_ethics,
                                    mandate_content::Gravity::Misconduct => {
                                        p.size.gravity_misconduct
                                    }
                                    mandate_content::Gravity::Crime => p.size.gravity_crime,
                                };
                                faults.push((
                                    format!("turn:secret.{}", h.id),
                                    Subject::of(What::Hidden { id: h.id }, None),
                                    size,
                                    hold(p, level),
                                    Some(h.id),
                                ));
                            }
                        }
                    }
                    for (id, subject, strength, chance, hidden) in faults {
                        let mut o = blank(id, kind, subject, strength * repeat(kind));
                        o.line.p = chance;
                        o.deeds.push(Deed::Pressed { person: other });
                        o.notes.push(note(
                            "exchange-cost-remembers",
                            &[("other", name.clone())],
                        ));
                        if let Some(h) = hidden {
                            o.deeds.push(Deed::Surface { hidden: h });
                            o.notes.push(note("exchange-cost-aired", &[]));
                        } else if chance < 1.0 {
                            o.notes.push(note("exchange-risk-secondhand", &[]));
                        }
                        o.args = vec![
                            ("other".into(), name.clone()),
                            (
                                "worry_key".into(),
                                o.subject
                                    .concern
                                    .as_deref()
                                    .map(|c| concern_key(content, c))
                                    .unwrap_or_default(),
                            ),
                        ];
                        push(o, false);
                    }
                }
                MoveKind::Promise => {
                    let made = match audience::my_race(v) {
                        Some((race, _)) if v.r.asks().promises_race == Some(race) => {
                            v.r.asks().promises_this_campaign
                        }
                        _ => 0,
                    };
                    let rules = &content.ask_params.promises;
                    // The player's word: less for each public promise broken.
                    let word = mandate_core::math::pow(
                        rules.broken_discount,
                        f64::from(v.r.asks().broken_public_promises()),
                    );
                    for m in &menu {
                        let id = match &m.answer {
                            Some(a) => format!("promise:{}/{a}", m.template),
                            None => format!("promise:{}", m.template),
                        };
                        let mut o = blank(
                            id,
                            kind,
                            Subject::of(
                                What::Pledge {
                                    template: m.template.clone(),
                                    answer: m.answer.clone(),
                                },
                                m.concern.clone(),
                            ),
                            word * repeat(kind),
                        );
                        o.deeds.push(Deed::Promise {
                            template: m.template.clone(),
                            answer: m.answer.clone(),
                        });
                        o.notes.push(note(
                            "exchange-cost-promise",
                            &[
                                ("n", (made + 1).to_string()),
                                ("of", rules.max_per_campaign.to_string()),
                            ],
                        ));
                        o.args = vec![
                            (
                                "promise_key".into(),
                                content
                                    .promise(&m.template)
                                    .map(|t| t.name_key.clone())
                                    .unwrap_or_default(),
                            ),
                            (
                                "worry_key".into(),
                                m.concern
                                    .as_deref()
                                    .map(|c| concern_key(content, c))
                                    .unwrap_or_default(),
                            ),
                        ];
                        push(o, false);
                    }
                }
                MoveKind::Act => {
                    // A power that lets the player draft, and a template
                    // that answers: the arc's, and the live crisis's.
                    let home = pl.home();
                    let at = Jurisdiction::town(home);
                    let mut templates: Vec<String> = arc.answers.clone();
                    if let Some(k) = crate::asks::offers::live_crisis(v.r) {
                        for t in crate::asks::resolve::crisis_answers(content, &k) {
                            if !templates.contains(&t) {
                                templates.push(t);
                            }
                        }
                    }
                    for id in templates {
                        let Some(t) = content.policies.get(&id) else {
                            continue;
                        };
                        if t.retired
                            || t.tier != mandate_content::TierKey::Local
                            || !crate::actions::player_holds_power_at(st, content, t.requires, at)
                            || !crate::policy::unlocked(st, content, t, home.0)
                        {
                            continue;
                        }
                        // Not one already on the player's desk or in force.
                        if v.r.policy().instances.iter().any(|i| {
                            i.sponsor == Some(pl.politician)
                                && i.template == id
                                && i.status != crate::policy::PolicyStatus::Ended
                        }) {
                            continue;
                        }
                        let concern = attacks::worry_of_template(content, p, t);
                        let votes = backing(v, at, t.requires) >= 0.5;
                        let strength = p.act_base + if votes { p.act_votes } else { 0.0 };
                        let mut o = blank(
                            format!("act:{id}"),
                            kind,
                            Subject::of(
                                What::Template {
                                    template: id.clone(),
                                },
                                concern.clone(),
                            ),
                            strength * repeat(kind),
                        );
                        o.deeds.push(Deed::Draft {
                            template: id.clone(),
                        });
                        o.notes.push(note("exchange-cost-draft", &[]));
                        if votes {
                            o.notes.push(note("exchange-note-votes", &[]));
                        }
                        o.args = vec![
                            ("law_key".into(), t.name_key.clone()),
                            (
                                "worry_key".into(),
                                concern
                                    .as_deref()
                                    .map(|c| concern_key(content, c))
                                    .unwrap_or_default(),
                            ),
                        ];
                        // "You have had N years" bites only where the
                        // player has no law on the subject.
                        let has_law = concern.as_deref().is_some_and(|c| {
                            attacks::laws(v, p).iter().any(|(_, lc, _)| lc == c)
                        });
                        push(o, has_law);
                    }
                }
                MoveKind::Own => {
                    if !(prompt.attack.source.fault() && prompt.stood) {
                        continue;
                    }
                    let mut o = blank("own".into(), kind, Subject::default(), 0.0);
                    o.deeds.push(Deed::Retire);
                    o.deeds.push(Deed::Standing {
                        delta: p.world.own_standing,
                    });
                    o.notes.push(note("exchange-cost-retires", &[]));
                    o.notes.push(note(
                        "exchange-cost-standing",
                        &[("n", format!("{}", p.world.own_standing.abs().round() as i64))],
                    ));
                    if def.temper.is_some() {
                        o.deeds.push(Deed::Cool {
                            by: p.temper.cool_own,
                        });
                        o.notes.push(note("exchange-note-cools", &[]));
                    }
                    push(o, false);
                }
                MoveKind::Deny => {
                    if !(prompt.attack.source.deniable() && prompt.stood) {
                        continue;
                    }
                    // It unravels with a chance equal to the evidence
                    // that exists.
                    let hidden = match prompt.attack.subject.what {
                        What::Hidden { id } => Some(id),
                        What::Scandal { id } => v.r.media().scandal(id).map(|s| s.hidden),
                        _ => None,
                    };
                    let evidence = hidden
                        .and_then(|h| v.r.narrative().hidden.iter().find(|x| x.id == h))
                        .map_or(0.5, |h| f64::from(h.evidence));
                    if evidence >= 1.0 {
                        continue;
                    }
                    let mut o = blank("deny".into(), kind, Subject::default(), 0.0);
                    o.line.p = (1.0 - evidence).clamp(0.0, 1.0);
                    o.notes.push(note("exchange-cost-denial", &[]));
                    push(o, false);
                }
                MoveKind::Offer => {
                    // Without a table: time. Stay for every question.
                    if def.table.is_some() || pl.slots_left() == 0 {
                        continue;
                    }
                    let mut o = blank(
                        "offer:time".into(),
                        kind,
                        Subject::of(What::Time, None),
                        p.stay_strength * repeat(kind),
                    );
                    o.deeds.push(Deed::Slot);
                    o.notes.push(note("exchange-cost-slot", &[]));
                    if def.temper.is_some() {
                        o.deeds.push(Deed::Cool {
                            by: p.temper.cool_stay,
                        });
                        o.notes.push(note("exchange-note-cools", &[]));
                    }
                    push(o, false);
                }
                MoveKind::Second => {
                    let agreed = def
                        .prep
                        .iter()
                        .any(|x| x.adds == Some(kind) && run.has_mark(&x.mark));
                    let backer = def.sides.second.as_deref().and_then(|r| run.person(r));
                    let (true, Some(b)) = (agreed, backer) else {
                        continue;
                    };
                    let name = crate::characters::profile(v.pol, v.r.world, b)
                        .map(|x| x.full_name())
                        .unwrap_or_default();
                    let mut o = blank(
                        "second".into(),
                        kind,
                        Subject::of(What::Backer, None),
                        p.second_base * repeat(kind),
                    );
                    // Its fit is the backer's tie to each group, not a worry.
                    o.line.fit = audience::ties(v, p, groups, b);
                    o.prep = true;
                    o.notes.push(note(
                        "exchange-cost-second",
                        &[("second", name.clone())],
                    ));
                    o.args = vec![("second".into(), name)];
                    push(o, false);
                }
                MoveKind::Proof => {
                    for x in def
                        .prep
                        .iter()
                        .filter(|x| x.adds == Some(kind) && run.has_mark(&x.mark))
                    {
                        let e = f64::from(x.evidence.unwrap_or(1.0)).clamp(0.0, 1.0);
                        let mut o = blank(
                            format!("proof:{}", x.mark),
                            kind,
                            Subject::of(
                                What::Found {
                                    mark: x.mark.clone(),
                                },
                                None,
                            ),
                            e * repeat(kind),
                        );
                        o.line.p = e;
                        o.prep = true;
                        o.notes.push(note("exchange-cost-public", &[]));
                        push(o, false);
                    }
                }
                MoveKind::Pass => {
                    push(blank("pass".into(), kind, Subject::default(), 0.0), false);
                }
            }
        }
        out.sort_by(|a, b| a.kind.cmp(&b.kind).then_with(|| a.id.cmp(&b.id)));
        out
    })
}

/// The lines shown (§3.3, step 3): the safe move; what preparation added;
/// the best that answers; the best that changes the subject; the best
/// that carries a chance; then the best of the rest. Ranked by the
/// engine's true numbers; shown in the table's order.
pub fn shown(p: &ExchangeParams, r: &RoundIn, weights: &[f64], opts: &[Opt]) -> Vec<usize> {
    let scores: Vec<f64> = opts
        .iter()
        .map(|o| {
            let (kr, kl) = landing::known(r, &o.line);
            landing::score(&landing::land(p, &kr, &kl), weights)
        })
        .collect();
    // Best first; ties to the kind earlier in the table, then the lower id
    // (the order `opts` is already in).
    let mut order: Vec<usize> = (0..opts.len()).collect();
    order.sort_by(|a, b| scores[*b].total_cmp(&scores[*a]).then(a.cmp(b)));
    let max = usize::from(p.list_max);
    let mut picked: Vec<usize> = Vec::new();
    let take = |i: Option<usize>, picked: &mut Vec<usize>| {
        if let Some(i) = i
            && !picked.contains(&i)
            && picked.len() < max
        {
            picked.push(i);
        }
    };
    take(opts.iter().position(|o| o.kind == MoveKind::Pass), &mut picked);
    let preps: Vec<usize> = order
        .iter()
        .copied()
        .filter(|i| opts[*i].prep)
        .take(usize::from(p.prep_pins))
        .collect();
    for i in preps {
        take(Some(i), &mut picked);
    }
    let first = |f: &dyn Fn(&Opt) -> bool, picked: &Vec<usize>| {
        order
            .iter()
            .copied()
            .find(|i| !picked.contains(i) && f(&opts[*i]))
    };
    let i = first(&|o| o.line.answers && o.kind != MoveKind::Pass, &picked);
    take(i, &mut picked);
    let i = first(&|o| !o.line.answers, &picked);
    take(i, &mut picked);
    let i = first(&|o| o.line.p < 1.0, &picked);
    take(i, &mut picked);
    while picked.len() < max {
        let Some(i) = first(&|_| true, &picked) else {
            break;
        };
        take(Some(i), &mut picked);
    }
    picked.sort_unstable();
    picked
}

/// The locked line (§3.3, step 4; §9): one kind the scene allows that the
/// state gives no line for, with a route to it. At most one a round, a
/// different one each round, and never a list of what was missed.
pub fn locked(
    st: &State,
    content: &ContentBundle,
    run: &ArcRun,
    def: &ArcExchangeDef,
    xs: &ArcExchangeState,
    opts: &[Opt],
) -> Option<Locked> {
    crate::condition::on_state(st, content, |v| {
        let has = |k: MoveKind| opts.iter().any(|o| o.kind == k);
        let other = def.sides.other.as_str();
        let other_id = run.person(other);
        let other_name = other_id
            .and_then(|o| crate::characters::profile(v.pol, v.r.world, o))
            .map(|x| x.full_name())
            .unwrap_or_default();
        let mut routes: Vec<Locked> = Vec::new();
        for kind in &def.moves {
            if has(*kind) || xs.used.iter().any(|u| u.split(':').next() == Some(kind.key())) {
                continue;
            }
            let (reason, args): (&str, Vec<(String, String)>) = match kind {
                MoveKind::Record => ("record", Vec::new()),
                MoveKind::Heard => {
                    // The heaviest group the player has not heard from.
                    let g = xs
                        .groups
                        .iter()
                        .filter(|g| matches!(g.group, Group::Area { .. }))
                        .filter(|g| audience::worries_known(v, &g.group) == Level::Unknown)
                        .max_by(|a, b| a.weight.total_cmp(&b.weight));
                    match g.and_then(|g| audience::name(v, &g.group)) {
                        Some(name) => ("heard", vec![("group".into(), name)]),
                        None => continue,
                    }
                }
                MoveKind::Turn => {
                    let known = other_id
                        .map_or(Level::Unknown, |o| audience::person_known(v, o));
                    let reason = if known == Level::Unknown {
                        "turn-unknown"
                    } else {
                        "turn-nothing"
                    };
                    (reason, vec![("other".into(), other_name.clone())])
                }
                MoveKind::Promise => ("promise", Vec::new()),
                MoveKind::Act => ("act", Vec::new()),
                MoveKind::Second => ("second", Vec::new()),
                MoveKind::Proof => ("proof", Vec::new()),
                MoveKind::Own
                | MoveKind::Deny
                | MoveKind::Offer
                | MoveKind::Pass => continue,
            };
            routes.push(Locked {
                kind: *kind,
                key: format!("exchange-locked-{reason}"),
                args,
            });
        }
        if routes.is_empty() {
            return None;
        }
        let i = usize::from(xs.round.saturating_sub(1)) % routes.len();
        Some(routes.swap_remove(i))
    })
}
