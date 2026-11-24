//! The Legacy record (DEC-006, DEC-100): an append-only record of an act's
//! career: posts held, elections, laws passed and works, promises kept and
//! broken, notable decisions, scandals and the courts, the final standing
//! and the ending. Facts only (DEC-054): the epilogue words them.
//!
//! It is read from what happened (the event log since the last turn, the
//! posts held, the characters' memories of the player), never written by
//! another domain. No unlocks: a record, nothing more (DEC-100).

use crate::events::EventKind;
use crate::politics::Post;
use mandate_content::ContentBundle;
use mandate_content::act::{EndingDef, EndingFact, EndingTone, WeightKey};
use mandate_core::SimDate;
use serde::{Deserialize, Serialize};

/// What one Legacy entry records.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LegacyKind {
    /// The player took up a post (a seat or an office of the constitution).
    PostTaken {
        post: Post,
    },
    /// The player left a post.
    PostLeft {
        post: Post,
    },
    /// An election the player stood in.
    Election {
        post: Post,
        won: bool,
        share: f32,
    },
    /// A law the player sponsored was enacted (`works`: in a works category).
    LawPassed {
        policy: u32,
        template: String,
        works: bool,
    },
    /// A bill the player sponsored failed.
    BillFailed {
        policy: u32,
        template: String,
    },
    /// A notable decision the player answered (weights `legacy.notable`).
    Decision {
        decision: u32,
        template: String,
        option: String,
    },
    /// Someone remembers the player keeping a promise to them.
    PromiseKept {
        person: u32,
    },
    /// Someone remembers the player breaking one.
    PromiseBroken {
        person: u32,
    },
    /// A campaign promise to a constituency's voters was kept or broken
    /// (DEC-097 §2; the asks' outcomes).
    CampaignPromise {
        template: String,
        kept: bool,
    },
    /// A story about the player broke.
    Scandal {
        scandal: u32,
        kind: String,
        severity: String,
    },
    /// A case against the player moved on (`stage`: charged, convicted, …).
    Case {
        case: u32,
        stage: String,
    },
    /// A sentence removed the player from office.
    RemovedFromOffice {
        case: u32,
    },
    /// The party would not renominate the player.
    Deselected,
    Imprisoned {
        years: u32,
    },
    Released,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LegacyEntry {
    pub date: SimDate,
    #[serde(flatten)]
    pub kind: LegacyKind,
    /// The event it records, when it came from one (the causal spine: the
    /// log keeps that event, and the chain behind it, as long as the entry).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_event: Option<crate::events::EventId>,
}

/// The tracks when the act closed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FinalStanding {
    pub mandate: f32,
    pub standing: f32,
    pub viability: f32,
    pub legal_jeopardy: f32,
    pub health: f32,
    pub money: i64,
    /// Posts held at the close.
    pub posts: Vec<Post>,
}

/// An ending as recorded: the content's keys and the facts it reported.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EndingRecord {
    pub id: String,
    pub title_key: String,
    pub text_key: String,
    pub tone: String,
    /// (fact key, value), in the ending's order.
    pub facts: Vec<(String, i64)>,
}

/// One act's record.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LegacyRecord {
    /// The act (the constitution's `career.acts` id; empty when the player
    /// held no post of any act when it began).
    pub act: String,
    pub began: SimDate,
    /// Posts held when the act began.
    pub start_posts: Vec<Post>,
    /// Posts held now (the diff makes `PostTaken` and `PostLeft`).
    pub holding: Vec<Post>,
    pub entries: Vec<LegacyEntry>,
    pub closed: Option<SimDate>,
    pub final_standing: Option<FinalStanding>,
    pub ending: Option<EndingRecord>,
}

impl LegacyRecord {
    pub fn count(&self, f: impl Fn(&LegacyKind) -> bool) -> usize {
        self.entries.iter().filter(|e| f(&e.kind)).count()
    }
    /// Distinct posts held this act (at its start or since).
    pub fn posts_held(&self) -> Vec<Post> {
        let mut out = self.start_posts.clone();
        for e in &self.entries {
            if let LegacyKind::PostTaken { post } = &e.kind
                && !out.contains(post)
            {
                out.push(*post);
            }
        }
        out
    }
}

