//! Lobbying and trading favours with colleagues (DEC-026, DEC-031).

use super::*;
use crate::spine::{Cause, PlayerAct};

/// Lobbying success odds shown to the player before acting (the same
/// formula as NPC deals, `npc::lobby_odds_between`).
pub fn lobby_odds(
    state: &State,
    content: &ContentBundle,
    target: PoliticianId,
    late_night: bool,
) -> f64 {
    let Some(pl) = state.player.as_ref() else {
        return 0.0;
    };
    crate::npc::lobby_odds_between(state, content, pl.politician, target, late_night)
}

/// Lobby a colleague, or trade them a favour, on a bill.
#[allow(clippy::too_many_arguments)]
pub(super) fn lobby(
    state: &mut State,
    content: &ContentBundle,
    pl: &crate::player::PlayerState,
    date: mandate_core::SimDate,
    m: u32,
    action: &Action,
    politician: &u32,
    policy: &u32,
    late_night: &bool,
) -> Result<ActionResult, EngineError> {
    let mut res = ActionResult::default();
    local_policy(state, *policy)?;
    let target = PoliticianId(*politician);
    let t = state
        .politics
        .get(target)
        .ok_or_else(|| bad("no such politician"))?;
    if t.municipality() != m || t.is_player {
        return Err(bad("you can only lobby colleagues in your town"));
    }
    let (tc, me) = (
        t.character,
        state
            .politics
            .get(pl.politician)
            .map(|p| p.character)
            .ok_or_else(|| bad("no player"))?,
    );
    if matches!(action, Action::TradeFavor { .. })
        && characters::favours_owed(&state.politics, me, tc) as i32 >= MAX_FAVOR_DEBT
    {
        return Err(bad("you already owe them too much"));
    }
    let odds = lobby_odds(state, content, target, *late_night);
    let negotiation = state
        .politics
        .get(pl.politician)
        .map(|x| x.negotiation)
        .unwrap_or(0.5) as f64;
    spend(
        state,
        content,
        if matches!(action, Action::TradeFavor { .. }) {
            "trade_favor"
        } else {
            "lobby"
        },
        *late_night,
    )?;
    crate::politics::add_record(
        &mut state.politics,
        pl.politician,
        content.career.record_gain.lobby as f32,
    );
    let rng = state.rng.get(Stream::Player);
    let trade = matches!(action, Action::TradeFavor { .. });
    let success = trade || rng.chance(odds);
    let lp = &content.council.lobby;
    let fx = &content.actions.lobby;
    let gain = if trade {
        fx.trade_gain
    } else if success {
        rng.range_f64(lp.gain[0], lp.gain[1]) * (0.5 + negotiation)
    } else {
        lp.fail_gain
    };
    *state
        .politics
        .sway
        .entry((*politician, *policy))
        .or_default() += gain as f32;
    let subject = Some(Subject::Policy { id: *policy });
    if trade {
        // A promise: the player owes them one (DEC-031 favour ledger).
        characters::owe(&mut state.politics, me, tc, "favour-bill", subject, date);
    }
    let kind = if trade {
        MemoryKind::PromisedFavour
    } else if success {
        MemoryKind::LobbiedMe
    } else {
        MemoryKind::PressedMe
    };
    characters::remember(
        &mut state.politics,
        content,
        date,
        tc,
        me,
        kind,
        subject,
        Some(if success {
            fx.opinion_success
        } else {
            fx.opinion_fail
        }),
    );
    // Lobbying is meeting (DEC-091): `known`; a second
    // meeting, or one at the diner late at night, `detailed`.
    let deep = *late_night
        || state.player.as_ref().is_some_and(|p| {
            p.knowledge
                .level(&knowledge::Subject::Person { person: tc })
                >= Level::Known
        });
    let src = knowledge::action_source(state, action.key());
    knowledge::teach_one(
        state,
        knowledge::Subject::Person { person: tc },
        if deep { Level::Detailed } else { Level::Known },
        src,
    );
    if *late_night && let Some(p) = state.player.as_mut() {
        // Back-room deals carry legal exposure (DEC-006).
        p.tracks.legal_jeopardy = (p.tracks.legal_jeopardy + fx.late_night_legal as f32).min(100.0);
    }
    if *late_night {
        // A back-room deal is a hidden fact (DEC-034): its fate
        // is settled now; the director times when it surfaces.
        let cause = Cause::Player(if trade {
            PlayerAct::TradeFavor
        } else {
            PlayerAct::Lobby
        });
        crate::media::hide(
            state,
            content,
            &content.media.late_night_deal,
            pl.politician,
            vec![crate::narrative::Actor::Person { id: tc }],
            cause,
        );
    }
    res.outcome_key = if trade {
        "outcome-favor-traded"
    } else if success {
        "outcome-lobby-success"
    } else {
        "outcome-lobby-fail"
    }
    .into();
    let name = state
        .politics
        .get(target)
        .map(|t| format!("{} {}", t.given, t.family))
        .unwrap_or_default();
    res.args.insert("name".into(), name);
    Ok(res)
}
