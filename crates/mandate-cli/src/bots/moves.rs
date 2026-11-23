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

pub(super) fn legislate(e: &mut Engine, content: &ContentBundle) {
    if own_bill(e).is_none() {
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
                .map(|(_, t)| t.id.clone())
        };
        if let Some(t) = best {
            let cmd = e.route(PlayerCommand::DraftPolicy {
                template: t,
                params: BTreeMap::new(),
            });
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
            act(
                e,
                Action::Lobby {
                    politician: t,
                    policy: bill,
                },
            );
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