/// The constitution act a post belongs to.
pub fn act_of_post(c: &mandate_content::ConstitutionDef, post: Post) -> Option<usize> {
    let tier = post.at().tier();
    c.career.acts.iter().position(|a| a.covers(tier, post.id()))
}

/// The act the player is in: that of the highest post they hold, else the
/// first act of the career.
fn current_act(state: &crate::State, content: &ContentBundle) -> String {
    let Some(c) = content.constitutions.get(&state.meta.constitution) else {
        return String::new();
    };
    let posts = crate::player::posts(state);
    posts
        .iter()
        .filter_map(|p| act_of_post(c, *p))
        .max()
        .or(if c.career.acts.is_empty() {
            None
        } else {
            Some(0)
        })
        .and_then(|i| c.career.acts.get(i))
        .map(|a| a.id.clone())
        .unwrap_or_default()
}

/// Open the current act's record (at play start, or for a run from before
/// the record: from now on).
pub fn begin(state: &mut crate::State, content: &ContentBundle) {
    if state.player.is_none() {
        return;
    }
    let posts = crate::player::posts(state);
    let date = state.calendar.date;
    let act = current_act(state, content);
    state.act.cursor = state.events.iter().last().map_or(0, |e| e.id.0 + 1);
    state.act.scanned_to = Some(date);
    state.act.records.push(LegacyRecord {
        act,
        began: date,
        start_posts: posts.clone(),
        holding: posts,
        entries: Vec::new(),
        closed: None,
        final_standing: None,
        ending: None,
    });
}

fn weight_key(w: crate::narrative::Weight) -> WeightKey {
    match w {
        crate::narrative::Weight::Climax => WeightKey::Climax,
        crate::narrative::Weight::Significant => WeightKey::Significant,
        crate::narrative::Weight::Minor => WeightKey::Minor,
    }
}

