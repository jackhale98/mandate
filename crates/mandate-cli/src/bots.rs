//! Bot players (M4, TEST_STRATEGY): scripted strategies play the player's
//! first local election through the same commands as the UI, across many
//! seeds. The gate ladder (ENGAGEMENT §7.2, DEC-097's balance principle)
//! compares them by class: single-verb routines < a blind mix < informed
//! play, with several informed styles that each win somewhere. Bots go where each
//! action is done before doing it, through the same auto-routing `Go`
//! command as the UI's agenda (DEC-079).
//!
//! Bots play as the player, so they know only what the player knows
//! (DEC-091): strategy code reads gated information (worries, support,
//! people, situational policies) only through [`Knowing`], never the true
//! state, and learns by the same commands. The blind-invariance test checks
//! it: perturbing truths a bot has not learned leaves its commands unchanged.

use mandate_content::{AiAction, ContentBundle};
use mandate_sim::actions::{Action, PlayerCommand};
use mandate_sim::characters::CharacterId;
use mandate_sim::places::Location;
use mandate_sim::player::knowledge::{Knowing, Level, PolicyRef};
use mandate_sim::policy::{PolicyStatus, ward_hoods};
use mandate_sim::world::NeighborhoodId;
use mandate_sim::{Command, CommandOutcome, Engine, EngineError, npc};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::Arc;

/// The lab switch (`--omniscient`, DEC-091 §1.10): every bot's player
/// reads every subject as `detailed`, for baselines. Process-wide for a lab
/// run; worlds stay independent and deterministic.
static OMNISCIENT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn set_omniscient(on: bool) {
    OMNISCIENT.store(on, std::sync::atomic::Ordering::Relaxed);
}

pub fn omniscient() -> bool {
    OMNISCIENT.load(std::sync::atomic::Ordering::Relaxed)
}

/// A bot's new game, with the lab switch applied.
fn new_game(content: &Arc<ContentBundle>, seed: u64) -> Option<Engine> {
    new_game_with(content, seed, omniscient())
}

/// A bot's new game, with the lab switch on or off for this game alone (the
/// `knowledge_matters` pass plays `mixed` omniscient beside the others).
fn new_game_with(content: &Arc<ContentBundle>, seed: u64, omniscient: bool) -> Option<Engine> {
    let mut e = generate(
        &WorldConfig {
            seed,
            size: WorldSize::Small,
            scenario: None,
        },
        content.clone(),
    )
    .ok()?;
    e.set_lab_omniscient(omniscient);
    Some(e)
}

pub const STRATEGIES: [&str; 13] = [
    "idle",
    "canvass",
    "media",
    "asks_only",
    "legislator",
    "mixed",
    "proxy",
    "corrupt",
    "explorer",
    "explorer_coalition",
    "explorer_legislator",
    "explorer_campaigner",
    "explore_only",
];

/// The strategy classes the gate ladder compares (ENGAGEMENT §2.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    /// One verb, repeated: `canvass`, `media`, `asks_only`.
    SingleVerb,
    /// A mix played without reading the town: `mixed`, `legislator`, `proxy`.
    BlindMix,
    /// Learn the town first, then lean on one way of playing: `explorer`
    /// and its three variants.
    Informed,
    /// Compared only by their own gates: `idle`, `explore_only`, `corrupt`.
    Probe,
}

impl Class {
    pub fn key(self) -> &'static str {
        match self {
            Class::SingleVerb => "single_verb",
            Class::BlindMix => "blind_mix",
            Class::Informed => "informed",
            Class::Probe => "probe",
        }
    }
}

pub fn class(s: &str) -> Class {
    match s {
        "canvass" | "media" | "asks_only" => Class::SingleVerb,
        "mixed" | "legislator" | "proxy" => Class::BlindMix,
        "explorer" | "explorer_coalition" | "explorer_legislator" | "explorer_campaigner" => {
            Class::Informed
        }
        _ => Class::Probe,
    }
}

/// Probes, not ways of playing: outside the class comparisons.
pub fn is_probe(s: &str) -> bool {
    class(s) == Class::Probe
}

/// Rote routines: single-verb routines and blind mixes together.
pub fn is_rote(s: &str) -> bool {
    matches!(class(s), Class::SingleVerb | Class::BlindMix)
}

/// The informed styles (each learns the town first, `EXPLORE_TURNS`).
pub fn is_informed(s: &str) -> bool {
    class(s) == Class::Informed
}

/// Turns `explorer` spends learning the town before it plays the mix.
pub const EXPLORE_TURNS: u32 = 10;

thread_local! {
    /// The commands bots send on this thread, while recording (the
    /// blind-invariance test).
    static RECORD: std::cell::RefCell<Option<Vec<String>>> = const { std::cell::RefCell::new(None) };
}

/// Start (or stop and return) recording the commands bots send on this
/// thread (the blind-invariance test; lab tools may record too).
#[cfg(test)]
pub fn record(on: bool) -> Vec<String> {
    RECORD.with(|r| {
        let mut r = r.borrow_mut();
        let out = r.take().unwrap_or_default();
        if on {
            *r = Some(Vec::new());
        }
        out
    })
}

thread_local! {
    /// The verbs bots used on this thread, counted while a game is played
    /// (the action mix the report prints).
    static MIX: std::cell::RefCell<BTreeMap<&'static str, u32>> =
        const { std::cell::RefCell::new(BTreeMap::new()) };
}

/// The verb a command counts as in the action mix (moves, meetings and
/// answers are not verbs).
fn verb(cmd: &PlayerCommand) -> Option<&'static str> {
    match cmd {
        PlayerCommand::Act { action, .. } => Some(action.key()),
        PlayerCommand::Go { then, .. } => then.as_deref().and_then(verb),
        PlayerCommand::Talk { topic, .. } => Some(match topic.as_str() {
            "topic.support" => "talk_support",
            "topic.pitch" | "topic.favour" => "talk_bill",
            _ => "talk",
        }),
        PlayerCommand::DraftPolicy { .. } => Some("draft"),
        PlayerCommand::IntroducePolicy { .. } => Some("introduce"),
        _ => None,
    }
}

/// Every command a bot sends goes through here.
fn apply(e: &mut Engine, cmd: Command) -> Result<CommandOutcome, EngineError> {
    RECORD.with(|r| {
        if let Some(v) = r.borrow_mut().as_mut() {
            v.push(serde_json::to_string(&cmd).unwrap_or_default());
        }
    });
    let v = match &cmd {
        Command::Player(p) => verb(p),
        _ => None,
    };
    let r = e.apply(cmd);
    if r.is_ok()
        && let Some(v) = v
    {
        MIX.with(|m| *m.borrow_mut().entry(v).or_default() += 1);
    }
    r
}

/// Take the action mix counted on this thread since the last take.
fn take_mix() -> BTreeMap<&'static str, u32> {
    MIX.with(|m| std::mem::take(&mut *m.borrow_mut()))
}

fn campaigning(e: &Engine) -> bool {
    let Some(me) = e.state().player.as_ref().map(|p| p.politician) else {
        return false;
    };
    e.state()
        .politics
        .elections
        .races
        .iter()
        .any(|r| r.candidates.iter().any(|c| c.politician == me))
}

fn act(e: &mut Engine, a: Action) -> bool {
    let cmd = e.route(PlayerCommand::Act {
        action: a,
        late_night: false,
    });
    apply(e, cmd).is_ok()
}

/// Canvass a neighbourhood on the worry the player knows it feels most
/// (none known: a general canvass).
fn canvass(e: &mut Engine, hood: u32) -> bool {
    let issue = Knowing::of(e).top_worry(NeighborhoodId(hood));
    act(
        e,
        Action::Canvass {
            neighborhood: hood,
            issue,
        },
    )
}

/// The ward neighbourhood where the player is least known.
fn coldest_hood(e: &Engine) -> Option<u32> {
    let st = e.state();
    let pl = st.player.as_ref()?;
    ward_hoods(&st.world, pl.ward)
        .into_iter()
        .min_by(|a, b| {
            let aw = |h: usize| {
                let cs = &st.population.hoods[h].cohorts;
                cs.iter()
                    .map(|c| c.player_awareness as f64 * c.count as f64)
                    .sum::<f64>()
                    / cs.iter().map(|c| c.count as f64).sum::<f64>().max(1.0)
            };
            aw(*a).total_cmp(&aw(*b))
        })
        .map(|h| h as u32)
}

fn own_bill(e: &Engine) -> Option<u32> {
    let st = e.state();
    let me = st.player.as_ref()?.politician;
    st.policy
        .instances
        .iter()
        .rev()
        .find(|p| p.sponsor == Some(me) && p.status == PolicyStatus::UnderConsideration)
        .map(|p| p.id.0)
}

/// A template's support in the player's ward as the player knows it (the
/// bill view's figure: coarse until learned).
fn known_ward_support(e: &Engine, template: &str) -> f64 {
    Knowing::of(e)
        .ward_support_of(&PolicyRef::Template {
            template: template.to_string(),
        })
        .unwrap_or(0.0)
}

/// Whom to lobby on the player's bill, from what the player can see (the
/// whip's exact leans are the chamber's, and they carry colleagues' hidden
/// goals): colleagues the player has worked on least about this bill (their
/// memories of the player are never gated), the other parties' first (the
/// player's own party follows its whip), then the best shown odds (which
/// carry the knowledge term: people the player knows well come first).
fn lobby_target(e: &Engine, content: &ContentBundle, bill: u32) -> Option<u32> {
    let st = e.state();
    let pl = st.player.as_ref()?;
    let mine = st.politics.get(pl.politician)?;
    let me = mine.character;
    let c = content.constitutions.get(&st.meta.constitution)?;
    let town = mandate_sim::world::Jurisdiction::town(pl.home());
    mandate_sim::politics::legislature_of(&st.politics, c, town)
        .iter()
        .filter(|id| **id != pl.politician)
        .filter_map(|id| st.politics.get(*id))
        .map(|p| {
            let worked = st
                .politics
                .characters
                .get(p.character)
                .map(|c| {
                    c.memories
                        .iter()
                        .filter(|m| {
                            m.actor == me
                                && m.subject
                                    == Some(mandate_sim::characters::Subject::Policy { id: bill })
                        })
                        .count()
                })
                .unwrap_or(0);
            let same = mine.party.is_some() && p.party == mine.party;
            let odds = mandate_sim::actions::lobby_odds(st, content, p.id, false);
            (worked, same, odds, p.id.0)
        })
        .min_by(|a, b| {
            a.0.cmp(&b.0)
                .then(a.1.cmp(&b.1))
                .then(b.2.total_cmp(&a.2))
                .then(a.3.cmp(&b.3))
        })
        .map(|x| x.3)
}

