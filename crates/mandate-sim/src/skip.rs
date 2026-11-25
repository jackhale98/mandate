//! Time skips (DEC-012 "Term in review"): standing directives, the points a
//! skip can run to, interrupts, and the summary of what happened.
//!
//! A skip plays the player's turns with the proxy (`crate::proxy`), which
//! sends the same commands a player can, and ends each turn as `EndTurn`
//! does: NPCs, the director and the economy run as normal (the world does
//! not wait). It stops at its target, at an interrupt (`act_flow.jsonc`
//! `interrupts`: a change during the skip, never what was so when it
//! began), when the act ends, or when the player cancels between turns
//! (the safe point). The summary is facts and Fluent keys (DEC-054); it is
//! also the homecoming summary after an absence (REQ-010).

use crate::politics::{PoliticianId, Post};
use mandate_content::ContentBundle;
use mandate_content::act::{
    ArcStopStages, DirectiveLever, InterruptWhen, SkipTargetKind, VoteStance, WeightKey,
};
use mandate_core::SimDate;
use serde::{Deserialize, Serialize};

/// The player's standing directives (DEC-012): directive id → option id.
/// A directive not listed takes the content's default.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Directives {
    pub choices: Vec<(String, String)>,
}

impl Directives {
    /// The option chosen for a directive (the default when unset or no
    /// longer offered).
    pub fn chosen<'a>(&'a self, d: &'a mandate_content::act::DirectiveDef) -> &'a str {
        self.choices
            .iter()
            .find(|(k, _)| *k == d.id)
            .map(|(_, v)| v.as_str())
            .filter(|v| d.option(v).is_some() || d.concern_options)
            .unwrap_or(d.default.as_str())
    }
}

/// Whether `option` is offered by a directive.
pub fn offered(
    d: &mandate_content::act::DirectiveDef,
    option: &str,
    content: &ContentBundle,
) -> bool {
    d.option(option).is_some() || (d.concern_options && content.concern_keys().any(|k| k == option))
}

/// Directives as the proxy reads them.
#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    /// The worry to aim at; `None`: the strongest the player knows.
    pub focus: Option<String>,
    pub vote: VoteStance,
    pub ground: u8,
    pub fundraising: u8,
    pub press: u8,
    pub legislation: u8,
    pub max_risk: f32,
    pub rest_below: f32,
}

/// Resolve the player's directives against the content.
pub fn settings(dirs: &Directives, content: &ContentBundle) -> Settings {
    let mut s = Settings {
        focus: None,
        vote: VoteStance::WithParty,
        ground: 1,
        fundraising: 1,
        press: 0,
        legislation: 1,
        max_risk: 0.0,
        rest_below: 40.0,
    };
    for d in &content.act_flow.directives {
        let chosen = dirs.chosen(d);
        let opt = d.option(chosen);
        match d.lever {
            DirectiveLever::Focus => {
                s.focus = match opt {
                    Some(o) => o.concern.clone(),
                    None => Some(chosen.to_string()),
                }
            }
            DirectiveLever::Vote => {
                if let Some(v) = opt.and_then(|o| o.vote) {
                    s.vote = v;
                }
            }
            DirectiveLever::Caution => {
                if let Some(o) = opt {
                    s.max_risk = o.max_risk.unwrap_or(s.max_risk);
                    s.rest_below = o.rest_below.unwrap_or(s.rest_below);
                }
            }
            l => {
                let w = opt.and_then(|o| o.weight).unwrap_or(0);
                match l {
                    DirectiveLever::Ground => s.ground = w,
                    DirectiveLever::Fundraising => s.fundraising = w,
                    DirectiveLever::Press => s.press = w,
                    DirectiveLever::Legislation => s.legislation = w,
                    _ => {}
                }
            }
        }
    }
    s
}

