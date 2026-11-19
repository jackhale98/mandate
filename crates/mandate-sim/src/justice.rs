//! The legal path (DEC-006 Legal Jeopardy): a scandal may draw an
//! investigation; an investigation ends in charges or is dropped; charges go
//! to trial; a conviction brings a fine, removal from office, or prison.
//!
//! **Fairness (DEC-006).** A career-ending sentence (a long one, or a
//! disqualification) is handed down only after the Legal Jeopardy warning
//! and its response window (`NarrativeState::ending_allowed`); until then
//! the trial is adjourned. Charges raise the track past the warning line,
//! and a trial comes at least `trial_days[0]` (≥ the response time) later.
//!
//! **Symmetry (DEC-030).** NPCs go through the same stages with the same
//! odds; they answer an investigation by personality where the player
//! answers a decision. Every draw is keyed by the case (seed,
//! `Stream::Media`, a tag, ids), so adjourning a trial never re-rolls it.

use crate::characters::CharacterId;
use crate::events::EventKind;
use crate::politics::PoliticianId;
use crate::scheduler::{Effect, Outs, Reads};
use mandate_content::{Discoverer, Gravity, LegalResponseKind};
use mandate_core::math::{clamp, sigmoid};
use mandate_core::{SimDate, Stream};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

const KEY_OPEN_DAYS: u64 = 0x1E61;
const KEY_OPEN: u64 = 0x1E62;
const KEY_INVESTIGATE_DAYS: u64 = 0x1E63;
const KEY_CHARGE: u64 = 0x1E64;
const KEY_TRIAL_DAYS: u64 = 0x1E65;
const KEY_CONVICT: u64 = 0x1E66;
const KEY_SENTENCE: u64 = 0x1E67;
const KEY_NPC_RESPONSE: u64 = 0x1E68;

/// Where a case stands (and what each development was).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Investigation,
    Charged,
    /// The trial waits for the fairness window (DEC-006).
    Adjourned,
    Dropped,
    Acquitted,
    Convicted,
}

impl Stage {
    pub fn key(self) -> &'static str {
        match self {
            Stage::Investigation => "investigation",
            Stage::Charged => "charged",
            Stage::Adjourned => "adjourned",
            Stage::Dropped => "dropped",
            Stage::Acquitted => "acquitted",
            Stage::Convicted => "convicted",
        }
    }

    pub fn open(self) -> bool {
        matches!(
            self,
            Stage::Investigation | Stage::Charged | Stage::Adjourned
        )
    }
}

/// A sentence (DEC-006): survivable (a fine, removal, a short sentence) or
/// career-ending (a long sentence, a disqualification).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Sentence {
    Fine {
        amount: f64,
    },
    /// A fine and removal from office.
    Removed {
        amount: f64,
    },
    Prison {
        years: u8,
    },
    /// Barred from office for life (with any prison term).
    Disqualified {
        years: u8,
    },
}

impl Sentence {
    pub fn key(&self) -> &'static str {
        match self {
            Sentence::Fine { .. } => "fine",
            Sentence::Removed { .. } => "removed",
            Sentence::Prison { .. } => "prison",
            Sentence::Disqualified { .. } => "disqualified",
        }
    }

    /// Ends a career (DEC-006 terminal ending).
    pub fn ends_career(&self, long_years: u8) -> bool {
        match self {
            Sentence::Prison { years } => *years >= long_years,
            Sentence::Disqualified { .. } => true,
            _ => false,
        }
    }

    pub fn prison_years(&self) -> u8 {
        match self {
            Sentence::Prison { years } | Sentence::Disqualified { years } => *years,
            _ => 0,
        }
    }
}

/// A case against a politician.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Case {
    pub id: u32,
    pub scandal: u32,
    pub hidden: u32,
    pub subject: PoliticianId,
    pub kind: String,
    pub severity: Gravity,
    pub evidence: f32,
    pub town: u32,
    pub by: Discoverer,
    pub opened: SimDate,
    pub stage: Stage,
    /// When the stage ends (the investigation's end, the trial).
    pub next_on: SimDate,
    pub response: Option<LegalResponseKind>,
    pub responded: Option<SimDate>,
    /// The ally blamed, if any.
    pub ally: Option<CharacterId>,
    /// Developments: (stage, date).
    pub history: Vec<(Stage, SimDate)>,
    pub sentence: Option<Sentence>,
}

impl Case {
    pub fn lawyer(&self) -> bool {
        self.response == Some(LegalResponseKind::Lawyer)
    }
}

