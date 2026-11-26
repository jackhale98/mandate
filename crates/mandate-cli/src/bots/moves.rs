//! The small shared moves: campaigning, canvassing, lobbying, legislating and buying ads.

use super::*;

pub(super) fn campaigning(e: &Engine) -> bool {
    let Some(me) = e.state().player.as_ref().map(|p| p.politician) else {
        return false;
    };
    e.state()
        .politics
        .elections
        .races
        .iter()
        .any(|r| r.candidates.iter().any(|c| c.politician == me))
}

pub(super) fn act(e: &mut Engine, a: Action) -> bool {
    let cmd = e.route(PlayerCommand::Act {
        action: a,
        late_night: false,
    });
    apply(e, cmd).is_ok()
}

/// Canvass a neighbourhood on the worry the player knows it feels most
/// (none known: a general canvass).
pub(super) fn canvass(e: &mut Engine, hood: u32) -> bool {
    let issue = Knowing::of(e).top_worry(NeighborhoodId(hood));
    act(
        e,
        Action::Canvass {
            neighborhood: hood,
            issue,
        },
    )
}

/// The ward neighbourhood where the player is least known.
pub(super) fn coldest_hood(e: &Engine) -> Option<u32> {
    let st = e.state();
    let pl = st.player.as_ref()?;
    ward_hoods(&st.world, pl.ward)
        .into_iter()
        .min_by(|a, b| {
            let aw = |h: usize| {
                let cs = &st.population.hoods[h].cohorts;
                cs.iter()
                    .map(|c| c.player_awareness as f64 * c.count as f64)
                    .sum::<f64>()
                    / cs.iter().map(|c| c.count as f64).sum::<f64>().max(1.0)
            };
            aw(*a).total_cmp(&aw(*b))
        })
        .map(|h| h as u32)
}

pub(super) fn own_bill(e: &Engine) -> Option<u32> {
    let st = e.state();
    let me = st.player.as_ref()?.politician;
    st.policy
        .instances
        .iter()
        .rev()
        .find(|p| p.sponsor == Some(me) && p.status == PolicyStatus::UnderConsideration)
        .map(|p| p.id.0)
}

/// A template's support in the player's ward as the player knows it (the
/// bill view's figure: coarse until learned).
pub(super) fn known_ward_support(e: &Engine, template: &str) -> f64 {
    Knowing::of(e)
        .ward_support_of(&PolicyRef::Template {
            template: template.to_string(),
        })
        .unwrap_or(0.0)
}

/// Whom to lobby on the player's bill, from what the player can see (the
/// whip's exact leans are the chamber's, and they carry colleagues' hidden
/// goals): colleagues the player has worked on least about this bill (their
/// memories of the player are never gated), the other parties' first (the
/// player's own party follows its whip), then the best shown odds (which
/// carry the knowledge term: people the player knows well come first).
pub(super) fn lobby_target(e: &Engine, content: &ContentBundle, bill: u32) -> Option<u32> {
    let st = e.state();
    let pl = st.player.as_ref()?;
    let mine = st.politics.get(pl.politician)?;
    let me = mine.character;
    let c = content.constitutions.get(&st.meta.constitution)?;
    let town = mandate_sim::world::Jurisdiction::town(pl.home());
    mandate_sim::politics::legislature_of(&st.politics, c, town)
        .iter()
        .filter(|id| **id != pl.politician)
        .filter_map(|id| st.politics.get(*id))
        .map(|p| {
            let worked = st
                .politics
                .characters
                .get(p.character)
                .map(|c| {
                    c.memories
                        .iter()
                        .filter(|m| {
                            m.actor == me
                                && m.subject
                                    == Some(mandate_sim::characters::Subject::Policy { id: bill })
                        })
                        .count()
                })
                .unwrap_or(0);
            let same = mine.party.is_some() && p.party == mine.party;
            let odds = mandate_sim::actions::lobby_odds(st, content, p.id, false);
            (worked, same, odds, p.id.0)
        })
        .min_by(|a, b| {
            a.0.cmp(&b.0)
                .then(a.1.cmp(&b.1))
                .then(b.2.total_cmp(&a.2))
                .then(a.3.cmp(&b.3))
        })
        .map(|x| x.3)
}

/// Lobby a colleague on a bill in conversation (M6 note 29), from the list
/// (no walk): the argument offered with the best shown chance (none that
/// owes a favour or meets after hours, as the one-tap lobby was), as many
/// exchanges as the week allows. Whether any was made.
pub(super) fn lobby(e: &mut Engine, content: &ContentBundle, target: u32, bill: u32) -> bool {
    use mandate_sim::actions::lobbying::{self, Argument};
    let Some(person) = e
        .state()
        .politics
        .get(mandate_sim::politics::PoliticianId(target))
        .map(|p| p.character)
    else {
        return false;
    };
    let mut any = false;
    for _ in 0..content.actions.lobby.arguments.exchanges {
        let best = lobbying::arguments(e.state(), content, person)
            .into_iter()
            .filter(|p| {
                p.policy == bill && p.argument.rolled() && !p.late && p.argument != Argument::Rally
            })
            .max_by(|a, b| a.odds.total_cmp(&b.odds).then(b.argument.cmp(&a.argument)))
            .map(|p| lobbying::choice_id(bill, p.argument));
        let Some(topic) = best else { break };
        let cmd = PlayerCommand::Talk {
            to: mandate_sim::talk::TalkTarget::Person { person },
            topic,
            about: None,
            late_night: false,
        };
        if apply(e, Command::Player(cmd)).is_err() {
            break;
        }
        any = true;
    }
    any
}

