//! Powers: what the player's posts allow (REQ-009, DEC-098), party lines and wards.

use super::*;

/// Powers the player holds through their current role (REQ-009: read from the constitution).
pub fn player_powers(state: &State, content: &ContentBundle) -> Vec<Power> {
    let Some(pl) = &state.player else {
        return Vec::new();
    };
    let Some(me) = state.politics.get(pl.politician) else {
        return Vec::new();
    };
    powers_for(content, &state.meta.constitution, me.role)
}

/// Powers a role holds under a constitution: its post's, as the
/// constitution lists them (DEC-098). Out of office: none, only
/// campaigning and networking.
pub fn powers_for(content: &ContentBundle, constitution: &str, role: Role) -> Vec<Power> {
    let (Some(c), Some(post)) = (content.constitutions.get(constitution), role.post()) else {
        return Vec::new();
    };
    crate::politics::powers_of(c, post).to_vec()
}

/// Whether the player holds `power` for a jurisdiction (DEC-098).
pub fn player_holds_power_at(
    state: &State,
    content: &ContentBundle,
    power: Power,
    at: crate::world::Jurisdiction,
) -> bool {
    let (Some(pl), Some(c)) = (
        &state.player,
        content.constitutions.get(&state.meta.constitution),
    ) else {
        return false;
    };
    crate::politics::holds_power_at(&state.politics, c, pl.politician, power, at)
}

/// What the player's party expects on a bill: +1 yes, −1 no, 0 free vote,
/// with why (the party line: its platform against the bill, DEC-027).
pub fn party_stance(
    state: &State,
    content: &ContentBundle,
    policy: u32,
) -> Option<crate::politics::party_line::Stance> {
    let pl = state.player.as_ref()?;
    crate::politics::party_line::player_stance(
        content,
        &state.politics,
        &state.policy,
        &state.meta.constitution,
        pl.politician,
        policy,
    )
}

/// The party line as a sign: +1 yes, −1 no, 0 free vote.
pub fn party_line(state: &State, content: &ContentBundle, policy: u32) -> i8 {
    party_stance(state, content, policy).map_or(0, |s| s.line)
}

/// The district of the seat a politician holds, if any.
pub fn ward_of(state: &State, politician: PoliticianId) -> Option<ConstituencyId> {
    state.politics.get(politician)?.role.constituency()
}