/// The justice domain.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct JusticeState {
    pub cases: Vec<Case>,
    pub next_case: u32,
    /// Scandals prosecutors have considered (opened or not).
    pub considered: BTreeSet<u32>,
    /// Politicians barred from office: (who, until; `None` = for life).
    pub barred: Vec<(PoliticianId, Option<SimDate>)>,
}

impl JusticeState {
    pub fn case(&self, id: u32) -> Option<&Case> {
        self.cases.iter().find(|c| c.id == id)
    }

    pub fn case_mut(&mut self, id: u32) -> Option<&mut Case> {
        self.cases.iter_mut().find(|c| c.id == id)
    }

    /// Open cases against a politician.
    pub fn open_against(&self, p: PoliticianId) -> impl Iterator<Item = &Case> {
        self.cases
            .iter()
            .filter(move |c| c.subject == p && c.stage.open())
    }

    pub fn barred(&self, p: PoliticianId, on: SimDate) -> bool {
        self.barred
            .iter()
            .any(|(x, until)| *x == p && until.is_none_or(|u| u > on))
    }
}

fn keyed(seed: u64, parts: &[u64]) -> mandate_core::SimRng {
    mandate_core::rng::keyed(seed, Stream::Media, parts)
}

fn days(seed: u64, tag: u64, id: u32, range: [u32; 2]) -> u32 {
    let [lo, hi] = range;
    lo + keyed(seed, &[tag, id as u64]).below((hi.saturating_sub(lo) + 1) as u64) as u32
}

/// The Legal Jeopardy floor a case holds the player's track at.
pub fn track_floor(content: &mandate_content::ContentBundle, c: &Case) -> f32 {
    let j = &content.justice;
    let base = match c.stage {
        Stage::Investigation => j.track_investigation,
        Stage::Charged | Stage::Adjourned => j.track_charged,
        _ => 0.0,
    };
    (base + 10.0 * c.evidence as f64).min(99.0) as f32
}

/// The chance an investigation ends in charges, after the subject's answer.
pub fn charge_chance(content: &mandate_content::ContentBundle, c: &Case) -> f64 {
    let j = &content.justice;
    let x = &j.responses;
    let mut ev = c.evidence as f64;
    if c.response == Some(LegalResponseKind::Cooperate) {
        ev += x.cooperate_evidence;
    }
    let mut p = j.charge.get(c.severity) * (ev + j.charge_evidence_base);
    match c.response {
        Some(LegalResponseKind::Cooperate) => p *= x.cooperate_charge,
        Some(LegalResponseKind::Lawyer) => p -= x.lawyer_charge,
        Some(LegalResponseKind::Stonewall) => {
            if c.evidence as f64 >= x.stonewall_strong {
                p += x.stonewall_charge;
            } else {
                p -= x.stonewall_relief;
            }
        }
        Some(LegalResponseKind::BlameAlly) => p *= x.blame_charge,
        _ => {}
    }
    clamp(p, 0.0, 0.98)
}

/// The public mood around a politician (0 hostile .. 1 friendly): the
/// player's approval in their town, an NPC's scandal valence.
fn mood(r: &Reads<'_>, p: PoliticianId) -> f64 {
    let pol = r.politics();
    let Some(x) = pol.get(p) else { return 0.5 };
    if x.is_player {
        let pop = r.population();
        let (mut a, mut w) = (0.0, 0.0);
        for h in r
            .world
            .hoods_of(crate::world::MunicipalityId(x.municipality()))
        {
            for c in &pop.hoods[h].cohorts {
                a += c.player_approval as f64 * c.count as f64;
                w += c.count as f64;
            }
        }
        clamp((a / w.max(1.0) + 1.0) / 2.0, 0.0, 1.0)
    } else {
        clamp(0.5 - x.scandal as f64 / 2.0, 0.0, 1.0)
    }
}

/// The chance of a conviction at trial.
pub fn conviction_chance(r: &Reads<'_>, c: &Case) -> f64 {
    let j = &r.content.justice;
    let lawyer = if c.lawyer() {
        j.responses.lawyer_convict
    } else {
        0.0
    };
    sigmoid(
        j.convict_base + j.convict_evidence * c.evidence as f64 - lawyer
            + j.convict_mood * (0.5 - mood(r, c.subject)),
    )
}

