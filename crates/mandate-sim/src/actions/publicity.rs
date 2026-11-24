//! Press statements, fundraising and advertising.

use super::*;
use crate::spine::{Cause, PlayerAct};

/// Chance a press statement turns into a gaffe (shown before acting).
pub fn press_gaffe_risk(state: &State, content: &ContentBundle) -> f64 {
    let oratory = state
        .player
        .as_ref()
        .and_then(|p| state.politics.get(p.politician))
        .map(|p| p.oratory as f64)
        .unwrap_or(0.5);
    let p = &content.actions.press;
    clamp(
        p.gaffe_base + p.gaffe_oratory * (1.0 - oratory),
        p.gaffe_min,
        p.gaffe_max,
    )
}

/// Make a statement to the local press, optionally about a bill.
pub(super) fn press(
    state: &mut State,
    content: &ContentBundle,
    pl: &crate::player::PlayerState,
    m: u32,
    policy: &Option<u32>,
) -> Result<ActionResult, EngineError> {
    let mut res = ActionResult::default();
    if let Some(id) = policy {
        local_policy(state, *id)?;
    }
    let risk = press_gaffe_risk(state, content);
    spend(state, content, "press", false)?;
    crate::politics::add_record(
        &mut state.politics,
        pl.politician,
        content.career.record_gain.press as f32,
    );
    let fx = &content.actions.press;
    let gaffe = state.rng.get(Stream::Player).chance(risk);
    let hoods: Vec<usize> = state.world.hoods_of(MunicipalityId(m)).collect();
    for h in hoods {
        for c in state.population.hoods[h].cohorts.iter_mut() {
            c.player_awareness = clamp(c.player_awareness as f64 + fx.awareness, 0.0, 1.0) as f32;
            if gaffe {
                c.player_approval = clamp(
                    c.player_approval as f64 - fx.gaffe_approval * c.player_awareness as f64,
                    -1.0,
                    1.0,
                ) as f32;
            }
        }
    }
    if let Some(id) = policy
        && let Some(p) = state.policy.instances.get_mut(*id as usize)
    {
        p.awareness = clamp(p.awareness as f64 + fx.bill_awareness, 0.0, 1.0) as f32;
    }
    if !gaffe {
        campaign_contact(state, content, None, fx.contact);
    }
    // Outlets decide whether and how to cover it (DEC-034).
    crate::media::statement(state, content, pl.politician, *policy);
    crate::asks::record_deed(
        state,
        crate::asks::Deed::Statement {
            about: match policy {
                Some(id) => crate::asks::StatementAbout::Bill { policy: *id },
                None => crate::asks::StatementAbout::General,
            },
        },
    );
    res.outcome_key = if gaffe {
        "outcome-press-gaffe"
    } else {
        "outcome-press"
    }
    .into();
    Ok(res)
}

/// Raise campaign money (late at night, with exposure).
pub(super) fn fundraise(
    state: &mut State,
    content: &ContentBundle,
    pl: &crate::player::PlayerState,
    m: u32,
    late_night: &bool,
) -> Result<ActionResult, EngineError> {
    let mut res = ActionResult::default();
    spend(state, content, "fundraise", *late_night)?;
    let oratory = state
        .politics
        .get(pl.politician)
        .map(|x| x.oratory)
        .unwrap_or(0.5) as f64;
    let fx = &content.actions.fundraise;
    let raised = state
        .rng
        .get(Stream::Player)
        .range_f64(fx.raised[0], fx.raised[1])
        * (0.5 + oratory);
    let money = Money::from_units(raised as i64);
    if let Some(p) = state.player.as_mut() {
        p.money += money;
        // The party likes a fundraiser; late-night money invites scrutiny.
        p.tracks.standing = (p.tracks.standing + fx.standing as f32).min(100.0);
        if *late_night {
            p.tracks.legal_jeopardy =
                (p.tracks.legal_jeopardy + fx.late_night_legal as f32).min(100.0);
        }
    }
    if *late_night {
        let cause = Cause::Player(PlayerAct::Fundraise);
        crate::media::hide(
            state,
            content,
            &content.media.late_night_fundraising,
            pl.politician,
            Vec::new(),
            cause,
        );
    }
    // Donor ties: low-income voters who know you notice who you court.
    let hoods: Vec<usize> = state.world.hoods_of(MunicipalityId(m)).collect();
    for h in hoods {
        for (_, c) in state.population.hoods[h]
            .cohorts
            .iter_mut()
            .enumerate()
            .filter(|(ci, _)| crate::population::class_of(*ci) == 0)
        {
            c.player_approval = clamp(
                c.player_approval as f64 - fx.donor_ties * c.player_awareness as f64,
                -1.0,
                1.0,
            ) as f32;
        }
    }
    // The party office (DEC-091): whoever is found there is
    // met, and there is talk of the rival's plans.
    let src = knowledge::action_source(state, "fundraise");
    let met: Vec<characters::CharacterId> = state
        .player
        .as_ref()
        .map(|p| characters::ambient::stored_at(state, content, p.location))
        .unwrap_or_default();
    let mut grants: Vec<knowledge::Grant> = met
        .into_iter()
        .filter(|c| Some(*c) != characters::player_character(state))
        .map(|person| knowledge::Grant {
            subject: knowledge::Subject::Person { person },
            level: Level::Known,
        })
        .collect();
    grants.push(knowledge::Grant {
        subject: knowledge::Subject::RivalPlans {
            town: MunicipalityId(m),
        },
        level: Level::Heard,
    });
    knowledge::teach(state, grants, &src);
    res.outcome_key = "outcome-fundraise".into();
    res.args.insert("amount".into(), money.units().to_string());
    Ok(res)
}

/// Buy local advertising for the campaign.
pub(super) fn advertise(
    state: &mut State,
    content: &ContentBundle,
    pl: &crate::player::PlayerState,
) -> Result<ActionResult, EngineError> {
    let mut res = ActionResult::default();
    let cost = Money::from_units(content.politics.campaign.ad_cost);
    if state.player.as_ref().is_none_or(|p| p.money < cost) {
        return Err(bad("not enough campaign funds"));
    }
    let me = pl.politician;
    if !state
        .politics
        .elections
        .races
        .iter()
        .any(|r| r.candidates.iter().any(|c| c.politician == me))
    {
        return Err(bad("you are not running in a campaign"));
    }
    // Money, not time: no slot, but at most three buys a turn.
    let n = state.player.as_ref().map(|p| p.ads_this_turn).unwrap_or(0);
    if n >= MAX_ADS_PER_TURN {
        return Err(bad("the ad market is saturated this turn"));
    }
    if let Some(p) = state.player.as_mut() {
        p.money -= cost;
        p.ads_this_turn += 1;
    }
    for race in state.politics.elections.races.iter_mut() {
        if let Some(c) = race.candidates.iter_mut().find(|c| c.politician == me) {
            crate::politics::elections::advertise(c, &content.politics.campaign, n);
        }
    }
    // Changing the subject cools a running scandal a little.
    crate::media::advertised(state, content, me);
    res.outcome_key = "outcome-advertise".into();
    res.args.insert("amount".into(), cost.units().to_string());
    Ok(res)
}
