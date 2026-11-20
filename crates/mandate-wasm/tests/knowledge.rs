//! Protocol redaction (DEC-091; M6_ENGINE_CONTRACTS §1.7, §1.8; task K2):
//! every gated view field at each knowledge level, the fields that are
//! never gated, the "Start with local knowledge" option at new game, the
//! notebook, and that views never write (the state hash is unchanged by
//! any query).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use mandate_protocol::*;
use mandate_sim::characters::CharacterId;
use mandate_sim::places::Location;
use mandate_sim::player::knowledge::{LedgerOrigin, Level, PolicyRef, Source, Subject};
use mandate_sim::world::{MunicipalityId, PlaceKind};
use mandate_sim::{Command, Engine};
use mandate_wasm::session::Session;
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::sync::Arc;

const LEVELS: [Level; 4] = [Level::Unknown, Level::Heard, Level::Known, Level::Detailed];

fn content() -> Arc<mandate_content::ContentBundle> {
    let (c, _) = mandate_content::load(&mandate_content::embedded::sources()).unwrap();
    Arc::new(c)
}

fn game(seed: u64) -> Engine {
    generate(
        &WorldConfig {
            seed,
            size: WorldSize::Small,
            scenario: None,
        },
        content(),
    )
    .unwrap()
}

/// Put a subject at exactly `level` (test setup, not a command).
fn set(e: &mut Engine, s: &Subject, level: Level) {
    e.setup(|st| {
        let on = st.calendar.date;
        let k = &mut st.player.as_mut().unwrap().knowledge;
        k.lab_omniscient = false;
        k.entries.remove(s);
        k.learn(s.clone(), level, on, Source::Lab);
    });
}

/// Forget everything (test setup).
fn wipe(e: &mut Engine) {
    e.setup(|st| {
        let k = &mut st.player.as_mut().unwrap().knowledge;
        k.lab_omniscient = false;
        k.entries.clear();
        k.leads.clear();
    });
}

fn town(e: &Engine) -> MunicipalityId {
    e.state().player.as_ref().unwrap().municipality
}

fn place_of(e: &Engine, kind: PlaceKind) -> Option<mandate_sim::world::PlaceId> {
    let st = e.state();
    st.world
        .places
        .iter()
        .find(|p| p.municipality == town(e) && p.kind == kind && p.is_open())
        .map(|p| p.id)
}

fn stand_at(e: &mut Engine, loc: Location) {
    e.setup(|st| st.player.as_mut().unwrap().location = loc);
}

fn me(e: &Engine) -> CharacterId {
    mandate_sim::characters::player_character(e.state()).unwrap()
}

/// A view's JSON without the knowledge levels that travel with names (a
/// decision's cast is named in full; its level is reported as it is).
fn no_levels<T: serde::Serialize>(v: &T) -> serde_json::Value {
    fn strip(v: &mut serde_json::Value) {
        match v {
            serde_json::Value::Object(m) => {
                m.remove("level");
                m.values_mut().for_each(strip);
            }
            serde_json::Value::Array(a) => a.iter_mut().for_each(strip),
            _ => {}
        }
    }
    let mut j = serde_json::to_value(v).unwrap();
    strip(&mut j);
    j
}

fn tenth(v: f64) -> bool {
    ((v * 10.0).round() - v * 10.0).abs() < 1e-9
}

