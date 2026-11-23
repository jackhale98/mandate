//! Learning the town as the player would: visits, meetings and the [`Knowing`]-gated reads.

use super::*;

pub(super) fn slots_left(e: &Engine) -> u8 {
    e.state()
        .player
        .as_ref()
        .map(|p| p.slots_left())
        .unwrap_or(0)
}

/// Where someone is now: the engine's answer (their daily routine,
/// DEC-031), when it is somewhere the player can go.
pub(super) fn whereabouts(e: &Engine, person: CharacterId) -> Option<Location> {
    let st = e.state();
    st.player.as_ref()?;
    mandate_sim::characters::routines::whereabouts(st, e.content(), person)
        .filter(|l| mandate_sim::places::validate_move(st, *l).is_ok())
}

pub(super) fn move_to(e: &mut Engine, location: Location) -> bool {
    if e.state().player.as_ref().map(|p| p.location) == Some(location) {
        return true;
    }
    apply(e, Command::Player(PlayerCommand::MoveTo { location })).is_ok()
}

/// Go to someone and talk to them on a topic (DEC-093, half a slot).
pub(super) fn talk(e: &mut Engine, person: CharacterId, topic: &str) -> bool {
    let Some(loc) = whereabouts(e, person) else {
        return false;
    };
    if !move_to(e, loc) {
        return false;
    }
    let cmd = Command::Player(PlayerCommand::Talk {
        to: mandate_sim::talk::TalkTarget::Person { person },
        topic: topic.into(),
        about: None,
        late_night: false,
    });
    apply(e, cmd).is_ok()
}

/// Walk the town: every place not yet visited (free; each first visit
/// gives its introduction and the names of the people there).
pub(super) fn tour(e: &mut Engine) {
    let todo: Vec<Location> = {
        let st = e.state();
        let Some(town) = st.player.as_ref().map(|p| p.municipality) else {
            return;
        };
        let k = Knowing::of(e);
        st.world
            .places
            .iter()
            .filter(|p| p.municipality == town && !k.visited(p.id))
            .map(|p| Location::Place { place: p.id })
            .collect()
    };
    for l in todo {
        move_to(e, l);
    }
}

/// A ward neighbourhood whose worries the player has not yet read in
/// detail, coldest first.
pub(super) fn unread_hood(e: &Engine) -> Option<u32> {
    let st = e.state();
    let pl = st.player.as_ref()?;
    let k = Knowing::of(e);
    let unread = |h: &usize| {
        k.level(&mandate_sim::player::knowledge::Subject::HoodConcerns {
            hood: NeighborhoodId(*h as u32),
        }) < Level::Detailed
    };
    let mut ward = ward_hoods(&st.world, pl.ward);
    ward.retain(unread);
    let aw = |h: usize| {
        let cs = &st.population.hoods[h].cohorts;
        cs.iter()
            .map(|c| c.player_awareness as f64 * c.count as f64)
            .sum::<f64>()
            / cs.iter().map(|c| c.count as f64).sum::<f64>().max(1.0)
    };
    ward.sort_by(|a, b| aw(*a).total_cmp(&aw(*b)).then(a.cmp(b)));
    ward.first().map(|h| *h as u32)
}

/// Someone whose name the player has heard but has not met properly: the
/// council first, then everyone else (in id order).
pub(super) fn unmet(e: &Engine) -> Option<CharacterId> {
    let st = e.state();
    let me = mandate_sim::characters::player_character(st);
    let k = Knowing::of(e);
    let council = mandate_sim::player::knowledge::council_people(st);
    let named = st
        .politics
        .characters
        .people
        .iter()
        // The people of places with purpose (seeded in a later round, PL1)
        // are met where they work, by the visit verbs (the coalition
        // builder's learning visits), not chased one by one in the exploring
        // turns: chasing them all spent the campaigner's early groundwork
        // and tilted the styles' variety (BALANCE_LOG, PL1).
        .filter(|c| {
            c.named.as_deref().is_some_and(|n| {
                e.content()
                    .occupation(&n.occupation)
                    .is_none_or(|o| o.round == 0)
            })
        })
        .map(|c| c.id);
    council
        .into_iter()
        .chain(named)
        .find(|c| Some(*c) != me && k.person(*c) == Level::Heard && whereabouts(e, *c).is_some())
}

/// Walk neighbourhood streets (the ward's; with `all`, the town's) and say
/// hello to the passers-by (free, `Meet`): residents met become people the
/// player knows.
pub(super) fn meet_passers_by(e: &mut Engine, content: &ContentBundle, all: bool) {
    let hoods: Vec<u32> = {
        let st = e.state();
        let Some(pl) = st.player.as_ref() else { return };
        if all {
            st.world
                .hoods_of(pl.municipality)
                .map(|h| h as u32)
                .collect()
        } else {
            ward_hoods(&st.world, pl.ward)
                .into_iter()
                .map(|h| h as u32)
                .collect()
        }
    };
    for h in hoods {
        let loc = Location::Street {
            neighborhood: NeighborhoodId(h),
        };
        let slots: Vec<u8> = mandate_sim::characters::ambient::ambient_at(e.state(), content, loc)
            .into_iter()
            .map(|x| x.0)
            .collect();
        if slots.is_empty() || !move_to(e, loc) {
            continue;
        }
        for slot in slots {
            let _ = apply(e, Command::Player(PlayerCommand::Meet { slot }));
        }
    }
}