/// The sentence a conviction brings (keyed by the case).
fn sentence(r: &Reads<'_>, c: &Case) -> Sentence {
    let j = &r.content.justice;
    let mut rng = keyed(r.meta.seed, &[KEY_SENTENCE, c.id as u64]);
    let fine = j.fine.get(c.severity);
    match c.severity {
        Gravity::Ethics => Sentence::Fine { amount: fine },
        Gravity::Misconduct => {
            if rng.chance(j.removal_misconduct) {
                Sentence::Removed { amount: fine }
            } else {
                Sentence::Fine { amount: fine }
            }
        }
        Gravity::Crime => {
            let [lo, hi] = j.prison_years;
            let mut years = lo + rng.below((hi.saturating_sub(lo) + 1) as u64) as u8;
            if c.response == Some(LegalResponseKind::Cooperate) {
                years = ((years as f64 * j.responses.cooperate_sentence).round() as u8).max(1);
            }
            if rng.chance(j.disqualify) {
                Sentence::Disqualified { years }
            } else {
                Sentence::Prison { years }
            }
        }
    }
}

/// An NPC's answer to an investigation, by personality (DEC-030).
fn npc_response(r: &Reads<'_>, p: PoliticianId, case: u32) -> LegalResponseKind {
    let pol = r.politics();
    let Some(x) = pol.get(p) else {
        return LegalResponseKind::None;
    };
    let u = keyed(r.meta.seed, &[KEY_NPC_RESPONSE, case as u64]).next_f64();
    let cost = r
        .content
        .dilemmas
        .iter()
        .find(|d| d.raised_by == Some(mandate_content::RaisedBy::Investigation))
        .and_then(|d| d.options.iter().find(|o| o.id == "lawyer"))
        .and_then(|o| {
            o.effects.iter().find_map(|e| match e {
                mandate_content::EffectDef::Money { amount } => Some(-amount),
                _ => None,
            })
        })
        .unwrap_or(3000.0);
    if x.integrity > 0.65 && u < 0.7 {
        LegalResponseKind::Cooperate
    } else if x.funds >= cost && u < 0.8 {
        LegalResponseKind::Lawyer
    } else if x.integrity < 0.35 && u < 0.5 {
        LegalResponseKind::Stonewall
    } else {
        LegalResponseKind::None
    }
}