#[test]
fn town_concerns_by_level() {
    let mut e = game(11);
    let s = Subject::TownConcerns { town: town(&e) };
    e.set_lab_omniscient(true);
    let full = office_view(&e).unwrap().town.concerns;
    let hall = place_of(&e, PlaceKind::TownHall).unwrap();
    let home = e.state().player.as_ref().unwrap().location;
    stand_at(&mut e, Location::Place { place: hall });
    let exact = office_view(&e).unwrap().town.concerns;
    stand_at(&mut e, home);
    assert!(full.len() >= 3, "a town has several concerns");
    for (level, pocket, prio, office) in [
        (Level::Unknown, 0, 0, 0),
        (Level::Heard, 1, 1, 1),
        (Level::Known, 2, 3, 3),
        (Level::Detailed, 2, 3, full.len()),
    ] {
        set(&mut e, &s, level);
        let p = pocket_view(&e).unwrap();
        assert_eq!(p.headlines.top_concern_keys.len(), pocket, "{level:?}");
        assert_eq!(p.headlines.concerns_knowledge.level, level.into());
        let c = town_places_view(&e).unwrap().character.unwrap();
        assert_eq!(c.priorities.len(), prio, "{level:?}");
        assert_eq!(c.priorities_knowledge.level, level.into());
        let o = office_view(&e).unwrap();
        assert_eq!(o.town.concerns.len(), office, "{level:?}");
        assert_eq!(o.town.concerns_knowledge.level, level.into());
        // Ranked as the truth, never reordered.
        for (a, b) in o.town.concerns.iter().zip(&full) {
            assert_eq!(a.id, b.id);
        }
        match level {
            Level::Heard => assert_eq!(c.priorities[0].weight, 0.0),
            Level::Known => assert!(c.priorities.iter().all(|x| tenth(x.weight))),
            Level::Detailed => assert_eq!(c.priorities[0].weight, exact[0].weight),
            Level::Unknown => {}
        }
        // Away from the statistics desk the office's figures are coarse at
        // any level (the stricter of place and knowledge wins).
        assert!(o.town.concerns.iter().all(|x| tenth(x.weight)));
    }
    // At the desk: exact only with detailed knowledge; the desk's own data
    // is live for anything heard of.
    stand_at(&mut e, Location::Place { place: hall });
    let desk = LocationDto::Place { place: hall.0 };
    set(&mut e, &s, Level::Detailed);
    assert_eq!(office_view(&e).unwrap().town.concerns, exact);
    let data = place_panel_view(&e, desk).unwrap().town_data.unwrap();
    assert_eq!(data.concerns.len(), full.len());
    set(&mut e, &s, Level::Known);
    assert!(tenth(office_view(&e).unwrap().town.concerns[0].weight));
    set(&mut e, &s, Level::Heard);
    let data = place_panel_view(&e, desk).unwrap().town_data.unwrap();
    assert_eq!(
        data.concerns.len(),
        full.len(),
        "live at the desk once heard"
    );
    set(&mut e, &s, Level::Unknown);
    let data = place_panel_view(&e, desk).unwrap().town_data.unwrap();
    assert!(data.concerns.is_empty());
    // Public statistics are never gated.
    assert!(data.population > 0.0);
}

#[test]
fn hood_concerns_by_level() {
    let mut e = game(12);
    let hall = place_of(&e, PlaceKind::TownHall).unwrap();
    stand_at(&mut e, Location::Place { place: hall });
    let hoods = town_places_view(&e).unwrap().neighborhoods;
    let h = &hoods[0];
    let s = Subject::HoodConcerns {
        hood: mandate_sim::world::NeighborhoodId(h.id),
    };
    for level in LEVELS {
        set(&mut e, &s, level);
        let v = town_places_view(&e).unwrap();
        let n = v.neighborhoods.iter().find(|x| x.id == h.id).unwrap();
        assert_eq!(
            n.top_concern_key.is_some(),
            level >= Level::Heard,
            "{level:?}"
        );
        assert_eq!(n.concern_knowledge.level, level.into());
        let data = place_panel_view(&e, LocationDto::Place { place: hall.0 })
            .unwrap()
            .town_data
            .unwrap();
        let row = data.neighborhoods.iter().find(|x| x.id == h.id).unwrap();
        assert_eq!(row.top_concern_key.is_some(), level >= Level::Heard);
        assert_eq!(row.top_concern_key, n.top_concern_key);
    }
}

#[test]
fn broker_by_level() {
    // A town whose broker exists.
    let (mut e, broker) = (1..40)
        .find_map(|seed| {
            let e = game(seed);
            let b = e.state().politics.characters.towns[town(&e).index()].broker?;
            Some((e, b))
        })
        .expect("some town has a broker");
    let s = Subject::Broker { town: town(&e) };
    // The person is known (so the sheet's name shows); `is_broker` follows the broker subject.
    set(&mut e, &Subject::Person { person: broker }, Level::Known);
    for level in LEVELS {
        set(&mut e, &s, level);
        let c = town_places_view(&e).unwrap().character.unwrap();
        assert_eq!(c.broker_knowledge.level, level.into());
        match level {
            Level::Unknown => assert!(c.broker.is_none()),
            _ => {
                let b = c.broker.as_ref().unwrap();
                assert_eq!(b.id, Some(broker.0));
                assert!(!b.name.is_empty());
                assert_eq!(b.rumoured, level == Level::Heard);
            }
        }
        assert_eq!(
            c.broker_stake_key.is_some(),
            level == Level::Detailed && c.industry.is_some()
        );
        let sheet = character_view(&e, broker.0).unwrap();
        assert_eq!(sheet.is_broker, level >= Level::Known, "{level:?}");
    }
}

