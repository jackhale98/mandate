//! Bot players (M4, TEST_STRATEGY): scripted strategies play the player's
//! first local election through the same commands as the UI, across many
//! seeds. A strategy that wins far more often than the others is a balance
//! bug ("no dominant strategy", owner checkpoint M4). Bots go where each
//! action is done before doing it, through the same auto-routing `Go`
//! command as the UI's agenda (DEC-079).

use mandate_content::{AiAction, ContentBundle};
use mandate_sim::actions::{Action, PlayerCommand};
use mandate_sim::policy::{PolicyStatus, public_support, ward_hoods};
use mandate_sim::{Command, Engine, npc};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::Arc;

pub const STRATEGIES: [&str; 7] = [
    "idle",
    "canvass",
    "media",
    "legislator",
    "mixed",
    "proxy",
    "corrupt",
];

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
    e.apply(e.route(PlayerCommand::Act {
        action: a,
        late_night: false,
    }))
    .is_ok()
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

fn legislate(e: &mut Engine, content: &ContentBundle) {
    if own_bill(e).is_none() {
        let best = {
            let st = e.state();
            let Some(pl) = st.player.as_ref() else { return };
            let hoods = ward_hoods(&st.world, pl.ward);
            let taken: Vec<&str> = st
                .policy
                .in_municipality(pl.municipality.0)
                .filter(|p| !matches!(p.status, PolicyStatus::Ended))
                .map(|p| p.template.as_str())
                .collect();
            content
                .policies
                .values()
                .filter(|t| {
                    t.tier == mandate_content::TierKey::Local
                        && !taken.contains(&t.id.as_str())
                        && mandate_sim::policy::unlocked(st, content, t, pl.municipality.0)
                })
                .max_by(|a, b| {
                    public_support(a, &st.population, &hoods).total_cmp(&public_support(
                        b,
                        &st.population,
                        &hoods,
                    ))
                })
                .map(|t| t.id.clone())
        };
        if let Some(t) = best {
            let r = e.apply(e.route(PlayerCommand::DraftPolicy {
                template: t,
                params: BTreeMap::new(),
            }));
            if let Some(id) = r.ok().and_then(|r| r.result).and_then(|x| x.created) {
                let _ = e.apply(e.route(PlayerCommand::IntroducePolicy { policy: id }));
            }
        }
    }
    if let Some(bill) = own_bill(e) {
        // Lobby the most persuadable colleague, then talk it up in public.
        let target = {
            let st = e.state();
            let me = st.player.as_ref().map(|p| p.politician);
            let inst = &st.policy.instances[bill as usize];
            content.policies.get(&inst.template).and_then(|t| {
                mandate_sim::policy::council_leans(
                    &content.council,
                    &st.world,
                    &st.population,
                    &st.politics,
                    st.meta.seed,
                    t,
                    inst,
                )
                .into_iter()
                .filter(|(p, _)| Some(*p) != me)
                .min_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
                .map(|(p, _)| p.0)
            })
        };
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
                let lean = v.ward_lean(id);
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
            if e.apply(cmd).is_ok() {
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
        let _ = e.apply(e.route(PlayerCommand::Act {
            action: a,
            late_night: late,
        }));
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

pub fn play_turn(e: &mut Engine, content: &ContentBundle, strategy: &str) {
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
    if camp && matches!(strategy, "media" | "mixed" | "corrupt") {
        buy_ads(e, 3);
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
                    coldest_hood(e).is_some_and(|h| act(e, Action::Canvass { neighborhood: h }))
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
            "mixed" | "corrupt" => {
                if tired(e) {
                    act(e, Action::Rest)
                } else if camp {
                    coldest_hood(e).is_some_and(|h| act(e, Action::Canvass { neighborhood: h }))
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
                    Some(AiAction::Canvass) => {
                        coldest_hood(e).is_some_and(|h| act(e, Action::Canvass { neighborhood: h }))
                    }
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
    let mut e = generate(
        &WorldConfig {
            seed,
            size: WorldSize::Small,
            scenario: None,
        },
        content.clone(),
    )
    .ok()?;
    let (crisis, industry) = situation(&e);
    let lost = |crisis: String, industry: String| Played {
        won: false,
        share: 0.0,
        crisis,
        industry,
    };
    let me = e.state().player.as_ref()?.politician;
    let start = e.state().calendar.date;
    for _ in 0..80 {
        if e.state()
            .politics
            .elections
            .last_election
            .is_some_and(|d| d > start)
        {
            break;
        }
        play_turn(&mut e, content, strategy);
        if e.apply(Command::EndTurn).is_err() {
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
pub const VARIETY_MIN_WINNERS: usize = 3;
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
            .filter(|s| **s != "idle")
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
        for (s, v) in row.iter().filter(|(k, _)| **k != "idle") {
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
        for seed in 1..=seeds {
            if let Some(p) = results.get(&(s, seed)) {
                let (won, share) = (p.won, p.share);
                shares.entry(seed).or_default().insert(s, share);
                situations.insert(seed, (p.crisis.clone(), p.industry.clone()));
                n += 1;
                wins += won as u32;
                if s == "idle" {
                    idle_share.insert(seed, share);
                }
                gain += share - idle_share.get(&seed).copied().unwrap_or(share);
            }
        }
        rows.insert(
            s.to_string(),
            json!({ "races": n, "win_rate": wins as f64 / n.max(1) as f64, "mean_share_gain_vs_idle": gain / n.max(1) as f64 }),
        );
    }
    // Dominance check: the best active strategy should not crush the rest.
    let rates: Vec<f64> = STRATEGIES
        .iter()
        .filter(|s| **s != "idle")
        .filter_map(|s| rows.get(*s).and_then(|v| v["win_rate"].as_f64()))
        .collect();
    let best = rates.iter().cloned().fold(0.0, f64::max);
    let median = {
        let mut r = rates.clone();
        r.sort_by(|a, b| a.total_cmp(b));
        r.get(r.len() / 2).copied().unwrap_or(0.0)
    };
    let variety = variety(&shares, &situations);
    json!({
        "seeds": seeds,
        "strategies": rows,
        // DEC-077 §5: the best routine differs across worlds and situations.
        "variety_ok": variety["pass"],
        "variety": variety,
        "best_minus_median": best - median,
        "no_dominant_strategy": best - median <= 0.3,
        "idle_win_rate": rows.get("idle").and_then(|v| v["win_rate"].as_f64()).unwrap_or(1.0),
        // DEC-076: doing nothing should rarely work.
        "idle_rarely_wins": rows.get("idle").and_then(|v| v["win_rate"].as_f64()).unwrap_or(1.0) <= 0.15,
        "effort_matters": rates.iter().cloned().fold(0.0, f64::max)
            > rows.get("idle").and_then(|v| v["win_rate"].as_f64()).unwrap_or(0.0),
    })
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
    let mut e = generate(
        &WorldConfig {
            seed,
            size: WorldSize::Small,
            scenario: None,
        },
        content.clone(),
    )
    .ok()?;
    let me = e.state().player.as_ref()?.politician;
    let start = e.state().calendar.date;
    let end = start.add_days(years * 365);
    let mut imprisoned = false;
    while e.state().calendar.date < end {
        play_turn(&mut e, content, strategy);
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
