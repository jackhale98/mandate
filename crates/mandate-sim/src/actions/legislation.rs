//! Legislation: drafting, amending, introducing and withdrawing bills, and declared votes.

use super::*;
use crate::spine::{Cause, Causes, Link, PlayerAct};

/// Amend a draft or a bill (`PlayerCommand::AmendPolicy`).
pub(super) fn amend(
    state: &mut State,
    content: &ContentBundle,
    cmd: &PlayerCommand,
    pl: &crate::player::PlayerState,
    m: u32,
) -> Result<ActionResult, EngineError> {
    let PlayerCommand::AmendPolicy {
        policy,
        params,
        clauses,
        choices,
    } = cmd
    else {
        return Err(bad("not an amendment"));
    };
    let mut res = ActionResult::default();
    let p = own_policy(state, *policy)?;
    let status = p.status;
    let template = p.template.clone();
    let family = p.family.is_some();
    if !matches!(
        status,
        PolicyStatus::Draft | PolicyStatus::UnderConsideration
    ) {
        return Err(bad("only drafts and bills can be amended"));
    }
    if choices.is_some() && !family {
        return Err(bad("only a proposal's bill has choices"));
    }
    // A family bill's sliders are checked against its composed rules.
    let params = if family {
        BTreeMap::new()
    } else {
        validate_params(content, &template, params)?
    };
    validate_clauses(state, m, clauses)?;
    let no_choices = BTreeMap::new();
    let new_choices = choices.as_ref().unwrap_or(&no_choices);
    if family {
        crate::policy::drafting::check_amend(
            state,
            content,
            PolicyInstanceId(*policy),
            new_choices,
            params_in(cmd),
        )
        .map_err(|e| bad(&e))?;
    }
    if status == PolicyStatus::UnderConsideration {
        spend(state, content, "amend", false)?;
        crate::politics::add_record(
            &mut state.politics,
            pl.politician,
            content.career.record_gain.amend as f32,
        );
        // Amendments cost goodwill already built.
        let keep = content.actions.amend.sway_keep as f32;
        for (k, v) in state.politics.sway.iter_mut() {
            if k.1 == *policy {
                *v *= keep;
            }
        }
    }
    if family {
        crate::policy::drafting::amend_choices(
            state,
            content,
            PolicyInstanceId(*policy),
            new_choices,
            params_in(cmd),
        )
        .map_err(|e| bad(&e))?;
    }
    if let Some(p) = state.policy.instances.get_mut(*policy as usize) {
        if family {
            // The area clause follows the choices; the rest are the player's.
            p.clauses.retain(|c| matches!(c, Clause::Area { .. }));
            p.clauses.extend(clauses.iter().cloned());
        } else {
            p.params = params;
            p.clauses = clauses.clone();
        }
    }
    res.outcome_key = "outcome-amended".into();
    Ok(res)
}

/// Introduce a draft as a bill (`PlayerCommand::IntroducePolicy`).
pub(super) fn introduce(
    state: &mut State,
    content: &ContentBundle,
    pl: &crate::player::PlayerState,
    date: mandate_core::SimDate,
    m: u32,
    policy: &u32,
) -> Result<ActionResult, EngineError> {
    let mut res = ActionResult::default();
    let p = own_policy(state, *policy)?;
    if p.status != PolicyStatus::Draft {
        return Err(bad("only drafts can be introduced"));
    }
    let days = content
        .constitutions
        .get(&state.meta.constitution)
        .and_then(|c| c.tier(TierKey::Local))
        .and_then(|t| t.procedure("ordinance"))
        .map(|p| p.deliberation_days)
        .unwrap_or(21);
    spend(state, content, "introduce", false)?;
    crate::politics::add_record(
        &mut state.politics,
        pl.politician,
        content.career.record_gain.introduce as f32,
    );
    // The player's act; a family bill answers the proposal on the table.
    let cause = Causes::from(Cause::Player(PlayerAct::Introduce)).and_event(
        crate::spine::proposal_of(state, crate::policy::PolicyInstanceId(*policy)),
        Link::Prompted,
    );
    let ev = state.events.record(
        date,
        EventKind::PolicyIntroduced {
            policy: *policy,
            municipality: m,
            sponsor: pl.politician.0,
        },
        cause,
    );
    if let Some(p) = state.policy.instances.get_mut(*policy as usize) {
        p.status = PolicyStatus::UnderConsideration;
        p.introduced_on = Some(date);
        p.next_stage_on = Some(date.add_days(days as u32));
        p.awareness = content.actions.introduce.awareness as f32;
        p.cause = Some(ev);
    }
    crate::asks::record_deed(state, crate::asks::Deed::Introduce { policy: *policy });
    learn_council_work(state, *policy);
    res.outcome_key = "outcome-introduced".into();
    res.args.insert("days".into(), days.to_string());
    Ok(res)
}