#[test]
fn people_by_level() {
    let mut e = game(13);
    // A councillor (not the player) and a worker at a place.
    let st = e.state();
    let council = mandate_sim::politics::body_members_at(
        &st.politics,
        mandate_sim::world::Jurisdiction::town(town(&e)),
    );
    let member = council
        .iter()
        .map(|id| st.politics.get(*id).unwrap())
        .find(|p| !p.is_player)
        .unwrap();
    let (mid, mchar) = (member.id.0, member.character);
    let (place, worker) = st
        .world
        .places
        .iter()
        .filter(|p| p.municipality == town(&e))
        .find_map(|p| {
            let at = mandate_sim::characters::ambient::regulars_at(
                &st.politics,
                st,
                Location::Place { place: p.id },
            );
            // Someone whose regular place this is (not a councillor in the chamber).
            at.into_iter()
                .find(|c| {
                    mandate_sim::characters::profile(&st.politics, &st.world, *c)
                        .is_some_and(|x| x.place == Some(p.id))
                })
                .map(|c| (p.id, c))
        })
        .unwrap();
    // Someone with goals, for the detailed sheet.
    for level in LEVELS {
        for who in [mchar, worker] {
            set(&mut e, &Subject::Person { person: who }, level);
        }
        let o = office_view(&e).unwrap();
        let m = o.council.iter().find(|m| m.id == mid).unwrap();
        assert_eq!(m.name.is_empty(), level < Level::Heard, "{level:?}");
        assert_eq!(m.level, level.into());
        assert!(!m.role_key.is_empty(), "role and ward are public");
        assert_eq!(
            m.convictions.len(),
            if level >= Level::Known { 2 } else { 0 }
        );
        assert_eq!(m.pragmatism.is_some(), level == Level::Detailed);
        let panel = place_panel_view(&e, LocationDto::Place { place: place.0 }).unwrap();
        let p = panel
            .people
            .iter()
            .find(|p| p.id == Some(worker.0))
            .unwrap();
        assert_eq!(p.name.is_empty(), level < Level::Heard);
        assert!(!p.role_key.is_empty(), "the role is always shown");
        assert_eq!(p.level, level.into());
        let c = character_view(&e, worker.0).unwrap();
        assert_eq!(c.name.is_empty(), level < Level::Heard);
        assert_eq!(c.place.is_some(), level >= Level::Heard);
        assert_eq!(c.knowledge.level, level.into());
        if level < Level::Known {
            assert!(c.personality.is_empty() && c.traits.is_empty());
            assert!(c.expertise_key.is_none());
        }
        assert_eq!(c.goals.is_some(), level == Level::Detailed);
        assert_eq!(c.favours_with_others.is_some(), level == Level::Detailed);
        // How they see the player is never gated.
        assert!(!c.opinion_band.is_empty() && !c.trust_band.is_empty());
    }
    // Passers-by are strangers until met; the lab sees them.
    let mut any = false;
    for p in &e.state().world.places {
        if p.municipality != town(&e) {
            continue;
        }
        let panel = place_panel_view(&e, LocationDto::Place { place: p.id.0 }).unwrap();
        for x in panel.people.iter().filter(|x| x.tier == "ambient") {
            any = true;
            assert!(x.name.is_empty());
            assert_eq!(x.level, KnowledgeLevelDto::Unknown);
        }
    }
    assert!(any, "some place has passers-by");
}

#[test]
fn the_player_is_never_gated() {
    let mut e = game(14);
    wipe(&mut e);
    let pid = e.state().player.as_ref().unwrap().politician.0;
    let o = office_view(&e).unwrap();
    let mine = o.council.iter().find(|m| m.id == pid).unwrap();
    assert!(!mine.name.is_empty());
    assert_eq!(mine.convictions.len(), 2);
    assert!(mine.pragmatism.is_some());
    let sheet = character_view(&e, me(&e).0).unwrap();
    assert!(sheet.is_player && !sheet.name.is_empty());
    assert_eq!(sheet.knowledge.level, KnowledgeLevelDto::Detailed);
    // Own state, the rules, action costs and odds, deadlines.
    let p = pocket_view(&e).unwrap();
    assert!(p.slots_total > 0 && !p.routes.is_empty());
    assert!(!o.player.name.is_empty() && !o.player.action_costs.is_empty());
}

#[test]
fn rival_plans_by_level() {
    let mut e = game(15);
    let c = town_places_view(&e).unwrap().character.unwrap();
    assert!(c.rival.is_some(), "the town has a rival");
    let s = Subject::RivalPlans { town: town(&e) };
    for (level, key) in [
        (Level::Unknown, None),
        (Level::Heard, Some("rival-plan-something")),
        (Level::Known, Some("rival-plan-seat")),
        (Level::Detailed, Some("rival-plan-focus")),
    ] {
        set(&mut e, &s, level);
        let c = town_places_view(&e).unwrap().character.unwrap();
        // The rival's identity is never gated.
        assert!(!c.rival.as_ref().unwrap().name.is_empty());
        let plan = c.rival_plan.as_ref();
        assert_eq!(plan.map(|p| p.key.as_str()), key, "{level:?}");
        if let Some(p) = plan {
            assert_eq!(p.knowledge.level, level.into());
            assert_eq!(p.post.is_some(), level >= Level::Known);
            assert_eq!(p.focus_key.is_some(), level == Level::Detailed);
        }
    }
}

