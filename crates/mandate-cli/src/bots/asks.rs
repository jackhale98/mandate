//! Constituent asks and promises: answering, drafting and working them.

use super::*;

/// How a strategy treats asks: `asks_only` accepts and works every ask;
/// `mixed` and the informed styles accept the asks they can meet by their
/// own routine and decline the rest; everyone else (and `idle`) never
/// answers, so offers lapse.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum AskStyle {
    All,
    Meetable,
}

pub(super) fn ask_style(strategy: &str) -> Option<AskStyle> {
    match strategy {
        "asks_only" => Some(AskStyle::All),
        "mixed" => Some(AskStyle::Meetable),
        s if is_informed(s) => Some(AskStyle::Meetable),
        _ => None,
    }
}

/// Styles that draft bills (a need to introduce or enact one is theirs).
pub(super) fn drafts(strategy: &str) -> bool {
    !matches!(strategy, "explorer_campaigner")
}

/// Can this strategy meet every need of the ask by its own routine?
pub(super) fn meetable(strategy: &str, a: &mandate_sim::asks::Ask) -> bool {
    use mandate_sim::asks::Need;
    a.needs.iter().all(|n| match n {
        Need::Introduce { .. } | Need::Enacted { .. } => drafts(strategy),
        _ => true,
    })
}

/// Slots an ask's needs take (votes, visits and asks kept by not acting
/// are free).
pub(super) fn ask_slots(a: &mandate_sim::asks::Ask) -> u32 {
    use mandate_sim::asks::Need;
    a.needs
        .iter()
        .map(|n| match n {
            Need::Canvass { .. } => 2,
            Need::TownHall { hoods } => hoods.len() as u32,
            Need::Statement { .. }
            | Need::Talk { .. }
            | Need::Introduce { .. }
            | Need::Enacted { .. }
            | Need::Casework => 1,
            _ => 0,
        })
        .sum()
}

/// Does keeping the ask win anything with the player's own voters? Its
/// approval reaches the ward or the town, or the neighbourhood it lands in
/// is one of the ward's. Read from the ask as the player sees it (its cast
/// and its stated effects), never from hidden state.
pub(super) fn pays_in_ward(
    e: &Engine,
    content: &ContentBundle,
    a: &mandate_sim::asks::Ask,
) -> bool {
    use mandate_content::{EffectDef, Scope};
    let st = e.state();
    let Some(pl) = st.player.as_ref() else {
        return false;
    };
    let ward: Vec<usize> = ward_hoods(&st.world, pl.ward);
    let keep = match content.ask(&a.template) {
        Some(t) => &t.keep,
        None => return true,
    };
    keep.iter().any(|x| match x {
        EffectDef::Approval {
            neighborhood: Some(role),
            ..
        } => matches!(
            mandate_sim::narrative::actor_of(&a.cast, role),
            Some(mandate_sim::narrative::Actor::Neighborhood { id }) if ward.contains(&(id as usize))
        ),
        EffectDef::Approval { scope, .. } => matches!(scope, Scope::Ward | Scope::Town),
        EffectDef::Standing { delta } => *delta > 0.0,
        _ => false,
    })
}

/// Answer the offers on the table. A blind mix takes every ask it can meet;
/// an informed style takes the free ones and those that pay with its own
/// voters, and in a campaign nothing that costs time outside the race.
pub(super) fn answer_asks(e: &mut Engine, content: &ContentBundle, strategy: &str) {
    let Some(style) = ask_style(strategy) else {
        return;
    };
    let informed = is_informed(strategy);
    let camp = campaigning(e);
    let offers: Vec<(u32, bool)> = e
        .state()
        .asks
        .asks
        .iter()
        .filter(|a| a.offered())
        .map(|a| {
            let accept = match style {
                AskStyle::All => true,
                AskStyle::Meetable if !informed => meetable(strategy, a),
                AskStyle::Meetable => {
                    meetable(strategy, a)
                        && (ask_slots(a) == 0 || pays_in_ward(e, content, a))
                        && !(camp && ask_slots(a) >= 2)
                }
            };
            (a.id.0, accept)
        })
        .collect();
    for (ask, accept) in offers {
        let _ = apply(
            e,
            Command::Player(PlayerCommand::RespondAsk { ask, accept }),
        );
    }
}

/// Draft a bill of one of these templates (the first the player can) and
/// introduce it.
pub(super) fn draft_one_of(e: &mut Engine, content: &ContentBundle, templates: &[String]) -> bool {
    let pick = {
        let st = e.state();
        let Some(pl) = st.player.as_ref() else {
            return false;
        };
        let me = pl.politician;
        // Already one of these before the council: nothing to draft.
        if st.policy.instances.iter().any(|p| {
            p.sponsor == Some(me)
                && p.status == PolicyStatus::UnderConsideration
                && templates.contains(&p.template)
        }) {
            return false;
        }
        templates
            .iter()
            .filter_map(|t| content.policies.get(t))
            .find(|t| mandate_sim::policy::unlocked(st, content, t, pl.home().0))
            .map(|t| t.id.clone())
    };
    let Some(t) = pick else {
        return false;
    };
    let cmd = e.route(PlayerCommand::DraftPolicy {
        template: t,
        params: BTreeMap::new(),
    });
    let Some(id) = apply(e, cmd)
        .ok()
        .and_then(|r| r.result)
        .and_then(|x| x.created)
    else {
        return false;
    };
    let cmd = e.route(PlayerCommand::IntroducePolicy { policy: id });
    apply(e, cmd).is_ok()
}

