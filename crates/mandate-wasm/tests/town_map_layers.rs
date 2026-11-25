//! The town map's layers (DEC-086, M6 note 11): every registry layer is
//! served for the player's home town, as far as the player knows it.
//!
//! - districts: every district of the home legislature appears exactly once
//!   and every neighbourhood belongs to one; the player's own is marked;
//! - people: no one below `heard` is on the layer, and a person is placed
//!   only where the player knows to find them;
//! - worries: a neighbourhood the player knows nothing of has no class (it is
//!   unknown, never low), and a worry the player's cut cannot place has none;
//! - the query writes nothing (the knowledge ledger and the state's hash stay).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use mandate_protocol::*;
use mandate_sim::characters::routines::{self, DayPart};
use mandate_sim::characters::{self, CharacterId};
use mandate_sim::concerns::Scope;
use mandate_sim::places::Location;
use mandate_sim::player::knowledge::{Level, Sighting, Source, Subject, knows};
use mandate_sim::world::{Jurisdiction, MunicipalityId, NeighborhoodId};
use mandate_sim::{Command, Engine, State};
use mandate_wasm::session::Session;
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::collections::BTreeSet;
use std::sync::Arc;

const SEEDS: [u64; 3] = [7, 53, 2026];

fn content() -> Arc<mandate_content::ContentBundle> {
    let (c, _) = mandate_content::load(&mandate_content::embedded::sources()).unwrap();
    Arc::new(c)
}

fn game(seed: u64, c: &Arc<mandate_content::ContentBundle>) -> Engine {
    let mut e = generate(
        &WorldConfig {
            seed,
            size: WorldSize::Small,
            scenario: None,
        },
        c.clone(),
    )
    .unwrap();
    e.apply(Command::EndTurn).unwrap();
    e
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
        k.seen.clear();
        for s in subjects {
            k.learn(s.clone(), level, on, Source::Lab);
        }
    });
}

fn layers(e: &Engine) -> TownMapLayersView {
    town_map_layers_view(e).unwrap()
}

fn layer<'a>(v: &'a TownMapLayersView, source: &str) -> &'a TownLayerDataView {
    &v.layers.iter().find(|l| l.source == source).unwrap().data
}

fn areas(v: &TownMapLayersView) -> &[TownAreaView] {
    match layer(v, "districts") {
        TownLayerDataView::Boundaries { areas, .. } => areas,
        other => panic!("{other:?}"),
    }
}

fn markers(v: &TownMapLayersView) -> (&[TownMarkerView], &[TownLayerFilterView]) {
    match layer(v, "people") {
        TownLayerDataView::Markers {
            markers, filters, ..
        } => (markers, filters),
        other => panic!("{other:?}"),
    }
}

fn shaded(v: &TownMapLayersView) -> (&[TownShadeOptionView], &[Vec<String>]) {
    match layer(v, "worries") {
        TownLayerDataView::Shaded {
            options, details, ..
        } => (options, details),
        other => panic!("{other:?}"),
    }
}

