//! The ground game's memory (DEC-097 §1; ENGAGEMENT §2.1): how often, and
//! how lately, the player has canvassed each neighbourhood. A canvass builds
//! name recognition first; its approval fades fast when the same doorsteps
//! are knocked again within weeks, and counts for more in campaign season.
//!
//! The curve is one pure function ([`canvass_curve`]) shared by the command
//! and any preview, so what the player is told is what happens.

use crate::state::State;
use crate::world::NeighborhoodId;
use mandate_content::CanvassEffects;
use mandate_core::SimDate;
use serde::{Deserialize, Serialize};

/// Per-neighbourhood canvass memory (player state; DEC-097 §1).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GroundState {
    /// Neighbourhoods the player has canvassed, by id (ascending).
    #[serde(default)]
    pub hoods: Vec<HoodVisits>,
    /// The turn of the player's last canvass (the same-turn cut).
    #[serde(default)]
    pub last_turn: Option<u32>,
}

/// One neighbourhood's canvass memory: the decayed count of visits as of
/// the last one, and its date.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HoodVisits {
    pub hood: NeighborhoodId,
    pub visits: f64,
    pub last: SimDate,
}

impl GroundState {
    /// Visits to `hood` as remembered `on` a date: each one fades by `keep`
    /// a month (30 days).
    pub fn visits_now(&self, hood: NeighborhoodId, on: SimDate, keep: f64) -> f64 {
        self.hoods
            .iter()
            .find(|h| h.hood == hood)
            .map(|h| {
                let months = h.last.days_until(on) as f64 / 30.0;
                h.visits * mandate_core::math::pow(keep, months)
            })
            .unwrap_or(0.0)
    }

    /// Remember a canvass of `hood` (`visits` already includes this one).
    pub fn remember(&mut self, hood: NeighborhoodId, visits: f64, on: SimDate, turn: u32) {
        match self.hoods.binary_search_by(|h| h.hood.cmp(&hood)) {
            Ok(i) => {
                self.hoods[i].visits = visits;
                self.hoods[i].last = on;
            }
            Err(i) => self.hoods.insert(
                i,
                HoodVisits {
                    hood,
                    visits,
                    last: on,
                },
            ),
        }
        self.last_turn = Some(turn);
    }
}

/// What a canvass of a neighbourhood is worth now (ENGAGEMENT §2.1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Curve {
    /// Visits remembered there before this one.
    pub visits: f64,
    /// How fresh the doorsteps are (1: never canvassed; falls with each
    /// recent visit).
    pub fresh: f64,
    /// The campaign-season multiplier (1 unless the player is a candidate
    /// in an open race).
    pub season: f64,
    /// The cut for a second canvass in one turn (1 for the first).
    pub same_turn: f64,
    /// The share of the awareness gap a canvass closes, before its own
    /// `awareness` rate: `max(fresh, awareness_floor)`.
    pub awareness: f64,
    /// The multiplier on the approval gain (× fit × room × `approval`):
    /// `fresh × season × same_turn`.
    pub approval: f64,
}

/// Whether the player is a candidate in an open race (campaign season).
pub fn candidate(state: &State) -> bool {
    let Some(me) = state.player.as_ref().map(|p| p.politician) else {
        return false;
    };
    state
        .politics
        .elections
        .races
        .iter()
        .any(|r| r.candidates.iter().any(|c| c.politician == me))
}

/// The pure curve: `visits` remembered, in or out of campaign season,
/// with or without a canvass already this turn.
pub fn curve(fx: &CanvassEffects, visits: f64, campaign: bool, again: bool) -> Curve {
    let decay = if campaign {
        fx.hood_decay_campaign
    } else {
        fx.hood_decay
    };
    let fresh = mandate_core::math::pow(decay, visits);
    let season = if campaign { fx.campaign_mult } else { 1.0 };
    let same_turn = if again { fx.same_turn_mult } else { 1.0 };
    Curve {
        visits,
        fresh,
        season,
        same_turn,
        awareness: fresh.max(fx.awareness_floor),
        approval: fresh * season * same_turn,
    }
}

