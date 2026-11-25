//! One answer to "what does this place worry about most".
//!
//! `mandate_sim::concerns::ranked` is the only code that ranks the worries
//! of a scope (a jurisdiction, a district, a neighbourhood), by a named
//! reading (`felt`: what people say and the player is told; `measured`: by
//! level alone, what conditions on a statistic read). These tests hold every
//! reader to it:
//!
//! - over a band of seeds and several turns, each condition, picker, state
//!   and view names the same top worry as `ranked` for its scope and reading;
//! - at each knowledge level the notebook's worries card, a reply about the
//!   whole town, the town card, the pocket, the office and the canvass
//!   preview show cuts of one list, so the player is never told two things.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use mandate_content::Condition;
use mandate_content::condition::Worry;
use mandate_content::conversation::ConcernOf;
use mandate_protocol::*;
use mandate_sim::actions::PlayerCommand;
use mandate_sim::characters::{self, CharacterId, Relationship};
use mandate_sim::concerns::{self, Scope};
use mandate_sim::places::Location;
use mandate_sim::player::knowledge::{Knowing, Level, Source, Subject, hood_reading};
use mandate_sim::talk::{self, AboutRef, TalkTarget};
use mandate_sim::world::{Jurisdiction, MunicipalityId, NeighborhoodId};
use mandate_sim::{Command, Engine, State};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::sync::Arc;

fn content() -> Arc<mandate_content::ContentBundle> {
    let (c, _) = mandate_content::load(&mandate_content::embedded::sources()).unwrap();
    Arc::new(c)
}

fn game(seed: u64, c: &Arc<mandate_content::ContentBundle>) -> Engine {
    generate(
        &WorldConfig {
            seed,
            size: WorldSize::Small,
            scenario: None,
        },
        c.clone(),
    )
    .unwrap()
}

fn copy(e: &Engine) -> Engine {
    Engine::from_state(e.state().clone(), e.content_arc()).unwrap()
}

fn home(st: &State) -> MunicipalityId {
    mandate_sim::player::home(st).unwrap()
}

/// Know exactly these subjects at `level`, and nothing else (test setup).
fn know(e: &mut Engine, subjects: &[Subject], level: Level) {
    e.setup(|st| {
        let on = st.calendar.date;
        let k = &mut st.player.as_mut().unwrap().knowledge;
        k.lab_omniscient = false;
        k.entries.clear();
        k.leads.clear();
        for s in subjects {
            k.learn(s.clone(), level, on, Source::Lab);
        }
    });
}

/// The ids of a ranking's worries, of those anyone worries about.
fn ids(c: &mandate_content::ContentBundle, ranking: &[(usize, f64)]) -> Vec<String> {
    concerns::top(ranking, usize::MAX)
        .map(|(i, _)| c.concerns[i].id.clone())
        .collect()
}

fn name_key(c: &mandate_content::ContentBundle, id: &str) -> String {
    c.concerns
        .iter()
        .find(|x| x.id == id)
        .unwrap()
        .name_key
        .clone()
}

/// A `concern` condition on one worry of the town, among the `top`.
fn town_worry(id: &str, top: u8, by: Worry) -> Condition {
    Condition::Concern {
        of: ConcernOf::Town,
        concern: None,
        concerns: vec![id.into()],
        top: Some(top),
        by: Some(by),
    }
}

/// The notebook's entry for a subject: its concern name keys, strongest
/// first.
fn noted(e: &Engine, subject: &SubjectDto) -> Vec<String> {
    let notebook = notebook_view(e).unwrap();
    let Some(entry) = notebook
        .sections
        .iter()
        .flat_map(|s| &s.entries)
        .find(|x| &x.subject == subject)
    else {
        return Vec::new();
    };
    ["concern_key", "concern2_key", "concern3_key"]
        .iter()
        .filter_map(|k| entry.args.get(*k).cloned())
        .collect()
}

/// A ranking is in the one order: stronger first, equal weights in registry
/// order.
fn in_order(r: &[(usize, f64)]) -> bool {
    r.windows(2)
        .all(|w| w[0].1 > w[1].1 || (w[0].1 == w[1].1 && w[0].0 < w[1].0))
}

