//! Bot players (M4, TEST_STRATEGY): scripted strategies play the player's
//! first local election through the same commands as the UI, across many
//! seeds. A strategy that wins far more often than the others is a balance
//! bug ("no dominant strategy", owner checkpoint M4). Bots go where each
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
    let mut e = generate(
        &WorldConfig {
            seed,
            size: WorldSize::Small,
            scenario: None,
        },
        content.clone(),
    )
    .ok()?;
    e.set_lab_omniscient(omniscient());
    Some(e)
}

pub const STRATEGIES: [&str; 9] = [
    "idle",
    "canvass",
    "media",
    "legislator",
    "mixed",
    "proxy",
    "corrupt",
    "explorer",
    "explore_only",
];

/// Probes, not routines: `idle` (the baseline) and `explore_only` (the
/// "exploring alone never wins" gate) stay out of the dominance and variety
/// checks, which compare ways of playing.
pub fn is_probe(s: &str) -> bool {
    matches!(s, "idle" | "explore_only")
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

/// Every command a bot sends goes through here.
fn apply(e: &mut Engine, cmd: Command) -> Result<CommandOutcome, EngineError> {
    RECORD.with(|r| {
        if let Some(v) = r.borrow_mut().as_mut() {
            v.push(serde_json::to_string(&cmd).unwrap_or_default());
        }
    });
    e.apply(cmd)
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
        // The NPC AI's pragmatist (the time-skip proxy).
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
        .filter(|c| c.named.is_some())
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

/// One turn of learning: walk the town and the streets, read the ward's worries on the
/// doorsteps, and meet the people named so far, asking who really runs
/// things while that is a rumour. Returns whether any time went on it.
/// With `all` (the `explore_only` probe) it only finds out: walks, meets and
/// asks, and never campaigns (canvassing teaches too, but it is the ground
/// game), and nothing else is ever done.
fn explore(e: &mut Engine, content: &ContentBundle, all: bool) -> bool {
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

/// Whether `explorer` is still in its learning turns.
fn exploring(e: &Engine, start_turn: u32) -> bool {
    e.state().calendar.turn < start_turn + EXPLORE_TURNS && !campaigning(e)
}

/// A bot's turn; `start_turn` is the turn play began (for `explorer`).
pub fn play_turn_from(e: &mut Engine, content: &ContentBundle, strategy: &str, start_turn: u32) {
    if strategy == "explore_only" {
        // Only learning: no decisions answered, no campaigning, no rest.
        if e.state().player.as_ref().is_some_and(|p| p.status.active()) {
            explore(e, content, true);
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
    if camp && matches!(strategy, "media" | "mixed" | "corrupt" | "explorer") {
        buy_ads(e, 3);
    }
    // The explorer's first turns go on learning the town; then it plays the mix.
    if strategy == "explorer" && exploring(e, start_turn) {
        explore(e, content, false);
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
            "idle" => false,
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
    let mut e = new_game(content, seed)?;
    let (crisis, industry) = situation(&e);
    let lost = |crisis: String, industry: String| Played {
        won: false,
        share: 0.0,
        crisis,
        industry,
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
    })
}

/// The variety gate (DEC-077 §5): the best strategy must differ across
/// worlds, so no single routine wins everywhere. Per world (seed), the best
/// active strategies are those within `VARIETY_TIE` of the highest vote
/// share (a near-tie is shared: `corrupt` is `mixed` plus shortcuts and often
/// lands within a hair of it). The gate needs at least
/// `VARIETY_MIN_WINNERS` strategies that are the sole best somewhere, and no
/// strategy among the best in more than `VARIETY_MAX_SHARE` of the worlds.
pub const VARIETY_MIN_WINNERS: usize = 2;
pub const VARIETY_MAX_SHARE: f64 = 0.6;
pub const VARIETY_TIE: f64 = 0.002;

pub fn variety(
    shares: &BTreeMap<u64, BTreeMap<&'static str, f64>>,
    situations: &BTreeMap<u64, (String, String)>,
) -> serde_json::Value {
    let mut among_best: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut sole_best: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut worlds = Vec::new();
    let mut n = 0usize;
    for (seed, row) in shares {
        let active: Vec<(&'static str, f64)> = STRATEGIES
            .iter()
            .filter(|s| !is_probe(s))
            .filter_map(|s| row.get(s).map(|v| (*s, *v)))
            .collect();
        let Some(top) = active.iter().map(|x| x.1).reduce(f64::max) else {
            continue;
        };
        n += 1;
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
            "shares": row,
        }));
    }
    let top = among_best.values().copied().max().unwrap_or(0) as f64 / n.max(1) as f64;
    // Best strategy by situation: the highest mean share among its worlds.
    let mut by_situation: BTreeMap<String, BTreeMap<&'static str, (f64, u32)>> = BTreeMap::new();
    for (seed, row) in shares {
        let key = situations
            .get(seed)
            .map(|x| x.0.clone())
            .unwrap_or_else(|| "none".into());
        let e = by_situation.entry(key).or_default();
        for (s, v) in row.iter().filter(|(k, _)| !is_probe(k)) {
            let x = e.entry(s).or_default();
            x.0 += v;
            x.1 += 1;
        }
    }
    let situation_rows: Vec<serde_json::Value> = by_situation
        .iter()
        .map(|(k, m)| {
            let best = STRATEGIES
                .iter()
                .filter_map(|s| m.get(s).map(|(t, c)| (*s, t / (*c).max(1) as f64, *c)))
                .fold(None::<(&str, f64, u32)>, |b, x| match b {
                    Some(b) if b.1 >= x.1 => Some(b),
                    _ => Some(x),
                });
            json!({
                "crisis": k,
                "worlds": best.map(|b| b.2).unwrap_or(0),
                "best": best.map(|b| b.0).unwrap_or(""),
                "mean_share": best.map(|b| b.1).unwrap_or(0.0),
            })
        })
        .collect();
    let distinct = sole_best.len();
    json!({
        "worlds": worlds,
        "best_counts": among_best,
        "sole_best_counts": sole_best,
        "distinct_best": distinct,
        "top_best_share": top,
        "tie": VARIETY_TIE,
        "by_situation": situation_rows,
        "min_distinct": VARIETY_MIN_WINNERS,
        "max_top_share": VARIETY_MAX_SHARE,
        "pass": distinct >= VARIETY_MIN_WINNERS && top <= VARIETY_MAX_SHARE,
    })
}

/// Every strategy on every seed. Each world is its own single-threaded,
/// deterministic simulation; independent worlds run on up to three threads
/// (a lab convenience: results are keyed, so the report never depends on
/// scheduling).
fn play_all(content: &Arc<ContentBundle>, seeds: u64) -> BTreeMap<(&'static str, u64), Played> {
    let jobs: Vec<(&'static str, u64)> = STRATEGIES
        .iter()
        .flat_map(|s| (1..=seeds).map(move |seed| (*s, seed)))
        .collect();
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
                        .filter_map(|(s, seed)| {
                            play_full(content, *seed, s).map(|p| ((*s, *seed), p))
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

pub fn run(content: &Arc<ContentBundle>, seeds: u64) -> serde_json::Value {
    let mut rows = BTreeMap::new();
    let mut idle_share: BTreeMap<u64, f64> = BTreeMap::new();
    let mut shares: BTreeMap<u64, BTreeMap<&'static str, f64>> = BTreeMap::new();
    let mut situations: BTreeMap<u64, (String, String)> = BTreeMap::new();
    let results = play_all(content, seeds);
    for s in STRATEGIES {
        let mut wins = 0;
        let mut n = 0;
        let mut gain = 0.0;
        let mut total = 0.0;
        for seed in 1..=seeds {
            if let Some(p) = results.get(&(s, seed)) {
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
            }
        }
        rows.insert(
            s.to_string(),
            json!({
                "races": n,
                "win_rate": wins as f64 / n.max(1) as f64,
                "mean_share": total / n.max(1) as f64,
                "mean_share_gain_vs_idle": gain / n.max(1) as f64,
            }),
        );
    }
    let rate = |s: &str| rows.get(s).and_then(|v| v["win_rate"].as_f64());
    let mean = |s: &str| rows.get(s).and_then(|v| v["mean_share"].as_f64());
    // Dominance check: the best way of playing should not crush the rest.
    let rates: Vec<f64> = STRATEGIES
        .iter()
        .filter(|s| !is_probe(s))
        .filter_map(|s| rate(s))
        .collect();
    let best = rates.iter().cloned().fold(0.0, f64::max);
    let median = {
        let mut r = rates.clone();
        r.sort_by(|a, b| a.total_cmp(b));
        r.get(r.len() / 2).copied().unwrap_or(0.0)
    };
    let idle = rate("idle").unwrap_or(1.0);
    // Corruption tempts but never dominates: its win rate is at most
    // `CORRUPT_MARGIN` above the best honest routine's.
    let honest = STRATEGIES
        .iter()
        .filter(|s| !is_probe(s) && **s != "corrupt")
        .filter_map(|s| rate(s))
        .fold(0.0, f64::max);
    let corrupt = rate("corrupt").unwrap_or(0.0);
    // DEC-091 (M6_ENGINE_CONTRACTS §1.10): exploring early pays...
    let (explorer, mixed) = (
        mean("explorer").unwrap_or(0.0),
        mean("mixed").unwrap_or(0.0),
    );
    let explore_pays = explorer >= mixed + EXPLORE_MARGIN
        && rate("explorer").unwrap_or(0.0) >= rate("mixed").unwrap_or(1.0);
    // ...and exploring alone never wins.
    let explore_only = rate("explore_only").unwrap_or(1.0);
    let explore_alone = explore_only <= idle + EXPLORE_ONLY_OVER_IDLE && explore_only < median;
    let variety = variety(&shares, &situations);
    json!({
        "seeds": seeds,
        "omniscient": omniscient(),
        "strategies": rows,
        // DEC-077 §5: the best routine differs across worlds and situations.
        "variety_ok": variety["pass"],
        "variety": variety,
        "best_minus_median": best - median,
        "no_dominant_strategy": best - median <= 0.3,
        "idle_win_rate": idle,
        // DEC-076: doing nothing should rarely work.
        "idle_rarely_wins": idle <= 0.15,
        "effort_matters": best > idle,
        "corrupt_minus_best_honest": corrupt - honest,
        "corrupt_not_dominant": corrupt <= honest + CORRUPT_MARGIN,
        "explorer_minus_mixed_share": explorer - mixed,
        "exploring_pays": explore_pays,
        "explore_only_win_rate": explore_only,
        "exploring_alone_never_wins": explore_alone,
    })
}

/// The bots' gates (`bots` fails if any is false).
pub const GATES: [&str; 7] = [
    "no_dominant_strategy",
    "effort_matters",
    "idle_rarely_wins",
    "corrupt_not_dominant",
    "variety_ok",
    "exploring_pays",
    "exploring_alone_never_wins",
];

/// "Exploring early pays" (§1.10): `explorer`'s mean first-election vote
/// share is at least this much above `mixed`'s (one point), and its win
/// rate at least `mixed`'s.
pub const EXPLORE_MARGIN: f64 = 0.01;

/// "Exploring alone never wins": `explore_only` wins at most this much more
/// often than `idle`, and less often than the median way of playing.
pub const EXPLORE_ONLY_OVER_IDLE: f64 = 0.05;

/// "Corrupt not dominant": the corrupt routine's win rate is at most this
/// much above the best honest routine's.
pub const CORRUPT_MARGIN: f64 = 0.1;

/// The gates a report fails, by name. Under the lab switch (`--omniscient`,
/// a baseline) knowledge is free, so "exploring early pays" is reported but
/// not enforced.
pub fn failed_gates(report: &serde_json::Value) -> Vec<&'static str> {
    let omniscient = report["omniscient"].as_bool().unwrap_or(false);
    GATES
        .iter()
        .copied()
        .filter(|g| !(omniscient && *g == "exploring_pays"))
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