/// Justice domain, Institutions phase, weekly: prosecutors consider new
/// scandals, investigations end, trials are held (or adjourned for the
/// fairness window), sentences land, and the barred leave office.
pub fn weekly(j: &mut JusticeState, r: &Reads<'_>, out: &mut Outs<'_>) {
    let media = r.media();
    if !media.active || r.player.is_none() {
        return;
    }
    let content = r.content;
    let jp = &content.justice;
    let seed = r.meta.seed;
    let date = r.date;
    let player = r.player.map(|p| p.politician);
    // 1. Prosecutors consider each scandal once, a few weeks after it broke.
    for s in &media.scandals {
        if j.considered.contains(&s.id) {
            continue;
        }
        let wait = days(seed, KEY_OPEN_DAYS, s.id, jp.open_days);
        if s.broke.add_days(wait) > date {
            continue;
        }
        j.considered.insert(s.id);
        let official = matches!(s.by, Discoverer::Auditor | Discoverer::Investigator);
        let p = jp.open.get(s.severity) * s.evidence as f64
            + if official { jp.official_bonus } else { 0.0 };
        if keyed(seed, &[KEY_OPEN, s.id as u64]).next_f64() >= p {
            continue;
        }
        let id = j.next_case;
        j.next_case += 1;
        let len = days(seed, KEY_INVESTIGATE_DAYS, id, jp.investigation_days);
        let is_player = Some(s.subject) == player;
        let response = (!is_player).then(|| npc_response(r, s.subject, id));
        let case = Case {
            id,
            scandal: s.id,
            hidden: s.hidden,
            subject: s.subject,
            kind: s.kind.clone(),
            severity: s.severity,
            evidence: s.evidence,
            town: s.town,
            by: s.by,
            opened: date,
            stage: Stage::Investigation,
            next_on: date.add_days(len),
            response,
            responded: response.map(|_| date),
            ally: None,
            history: vec![(Stage::Investigation, date)],
            sentence: None,
        };
        let ev = out.events.record(
            date,
            EventKind::CaseOpened {
                case: id,
                politician: s.subject.0,
            },
            vec![],
        );
        if is_player {
            out.events.pin(ev);
        }
        j.cases.push(case);
    }
    // 2. Stages end.
    let fairness = r.narrative();
    for c in j.cases.iter_mut().filter(|c| c.stage.open()) {
        if c.next_on > date {
            continue;
        }
        let is_player = Some(c.subject) == player;
        match c.stage {
            Stage::Investigation => {
                let p = charge_chance(content, c);
                if keyed(seed, &[KEY_CHARGE, c.id as u64]).next_f64() < p {
                    c.stage = Stage::Charged;
                    c.next_on = date.add_days(days(seed, KEY_TRIAL_DAYS, c.id, jp.trial_days));
                } else {
                    c.stage = Stage::Dropped;
                }
                c.history.push((c.stage, date));
                let ev = out.events.record(
                    date,
                    EventKind::CaseDeveloped {
                        case: c.id,
                        politician: c.subject.0,
                        stage: c.stage.key().into(),
                    },
                    vec![],
                );
                if is_player {
                    out.events.pin(ev);
                }
            }
            Stage::Charged | Stage::Adjourned => {
                let convicted =
                    keyed(seed, &[KEY_CONVICT, c.id as u64]).next_f64() < conviction_chance(r, c);
                let s = convicted.then(|| sentence(r, c));
                // DEC-006: no career-ending sentence without a warning and the
                // response window; the trial waits (the same draws later).
                if is_player
                    && s.is_some_and(|s| s.ends_career(jp.long_years))
                    && !fairness.ending_allowed(content, "legal_jeopardy", date)
                {
                    if c.stage != Stage::Adjourned {
                        c.stage = Stage::Adjourned;
                        c.history.push((Stage::Adjourned, date));
                    }
                    c.next_on = date.add_days(content.director.fairness.response_days.max(7));
                    continue;
                }
                c.stage = if convicted {
                    Stage::Convicted
                } else {
                    Stage::Acquitted
                };
                c.sentence = s;
                c.history.push((c.stage, date));
                let ev = out.events.record(
                    date,
                    EventKind::CaseDeveloped {
                        case: c.id,
                        politician: c.subject.0,
                        stage: c.stage.key().into(),
                    },
                    vec![],
                );
                if is_player {
                    out.events.pin(ev);
                }
                if let Some(s) = s {
                    let years = s.prison_years();
                    let removed = !matches!(s, Sentence::Fine { .. });
                    let life = s.ends_career(jp.long_years);
                    if removed {
                        let until = (years > 0 && !life).then(|| {
                            date.add_days(years as u32 * mandate_core::time::DAYS_PER_YEAR)
                        });
                        out.effects.push(Effect::Removal {
                            politician: c.subject,
                            barred: years > 0 || life,
                        });
                        if years > 0 || life {
                            j.barred.push((c.subject, if life { None } else { until }));
                        }
                    }
                    out.effects.push(Effect::Sentenced {
                        politician: c.subject,
                        case: c.id,
                        sentence: s,
                    });
                }
            }
            _ => {}
        }
    }
    // 3. The player's track shows their open case (DEC-006: a visible track).
    if let Some(p) = player {
        let floor = j
            .open_against(p)
            .map(|c| track_floor(content, c))
            .fold(0.0f32, f32::max);
        if floor > 0.0 {
            out.effects.push(Effect::PlayerTrack {
                standing: 0.0,
                viability: 0.0,
                money: 0.0,
                legal_floor: Some(floor),
            });
        }
    }
    // 4. The barred cannot hold office (a prisoner re-elected leaves again).
    let pol = r.politics();
    j.barred.retain(|(_, until)| until.is_none_or(|u| u > date));
    for (p, _) in &j.barred {
        if pol.get(*p).is_some_and(|x| x.in_office()) {
            out.effects.push(Effect::Removal {
                politician: *p,
                barred: true,
            });
        }
    }
    if j.cases.len() > 300
        && let Some(i) = j.cases.iter().position(|c| !c.stage.open())
    {
        j.cases.remove(i);
    }
}

/// The player (or anyone) answers an investigation (the Commands phase).
/// Blaming an ally points investigators at them: their scandal valence
/// rises (the memory is the decision's own effect).
pub fn respond(
    state: &mut crate::State,
    content: &mandate_content::ContentBundle,
    case: u32,
    kind: LegalResponseKind,
    ally: Option<CharacterId>,
) {
    let date = state.calendar.date;
    let Some(c) = state.justice.case_mut(case) else {
        return;
    };
    c.response = Some(kind);
    c.responded = Some(date);
    if kind == LegalResponseKind::BlameAlly {
        c.ally = ally;
        if let Some(a) = ally
            && let Some(p) = state.politics.characters.get(a).and_then(|x| x.politician)
            && let Some(pp) = state.politics.get_mut(p)
        {
            pp.scandal = (pp.scandal + content.justice.responses.blame_valence as f32).min(1.0);
        }
    }
}