/// Every reader of a town's and its neighbourhoods' worries against
/// `concerns::ranked`, for one state. Returns how many readings it compared
/// and whether the town's felt and measured orders differ here (the worlds
/// where two routes to "the top worry" used to disagree).
fn check_state(e: &Engine, tag: &str) -> (usize, bool) {
    let c = e.content();
    let st = e.state();
    let pop = &st.population;
    let town = home(st);
    let hoods: Vec<usize> = st.world.hoods_of(town).collect();
    let mut checked = 0;

    // --- The town, by each reading -------------------------------------------------
    let felt = concerns::ranked(pop, c, &st.world, Scope::town(town), Worry::Felt);
    let measured = concerns::ranked(pop, c, &st.world, Scope::town(town), Worry::Measured);
    assert_eq!(felt.len(), c.concerns.len(), "{tag}");
    assert!(in_order(&felt) && in_order(&measured), "{tag}");
    let (felt_ids, measured_ids) = (ids(c, &felt), ids(c, &measured));
    // All of a town's neighbourhoods are the town, whichever way it is asked.
    for by in [Worry::Felt, Worry::Measured] {
        assert_eq!(
            concerns::ranked(pop, c, &st.world, Scope::District(&hoods), by),
            concerns::ranked(pop, c, &st.world, Scope::town(town), by),
            "{tag}"
        );
    }
    // The condition (talk, dilemmas, asks, the director's themes, a town's
    // situation): a worry is among the top n exactly when `ranked` puts it
    // there.
    for (by, order) in [(Worry::Felt, &felt_ids), (Worry::Measured, &measured_ids)] {
        for x in &c.concerns {
            let place = order.iter().position(|i| *i == x.id);
            for top in 1..=3u8 {
                assert_eq!(
                    mandate_sim::policy::situation_holds(
                        st,
                        c,
                        &town_worry(&x.id, top, by),
                        town.0
                    ),
                    place.is_some_and(|p| p < top as usize),
                    "{tag}: {} among the top {top} {by:?} of {order:?}",
                    x.id
                );
                checked += 1;
            }
        }
    }
    // The town's priorities, kept with its character: the measured top three.
    let priorities = &st.politics.characters.towns[town.index()].priorities;
    assert_eq!(
        priorities[..],
        measured_ids[..measured_ids.len().min(3)],
        "{tag}"
    );
    checked += 1;

    // --- Each neighbourhood, felt --------------------------------------------------
    let mut hood_felt = Vec::new();
    for &h in &hoods {
        let hood = NeighborhoodId(h as u32);
        let r = concerns::ranked(pop, c, &st.world, Scope::Neighbourhood(hood), Worry::Felt);
        assert!(in_order(&r), "{tag}");
        // A district of one neighbourhood is that neighbourhood.
        assert_eq!(
            concerns::ranked(pop, c, &st.world, Scope::District(&[h]), Worry::Felt),
            r,
            "{tag}"
        );
        let order = ids(c, &r);
        // What a first-hand reading records.
        let reading: Vec<String> = hood_reading(st, c, hood)
            .into_iter()
            .filter(|(_, w)| *w > 0)
            .map(|(id, _)| id)
            .collect();
        assert_eq!(reading[..], order[..reading.len()], "{tag} hood {h}");
        // Overheard lines about a worry are heard only where it is among
        // the top three.
        for l in mandate_sim::overheard::available(st, c, Location::Street { neighborhood: hood }) {
            let def = c.overheard.iter().find(|d| d.id == l.id).unwrap();
            if let Some(worry) = &def.concern {
                assert!(
                    order.iter().take(3).any(|i| i == worry),
                    "{tag} hood {h}: `{}` heard of {worry}, top {order:?}",
                    l.id
                );
                checked += 1;
            }
        }
        // The canvass match (what the bots canvass on): no worry fits
        // better than the top one.
        if let Some(first) = order.first() {
            let fit = |id: &str| mandate_sim::actions::canvass_match(st, c, hood, Some(id));
            assert!(c.concerns.iter().all(|x| fit(&x.id) <= fit(first)), "{tag}");
            checked += 1;
        }
        hood_felt.push((hood, order));
    }

    // --- Larger jurisdictions (tier-generic) ------------------------------------
    let region = st.world.municipalities[town.index()].region;
    for j in [Jurisdiction::Region { id: region }, Jurisdiction::Nation] {
        for by in [Worry::Felt, Worry::Measured] {
            let r = concerns::ranked(pop, c, &st.world, Scope::Jurisdiction(j), by);
            assert_eq!(r.len(), c.concerns.len(), "{tag} {j:?}");
            assert!(in_order(&r), "{tag} {j:?}");
        }
    }

    // --- What the player is told, knowing everything -----------------------------
    let mut k = copy(e);
    let mut subjects = vec![Subject::TownConcerns { town }];
    subjects.extend(hoods.iter().map(|h| Subject::HoodConcerns {
        hood: NeighborhoodId(*h as u32),
    }));
    know(&mut k, &subjects, Level::Detailed);
    let st = k.state();
    let c = k.content();
    let keys = |v: &[String]| -> Vec<String> { v.iter().map(|id| name_key(c, id)).collect() };
    let felt_keys = keys(&felt_ids);
    let view_ids = |v: Vec<ConcernView>| -> Vec<String> { v.into_iter().map(|x| x.id).collect() };
    // The lookup every view shares, and the engine's own knowledge view.
    assert_eq!(
        view_ids(concerns_for(st, c, Scope::town(town))),
        felt_ids,
        "{tag}"
    );
    let known = Knowing::new(st, c);
    assert_eq!(
        known.town_concerns().unwrap()[..felt_ids.len()],
        felt_ids[..],
        "{tag}"
    );
    // The town card (any town's, on the map).
    let card = municipality_view(&k, town.0).unwrap().top_concerns;
    assert_eq!(view_ids(card), felt_ids[..felt_ids.len().min(3)], "{tag}");
    // The notebook's "What <town> worries about".
    assert_eq!(
        noted(&k, &SubjectDto::TownConcerns { town: town.0 }),
        felt_keys[..felt_keys.len().min(3)],
        "{tag}"
    );
    // The pocket, the office and the town's character card.
    assert_eq!(
        pocket_view(&k).unwrap().headlines.top_concern_keys,
        felt_keys[..felt_keys.len().min(2)],
        "{tag}"
    );
    assert_eq!(
        view_ids(office_view(&k).unwrap().town.concerns),
        felt_ids,
        "{tag}"
    );
    let places = town_places_view(&k).unwrap();
    assert_eq!(
        view_ids(places.character.unwrap().priorities),
        felt_ids[..felt_ids.len().min(3)],
        "{tag}"
    );
    checked += 6;
    for (hood, order) in &hood_felt {
        let order_keys = keys(order);
        assert_eq!(
            view_ids(concerns_for(st, c, Scope::Neighbourhood(*hood))),
            order[..],
            "{tag}"
        );
        assert_eq!(
            known.concerns_of(*hood).unwrap()[..order.len()],
            order[..],
            "{tag}"
        );
        assert_eq!(known.top_worry(*hood).as_ref(), order.first(), "{tag}");
        assert_eq!(
            noted(&k, &SubjectDto::HoodConcerns { hood: hood.0 }),
            order_keys[..order_keys.len().min(3)],
            "{tag}"
        );
        // The town's map of neighbourhoods names each one's top worry.
        let shown = places
            .neighborhoods
            .iter()
            .find(|n| n.id == hood.0)
            .unwrap();
        assert_eq!(shown.top_concern_key.as_ref(), order_keys.first(), "{tag}");
        // The canvass preview: the neighbourhood's worries, then the town's
        // it does not already list, each in the one order.
        let canvass = canvass_view(&k, hood.0).unwrap();
        let from = |source: &str| -> Vec<String> {
            canvass
                .worries
                .iter()
                .filter(|w| w.source == source)
                .map(|w| w.id.clone())
                .collect()
        };
        assert_eq!(from("hood"), order[..], "{tag}");
        let rest: Vec<String> = felt_ids
            .iter()
            .filter(|i| !order.contains(i))
            .cloned()
            .collect();
        assert_eq!(from("town"), rest, "{tag}");
        assert!(
            canvass
                .worries
                .iter()
                .all(|w| w.top == (Some(&w.id) == order.first()))
        );
        checked += 6;
    }
    let differ = felt_ids[..felt_ids.len().min(3)] != measured_ids[..measured_ids.len().min(3)];
    (checked, differ)
}