#[test]
fn policy_support_by_level() {
    let mut e = game(16);
    let m = town(&e);
    let template = "policy.transit_frequency".to_string();
    assert!(e.content().policies.contains_key(&template));
    let s = Subject::PolicySupport {
        town: m,
        policy: PolicyRef::Template {
            template: template.clone(),
        },
    };
    e.set_lab_omniscient(true);
    let exact = office_view(&e)
        .unwrap()
        .catalog
        .into_iter()
        .find(|t| t.id == template)
        .unwrap()
        .support;
    for level in LEVELS {
        set(&mut e, &s, level);
        let o = office_view(&e).unwrap();
        let t = o.catalog.iter().find(|t| t.id == template).unwrap();
        // The standard menu is never gated: parameters, cost and side effects stay.
        assert!(!t.params.is_empty() && t.est_cost_default > 0.0);
        assert!(t.knowledge.is_none());
        let sup = &t.support;
        assert_eq!(sup.level, level.into());
        assert_eq!(sup.band, exact.band, "the band is always shown");
        let preview = policy_preview(&e, &template, &[]).unwrap().support;
        assert_eq!(&preview, sup);
        match level {
            Level::Unknown | Level::Heard => {
                assert!(sup.low.is_none() && sup.middle.is_none());
                assert!(sup.high.is_none() && sup.overall.is_none());
            }
            Level::Known => {
                assert!(tenth(sup.low.unwrap()) && tenth(sup.overall.unwrap()));
            }
            Level::Detailed => assert_eq!(sup, &exact),
        }
    }
    // A bill's ward support: to 0.25, 0.1, exact.
    assert_eq!(gate_ward_support(Level::Heard, 0.37), 0.25);
    assert!((gate_ward_support(Level::Known, 0.37) - 0.4).abs() < 1e-9);
}

#[test]
fn bills_by_level() {
    // Play a few turns so that someone else's bill is before the council.
    let (mut e, bill) = (17..27)
        .find_map(|seed| {
            let mut e = game(seed);
            for _ in 0..10 {
                e.apply(Command::EndTurn).unwrap();
            }
            e.set_lab_omniscient(true);
            let bill = office_view(&e)
                .unwrap()
                .bills
                .into_iter()
                .find(|b| !b.sponsor_is_player && !b.sponsor_name.is_empty())?;
            Some((e, bill))
        })
        .expect("someone else's bill in ten towns");
    let st = e.state();
    let inst = st
        .policy
        .instances
        .iter()
        .find(|p| p.id.0 == bill.id)
        .unwrap();
    let sponsor = st.politics.get(inst.sponsor.unwrap()).unwrap().character;
    let support = Subject::PolicySupport {
        town: town(&e),
        policy: PolicyRef::Template {
            template: inst.template.clone(),
        },
    };
    let exact_ward = bill.ward_support;
    for level in LEVELS {
        set(&mut e, &Subject::Person { person: sponsor }, level);
        set(&mut e, &support, level);
        let o = office_view(&e).unwrap();
        let b = o.bills.iter().find(|b| b.id == bill.id).unwrap();
        assert_eq!(b.sponsor_name.is_empty(), level < Level::Heard);
        assert_eq!(b.support.level, level.into());
        assert_eq!(
            b.ward_support,
            gate_ward_support(level, exact_ward),
            "{level:?}"
        );
    }
}

#[test]
fn situational_policies_by_level() {
    // A run where a situational policy is on the table.
    let found = (1..60).find_map(|seed| {
        let mut e = game(seed);
        e.set_lab_omniscient(true);
        let t = office_view(&e)
            .unwrap()
            .catalog
            .into_iter()
            .find(|t| t.situational && t.available)?;
        Some((e, t))
    });
    let Some((mut e, full)) = found else {
        panic!("no situational policy on the table in 60 towns");
    };
    let s = Subject::Situational {
        town: town(&e),
        template: full.id.clone(),
    };
    for level in LEVELS {
        set(&mut e, &s, level);
        let o = office_view(&e).unwrap();
        let t = o.catalog.iter().find(|t| t.id == full.id);
        match level {
            Level::Unknown => assert!(t.is_none(), "omitted while unknown"),
            Level::Heard => {
                let t = t.unwrap();
                assert_eq!(
                    t.knowledge.as_ref().unwrap().level,
                    KnowledgeLevelDto::Heard
                );
                assert!(t.params.is_empty() && t.side_effects.is_empty());
                assert_eq!(t.est_cost_default, 0.0);
                assert_eq!(t.unlock_reason_key, full.unlock_reason_key);
                assert_eq!(t.name_key, full.name_key);
            }
            _ => {
                let t = t.unwrap();
                assert_eq!(t.params, full.params);
                assert_eq!(t.est_cost_default, full.est_cost_default);
            }
        }
        // The standard menu is listed whatever the player knows.
        assert!(o.catalog.iter().filter(|t| !t.situational).count() >= 8);
    }
}