fn legislate(e: &mut Engine, content: &ContentBundle) {
    if own_bill(e).is_none() {
        let best = {
            let st = e.state();
            let Some(pl) = st.player.as_ref() else { return };
            let k = Knowing::of(e);
            let taken: Vec<&str> = st
                .policy
                .in_municipality(pl.municipality.0)
                .filter(|p| !matches!(p.status, PolicyStatus::Ended))
                .map(|p| p.template.as_str())
                .collect();
            // The menu as the player sees it, ranked by the ward support the
            // player knows (DEC-091: never the true figure).
            content
                .policies
                .values()
                .filter(|t| {
                    k.on_menu(t)
                        && !taken.contains(&t.id.as_str())
                        && mandate_sim::policy::unlocked(st, content, t, pl.municipality.0)
                })
                .map(|t| (known_ward_support(e, &t.id), t))
                .max_by(|a, b| a.0.total_cmp(&b.0))
                .map(|(_, t)| t.id.clone())
        };
        if let Some(t) = best {
            let cmd = e.route(PlayerCommand::DraftPolicy {
                template: t,
                params: BTreeMap::new(),
            });
            let r = apply(e, cmd);
            if let Some(id) = r.ok().and_then(|r| r.result).and_then(|x| x.created) {
                let cmd = e.route(PlayerCommand::IntroducePolicy { policy: id });
                let _ = apply(e, cmd);
            }
        }
    }
    if let Some(bill) = own_bill(e) {
        // Lobby a colleague, then talk it up in public.
        let target = lobby_target(e, content, bill);
        if let Some(t) = target {
            act(
                e,
                Action::Lobby {
                    politician: t,
                    policy: bill,
                },
            );
        }
        if let Some(h) = coldest_hood(e) {
            act(
                e,
                Action::TownHall {
                    neighborhood: h,
                    policy: Some(bill),
                },
            );
        }
    }
}

fn tired(e: &Engine) -> bool {
    e.state()
        .player
        .as_ref()
        .is_some_and(|p| p.tracks.health < 40.0)
}

fn buy_ads(e: &mut Engine, n: usize) {
    for _ in 0..n {
        if !act(e, Action::Advertise) {
            break;
        }
    }
}

/// How a strategy weighs an option's effects (DEC-073): a simple,
/// deterministic policy per strategy for answering decisions.
struct Taste {
    opinion: f64,
    approval: f64,
    awareness: f64,
    money: f64,
    health: f64,
    slot: f64,
    standing: f64,
    legal: f64,
    record: f64,
    owe: f64,
    draft: f64,
    /// A borderline or illegal option (its risk): honest strategies never.
    shady: f64,
    /// Preferred answers to a story and to an investigation.
    deny: bool,
    blame: bool,
}

fn taste(strategy: &str) -> Taste {
    let base = Taste {
        opinion: 0.05,
        approval: 60.0,
        awareness: 20.0,
        money: 0.0003,
        health: 0.2,
        slot: -1.0,
        standing: 0.4,
        legal: -0.5,
        record: 20.0,
        owe: -0.5,
        draft: 0.3,
        shady: -1000.0,
        deny: false,
        blame: false,
    };
    match strategy {
        "canvass" => Taste {
            approval: 100.0,
            awareness: 50.0,
            slot: -1.5,
            ..base
        },
        "media" => Taste {
            money: 0.001,
            approval: 30.0,
            standing: 0.6,
            slot: -1.2,
            ..base
        },
        "legislator" => Taste {
            draft: 2.0,
            record: 40.0,
            standing: 0.6,
            approval: 40.0,
            ..base
        },
        "mixed" => Taste {
            approval: 80.0,
            opinion: 0.08,
            standing: 0.5,
            health: 0.3,
            slot: -0.8,
            legal: -1.0,
            draft: 0.5,
            ..base
        },
        // Mixed play, plus every borderline or illegal shortcut on offer
        // (measures that corruption tempts but does not dominate).
        "corrupt" => Taste {
            approval: 80.0,
            opinion: 0.08,
            standing: 0.5,
            health: 0.3,
            slot: -0.8,
            money: 0.001,
            legal: -0.2,
            draft: 0.5,
            shady: -0.5,
            deny: true,
            blame: true,
            ..base
        },
        // The informed styles weigh what their way of playing builds on.
        "explorer_campaigner" => Taste {
            approval: 90.0,
            awareness: 40.0,
            standing: 0.6,
            ..base
        },
        "explorer_legislator" => Taste {
            draft: 1.5,
            record: 35.0,
            standing: 0.7,
            ..base
        },
        "explorer_coalition" => Taste {
            opinion: 0.15,
            owe: -0.25,
            standing: 0.7,
            ..base
        },
        // The NPC AI's pragmatist (the time-skip proxy; `explorer` too).
        _ => Taste {
            standing: 0.8,
            opinion: 0.1,
            ..base
        },
    }
}

fn option_score(
    e: &Engine,
    content: &ContentBundle,
    d: &mandate_sim::narrative::Decision,
    o: &mandate_content::OptionDef,
    t: &Taste,
) -> f64 {
    use mandate_content::{EffectDef, VoteChoice};
    let st = e.state();
    let r = mandate_sim::scheduler::Reads::of_state(st, content);
    let v = mandate_sim::narrative::cast::View::new(&r, &st.politics);
    let k = &content.actions.vote;
    o.effects
        .iter()
        .map(|x| match x {
            EffectDef::Opinion { delta, .. } => t.opinion * delta,
            EffectDef::Approval { delta, .. } => t.approval * delta,
            EffectDef::Awareness { delta, .. } => t.awareness * delta,
            EffectDef::Money { amount } => t.money * amount,
            EffectDef::Health { delta } => t.health * delta,
            EffectDef::Slots { n } => t.slot * *n as f64,
            EffectDef::Standing { delta } => t.standing * delta,
            EffectDef::LegalJeopardy { delta } => t.legal * delta,
            EffectDef::Record { delta } => t.record * delta,
            EffectDef::Favour { owed_to_player, .. } => {
                if *owed_to_player {
                    -t.owe
                } else {
                    t.owe
                }
            }
            EffectDef::Repay { .. } => 0.0,
            // Knowledge never sways a bot's choice (DEC-091: it costs
            // effectiveness, never fairness; K4 adds the effectiveness).
            EffectDef::Learn { .. } | EffectDef::RevealPlace { .. } => 0.0,
            // A rider or a favour called in helps a bill pass; a favour
            // called in is spent.
            EffectDef::Rider { .. } => 0.5 * t.draft,
            EffectDef::CallIn { .. } => 0.5 * t.draft + t.owe,
            EffectDef::Draft { .. } => t.draft,
            EffectDef::Hidden { .. } => t.shady,
            EffectDef::ScandalResponse { response } => {
                use mandate_content::ScandalResponseKind as R;
                match response {
                    R::Deny if t.deny => 3.0,
                    R::Explain => 2.0,
                    R::CounterAttack => 1.0,
                    R::Apologise => 0.5,
                    _ => 0.0,
                }
            }
            EffectDef::LegalResponse { response, .. } => {
                use mandate_content::LegalResponseKind as R;
                match response {
                    R::BlameAlly if t.blame => 4.0,
                    R::Lawyer => 3.0,
                    R::Cooperate => 1.0,
                    _ => 0.0,
                }
            }
            EffectDef::Vote { role, vote } => {
                let Some(mandate_sim::narrative::Actor::Policy { id }) =
                    mandate_sim::narrative::actor_of(&d.cast, role)
                else {
                    return 0.0;
                };
                let line = v.party_line(id) as f64;
                // The ward's support as the bill view shows it.
                let lean = st
                    .policy
                    .instances
                    .get(id as usize)
                    .map(|p| known_ward_support(e, &p.template))
                    .unwrap_or(0.0);
                let sign = match vote {
                    VoteChoice::Yes => 1.0,
                    VoteChoice::No => -1.0,
                    VoteChoice::Abstain => 0.0,
                    VoteChoice::WithParty => line,
                    VoteChoice::WithWard => lean.signum(),
                };
                let standing = if sign == 0.0 {
                    k.standing_abstain
                } else if line == 0.0 {
                    0.0
                } else if line == sign {
                    k.standing_loyal
                } else {
                    k.standing_defy
                };
                t.standing * standing + t.approval * k.ward_reaction * lean * sign
            }
        })
        .sum()
}

/// Answer every pending decision by the strategy's taste (idle never does:
/// its decisions take their defaults at the deadline).
pub fn answer_decisions(e: &mut Engine, content: &ContentBundle, strategy: &str) {
    if strategy == "idle" {
        return;
    }
    let t = taste(strategy);
    let pending: Vec<mandate_sim::narrative::Decision> =
        e.state().narrative.pending().cloned().collect();
    for d in pending {
        let Some(def) = content.dilemma(&d.template) else {
            continue;
        };
        let mut opts: Vec<(f64, &str)> = def
            .options
            .iter()
            .map(|o| (option_score(e, content, &d, o, &t), o.id.as_str()))
            .collect();
        opts.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(b.1)));
        for (_, id) in opts {
            let cmd = Command::Player(PlayerCommand::Answer {
                decision: d.id.0,
                option: id.to_string(),
            });
            if apply(e, cmd).is_ok() {
                break;
            }
        }
    }
}

/// The corrupt strategy's shady actions (DEC-034): an envelope at the diner
/// every eighteen months or so, the campaign card instead of rest when worn
/// out (as often), and a rival's file leaked once a campaign.
fn shady_actions(e: &mut Engine, camp: bool) {
    let st = e.state();
    let Some(pl) = st.player.as_ref() else { return };
    let turn = st.calendar.turn;
    // Its own hidden acts of a kind within `days` (the facts it left behind).
    let recent = |kind: &str, days: u32| {
        st.narrative.hidden.iter().any(|h| {
            h.subject == Some(pl.politician)
                && h.kind == kind
                && h.date.days_until(st.calendar.date) < days
        })
    };
    let money = pl.money.units();
    let tired = pl.tracks.health < 45.0;
    let bribe = !recent("bribe", 540) && !pl.late_night_used && turn > 2;
    let dip = tired && money > 2500 && !recent("campaign_funds", 540);
    let leak = camp && !recent("leaked_file", 300);
    let go = |e: &mut Engine, a: Action, late: bool| {
        let cmd = e.route(PlayerCommand::Act {
            action: a,
            late_night: late,
        });
        let _ = apply(e, cmd);
    };
    if bribe {
        go(e, Action::TakeBribe, true);
    }
    if dip {
        go(e, Action::DipFunds, false);
    }
    if leak {
        go(e, Action::LeakFile, false);
    }
}

