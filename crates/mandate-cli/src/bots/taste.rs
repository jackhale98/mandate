//! Per-strategy taste: how a bot scores a dilemma's options, and its shady actions.

use super::*;

/// How a strategy weighs an option's effects (DEC-073): a simple,
/// deterministic policy per strategy for answering decisions.
pub(super) struct Taste {
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

pub(super) fn taste(strategy: &str) -> Taste {
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

pub(super) fn option_score(
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
            // A bot's taste has no term for the town's air or its industry.
            EffectDef::Inspection { .. } => 0.0,
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
pub(super) fn shady_actions(e: &mut Engine, camp: bool) {
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
