//! The directed strategy's turn and the explore window.

use super::*;

/// Whether `explorer` is still in its learning turns.
pub(super) fn exploring(e: &Engine, start_turn: u32) -> bool {
    e.state().calendar.turn < start_turn + EXPLORE_TURNS && !campaigning(e)
}

/// The time-skip proxy's turn (DEC-012), by the player's directives (the
/// content's defaults unless set): the `directed` probe plays every turn
/// this way, as a career skipped from start to finish would.
pub fn directed_turn(e: &mut Engine) {
    let mut t = mandate_sim::proxy::ProxyTurn::new(e.state(), e.content());
    while let Some(cmd) = t.next(e.state(), e.content()) {
        let r = apply(e, Command::Player(cmd));
        let created = r
            .as_ref()
            .ok()
            .and_then(|o| o.result.as_ref())
            .and_then(|x| x.created);
        t.result(r.is_ok(), created);
    }
}