/// Work the accepted asks and promises, soonest due first: one step of
/// each, while time is left. An informed style in a campaign leaves its
/// doorsteps first and works only what falls due before its next turn.
pub(super) fn work_asks(e: &mut Engine, content: &ContentBundle, strategy: &str) {
    use mandate_sim::asks::{Need, StatementNeed};
    if ask_style(strategy).is_none() {
        return;
    }
    let urgent_only = is_informed(strategy) && campaigning(e);
    let horizon = e.state().calendar.date.add_days(8);
    let mut todo: Vec<(mandate_core::SimDate, u32, Need, Vec<NeighborhoodId>)> = e
        .state()
        .asks
        .asks
        .iter()
        .filter(|a| a.accepted())
        .filter_map(|a| {
            a.current()
                .map(|n| (a.due, a.id.0, n.clone(), a.done.clone()))
        })
        .collect();
    todo.sort_by_key(|x| (x.0, x.1));
    for (due, ask, need, done) in todo {
        if urgent_only && due > horizon {
            continue;
        }
        let slots = slots_left(e);
        match need {
            Need::Vote { policy, vote } => {
                let cmd = e.route(PlayerCommand::CastVote { policy, vote });
                let _ = apply(e, cmd);
            }
            Need::Statement { about } if slots >= 1 => {
                let policy = match about {
                    StatementNeed::Bill { policy } => Some(policy),
                    _ => None,
                };
                act(e, Action::Press { policy });
            }
            Need::TownHall { hoods } if slots >= 1 => {
                if let Some(h) = hoods.iter().find(|h| !done.contains(h)) {
                    act(
                        e,
                        Action::TownHall {
                            neighborhood: h.0,
                            policy: own_bill(e),
                        },
                    );
                }
            }
            Need::Canvass { hood, .. } if slots >= 2 => {
                canvass(e, hood.0);
            }
            Need::Visit { place } => {
                move_to(e, Location::Place { place });
            }
            Need::Talk { person } if slots >= 1 => {
                // The ask's own choice in the conversation (talk hooks, S4).
                let topic = mandate_sim::asks::hook::choice_id(mandate_sim::asks::AskId(ask));
                talk(e, person, &topic);
            }
            Need::Introduce { templates } | Need::Enacted { templates, .. } if slots >= 1 => {
                draft_one_of(e, content, &templates);
            }
            Need::Casework if slots >= 1 => {
                act(e, Action::Casework { ask });
            }
            _ => {}
        }
    }
}

/// While campaigning, make promises from the menu (up to the cap), aimed
/// at the worries the player knows the race's neighbourhoods feel most (a
/// promise persuades where its worry is felt: ENGAGEMENT §3.5), then the
/// crisis's answer (styles that draft), then a promise of presence.
pub(super) fn make_promises(e: &mut Engine, content: &ContentBundle, strategy: &str) {
    if ask_style(strategy).is_none() || !campaigning(e) {
        return;
    }
    for _ in 0..3 {
        let menu = mandate_sim::asks::promise_menu(e.state(), content);
        // Known worries: how many of the ward's neighbourhoods the player
        // knows feel each most.
        let mut known: BTreeMap<String, i32> = BTreeMap::new();
        {
            let k = Knowing::of(e);
            let st = e.state();
            if let Some(pl) = st.player.as_ref() {
                for h in ward_hoods(&st.world, pl.ward) {
                    if let Some(w) = k.top_worry(NeighborhoodId(h as u32)) {
                        *known.entry(w).or_default() += 1;
                    }
                }
            }
        }
        let rank = |o: &mandate_sim::asks::PromiseOption| -> Option<i32> {
            match (o.template.as_str(), &o.concern, &o.answer) {
                (_, Some(c), None) if drafts(strategy) || c == "concern.taxes" => {
                    known.get(c).map(|n| -n)
                }
                ("promise.answer_crisis", _, _) if drafts(strategy) => Some(5),
                ("promise.town_halls", _, _) => Some(6),
                _ => None,
            }
        };
        let Some(o) = menu
            .iter()
            .filter_map(|o| rank(o).map(|r| (r, o)))
            .min_by_key(|(r, _)| *r)
            .map(|(_, o)| o.clone())
        else {
            return;
        };
        let args = o
            .answer
            .map(|a| vec![("answer".to_string(), a)])
            .unwrap_or_default();
        if apply(
            e,
            Command::Player(PlayerCommand::Promise {
                template: o.template,
                args,
            }),
        )
        .is_err()
        {
            return;
        }
    }
}