/// The open proposals (DEC-092) the player knows well enough to draft
/// (`known`, DEC-091), holds the power for, and no one has a live bill of,
/// each
/// with its default package's ward support as the player knows it (the
/// higher of the proposal's and the family's support subjects, as the
/// sheet shows it: coarse until learned).
pub(super) fn known_proposals(e: &Engine, content: &ContentBundle) -> Vec<(f64, u32)> {
    let st = e.state();
    let Some(pl) = st.player.as_ref() else {
        return Vec::new();
    };
    let k = Knowing::of(e);
    let focus = st.focus();
    let hoods = ward_hoods(&st.world, pl.ward);
    let mut out = Vec::new();
    for inst in st
        .policy
        .families
        .instances
        .iter()
        .filter(|i| i.is_open() && focus.contains(&i.at))
    {
        use mandate_sim::player::knowledge::Subject;
        if k.level(&Subject::Proposal { proposal: inst.id }) < Level::Known {
            continue;
        }
        let Some(f) = content.family(&inst.family) else {
            continue;
        };
        if !mandate_sim::actions::player_holds_power_at(st, content, f.requires, inst.at) {
            continue;
        }
        // One live bill per proposal (as for NPCs).
        let live = inst.drafts.iter().any(|d| {
            st.policy
                .get(*d)
                .is_some_and(|p| p.status != PolicyStatus::Ended)
        });
        if live {
            continue;
        }
        let Some(t) =
            mandate_sim::policy::resolve::compose(content, f, Some(inst), &BTreeMap::new())
        else {
            continue;
        };
        let v = mandate_sim::policy::public_support(&t, &st.population, &hoods);
        let support = |policy: PolicyRef| {
            k.level(&Subject::PolicySupport {
                town: pl.municipality,
                policy,
            })
        };
        let level = support(PolicyRef::Template {
            template: f.id.clone(),
        })
        .max(support(PolicyRef::Proposal { proposal: inst.id }));
        let coarse = |step: f64| (v / step).round() * step;
        let shown = match level {
            Level::Unknown | Level::Heard => coarse(0.25),
            Level::Known => coarse(0.1),
            Level::Detailed => v,
        };
        out.push((shown, inst.id.0));
    }
    out
}

pub(super) fn legislate(e: &mut Engine, content: &ContentBundle) {
    if own_bill(e).is_none() {
        // The best open proposal the player knows (DEC-092), drafted at its
        // defaults when it beats the best of the standard menu.
        let proposal = known_proposals(e, content)
            .into_iter()
            .max_by(|a, b| a.0.total_cmp(&b.0).then(b.1.cmp(&a.1)));
        let best = {
            let st = e.state();
            let Some(pl) = st.player.as_ref() else { return };
            let k = Knowing::of(e);
            let taken: Vec<&str> = st
                .policy
                .in_municipality(pl.municipality.0)
                .filter(|p| !matches!(p.status, PolicyStatus::Ended))
                .map(|p| p.template.as_str())
                .collect();
            // The menu as the player sees it, ranked by the ward support the
            // player knows (DEC-091: never the true figure).
            content
                .policies
                .values()
                .filter(|t| {
                    k.on_menu(t)
                        && !taken.contains(&t.id.as_str())
                        && mandate_sim::policy::unlocked(st, content, t, pl.municipality.0)
                })
                .map(|t| (known_ward_support(e, &t.id), t))
                .max_by(|a, b| a.0.total_cmp(&b.0))
                .map(|(s, t)| (s, t.id.clone()))
        };
        let cmd = match (best, proposal) {
            (Some((s, _)), Some((p, proposal))) if p > s => Some(PlayerCommand::DraftFamily {
                proposal,
                choices: BTreeMap::new(),
                params: BTreeMap::new(),
                sponsor: None,
            }),
            (None, Some((_, proposal))) => Some(PlayerCommand::DraftFamily {
                proposal,
                choices: BTreeMap::new(),
                params: BTreeMap::new(),
                sponsor: None,
            }),
            (Some((_, template)), _) => Some(PlayerCommand::DraftPolicy {
                template,
                params: BTreeMap::new(),
            }),
            (None, None) => None,
        };
        if let Some(cmd) = cmd {
            let cmd = e.route(cmd);
            let r = apply(e, cmd);
            if let Some(id) = r.ok().and_then(|r| r.result).and_then(|x| x.created) {
                let cmd = e.route(PlayerCommand::IntroducePolicy { policy: id });
                let _ = apply(e, cmd);
            }
        }
    }
    if let Some(bill) = own_bill(e) {
        // Lobby a colleague, then talk it up in public.
        let target = lobby_target(e, content, bill);
        if let Some(t) = target {
            lobby(e, content, t, bill);
        }
        if let Some(h) = coldest_hood(e) {
            act(
                e,
                Action::TownHall {
                    neighborhood: h,
                    policy: Some(bill),
                },
            );
        }
    }
}

pub(super) fn tired(e: &Engine) -> bool {
    e.state()
        .player
        .as_ref()
        .is_some_and(|p| p.tracks.health < 40.0)
}

pub(super) fn buy_ads(e: &mut Engine, n: usize) {
    for _ in 0..n {
        if !act(e, Action::Advertise) {
            break;
        }
    }
}