#[test]
fn every_district_appears_once_and_every_neighbourhood_belongs_to_one() {
    let c = content();
    for seed in SEEDS {
        let e = game(seed, &c);
        let st = e.state();
        let at = Jurisdiction::town(home(st));
        let v = layers(&e);
        assert_eq!(v.at.id, at.raw(), "seed {seed}");
        let areas = areas(&v);
        // Every district of the home body, once, in number order.
        let want: Vec<u32> = {
            let mut n: Vec<u32> = at.constituencies(&st.world).map(|d| d.number).collect();
            n.sort_unstable();
            n
        };
        assert!(!want.is_empty(), "seed {seed}: the home town has districts");
        assert_eq!(
            areas.iter().map(|a| a.number).collect::<Vec<_>>(),
            want,
            "seed {seed}"
        );
        // Every neighbourhood of the town belongs to exactly one.
        for h in &v.hoods {
            let n = areas.iter().filter(|a| a.hoods.contains(&h.id)).count();
            assert_eq!(
                n, 1,
                "seed {seed}: neighbourhood {} in {n} districts",
                h.name
            );
        }
        let listed: usize = areas.iter().map(|a| a.hoods.len()).sum();
        assert_eq!(listed, v.hoods.len(), "seed {seed}");
        assert_eq!(
            v.hoods.iter().map(|h| h.id).collect::<Vec<_>>(),
            st.world
                .hoods_of(home(st))
                .map(|h| h as u32)
                .collect::<Vec<_>>(),
            "seed {seed}: the town's neighbourhoods, in its order"
        );
        // Named as the "where you are" chip names it, from the constitution's tier.
        for a in areas {
            assert_eq!(a.label_key, "ui-post-seat", "seed {seed}");
            assert_eq!(a.label_args["n"], a.number.to_string(), "seed {seed}");
            assert_eq!(a.label_args["tier"], v.at.tier, "seed {seed}");
            assert_eq!(a.seat.constituency_number, Some(a.number), "seed {seed}");
            // The holder is the seat's, and is shown whatever is known of
            // them (the role is public; the name is the ledger's).
            let h = a
                .holder
                .as_ref()
                .unwrap_or_else(|| panic!("seed {seed}: vacant"));
            assert_eq!(h.seat.as_ref(), Some(&a.seat), "seed {seed}");
            if !h.is_player && h.person.level == KnowledgeLevelDto::Unknown {
                assert_eq!(h.person.name, "", "seed {seed}: a stranger is not named");
            }
        }
        // The player's own: the seat they hold, and where they live.
        let mine: Vec<u32> = mandate_sim::player::posts(st)
            .into_iter()
            .filter_map(|p| p.constituency())
            .map(|d| st.world.constituencies[d.index()].number)
            .collect();
        assert_eq!(
            areas
                .iter()
                .filter(|a| a.yours)
                .map(|a| a.number)
                .collect::<Vec<_>>(),
            mine,
            "seed {seed}"
        );
        for a in areas.iter().filter(|a| a.yours) {
            assert!(a.holder.as_ref().unwrap().is_player, "seed {seed}");
        }
        assert_eq!(areas.iter().filter(|a| a.home).count(), 1, "seed {seed}");
        // The legend counts every seat.
        let TownLayerDataView::Boundaries { legend, .. } = layer(&v, "districts") else {
            panic!()
        };
        assert_eq!(
            legend.iter().map(|b| b.count as usize).sum::<usize>(),
            areas.len(),
            "seed {seed}"
        );
    }
}

#[test]
fn no_one_below_heard_is_on_the_people_layer() {
    let c = content();
    for seed in SEEDS {
        let base = game(seed, &c);
        let town = home(base.state());
        let folk = routines::townsfolk(base.state(), town);
        assert!(folk.len() >= 4, "seed {seed}");
        // As the game starts: only those the ledger has at `heard` or above.
        let on_layer = |e: &Engine| -> BTreeSet<u32> {
            let v = layers(e);
            let (ms, _) = markers(&v);
            for m in ms {
                assert!(m.person.level >= KnowledgeLevelDto::Heard, "seed {seed}");
                assert_ne!(m.person.name, "", "seed {seed}");
                assert!(!m.is_player, "seed {seed}");
            }
            ms.iter().map(|m| m.person.id.unwrap()).collect()
        };
        let heard = |e: &Engine| -> BTreeSet<u32> {
            folk.iter()
                .filter(|id| knows(e.state(), &Subject::Person { person: **id }) >= Level::Heard)
                .map(|id| id.0)
                .collect()
        };
        assert_eq!(on_layer(&base), heard(&base), "seed {seed}: the start");
        // Knowing no one: no one. Hearing of two: those two, and no others.
        let mut e = Engine::from_state(base.state().clone(), base.content_arc()).unwrap();
        know(&mut e, &[], Level::Heard);
        assert!(on_layer(&e).is_empty(), "seed {seed}: knowing no one");
        let two: Vec<Subject> = folk[..2]
            .iter()
            .map(|id| Subject::Person { person: *id })
            .collect();
        know(&mut e, &two, Level::Heard);
        assert_eq!(
            on_layer(&e),
            folk[..2].iter().map(|id| id.0).collect::<BTreeSet<_>>(),
            "seed {seed}: two heard of"
        );
    }
}

