//! Places with purpose (DEC-079 note; task PL1): the generic, data-driven
//! `visit` action. Every place kind offers something; a visit's shown odds
//! are the odds rolled; what it teaches lands in the ledger; a closed works
//! offers nothing; the people seeded at these places are there by their
//! routines; and nothing in it names an office (a renamed constitution and
//! a seat of a higher tier play the same).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use mandate_content::ContentBundle;
use mandate_protocol::*;
use mandate_sim::actions::{Action, PlayerCommand};
use mandate_sim::characters::routines::{self, Now};
use mandate_sim::places::Location;
use mandate_sim::player::knowledge::{Level, Subject};
use mandate_sim::visits;
use mandate_sim::world::{PlaceId, PlaceKind};
use mandate_sim::{Command, Engine};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::sync::{Arc, OnceLock};

fn content() -> Arc<ContentBundle> {
    static C: OnceLock<Arc<ContentBundle>> = OnceLock::new();
    C.get_or_init(|| {
        let (c, _) = mandate_content::load(&mandate_content::embedded::sources()).unwrap();
        Arc::new(c)
    })
    .clone()
}

fn game_with(seed: u64, c: Arc<ContentBundle>) -> Engine {
    generate(&WorldConfig::new(seed, WorldSize::Small), c).unwrap()
}

/// The first seed whose player's town has a place of this kind.
fn seed_with(kind: PlaceKind) -> u64 {
    (1u64..=60)
        .find(|s| place_in(&game_with(*s, content()), kind).is_some())
        .unwrap_or_else(|| panic!("no seed in 1..=60 has a {kind:?} in the player's town"))
}

/// Seeds of interest, found once.
fn factory_seed() -> u64 {
    static S: OnceLock<u64> = OnceLock::new();
    *S.get_or_init(|| seed_with(PlaceKind::Factory))
}

fn place_in(e: &Engine, kind: PlaceKind) -> Option<PlaceId> {
    let st = e.state();
    let town = st.player.as_ref()?.home();
    st.world.municipalities[town.index()]
        .places
        .iter()
        .copied()
        .find(|p| st.world.places[p.index()].kind == kind)
}

fn go(e: &mut Engine, place: PlaceId) {
    e.apply(Command::Player(PlayerCommand::MoveTo {
        location: Location::Place { place },
    }))
    .unwrap();
}

fn visit(e: &mut Engine, verb: &str) -> Result<mandate_sim::actions::ActionResult, String> {
    e.apply(Command::Player(PlayerCommand::Act {
        action: Action::Visit { verb: verb.into() },
        late_night: false,
    }))
    .map(|o| o.result.unwrap())
    .map_err(|e| e.to_string())
}

/// A morning (no slot spent yet) on a weekday of the turn.
fn morning(e: &mut Engine) {
    e.setup(|s| {
        let p = s.player.as_mut().unwrap();
        p.slots_used = 0;
        p.talk.half_slot = None;
    });
}

fn level(e: &Engine, s: &Subject) -> Level {
    e.state().player.as_ref().unwrap().knowledge.level(s)
}

#[test]
fn every_place_kind_offers_something_to_do() {
    let c = content();
    for k in &c.places {
        if k.key == "home" {
            assert!(!k.actions.is_empty(), "home offers rest");
            continue;
        }
        assert!(
            !k.actions.is_empty() || !k.services.is_empty() || !k.verbs.is_empty(),
            "place kind `{}` offers nothing to do",
            k.key
        );
    }
    // Every kind worldgen can generate is in the registry.
    assert!(mandate_sim::places::registry_issues(&c).is_empty());
}