/// Over a band of seeds and several turns: every reader names the same top
/// worries as `concerns::ranked`, for its scope and its reading.
#[test]
fn every_reader_names_the_same_top_worry() {
    let c = content();
    let (mut checked, mut differ) = (0, 0);
    for seed in 1..=12u64 {
        let mut e = game(seed, &c);
        let mut turn = 0;
        for until in [0, 2, 5, 9] {
            while turn < until {
                e.apply(Command::EndTurn).unwrap();
                turn += 1;
            }
            let (n, d) = check_state(&e, &format!("seed {seed} turn {turn}"));
            checked += n;
            differ += usize::from(d);
        }
    }
    assert!(checked > 2000, "{checked}");
    // The band has worlds where a town's felt and measured top three are
    // not the same list: where two routes to "the top worry" disagreed.
    assert!(differ > 0, "no world tells the two readings apart");
}

// --- What the player is told (protocol) --------------------------------------------

fn me(e: &Engine) -> CharacterId {
    characters::player_character(e.state()).unwrap()
}

/// The speaker thinks well enough of the player to answer (test setup).
fn friendly(e: &mut Engine, who: CharacterId) {
    let me = me(e);
    e.setup(|st| {
        st.politics.characters.edges.insert(
            (who.0, me.0),
            Relationship {
                opinion: 30.0,
                trust: 0.6,
                bonds: Vec::new(),
            },
        );
    });
}