/// Withdraw a draft or a bill (`PlayerCommand::WithdrawPolicy`).
pub(super) fn withdraw(
    state: &mut State,
    date: mandate_core::SimDate,
    m: u32,
    policy: &u32,
) -> Result<ActionResult, EngineError> {
    let mut res = ActionResult::default();
    let p = own_policy(state, *policy)?;
    if !matches!(
        p.status,
        PolicyStatus::Draft | PolicyStatus::UnderConsideration
    ) {
        return Err(bad("only drafts and bills can be withdrawn"));
    }
    let was_bill = p.status == PolicyStatus::UnderConsideration;
    if let Some(p) = state.policy.instances.get_mut(*policy as usize) {
        p.status = PolicyStatus::Ended;
        p.end_reason = Some(crate::policy::EndReason::Withdrawn);
        p.next_stage_on = None;
    }
    if was_bill {
        let cause = Cause::Player(PlayerAct::Withdraw);
        state.events.record(
            date,
            EventKind::PolicyWithdrawn {
                policy: *policy,
                municipality: m,
            },
            cause,
        );
    }
    res.outcome_key = "outcome-withdrawn".into();
    Ok(res)
}

/// Declare the player's vote on a bill (public and final): party, ward, and
/// colleagues react now. Shared by the council chamber and decisions.
pub(crate) fn declare_vote(
    state: &mut State,
    content: &ContentBundle,
    policy: &u32,
    vote: &crate::politics::Vote,
) -> Result<ActionResult, EngineError> {
    let date = state.calendar.date;
    let pl = state
        .player
        .as_ref()
        .ok_or_else(|| bad("no player"))?
        .clone();
    let mut res = ActionResult::default();
    use crate::politics::Vote;
    let p = local_policy(state, *policy)?;
    if p.status != PolicyStatus::UnderConsideration {
        return Err(bad("only bills before the council can be voted on"));
    }
    if p.sponsor == Some(pl.politician) {
        return Err(bad("sponsors vote for their own bills"));
    }
    // A vote (or a signature) on the bill, for the bill's jurisdiction.
    let at = p.at();
    if !player_holds_power_at(state, content, Power::ApproveOrdinance, at)
        && !player_holds_power_at(state, content, Power::Veto, at)
    {
        return Err(bad("you have no vote on this bill"));
    }
    let key = (pl.politician.0, *policy);
    if state.politics.declared_votes.contains_key(&key) {
        return Err(bad("you have already declared your vote"));
    }
    let sponsor = p.sponsor;
    let awareness = p.awareness as f64;
    let sig = crate::policy::resolve(content, &state.policy, p)
        .map(|t| t.signature_weights.clone())
        .unwrap_or_default();
    let line = party_line(state, content, *policy);
    let sign = match vote {
        Vote::Yes => 1.0,
        Vote::No => -1.0,
        Vote::Abstain => 0.0,
    };
    state.politics.declared_votes.insert(key, sign as i8);
    crate::asks::record_deed(
        state,
        crate::asks::Deed::Vote {
            policy: *policy,
            vote: *vote,
        },
    );
    let fx = &content.actions.vote;
    crate::politics::add_record(
        &mut state.politics,
        pl.politician,
        content.career.record_gain.vote as f32,
    );
    // Party: loyalty is noticed a little, rebellion a lot.
    if let Some(pls) = state.player.as_mut() {
        let d = if line == 0 || *vote == Vote::Abstain {
            if *vote == Vote::Abstain {
                fx.standing_abstain
            } else {
                0.0
            }
        } else if (line as f64) == sign {
            fx.standing_loyal
        } else {
            fx.standing_defy
        };
        pls.tracks.standing = clamp(pls.tracks.standing as f64 + d, 0.0, 100.0) as f32;
    }
    // The ward: voters who care reward or punish the stand; dodging costs a little.
    for h in crate::policy::ward_hoods(&state.world, pl.ward) {
        for c in state.population.hoods[h].cohorts.iter_mut() {
            let a = crate::politics::alignment(&sig, &c.attitudes);
            let room = 1.0 - (c.player_approval as f64).abs();
            let delta = if *vote == Vote::Abstain {
                fx.ward_abstain
            } else {
                fx.ward_reaction * a * sign * (0.5 + awareness) * room
            };
            c.player_approval = clamp(c.player_approval as f64 + delta, -1.0, 1.0) as f32;
        }
    }
    // The sponsor: support earns goodwill and repays an owed favour;
    // voting against them while owing them is a broken promise.
    let me = state.politics.get(pl.politician).map(|p| p.character);
    let subject = Some(Subject::Policy { id: *policy });
    if let Some(sid) = sponsor
        && let Some(sp) = state.politics.get(sid)
        && !sp.is_player
        && let Some(me) = me
    {
        let (sc, name) = (sp.character, sp.full_name());
        let owed = characters::favours_owed(&state.politics, me, sc) > 0;
        let pol = &mut state.politics;
        match vote {
            Vote::Yes => {
                characters::remember(
                    pol,
                    content,
                    date,
                    sc,
                    me,
                    MemoryKind::BackedMyBill,
                    subject,
                    Some(fx.sponsor_yes),
                );
                if owed && let Some(f) = characters::repay(pol, me, sc) {
                    characters::remember(
                        pol,
                        content,
                        date,
                        sc,
                        me,
                        MemoryKind::KeptPromise,
                        f.subject.or(subject),
                        None,
                    );
                    res.args.insert("repaid".into(), name);
                }
            }
            Vote::No => {
                let (kind, hit) = if owed {
                    (MemoryKind::BrokePromise, fx.sponsor_no_owed)
                } else {
                    (MemoryKind::OpposedMyBill, fx.sponsor_no)
                };
                characters::remember(pol, content, date, sc, me, kind, subject, Some(-hit));
            }
            Vote::Abstain => {}
        }
    }
    // Breaking with the party line is noticed by party colleagues.
    if line != 0
        && *vote != Vote::Abstain
        && (line as f64) != sign
        && let Some(me) = me
    {
        let party = state.politics.get(pl.politician).and_then(|p| p.party);
        let colleagues: Vec<_> = crate::politics::officials_at(&state.politics, at)
            .into_iter()
            .filter_map(|id| state.politics.get(id))
            .filter(|p| !p.is_player && p.party.is_some() && p.party == party)
            .map(|p| p.character)
            .collect();
        for c in colleagues {
            characters::remember(
                &mut state.politics,
                content,
                date,
                c,
                me,
                MemoryKind::BrokeRanks,
                subject,
                None,
            );
        }
    }
    res.outcome_key = match vote {
        Vote::Yes => "outcome-vote-yes",
        Vote::No => "outcome-vote-no",
        Vote::Abstain => "outcome-vote-abstain",
    }
    .into();

    Ok(res)
}