#[test]
fn touring_the_works_lands_in_the_ledger() {
    let mut e = game_with(factory_seed(), content());
    let c = content();
    let factory = place_in(&e, PlaceKind::Factory).unwrap();
    go(&mut e, factory);
    morning(&mut e);
    let town = e.state().world.places[factory.index()].municipality;
    let sector = Subject::Sector {
        town,
        sector: "sector.manufacturing".into(),
    };
    let site = Subject::Site { place: factory };
    assert_eq!(level(&e, &sector), Level::Unknown);
    // The panel lists the verb, available, with its cost.
    let panel = place_panel_view(&e, LocationDto::of(Location::Place { place: factory })).unwrap();
    let tour = panel.verbs.iter().find(|v| v.verb == "verb.tour_works").unwrap();
    assert!(tour.available, "{:?}", tour.blocked_key);
    assert_eq!(tour.halves, 2);
    let slots = e.state().player.as_ref().unwrap().slots_left();
    let res = visit(&mut e, "verb.tour_works").unwrap();
    assert_eq!(res.outcome_key, "outcome-verb-tour_works");
    assert!(res.args["learned"].parse::<u32>().unwrap() > 0);
    assert_eq!(e.state().player.as_ref().unwrap().slots_left(), slots - 1);
    // The industry and the site: detailed, with dated readings.
    assert_eq!(level(&e, &sector), Level::Detailed);
    assert_eq!(level(&e, &site), Level::Detailed);
    let ledger = &e.state().player.as_ref().unwrap().knowledge;
    let reading = ledger.entry(&sector).unwrap().reading.as_ref().unwrap();
    assert!(reading.values.iter().any(|(k, _)| k == "use"));
    assert!(ledger.entry(&site).unwrap().reading.is_some());
    // Everyone working there now is known by name.
    let now = Now::of(e.state(), &c);
    let here = routines::present(e.state(), &c, &now, Location::Place { place: factory });
    assert!(!here.is_empty(), "nobody at the works on a weekday morning");
    for p in here {
        assert!(level(&e, &Subject::Person { person: p }) >= Level::Known);
    }
    // The notebook files the industry under places, in words.
    let nb = notebook_view(&e).unwrap();
    let places = nb.sections.iter().find(|s| s.key == "places").unwrap();
    let entry = places
        .entries
        .iter()
        .find(|x| x.title_key == "notebook-sector")
        .unwrap();
    assert!(["growing", "steady", "shedding"].contains(&entry.args["outlook"].as_str()));
}

#[test]
fn a_visits_shown_odds_are_the_odds_rolled() {
    let c = content();
    let mut e = game_with(factory_seed(), content());
    let factory = place_in(&e, PlaceKind::Factory).unwrap();
    let town = e.state().world.places[factory.index()].municipality;
    let k = c.sector_index("sector.manufacturing").unwrap();
    // Layoffs loom: the works runs well below capacity.
    e.setup(|s| {
        let sec = &mut s.economy.munis[town.index()].sectors[k];
        sec.jobs = sec.capacity * 0.7;
    });
    go(&mut e, factory);
    let def = c.actions.visit.verb("verb.photo_op_gate").unwrap().clone();
    let (mut hits, mut misses) = (0, 0);
    for turn in 0..40u32 {
        let mut t = Engine::from_state(e.state().clone(), content()).unwrap();
        t.setup(|s| s.calendar.turn += turn);
        morning(&mut t);
        let shown = place_panel_view(&t, LocationDto::of(Location::Place { place: factory }))
            .unwrap()
            .verbs
            .into_iter()
            .find(|v| v.verb == def.id)
            .unwrap()
            .risk
            .unwrap();
        let odds = visits::risk(t.state(), &c, factory, &def).unwrap();
        assert_eq!(shown, odds, "the panel shows the engine's odds");
        assert!(odds >= def.risk.as_ref().unwrap().base + 0.29, "shedding adds risk: {odds}");
        let expect = visits::goes_wrong(t.state(), factory, &def.id, odds);
        let res = visit(&mut t, &def.id).unwrap();
        let heckled = res.outcome_key == "outcome-verb-photo_op_gate-heckled";
        assert_eq!(heckled, expect, "turn {turn}: the roll is the odds shown");
        if heckled {
            hits += 1;
        } else {
            misses += 1;
        }
    }
    assert!(hits > 0 && misses > 0, "{hits} heckled, {misses} not");
}

#[test]
fn a_closed_works_offers_no_visit() {
    let mut e = game_with(factory_seed(), content());
    let factory = place_in(&e, PlaceKind::Factory).unwrap();
    go(&mut e, factory);
    morning(&mut e);
    let date = e.state().calendar.date;
    e.setup(|s| {
        assert!(s.world.close_place(factory, date));
    });
    let loc = Location::Place { place: factory };
    assert!(visits::verbs_at(e.state(), &content(), loc).is_empty());
    // Standing at the shut gates: not here (another works in town, if any,
    // still offers it).
    let err = visit(&mut e, "verb.tour_works").unwrap_err();
    assert!(
        err.contains("nowhere in your town") || err.contains("first"),
        "{err}"
    );
    let elsewhere = mandate_sim::places::locations_for(
        e.state(),
        &content(),
        mandate_sim::places::Offer::Verb("verb.tour_works"),
        None,
    );
    assert!(!elsewhere.contains(&loc));
    // The panel is gone too (a closed place cannot be entered).
    assert!(place_panel_view(&e, LocationDto::of(loc)).is_none());
}

