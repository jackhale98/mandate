//! The informed styles' turns: campaigner, legislator and coalition.

use super::*;

/// Bots ask for support only when the shown odds are at least this.
pub const SUPPORT_MIN_ODDS: f64 = 0.3;

// --- The informed styles (ENGAGEMENT §2.4) ----------------------------------------

/// The fit the player expects from canvassing a neighbourhood on its top
/// worry, from their own last reading of its doorsteps (a `detailed`
/// reading is the player's knowledge, DEC-091): the canvass match formula
/// on the reading's top worry over its mean. Without a reading, 1.
pub(super) fn expected_fit(e: &Engine, content: &ContentBundle, hood: u32) -> f64 {
    let Some(pl) = e.state().player.as_ref() else {
        return 1.0;
    };
    let s = mandate_sim::player::knowledge::Subject::HoodConcerns {
        hood: NeighborhoodId(hood),
    };
    let Some(r) = pl.knowledge.entry(&s).and_then(|x| x.reading.as_ref()) else {
        return 1.0;
    };
    let vals: Vec<f64> = r.values.iter().map(|v| v.1 as f64).collect();
    let mean = vals.iter().sum::<f64>() / vals.len().max(1) as f64;
    let top = vals.iter().copied().fold(0.0, f64::max);
    if mean <= 0.0 {
        return 1.0;
    }
    let k = &content.knowledge.effects.canvass;
    (1.0 + k.match_bonus * (top / mean - 1.0)).clamp(k.floor, k.ceiling)
}

/// Mean awareness of the player in a neighbourhood (never gated).
pub(super) fn hood_awareness(e: &Engine, h: usize) -> f64 {
    let cs = &e.state().population.hoods[h].cohorts;
    cs.iter()
        .map(|c| c.player_awareness as f64 * c.count as f64)
        .sum::<f64>()
        / cs.iter().map(|c| c.count as f64).sum::<f64>().max(1.0)
}

/// The ward neighbourhood where a canvass is worth most now, by what the
/// player knows: how fresh its doorsteps are (their own memory of where
/// they have been), the fit they expect on its top worry, and how far they
/// are from being known there. Returns the hood and its approval
/// multiplier (fresh × season).
pub(super) fn ground_hood(e: &Engine, content: &ContentBundle) -> Option<(u32, f64)> {
    let st = e.state();
    let pl = st.player.as_ref()?;
    let fx = &content.actions.canvass;
    ward_hoods(&st.world, pl.ward)
        .into_iter()
        .map(|h| {
            let c = mandate_sim::player::ground::canvass_curve(st, fx, NeighborhoodId(h as u32));
            let gap = 1.0 - hood_awareness(e, h);
            let value = c.approval * expected_fit(e, content, h as u32) + gap * c.awareness;
            (value, h as u32, c.approval)
        })
        .max_by(|a, b| a.0.total_cmp(&b.0).then(b.1.cmp(&a.1)))
        .map(|x| (x.1, x.2))
}

/// The campaigner (the ground game where it fits, and the press): between
/// elections it canvasses only doorsteps that are fresh (where it pays
/// most, by what the player knows), and otherwise works a bill and raises
/// money for the campaign; in a campaign, the doorsteps where a canvass is
/// worth most, then people it has met asked to speak up, then the press.
pub(super) fn campaigner_turn(
    e: &mut Engine,
    content: &ContentBundle,
    camp: bool,
    slots: u8,
) -> bool {
    if tired(e) {
        return act(e, Action::Rest);
    }
    if slots >= 2
        && let Some((h, worth)) = ground_hood(e, content)
        && (camp || worth >= CAMPAIGNER_FRESH)
    {
        return canvass(e, h);
    }
    if camp {
        let policy = own_bill(e);
        return ask_support(e, content) || act(e, Action::Press { policy });
    }
    legislate(e, content);
    // Between elections, the odd slot goes on being seen at a works
    // gate where that is safe (places with purpose), else on the donors.
    photo_op(e, content) || act(e, Action::Fundraise)
}

/// Odds of a photo op going wrong the campaigner accepts.
pub const CAMPAIGNER_PHOTO_RISK: f64 = 0.15;

