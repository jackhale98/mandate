//! Town layout v2 engine views (TOWN_GENERATION §4.1, §4.2; DEC-090 D3):
//! the geography view is static for a run, survives save and load, and is
//! pinned (the same hash is pinned for the WASM build in
//! `web/src/hub/fixtures/engineViews.test.ts`); the state view follows
//! DEC-079's information depth; closed places are kept, never deleted.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use mandate_protocol::*;
use mandate_sim::world::{MunicipalityId, PlaceKind, PlaceStatus};
use mandate_wasm::session::Session;
use std::sync::Arc;

/// FNV-1a (32-bit) of the view's JSON, as the client test computes it from
/// the WASM build's response.
const PINNED_GEOGRAPHY: (&str, u64, &str) = ("small", 777, "00000000");

fn fnv1a(s: &str) -> String {
    let mut h: u32 = 0x811c_9dc5;
    for b in s.bytes() {
        h ^= u32::from(b);
        h = h.wrapping_mul(0x0100_0193);
    }
    format!("{h:08x}")
}

fn req(id: u32, body: RequestBody) -> String {
    serde_json::to_string(&Request {
        protocol_version: PROTOCOL_VERSION,
        request_id: id,
        body,
    })
    .unwrap()
}

fn send(s: &mut Session, id: u32, body: RequestBody) -> ResponseBody {
    let r: Response = serde_json::from_str(&s.handle_json(&req(id, body))).unwrap();
    match r.result {
        ResponseResult::Ok { body } => body,
        ResponseResult::Err { error } => panic!("request {id} failed: {}", error.message),
    }
}

fn new_game(seed: u64) -> Session {
    let mut s = Session::new().unwrap();
    send(
        &mut s,
        1,
        RequestBody::NewGame(NewGameParams {
            seed,
            size: WorldSizeDto::Small,
            scenario: None,
            storyteller: None,
        }),
    );
    s
}

fn home_id(s: &Session) -> u32 {
    s.engine()
        .unwrap()
        .state()
        .player
        .as_ref()
        .unwrap()
        .municipality
        .0
}

fn geography(s: &mut Session, id: u32) -> TownGeographyView {
    match send(s, 90, RequestBody::Query(QueryDto::TownGeography { id })) {
        ResponseBody::TownGeography(v) => v,
        _ => panic!("not a geography view"),
    }
}

fn town_state(s: &mut Session, id: u32) -> TownStateView {
    match send(s, 91, RequestBody::Query(QueryDto::TownState { id })) {
        ResponseBody::TownState(v) => v,
        _ => panic!("not a state view"),
    }
}

fn save(s: &Session) -> Vec<u8> {
    s.export_save(&SaveRequestDto {
        run_id: "ab".repeat(16),
        revision: 1,
        parent_revision: None,
        device_id: "d".into(),
        saved_at: "2026-11-19T00:00:00Z".into(),
        run_mode: RunModeDto::Ironman,
    })
    .unwrap()
}

#[test]
fn geography_is_static_survives_save_and_load_and_is_pinned() {
    let (_, seed, pinned) = PINNED_GEOGRAPHY;
    let mut s = new_game(seed);
    let id = home_id(&s);
    let all: Vec<TownGeographyView> = (0..s.engine().unwrap().state().world.municipalities.len())
        .map(|m| geography(&mut s, m as u32))
        .collect();
    let g = all[id as usize].clone();
    let json = serde_json::to_string(&g).unwrap();
    // Months of play change nothing in it.
    for i in 0..4 {
        send(&mut s, 10 + i, RequestBody::Command(CommandDto::EndTurn));
    }
    assert_eq!(geography(&mut s, id), g, "static for a run");
    // Nor does a save and load (the terrain is rebuilt from the seed).
    let bytes = save(&s);
    let mut s2 = Session::new().unwrap();
    s2.import_save(&bytes).unwrap();
    for (m, v) in all.iter().enumerate() {
        assert_eq!(&geography(&mut s2, m as u32), v, "town {m} after load");
    }
    assert_eq!(
        fnv1a(&json),
        pinned,
        "the geography view changed; if intended, bump TOWN_GEOGRAPHY_VERSION and \
         re-pin here and in web/src/hub/fixtures/engineViews.test.ts:\n{json}"
    );
}