/// What the player and their town looked like at a point (the summary's
/// before and after).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub date: SimDate,
    pub turn: u32,
    pub mandate: f32,
    pub standing: f32,
    pub viability: f32,
    pub legal_jeopardy: f32,
    pub health: f32,
    pub money: i64,
    pub posts: Vec<Post>,
    /// The home town's residents and unemployment rate (0..1).
    pub population: u64,
    pub unemployment: f32,
    /// Who holds the home jurisdiction's executive office.
    pub executive: Option<PoliticianId>,
    /// Laws in force at home.
    pub laws: u32,
}

/// Something that happened during a skip.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SummaryItem {
    /// An election the player stood in.
    Election {
        post: Post,
        won: bool,
    },
    /// A law enacted at home (`mine`: the player sponsored it).
    LawEnacted {
        policy: u32,
        mine: bool,
    },
    /// A bill of the player's failed.
    BillFailed {
        policy: u32,
    },
    /// The proxy answered a decision for the player.
    Answered {
        decision: u32,
        template: String,
        option: String,
    },
    /// A decision passed its deadline and took its default.
    Defaulted {
        decision: u32,
        template: String,
    },
    /// A story about the player broke.
    Scandal {
        scandal: u32,
        kind: String,
    },
    /// A new chapter began.
    Chapter {
        index: u32,
        kind: String,
    },
    /// A disaster struck the home town.
    Disaster {
        hazard: String,
    },
    /// A plant closed at home.
    PlantClosure {
        sector: String,
        jobs: u32,
    },
    /// The player declared a vote on a bill.
    Voted {
        policy: u32,
        vote: String,
    },
    /// A law at home stopped being in force: `expired` (a sunset clause),
    /// `repealed`.
    LawEnded {
        policy: u32,
        reason: String,
    },
    /// A bill of the player's was withdrawn.
    BillWithdrawn {
        policy: u32,
    },
    /// An ask or promise ended (DEC-097 §2): `kept`, `broken`, `void`, or
    /// `declined` (an offer left to lapse).
    AskResolved {
        ask: u32,
        template: String,
        outcome: String,
    },
    /// Someone asked the player for something (still open, or lapsed).
    AskOffered {
        ask: u32,
        template: String,
    },
    /// A crisis began (`began`) or ended at home.
    Crisis {
        kind: String,
        began: bool,
    },
    /// The party refused to renominate the player.
    Deselected,
    /// The party did not nominate the player for the post they filed for
    /// (`EventKind::NominationRefused`): why, and what they stand for now.
    NotNominated {
        post: Post,
        reason: String,
        nominee: Option<u32>,
        standing: u32,
        min_standing: Option<u32>,
        instead: Option<Post>,
    },
    /// A case against the player: `opened`, `charged`, `dropped`,
    /// `acquitted`, `convicted`.
    Case {
        case: u32,
        stage: String,
    },
    /// The player went to prison, or came out.
    Imprisoned {
        years: u32,
    },
    Released,
    /// A rumour about the player began to spread.
    Rumour {
        rumour: u32,
    },
    /// A national recession began (`began`) or recovery set in.
    Recession {
        began: bool,
    },
    /// A proposal (a policy family, DEC-092) came onto the table at home,
    /// or left it without a bill.
    Proposal {
        proposal: u32,
        family: String,
        opened: bool,
    },
}

/// Why a skip stopped.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SkipStop {
    /// It reached its target.
    Reached,
    /// An interrupt fired: its content id and Fluent key, with arguments.
    Interrupted {
        interrupt: String,
        key: String,
        args: Vec<(String, String)>,
    },
    /// The player cancelled it.
    Cancelled,
    /// The act ended (the ending's content id).
    ActEnded { ending: String },
    /// It ran the longest skip allowed (`act_flow.max_turns`).
    Limit,
}

/// A skip in progress (it runs in chunks so the player can cancel).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SkipRun {
    pub target: SkipTargetKind,
    /// It stops before a turn once the date is at or past this.
    pub until: SimDate,
    pub start: Snapshot,
    pub turns: u32,
    /// Interrupt keys already so (they do not fire again), sorted.
    pub seen: Vec<String>,
    /// The event log entry looked at last (exclusive).
    pub cursor: u32,
    pub items: Vec<SummaryItem>,
    /// The crises live at home when last looked (to tell when one begins
    /// or ends); `None` in a skip saved before it was kept (the next look
    /// starts it, reporting nothing).
    #[serde(default)]
    pub crises: Option<Vec<String>>,
}