#[test]
fn a_person_is_placed_only_where_the_player_knows_to_find_them() {
    let c = content();
    for seed in SEEDS {
        let base = game(seed, &c);
        let st = base.state();
        let town = home(st);
        let folk = routines::townsfolk(st, town);
        let mut e = Engine::from_state(st.clone(), base.content_arc()).unwrap();
        let all: Vec<Subject> = folk
            .iter()
            .map(|id| Subject::Person { person: *id })
            .collect();
        know(&mut e, &all, Level::Heard);
        let v = layers(&e);
        let (ms, filters) = markers(&v);
        assert_eq!(ms.len(), folk.len(), "seed {seed}");
        let mut unplaced = None;
        for m in ms {
            let id = CharacterId(m.person.id.unwrap());
            match m.basis.as_str() {
                // Their workplace: what their routine's `works` resolves to.
                "works" => {
                    let w = routines::workplace(e.state(), &c, id).unwrap();
                    assert_eq!(
                        m.at.as_ref().map(|a| &a.location),
                        Some(&LocationDto::of(w)),
                        "seed {seed}"
                    );
                    assert!(m.seen.is_none(), "seed {seed}");
                }
                // Never seen (the ledger's sightings were cleared), no workplace.
                "unknown" => {
                    assert!(m.at.is_none(), "seed {seed}");
                    assert!(
                        routines::workplace(e.state(), &c, id).is_none(),
                        "seed {seed}"
                    );
                    unplaced = Some(id);
                }
                other => panic!("seed {seed}: basis {other} with no sighting"),
            }
            // Never where they are now unless that is what the player knows.
            assert!(m.filters.contains(&"all".to_string()), "seed {seed}");
            assert!(
                !m.filters.contains(&"met".to_string()),
                "seed {seed}: heard, not met"
            );
        }
        // The filters count what they keep; the body's is the legislature's members.
        for f in filters {
            let n = ms.iter().filter(|m| m.filters.contains(&f.id)).count() as u32;
            assert_eq!(f.count, n, "seed {seed}: filter {}", f.id);
        }
        let members: BTreeSet<u32> = ms
            .iter()
            .filter(|m| m.filters.contains(&"body".to_string()))
            .map(|m| m.person.id.unwrap())
            .collect();
        let st = e.state();
        let constitution = &c.constitutions[&st.meta.constitution];
        let at = Jurisdiction::town(town);
        let me = characters::player_character(st);
        let want: BTreeSet<u32> =
            mandate_sim::politics::legislature_of(&st.politics, constitution, at)
                .iter()
                .filter_map(|p| st.politics.get(*p))
                .map(|p| p.character)
                .filter(|ch| Some(*ch) != me)
                .map(|ch| ch.0)
                .collect();
        assert_eq!(members, want, "seed {seed}: the body filter");
        assert_eq!(
            filters
                .iter()
                .find(|f| f.id == "body")
                .unwrap()
                .label_key
                .as_deref(),
            mandate_sim::politics::legislature(constitution, at.tier())
                .map(|b| b.name_key.as_str()),
            "seed {seed}: named by the constitution"
        );
        // Someone with no workplace is placed once seen, where they were seen.
        let Some(who) = unplaced else { continue };
        let hood = NeighborhoodId(st.world.hoods_of(town).next().unwrap() as u32);
        let at = Location::Street { neighborhood: hood };
        let on = st.calendar.date;
        e.setup(|st| {
            st.player.as_mut().unwrap().knowledge.saw(
                who,
                Sighting {
                    on,
                    part: DayPart::Morning,
                    at,
                },
            );
        });
        let v = layers(&e);
        let m = markers(&v)
            .0
            .iter()
            .find(|m| m.person.id == Some(who.0))
            .unwrap();
        assert_eq!(m.basis, "last_seen", "seed {seed}");
        assert_eq!(
            m.at.as_ref().map(|a| &a.location),
            Some(&LocationDto::of(at)),
            "seed {seed}"
        );
        assert_eq!(m.seen.as_ref().unwrap().key, "seen-last", "seed {seed}");
    }
}

