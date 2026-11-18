//! The player (DEC-011, DEC-013): day-part slots, Mandate, resources, and the
//! pressure tracks (DEC-006). Player commands are the Commands phase: they may
//! write the immediate effects of an action into other domains (sway,
//! awareness), which is the only cross-domain write outside effects.

use crate::politics::PoliticianId;
use crate::population::CLASSES;
use crate::scheduler::{Outs, Reads};
use crate::world::{ConstituencyId, MunicipalityId};
use mandate_core::math::clamp;
use mandate_core::{Money, SimDate};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tracks {
    /// Electability and backing (0..100; 0 = political wilderness).
    pub viability: f32,
    /// Legal exposure (0..100; 100 = prosecution).
    pub legal_jeopardy: f32,
    /// Personal security risk (0..100).
    pub threat: f32,
    /// Public trust and scandal exposure (0..100; 0 = disgrace).
    pub standing: f32,
    /// Health (0..100).
    pub health: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActionRecord {
    pub date: SimDate,
    /// Fluent key of the action, e.g. `action-lobby`.
    pub action: String,
    /// Fluent key of the outcome.
    pub outcome: String,
    pub args: Vec<(String, String)>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlayerState {
    pub politician: PoliticianId,
    pub municipality: MunicipalityId,
    pub ward: ConstituencyId,
    pub standard_slots: u8,
    pub slots_used: u8,
    pub late_night_used: bool,
    /// Public legitimacy behind the player (0..100), DEC-011.
    pub mandate: f32,
    pub money: Money,
    pub staff: u8,
    pub tracks: Tracks,
    pub actions: Vec<ActionRecord>,
    /// What the player intends at the next local election (default: defend their seat).
    #[serde(default)]
    pub filing: Option<crate::politics::elections::Filing>,
    /// Ad buys this turn (the local market saturates).
    #[serde(default)]
    pub ads_this_turn: u8,
    /// The last election whose result has been applied to the tracks.
    #[serde(default)]
    pub last_election_seen: Option<SimDate>,
}

impl PlayerState {
    /// Slots available this turn: exhaustion costs time (health is a resource).
    pub fn slot_cap(&self) -> u8 {
        let tired = (self.tracks.health < 35.0) as u8 + (self.tracks.health < 15.0) as u8;
        self.standard_slots.saturating_sub(tired).max(1)
    }

    pub fn slots_left(&self) -> u8 {
        self.slot_cap().saturating_sub(self.slots_used)
    }

    pub fn log(&mut self, date: SimDate, action: &str, outcome: &str, args: Vec<(String, String)>) {
        self.actions.push(ActionRecord {
            date,
            action: action.into(),
            outcome: outcome.into(),
            args,
        });
        if self.actions.len() > 40 {
            self.actions.remove(0);
        }
    }
}

/// Monthly: Mandate from the player's standing with their own voters, and
/// slow movement of the pressure tracks.
pub fn monthly(pl: &mut PlayerState, r: &Reads<'_>, _out: &mut Outs<'_>) {
    let pop = r.population();
    let hoods = crate::policy::ward_hoods(r.world, pl.ward);
    let mut acc = 0.0;
    let mut w = 0.0;
    for h in hoods {
        for c in &pop.hoods[h].cohorts {
            let weight = c.count as f64 * c.turnout as f64;
            acc += (c.player_approval as f64) * (c.player_awareness as f64) * weight;
            w += weight;
        }
    }
    let net = if w > 0.0 { acc / w } else { 0.0 };
    let target = clamp(50.0 + 100.0 * net, 0.0, 100.0);
    pl.mandate = (pl.mandate as f64 * 0.8 + target * 0.2) as f32;
    pl.tracks.viability = clamp(
        pl.tracks.viability as f64 * 0.9
            + (pl.mandate as f64 * 0.7 + pl.tracks.standing as f64 * 0.3) * 0.1,
        0.0,
        100.0,
    ) as f32;
    pl.tracks.health = (pl.tracks.health + 2.0).min(100.0);
    // The party notices a passenger, and rewards a worker (DEC-076).
    let record = r
        .politics()
        .get(pl.politician)
        .map(|p| p.record as f64)
        .unwrap_or(0.0);
    let d = if record < 0.2 {
        -1.5
    } else if record > 0.5 {
        0.5
    } else {
        0.0
    };
    pl.tracks.standing = clamp(pl.tracks.standing as f64 + d, 0.0, 100.0) as f32;
    // An election result lands on the career: margins become Mandate (DEC-011).
    let pol = r.politics();
    if pol.elections.last_election.is_some() && pol.elections.last_election != pl.last_election_seen
    {
        pl.last_election_seen = pol.elections.last_election;
        let date = pol.elections.last_election;
        if let Some(res) = pol
            .elections
            .results
            .iter()
            .filter(|x| Some(x.date) == date)
            .find(|x| x.candidates.iter().any(|c| c.politician == pl.politician))
        {
            let mine = res
                .candidates
                .iter()
                .find(|c| c.politician == pl.politician)
                .map(|c| c.share)
                .unwrap_or(0.0) as f64;
            let best_other = res
                .candidates
                .iter()
                .filter(|c| c.politician != pl.politician)
                .map(|c| c.share as f64)
                .fold(0.0, f64::max);
            let margin = mine - best_other;
            if res.winner == pl.politician {
                pl.mandate = clamp(55.0 + 150.0 * margin, 40.0, 100.0) as f32;
                pl.tracks.viability = clamp(pl.tracks.viability as f64 + 10.0, 0.0, 100.0) as f32;
                pl.tracks.standing = clamp(pl.tracks.standing as f64 + 5.0, 0.0, 100.0) as f32;
            } else {
                pl.tracks.viability = clamp(
                    pl.tracks.viability as f64 - 25.0 + 50.0 * margin.max(-0.5),
                    0.0,
                    100.0,
                ) as f32;
                pl.tracks.standing = clamp(pl.tracks.standing as f64 - 10.0, 0.0, 100.0) as f32;
            }
        }
        // A new cycle: by default the player defends whatever they hold.
        pl.filing = None;
    }
    let _ = CLASSES;
}
