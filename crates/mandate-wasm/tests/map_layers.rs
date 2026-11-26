//! The country map's layers (DEC-089): every registry layer resolves to a
//! figure the engine serves, with a value and a class for every town; the
//! legend's classes cover the towns; the view stays small.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use mandate_content::map_layers::MapLayerSource;
use mandate_protocol::*;
use mandate_wasm::session::Session;

fn req(id: u32, body: RequestBody) -> String {
    serde_json::to_string(&Request {
        protocol_version: PROTOCOL_VERSION,
        request_id: id,
        body,
    })
    .unwrap()
}

fn session(seed: u64, size: WorldSizeDto) -> Session {
    let mut s = Session::new().unwrap();
    s.handle_json(&req(
        1,
        RequestBody::NewGame(NewGameParams {
            seed,
            size,
            scenario: None,
            storyteller: None,
            local_knowledge: false,
            recent_openings: Vec::new(),
        }),
    ));
    s
}

fn query(s: &mut Session, q: QueryDto) -> ResponseBody {
    let r: Response = serde_json::from_str(&s.handle_json(&req(2, RequestBody::Query(q)))).unwrap();
    match r.result {
        ResponseResult::Ok { body } => body,
        other => panic!("{other:?}"),
    }
}

fn layers(s: &mut Session) -> MapLayersView {
    match query(s, QueryDto::MapLayers) {
        ResponseBody::MapLayers(v) => v,
        other => panic!("{other:?}"),
    }
}

fn map(s: &mut Session) -> MapView {
    match query(s, QueryDto::Map) {
        ResponseBody::Map(v) => v,
        other => panic!("{other:?}"),
    }
}

#[test]
fn every_layer_resolves_to_a_served_field_with_values_for_every_town() {
    let mut s = session(777, WorldSizeDto::Small);
    let content = s.engine().unwrap().content().clone();
    assert_eq!(content.map_layers.len(), 7);
    let m = map(&mut s);
    // Advance a turn so the figures are the simulation's, not just the start's.
    s.handle_json(&req(3, RequestBody::Command(CommandDto::EndTurn)));
    let v = layers(&mut s);
    assert_eq!(v.layers.len(), content.map_layers.len());
    assert_eq!(
        v.towns.iter().map(|t| t.id).collect::<Vec<_>>(),
        m.municipalities.iter().map(|t| t.id).collect::<Vec<_>>(),
        "towns follow the map's municipalities"
    );
    let towns = serde_json::to_value(&v.towns).unwrap();
    for (def, layer) in content.map_layers.iter().zip(&v.layers) {
        assert_eq!(layer.id, def.id);
        assert_eq!(layer.source, def.source.key());
        assert_eq!(layer.classes.len(), v.towns.len(), "{}", def.id);
        for (t, class) in towns.as_array().unwrap().iter().zip(&layer.classes) {
            let field = t
                .get(&layer.source)
                .unwrap_or_else(|| panic!("{}: no field `{}`", def.id, layer.source));
            assert!(
                !field.is_null(),
                "{}: town {} has no value",
                def.id,
                t["id"]
            );
            let class = class.unwrap_or_else(|| panic!("{}: town {} unclassed", def.id, t["id"]));
            assert!((class as usize) < layer.bins.len(), "{}", def.id);
        }
        // The legend's counts are the towns in each class.
        let total: u32 = layer.bins.iter().map(|b| b.count).sum();
        assert_eq!(total as usize, v.towns.len(), "{}", def.id);
        match layer.scale {
            MapLayerScaleView::Categorical => {
                assert!(layer.bins.len() <= def.legend.classes as usize + 1);
                assert!(layer.bins.iter().all(|b| b.count > 0));
            }
            MapLayerScaleView::Diverging => {
                assert_eq!(layer.bins.len(), def.legend.classes as usize);
                let mid = layer.midpoint.unwrap();
                let c = layer.bins.len() / 2;
                let b = &layer.bins[c];
                assert!(b.lo.unwrap() <= mid && mid < b.hi.unwrap(), "{b:?} {mid}");
            }
            MapLayerScaleView::Sequential => {
                assert!(layer.bins.len() >= 2, "{}", def.id);
                assert!(layer.bins.first().unwrap().lo.is_none());
                assert!(layer.bins.last().unwrap().hi.is_none());
                for w in layer.bins.windows(2) {
                    assert!(w[0].hi.unwrap() < w[1].hi.unwrap_or(f64::INFINITY));
                }
            }
        }
    }
    // Every source the registry can name is a field of the town figures.
    for src in MapLayerSource::ALL {
        assert!(towns[0].get(src.key()).is_some(), "{}", src.key());
    }
}

#[test]
fn layers_are_reproducible_and_survive_save_load() {
    let mut a = session(5, WorldSizeDto::Small);
    let mut b = session(5, WorldSizeDto::Small);
    assert_eq!(layers(&mut a), layers(&mut b));
    let before = layers(&mut a);
    let save = a
        .export_save(&SaveRequestDto {
            run_id: "ab".repeat(16),
            revision: 1,
            parent_revision: None,
            device_id: "d".into(),
            saved_at: "2026-11-19T00:00:00Z".into(),
            run_mode: RunModeDto::Story,
        })
        .unwrap();
    let mut c = Session::new().unwrap();
    c.import_save(&save).unwrap();
    assert_eq!(layers(&mut c), before);
}

#[test]
fn the_layers_view_is_small() {
    let mut s = session(777, WorldSizeDto::Large);
    let json = serde_json::to_string(&layers(&mut s)).unwrap();
    // 84 towns: about 24 KB, beside a 0.5 MB raster.
    assert!(json.len() < 40_000, "{} bytes", json.len());
}

/// `cargo test -p mandate-wasm --test map_layers -- --ignored --nocapture`
#[test]
#[ignore]
fn measure_map_payload() {
    for size in [
        WorldSizeDto::Small,
        WorldSizeDto::Medium,
        WorldSizeDto::Large,
    ] {
        let mut s = session(777, size);
        let m = serde_json::to_string(&map(&mut s)).unwrap();
        let l = serde_json::to_string(&layers(&mut s)).unwrap();
        let v = layers(&mut s);
        eprintln!(
            "{size:?}: {} towns, map {} bytes, layers {} bytes (towns {} bytes)",
            v.towns.len(),
            m.len(),
            l.len(),
            serde_json::to_string(&v.towns).unwrap().len()
        );
        if size == WorldSizeDto::Small {
            eprintln!("{}", serde_json::to_string(&v.towns[0]).unwrap());
            for l in &v.layers {
                eprintln!(
                    "{} {:?} {}",
                    l.id,
                    l.midpoint,
                    serde_json::to_string(&l.bins).unwrap()
                );
            }
        }
    }
}