#[test]
fn verbs_follow_the_hours_and_who_is_there() {
    let c = content();
    let mut e = game_with(factory_seed(), content());
    let school = place_in(&e, PlaceKind::School).unwrap();
    go(&mut e, school);
    let loc = Location::Place { place: school };
    let head = c.actions.visit.verb("verb.talk_to_head").unwrap();
    let cap = e.state().player.as_ref().unwrap().slot_cap();
    // Evening: the school is shut.
    e.setup(|s| s.player.as_mut().unwrap().slots_used = cap - 1);
    assert_eq!(
        visits::availability(e.state(), &c, loc, head, false),
        Err(visits::Blocked::Hours)
    );
    assert!(visit(&mut e, "verb.talk_to_head").is_err());
    // A weekday morning with the head teacher in: allowed (find one).
    let mut done = false;
    for _ in 0..8 {
        morning(&mut e);
        let now = Now::of(e.state(), &c);
        let in_school = routines::present(e.state(), &c, &now, loc).into_iter().any(|p| {
            e.state()
                .politics
                .characters
                .get(p)
                .and_then(|x| x.named.as_deref())
                .is_some_and(|n| n.occupation == "occupation.head_teacher")
        });
        if in_school {
            let res = visit(&mut e, "verb.talk_to_head").unwrap();
            assert!(res.args.contains_key("name"));
            // Half a slot: a second half-slot thing fits in the same slot.
            assert_eq!(e.state().player.as_ref().unwrap().slots_used, 1);
            visit(&mut e, "verb.talk_to_head").unwrap();
            assert_eq!(e.state().player.as_ref().unwrap().slots_used, 1);
            done = true;
            break;
        }
        assert_eq!(
            visits::availability(e.state(), &c, loc, head, false),
            Err(visits::Blocked::Nobody)
        );
        e.apply(Command::EndTurn).unwrap();
    }
    assert!(done, "the head teacher was never in on a morning");
}

#[test]
fn named_people_work_at_these_places_and_routines_bring_them_there() {
    let c = content();
    let kinds = [
        PlaceKind::Factory,
        PlaceKind::Mill,
        PlaceKind::Mine,
        PlaceKind::Port,
        PlaceKind::Farm,
        PlaceKind::School,
        PlaceKind::Clinic,
        PlaceKind::University,
        PlaceKind::Park,
        PlaceKind::Hotel,
        PlaceKind::TechCampus,
        PlaceKind::BusinessAssociation,
    ];
    let mut seen_kinds = std::collections::BTreeSet::new();
    for seed in [factory_seed(), 4242, 5] {
        let mut e = game_with(seed, content());
        let town = e.state().player.as_ref().unwrap().home();
        let places: Vec<PlaceId> = e.state().world.municipalities[town.index()]
            .places
            .iter()
            .copied()
            .filter(|p| kinds.contains(&e.state().world.places[p.index()].kind))
            .collect();
        // Who works where, and whether routines ever put them there by day.
        let staff = |e: &Engine, p: PlaceId| -> Vec<mandate_sim::characters::CharacterId> {
            e.state()
                .politics
                .characters
                .people
                .iter()
                .filter(|c| c.named.as_deref().is_some_and(|n| n.place == Some(p)))
                .map(|c| c.id)
                .collect()
        };
        let mut found: Vec<(PlaceId, Vec<mandate_sim::characters::CharacterId>)> = Vec::new();
        for p in &places {
            let s = staff(&e, *p);
            let kind = e.state().world.places[p.index()].kind;
            assert!(
                (1..=3).contains(&s.len()),
                "seed {seed}: {} people at a {kind:?}",
                s.len()
            );
            seen_kinds.insert(kind);
            found.push((*p, s));
        }
        let mut met: std::collections::BTreeSet<_> = Default::default();
        for _ in 0..6 {
            for used in [0u8, 1] {
                e.setup(|s| s.player.as_mut().unwrap().slots_used = used);
                let now = Now::of(e.state(), &c);
                for (p, s) in &found {
                    let here = routines::present(e.state(), &c, &now, Location::Place { place: *p });
                    met.extend(s.iter().filter(|x| here.contains(x)).copied());
                }
            }
            e.apply(Command::EndTurn).unwrap();
        }
        for (p, s) in &found {
            assert!(
                s.iter().any(|x| met.contains(x)),
                "seed {seed}: nobody of {p:?} was ever there by day"
            );
        }
    }
    assert!(seen_kinds.len() >= 4, "too few kinds sampled: {seen_kinds:?}");
}