/// Everyone the player can find in town: a copy of the game standing where
/// they are, and who they are.
fn people(e: &Engine) -> Vec<(Engine, CharacterId)> {
    let st = e.state();
    let town = home(st);
    let mut spots: Vec<Location> = st
        .world
        .places
        .iter()
        .filter(|p| p.municipality == town && p.is_open())
        .map(|p| Location::Place { place: p.id })
        .collect();
    spots.extend(st.world.hoods_of(town).map(|h| Location::Street {
        neighborhood: NeighborhoodId(h as u32),
    }));
    let mut out = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for loc in spots {
        let mut x = copy(e);
        if x.apply(Command::Player(PlayerCommand::MoveTo { location: loc }))
            .is_err()
        {
            continue;
        }
        for who in talk::present(x.state(), x.content()) {
            if seen.insert(who) {
                out.push((copy(&x), who));
            }
        }
    }
    out
}

/// The worry a reply about the whole town names (`line.worries.whole.*`),
/// if that is what the person answered.
fn said_of_the_whole_town(e: &mut Engine, who: CharacterId) -> Option<String> {
    let town = home(e.state());
    friendly(e, who);
    let o = e
        .apply(Command::Player(PlayerCommand::Talk {
            to: TalkTarget::Person { person: who },
            topic: "topic.worries".into(),
            about: Some(AboutRef::Jurisdiction {
                at: Jurisdiction::town(town),
            }),
            late_night: false,
        }))
        .ok()?;
    let v = exchange_view(e, o.result?.created?)?;
    let s = v.parts.iter().find(|p| p.part == "substance")?;
    s.key
        .starts_with("talk-worries-whole")
        .then(|| s.args.get("concern_key").cloned())
        .flatten()
}