#[test]
fn geography_is_well_formed_for_every_town() {
    let mut s = new_game(4242);
    let st = s.engine().unwrap().state().clone();
    let w = &st.world;
    let mut rivers = 0;
    let mut coasts = 0;
    for (m, mu) in w.municipalities.iter().enumerate() {
        let g = geography(&mut s, m as u32);
        assert_eq!(g.version, TOWN_GEOGRAPHY_VERSION);
        assert_eq!(g.name, mu.name);
        let n = (g.patch.size * g.patch.size) as usize;
        for layer in [
            g.patch.cover.len(),
            g.patch.biome.len(),
            g.patch.river.len(),
            g.patch.downstream.len(),
            g.patch.forest_pm.len(),
            g.patch.minerals_pm.len(),
            g.patch.elevation_m.len(),
        ] {
            assert_eq!(layer, n);
        }
        let ok = |b: u8| b < 16 || b == 255;
        assert!(ok(g.wind_from) && ok(g.relief.uphill) && ok(g.relief.high_ground));
        assert!(ok(g.woods.bearing) && ok(g.minerals.bearing));
        assert!(g.patch.downstream.iter().all(|b| ok(*b) && (*b == 255 || b % 2 == 0)));
        // Neighbourhoods are the town's, in its order, with baselines.
        let ids: Vec<u32> = mu.neighborhoods.iter().map(|h| h.0).collect();
        assert_eq!(g.neighborhoods.iter().map(|h| h.id).collect::<Vec<_>>(), ids);
        assert_eq!(
            g.baseline_population,
            g.neighborhoods.iter().map(|h| h.baseline_population).sum::<u32>()
        );
        for h in &g.neighborhoods {
            assert!(h.adjacent.iter().all(|a| ids.contains(a) && *a != h.id));
        }
        if let Some(r) = &g.river {
            rivers += 1;
            assert!((1..=3).contains(&r.class));
            assert!(r.enters < 16 && r.leaves < 16);
            assert!(
                (r.enters as i32 - r.leaves as i32).rem_euclid(16) >= 4
                    && (r.leaves as i32 - r.enters as i32).rem_euclid(16) >= 4,
                "a river crosses the town"
            );
        }
        assert!(g.creeks.len() <= 2);
        if let Some(c) = &g.coast {
            coasts += 1;
            assert!(c.sea < 16);
        }
        assert_eq!(g.coast.is_some() || mu.coastal, g.coast.is_some());
        // Every link of the town leaves it toward a real municipality.
        let incident = w
            .links
            .iter()
            .filter(|l| l.a == mu.id || l.b == mu.id)
            .count();
        assert!(g.links.len() <= incident * 2 + 8);
        for l in &g.links {
            assert!(l.bearing < 16 && (l.to as usize) < w.municipalities.len());
            assert_eq!(l.to_name, w.municipalities[l.to as usize].name);
        }
        let sorted = g
            .links
            .windows(2)
            .all(|p| (p[0].bearing, p[0].mode, p[0].to) <= (p[1].bearing, p[1].mode, p[1].to));
        assert!(sorted);
    }
    assert!(rivers > 0 && coasts > 0, "{rivers} rivers, {coasts} coasts");
}

#[test]
fn town_state_is_rounded_except_at_the_town_hall() {
    let mut s = new_game(31337);
    let id = home_id(&s);
    let at_home = town_state(&mut s, id);
    assert!(!at_home.detailed);
    assert!(at_home.sectors.is_empty(), "no jobs per sector away from the hall");
    assert_eq!(at_home.population % 100, 0);
    assert_eq!(at_home.housing.stock % 100, 0);
    assert_eq!(at_home.housing.vacancy_pm % 10, 0);
    assert!(at_home.neighborhoods.iter().all(|h| h.population % 100 == 0));
    assert!(
        at_home
            .places
            .iter()
            .all(|p| p.status == TownPlaceStatus::Open && p.months == 0)
    );
    // At the town hall: precise.
    let hall = s
        .engine()
        .unwrap()
        .state()
        .world
        .places
        .iter()
        .find(|p| p.municipality.0 == id && p.kind == PlaceKind::TownHall)
        .unwrap()
        .id;
    send(
        &mut s,
        2,
        RequestBody::Command(CommandDto::MoveTo {
            location: LocationDto::Place { place: hall.0 },
        }),
    );
    let precise = town_state(&mut s, id);
    assert!(precise.detailed);
    assert!(!precise.sectors.is_empty());
    let e = s.engine().unwrap();
    let st = e.state();
    let base = st.town_baseline(MunicipalityId(id)).unwrap();
    for (h, b) in precise.neighborhoods.iter().zip(&base.neighborhoods) {
        assert_eq!(h.id, b.id.0);
        assert_eq!(u64::from(h.population), st.population.hood_total(b.id.index()));
    }
    // As play began, population and housing are at their baselines.
    assert!(
        precise
            .neighborhoods
            .iter()
            .all(|h| (990..=1010).contains(&h.population_pm))
    );
    assert_eq!(precise.housing.baseline_stock, base.housing_stock);
    assert!((990..=1010).contains(&precise.housing.stock_pm));
    // Rounding is to the nearest 100 and whole percent.
    assert_eq!(
        at_home.population,
        ((f64::from(precise.population) / 100.0).round() * 100.0) as u32
    );
    // Other towns are always rounded (the map is not a way to read them).
    let other = (id + 1) % st.world.municipalities.len() as u32;
    assert!(!town_state(&mut s, other).detailed);
    // The view is read-only: it never advances the state.
    let h = s.engine().unwrap().state_hash();
    town_state(&mut s, id);
    geography(&mut s, id);
    assert_eq!(s.engine().unwrap().state_hash(), h);
}