/// Where a skip has got to (for a progress indicator).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SkipProgress {
    pub target: SkipTargetKind,
    pub from: SimDate,
    pub date: SimDate,
    pub until: SimDate,
    pub turns: u32,
    /// The skip is over (the summary is `State::act.last_skip`).
    pub done: bool,
}

impl SkipProgress {
    pub fn of(run: &SkipRun, date: SimDate, done: bool) -> Self {
        SkipProgress {
            target: run.target,
            from: run.start.date,
            date,
            until: run.until,
            turns: run.turns,
            done,
        }
    }
    /// The share of the way done (0..1).
    pub fn fraction(&self) -> f64 {
        let total = self.from.days_until(self.until).max(1) as f64;
        (self.from.days_until(self.date) as f64 / total).clamp(0.0, 1.0)
    }
}

/// A finished skip: the summary shown when play resumes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SkipSummary {
    pub target: SkipTargetKind,
    pub start: Snapshot,
    pub end: Snapshot,
    pub turns: u32,
    pub stop: SkipStop,
    pub items: Vec<SummaryItem>,
}

/// Summary items kept.
pub const ITEMS_CAP: usize = 80;

/// A point the engine offers to skip to, with its date.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Target {
    pub target: SkipTargetKind,
    pub name_key: String,
    pub until: SimDate,
}

/// The post the player's next election is for: the one they file for, else
/// the one they hold.
pub fn election_post(state: &crate::State, content: &ContentBundle) -> Option<Post> {
    let pl = state.player.as_ref()?;
    match pl.filing {
        Some(crate::politics::elections::Filing::Stand { post }) => Some(post),
        Some(crate::politics::elections::Filing::Retire) => None,
        None => crate::player::posts(state).first().copied().or_else(|| {
            crate::politics::elections::seat_filing(state, content).and_then(|f| match f {
                crate::politics::elections::Filing::Stand { post } => Some(post),
                _ => None,
            })
        }),
    }
}

/// The next election day for the player's post (the running race's, if
/// they are a candidate), with its campaign length in days.
fn election_day(state: &crate::State, content: &ContentBundle) -> Option<(SimDate, u32)> {
    let pl = state.player.as_ref()?;
    let c = content.constitutions.get(&state.meta.constitution)?;
    let post = election_post(state, content)?;
    let (cal, term) = crate::politics::elections::calendar_for(c, post)?;
    let weeks = cal.campaign_weeks as u32 * 7;
    if let Some(r) = state
        .politics
        .elections
        .races
        .iter()
        .find(|r| r.candidates.iter().any(|x| x.politician == pl.politician))
    {
        return Some((r.election_day, weeks));
    }
    let today = state.calendar.date;
    Some((
        crate::politics::elections::next_election_day(cal, term, today),
        weeks,
    ))
}

/// The points a skip can run to now, in content order (none while the
/// act is over or the player is out of play). A target already reached is
/// not offered.
pub fn targets(state: &crate::State, content: &ContentBundle) -> Vec<Target> {
    let Some(pl) = state.player.as_ref() else {
        return Vec::new();
    };
    if !pl.status.active() || state.act.over() || content.act_flow.directives.is_empty() {
        return Vec::new();
    }
    let today = state.calendar.date;
    let election = election_day(state, content);
    let campaigning = state
        .politics
        .elections
        .races
        .iter()
        .any(|r| r.candidates.iter().any(|x| x.politician == pl.politician));
    let mut out = Vec::new();
    for t in &content.act_flow.targets {
        let until = match t.target {
            SkipTargetKind::NextSession => state.narrative.chapter.as_ref().map(|c| c.end),
            SkipTargetKind::Campaign => election
                .filter(|_| !campaigning)
                .map(|(d, w)| SimDate(d.0.saturating_sub(w))),
            // The eve of the final week: the player plays election week.
            SkipTargetKind::Election => election.map(|(d, _)| SimDate(d.0.saturating_sub(7))),
            // Through the vote to its result.
            SkipTargetKind::EndOfTerm => election.map(|(d, _)| d),
        };
        if let Some(until) = until.filter(|u| *u > today) {
            out.push(Target {
                target: t.target,
                name_key: t.name_key.clone(),
                until,
            });
        }
    }
    out
}