/// Read the turn's events, posts and memories into the record.
pub fn observe(state: &mut crate::State, content: &ContentBundle) {
    let Some(pl) = state.player.as_ref() else {
        return;
    };
    let me = pl.politician;
    let my_char = state.politics.get(me).map(|p| p.character);
    let date = state.calendar.date;
    let legacy = &content.act_flow.legacy;
    let cursor = state.act.cursor;
    let mut new: Vec<LegacyEntry> = Vec::new();
    // The event being read (none for entries from memories and posts).
    let reading = std::cell::Cell::new(None);
    let mut push = |d: SimDate, kind: LegacyKind| {
        new.push(LegacyEntry {
            date: d,
            kind,
            source_event: reading.get(),
        })
    };
    let mut last = cursor;
    for ev in state.events.iter().filter(|e| e.id.0 >= cursor) {
        last = ev.id.0 + 1;
        reading.set(Some(ev.id));
        match &ev.kind {
            EventKind::RaceDecided {
                player_ran: true,
                player_won,
                post: Some(post),
                ..
            } => {
                let share = state
                    .politics
                    .elections
                    .results
                    .iter()
                    .rev()
                    .find(|r| r.date == ev.date && r.seat == *post)
                    .and_then(|r| r.candidates.iter().find(|c| c.politician == me))
                    .map_or(0.0, |c| c.share);
                push(
                    ev.date,
                    LegacyKind::Election {
                        post: *post,
                        won: *player_won,
                        share,
                    },
                );
            }
            EventKind::PolicyEnacted { policy, .. } | EventKind::PolicyFailed { policy, .. } => {
                let Some(p) = state.policy.instances.get(*policy as usize) else {
                    continue;
                };
                if p.sponsor != Some(me) {
                    continue;
                }
                let template = p.template.clone();
                if matches!(ev.kind, EventKind::PolicyEnacted { .. }) {
                    let works = crate::policy::resolve(content, &state.policy, p)
                        .is_some_and(|t| legacy.works_categories.contains(&t.category));
                    push(
                        ev.date,
                        LegacyKind::LawPassed {
                            policy: *policy,
                            template,
                            works,
                        },
                    );
                } else {
                    push(
                        ev.date,
                        LegacyKind::BillFailed {
                            policy: *policy,
                            template,
                        },
                    );
                }
            }
            EventKind::DecisionResolved {
                decision,
                template,
                option,
                by_default: false,
            } => {
                let notable = state
                    .narrative
                    .decision(crate::narrative::DecisionId(*decision))
                    .is_some_and(|d| legacy.notable.contains(&weight_key(d.weight)));
                if notable {
                    push(
                        ev.date,
                        LegacyKind::Decision {
                            decision: *decision,
                            template: template.clone(),
                            option: option.clone(),
                        },
                    );
                }
            }
            EventKind::ScandalBroke {
                scandal,
                politician,
                severity,
                kind,
                ..
            } if *politician == me.0 => push(
                ev.date,
                LegacyKind::Scandal {
                    scandal: *scandal,
                    kind: kind.clone(),
                    severity: severity.clone(),
                },
            ),
            EventKind::CaseDeveloped {
                case,
                politician,
                stage,
            } if *politician == me.0 => {
                push(
                    ev.date,
                    LegacyKind::Case {
                        case: *case,
                        stage: stage.clone(),
                    },
                );
                let removed = stage == "convicted"
                    && state
                        .justice
                        .cases
                        .iter()
                        .find(|c| c.id == *case)
                        .and_then(|c| c.sentence)
                        .is_some_and(|s| !matches!(s, crate::justice::Sentence::Fine { .. }));
                if removed {
                    push(ev.date, LegacyKind::RemovedFromOffice { case: *case });
                }
            }
            EventKind::PlayerDeselected => push(ev.date, LegacyKind::Deselected),
            EventKind::PlayerImprisoned { years, .. } => {
                push(ev.date, LegacyKind::Imprisoned { years: *years })
            }
            EventKind::PlayerReleased => push(ev.date, LegacyKind::Released),
            // Campaign promises (an ask's to a person is remembered by them,
            // below).
            EventKind::AskResolved {
                ask,
                template,
                outcome,
                ..
            } if state
                .asks
                .get(crate::asks::AskId(*ask))
                .is_some_and(|a| a.promise)
                && (outcome == "kept" || outcome == "broken") =>
            {
                push(
                    ev.date,
                    LegacyKind::CampaignPromise {
                        template: template.clone(),
                        kept: outcome == "kept",
                    },
                )
            }
            _ => {}
        }
    }
    reading.set(None);
    // Promises kept and broken, as the people they were made to remember them.
    let from = state.act.scanned_to.unwrap_or(date);
    if let Some(mc) = my_char {
        for ch in &state.politics.characters.people {
            for m in ch
                .memories
                .iter()
                .filter(|m| m.actor == mc && m.date > from && m.date <= date && ch.id != mc)
            {
                match m.kind {
                    crate::characters::MemoryKind::KeptPromise => {
                        push(m.date, LegacyKind::PromiseKept { person: ch.id.0 })
                    }
                    crate::characters::MemoryKind::BrokePromise => {
                        push(m.date, LegacyKind::PromiseBroken { person: ch.id.0 })
                    }
                    _ => {}
                }
            }
        }
    }
    // Posts taken and left.
    let posts = crate::player::posts(state);
    let Some(rec) = state.act.records.last_mut() else {
        return;
    };
    for p in rec.holding.iter().filter(|p| !posts.contains(p)) {
        push(date, LegacyKind::PostLeft { post: *p });
    }
    for p in posts.iter().filter(|p| !rec.holding.contains(p)) {
        push(date, LegacyKind::PostTaken { post: *p });
    }
    rec.holding = posts;
    // Same-day entries in a stable order: as found (events, memories, posts).
    new.sort_by_key(|e| e.date);
    rec.entries.extend(new);
    // Bounded: the oldest notable decisions go first, then the oldest entries.
    let cap = legacy.max_entries.max(1) as usize;
    while rec.entries.len() > cap {
        let i = rec
            .entries
            .iter()
            .position(|e| matches!(e.kind, LegacyKind::Decision { .. }))
            .unwrap_or(0);
        rec.entries.remove(i);
    }
    state.act.cursor = last;
    state.act.scanned_to = Some(date);
}

