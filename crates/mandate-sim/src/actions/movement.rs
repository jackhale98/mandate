//! Moving within the town (DEC-079).

use super::*;

/// Move within the town: free (DEC-079 rule 1). Arriving teaches lightly
/// (DEC-091: a place's introduction, overheard names and rumours).
pub(super) fn move_to(
    state: &mut State,
    content: &ContentBundle,
    location: crate::places::Location,
) -> Result<ActionResult, EngineError> {
    crate::places::validate_move(state, location)?;
    let kind = crate::places::kind_key(state, location);
    if let Some(p) = state.player.as_mut() {
        p.location = location;
    }
    crate::places::arrive(state, content, location);
    if let crate::places::Location::Place { place } = location {
        crate::asks::record_deed(state, crate::asks::Deed::Arrive { place });
    }
    let mut res = ActionResult {
        outcome_key: "outcome-moved".into(),
        ..Default::default()
    };
    res.args.insert("kind".into(), kind.into());
    Ok(res)
}