/// The home town's figures for a snapshot.
pub fn snapshot(state: &crate::State, content: &ContentBundle) -> Snapshot {
    let pl = state.player.as_ref();
    let home = pl.map(|p| p.home());
    let population = home.map_or(0, |m| {
        state
            .world
            .hoods_of(m)
            .filter_map(|h| state.population.hoods.get(h))
            .map(|h| h.cohorts.iter().map(|c| c.count as u64).sum::<u64>())
            .sum()
    });
    let unemployment = home
        .and_then(|m| state.economy.munis.get(m.index()))
        .map_or(0.0, |e| e.unemployment as f32);
    let executive = home.and_then(|m| {
        let c = content.constitutions.get(&state.meta.constitution)?;
        crate::politics::executive_of(&state.politics, c, crate::world::Jurisdiction::town(m))
    });
    let laws = home.map_or(0, |m| {
        state
            .policy
            .in_municipality(m.0)
            .filter(|p| p.active())
            .count() as u32
    });
    Snapshot {
        date: state.calendar.date,
        turn: state.calendar.turn,
        mandate: pl.map_or(0.0, |p| p.mandate),
        standing: pl.map_or(0.0, |p| p.tracks.standing),
        viability: pl.map_or(0.0, |p| p.tracks.viability),
        legal_jeopardy: pl.map_or(0.0, |p| p.tracks.legal_jeopardy),
        health: pl.map_or(0.0, |p| p.tracks.health),
        money: pl.map_or(0, |p| p.money.units()),
        posts: crate::player::posts(state),
        population,
        unemployment,
        executive,
        laws,
    }
}

fn weight_key(w: crate::narrative::Weight) -> WeightKey {
    match w {
        crate::narrative::Weight::Climax => WeightKey::Climax,
        crate::narrative::Weight::Significant => WeightKey::Significant,
        crate::narrative::Weight::Minor => WeightKey::Minor,
    }
}

/// A decision the proxy leaves to the player: a weight it does not answer,
/// or one about a story or a case.
pub fn reserved(d: &crate::narrative::Decision, content: &ContentBundle) -> bool {
    d.about.is_some()
        || !content
            .act_flow
            .proxy
            .answers
            .contains(&weight_key(d.weight))
}

/// Whether a beat of a run's stage is the player's to take now: asked of
/// them and not yet settled (a decision, an ask), or one the engine would
/// offer them (its conditions hold for this player: they hold the power a
/// decision needs, the person to talk to is cast).
fn can_act(
    state: &crate::State,
    content: &ContentBundle,
    run: &crate::arcs::ArcRun,
    stage: &mandate_content::arcs::StageDef,
) -> bool {
    if run.open.iter().any(|b| b.stage == stage.id) {
        return true;
    }
    let people = run.narrative_cast();
    crate::condition::on_state(state, content, |v| {
        stage.beats.iter().any(|beat| {
            beat.kind.player() && crate::arcs::step::available(v, run, &people, stage, beat)
        })
    })
}

/// An interrupt key that holds: (the interrupt's index, the key, the
/// arguments its message reads).
pub type InterruptKey = (usize, String, Vec<(String, String)>);