/// Draft a bill sponsored by the player. Returns the new policy id.
pub(crate) fn draft(
    state: &mut State,
    content: &ContentBundle,
    template: &String,
    params: &BTreeMap<String, f64>,
) -> Result<u32, EngineError> {
    let pl = state
        .player
        .as_ref()
        .ok_or_else(|| bad("no player"))?
        .clone();
    let m = pl.municipality.0;
    let t = content
        .policies
        .get(template)
        .ok_or_else(|| bad("unknown policy template"))?;
    // Retired templates (DEC-092 §2.19) are drafted by no one.
    if t.retired {
        return Err(bad("that policy is no longer on offer"));
    }
    if t.tier != TierKey::Local {
        return Err(bad(
            "that policy is not available at your level of government",
        ));
    }
    // Drafted for the player's town (the local tier, checked above).
    let at = crate::world::Jurisdiction::town(pl.home());
    if !player_holds_power_at(state, content, t.requires, at) {
        return Err(bad("your office cannot propose this"));
    }
    if !crate::policy::unlocked(state, content, t, m) {
        return Err(bad(
            "that policy is not on the table while the town's situation does not call for it",
        ));
    }
    let params = validate_params(content, template, params)?;
    let id = state.policy.ids.alloc();
    state.policy.instances.push(PolicyInstance {
        id,
        template: template.clone(),
        tier: TierKey::Local,
        jurisdiction: m,
        sponsor: Some(pl.politician),
        params,
        clauses: Vec::new(),
        status: PolicyStatus::Draft,
        stage: 0,
        next_stage_on: None,
        votes: Vec::new(),
        introduced_on: None,
        enacted_on: None,
        progress: 0.0,
        awareness: 0.0,
        end_reason: None,
        cause: None,
        ledger: BTreeMap::new(),
        family: None,
        estimate: None,
    });
    Ok(id.0)
}