/// The player is never told two different things: at each knowledge level
/// the notebook's "What <town> worries about" card, the canvass preview,
/// the pocket, the office and the town's character card show the first
/// worries of one list, the town card on the map shows the first three of
/// the same list, and a reply about the whole town names its first.
#[test]
fn the_player_is_told_one_ranking_of_the_towns_worries() {
    let c = content();
    let (mut spoken, mut differ) = (0, 0);
    for seed in [7u64, 53, 59] {
        let mut base = game(seed, &c);
        for _ in 0..3 {
            base.apply(Command::EndTurn).unwrap();
        }
        let st = base.state();
        let town = home(st);
        let hood = NeighborhoodId(st.world.hoods_of(town).next().unwrap() as u32);
        // The one list, as residents feel it.
        let all: Vec<ConcernView> = concerns_for(st, &c, Scope::town(town));
        let list: Vec<String> = all.iter().map(|x| x.name_key.clone()).collect();
        assert!(list.len() >= 3, "seed {seed}: {list:?}");
        let measured = ids(
            &c,
            &concerns::ranked(
                &st.population,
                &c,
                &st.world,
                Scope::town(town),
                Worry::Measured,
            ),
        );
        let by_level: Vec<String> = measured.iter().map(|id| name_key(&c, id)).collect();
        differ += usize::from(by_level[0] != list[0]);
        // The town card on the map follows the ledger, as the notebook does
        // (M6 note 22): a town the player has learned nothing of shows none.
        let card = |e: &Engine, town: MunicipalityId| -> Vec<String> {
            municipality_view(e, town.0)
                .unwrap()
                .top_concerns
                .into_iter()
                .map(|x| x.name_key)
                .collect()
        };
        let elsewhere = st
            .world
            .municipalities
            .iter()
            .map(|m| m.id)
            .find(|m| {
                mandate_sim::player::knowledge::knows(st, &Subject::TownConcerns { town: *m })
                    == Level::Unknown
                    && !concerns_for(st, &c, Scope::town(*m)).is_empty()
            })
            .unwrap();
        assert!(
            card(&base, elsewhere).is_empty(),
            "seed {seed}: the card of a town not learned of"
        );
        for (level, n) in [
            (Level::Heard, 1),
            (Level::Known, 3),
            (Level::Detailed, usize::MAX),
        ] {
            let mut e = copy(&base);
            know(&mut e, &[Subject::TownConcerns { town }], level);
            let tag = format!("seed {seed} at {level:?}");
            let cut = |cap: usize| list[..list.len().min(n).min(cap)].to_vec();
            // The notebook names up to three.
            assert_eq!(
                noted(&e, &SubjectDto::TownConcerns { town: town.0 }),
                cut(3),
                "{tag}: the notebook"
            );
            // The canvass preview of a neighbourhood the player knows
            // nothing of lists the town's worries only.
            let canvass: Vec<String> = canvass_view(&e, hood.0)
                .unwrap()
                .worries
                .into_iter()
                .map(|w| {
                    assert_eq!(w.source, "town", "{tag}");
                    w.name_key
                })
                .collect();
            assert_eq!(canvass, cut(usize::MAX), "{tag}: the canvass preview");
            assert_eq!(
                pocket_view(&e).unwrap().headlines.top_concern_keys,
                cut(2),
                "{tag}: the pocket"
            );
            let office: Vec<String> = office_view(&e)
                .unwrap()
                .town
                .concerns
                .into_iter()
                .map(|x| x.name_key)
                .collect();
            assert_eq!(office, cut(usize::MAX), "{tag}: the office");
            let character: Vec<String> = town_places_view(&e)
                .unwrap()
                .character
                .unwrap()
                .priorities
                .into_iter()
                .map(|x| x.name_key)
                .collect();
            assert_eq!(character, cut(3), "{tag}: the town's character");
            // The town card shows what the notebook shows: the first one at
            // `heard`, three from `known`, of the one list.
            assert_eq!(card(&e, town), cut(3), "{tag}: the town card");
        }
        // A reply about the whole town names the first of the list, and the
        // notebook then says the same.
        for (here, who) in people(&base) {
            let mut e = copy(&here);
            know(&mut e, &[], Level::Unknown);
            let Some(said) = said_of_the_whole_town(&mut e, who) else {
                continue;
            };
            assert_eq!(said, list[0], "seed {seed}: a reply about the whole town");
            let after = noted(&e, &SubjectDto::TownConcerns { town: town.0 });
            assert_eq!(
                after.first(),
                Some(&said),
                "seed {seed}: the notebook after it"
            );
            // Having heard it, the town card names it too.
            assert_eq!(
                card(&e, town).first(),
                Some(&said),
                "seed {seed}: the town card after it"
            );
            spoken += 1;
        }
    }
    assert!(spoken > 0, "someone answered about the whole town");
    // In some of these worlds the strongest worry by level is not the one
    // most felt: views reading one and the other named different top
    // worries there.
    assert!(differ > 0, "no world tells the two readings apart");
}