/// Every interrupt key that holds now, with the interrupt's index and the
/// arguments its message reads. A key fires when it was not so before.
pub fn interrupt_keys(state: &crate::State, content: &ContentBundle) -> Vec<InterruptKey> {
    let Some(pl) = state.player.as_ref() else {
        return Vec::new();
    };
    let me = pl.politician;
    let today = state.calendar.date;
    let mut out = Vec::new();
    for (i, def) in content.act_flow.interrupts.iter().enumerate() {
        let id = &def.id;
        let mut add =
            |k: String, args: Vec<(String, String)>| out.push((i, format!("{id}:{k}"), args));
        match &def.when {
            InterruptWhen::Track {
                track,
                threshold,
                below,
            } => {
                let v = crate::endings::track(pl, *track);
                let crossed = if *below {
                    v <= *threshold
                } else {
                    v >= *threshold
                };
                if crossed {
                    add(
                        track.key().into(),
                        vec![
                            ("track".into(), track.key().into()),
                            ("value".into(), format!("{}", v.round() as i64)),
                        ],
                    );
                }
            }
            InterruptWhen::Decision { weights, about } => {
                for d in state.narrative.pending() {
                    if weights.contains(&weight_key(d.weight)) || (*about && d.about.is_some()) {
                        add(
                            d.id.0.to_string(),
                            vec![
                                ("decision".into(), d.id.0.to_string()),
                                ("template".into(), d.template.clone()),
                            ],
                        );
                    }
                }
            }
            InterruptWhen::Scandal { min_severity } => {
                for s in
                    state.media.scandals.iter().filter(|s| {
                        s.subject == me && s.ended.is_none() && s.severity >= *min_severity
                    })
                {
                    add(
                        s.id.to_string(),
                        vec![
                            ("scandal".into(), s.id.to_string()),
                            ("kind".into(), s.kind.clone()),
                        ],
                    );
                }
            }
            InterruptWhen::Election => {
                for r in state
                    .politics
                    .elections
                    .races
                    .iter()
                    .filter(|r| r.candidates.iter().any(|c| c.politician == me))
                {
                    add(
                        format!("{}:{}", r.election_day.0, r.seat.id()),
                        vec![("days".into(), today.days_until(r.election_day).to_string())],
                    );
                }
            }
            InterruptWhen::SponsoredVote { days } => {
                let soon = today.add_days(*days as u32);
                for p in state.policy.instances.iter().filter(|p| {
                    p.sponsor == Some(me)
                        && p.status == crate::policy::PolicyStatus::UnderConsideration
                }) {
                    if let Some(on) = p.next_stage_on.filter(|d| *d <= soon) {
                        add(
                            format!("{}:{}", p.id.0, p.stage),
                            vec![
                                ("policy".into(), p.id.0.to_string()),
                                ("days".into(), today.days_until(on).to_string()),
                            ],
                        );
                    }
                }
            }
            InterruptWhen::ArcStage { material, standing } => {
                // The moments of a run that are the player's to meet
                // (DEC-102 clause 7), by the kind of arc. Keyed by run and
                // moment: one that comes during the skip is new; what was
                // so as it began is not.
                let focus = pl.focus();
                let lead = state.narrative.chapter.as_ref().and_then(|c| c.lead);
                let args = |template: &str, stage: &str, run: u32| {
                    vec![
                        ("arc".into(), template.to_string()),
                        ("stage".into(), stage.to_string()),
                        ("run".into(), run.to_string()),
                    ]
                };
                for run in state.arcs.runs.iter().filter(|r| focus.contains(&r.at)) {
                    let Some(def) = content.arc(&run.template) else {
                        continue;
                    };
                    let Some(stage) = def.stage(&run.stage) else {
                        continue;
                    };
                    // A hidden stage is not the player's to know of.
                    if stage.hidden {
                        continue;
                    }
                    let rule = if def.standing.is_some() {
                        standing
                    } else {
                        material
                    };
                    if rule.raised {
                        add(
                            format!("{}:+raised", run.id.0),
                            args(&run.template, &run.stage, run.id.0),
                        );
                    }
                    let listed = match rule.stages {
                        ArcStopStages::None => false,
                        ArcStopStages::Turn => stage.climax,
                        ArcStopStages::Every => true,
                    };
                    if !listed {
                        continue;
                    }
                    let acts = can_act(state, content, run, stage);
                    let theirs = !rule.lead_or_actor || lead == Some(run.id.0) || acts;
                    if theirs && (rule.idle || acts) {
                        add(
                            format!("{}:{}", run.id.0, run.stage),
                            args(&run.template, &run.stage, run.id.0),
                        );
                    }
                }
                // An exit has nothing for the player to do.
                for p in state.arcs.past.iter().filter(|p| focus.contains(&p.at)) {
                    let Some(def) = content.arc(&p.template) else {
                        continue;
                    };
                    let rule = if def.standing.is_some() {
                        standing
                    } else {
                        material
                    };
                    if rule.idle && rule.stages == ArcStopStages::Every {
                        add(
                            format!("{}:{}", p.id.0, p.exit),
                            args(&p.template, &p.exit, p.id.0),
                        );
                    }
                }
            }
            InterruptWhen::RequestDue {
                days,
                offers,
                big_offers,
            } => {
                let soon = today.add_days(*days as u32);
                // A request raised by someone's goal (DEC-032), or an
                // arc's decision beat.
                for d in state
                    .narrative
                    .pending()
                    .filter(|d| (d.goal.is_some() || d.arc.is_some()) && d.deadline <= soon)
                {
                    add(
                        d.id.0.to_string(),
                        vec![
                            ("decision".into(), d.id.0.to_string()),
                            ("template".into(), d.template.clone()),
                        ],
                    );
                }
                // Asks and promises the player accepted, falling due (DEC-097
                // §2), through the asks' read API. With `offers`, an ask
                // still on offer stops the skip too, as its last day to
                // answer nears: the stand-in makes no promises for the
                // player (DEC-102 clause 7). Without, offers do not stop a
                // skip: left unanswered, they lapse as declined. With
                // `big_offers`, only a big ask on offer stops it.
                for a in state
                    .asks
                    .due_within(today, *days as u32)
                    .into_iter()
                    .filter(|a| {
                        a.accepted() || (a.offered() && (*offers || (*big_offers && a.big)))
                    })
                {
                    // An offer and the promise it becomes are two stops.
                    let kind = if a.offered() { "offer" } else { "ask" };
                    add(
                        format!("{kind}:{}", a.id.0),
                        vec![
                            ("ask".into(), a.id.0.to_string()),
                            ("template".into(), a.template.clone()),
                            ("status".into(), a.status.key().into()),
                        ],
                    );
                }
            }
            InterruptWhen::Status => {
                let k = match pl.status {
                    crate::player::RunStatus::Active => None,
                    crate::player::RunStatus::Imprisoned { .. } => Some("imprisoned"),
                    crate::player::RunStatus::Ended { .. } => Some("ended"),
                };
                if let Some(k) = k {
                    add(k.into(), vec![("status".into(), k.into())]);
                }
            }
        }
    }
    out
}