/// Council work on a bill (introducing it, declaring a vote, DEC-091): the
/// debate shows every colleague's stance (`known`) and who backs the bill.
pub(super) fn learn_council_work(state: &mut State, policy: u32) {
    let Some(p) = state.policy.instances.get(policy as usize) else {
        return;
    };
    let (template, town) = (p.template.clone(), MunicipalityId(p.jurisdiction));
    let src = knowledge::Source::Council {
        policy: crate::policy::PolicyInstanceId(policy),
    };
    let mut grants: Vec<knowledge::Grant> = knowledge::council_people(state)
        .into_iter()
        .map(|person| knowledge::Grant {
            subject: knowledge::Subject::Person { person },
            level: Level::Known,
        })
        .collect();
    grants.push(knowledge::Grant {
        subject: knowledge::Subject::PolicySupport {
            town,
            policy: knowledge::PolicyRef::Template { template },
        },
        level: Level::Known,
    });
    knowledge::teach(state, grants, &src);
}

/// Draft an open proposal (DEC-092 §2.11): refused unless it is open, the
/// player knows it (`known`, DEC-091), and holds a post with the family's
/// power at its jurisdiction; then the shared checks of
/// `policy::drafting::prepare`. Adopting a sponsor's version pays its price.
pub(crate) fn draft_family(
    state: &mut State,
    content: &ContentBundle,
    proposal: FamilyInstanceId,
    choices: &BTreeMap<String, String>,
    params: &BTreeMap<String, f64>,
    sponsor: Option<&str>,
) -> Result<u32, EngineError> {
    let pl = state
        .player
        .as_ref()
        .ok_or_else(|| bad("no player"))?
        .clone();
    let inst = state
        .policy
        .families
        .get(proposal)
        .ok_or_else(|| bad("no such proposal"))?;
    if !inst.is_open() {
        return Err(bad("that proposal is no longer on the table"));
    }
    let (at, family) = (inst.at, inst.family.clone());
    let f = content
        .family(&family)
        .ok_or_else(|| bad("that proposal's rules no longer exist"))?;
    if knowledge::knows(state, &knowledge::Subject::Proposal { proposal }) < Level::Known {
        return Err(bad(
            "you have only heard of this proposal: find out more first",
        ));
    }
    if !player_holds_power_at(state, content, f.requires, at) {
        return Err(bad("your office cannot propose this"));
    }
    let prepared =
        crate::policy::drafting::prepare(state, content, proposal, choices, params, sponsor)
            .map_err(|e| bad(&e))?;
    let id = crate::policy::drafting::make_bill(
        state,
        content,
        prepared,
        pl.politician,
        PolicyStatus::Draft,
        Vec::new(),
    );
    if sponsor.is_some() {
        crate::policy::drafting::adopt(state, content, id);
    }
    Ok(id.0)
}
