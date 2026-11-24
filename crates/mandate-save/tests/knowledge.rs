//! The knowledge ledger in whole games (DEC-091, M6_ENGINE_CONTRACTS §1):
//! seeding at play start, the start option, the lab switch, effects, zones,
//! introductions, and determinism (the ledger is written but, until K4,
//! nothing reads it for outcomes).
#![allow(clippy::unwrap_used, clippy::expect_used)]
use mandate_content::{bundle::read_dir_sources, load};
use mandate_sim::player::knowledge::{
    self, Knowing, LedgerOrigin, Level, PolicyRef, Source, Subject, Zone,
};
use mandate_sim::world::{MunicipalityId, NeighborhoodId, PlaceKind};
use mandate_sim::{Command, Engine};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::path::Path;
use std::sync::Arc;

fn content() -> Arc<mandate_content::ContentBundle> {
    let src =
        read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap();
    Arc::new(load(&src).unwrap().0)
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

fn ledger(e: &Engine) -> &knowledge::KnowledgeLedger {
    &e.state().player.as_ref().unwrap().knowledge
}

fn council_people(e: &Engine) -> Vec<mandate_sim::characters::CharacterId> {
    let st = e.state();
    let pl = st.player.as_ref().unwrap();
    let at = mandate_sim::world::Jurisdiction::town(pl.home());
    mandate_sim::politics::officials_at(&st.politics, at)
        .iter()
        .filter(|id| **id != pl.politician)
        .map(|id| st.politics.get(*id).unwrap().character)
        .collect()
}

#[test]
fn play_starts_with_the_sitting_councillors_knowledge() {
    let c = content();
    let e = game(21, &c);
    let k = ledger(&e);
    let pl = e.state().player.as_ref().unwrap();
    let town = pl.municipality;
    assert_eq!(
        k.origin,
        LedgerOrigin::Profile {
            profile: "start.sitting_councillor".into()
        }
    );
    assert_eq!(k.zone, Some(Zone::Town { town }));
    assert_eq!(k.level(&Subject::TownConcerns { town }), Level::Known);
    for h in mandate_sim::policy::ward_hoods(&e.state().world, pl.ward) {
        let hood = NeighborhoodId(h as u32);
        assert_eq!(k.level(&Subject::HoodConcerns { hood }), Level::Known);
    }
    let people = council_people(&e);
    assert!(!people.is_empty());
    for person in people {
        assert_eq!(k.level(&Subject::Person { person }), Level::Known);
    }
    let me = mandate_sim::characters::player_character(e.state()).unwrap();
    assert_eq!(k.level(&Subject::Person { person: me }), Level::Unknown);
    let transit = c
        .policies
        .values()
        .find(|t| t.tier == mandate_content::TierKey::Local && t.unlock.is_none())
        .unwrap();
    let support = Subject::PolicySupport {
        town,
        policy: PolicyRef::Template {
            template: transit.id.clone(),
        },
    };
    assert_eq!(k.level(&support), Level::Known);
    // DEC-091: even a native doesn't know who really runs things.
    assert_eq!(k.level(&Subject::Broker { town }), Level::Unknown);
    assert!(k.entries.values().all(|x| x.level < Level::Detailed));
    assert!(k.entries.values().flat_map(|x| &x.learned).all(|l| {
        l.on == e.state().calendar.date
            && l.source
                == Source::Start {
                    profile: "start.sitting_councillor".into(),
                }
    }));
    assert!(k.leads.is_empty() && k.visited.is_empty() && !k.lab_omniscient);
}

#[test]
fn the_start_can_be_chosen_before_play_only() {
    let c = content();
    let mut e = game(22, &c);
    let plain = ledger(&e).clone();
    assert!(e.set_knowledge_start(Some("start.nobody"), false).is_err());
    e.set_knowledge_start(Some("start.newcomer"), false)
        .unwrap();
    let town = e.state().player.as_ref().unwrap().municipality;
    let k = ledger(&e);
    assert_eq!(k.level(&Subject::TownConcerns { town }), Level::Unknown);
    for person in council_people(&e) {
        assert_eq!(ledger(&e).level(&Subject::Person { person }), Level::Heard);
    }
    e.set_knowledge_start(None, true).unwrap();
    let k = ledger(&e);
    assert!(matches!(k.origin, LedgerOrigin::LocalOption { .. }));
    assert_eq!(k.level(&Subject::Broker { town }), Level::Known);
    assert!(k.entries.len() > plain.entries.len());
    for (s, x) in &plain.entries {
        assert!(k.level(s) >= x.level);
    }
    // The option is a convenience: never detailed.
    assert!(k.entries.values().all(|x| x.level < Level::Detailed));
    assert!(
        k.entries
            .values()
            .flat_map(|x| &x.learned)
            .any(|l| l.source == Source::LocalOption)
    );
    e.apply(Command::EndTurn).unwrap();
    assert!(e.set_knowledge_start(None, false).is_err());
}

#[test]
fn knowing_reads_only_what_the_player_knows() {
    let c = content();
    let mut e = game(23, &c);
    let (town, ward) = {
        let pl = e.state().player.as_ref().unwrap();
        (pl.municipality, pl.ward)
    };
    let ward_hoods = mandate_sim::policy::ward_hoods(&e.state().world, ward);
    let outside = e
        .state()
        .world
        .hoods_of(town)
        .find(|h| !ward_hoods.contains(h))
        .map(|h| NeighborhoodId(h as u32))
        .expect("a town with more than one ward");
    let inside = NeighborhoodId(ward_hoods[0] as u32);
    let councillor = council_people(&e)[0];
    let transit = PolicyRef::Template {
        template: c
            .policies
            .values()
            .find(|t| t.tier == mandate_content::TierKey::Local && t.unlock.is_none())
            .unwrap()
            .id
            .clone(),
    };
    {
        let k = Knowing::of(&e);
        assert_eq!(k.town(), Some(town));
        assert_eq!(k.town_concerns().unwrap().len(), 3);
        assert_eq!(k.concerns_of(inside).unwrap().len(), 3);
        assert_eq!(k.concerns_of(outside), None);
        assert_eq!(k.broker(), None);
        assert_eq!(k.goals_of(councillor), None, "goals need a second meeting");
        let s = k.support_of(&transit).unwrap();
        assert_eq!(s, (s * 10.0).round() / 10.0, "known: to 0.1");
    }
    e.set_lab_omniscient(true);
    let k = Knowing::of(&e);
    assert_eq!(k.level(&Subject::Broker { town }), Level::Detailed);
    assert!(k.concerns_of(outside).unwrap().len() > 3);
    assert_eq!(
        k.broker(),
        e.state().politics.characters.towns[town.index()].broker
    );
    assert!(k.goals_of(councillor).is_some());
    assert!(k.support_of(&transit).is_some());
}

#[test]
fn other_systems_teach_through_effects() {
    use mandate_sim::scheduler::{Effect, Request, Schedule};
    // Lab requests: every effect names its cause (the causal spine).
    let lab = |effects: Vec<Effect>| -> Vec<Request> {
        effects
            .into_iter()
            .map(|effect| Request {
                effect,
                cause: mandate_sim::spine::Cause::Lab,
            })
            .collect()
    };
    let c = content();
    let e = game(24, &c);
    let mut st = e.state().clone();
    let town = st.player.as_ref().unwrap().municipality;
    let broker = Subject::Broker { town };
    let hood = Subject::HoodConcerns {
        hood: NeighborhoodId(st.world.hoods_of(town).last().unwrap() as u32),
    };
    let lead = knowledge::Lead {
        subject: broker.clone(),
        level: Level::Known,
        key: "lead-broker".into(),
        to: knowledge::LeadTarget::PlaceKind {
            town,
            kind: "union_hall".into(),
        },
        since: st.calendar.date,
        source: Source::Lab,
    };
    let reading = knowledge::Reading {
        on: st.calendar.date,
        source: Source::Lab,
        values: vec![("concern.jobs".into(), 700)],
    };
    let effects = vec![
        Effect::Lead { lead },
        Effect::Learn {
            subject: broker.clone(),
            level: Level::Heard,
            source: Source::Story { story: 1 },
        },
        // A reading before the identity is known is ignored.
        Effect::Read {
            subject: hood.clone(),
            reading: reading.clone(),
        },
    ];
    let sched = Schedule::new(mandate_sim::systems::default_systems());
    let left = sched.advance_with(&mut st, &c, 1, lab(effects), &mut |_| {});
    assert!(left.is_empty(), "the player domain takes its effects");
    let k = &st.player.as_ref().unwrap().knowledge;
    assert_eq!(k.level(&broker), Level::Heard);
    assert_eq!(k.leads.len(), 1, "a lead to `known` stays while only heard");
    if k.level(&hood) < Level::Known {
        assert!(k.entry(&hood).is_none_or(|x| x.reading.is_none()));
    }
    let effects = vec![
        Effect::Learn {
            subject: broker.clone(),
            level: Level::Known,
            source: Source::Lab,
        },
        Effect::Learn {
            subject: hood.clone(),
            level: Level::Known,
            source: Source::Lab,
        },
        Effect::Read {
            subject: hood.clone(),
            reading: reading.clone(),
        },
    ];
    sched.advance_with(&mut st, &c, 1, lab(effects), &mut |_| {});
    let k = &st.player.as_ref().unwrap().knowledge;
    assert_eq!(k.level(&broker), Level::Known);
    assert!(k.leads.is_empty(), "the lead was followed");
    assert_eq!(k.level(&hood), Level::Detailed);
    assert_eq!(k.entry(&hood).unwrap().reading.as_ref(), Some(&reading));
}

#[test]
fn a_new_zone_starts_fresh_with_the_primer_and_forgets_nothing() {
    let c = content();
    let e = game(25, &c);
    let mut st = e.state().clone();
    let home = st.player.as_ref().unwrap().municipality;
    let other = MunicipalityId(if home.0 == 0 { 1 } else { 0 });
    let before = st.player.as_ref().unwrap().knowledge.clone();
    knowledge::enter_zone(&mut st, &c, Zone::Town { town: other }, None);
    let k = &st.player.as_ref().unwrap().knowledge;
    assert_eq!(k.zone, Some(Zone::Town { town: other }));
    for (s, x) in &before.entries {
        assert_eq!(k.level(s), x.level, "identities never fade");
    }
    assert_eq!(
        k.level(&Subject::TownConcerns { town: other }),
        Level::Heard
    );
    assert_eq!(k.level(&Subject::Broker { town: other }), Level::Heard);
    assert_eq!(k.level(&Subject::Broker { town: home }), Level::Unknown);
    let c2 = mandate_sim::politics::body_members_at(
        &st.politics,
        mandate_sim::world::Jurisdiction::town(other),
    );
    let first = st.politics.get(c2[0]).unwrap().character;
    assert_eq!(k.level(&Subject::Person { person: first }), Level::Heard);
    assert!(
        k.entry(&Subject::Broker { town: other }).unwrap().learned[0].source
            == Source::Primer { party: None }
    );
}

#[test]
fn first_visits_introduce_the_place() {
    let c = content();
    let e = game(26, &c);
    let st = e.state();
    let town = st.player.as_ref().unwrap().municipality;
    let chamber = st
        .world
        .places
        .iter()
        .find(|p| p.municipality == town && p.kind == PlaceKind::CouncilChamber)
        .unwrap();
    let intro = knowledge::visit_intro(st, &c, chamber.id);
    for person in council_people(&e) {
        assert!(
            intro
                .iter()
                .any(|g| g.subject == Subject::Person { person } && g.level == Level::Known)
        );
    }
    // A place with Named staff introduces them by name (`heard`) at least.
    if let Some((p, who)) = st
        .world
        .places
        .iter()
        .filter(|p| p.municipality == town && p.kind != PlaceKind::CouncilChamber)
        .find_map(|p| {
            let who = mandate_sim::characters::ambient::regulars_at(
                &st.politics,
                st,
                mandate_sim::places::Location::Place { place: p.id },
            );
            (!who.is_empty()).then_some((p, who))
        })
    {
        let intro = knowledge::visit_intro(st, &c, p.id);
        for person in who {
            assert!(
                intro
                    .iter()
                    .any(|g| g.subject == Subject::Person { person })
            );
        }
    }
}

/// The ledger is written but nothing reads it for outcomes yet (K1): two
/// runs of a world, one with the lab switch and the local option on, differ
/// only in the player's ledger, and the same seed gives the same ledger.
#[test]
fn the_ledger_changes_nothing_else_and_is_deterministic() {
    let c = content();
    let mut a = game(27, &c);
    let mut b = game(27, &c);
    let mut d = game(27, &c);
    b.set_knowledge_start(None, true).unwrap();
    b.set_lab_omniscient(true);
    for _ in 0..20 {
        a.apply(Command::EndTurn).unwrap();
        b.apply(Command::EndTurn).unwrap();
        d.apply(Command::EndTurn).unwrap();
    }
    assert_eq!(a.state_hash(), d.state_hash());
    assert_ne!(a.state_hash(), b.state_hash());
    let strip = |e: &Engine| {
        let mut v = serde_json::to_value(e.state()).unwrap();
        v.pointer_mut("/player")
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove("knowledge");
        v
    };
    assert_eq!(strip(&a), strip(&b));
    // Producers (K3) only ever add: everything the start knew is still known.
    let start = game(27, &c);
    for (s, x) in &ledger(&start).entries {
        assert!(ledger(&a).level(s) >= x.level, "{s:?}");
    }
}