/// Start a skip to `target` (it must be offered now).
pub fn start(
    state: &crate::State,
    content: &ContentBundle,
    target: SkipTargetKind,
) -> Option<SkipRun> {
    let t = targets(state, content)
        .into_iter()
        .find(|t| t.target == target)?;
    let mut seen: Vec<String> = interrupt_keys(state, content)
        .into_iter()
        .map(|(_, k, _)| k)
        .chain(exchange_keys(state, content).into_iter().map(|x| x.0))
        .collect();
    seen.sort();
    seen.dedup();
    Some(SkipRun {
        target,
        until: t.until,
        start: snapshot(state, content),
        turns: 0,
        seen,
        cursor: state.events.iter().last().map_or(0, |e| e.id.0 + 1),
        items: Vec::new(),
        crises: Some(home_crises(state)),
    })
}

/// The kinds of crisis live in the player's home town, in registry order.
fn home_crises(state: &crate::State) -> Vec<String> {
    let Some(pl) = state.player.as_ref() else {
        return Vec::new();
    };
    state
        .politics
        .characters
        .towns
        .get(pl.home().index())
        .map(|t| t.crises.iter().map(|c| c.kind.clone()).collect())
        .unwrap_or_default()
}

/// An exchange scene on offer (DEC-112; DEC-102 clause 7): a key for each,
/// with the arguments its message reads. A skip stops when one opens, and
/// the stand-in never plays one. (Built in until the act flow has a
/// data-driven interrupt for an arc's stage.)
fn exchange_keys(
    state: &crate::State,
    content: &ContentBundle,
) -> Vec<(String, Vec<(String, String)>)> {
    crate::arcs::exchange::view::open_scenes(state, content)
        .into_iter()
        .map(|(run, beat, _)| {
            let last = state
                .arcs
                .run(run)
                .and_then(crate::arcs::exchange::last_day)
                .map(|d| d.calendar());
            let mut args = vec![
                ("run".to_string(), run.0.to_string()),
                ("beat".to_string(), beat.key()),
            ];
            if let Some(c) = last {
                args.push(("last_day".into(), c.day.to_string()));
                args.push(("last_month_key".into(), format!("month-{}", c.month)));
            }
            (format!("interrupt.exchange:{}.{}", run.0, beat.key()), args)
        })
        .collect()
}