/// A fact's value for the current record.
pub fn fact(state: &crate::State, rec: &LegacyRecord, f: EndingFact) -> i64 {
    let tracks = state.player.as_ref().map(|p| (&p.tracks, p.mandate));
    let n = |k: fn(&LegacyKind) -> bool| rec.count(k) as i64;
    match f {
        EndingFact::PostsHeld => rec.posts_held().len() as i64,
        EndingFact::Years => (rec.began.days_until(state.calendar.date) as f64
            / mandate_core::time::DAYS_PER_YEAR as f64)
            .round() as i64,
        EndingFact::LawsPassed => n(|k| matches!(k, LegacyKind::LawPassed { .. })),
        EndingFact::Works => n(|k| matches!(k, LegacyKind::LawPassed { works: true, .. })),
        EndingFact::ElectionsWon => n(|k| matches!(k, LegacyKind::Election { won: true, .. })),
        EndingFact::ElectionsLost => n(|k| matches!(k, LegacyKind::Election { won: false, .. })),
        EndingFact::PromisesKept => n(|k| {
            matches!(
                k,
                LegacyKind::PromiseKept { .. } | LegacyKind::CampaignPromise { kept: true, .. }
            )
        }),
        EndingFact::PromisesBroken => n(|k| {
            matches!(
                k,
                LegacyKind::PromiseBroken { .. } | LegacyKind::CampaignPromise { kept: false, .. }
            )
        }),
        EndingFact::Decisions => n(|k| matches!(k, LegacyKind::Decision { .. })),
        EndingFact::Scandals => n(|k| matches!(k, LegacyKind::Scandal { .. })),
        EndingFact::Mandate => tracks.map_or(0, |(_, m)| m.round() as i64),
        EndingFact::Standing => tracks.map_or(0, |(t, _)| t.standing.round() as i64),
        EndingFact::Viability => tracks.map_or(0, |(t, _)| t.viability.round() as i64),
    }
}

fn tone_key(t: EndingTone) -> &'static str {
    match t {
        EndingTone::Triumph => "triumph",
        EndingTone::Neutral => "neutral",
        EndingTone::Defeat => "defeat",
    }
}

/// Close the current act's record with an ending.
pub fn close(state: &mut crate::State, _content: &ContentBundle, e: &EndingDef) {
    let date = state.calendar.date;
    let Some(rec) = state.act.records.last() else {
        return;
    };
    let facts: Vec<(String, i64)> = e
        .facts
        .iter()
        .map(|f| (f.key().to_string(), fact(state, rec, *f)))
        .collect();
    let fin = state.player.as_ref().map(|p| FinalStanding {
        mandate: p.mandate,
        standing: p.tracks.standing,
        viability: p.tracks.viability,
        legal_jeopardy: p.tracks.legal_jeopardy,
        health: p.tracks.health,
        money: p.money.units(),
        posts: rec.holding.clone(),
    });
    let act = rec.act.clone();
    if let Some(rec) = state.act.records.last_mut() {
        rec.closed = Some(date);
        rec.final_standing = fin;
        rec.ending = Some(EndingRecord {
            id: e.id.clone(),
            title_key: e.title_key.clone(),
            text_key: e.text_key.clone(),
            tone: tone_key(e.tone).into(),
            facts,
        });
    }
    state.act.ending = Some(crate::act::ActEnding {
        act,
        ending: e.id.clone(),
        date,
    });
}