/// A photo op at a works gate in the player's ward, with the odds shown
/// at most `CAMPAIGNER_PHOTO_RISK` (the shown odds, DEC-073), once a turn
/// (the player's own log says whether there was one today).
pub(super) fn photo_op(e: &mut Engine, content: &ContentBundle) -> bool {
    let verb = "verb.photo_op_gate";
    let pick = {
        let st = e.state();
        let Some(pl) = st.player.as_ref() else {
            return false;
        };
        let today = st.calendar.date;
        if pl
            .actions
            .iter()
            .any(|a| a.date == today && a.action == "action-verb-photo-op-gate")
        {
            return false;
        }
        let Some(def) = content.actions.visit.verb(verb) else {
            return false;
        };
        let ward = ward_hoods(&st.world, pl.ward);
        st.world
            .places
            .iter()
            .filter(|p| p.municipality == pl.home() && ward.contains(&p.neighborhood.index()))
            .map(|p| Location::Place { place: p.id })
            .find(|loc| {
                let Location::Place { place } = *loc else {
                    return false;
                };
                mandate_sim::visits::verbs_at(st, content, *loc)
                    .iter()
                    .any(|v| v.id == verb)
                    && mandate_sim::visits::availability(st, content, *loc, def, false).is_ok()
                    && mandate_sim::visits::risk(st, content, place, def)
                        .is_some_and(|r| r <= CAMPAIGNER_PHOTO_RISK)
            })
    };
    let Some(location) = pick else {
        return false;
    };
    let then = PlayerCommand::Act {
        action: Action::Visit { verb: verb.into() },
        late_night: false,
    };
    let cmd = if e.state().player.as_ref().map(|p| p.location) == Some(location) {
        then
    } else {
        PlayerCommand::Go {
            location,
            then: Some(Box::new(then)),
        }
    };
    apply(e, Command::Player(cmd)).is_ok()
}

/// Between elections the campaigner canvasses only doorsteps at least this
/// fresh.
pub const CAMPAIGNER_FRESH: f64 = 0.7;

/// The legislator (bills and delivery): always a bill in council, lobbied,
/// taken to a town hall and to the press; in a campaign, the doorsteps,
/// then people it has met asked to speak up, then a town hall on its bill
/// where the player is least known.
pub(super) fn legislator_turn(
    e: &mut Engine,
    content: &ContentBundle,
    camp: bool,
    slots: u8,
) -> bool {
    if tired(e) {
        return act(e, Action::Rest);
    }
    if camp {
        if slots >= 2
            && let Some(h) = coldest_hood(e)
        {
            return canvass(e, h);
        }
        if ask_support(e, content) {
            return true;
        }
        return coldest_hood(e).is_some_and(|h| {
            act(
                e,
                Action::TownHall {
                    neighborhood: h,
                    policy: own_bill(e),
                },
            )
        });
    }
    legislate(e, content);
    let policy = own_bill(e);
    act(e, Action::Press { policy }) || act(e, Action::Fundraise)
}

/// The coalition builder (talk, lobbying, favours): it meets the ward's
/// residents on its walks, lobbies colleagues on its bill, and asks the
/// people it has met to speak up for it, between elections and in a
/// campaign (the asks first, then the doorsteps).
pub(super) fn coalition_turn(
    e: &mut Engine,
    content: &ContentBundle,
    camp: bool,
    slots: u8,
) -> bool {
    if tired(e) {
        return act(e, Action::Rest);
    }
    meet_passers_by(e, content, false);
    if camp {
        if ask_support(e, content) {
            return true;
        }
        return slots >= 2 && coldest_hood(e).is_some_and(|h| canvass(e, h));
    }
    if own_bill(e).is_none() {
        legislate(e, content);
    } else if let Some(bill) = own_bill(e)
        && let Some(t) = lobby_target(e, content, bill)
    {
        lobby(e, content, t, bill);
    }
    ask_support(e, content) || act(e, Action::Fundraise)
}

// --- Asks and promises (DEC-097 §2; ENGAGEMENT §3.9) ---------------------------------------