/// The first interrupt that newly holds, as a stop (and every key now so
/// is marked seen).
pub fn check_interrupts(
    run: &mut SkipRun,
    state: &crate::State,
    content: &ContentBundle,
) -> Option<SkipStop> {
    let mut fired: Option<SkipStop> = None;
    for (k, args) in exchange_keys(state, content) {
        if let Err(at) = run.seen.binary_search(&k) {
            run.seen.insert(at, k);
            if fired.is_none() {
                fired = Some(SkipStop::Interrupted {
                    interrupt: "interrupt.exchange".into(),
                    key: "exchange-interrupt".into(),
                    args,
                });
            }
        }
    }
    for (i, k, args) in interrupt_keys(state, content) {
        if run.seen.binary_search(&k).is_ok() {
            continue;
        }
        if fired.is_none()
            && let Some(def) = content.act_flow.interrupts.get(i)
        {
            fired = Some(SkipStop::Interrupted {
                interrupt: def.id.clone(),
                key: def.key.clone(),
                args,
            });
        }
        if let Err(at) = run.seen.binary_search(&k) {
            run.seen.insert(at, k);
        }
    }
    fired
}

/// Read the turn's events into the summary.
pub fn observe(run: &mut SkipRun, state: &crate::State) {
    let Some(pl) = state.player.as_ref() else {
        return;
    };
    let me = pl.politician;
    let home = pl.home().0;
    let cursor = run.cursor;
    let mut last = cursor;
    let mine = |policy: u32| {
        state
            .policy
            .instances
            .get(policy as usize)
            .is_some_and(|p| p.sponsor == Some(me))
    };
    for ev in state.events.iter().filter(|e| e.id.0 >= cursor) {
        last = ev.id.0 + 1;
        use crate::events::EventKind as K;
        let item = match &ev.kind {
            K::RaceDecided {
                player_ran: true,
                player_won,
                post: Some(post),
                ..
            } => Some(SummaryItem::Election {
                post: *post,
                won: *player_won,
            }),
            K::PolicyEnacted {
                policy,
                municipality,
                ..
            } if *municipality == home => Some(SummaryItem::LawEnacted {
                policy: *policy,
                mine: state
                    .policy
                    .instances
                    .get(*policy as usize)
                    .is_some_and(|p| p.sponsor == Some(me)),
            }),
            K::PolicyFailed { policy, .. } if mine(*policy) => {
                Some(SummaryItem::BillFailed { policy: *policy })
            }
            K::DecisionResolved {
                decision,
                template,
                by_default: true,
                ..
            } => Some(SummaryItem::Defaulted {
                decision: *decision,
                template: template.clone(),
            }),
            K::ScandalBroke {
                scandal,
                politician,
                kind,
                ..
            } if *politician == me.0 => Some(SummaryItem::Scandal {
                scandal: *scandal,
                kind: kind.clone(),
            }),
            K::ChapterBegan { index, kind } => Some(SummaryItem::Chapter {
                index: *index,
                kind: kind.clone(),
            }),
            K::Disaster {
                municipality,
                hazard,
                ..
            } if *municipality == home => Some(SummaryItem::Disaster {
                hazard: format!("{hazard:?}").to_lowercase(),
            }),
            K::PlantClosure {
                municipality,
                sector,
                jobs,
                ..
            } if *municipality == home => Some(SummaryItem::PlantClosure {
                sector: sector.clone(),
                jobs: *jobs,
            }),
            K::PolicyExpired {
                policy,
                municipality,
            } if *municipality == home => Some(SummaryItem::LawEnded {
                policy: *policy,
                reason: "expired".into(),
            }),
            K::PolicyWithdrawn { policy, .. } if mine(*policy) => {
                Some(SummaryItem::BillWithdrawn { policy: *policy })
            }
            K::AskOffered { ask, template, .. } => Some(SummaryItem::AskOffered {
                ask: *ask,
                template: template.clone(),
            }),
            K::AskResolved {
                ask,
                template,
                outcome,
                ..
            } => Some(SummaryItem::AskResolved {
                ask: *ask,
                template: template.clone(),
                outcome: outcome.clone(),
            }),
            K::PlayerDeselected => Some(SummaryItem::Deselected),
            K::NominationRefused {
                post,
                reason,
                nominee,
                standing,
                min_standing,
                instead,
            } => Some(SummaryItem::NotNominated {
                post: *post,
                reason: reason.clone(),
                nominee: *nominee,
                standing: *standing,
                min_standing: *min_standing,
                instead: *instead,
            }),
            K::CaseOpened { case, politician } if *politician == me.0 => Some(SummaryItem::Case {
                case: *case,
                stage: "investigation".into(),
            }),
            K::CaseDeveloped {
                case,
                politician,
                stage,
            } if *politician == me.0 => Some(SummaryItem::Case {
                case: *case,
                stage: stage.clone(),
            }),
            K::PlayerImprisoned { years, .. } => Some(SummaryItem::Imprisoned { years: *years }),
            K::PlayerReleased => Some(SummaryItem::Released),
            K::RumourSpread { rumour, politician } if *politician == me.0 => {
                Some(SummaryItem::Rumour { rumour: *rumour })
            }
            K::GlobalRecessionBegan { .. } => Some(SummaryItem::Recession { began: true }),
            K::GlobalRecoveryBegan => Some(SummaryItem::Recession { began: false }),
            K::ProposalOpened {
                proposal,
                family,
                at,
            } if *at == crate::world::Jurisdiction::town(pl.home()) => {
                Some(SummaryItem::Proposal {
                    proposal: *proposal,
                    family: family.clone(),
                    opened: true,
                })
            }
            K::ProposalLapsed {
                proposal,
                family,
                at,
            } if *at == crate::world::Jurisdiction::town(pl.home()) => {
                Some(SummaryItem::Proposal {
                    proposal: *proposal,
                    family: family.clone(),
                    opened: false,
                })
            }
            _ => None,
        };
        if let Some(i) = item {
            push(run, i);
        }
    }
    run.cursor = last;
    // Crises have no event: compare the town's live ones with the last look.
    let now = home_crises(state);
    let before = run.crises.take().unwrap_or_else(|| now.clone());
    for k in now.iter().filter(|k| !before.contains(k)) {
        push(
            run,
            SummaryItem::Crisis {
                kind: k.clone(),
                began: true,
            },
        );
    }
    for k in before.into_iter().filter(|k| !now.contains(k)) {
        push(
            run,
            SummaryItem::Crisis {
                kind: k,
                began: false,
            },
        );
    }
    run.crises = Some(now);
}

/// Add an item, keeping the newest `ITEMS_CAP` (chapters go first).
pub fn push(run: &mut SkipRun, item: SummaryItem) {
    run.items.push(item);
    while run.items.len() > ITEMS_CAP {
        let i = run
            .items
            .iter()
            .position(|x| matches!(x, SummaryItem::Chapter { .. }))
            .unwrap_or(0);
        run.items.remove(i);
    }
}

/// Close a skip into its summary.
pub fn finish(
    run: SkipRun,
    state: &crate::State,
    content: &ContentBundle,
    stop: SkipStop,
) -> SkipSummary {
    SkipSummary {
        target: run.target,
        end: snapshot(state, content),
        start: run.start,
        turns: run.turns,
        stop,
        items: run.items,
    }
}