/// DEC-090 D3: a place that shuts is marked closed, never deleted. It keeps
/// its id and its slot, offers nothing, and cannot be entered; saves keep it.
#[test]
fn closed_places_are_kept_and_offer_nothing() {
    let mut s = new_game(4242);
    let id = home_id(&s);
    let hash_before = s.engine().unwrap().state_hash();
    // While every place is open the state (and its hash) has no status field.
    let json = serde_json::to_string(&s.engine().unwrap().state().world.places).unwrap();
    assert!(!json.contains("status") && !json.contains("opened"));
    let content = Arc::new(s.content().clone());
    let bytes = save(&s);
    let (state, _) = mandate_save::decode(&bytes).unwrap();
    let mut e = mandate_sim::Engine::from_state(state, content.clone()).unwrap();
    assert_eq!(e.state_hash(), hash_before);
    let diner = e
        .state()
        .world
        .places
        .iter()
        .find(|p| p.municipality.0 == id && p.kind == PlaceKind::Diner)
        .unwrap()
        .id;
    let n_places = e.state().world.places.len();
    let today = e.state().calendar.date;
    e.setup(|st| assert!(st.world.close_place(diner, today)));
    let st = e.state();
    assert_eq!(st.world.places.len(), n_places, "nothing is deleted");
    assert!(st.world.municipalities[id as usize].places.contains(&diner));
    assert_eq!(
        st.world.places[diner.index()].status,
        PlaceStatus::Closed { since: today }
    );
    let loc = mandate_sim::places::Location::Place { place: diner };
    assert!(mandate_sim::places::validate_move(st, loc).is_err());
    assert!(mandate_sim::places::actions_at(st, &content, loc).is_empty());
    assert!(mandate_sim::places::services_at(st, &content, loc).is_empty());
    assert_ne!(e.state_hash(), hash_before);
    // The state view reports it closed; a save keeps it closed.
    let v = town_state_view(&e, id).unwrap();
    let p = v.places.iter().find(|p| p.id == diner.0).unwrap();
    assert_eq!(p.status, TownPlaceStatus::Closed);
    let (bytes, _) = mandate_save::encode(
        e.state(),
        &mandate_save::SaveRequest {
            run_id: "cd".repeat(16),
            revision: 2,
            parent_revision: None,
            device_id: "d".into(),
            saved_at: "2026-11-19T00:00:00Z".into(),
            protocol_version: PROTOCOL_VERSION,
            run_mode: mandate_save::RunMode::Ironman,
        },
    )
    .unwrap();
    let (back, _) = mandate_save::decode(&bytes).unwrap();
    assert_eq!(
        back.world.places[diner.index()].status,
        PlaceStatus::Closed { since: today }
    );
    // Reopened, the state is as it was.
    e.setup(|st| assert!(st.world.reopen_place(diner)));
    assert_eq!(e.state_hash(), hash_before);
    // A place added during the run takes the next id and shows as new.
    let hood = e.state().world.municipalities[id as usize].neighborhoods[0];
    let mut added = None;
    e.setup(|st| added = st.world.open_place(PlaceKind::Clinic, hood, String::new(), today));
    let added = added.unwrap();
    assert_eq!(added.index(), n_places);
    let v = town_state_view(&e, id).unwrap();
    assert_eq!(v.places.last().unwrap().id, added.0);
    assert_eq!(v.places.last().unwrap().status, TownPlaceStatus::New);
}
