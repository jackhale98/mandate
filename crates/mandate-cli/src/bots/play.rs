//! Playing a turn per strategy (the dispatcher) and playing a whole game.

use super::*;

/// Play a strategy's turn and end it.
///
/// `directed` is a career skipped from start to finish (DEC-102 clause 7):
/// each of its turns is one turn of a real time skip (`Command::Skip` to
/// the furthest point on offer), so every limit a skip has is the probe's
/// too: it learns nothing, answers no ask, and a stop is skipped on from.
/// Where no skip is on offer (election day, prison) it falls back to the
/// proxy's commands and `EndTurn`.
pub fn turn(
    e: &mut Engine,
    content: &ContentBundle,
    strategy: &str,
    start_turn: u32,
) -> Result<(), EngineError> {
    if strategy == "directed" {
        let target = mandate_sim::skip::targets(e.state(), e.content())
            .last()
            .map(|t| t.target);
        if let Some(target) = target {
            let before = e.state().calendar.turn;
            let r = apply(
                e,
                Command::Skip {
                    target,
                    limit: Some(1),
                },
            );
            if r.is_ok() && e.state().calendar.turn > before {
                return Ok(());
            }
        }
    }
    play_turn_from(e, content, strategy, start_turn);
    apply(e, Command::EndTurn).map(|_| ())
}

/// A bot's turn; `start_turn` is the turn play began (for `explorer`).
/// (`directed` here is the proxy's commands alone, without a skip's
/// limits: the tournament plays it through [`turn`].)
pub fn play_turn_from(e: &mut Engine, content: &ContentBundle, strategy: &str, start_turn: u32) {
    if strategy == "directed" {
        directed_turn(e);
        return;
    }
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
    // Asks first (ENGAGEMENT §3.9): answer the offers, make the campaign's
    // promises, then work what was promised.
    answer_asks(e, content, strategy);
    make_promises(e, content, strategy);
    work_asks(e, content, strategy);
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
            // `asks_only` works every ask (above) and does nothing else.
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
pub(super) fn situation(e: &Engine) -> (String, String) {
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
        if turn(&mut e, content, strategy, start_turn).is_err() {
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