// --- Learning the town (DEC-091) --------------------------------------------------

fn slots_left(e: &Engine) -> u8 {
    e.state()
        .player
        .as_ref()
        .map(|p| p.slots_left())
        .unwrap_or(0)
}

/// Where someone is now: the engine's answer (their daily routine,
/// DEC-031), when it is somewhere the player can go.
fn whereabouts(e: &Engine, person: CharacterId) -> Option<Location> {
    let st = e.state();
    st.player.as_ref()?;
    mandate_sim::characters::routines::whereabouts(st, e.content(), person)
        .filter(|l| mandate_sim::places::validate_move(st, *l).is_ok())
}

fn move_to(e: &mut Engine, location: Location) -> bool {
    if e.state().player.as_ref().map(|p| p.location) == Some(location) {
        return true;
    }
    apply(e, Command::Player(PlayerCommand::MoveTo { location })).is_ok()
}

/// Go to someone and talk to them on a topic (DEC-093, half a slot).
fn talk(e: &mut Engine, person: CharacterId, topic: &str) -> bool {
    let Some(loc) = whereabouts(e, person) else {
        return false;
    };
    if !move_to(e, loc) {
        return false;
    }
    let cmd = Command::Player(PlayerCommand::Talk {
        to: mandate_sim::talk::TalkTarget::Person { person },
        topic: topic.into(),
        about: None,
        late_night: false,
    });
    apply(e, cmd).is_ok()
}

/// Walk the town: every place not yet visited (free; each first visit
/// gives its introduction and the names of the people there).
fn tour(e: &mut Engine) {
    let todo: Vec<Location> = {
        let st = e.state();
        let Some(town) = st.player.as_ref().map(|p| p.municipality) else {
            return;
        };
        let k = Knowing::of(e);
        st.world
            .places
            .iter()
            .filter(|p| p.municipality == town && !k.visited(p.id))
            .map(|p| Location::Place { place: p.id })
            .collect()
    };
    for l in todo {
        move_to(e, l);
    }
}

/// A ward neighbourhood whose worries the player has not yet read in
/// detail, coldest first.
fn unread_hood(e: &Engine) -> Option<u32> {
    let st = e.state();
    let pl = st.player.as_ref()?;
    let k = Knowing::of(e);
    let unread = |h: &usize| {
        k.level(&mandate_sim::player::knowledge::Subject::HoodConcerns {
            hood: NeighborhoodId(*h as u32),
        }) < Level::Detailed
    };
    let mut ward = ward_hoods(&st.world, pl.ward);
    ward.retain(unread);
    let aw = |h: usize| {
        let cs = &st.population.hoods[h].cohorts;
        cs.iter()
            .map(|c| c.player_awareness as f64 * c.count as f64)
            .sum::<f64>()
            / cs.iter().map(|c| c.count as f64).sum::<f64>().max(1.0)
    };
    ward.sort_by(|a, b| aw(*a).total_cmp(&aw(*b)).then(a.cmp(b)));
    ward.first().map(|h| *h as u32)
}

/// Someone whose name the player has heard but has not met properly: the
/// council first, then everyone else (in id order).
fn unmet(e: &Engine) -> Option<CharacterId> {
    let st = e.state();
    let me = mandate_sim::characters::player_character(st);
    let k = Knowing::of(e);
    let council = mandate_sim::player::knowledge::council_people(st);
    let named = st
        .politics
        .characters
        .people
        .iter()
        // The people of places with purpose (seeded in a later round, PL1)
        // are met where they work, by the visit verbs (the coalition
        // builder's learning visits), not chased one by one in the exploring
        // turns: chasing them all spent the campaigner's early groundwork
        // and tilted the styles' variety (BALANCE_LOG, PL1).
        .filter(|c| {
            c.named.as_deref().is_some_and(|n| {
                e.content()
                    .occupation(&n.occupation)
                    .is_none_or(|o| o.round == 0)
            })
        })
        .map(|c| c.id);
    council
        .into_iter()
        .chain(named)
        .find(|c| Some(*c) != me && k.person(*c) == Level::Heard && whereabouts(e, *c).is_some())
}

/// Walk neighbourhood streets (the ward's; with `all`, the town's) and say
/// hello to the passers-by (free, `Meet`): residents met become people the
/// player knows.
fn meet_passers_by(e: &mut Engine, content: &ContentBundle, all: bool) {
    let hoods: Vec<u32> = {
        let st = e.state();
        let Some(pl) = st.player.as_ref() else { return };
        if all {
            st.world
                .hoods_of(pl.municipality)
                .map(|h| h as u32)
                .collect()
        } else {
            ward_hoods(&st.world, pl.ward)
                .into_iter()
                .map(|h| h as u32)
                .collect()
        }
    };
    for h in hoods {
        let loc = Location::Street {
            neighborhood: NeighborhoodId(h),
        };
        let slots: Vec<u8> = mandate_sim::characters::ambient::ambient_at(e.state(), content, loc)
            .into_iter()
            .map(|x| x.0)
            .collect();
        if slots.is_empty() || !move_to(e, loc) {
            continue;
        }
        for slot in slots {
            let _ = apply(e, Command::Player(PlayerCommand::Meet { slot }));
        }
    }
}

/// What a verb would teach about a place that the player does not know
/// yet: its industry, its site or its service's need (places with
/// purpose). Read through `Knowing`, as everything a bot learns from.
fn unlearned(e: &Engine, place: mandate_sim::world::PlaceId, verb: &str) -> bool {
    use mandate_content::knowledge::GrantSubject as G;
    use mandate_content::visits::VisitEffect;
    use mandate_sim::player::knowledge::Subject;
    let st = e.state();
    let content = e.content();
    let (Some(def), Some(p)) = (
        content.actions.visit.verb(verb),
        st.world.places.get(place.index()),
    ) else {
        return false;
    };
    let k = Knowing::of(e);
    let kind = content.place_kind(p.kind.key());
    def.effects.iter().any(|fx| {
        let VisitEffect::Learn { grants } = fx else {
            return false;
        };
        grants.iter().any(|g| {
            let s = match g.subject {
                G::Sector => kind
                    .and_then(|d| d.sector.clone())
                    .map(|sector| Subject::Sector {
                        town: p.municipality,
                        sector,
                    }),
                G::Site => Some(Subject::Site { place }),
                G::Service => Some(Subject::Service { place }),
                _ => None,
            };
            s.is_some_and(|s| k.level(&s) < Level::Known)
        })
    })
}

/// Visit a place to learn what it teaches (places with purpose): the first
/// place in town, in id order, with a verb that teaches something not yet
/// known and can be done now. Returns whether it was done.
fn learning_visit(e: &mut Engine) -> bool {
    let pick = {
        let st = e.state();
        let content = e.content();
        let Some(town) = mandate_sim::player::home(st) else {
            return false;
        };
        st.world
            .places
            .iter()
            .filter(|p| p.municipality == town && p.is_open())
            .find_map(|p| {
                let loc = Location::Place { place: p.id };
                mandate_sim::visits::verbs_at(st, content, loc)
                    .into_iter()
                    .find(|v| {
                        unlearned(e, p.id, &v.id)
                            && mandate_sim::visits::availability(st, content, loc, v, false).is_ok()
                    })
                    .map(|v| (loc, v.id.clone()))
            })
    };
    let Some((location, verb)) = pick else {
        return false;
    };
    let then = PlayerCommand::Act {
        action: Action::Visit { verb },
        late_night: false,
    };
    let cmd = if e.state().player.as_ref().map(|p| p.location) == Some(location) {
        then
    } else {
        PlayerCommand::Go {
            location,
            then: Some(Box::new(then)),
        }
    };
    apply(e, Command::Player(cmd)).is_ok()
}

/// One turn of learning: walk the town and the streets, read the ward's worries on the
/// doorsteps, and meet the people named so far, asking who really runs
/// things while that is a rumour. Returns whether any time went on it.
/// With `all` (the `explore_only` probe) it only finds out: walks, meets and
/// asks, and never campaigns (canvassing teaches too, but it is the ground
/// game), and nothing else is ever done. With `visits` (the coalition
/// builder, who goes where people work) it also visits the works, fields
/// and services to learn what they teach and meet the people there.
fn explore(e: &mut Engine, content: &ContentBundle, all: bool, visits: bool) -> bool {
    tour(e);
    meet_passers_by(e, content, all);
    let mut busy = false;
    while slots_left(e) > 0 {
        let ok = if !all
            && !tired(e)
            && slots_left(e) >= 2
            && let Some(h) = unread_hood(e)
        {
            canvass(e, h)
        } else if let Some(p) = unmet(e) {
            // Who really runs things, while that is still a rumour.
            let topic = if Knowing::of(e).broker().is_none() {
                "topic.who_runs"
            } else {
                "topic.worries"
            };
            talk(e, p, topic) || talk(e, p, "topic.worries")
        } else if visits && !all {
            // The town's works, fields and services (places with purpose).
            learning_visit(e)
        } else {
            false
        };
        if !ok {
            break;
        }
        busy = true;
    }
    busy
}

/// Ask someone the player has met properly, who lives in the ward and is
/// not a politician, to speak up for them (DEC-093 `ask_support`), the best
/// shown odds first, each person once; at most two (a slot). Returns whether
/// anyone was asked.
fn ask_support(e: &mut Engine, content: &ContentBundle) -> bool {
    let mut asked = false;
    for _ in 0..2 {
        let pick = {
            let st = e.state();
            let Some(pl) = st.player.as_ref() else {
                return asked;
            };
            let ward = ward_hoods(&st.world, pl.ward);
            let k = Knowing::of(e);
            let me = mandate_sim::characters::player_character(st);
            st.politics
                .characters
                .people
                .iter()
                .filter(|c| Some(c.id) != me && c.politician.is_none())
                .filter(|c| {
                    c.named
                        .as_deref()
                        .is_some_and(|n| ward.contains(&(n.home as usize)))
                })
                .filter(|c| k.person(c.id) >= Level::Known)
                .filter(|c| {
                    !pl.talk
                        .log
                        .iter()
                        .any(|x| x.speaker == c.id && x.topic == "topic.support")
                })
                .filter(|c| whereabouts(e, c.id).is_some())
                .filter_map(|c| {
                    // The odds the talk panel shows (DEC-073).
                    let odds =
                        mandate_sim::talk::odds(st, content, c.id, "topic.support", None, false)?;
                    (odds >= SUPPORT_MIN_ODDS).then_some((odds, c.id))
                })
                .max_by(|a, b| a.0.total_cmp(&b.0).then(b.1.cmp(&a.1)))
                .map(|x| x.1)
        };
        let Some(person) = pick else { break };
        if !talk(e, person, "topic.support") {
            break;
        }
        asked = true;
    }
    asked
}

