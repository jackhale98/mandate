//! NPC goals (DEC-032): goals emerge from circumstances plus personality.
//! Each month the people of the player's town take up goals whose
//! conditions hold (`goals.jsonc`) and give them up when the conditions fail
//! or after `max_days`. A goal drives the NPC AI through its `drive` (the
//! DEC-030 utility AI, the same rules as the player): a council lean on the
//! player's bills, a lobbying focus against them, or a higher rank for bills
//! that answer the live crisis. When a goal needs the player, the director
//! surfaces its request (a lightweight dilemma).
//!
//! Goals are kept for the player's town only: they are about the player or
//! the town the player can act in. (NPC goals elsewhere are deferred.)

use super::CharacterId;
use crate::narrative::cast::{self, View};
use crate::narrative::{Actor, CastEntry};
use crate::politics::PoliticsState;
use crate::scheduler::Reads;
use mandate_content::{ContentBundle, ContentKey, GoalDef, GoalDrive};
use mandate_core::SimDate;
use serde::{Deserialize, Serialize};

/// A goal someone holds (or held: `until`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Goal {
    pub holder: CharacterId,
    /// Goal content id.
    pub goal: ContentKey,
    pub since: SimDate,
    /// The drive's strength, copied when the goal was taken up.
    pub lean: f32,
    pub champion: f32,
}

/// A goal given up recently (it may not be taken up again for `max_days`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EndedGoal {
    pub holder: CharacterId,
    pub goal: ContentKey,
    pub until: SimDate,
}

fn holder_cast(holder: CharacterId) -> Vec<CastEntry> {
    vec![CastEntry {
        role: "holder".into(),
        actor: Actor::Person { id: holder },
    }]
}

/// Whether `holder` meets a goal's personality, trait, and live conditions.
pub fn qualifies(v: &View<'_>, g: &GoalDef, holder: CharacterId) -> bool {
    let Some(p) = super::profile(v.pol, v.r.world, holder) else {
        return false;
    };
    let x = &p.personality;
    for (axis, [lo, hi]) in &g.personality {
        let val = match axis.as_str() {
            "ambition" => x.ambition,
            "integrity" => x.integrity,
            "warmth" => x.warmth,
            "boldness" => x.boldness,
            _ => x.loyalty,
        } as f64;
        if val < *lo || val > *hi {
            return false;
        }
    }
    if !g.traits.is_empty() && !p.traits.iter().any(|t| g.traits.contains(t)) {
        return false;
    }
    let c = holder_cast(holder);
    g.conditions.iter().all(|cond| cast::holds(v, cond, &c))
}

/// Who may hold a goal in the player's town.
pub fn holders(v: &View<'_>, g: &GoalDef) -> Vec<CharacterId> {
    let mut out: Vec<CharacterId> = Vec::new();
    if let Some(m) = v.town() {
        for c in &v.pol.characters.people {
            if let Some(n) = c.named.as_deref()
                && g.holder.occupations.contains(&n.occupation)
                && v.town_of(c.id) == Some(m)
            {
                out.push(c.id);
            }
        }
    }
    if g.holder.officeholders {
        for p in v.officeholders() {
            if let Some(x) = v.pol.get(p) {
                out.push(x.character);
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Politics domain, monthly: take up and give up goals in the player's town.
pub fn refresh(pol: &mut PoliticsState, r: &Reads<'_>) {
    if r.player.is_none() {
        if !pol.characters.goals.is_empty() {
            pol.characters.goals.clear();
        }
        return;
    }
    let date = r.date;
    let (kept, ended, added) = {
        let v = View::new(r, pol);
        let content = r.content;
        let mut kept = Vec::new();
        let mut ended = Vec::new();
        for g in &pol.characters.goals {
            let Some(def) = content.goal(&g.goal) else {
                continue;
            };
            let expired = g.since.add_days(def.max_days) <= date;
            if !expired && qualifies(&v, def, g.holder) {
                kept.push(g.clone());
            } else {
                ended.push(EndedGoal {
                    holder: g.holder,
                    goal: g.goal.clone(),
                    until: date.add_days(def.max_days),
                });
            }
        }
        let mut added = Vec::new();
        for def in &content.goals {
            for h in holders(&v, def) {
                let held = kept
                    .iter()
                    .any(|g: &Goal| g.holder == h && g.goal == def.id);
                let resting = pol
                    .characters
                    .ended_goals
                    .iter()
                    .chain(ended.iter())
                    .any(|e| e.holder == h && e.goal == def.id && e.until > date);
                if held || resting || !qualifies(&v, def, h) {
                    continue;
                }
                let (lean, champion) = match def.drive {
                    GoalDrive::None => (0.0, 0.0),
                    GoalDrive::OpposePlayer { lean } => (-lean as f32, 0.0),
                    GoalDrive::BackPlayer { lean } => (lean as f32, 0.0),
                    GoalDrive::Champion { bonus } => (0.0, bonus as f32),
                };
                added.push(Goal {
                    holder: h,
                    goal: def.id.clone(),
                    since: date,
                    lean,
                    champion,
                });
            }
        }
        (kept, ended, added)
    };
    pol.characters.goals = kept;
    pol.characters.goals.extend(added);
    pol.characters.ended_goals.retain(|e| e.until > date);
    pol.characters.ended_goals.extend(ended);
}

/// A council member's lean on a bill from their goals (DEC-030: added to the
/// same lean as lobbying sway). Only the player's bills are affected.
pub fn lean_on(pol: &PoliticsState, holder: CharacterId, sponsor_is_player: bool) -> f64 {
    if !sponsor_is_player {
        return 0.0;
    }
    pol.characters
        .goals
        .iter()
        .filter(|g| g.holder == holder)
        .map(|g| g.lean as f64)
        .sum()
}

/// The holder is out to stop the player (an opposing lean).
pub fn opposes_player(pol: &PoliticsState, holder: CharacterId) -> bool {
    pol.characters
        .goals
        .iter()
        .any(|g| g.holder == holder && g.lean < 0.0)
}

/// The holder's extra rank for bills that answer the live crisis.
pub fn champion_bonus(pol: &PoliticsState, holder: CharacterId) -> f64 {
    pol.characters
        .goals
        .iter()
        .filter(|g| g.holder == holder)
        .map(|g| g.champion as f64)
        .sum()
}

/// Drop goals whose template a content update removed.
pub fn on_load(pol: &mut PoliticsState, content: &ContentBundle) {
    pol.characters
        .goals
        .retain(|g| content.goal(&g.goal).is_some());
    pol.characters
        .ended_goals
        .retain(|g| content.goal(&g.goal).is_some());
}