#[test]
fn people_seeded_later_append_ids() {
    // The PL1 people (seeding round 1) come after everyone of round 0.
    let e = game_with(factory_seed(), content());
    let c = content();
    let round = |o: &str| c.occupation(o).map(|d| d.round).unwrap_or(0);
    let mut last0 = 0;
    let mut first1 = u32::MAX;
    for p in &e.state().politics.characters.people {
        if let Some(n) = p.named.as_deref() {
            if round(&n.occupation) == 0 {
                last0 = last0.max(p.id.0);
            } else {
                first1 = first1.min(p.id.0);
            }
        }
    }
    assert!(first1 != u32::MAX, "no round-1 people");
    // Named people of round 0 are all seeded before round 1 (passers-by
    // promoted later have later ids, and none exist yet).
    assert!(last0 < first1, "round 0 ends at {last0}, round 1 starts at {first1}");
}

/// The base pack with its local posts renamed (as in the save crate's
/// constitution tests): the council is a `moot` of `alderman`s, the mayor a
/// `reeve`. Visits name no office, so they play the same.
fn renamed() -> Arc<ContentBundle> {
    let mut src = mandate_content::embedded::sources();
    for s in src.iter_mut() {
        let p = s.path.replace('\\', "/");
        if p.contains("locale/") || p.ends_with("council.jsonc") || p.ends_with("interiors.jsonc") {
            continue;
        }
        let mut t = s
            .text
            .replace(r#"{ "type": "mayor" }"#, r#"{ "type": "office" }"#);
        t = t.replace("\"councillor\"", "\"alderman\"");
        t = t.replace("\"mayor\"", "\"reeve\"");
        if p.contains("constitutions/") {
            t = t.replace("\"council\"", "\"moot\"");
        }
        s.text = t;
    }
    Arc::new(mandate_content::load(&src).unwrap().0)
}

#[test]
fn visits_work_under_a_renamed_constitution_and_at_a_higher_tiers_seat() {
    let c = renamed();
    let mut e = game_with(factory_seed(), c.clone());
    let factory = place_in(&e, PlaceKind::Factory).unwrap();
    go(&mut e, factory);
    morning(&mut e);
    let res = visit(&mut e, "verb.tour_works").unwrap();
    assert_eq!(res.outcome_key, "outcome-verb-tour_works");
    // A seat of another tier (M7): the regional assembly's gallery is a verb
    // like any other, about whoever is there.
    let park = place_in(&e, PlaceKind::Park).unwrap();
    e.setup(|s| s.world.places[park.index()].kind = PlaceKind::RegionalAssembly);
    go(&mut e, park);
    morning(&mut e);
    let loc = Location::Place { place: park };
    let offered: Vec<String> = visits::verbs_at(e.state(), &c, loc)
        .into_iter()
        .map(|v| v.id.clone())
        .collect();
    assert_eq!(offered, vec!["verb.public_gallery".to_string()]);
    let res = visit(&mut e, "verb.public_gallery").unwrap();
    assert_eq!(res.outcome_key, "outcome-verb-public_gallery");
}

#[test]
fn visits_are_deterministic_and_replay() {
    let mut a = game_with(factory_seed(), content());
    let mut b = game_with(factory_seed(), content());
    for e in [&mut a, &mut b] {
        let factory = place_in(e, PlaceKind::Factory).unwrap();
        e.apply(Command::Player(PlayerCommand::Go {
            location: Location::Place { place: factory },
            then: Some(Box::new(PlayerCommand::Act {
                action: Action::Visit {
                    verb: "verb.listen_at_gate".into(),
                },
                late_night: false,
            })),
        }))
        .unwrap();
        e.apply(Command::Player(PlayerCommand::Act {
            action: Action::Visit {
                verb: "verb.photo_op_gate".into(),
            },
            late_night: false,
        }))
        .unwrap();
        e.apply(Command::EndTurn).unwrap();
    }
    assert_eq!(a.state_hash(), b.state_hash());
}

/// A seat of a higher tier in the player's town (medium seed 7 is a
/// regional capital): its public gallery is offered and done like any verb.
#[test]
fn the_public_gallery_of_a_regional_assembly() {
    let mut e = generate(&WorldConfig::new(7, WorldSize::Medium), content()).unwrap();
    let assembly = place_in(&e, PlaceKind::RegionalAssembly)
        .expect("medium seed 7's town is a regional capital");
    go(&mut e, assembly);
    morning(&mut e);
    let panel =
        place_panel_view(&e, LocationDto::of(Location::Place { place: assembly })).unwrap();
    let gallery = panel
        .verbs
        .iter()
        .find(|v| v.verb == "verb.public_gallery")
        .unwrap();
    assert!(gallery.available, "{:?}", gallery.blocked_key);
    assert_eq!(gallery.halves, 1);
    let res = visit(&mut e, "verb.public_gallery").unwrap();
    assert_eq!(res.outcome_key, "outcome-verb-public_gallery");
}