/// Bots ask for support only when the shown odds are at least this.
pub const SUPPORT_MIN_ODDS: f64 = 0.3;

// --- The informed styles (ENGAGEMENT §2.4) ----------------------------------------

/// The fit the player expects from canvassing a neighbourhood on its top
/// worry, from their own last reading of its doorsteps (a `detailed`
/// reading is the player's knowledge, DEC-091): the canvass match formula
/// on the reading's top worry over its mean. Without a reading, 1.
fn expected_fit(e: &Engine, content: &ContentBundle, hood: u32) -> f64 {
    let Some(pl) = e.state().player.as_ref() else {
        return 1.0;
    };
    let s = mandate_sim::player::knowledge::Subject::HoodConcerns {
        hood: NeighborhoodId(hood),
    };
    let Some(r) = pl.knowledge.entry(&s).and_then(|x| x.reading.as_ref()) else {
        return 1.0;
    };
    let vals: Vec<f64> = r.values.iter().map(|v| v.1 as f64).collect();
    let mean = vals.iter().sum::<f64>() / vals.len().max(1) as f64;
    let top = vals.iter().copied().fold(0.0, f64::max);
    if mean <= 0.0 {
        return 1.0;
    }
    let k = &content.knowledge.effects.canvass;
    (1.0 + k.match_bonus * (top / mean - 1.0)).clamp(k.floor, k.ceiling)
}

/// Mean awareness of the player in a neighbourhood (never gated).
fn hood_awareness(e: &Engine, h: usize) -> f64 {
    let cs = &e.state().population.hoods[h].cohorts;
    cs.iter()
        .map(|c| c.player_awareness as f64 * c.count as f64)
        .sum::<f64>()
        / cs.iter().map(|c| c.count as f64).sum::<f64>().max(1.0)
}

/// The ward neighbourhood where a canvass is worth most now, by what the
/// player knows: how fresh its doorsteps are (their own memory of where
/// they have been), the fit they expect on its top worry, and how far they
/// are from being known there. Returns the hood and its approval
/// multiplier (fresh × season).
fn ground_hood(e: &Engine, content: &ContentBundle) -> Option<(u32, f64)> {
    let st = e.state();
    let pl = st.player.as_ref()?;
    let fx = &content.actions.canvass;
    ward_hoods(&st.world, pl.ward)
        .into_iter()
        .map(|h| {
            let c = mandate_sim::player::ground::canvass_curve(st, fx, NeighborhoodId(h as u32));
            let gap = 1.0 - hood_awareness(e, h);
            let value = c.approval * expected_fit(e, content, h as u32) + gap * c.awareness;
            (value, h as u32, c.approval)
        })
        .max_by(|a, b| a.0.total_cmp(&b.0).then(b.1.cmp(&a.1)))
        .map(|x| (x.1, x.2))
}

/// The campaigner (the ground game where it fits, and the press): between
/// elections it canvasses only doorsteps that are fresh (where it pays
/// most, by what the player knows), and otherwise works a bill and raises
/// money for the campaign; in a campaign, the doorsteps where a canvass is
/// worth most, then people it has met asked to speak up, then the press.
fn campaigner_turn(e: &mut Engine, content: &ContentBundle, camp: bool, slots: u8) -> bool {
    if tired(e) {
        return act(e, Action::Rest);
    }
    if slots >= 2
        && let Some((h, worth)) = ground_hood(e, content)
        && (camp || worth >= CAMPAIGNER_FRESH)
    {
        return canvass(e, h);
    }
    if camp {
        let policy = own_bill(e);
        return ask_support(e, content) || act(e, Action::Press { policy });
    }
    legislate(e, content);
    // Between elections, the odd slot goes on being seen at a works
    // gate where that is safe (places with purpose), else on the donors.
    photo_op(e, content) || act(e, Action::Fundraise)
}

/// Odds of a photo op going wrong the campaigner accepts.
pub const CAMPAIGNER_PHOTO_RISK: f64 = 0.15;

/// A photo op at a works gate in the player's ward, with the odds shown
/// at most `CAMPAIGNER_PHOTO_RISK` (the shown odds, DEC-073), once a turn
/// (the player's own log says whether there was one today).
fn photo_op(e: &mut Engine, content: &ContentBundle) -> bool {
    let verb = "verb.photo_op_gate";
    let pick = {
        let st = e.state();
        let Some(pl) = st.player.as_ref() else {
            return false;
        };
        let today = st.calendar.date;
        if pl
            .actions
            .iter()
            .any(|a| a.date == today && a.action == "action-verb-photo-op-gate")
        {
            return false;
        }
        let Some(def) = content.actions.visit.verb(verb) else {
            return false;
        };
        let ward = ward_hoods(&st.world, pl.ward);
        st.world
            .places
            .iter()
            .filter(|p| p.municipality == pl.home() && ward.contains(&p.neighborhood.index()))
            .map(|p| Location::Place { place: p.id })
            .find(|loc| {
                let Location::Place { place } = *loc else {
                    return false;
                };
                mandate_sim::visits::verbs_at(st, content, *loc)
                    .iter()
                    .any(|v| v.id == verb)
                    && mandate_sim::visits::availability(st, content, *loc, def, false).is_ok()
                    && mandate_sim::visits::risk(st, content, place, def)
                        .is_some_and(|r| r <= CAMPAIGNER_PHOTO_RISK)
            })
    };
    let Some(location) = pick else {
        return false;
    };
    let then = PlayerCommand::Act {
        action: Action::Visit { verb: verb.into() },
        late_night: false,
    };
    let cmd = if e.state().player.as_ref().map(|p| p.location) == Some(location) {
        then
    } else {
        PlayerCommand::Go {
            location,
            then: Some(Box::new(then)),
        }
    };
    apply(e, Command::Player(cmd)).is_ok()
}

/// Between elections the campaigner canvasses only doorsteps at least this
/// fresh.
pub const CAMPAIGNER_FRESH: f64 = 0.7;

/// The legislator (bills and delivery): always a bill in council, lobbied,
/// taken to a town hall and to the press; in a campaign, the doorsteps,
/// then people it has met asked to speak up, then a town hall on its bill
/// where the player is least known.
fn legislator_turn(e: &mut Engine, content: &ContentBundle, camp: bool, slots: u8) -> bool {
    if tired(e) {
        return act(e, Action::Rest);
    }
    if camp {
        if slots >= 2
            && let Some(h) = coldest_hood(e)
        {
            return canvass(e, h);
        }
        if ask_support(e, content) {
            return true;
        }
        return coldest_hood(e).is_some_and(|h| {
            act(
                e,
                Action::TownHall {
                    neighborhood: h,
                    policy: own_bill(e),
                },
            )
        });
    }
    legislate(e, content);
    let policy = own_bill(e);
    act(e, Action::Press { policy }) || act(e, Action::Fundraise)
}

/// The coalition builder (talk, lobbying, favours): it meets the ward's
/// residents on its walks, lobbies colleagues on its bill, and asks the
/// people it has met to speak up for it, between elections and in a
/// campaign (the asks first, then the doorsteps).
fn coalition_turn(e: &mut Engine, content: &ContentBundle, camp: bool, slots: u8) -> bool {
    if tired(e) {
        return act(e, Action::Rest);
    }
    meet_passers_by(e, content, false);
    if camp {
        if ask_support(e, content) {
            return true;
        }
        return slots >= 2 && coldest_hood(e).is_some_and(|h| canvass(e, h));
    }
    if own_bill(e).is_none() {
        legislate(e, content);
    } else if let Some(bill) = own_bill(e)
        && let Some(t) = lobby_target(e, content, bill)
    {
        act(
            e,
            Action::Lobby {
                politician: t,
                policy: bill,
            },
        );
    }
    ask_support(e, content) || act(e, Action::Fundraise)
}

/// Whether `explorer` is still in its learning turns.
fn exploring(e: &Engine, start_turn: u32) -> bool {
    e.state().calendar.turn < start_turn + EXPLORE_TURNS && !campaigning(e)
}

/// A bot's turn; `start_turn` is the turn play began (for `explorer`).
pub fn play_turn_from(e: &mut Engine, content: &ContentBundle, strategy: &str, start_turn: u32) {
    if strategy == "explore_only" {
        // Only learning: no decisions answered, no campaigning, no rest.
        if e.state().player.as_ref().is_some_and(|p| p.status.active()) {
            explore(e, content, true, false);
        }
        return;
    }
    answer_decisions(e, content, strategy);
    if e.state()
        .player
        .as_ref()
        .is_some_and(|p| !p.status.active())
    {
        return;
    }
    let camp = campaigning(e);
    if strategy == "corrupt" {
        shady_actions(e, camp);
    }
    // Money, not time: ads take no slot.
    if camp
        && matches!(
            strategy,
            "media" | "mixed" | "corrupt" | "explorer" | "explorer_campaigner"
        )
    {
        buy_ads(e, 3);
    }
    if camp && matches!(strategy, "explorer_coalition" | "explorer_legislator") {
        buy_ads(e, 1);
    }
    // The informed styles' first turns go on learning the town; then each
    // plays its own way (`explorer`: the mix).
    if is_informed(strategy) && exploring(e, start_turn) {
        explore(e, content, false, strategy == "explorer_coalition");
    }
    for _ in 0..3 {
        let slots = e
            .state()
            .player
            .as_ref()
            .map(|p| p.slots_left())
            .unwrap_or(0);
        if slots == 0 {
            break;
        }
        let ok = match strategy {
            // `asks_only` works every ask and nothing else: a stub until the
            // asks land (E6–E8), so it only answers decisions.
            "idle" | "asks_only" => false,
            "explorer_campaigner" => campaigner_turn(e, content, camp, slots),
            "explorer_legislator" => legislator_turn(e, content, camp, slots),
            "explorer_coalition" => coalition_turn(e, content, camp, slots),
            "canvass" => {
                if tired(e) {
                    act(e, Action::Rest)
                } else {
                    coldest_hood(e).is_some_and(|h| canvass(e, h))
                }
            }
            "media" => act(e, Action::Fundraise),
            "legislator" => {
                legislate(e, content);
                coldest_hood(e).is_some_and(|h| {
                    act(
                        e,
                        Action::TownHall {
                            neighborhood: h,
                            policy: None,
                        },
                    )
                })
            }
            "mixed" | "corrupt" | "explorer" => {
                if tired(e) {
                    act(e, Action::Rest)
                } else if camp {
                    // A canvass takes two slots; the odd one goes on asking
                    // people the player has met for their support.
                    if slots >= 2 {
                        coldest_hood(e).is_some_and(|h| canvass(e, h))
                    } else {
                        ask_support(e, content)
                    }
                } else {
                    legislate(e, content);
                    act(e, Action::Fundraise)
                }
            }
            "proxy" => {
                // The NPC AI drives the player (the time-skip proxy, DEC-012/DEC-030).
                let Some(me) = e.state().player.as_ref().map(|p| p.politician) else {
                    return;
                };
                let x = npc::inputs(e.state(), content, me);
                let scored = npc::score(content, "pragmatist", &x);
                match scored.first().map(|s| s.0) {
                    Some(AiAction::Canvass) => coldest_hood(e).is_some_and(|h| canvass(e, h)),
                    Some(AiAction::TownHall) => coldest_hood(e).is_some_and(|h| {
                        act(
                            e,
                            Action::TownHall {
                                neighborhood: h,
                                policy: None,
                            },
                        )
                    }),
                    Some(AiAction::Fundraise) => act(e, Action::Fundraise),
                    Some(AiAction::ProposeBill) | Some(AiAction::Lobby) => {
                        legislate(e, content);
                        true
                    }
                    _ => act(e, Action::Rest),
                }
            }
            _ => false,
        };
        if !ok {
            break;
        }
    }
}