#[test]
fn ordinary_places_are_always_listed() {
    let mut e = game(18);
    wipe(&mut e);
    let v = town_places_view(&e).unwrap();
    let listed = v
        .neighborhoods
        .iter()
        .flat_map(|h| &h.spots)
        .filter(|s| matches!(s.location.location, LocationDto::Place { .. }))
        .count();
    let st = e.state();
    let mu = &st.world.municipalities[town(&e).index()];
    assert_eq!(listed, mu.places.len());
}

#[test]
fn never_gated_with_an_empty_ledger() {
    let mut e = game(19);
    for _ in 0..14 {
        e.apply(Command::EndTurn).unwrap();
    }
    e.set_lab_omniscient(true);
    let omni_story = story_view(&e);
    let omni_pocket = pocket_view(&e).unwrap();
    wipe(&mut e);
    let p = pocket_view(&e).unwrap();
    // Decisions, alerts, news, the legal view, deadlines: aimed at the player.
    assert_eq!(no_levels(&p.decisions), no_levels(&omni_pocket.decisions));
    assert_eq!(p.alerts, omni_pocket.alerts);
    assert_eq!(p.news, omni_pocket.news);
    assert_eq!(p.legal, omni_pocket.legal);
    assert_eq!(p.bills.len(), omni_pocket.bills.len());
    assert_eq!(p.money, omni_pocket.money);
    assert_eq!(p.headlines.unemployment, omni_pocket.headlines.unemployment);
    assert_eq!(no_levels(&story_view(&e)), no_levels(&omni_story));
    for d in &p.decisions {
        assert!(
            d.people.iter().all(|x| !x.name.is_empty()),
            "a decision's cast is named"
        );
    }
    // Campaigns, polls and results; the map's public layers.
    e.set_lab_omniscient(true);
    let omni_campaign = campaign_view(&e);
    let omni_town = municipality_view(&e, town(&e).0);
    wipe(&mut e);
    assert_eq!(campaign_view(&e), omni_campaign);
    assert_eq!(municipality_view(&e, town(&e).0), omni_town);
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

fn session(seed: u64, local_knowledge: bool) -> Session {
    let mut s = Session::new().unwrap();
    send(
        &mut s,
        1,
        RequestBody::NewGame(NewGameParams {
            seed,
            size: WorldSizeDto::Small,
            scenario: None,
            storyteller: None,
            local_knowledge,
        }),
    );
    s
}

#[test]
fn local_knowledge_option_at_new_game() {
    let off = session(21, false);
    let on = session(21, true);
    let k = |s: &Session| {
        s.engine()
            .unwrap()
            .state()
            .player
            .as_ref()
            .unwrap()
            .knowledge
            .clone()
    };
    assert!(matches!(k(&off).origin, LedgerOrigin::Profile { .. }));
    assert!(matches!(k(&on).origin, LedgerOrigin::LocalOption { .. }));
    let t = town(off.engine().unwrap());
    let broker = Subject::Broker { town: t };
    assert_eq!(k(&off).level(&broker), Level::Unknown);
    assert_eq!(k(&on).level(&broker), Level::Known);
    let card = |s: &Session| {
        town_places_view(s.engine().unwrap())
            .unwrap()
            .character
            .unwrap()
    };
    assert!(card(&off).broker.is_none());
    let b = card(&on).broker.expect("this town has a broker");
    assert!(!b.rumoured && !b.name.is_empty());
    // Every neighbourhood's worries, as a local knows them.
    assert!(
        town_places_view(on.engine().unwrap())
            .unwrap()
            .neighborhoods
            .iter()
            .all(|h| h.top_concern_key.is_some()
                || h.concern_knowledge.level == KnowledgeLevelDto::Known)
    );
    // The option is engine state: the worlds are otherwise the same.
    assert_ne!(
        off.engine().unwrap().state_hash(),
        on.engine().unwrap().state_hash()
    );
    let n = |s: &Session| notebook_view(s.engine().unwrap()).unwrap();
    assert_eq!(n(&off).origin, "profile");
    assert_eq!(n(&on).origin, "local_option");
    // Omitted in JSON, it is off (`#[serde(default)]`).
    let mut s = Session::new().unwrap();
    let r = s.handle_json(
        r#"{"protocol_version":1,"request_id":1,"body":{"kind":"new_game","payload":{"seed":"21","size":"small","scenario":null,"storyteller":null}}}"#,
    );
    assert!(r.contains(r#""status":"ok""#), "{r}");
    assert_eq!(
        s.engine().unwrap().state_hash(),
        off.engine().unwrap().state_hash()
    );
}

#[test]
fn the_notebook_lists_what_is_known() {
    let mut s = session(22, false);
    for i in 0..6 {
        send(&mut s, 10 + i, RequestBody::Command(CommandDto::EndTurn));
    }
    let ResponseBody::Notebook(n) = send(&mut s, 30, RequestBody::Query(QueryDto::Notebook)) else {
        panic!("not a notebook");
    };
    let e = s.engine().unwrap();
    let ledger = &e.state().player.as_ref().unwrap().knowledge;
    let entries: usize = n.sections.iter().map(|x| x.entries.len()).sum();
    assert_eq!(entries, ledger.entries.len());
    assert!(n.sections.iter().all(|x| !x.entries.is_empty()));
    assert_eq!(n.origin, "profile");
    assert!(n.zone.as_deref().is_some_and(|z| !z.is_empty()));
    let order: Vec<&str> = n.sections.iter().map(|x| x.key.as_str()).collect();
    let mut sorted = order.clone();
    sorted.sort_by_key(|k| NOTEBOOK_SECTIONS.iter().position(|x| x == k));
    assert_eq!(order, sorted);
    for sec in &n.sections {
        for x in &sec.entries {
            assert!(x.level >= KnowledgeLevelDto::Heard);
            assert!(!x.history.is_empty());
            assert!(x.history.windows(2).all(|w| w[0].level < w[1].level));
        }
    }
    let first = &n.sections[0].entries[0];
    assert!(first.history[0].key.starts_with("learned-"));
    assert_eq!(n.leads.len(), ledger.leads.len());
    // The pocket's badge counts this turn's subjects.
    assert_eq!(pocket_view(e).unwrap().notebook_new, notebook_new(e));
}

/// The notebook badge counts what was learned in play, not what a run
/// starts with (start profile, local knowledge, primer).
#[test]
fn the_notebook_badge_starts_at_zero() {
    for local in [false, true] {
        let mut s = session(23, local);
        let e = s.engine().unwrap();
        assert!(
            !e.state()
                .player
                .as_ref()
                .unwrap()
                .knowledge
                .entries
                .is_empty()
        );
        assert_eq!(notebook_new(e), 0, "local knowledge {local}");
        assert_eq!(pocket_view(e).unwrap().notebook_new, 0);
        // Canvass a neighbourhood (walked there first): that is news.
        let hood = town_places_view(e).unwrap().neighborhoods[0].id;
        send(
            &mut s,
            40,
            RequestBody::Command(CommandDto::Go {
                location: LocationDto::Street { neighborhood: hood },
                then: Some(Box::new(CommandDto::Act {
                    action: ActionDto::Canvass {
                        neighborhood: hood,
                        issue: None,
                    },
                    late_night: false,
                })),
            }),
        );
        assert!(
            notebook_new(s.engine().unwrap()) > 0,
            "local knowledge {local}"
        );
    }
}

/// Every Fluent key a gated view or the notebook can emit exists in English.
#[test]
fn knowledge_keys_are_in_the_locale() {
    let c = content();
    let ftl = c.locale_text("en");
    let has = |k: &str| {
        ftl.lines()
            .any(|l| l.split_once(" =").is_some_and(|(id, _)| id.trim() == k))
    };
    let mut keys: Vec<String> = Vec::new();
    for l in ["unknown", "heard", "known", "detailed"] {
        keys.push(format!("knowledge-level-{l}"));
    }
    for b in ["popular", "divisive", "unpopular"] {
        keys.push(format!("support-band-{b}"));
    }
    for k in [
        "rival-plan-something",
        "rival-plan-seat",
        "rival-plan-focus",
        "rival-seat-office",
        "rival-seat-ward",
        "knowledge-broker-unknown",
        "knowledge-person-unnamed",
        "knowledge-rumoured",
        "knowledge-proposal-stub",
    ] {
        keys.push(k.into());
    }
    for s in NOTEBOOK_SECTIONS {
        keys.push(format!("notebook-section-{s}"));
    }
    for s in [
        "town-concerns",
        "hood-concerns",
        "broker",
        "person",
        "rival-plans",
        "policy-support",
        "proposal",
        "situational",
        "place",
        "layer",
        "secret",
    ] {
        keys.push(format!("notebook-{s}"));
    }
    for s in [
        "start",
        "local-option",
        "primer",
        "migrated",
        "visit",
        "overheard",
        "action",
        "met",
        "service",
        "story",
        "decision",
        "council",
        "sponsor",
        "talk",
        "lab",
    ] {
        keys.push(format!("learned-{s}"));
    }
    // And what a played notebook says.
    let mut s = session(23, true);
    for i in 0..4 {
        send(&mut s, 10 + i, RequestBody::Command(CommandDto::EndTurn));
    }
    let n = notebook_view(s.engine().unwrap()).unwrap();
    for x in n.sections.iter().flat_map(|x| &x.entries) {
        keys.push(x.title_key.clone());
        keys.extend(x.history.iter().map(|h| h.key.clone()));
        keys.extend(
            x.args
                .iter()
                .chain(x.history.iter().flat_map(|h| &h.args))
                .filter(|(k, _)| k.ends_with("_key"))
                .map(|(_, v)| v.clone()),
        );
    }
    keys.extend(n.leads.iter().map(|l| l.key.clone()));
    // K5: placeholders and their standing leads, from a run without the option.
    let off = session(23, false);
    let n = notebook_view(off.engine().unwrap()).unwrap();
    assert!(!n.gaps.is_empty());
    for g in &n.gaps {
        keys.push(g.title_key.clone());
        keys.extend(g.lead.iter().map(|l| l.key.clone()));
        keys.extend(
            g.lead
                .iter()
                .flat_map(|l| &l.args)
                .filter(|(k, _)| k.ends_with("_key"))
                .map(|(_, v)| v.clone()),
        );
    }
    for x in n.sections.iter().flat_map(|x| &x.entries) {
        keys.extend(x.lead.iter().map(|l| l.key.clone()));
    }
    let missing: Vec<&String> = keys.iter().filter(|k| !has(k)).collect();
    assert!(missing.is_empty(), "missing Fluent keys: {missing:?}");
}

/// K5: the notebook's placeholders cover public subjects only, each with a
/// way to learn it; learning one fills its entry; `NotebookSeen` clears the
/// badge and the entries' "new" marks, survives a save, and changes nothing
/// but the ledger's seen marks.
#[test]
fn the_notebook_shows_gaps_and_seeing_it_clears_the_badge() {
    let mut s = session(25, false);
    let e = s.engine().unwrap();
    let n = notebook_view(e).unwrap();
    let ledger = &e.state().player.as_ref().unwrap().knowledge;
    // Gaps are unknown subjects, never people or places, each with a lead.
    for g in &n.gaps {
        assert!(matches!(
            g.subject,
            SubjectDto::TownConcerns { .. }
                | SubjectDto::HoodConcerns { .. }
                | SubjectDto::Broker { .. }
                | SubjectDto::RivalPlans { .. }
                | SubjectDto::PolicySupport { .. }
        ));
        assert!(g.lead.is_some(), "{g:?}");
        assert!(!g.args.contains_key("name") && !g.args.contains_key("concern_key"));
    }
    // A neighbourhood not known yet: canvassing it is the lead.
    let gap = n
        .gaps
        .iter()
        .find(|g| matches!(g.subject, SubjectDto::HoodConcerns { .. }))
        .expect("the default start knows only the ward's worries");
    let SubjectDto::HoodConcerns { hood } = gap.subject else {
        unreachable!()
    };
    let lead = gap.lead.as_ref().unwrap();
    assert_eq!(lead.key, "lead-notebook-canvass");
    assert_eq!(lead.level, KnowledgeLevelDto::Known);
    assert_eq!(lead.subject, gap.subject);
    assert!(lead.route.is_some());
    // The broker is a gap; its lead does not name them.
    let broker = n
        .gaps
        .iter()
        .find(|g| matches!(g.subject, SubjectDto::Broker { .. }))
        .expect("the broker is not known without the option");
    assert!(!broker.lead.as_ref().unwrap().args.contains_key("name"));
    assert!(!ledger.entries.is_empty());
    assert_eq!(notebook_new(e), 0);
    // Follow the lead: canvass there.
    let r = send(
        &mut s,
        40,
        RequestBody::Command(CommandDto::Go {
            location: LocationDto::Street { neighborhood: hood },
            then: Some(Box::new(CommandDto::Act {
                action: ActionDto::Canvass {
                    neighborhood: hood,
                    issue: None,
                },
                late_night: false,
            })),
        }),
    );
    assert!(matches!(r, ResponseBody::CommandApplied(_)));
    let e = s.engine().unwrap();
    let n = notebook_view(e).unwrap();
    assert!(!n.gaps.iter().any(|g| g.subject == gap.subject));
    let entry = n
        .sections
        .iter()
        .flat_map(|x| &x.entries)
        .find(|x| x.subject == gap.subject)
        .expect("the canvass filled the entry");
    assert!(entry.new_this_turn);
    assert!(entry.args.contains_key("concern_key"));
    let fresh = notebook_new(e);
    assert!(fresh > 0);
    assert_eq!(
        n.sections
            .iter()
            .flat_map(|x| &x.entries)
            .filter(|x| x.new_this_turn)
            .count() as u32,
        fresh
    );
    // Seeing the notebook clears the badge and the marks.
    let before = serde_json::to_value(e.state()).unwrap();
    let r = send(&mut s, 41, RequestBody::Command(CommandDto::NotebookSeen));
    assert!(matches!(r, ResponseBody::CommandApplied(_)));
    let e = s.engine().unwrap();
    assert_eq!(notebook_new(e), 0);
    assert_eq!(pocket_view(e).unwrap().notebook_new, 0);
    let n = notebook_view(e).unwrap();
    assert!(
        n.sections
            .iter()
            .flat_map(|x| &x.entries)
            .all(|x| !x.new_this_turn)
    );
    // Nothing but the seen marks changed.
    fn strip(v: &mut serde_json::Value) {
        match v {
            serde_json::Value::Object(m) => {
                m.remove("seen");
                m.values_mut().for_each(strip);
            }
            serde_json::Value::Array(a) => a.iter_mut().for_each(strip),
            _ => {}
        }
    }
    let mut before = before;
    let mut after = serde_json::to_value(e.state()).unwrap();
    assert!(after != before, "the seen marks are state");
    strip(&mut after);
    strip(&mut before);
    assert!(after == before, "only the seen marks change");
    // The marks are state: they survive a save and load.
    let json = serde_json::to_string(e.state()).unwrap();
    let back: mandate_sim::State = serde_json::from_str(&json).unwrap();
    let pl = back.player.as_ref().unwrap();
    assert!(pl.knowledge.entries.values().all(|x| x.seen == x.level));
    // Twice is harmless; a later lesson is new again.
    send(&mut s, 42, RequestBody::Command(CommandDto::NotebookSeen));
    assert_eq!(notebook_new(s.engine().unwrap()), 0);
}

/// Every query, twice: views never write, so the state hash is unchanged,
/// and it matches a session that made no queries at all.
#[test]
fn views_do_not_change_the_state_hash() {
    let mut quiet = session(24, false);
    let mut busy = session(24, false);
    for i in 0..3 {
        send(
            &mut quiet,
            10 + i,
            RequestBody::Command(CommandDto::EndTurn),
        );
        send(&mut busy, 10 + i, RequestBody::Command(CommandDto::EndTurn));
    }
    let e = busy.engine().unwrap();
    let st = e.state();
    let m = town(e).0;
    let place = st.world.municipalities[m as usize].places[0].0;
    let person = me(e).0;
    let template = "policy.transit_frequency".to_string();
    let queries = vec![
        QueryDto::Overview,
        QueryDto::Office,
        QueryDto::Campaign,
        QueryDto::Map,
        QueryDto::Municipality { id: m },
        QueryDto::Pocket,
        QueryDto::Town,
        QueryDto::Place {
            location: LocationDto::Place { place },
        },
        QueryDto::Character { id: person },
        QueryDto::Story,
        QueryDto::Newspaper,
        QueryDto::PolicyPreview {
            template,
            params: vec![],
        },
        QueryDto::Diagnostics,
        QueryDto::TownGeography { id: m },
        QueryDto::TownState { id: m },
        QueryDto::Notebook,
    ];
    let before = busy.engine().unwrap().state_hash();
    for round in 0..2 {
        for (i, q) in queries.iter().enumerate() {
            let r: Response = serde_json::from_str(
                &busy.handle_json(&req(100 + i as u32, RequestBody::Query(q.clone()))),
            )
            .unwrap();
            // Terrain-backed views need the raster, which this session has.
            assert!(
                matches!(r.result, ResponseResult::Ok { .. }),
                "round {round}: {q:?} failed"
            );
        }
    }
    assert_eq!(busy.engine().unwrap().state_hash(), before);
    assert_eq!(quiet.engine().unwrap().state_hash(), before);
    // And the game goes on identically.
    send(&mut quiet, 200, RequestBody::Command(CommandDto::EndTurn));
    send(&mut busy, 200, RequestBody::Command(CommandDto::EndTurn));
    assert_eq!(
        quiet.engine().unwrap().state_hash(),
        busy.engine().unwrap().state_hash()
    );
}