#[test]
fn an_unknown_neighbourhood_is_unknown_on_the_worries_layer() {
    let c = content();
    for seed in SEEDS {
        let base = game(seed, &c);
        let st = base.state();
        let town = home(st);
        let hoods: Vec<NeighborhoodId> = st
            .world
            .hoods_of(town)
            .map(|h| NeighborhoodId(h as u32))
            .collect();
        let mut e = Engine::from_state(st.clone(), base.content_arc()).unwrap();
        // Nothing known: every neighbourhood is unknown under every choice.
        know(&mut e, &[], Level::Heard);
        let v = layers(&e);
        let (options, details) = shaded(&v);
        assert_eq!(options[0].id, "top", "seed {seed}");
        assert_eq!(
            options[1..]
                .iter()
                .map(|o| o.id.as_str())
                .collect::<Vec<_>>(),
            c.concerns.iter().map(|d| d.id.as_str()).collect::<Vec<_>>(),
            "seed {seed}: one choice per worry, in registry order"
        );
        for o in options {
            assert_eq!(o.cells.len(), hoods.len(), "seed {seed}");
            assert_eq!(o.unknown as usize, hoods.len(), "seed {seed}: {}", o.id);
            for cell in &o.cells {
                assert_eq!(cell.class, None, "seed {seed}: {} is not low", o.id);
                assert_eq!(cell.label_key, None, "seed {seed}");
            }
            assert!(o.bins.iter().all(|b| b.count == 0), "seed {seed}");
        }
        assert!(details.iter().all(Vec::is_empty), "seed {seed}");

        // One neighbourhood, by level; the others stay unknown.
        let ranked = concerns_for(e.state(), &c, Scope::Neighbourhood(hoods[0]));
        assert!(ranked.len() >= 3, "seed {seed}: {ranked:?}");
        for (level, cut) in [
            (Level::Heard, 1),
            (Level::Known, 3),
            (Level::Detailed, usize::MAX),
        ] {
            know(&mut e, &[Subject::HoodConcerns { hood: hoods[0] }], level);
            let v = layers(&e);
            let (options, details) = shaded(&v);
            let tag = format!("seed {seed} at {level:?}");
            // Its top worry is named; the rest of the town is not known.
            let top = &options[0];
            assert_eq!(
                top.cells[0].label_key.as_deref(),
                Some(ranked[0].name_key.as_str()),
                "{tag}"
            );
            assert!(top.cells[0].class.is_some(), "{tag}");
            assert_eq!(top.unknown as usize, hoods.len() - 1, "{tag}");
            assert_eq!(
                details[0],
                ranked
                    .iter()
                    .take(cut.min(3))
                    .map(|x| x.name_key.clone())
                    .collect::<Vec<_>>(),
                "{tag}: the notebook's cut"
            );
            for o in &options[1..] {
                let place = ranked.iter().position(|x| x.id == o.id);
                let want = match place {
                    Some(0) => Some(2),
                    // Only the top worry is known at `heard`: the rest are
                    // unknown, not low.
                    _ if level == Level::Heard => None,
                    Some(1 | 2) => Some(1),
                    _ => Some(0),
                };
                assert_eq!(o.cells[0].class, want, "{tag}: {}", o.id);
                assert_eq!(
                    o.cells[0].class.is_some(),
                    o.cells[0].label_key.is_some(),
                    "{tag}"
                );
                assert!(
                    o.cells[1..].iter().all(|x| x.class.is_none()),
                    "{tag}: {}",
                    o.id
                );
                assert_eq!(
                    o.bins.iter().map(|b| b.count).sum::<u32>() + o.unknown,
                    hoods.len() as u32,
                    "{tag}"
                );
            }
        }
    }
}

fn req(id: u32, body: RequestBody) -> String {
    serde_json::to_string(&Request {
        protocol_version: PROTOCOL_VERSION,
        request_id: id,
        body,
    })
    .unwrap()
}

#[test]
fn the_query_serves_every_registry_layer_and_writes_nothing() {
    let mut s = Session::new().unwrap();
    s.handle_json(&req(
        1,
        RequestBody::NewGame(NewGameParams {
            seed: 777,
            size: WorldSizeDto::Small,
            scenario: None,
            storyteller: None,
            local_knowledge: false,
        }),
    ));
    s.handle_json(&req(2, RequestBody::Command(CommandDto::EndTurn)));
    let before = s.engine().unwrap().state_hash();
    let ledger = s
        .engine()
        .unwrap()
        .state()
        .player
        .as_ref()
        .unwrap()
        .knowledge
        .clone();
    let json = s.handle_json(&req(3, RequestBody::Query(QueryDto::TownMapLayers)));
    let r: Response = serde_json::from_str(&json).unwrap();
    let v = match r.result {
        ResponseResult::Ok {
            body: ResponseBody::TownMapLayers(v),
        } => v,
        other => panic!("{other:?}"),
    };
    let e = s.engine().unwrap();
    assert_eq!(e.state_hash(), before, "a query changes nothing");
    assert_eq!(e.state().player.as_ref().unwrap().knowledge, ledger);
    let registry = &e.content().town_map_layers;
    assert_eq!(registry.len(), 3);
    assert_eq!(v.layers.len(), registry.len());
    for (def, l) in registry.iter().zip(&v.layers) {
        assert_eq!(l.id, def.id);
        assert_eq!(l.source, def.source.key());
        assert_eq!(l.label_key, def.label_key);
        assert_eq!(l.args["tier"], v.at.tier);
        let kind = serde_json::to_value(&l.data).unwrap()["kind"].clone();
        assert_eq!(kind, def.kind.key(), "{}", def.id);
    }
    // The worries choices are those the registry's source asks for.
    let TownLayerDataView::Shaded {
        param,
        options,
        knowledge,
        ..
    } = layer(&v, "worries")
    else {
        panic!()
    };
    assert_eq!(param.as_deref(), Some("worry"));
    assert_eq!(options.len(), 1 + e.content().concerns.len());
    assert_eq!(knowledge.len(), v.hoods.len());
    // Small enough to fetch after every command.
    assert!(json.len() < 96 * 1024, "{} bytes", json.len());
    println!("town map layers: {} bytes", json.len());
}