/// Play one seed's first election with a strategy. Returns (won, share).
pub fn play(content: &Arc<ContentBundle>, seed: u64, strategy: &str) -> Option<(bool, f64)> {
    play_full(content, seed, strategy).map(|p| (p.won, p.share))
}

/// One bot's first election, with the situation its town started in.
pub struct Played {
    pub won: bool,
    pub share: f64,
    /// The player's town when play began (DEC-077 §5): its live crisis (or
    /// `none`) and its defining industry (or `mixed`).
    pub crisis: String,
    pub industry: String,
    /// The verbs used up to the election (the action mix).
    pub mix: BTreeMap<&'static str, u32>,
}

/// The player's town's situation when play begins.
fn situation(e: &Engine) -> (String, String) {
    let st = e.state();
    let Some(t) = st
        .player
        .as_ref()
        .and_then(|p| st.politics.characters.towns.get(p.municipality.index()))
    else {
        return ("none".into(), "mixed".into());
    };
    (
        t.crisis()
            .map(|c| c.kind.trim_start_matches("crisis.").to_string())
            .unwrap_or_else(|| "none".into()),
        t.industry
            .as_deref()
            .map(|s| s.trim_start_matches("sector.").to_string())
            .unwrap_or_else(|| "mixed".into()),
    )
}

pub fn play_full(content: &Arc<ContentBundle>, seed: u64, strategy: &str) -> Option<Played> {
    play_full_with(content, seed, strategy, omniscient())
}

/// One bot's first election, with the lab switch on or off for this game.
pub fn play_full_with(
    content: &Arc<ContentBundle>,
    seed: u64,
    strategy: &str,
    omniscient: bool,
) -> Option<Played> {
    let mut e = new_game_with(content, seed, omniscient)?;
    take_mix();
    let (crisis, industry) = situation(&e);
    let lost = |crisis: String, industry: String| Played {
        won: false,
        share: 0.0,
        crisis,
        industry,
        mix: take_mix(),
    };
    let me = e.state().player.as_ref()?.politician;
    let start = e.state().calendar.date;
    let start_turn = e.state().calendar.turn;
    for _ in 0..80 {
        if e.state()
            .politics
            .elections
            .last_election
            .is_some_and(|d| d > start)
        {
            break;
        }
        play_turn_from(&mut e, content, strategy, start_turn);
        if apply(&mut e, Command::EndTurn).is_err() {
            // The career ended (DEC-006): the election is lost.
            return Some(lost(crisis, industry));
        }
    }
    let st = e.state();
    let date = st.politics.elections.last_election?;
    let Some(r) = st
        .politics
        .elections
        .results
        .iter()
        .find(|r| r.date == date && r.candidates.iter().any(|c| c.politician == me))
    else {
        // Not on the ballot (in prison, or deselected): a loss.
        return Some(lost(crisis, industry));
    };
    let share = r.candidates.iter().find(|c| c.politician == me)?.share as f64;
    Some(Played {
        won: r.winner == me,
        share,
        crisis,
        industry,
        mix: take_mix(),
    })
}

/// Per world, the best of a set of strategies are those within
/// `VARIETY_TIE` of the highest vote share (a near-tie is shared).
pub const VARIETY_TIE: f64 = 0.002;

/// `good_styles_vary` (ENGAGEMENT §7.2; DEC-077 §5 as amended by DEC-097):
/// among the informed styles, (a) at least `STYLES_MIN_WINNERS` whose mean
/// share is within `STYLES_NEAR` of the best style's are each the sole best
/// in at least `STYLES_MIN_SOLE` of the worlds; (b) none is among the best
/// in more than `STYLES_MAX_SHARE` of the worlds; (c) the best style differs
/// by starting situation (crisis or industry).
pub const STYLES_MIN_WINNERS: usize = 2;
pub const STYLES_NEAR: f64 = 0.015;
pub const STYLES_MIN_SOLE: f64 = 0.2;
pub const STYLES_MAX_SHARE: f64 = 0.6;