/// What a verb would teach about a place that the player does not know
/// yet: its industry, its site or its service's need (places with
/// purpose). Read through `Knowing`, as everything a bot learns from.
pub(super) fn unlearned(e: &Engine, place: mandate_sim::world::PlaceId, verb: &str) -> bool {
    use mandate_content::knowledge::GrantSubject as G;
    use mandate_content::visits::VisitEffect;
    use mandate_sim::player::knowledge::Subject;
    let st = e.state();
    let content = e.content();
    let (Some(def), Some(p)) = (
        content.actions.visit.verb(verb),
        st.world.places.get(place.index()),
    ) else {
        return false;
    };
    let k = Knowing::of(e);
    let kind = content.place_kind(p.kind.key());
    def.effects.iter().any(|fx| {
        let VisitEffect::Learn { grants } = fx else {
            return false;
        };
        grants.iter().any(|g| {
            let s = match g.subject {
                G::Sector => kind
                    .and_then(|d| d.sector.clone())
                    .map(|sector| Subject::Sector {
                        town: p.municipality,
                        sector,
                    }),
                G::Site => Some(Subject::Site { place }),
                G::Service => Some(Subject::Service { place }),
                _ => None,
            };
            s.is_some_and(|s| k.level(&s) < Level::Known)
        })
    })
}

/// Visit a place to learn what it teaches (places with purpose): the first
/// place in town, in id order, with a verb that teaches something not yet
/// known and can be done now. Returns whether it was done.
pub(super) fn learning_visit(e: &mut Engine) -> bool {
    let pick = {
        let st = e.state();
        let content = e.content();
        let Some(town) = mandate_sim::player::home(st) else {
            return false;
        };
        st.world
            .places
            .iter()
            .filter(|p| p.municipality == town && p.is_open())
            .find_map(|p| {
                let loc = Location::Place { place: p.id };
                mandate_sim::visits::verbs_at(st, content, loc)
                    .into_iter()
                    .find(|v| {
                        unlearned(e, p.id, &v.id)
                            && mandate_sim::visits::availability(st, content, loc, v, false).is_ok()
                    })
                    .map(|v| (loc, v.id.clone()))
            })
    };
    let Some((location, verb)) = pick else {
        return false;
    };
    let then = PlayerCommand::Act {
        action: Action::Visit { verb },
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

/// One turn of learning: walk the town and the streets, read the ward's worries on the
/// doorsteps, and meet the people named so far, asking who really runs
/// things while that is a rumour. Returns whether any time went on it.
/// With `all` (the `explore_only` probe) it only finds out: walks, meets and
/// asks, and never campaigns (canvassing teaches too, but it is the ground
/// game), and nothing else is ever done. With `visits` (the coalition
/// builder, who goes where people work) it also visits the works, fields
/// and services to learn what they teach and meet the people there.
pub(super) fn explore(e: &mut Engine, content: &ContentBundle, all: bool, visits: bool) -> bool {
    tour(e);
    meet_passers_by(e, content, all);
    let mut busy = false;
    while slots_left(e) > 0 {
        let ok = if !all
            && !tired(e)
            && slots_left(e) >= 2
            && let Some(h) = unread_hood(e)
        {
            canvass(e, h)
        } else if let Some(p) = unmet(e) {
            // Who really runs things, while that is still a rumour.
            let topic = if Knowing::of(e).broker().is_none() {
                "topic.who_runs"
            } else {
                "topic.worries"
            };
            talk(e, p, topic) || talk(e, p, "topic.worries")
        } else if visits && !all {
            // The town's works, fields and services (places with purpose).
            learning_visit(e)
        } else {
            false
        };
        if !ok {
            break;
        }
        busy = true;
    }
    busy
}

/// Ask someone the player has met properly, who lives in the ward and is
/// not a politician, to speak up for them (DEC-093 `ask_support`), the best
/// shown odds first, each person once; at most two (a slot). Returns whether
/// anyone was asked.
pub(super) fn ask_support(e: &mut Engine, content: &ContentBundle) -> bool {
    let mut asked = false;
    for _ in 0..2 {
        let pick = {
            let st = e.state();
            let Some(pl) = st.player.as_ref() else {
                return asked;
            };
            let ward = ward_hoods(&st.world, pl.ward);
            let k = Knowing::of(e);
            let me = mandate_sim::characters::player_character(st);
            st.politics
                .characters
                .people
                .iter()
                .filter(|c| Some(c.id) != me && c.politician.is_none())
                .filter(|c| {
                    c.named
                        .as_deref()
                        .is_some_and(|n| ward.contains(&(n.home as usize)))
                })
                .filter(|c| k.person(c.id) >= Level::Known)
                .filter(|c| {
                    !pl.talk
                        .log
                        .iter()
                        .any(|x| x.speaker == c.id && x.topic == "topic.support")
                })
                .filter(|c| whereabouts(e, c.id).is_some())
                .filter_map(|c| {
                    // The odds the talk panel shows (DEC-073).
                    let odds =
                        mandate_sim::talk::odds(st, content, c.id, "topic.support", None, false)?;
                    (odds >= SUPPORT_MIN_ODDS).then_some((odds, c.id))
                })
                .max_by(|a, b| a.0.total_cmp(&b.0).then(b.1.cmp(&a.1)))
                .map(|x| x.1)
        };
        let Some(person) = pick else { break };
        if !talk(e, person, "topic.support") {
            break;
        }
        asked = true;
    }
    asked
}
