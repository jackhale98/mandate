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

pub const STRATEGIES: [&str; 6] = ["idle", "canvass", "media", "legislator", "mixed", "proxy"];

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
                    t.tier == mandate_content::TierKey::Local && !taken.contains(&t.id.as_str())
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

fn play_turn(e: &mut Engine, content: &ContentBundle, strategy: &str) {
    let camp = campaigning(e);
    // Money, not time: ads take no slot.
    if camp && matches!(strategy, "media" | "mixed") {
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
            "mixed" => {
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
        e.apply(Command::EndTurn).ok()?;
    }
    let st = e.state();
    let date = st.politics.elections.last_election?;
    let r = st
        .politics
        .elections
        .results
        .iter()
        .find(|r| r.date == date && r.candidates.iter().any(|c| c.politician == me))?;
    let share = r.candidates.iter().find(|c| c.politician == me)?.share as f64;
    Some((r.winner == me, share))
}

pub fn run(content: &Arc<ContentBundle>, seeds: u64) -> serde_json::Value {
    let mut rows = BTreeMap::new();
    let mut idle_share: BTreeMap<u64, f64> = BTreeMap::new();
    for s in STRATEGIES {
        let mut wins = 0;
        let mut n = 0;
        let mut gain = 0.0;
        for seed in 1..=seeds {
            if let Some((won, share)) = play(content, seed, s) {
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
    json!({
        "seeds": seeds,
        "strategies": rows,
        "best_minus_median": best - median,
        "no_dominant_strategy": best - median <= 0.3,
        "idle_win_rate": rows.get("idle").and_then(|v| v["win_rate"].as_f64()).unwrap_or(1.0),
        // DEC-076: doing nothing should rarely work.
        "idle_rarely_wins": rows.get("idle").and_then(|v| v["win_rate"].as_f64()).unwrap_or(1.0) <= 0.15,
        "effort_matters": rates.iter().cloned().fold(0.0, f64::max)
            > rows.get("idle").and_then(|v| v["win_rate"].as_f64()).unwrap_or(0.0),
    })
}