/// The best of `among` by situation: the highest mean share in each group
/// of worlds (`key` picks the group: crisis or industry).
fn best_by(
    shares: &BTreeMap<u64, BTreeMap<&'static str, f64>>,
    situations: &BTreeMap<u64, (String, String)>,
    among: &[&'static str],
    key: impl Fn(&(String, String)) -> String,
) -> Vec<serde_json::Value> {
    let mut groups: BTreeMap<String, BTreeMap<&'static str, (f64, u32)>> = BTreeMap::new();
    for (seed, row) in shares {
        let k = situations
            .get(seed)
            .map(&key)
            .unwrap_or_else(|| "none".into());
        let g = groups.entry(k).or_default();
        for s in among {
            if let Some(v) = row.get(s) {
                let x = g.entry(s).or_default();
                x.0 += v;
                x.1 += 1;
            }
        }
    }
    groups
        .iter()
        .map(|(k, m)| {
            let best = among
                .iter()
                .filter_map(|s| m.get(s).map(|(t, c)| (*s, t / (*c).max(1) as f64, *c)))
                .fold(None::<(&str, f64, u32)>, |b, x| match b {
                    Some(b) if b.1 >= x.1 => Some(b),
                    _ => Some(x),
                });
            json!({
                "situation": k,
                "worlds": best.map(|b| b.2).unwrap_or(0),
                "best": best.map(|b| b.0).unwrap_or(""),
                "mean_share": best.map(|b| b.1).unwrap_or(0.0),
            })
        })
        .collect()
}

/// The variety of the best way of playing across worlds, among `among`
/// (the informed styles for `good_styles_vary`).
pub fn variety(
    shares: &BTreeMap<u64, BTreeMap<&'static str, f64>>,
    situations: &BTreeMap<u64, (String, String)>,
    among: &[&'static str],
) -> serde_json::Value {
    let mut among_best: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut sole_best: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut totals: BTreeMap<&'static str, (f64, u32)> = BTreeMap::new();
    let mut worlds = Vec::new();
    let mut n = 0usize;
    for (seed, row) in shares {
        let active: Vec<(&'static str, f64)> = among
            .iter()
            .filter_map(|s| row.get(s).map(|v| (*s, *v)))
            .collect();
        let Some(top) = active.iter().map(|x| x.1).reduce(f64::max) else {
            continue;
        };
        n += 1;
        for (s, v) in &active {
            let t = totals.entry(s).or_default();
            t.0 += v;
            t.1 += 1;
        }
        let best: Vec<&'static str> = active
            .iter()
            .filter(|x| x.1 >= top - VARIETY_TIE)
            .map(|x| x.0)
            .collect();
        for s in &best {
            *among_best.entry(s).or_default() += 1;
        }
        if let [only] = best.as_slice() {
            *sole_best.entry(only).or_default() += 1;
        }
        let second = active
            .iter()
            .filter(|x| !best.contains(&x.0))
            .map(|x| x.1)
            .fold(f64::MIN, f64::max);
        let (crisis, industry) = situations.get(seed).cloned().unwrap_or_default();
        worlds.push(json!({
            "seed": seed, "crisis": crisis, "industry": industry,
            "best": best, "share": top,
            "margin": if second > f64::MIN { top - second } else { 0.0 },
        }));
    }
    let nf = n.max(1) as f64;
    let means: BTreeMap<&'static str, f64> = totals
        .iter()
        .map(|(s, (t, c))| (*s, t / (*c).max(1) as f64))
        .collect();
    let best_mean = means.values().copied().fold(f64::MIN, f64::max);
    // (a) Several near-best styles each the sole best somewhere.
    let winners: Vec<&'static str> = among
        .iter()
        .copied()
        .filter(|s| means.get(s).is_some_and(|m| *m >= best_mean - STYLES_NEAR))
        .filter(|s| sole_best.get(s).copied().unwrap_or(0) as f64 / nf >= STYLES_MIN_SOLE)
        .collect();
    let a = winners.len() >= STYLES_MIN_WINNERS;
    // (b) None among the best in most worlds.
    let top = among_best.values().copied().max().unwrap_or(0) as f64 / nf;
    let b = top <= STYLES_MAX_SHARE;
    // (c) The best style differs by starting situation.
    let by_crisis = best_by(shares, situations, among, |s| s.0.clone());
    let by_industry = best_by(shares, situations, among, |s| s.1.clone());
    let distinct = |rows: &[serde_json::Value]| {
        let mut v: Vec<&str> = rows.iter().filter_map(|r| r["best"].as_str()).collect();
        v.sort();
        v.dedup();
        v.len()
    };
    let c = distinct(&by_crisis) >= 2 || distinct(&by_industry) >= 2;
    json!({
        "among": among,
        "worlds": worlds,
        "mean_share": means,
        "best_counts": among_best,
        "sole_best_counts": sole_best,
        "distinct_best": sole_best.len(),
        "top_best_share": top,
        "tie": VARIETY_TIE,
        "near_best_winners": winners,
        "by_crisis": by_crisis,
        "by_industry": by_industry,
        "a": a,
        "b": b,
        "c": c,
        "pass": a && b && c,
    })
}

/// Which games a tournament plays: every strategy on every seed, and
/// `mixed` with the lab switch on (`knowledge_matters`) unless the whole run
/// is omniscient already.
fn jobs(seeds: u64) -> Vec<(&'static str, u64, bool)> {
    let mut v: Vec<(&'static str, u64, bool)> = STRATEGIES
        .iter()
        .flat_map(|s| (1..=seeds).map(move |seed| (*s, seed, false)))
        .collect();
    if !omniscient() {
        v.extend((1..=seeds).map(|seed| ("mixed", seed, true)));
    }
    v
}

/// The games of a tournament. Each world is its own single-threaded,
/// deterministic simulation; independent worlds run on up to three threads
/// (a lab convenience: results are keyed, so the report never depends on
/// scheduling). Keyed by (strategy, seed, omniscient).
fn play_all(
    content: &Arc<ContentBundle>,
    seeds: u64,
) -> BTreeMap<(&'static str, u64, bool), Played> {
    let jobs = jobs(seeds);
    let workers = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .clamp(1, 3);
    let mut out = BTreeMap::new();
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..workers)
            .map(|w| {
                let jobs = &jobs;
                scope.spawn(move || {
                    jobs.iter()
                        .skip(w)
                        .step_by(workers)
                        .filter_map(|(s, seed, omni)| {
                            let omni = *omni || omniscient();
                            play_full_with(content, *seed, s, omni)
                                .map(|p| ((*s, *seed, omni && !omniscient()), p))
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        for h in handles {
            if let Ok(v) = h.join() {
                out.extend(v);
            }
        }
    });
    out
}

/// Margins of the gate ladder (ENGAGEMENT §7.2), in vote-share points
/// (0.01 = one point) and win-rate shares.
pub const ROTE_SHARE_MARGIN: f64 = 0.01;
pub const ROTE_WIN_MARGIN: f64 = 0.05;
/// `ladder_learned`: + 1.0 point until E8, + 3.0 from E8.
pub const LEARNED_MARGIN: f64 = 0.01;
pub const KNOWLEDGE_MARGIN: f64 = 0.03;
pub const SKILL_SHARE_MARGIN: f64 = 0.03;
pub const SKILL_WIN_MARGIN: f64 = 0.10;
pub const IDLE_CAP: f64 = 0.15;

/// The share of each verb in a mix (verbs used / all verbs used).
fn mix_shares(mix: &BTreeMap<&'static str, u32>) -> BTreeMap<&'static str, f64> {
    let total = mix.values().sum::<u32>().max(1) as f64;
    mix.iter().map(|(k, v)| (*k, *v as f64 / total)).collect()
}

/// How far apart two action mixes are: half the sum of the differences in
/// each verb's share (0: the same mix; 1: no verb in common).
fn mix_distance(a: &BTreeMap<&'static str, f64>, b: &BTreeMap<&'static str, f64>) -> f64 {
    let keys: std::collections::BTreeSet<&&str> = a.keys().chain(b.keys()).collect();
    keys.into_iter()
        .map(|k| (a.get(*k).unwrap_or(&0.0) - b.get(*k).unwrap_or(&0.0)).abs())
        .sum::<f64>()
        / 2.0
}

pub fn run(content: &Arc<ContentBundle>, seeds: u64) -> serde_json::Value {
    let mut rows = BTreeMap::new();
    let mut idle_share: BTreeMap<u64, f64> = BTreeMap::new();
    let mut shares: BTreeMap<u64, BTreeMap<&'static str, f64>> = BTreeMap::new();
    let mut situations: BTreeMap<u64, (String, String)> = BTreeMap::new();
    let mut mixes: BTreeMap<&'static str, BTreeMap<&'static str, f64>> = BTreeMap::new();
    let results = play_all(content, seeds);
    for s in STRATEGIES {
        let mut wins = 0;
        let mut n = 0;
        let mut gain = 0.0;
        let mut total = 0.0;
        let mut mix: BTreeMap<&'static str, u32> = BTreeMap::new();
        for seed in 1..=seeds {
            if let Some(p) = results.get(&(s, seed, false)) {
                let (won, share) = (p.won, p.share);
                shares.entry(seed).or_default().insert(s, share);
                situations.insert(seed, (p.crisis.clone(), p.industry.clone()));
                n += 1;
                wins += won as u32;
                total += share;
                if s == "idle" {
                    idle_share.insert(seed, share);
                }
                gain += share - idle_share.get(&seed).copied().unwrap_or(share);
                for (k, v) in &p.mix {
                    *mix.entry(k).or_default() += v;
                }
            }
        }
        let per_career: BTreeMap<&'static str, f64> = mix
            .iter()
            .map(|(k, v)| (*k, *v as f64 / n.max(1) as f64))
            .collect();
        let shares_of = mix_shares(&mix);
        mixes.insert(s, shares_of.clone());
        rows.insert(
            s.to_string(),
            json!({
                "class": class(s).key(),
                "races": n,
                "win_rate": wins as f64 / n.max(1) as f64,
                "mean_share": total / n.max(1) as f64,
                "mean_share_gain_vs_idle": gain / n.max(1) as f64,
                "mix": shares_of,
                "verbs_per_career": per_career,
            }),
        );
    }
    let rate = |s: &str| rows.get(s).and_then(|v| v["win_rate"].as_f64());
    let mean = |s: &str| rows.get(s).and_then(|v| v["mean_share"].as_f64());
    let of = |f: fn(&str) -> bool| -> Vec<&'static str> {
        STRATEGIES.iter().copied().filter(|s| f(s)).collect()
    };
    let (single, rote, informed) = (
        of(|s| class(s) == Class::SingleVerb),
        of(is_rote),
        of(is_informed),
    );
    let honest = of(|s| !is_probe(s));
    let max_of = |set: &[&'static str], f: &dyn Fn(&str) -> Option<f64>| {
        set.iter().filter_map(|s| f(s)).fold(0.0, f64::max)
    };
    let best = max_of(&honest, &rate);
    let median_of = |set: &[&'static str]| {
        let mut r: Vec<f64> = set.iter().filter_map(|s| rate(s)).collect();
        r.sort_by(|a, b| a.total_cmp(b));
        r.get(r.len() / 2).copied().unwrap_or(0.0)
    };
    let idle = rate("idle").unwrap_or(1.0);
    // Rung 0: effort pays, and exploring alone never wins.
    let idle_cap = idle <= IDLE_CAP;
    let effort_matters = best > idle;
    let explore_only = rate("explore_only").unwrap_or(1.0);
    let rote_median = median_of(&rote);
    let explore_alone = explore_only <= idle + EXPLORE_ONLY_OVER_IDLE && explore_only < rote_median;
    // Rung 1: a blind mix beats every single verb.
    let (mixed_mean, mixed_rate) = (mean("mixed").unwrap_or(0.0), rate("mixed").unwrap_or(0.0));
    let ladder_rote = single.iter().all(|s| {
        mixed_mean >= mean(s).unwrap_or(1.0) + ROTE_SHARE_MARGIN
            && mixed_rate >= rate(s).unwrap_or(1.0) + ROTE_WIN_MARGIN
    });
    // Rung 2: learning the town pays, and knowing it is worth as much.
    let explorer = mean("explorer").unwrap_or(0.0);
    let ladder_learned =
        explorer >= mixed_mean + LEARNED_MARGIN && rate("explorer").unwrap_or(0.0) >= mixed_rate;
    let omni: Vec<&Played> = (1..=seeds)
        .filter_map(|seed| results.get(&("mixed", seed, true)))
        .collect();
    let omni_mean =
        (!omni.is_empty()).then(|| omni.iter().map(|p| p.share).sum::<f64>() / omni.len() as f64);
    let knowledge_matters = omni_mean.map(|m| m >= mixed_mean + KNOWLEDGE_MARGIN);
    // Rung 3: informed play beats rote play by a clear margin, in several styles.
    let (best_informed_mean, best_rote_mean) = (max_of(&informed, &mean), max_of(&rote, &mean));
    let (best_informed_rate, best_rote_rate) = (max_of(&informed, &rate), max_of(&rote, &rate));
    let skill_gap = best_informed_mean >= best_rote_mean + SKILL_SHARE_MARGIN
        && best_informed_rate >= best_rote_rate + SKILL_WIN_MARGIN;
    let styles = variety(&shares, &situations, &informed);
    // Corruption tempts but never dominates.
    let corrupt = rate("corrupt").unwrap_or(0.0);
    let corrupt_ok = corrupt <= best + CORRUPT_MARGIN;
    // The informed styles each play their own way (reported).
    let explorer_mix = mixes.get("explorer").cloned().unwrap_or_default();
    let mix_vs_explorer: BTreeMap<&'static str, f64> = informed
        .iter()
        .filter(|s| **s != "explorer")
        .map(|s| {
            (
                *s,
                mix_distance(mixes.get(s).unwrap_or(&BTreeMap::new()), &explorer_mix),
            )
        })
        .collect();
    // Is `canvass` still the best way of playing (the owner's worry)?
    let top_by = |f: &dyn Fn(&str) -> Option<f64>| {
        honest
            .iter()
            .copied()
            .max_by(|a, b| {
                f(a).unwrap_or(0.0)
                    .total_cmp(&f(b).unwrap_or(0.0))
                    .then(b.cmp(a))
            })
            .unwrap_or("")
    };
    let enforced = |g: &str| ENFORCED.contains(&g);
    let rung = |r: u8, g: &str, pass: serde_json::Value, detail: serde_json::Value| json!({ "rung": r, "gate": g, "pass": pass, "enforced": enforced(g), "detail": detail });
    let ladder = vec![
        rung(
            0,
            "idle_cap",
            idle_cap.into(),
            json!({ "idle_win_rate": idle }),
        ),
        rung(
            0,
            "effort_matters",
            effort_matters.into(),
            json!({ "best_win_rate": best }),
        ),
        rung(
            0,
            "exploring_alone_never_wins",
            explore_alone.into(),
            json!({ "explore_only_win_rate": explore_only, "rote_median_win_rate": rote_median }),
        ),
        rung(
            1,
            "ladder_rote",
            ladder_rote.into(),
            json!({ "mixed_mean_share": mixed_mean, "mixed_win_rate": mixed_rate }),
        ),
        rung(
            2,
            "ladder_learned",
            ladder_learned.into(),
            json!({ "explorer_minus_mixed_share": explorer - mixed_mean, "margin": LEARNED_MARGIN }),
        ),
        rung(
            2,
            "knowledge_matters",
            knowledge_matters.into(),
            json!({ "omniscient_mixed_mean_share": omni_mean, "mixed_mean_share": mixed_mean }),
        ),
        rung(
            3,
            "skill_gap",
            skill_gap.into(),
            json!({
                "share_gap": best_informed_mean - best_rote_mean,
                "win_rate_gap": best_informed_rate - best_rote_rate,
            }),
        ),
        rung(
            3,
            "good_styles_vary",
            styles["pass"].clone(),
            json!({ "a": styles["a"], "b": styles["b"], "c": styles["c"] }),
        ),
        rung(
            9,
            "corrupt_not_dominant",
            corrupt_ok.into(),
            json!({ "corrupt_minus_best_honest": corrupt - best }),
        ),
    ];
    json!({
        "seeds": seeds,
        "omniscient": omniscient(),
        "strategies": rows,
        "ladder": ladder,
        "idle_cap": idle_cap,
        "idle_win_rate": idle,
        "effort_matters": effort_matters,
        "explore_only_win_rate": explore_only,
        "exploring_alone_never_wins": explore_alone,
        "ladder_rote": ladder_rote,
        "explorer_minus_mixed_share": explorer - mixed_mean,
        "ladder_learned": ladder_learned,
        "omniscient_mixed_mean_share": omni_mean,
        "knowledge_matters": knowledge_matters,
        "skill_gap": skill_gap,
        "good_styles_vary": styles["pass"],
        "good_styles_vary_b": styles["b"],
        "styles": styles,
        "corrupt_minus_best_honest": corrupt - best,
        "corrupt_not_dominant": corrupt_ok,
        "best_minus_median": best - median_of(&honest),
        "mix_vs_explorer": mix_vs_explorer,
        "best_win_rate_strategy": top_by(&rate),
        "best_mean_share_strategy": top_by(&mean),
    })
}

/// The gate ladder (ENGAGEMENT §7.2), every rung, reported in this order.
pub const GATES: [&str; 9] = [
    "idle_cap",
    "effort_matters",
    "exploring_alone_never_wins",
    "ladder_rote",
    "ladder_learned",
    "knowledge_matters",
    "skill_gap",
    "good_styles_vary",
    "corrupt_not_dominant",
];

/// The gates `bots` fails on from E1 (the ladder's "Enforced" column): the
/// rest are reported until E8 (and `ladder_learned` rises to + 3.0 then).
/// `good_styles_vary_b` is clause (b) of `good_styles_vary`.
pub const ENFORCED: [&str; 6] = [
    "idle_cap",
    "effort_matters",
    "exploring_alone_never_wins",
    "ladder_learned",
    "good_styles_vary_b",
    "corrupt_not_dominant",
];

/// "Exploring alone never wins": `explore_only` wins at most this much more
/// often than `idle`, and less often than the median rote routine.
pub const EXPLORE_ONLY_OVER_IDLE: f64 = 0.05;

/// "Corrupt not dominant": the corrupt routine's win rate is at most this
/// much above the best honest strategy's.
pub const CORRUPT_MARGIN: f64 = 0.1;

/// The enforced gates a report fails, by name. Under the lab switch
/// (`--omniscient`, a baseline) knowledge is free, so `ladder_learned` is
/// reported but not enforced.
pub fn failed_gates(report: &serde_json::Value) -> Vec<&'static str> {
    let omniscient = report["omniscient"].as_bool().unwrap_or(false);
    ENFORCED
        .iter()
        .copied()
        .filter(|g| !(omniscient && *g == "ladder_learned"))
        .filter(|g| !report[*g].as_bool().unwrap_or(false))
        .collect()
}

/// A career's outcome over several years with a strategy (DEC-076 style
/// distribution for the corrupt strategy): elections won and lost, scandals,
/// investigations, convictions, prison, and a career ended.
pub fn career(
    content: &Arc<ContentBundle>,
    seed: u64,
    strategy: &str,
    years: u32,
) -> Option<serde_json::Value> {
    let mut e = new_game(content, seed)?;
    let me = e.state().player.as_ref()?.politician;
    let start = e.state().calendar.date;
    let end = start.add_days(years * 365);
    let start_turn = e.state().calendar.turn;
    let mut imprisoned = false;
    while e.state().calendar.date < end {
        play_turn_from(&mut e, content, strategy, start_turn);
        if e.state()
            .player
            .as_ref()
            .is_some_and(|p| matches!(p.status, mandate_sim::player::RunStatus::Imprisoned { .. }))
        {
            imprisoned = true;
        }
        if e.apply(Command::EndTurn).is_err() {
            break;
        }
    }
    let st = e.state();
    let (mut won, mut lost) = (0, 0);
    for r in st
        .politics
        .elections
        .results
        .iter()
        .filter(|r| r.date > start)
    {
        if r.candidates.iter().any(|c| c.politician == me) {
            if r.winner == me {
                won += 1;
            } else {
                lost += 1;
            }
        }
    }
    let scandals = st.media.scandals.iter().filter(|s| s.subject == me).count();
    let cases: Vec<&mandate_sim::justice::Case> = st
        .justice
        .cases
        .iter()
        .filter(|c| c.subject == me)
        .collect();
    let convicted = cases
        .iter()
        .filter(|c| c.stage == mandate_sim::justice::Stage::Convicted)
        .count();
    let prison = cases
        .iter()
        .filter(|c| c.sentence.is_some_and(|s| s.prison_years() > 0))
        .count();
    let ended = st
        .player
        .as_ref()
        .is_some_and(|p| matches!(p.status, mandate_sim::player::RunStatus::Ended { .. }));
    let hidden = st
        .narrative
        .hidden
        .iter()
        .filter(|h| h.subject == Some(me))
        .count();
    let mut kinds: BTreeMap<String, u32> = BTreeMap::new();
    for h in st.narrative.hidden.iter().filter(|h| h.subject == Some(me)) {
        *kinds.entry(h.kind.clone()).or_default() += 1;
    }
    let npc_scandals = st.media.scandals.iter().filter(|s| s.subject != me).count();
    let npc_cases = st.justice.cases.iter().filter(|c| c.subject != me).count();
    Some(json!({
        "seed": seed, "won": won, "lost": lost, "hidden_acts": hidden, "acts": kinds, "scandals": scandals,
        "cases": cases.len(), "convicted": convicted, "prison": prison > 0 || imprisoned, "career_ended": ended,
        "in_office": st.politics.get(me).is_some_and(|p| p.in_office()),
        "money": st.player.as_ref().map(|p| p.money.units()).unwrap_or(0),
        "npc_scandals": npc_scandals, "npc_cases": npc_cases,
    }))
}

/// The career distribution of a strategy across seeds.
pub fn careers(
    content: &Arc<ContentBundle>,
    seeds: u64,
    strategy: &str,
    years: u32,
) -> serde_json::Value {
    let rows: Vec<serde_json::Value> = (1..=seeds)
        .filter_map(|s| career(content, s, strategy, years))
        .collect();
    let n = rows.len().max(1) as f64;
    let count = |k: &str| {
        rows.iter()
            .filter(|r| r[k].as_bool().unwrap_or(false) || r[k].as_u64().unwrap_or(0) > 0)
            .count()
    };
    let sum = |k: &str| rows.iter().map(|r| r[k].as_u64().unwrap_or(0)).sum::<u64>();
    json!({
        "strategy": strategy, "seeds": seeds, "years": years,
        "elections_won": sum("won"), "elections_lost": sum("lost"),
        "careers_with_a_scandal": count("scandals") as f64 / n,
        "careers_investigated": count("cases") as f64 / n,
        "careers_convicted": count("convicted") as f64 / n,
        "careers_in_prison": count("prison") as f64 / n,
        "careers_ended": count("career_ended") as f64 / n,
        "careers_in_office_at_end": count("in_office") as f64 / n,
        "npc_scandals_per_career": sum("npc_scandals") as f64 / n,
        "npc_cases_per_career": sum("npc_cases") as f64 / n,
        "rows": rows,
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use mandate_sim::player::knowledge::Subject;

    /// The members of the player's town's legislature.
    fn legislature<'a>(
        st: &'a mandate_sim::State,
        content: &ContentBundle,
    ) -> &'a [mandate_sim::politics::PoliticianId] {
        let c = content.constitutions.get(&st.meta.constitution).unwrap();
        let town = mandate_sim::world::Jurisdiction::town(st.player.as_ref().unwrap().home());
        mandate_sim::politics::legislature_of(&st.politics, c, town)
    }

    fn content() -> Arc<ContentBundle> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
        crate::lab::load(&root, &[]).unwrap_or_else(|e| panic!("{e}"))
    }

    /// An independent copy of a game (the same state, through the load path).
    fn fork(e: &Engine) -> Engine {
        Engine::from_state(e.state().clone(), e.content_arc()).expect("fork")
    }

    /// Perturb truths the player has not learned (M6_ENGINE_CONTRACTS
    /// §1.10): the goals of people the player does not know in detail are
    /// handed round among them, and two neighbourhoods whose worries the
    /// player has not heard swap their residents' views (so their worries
    /// swap too). Returns how many facts changed.
    fn perturb(e: &mut Engine) -> usize {
        let mut changed = 0;
        let (unknown_people, hoods) = {
            let st = e.state();
            let k = Knowing::of(e);
            let pl = st.player.as_ref().expect("player");
            let unknown: Vec<CharacterId> = st
                .politics
                .characters
                .goals
                .iter()
                .map(|g| g.holder)
                .filter(|c| k.person(*c) < Level::Detailed)
                .collect();
            let ward = ward_hoods(&st.world, pl.ward);
            let hoods: Vec<usize> = st
                .world
                .hoods_of(pl.municipality)
                .filter(|h| !ward.contains(h))
                .filter(|h| {
                    k.level(&Subject::HoodConcerns {
                        hood: NeighborhoodId(*h as u32),
                    }) == Level::Unknown
                })
                .collect();
            (unknown, hoods)
        };
        e.setup(|s| {
            // Goals: each unknown holder's goals go to the next unknown holder.
            let holders: Vec<CharacterId> = {
                let mut v = unknown_people.clone();
                v.sort();
                v.dedup();
                v
            };
            if holders.len() >= 2 {
                for g in s.politics.characters.goals.iter_mut() {
                    if let Some(i) = holders.iter().position(|h| *h == g.holder) {
                        g.holder = holders[(i + 1) % holders.len()];
                        changed += 1;
                    }
                }
            }
            // Worries: two unheard neighbourhoods trade their residents' views.
            if let [a, b, ..] = hoods.as_slice() {
                let n = s.population.hoods[*a]
                    .cohorts
                    .len()
                    .min(s.population.hoods[*b].cohorts.len());
                for i in 0..n {
                    let x = s.population.hoods[*a].cohorts[i].attitudes.clone();
                    let y = std::mem::replace(&mut s.population.hoods[*b].cohorts[i].attitudes, x);
                    s.population.hoods[*a].cohorts[i].attitudes = y;
                    changed += 1;
                }
            }
        });
        changed
    }

    /// The commands a bot sends in one turn from this game.
    fn turn_commands(
        e: &Engine,
        content: &ContentBundle,
        strategy: &str,
        start: u32,
    ) -> Vec<String> {
        let mut g = fork(e);
        record(true);
        play_turn_from(&mut g, content, strategy, start);
        record(false)
    }

    /// Blind invariance (DEC-091; M6_ENGINE_CONTRACTS §1.10): a bot reads
    /// gated facts only through `Knowing`, so perturbing truths it has not
    /// learned leaves every command it sends unchanged, turn after turn, from
    /// the start into the campaign.
    #[test]
    fn bots_are_blind_to_what_they_have_not_learned() {
        let content = content();
        for (strategy, seed) in [
            ("explorer", 3),
            ("mixed", 5),
            ("legislator", 2),
            ("corrupt", 4),
            ("canvass", 6),
            ("proxy", 7),
            ("explorer_campaigner", 8),
            ("explorer_coalition", 9),
            ("explorer_legislator", 10),
        ] {
            let mut e = new_game(&content, seed).expect("game");
            let start = e.state().calendar.turn;
            let mut perturbed = 0;
            let mut sent = 0;
            let mut campaign = false;
            for _ in 0..26 {
                campaign |= campaigning(&e);
                let plain = turn_commands(&e, &content, strategy, start);
                let mut p = fork(&e);
                perturbed += perturb(&mut p);
                let blind = turn_commands(&p, &content, strategy, start);
                assert_eq!(
                    plain,
                    blind,
                    "{strategy} (seed {seed}, turn {}) acted on something it had not learned",
                    e.state().calendar.turn
                );
                sent += plain.len();
                play_turn_from(&mut e, &content, strategy, start);
                if e.apply(Command::EndTurn).is_err() {
                    break;
                }
            }
            assert!(perturbed > 0, "{strategy}: nothing was perturbed");
            assert!(sent > 0, "{strategy}: no commands were sent");
            assert!(campaign, "{strategy}: the campaign was never reached");
        }
    }

    /// The perturbation is real: the swapped neighbourhoods' worries change,
    /// while the player's view of them does not.
    #[test]
    fn the_perturbation_changes_hidden_worries() {
        let content = content();
        let e = new_game(&content, 3).expect("game");
        let mut p = fork(&e);
        assert!(perturb(&mut p) > 0);
        let st = e.state();
        let pl = st.player.as_ref().unwrap();
        let ward = ward_hoods(&st.world, pl.ward);
        let mut differs = false;
        for h in st
            .world
            .hoods_of(pl.municipality)
            .filter(|h| !ward.contains(h))
        {
            let hood = NeighborhoodId(h as u32);
            assert_eq!(
                Knowing::of(&e).concerns_of(hood),
                Knowing::of(&p).concerns_of(hood)
            );
            let truth = |g: &Engine| {
                mandate_sim::concerns::felt_concerns(
                    &g.state().population,
                    &content,
                    pl.municipality.index(),
                    &[h],
                )
            };
            differs |= truth(&e) != truth(&p);
        }
        assert!(differs, "no hidden worry changed");
    }

    /// The canvassing match shares one function with the command, and a
    /// neighbourhood's real top worry beats one it barely feels.
    #[test]
    fn canvassing_on_the_real_worry_works_better() {
        let content = content();
        let e = new_game(&content, 1).expect("game");
        let st = e.state();
        let pl = st.player.as_ref().unwrap();
        let hood = ward_hoods(&st.world, pl.ward)[0];
        let felt = mandate_sim::concerns::felt_concerns(
            &st.population,
            &content,
            pl.municipality.index(),
            &[hood],
        );
        let id = |i: usize| content.concerns[i].id.clone();
        let (top, last) = (id(felt[0].0), id(felt[felt.len() - 1].0));
        let m = |issue: Option<&str>| {
            mandate_sim::actions::canvass_match(st, &content, NeighborhoodId(hood as u32), issue)
        };
        let k = &content.knowledge.effects.canvass;
        assert_eq!(m(None), 1.0);
        assert!(m(Some(&top)) > 1.0 && m(Some(&top)) <= k.ceiling);
        assert!(m(Some(&last)) < 1.0 && m(Some(&last)) >= k.floor);
        // The command applies the same multiplier: a canvass on the top
        // worry raises approval more than one on the least felt.
        let gain = |issue: &str| {
            let mut g = fork(&e);
            let approval = |g: &Engine| -> f64 {
                g.state().population.hoods[hood]
                    .cohorts
                    .iter()
                    .map(|c| c.player_approval as f64)
                    .sum()
            };
            let before = approval(&g);
            let cmd = g.route(PlayerCommand::Act {
                action: Action::Canvass {
                    neighborhood: hood as u32,
                    issue: Some(issue.to_string()),
                },
                late_night: false,
            });
            let r = g.apply(cmd).expect("canvass");
            (approval(&g) - before, r.result.unwrap().args)
        };
        let (hi, args) = gain(&top);
        let (lo, _) = gain(&last);
        assert!(hi > lo, "{hi} vs {lo}");
        assert_eq!(args.get("issue"), Some(&top));
        // An unknown worry is refused.
        let mut g = fork(&e);
        let cmd = g.route(PlayerCommand::Act {
            action: Action::Canvass {
                neighborhood: hood as u32,
                issue: Some("concern.nonsense".into()),
            },
            late_night: false,
        });
        assert!(g.apply(cmd).is_err());
    }

    /// The canvass curve in a real game (DEC-097 §1; ENGAGEMENT §2.1): the
    /// campaign multiplier applies only while the player is a candidate in
    /// an open race, and the command applies the curve the preview shows (a
    /// repeat visit wins fewer over than a first one).
    #[test]
    fn the_campaign_multiplier_applies_only_while_a_candidate() {
        let content = content();
        let fx = &content.actions.canvass;
        let mut e = new_game(&content, 4).expect("game");
        let start = e.state().calendar.turn;
        let hood = |e: &Engine| {
            let st = e.state();
            ward_hoods(&st.world, st.player.as_ref().unwrap().ward)[0] as u32
        };
        let curve = |e: &Engine| {
            mandate_sim::player::ground::canvass_curve(e.state(), fx, NeighborhoodId(hood(e)))
        };
        assert!(!campaigning(&e));
        assert_eq!(curve(&e).season, 1.0);
        // Governing: a first visit, then a repeat, through the command.
        let gain = |e: &mut Engine| {
            let h = hood(e) as usize;
            let approval = |e: &Engine| -> f64 {
                e.state().population.hoods[h]
                    .cohorts
                    .iter()
                    .map(|c| c.player_approval as f64)
                    .sum()
            };
            let before = approval(e);
            let preview = curve(e);
            let cmd = e.route(PlayerCommand::Act {
                action: Action::Canvass {
                    neighborhood: h as u32,
                    issue: None,
                },
                late_night: false,
            });
            let r = e.apply(cmd).expect("canvass");
            let band = r.result.unwrap().args.get("fresh").cloned();
            (approval(e) - before, preview, band)
        };
        let (first, c1, b1) = gain(&mut e);
        assert_eq!(c1.fresh, 1.0);
        assert_eq!(b1.as_deref(), Some("new"));
        e.apply(Command::EndTurn).expect("turn");
        let (second, c2, _) = gain(&mut e);
        assert!(c2.fresh < 0.75 && c2.season == 1.0, "{c2:?}");
        assert!(second < first, "{second} vs {first}");
        // Play on to the campaign: the multiplier holds while a candidate.
        let mut seen = false;
        for _ in 0..80 {
            if campaigning(&e) {
                seen = true;
                let c = curve(&e);
                assert_eq!(c.season, fx.campaign_mult);
                assert_eq!(c.approval, c.fresh * c.season * c.same_turn);
                break;
            }
            assert_eq!(curve(&e).season, 1.0);
            play_turn_from(&mut e, &content, "mixed", start);
            e.apply(Command::EndTurn).expect("turn");
        }
        assert!(seen, "the campaign was never reached");
        // After the election, back to governing.
        while campaigning(&e) {
            e.apply(Command::EndTurn).expect("turn");
        }
        assert_eq!(curve(&e).season, 1.0);
    }

    /// The lobbying odds the player is shown carry the knowledge term (the
    /// shown and the rolled odds are one function), and knowing someone
    /// better never makes them harder to persuade.
    #[test]
    fn lobbying_odds_follow_what_the_player_knows() {
        let content = content();
        let e = new_game(&content, 2).expect("game");
        let st = e.state();
        let pl = st.player.as_ref().unwrap();
        let target = *legislature(st, &content)
            .iter()
            .find(|id| **id != pl.politician)
            .unwrap();
        let person = st.politics.get(target).unwrap().character;
        let odds = |level: Level| {
            let mut g = fork(&e);
            g.setup(|s| {
                let on = s.calendar.date;
                let k = &mut s.player.as_mut().unwrap().knowledge;
                k.entries.remove(&Subject::Person { person });
                if level > Level::Unknown {
                    k.learn(
                        Subject::Person { person },
                        level,
                        on,
                        mandate_sim::player::knowledge::Source::Lab,
                    );
                }
            });
            let shown = mandate_sim::actions::lobby_odds(g.state(), &content, target, false);
            let rolled = npc::lobby_odds_between(g.state(), &content, pl.politician, target, false);
            assert_eq!(shown, rolled);
            shown
        };
        let v: Vec<f64> = Level::ALL.iter().map(|l| odds(*l)).collect();
        assert!(v.windows(2).all(|w| w[0] <= w[1]), "{v:?}");
        assert!(v[3] > v[0], "{v:?}");
        // NPC deals use `known` (NPCs keep no ledger).
        let other = *legislature(st, &content)
            .iter()
            .find(|id| **id != pl.politician && **id != target)
            .unwrap();
        assert_eq!(
            npc::lobby_knowledge_logit(st, &content, other, target),
            content.knowledge.effects.lobby_logit.known
        );
    }
}