/// What a canvass of `hood` is worth now, for the player (the command and
/// any preview share it).
pub fn canvass_curve(state: &State, fx: &CanvassEffects, hood: NeighborhoodId) -> Curve {
    let Some(pl) = state.player.as_ref() else {
        return curve(fx, 0.0, false, false);
    };
    let visits = pl
        .ground
        .visits_now(hood, state.calendar.date, fx.keep);
    let again = pl.ground.last_turn == Some(state.calendar.turn);
    curve(fx, visits, candidate(state), again)
}

/// The doorsteps' freshness in words (`new` | `recent` | `tired`), for the
/// outcome line (DEC-051 "Why?").
pub fn fresh_band(fresh: f64) -> &'static str {
    if fresh >= 0.7 {
        "new"
    } else if fresh >= 0.3 {
        "recent"
    } else {
        "tired"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fx() -> CanvassEffects {
        CanvassEffects {
            awareness: 0.45,
            approval: 0.012,
            contact: 1.6,
            hood_decay: 0.5,
            hood_decay_campaign: 0.75,
            keep: 0.6,
            campaign_mult: 2.0,
            same_turn_mult: 0.5,
            awareness_floor: 0.5,
        }
    }

    /// Canvass `pattern` (neighbourhood per visit) every `days`, and return
    /// the freshness of the last visit's doorsteps.
    fn steady(pattern: &[u32], days: u32, campaign: bool, visits: usize) -> f64 {
        let fx = fx();
        let mut g = GroundState::default();
        let mut on = SimDate(1000);
        let mut fresh = 1.0;
        for (turn, i) in (0..visits).enumerate() {
            let hood = NeighborhoodId(pattern[i % pattern.len()]);
            let v = g.visits_now(hood, on, fx.keep);
            fresh = curve(&fx, v, campaign, false).fresh;
            g.remember(hood, v + 1.0, on, turn as u32);
            on = on.add_days(days);
        }
        fresh
    }

    #[test]
    fn the_same_neighbourhood_every_turn_goes_stale() {
        let f = steady(&[3], 14, false, 60);
        assert!(f < 0.1, "fresh {f}");
    }

    #[test]
    fn a_rotation_of_three_stays_half_fresh() {
        let f = steady(&[1, 2, 3], 14, false, 60);
        assert!((0.4..=0.6).contains(&f), "fresh {f}");
        // In a campaign, weekly turns: about as fresh (doorsteps are expected).
        let c = steady(&[1, 2, 3], 7, true, 60);
        assert!((0.4..=0.6).contains(&c), "campaign fresh {c}");
    }

    #[test]
    fn a_first_visit_is_fresh_and_awareness_has_a_floor() {
        let fx = fx();
        let first = curve(&fx, 0.0, false, false);
        assert_eq!(first.fresh, 1.0);
        assert_eq!(first.approval, 1.0);
        assert_eq!(first.awareness, 1.0);
        let stale = curve(&fx, 6.0, false, false);
        assert!(stale.fresh < 0.1);
        assert_eq!(stale.awareness, fx.awareness_floor);
    }

    #[test]
    fn the_campaign_multiplier_and_the_same_turn_cut() {
        let fx = fx();
        let out = curve(&fx, 1.0, false, false);
        let camp = curve(&fx, 1.0, true, false);
        assert_eq!(out.season, 1.0);
        assert_eq!(camp.season, fx.campaign_mult);
        assert!(camp.fresh > out.fresh);
        assert_eq!(camp.approval, camp.fresh * fx.campaign_mult);
        let again = curve(&fx, 1.0, false, true);
        assert_eq!(again.approval, out.approval * fx.same_turn_mult);
    }

    #[test]
    fn memory_fades_by_the_month() {
        let fx = fx();
        let mut g = GroundState::default();
        g.remember(NeighborhoodId(4), 2.0, SimDate(100), 0);
        assert_eq!(g.visits_now(NeighborhoodId(4), SimDate(100), fx.keep), 2.0);
        let later = g.visits_now(NeighborhoodId(4), SimDate(130), fx.keep);
        assert!((later - 1.2).abs() < 1e-9, "{later}");
        assert_eq!(g.visits_now(NeighborhoodId(5), SimDate(130), fx.keep), 0.0);
        // Kept sorted by neighbourhood id.
        g.remember(NeighborhoodId(1), 1.0, SimDate(130), 1);
        assert_eq!(g.hoods[0].hood, NeighborhoodId(1));
        assert_eq!(g.last_turn, Some(1));
    }
}
