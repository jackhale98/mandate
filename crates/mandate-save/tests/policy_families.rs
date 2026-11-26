//! Policy families in the simulation (DEC-092, DEC-098, DEC-099; task F2b),
//! with the sample families of `mandate-content/tests/fixtures/families`:
//!
//! - proposals open in the focus town, bound inside it, with decks keyed by
//!   jurisdiction; nothing opens elsewhere;
//! - a run with families is deterministic, twice and through save/load, and
//!   its family state survives save/load; a v23 save migrates;
//! - drafting needs the proposal `known`; a draft composes its choices (the
//!   area, slot references), a sponsor's version pays its price and pleases
//!   them, and amending away from it costs goodwill;
//! - families evaluate under a renamed constitution, and a family of another
//!   tier evaluates at its own tier with tier-generic conditions while one
//!   with local-only conditions never binds there;
//! - works start with an enacted bill, build, open, and when the law ends
//!   stall and decay, deterministically;
//! - missing families and options refuse a save under changed rules.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use mandate_content::policy_family::PolicyFamilyDef;
use mandate_content::{ContentBundle, Source, TierKey, bundle::read_dir_sources, load};
use mandate_save::*;
use mandate_sim::actions::PlayerCommand;
use mandate_sim::places::Location;
use mandate_sim::player::knowledge::{self, Level, Subject};
use mandate_sim::policy::families::{Bound, FamilyInstanceId, FamilyStatus};
use mandate_sim::policy::works::WorksStage;
use mandate_sim::policy::{PolicyInstanceId, PolicyStatus};
use mandate_sim::world::{Jurisdiction, PlaceKind};
use mandate_sim::{Command, Engine, State};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

const FAMILIES: &str =
    include_str!("../../mandate-content/tests/fixtures/families/test_families.jsonc");
const FTL: &str = include_str!("../../mandate-content/tests/fixtures/families/test_families.ftl");

fn base_sources() -> Vec<Source> {
    read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap()
}

/// The sample families beside the base pack, whose own families (F4) a
/// `disable` patch takes out of play, so the samples are what opens.
fn with_families(mut src: Vec<Source>) -> Vec<Source> {
    let base = load(&src).unwrap_or_else(|e| panic!("{e:#?}")).0;
    let items: Vec<serde_json::Value> = base
        .policy_families
        .iter()
        .map(|f| serde_json::json!({ "op": "disable", "family": f.id }))
        .collect();
    src.push(Source {
        path: "zz_test/policy_family_patches.jsonc".into(),
        text: serde_json::json!({ "kind": "policy_family_patches", "items": items }).to_string(),
    });
    src.push(Source {
        path: "zz_test/policy_families/test.jsonc".into(),
        text: FAMILIES.into(),
    });
    src.push(Source {
        path: "zz_test/locale/en/test.ftl".into(),
        text: FTL.into(),
    });
    src
}

fn content() -> Arc<ContentBundle> {
    Arc::new(
        load(&with_families(base_sources()))
            .unwrap_or_else(|e| panic!("{e:#?}"))
            .0,
    )
}

/// The base pack with its local posts renamed (as R1's test): the council
/// is a `moot` of `alderman`s, the mayor a `reeve`.
fn renamed() -> Arc<ContentBundle> {
    let mut src = base_sources();
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
            t = t.replace("act.local", "act.borough");
        }
        if p.ends_with("endings.jsonc") {
            t = t.replace("act.local", "act.borough");
        }
        s.text = t;
    }
    Arc::new(
        load(&with_families(src))
            .unwrap_or_else(|e| panic!("{e:#?}"))
            .0,
    )
}

fn game(c: &Arc<ContentBundle>, seed: u64) -> Engine {
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

fn months(e: &mut Engine, n: u32) {
    e.apply(Command::AdvanceDays { days: 31 * n }).unwrap();
}

fn req() -> SaveRequest {
    SaveRequest {
        run_id: "00112233445566778899aabbccddeeff".into(),
        revision: 1,
        parent_revision: None,
        device_id: "test-device".into(),
        saved_at: "2026-01-01T00:00:00Z".into(),
        protocol_version: 1,
        run_mode: RunMode::Ironman,
    }
}

fn home(s: &State) -> Jurisdiction {
    Jurisdiction::town(s.player.as_ref().unwrap().home())
}

fn open_of(s: &State, family: &str) -> Option<FamilyInstanceId> {
    s.policy
        .families
        .instances
        .iter()
        .find(|i| i.is_open() && i.family == family)
        .map(|i| i.id)
}

/// A game with an open proposal of `family` (searching seeds).
fn game_with(c: &Arc<ContentBundle>, family: &str) -> (Engine, FamilyInstanceId) {
    for seed in 1..40 {
        let mut e = game(c, seed);
        months(&mut e, 2);
        if let Some(id) = open_of(e.state(), family) {
            return (e, id);
        }
    }
    panic!("no seed opens {family}");
}

/// The player knows a proposal, and stands in the town hall.
fn ready_to_draft(e: &mut Engine, proposal: FamilyInstanceId, level: Level) {
    e.setup(|s| {
        knowledge::teach_one(
            s,
            Subject::Proposal { proposal },
            level,
            knowledge::Source::Lab,
        );
        let town = s.player.as_ref().unwrap().home();
        let hall = s
            .world
            .places
            .iter()
            .find(|p| p.municipality == town && p.kind == PlaceKind::TownHall)
            .map(|p| p.id)
            .expect("a town hall");
        s.player.as_mut().unwrap().location = Location::Place { place: hall };
    });
}

fn draft(
    e: &mut Engine,
    proposal: FamilyInstanceId,
    choices: &[(&str, &str)],
    sponsor: Option<&str>,
) -> Result<PolicyInstanceId, String> {
    let out = e
        .apply(Command::Player(PlayerCommand::DraftFamily {
            proposal: proposal.0,
            choices: choices
                .iter()
                .map(|(a, o)| (a.to_string(), o.to_string()))
                .collect(),
            params: BTreeMap::new(),
            sponsor: sponsor.map(str::to_string),
        }))
        .map_err(|e| e.to_string())?;
    Ok(PolicyInstanceId(out.result.unwrap().created.unwrap()))
}

#[test]
fn proposals_open_in_the_focus_town_and_bind_inside_it() {
    let c = content();
    let mut opened = 0;
    for seed in [3, 11, 29] {
        let mut e = game(&c, seed);
        months(&mut e, 3);
        let s = e.state();
        let at = home(s);
        let deck = s
            .policy
            .families
            .decks
            .get(&at)
            .expect("the home town's deck");
        // Every sample family is `always`: all three are in the deck, and
        // only the focus jurisdiction has one.
        assert_eq!(deck.len(), 3);
        assert_eq!(s.policy.families.decks.len(), 1);
        for i in &s.policy.families.instances {
            opened += 1;
            assert_eq!(i.at, at, "proposals open in the focus town only");
            for b in &i.bindings {
                match &b.bound {
                    Bound::Place { place } => {
                        let p = &s.world.places[place.index()];
                        assert!(at.contains_hood(&s.world, p.neighborhood));
                        assert!(i.args.contains_key(&b.slot));
                    }
                    Bound::Neighborhood { hood } => {
                        assert!(at.contains_hood(&s.world, *hood));
                        assert!(i.args.contains_key(&b.slot));
                    }
                    Bound::Hazard { .. } | Bound::Sector { .. } | Bound::Crisis { .. } => {
                        assert!(i.args.contains_key(&format!("{}_key", b.slot)));
                    }
                    Bound::Person { .. } | Bound::Unbound => {}
                }
            }
        }
        // The caps: at most `max_open_per_jurisdiction`, one per family
        // (two flood walls), and a second flood wall is somewhere else.
        let open: Vec<_> = s.policy.families.open_at(at).collect();
        assert!(open.len() <= c.policy_family_params.max_open_per_jurisdiction as usize);
        let walls: Vec<_> = open
            .iter()
            .filter(|i| i.family == "family.test_flood_wall")
            .collect();
        assert!(walls.len() <= 2);
        if walls.len() == 2 {
            assert_ne!(walls[0].bindings[0], walls[1].bindings[0]);
        }
        // The lead is posted to the player's notebook.
        let pl = s.player.as_ref().unwrap();
        assert!(
            pl.knowledge
                .leads
                .iter()
                .any(|l| matches!(l.subject, Subject::Proposal { .. })),
            "a lead for a proposal"
        );
    }
    assert!(opened > 0);
}

#[test]
fn families_are_deterministic_and_survive_save_load() {
    let c = content();
    let mut a = game(&c, 17);
    let mut b = game(&c, 17);
    months(&mut a, 4);
    months(&mut b, 4);
    assert_eq!(a.state_hash(), b.state_hash());
    assert!(!a.state().policy.families.instances.is_empty());
    // Save and load mid-run: the family state round-trips and the run
    // continues identically.
    let (bytes, meta) = encode(a.state(), &req()).unwrap();
    assert_eq!(meta.format_version, SAVE_FORMAT_VERSION);
    let (state, _) = decode(&bytes).unwrap();
    assert_eq!(state.policy.families, a.state().policy.families);
    assert_eq!(state.hash(), a.state_hash());
    let mut loaded = Engine::from_state(state, c.clone()).unwrap();
    months(&mut loaded, 5);
    months(&mut a, 5);
    assert_eq!(loaded.state_hash(), a.state_hash());
}

#[test]
fn v23_and_v24_saves_migrate() {
    // The base pack (no families): the migrated state is the same state.
    // A v23 or v24 save holds no arc (arcs came with v27): no arc content,
    // so none is raised in the first months (a plant on short time raises
    // one).
    let base = {
        let mut b = load(&base_sources()).unwrap().0;
        b.arcs.clear();
        // The base pack's own families (F4) out: no family state to migrate.
        b.policy_families.clear();
        Arc::new(b)
    };
    let mut e = game(&base, 5);
    months(&mut e, 2);
    let mut v =
        serde_json::json!({ "state": serde_json::to_value(e.state()).unwrap(), "log": null });
    let p = v["state"]["policy"].as_object_mut().unwrap();
    p.remove("families");
    p.remove("works");
    let migrated = migrate(24, v.clone()).unwrap();
    let st: State = serde_json::from_value(migrated["state"].clone()).unwrap();
    assert_eq!(st.hash(), e.state_hash());
    // From v23, the asks' step (E6) runs first: only `asks` and the family
    // state come back at their defaults.
    let migrated = migrate(23, v).unwrap();
    let mut st: State = serde_json::from_value(migrated["state"].clone()).unwrap();
    st.asks = e.state().asks.clone();
    assert_eq!(st.hash(), e.state_hash());
    // With families: a v23 run has none; the decks are drawn on load
    // (keyed: as a fresh start would) and proposals open at the next pass.
    // No arc content either, as above.
    let c = {
        let mut b = (*content()).clone();
        b.arcs.clear();
        Arc::new(b)
    };
    let mut e = game(&c, 5);
    months(&mut e, 2);
    let deck = e.state().policy.families.decks.clone();
    let mut v =
        serde_json::json!({ "state": serde_json::to_value(e.state()).unwrap(), "log": null });
    let p = v["state"]["policy"].as_object_mut().unwrap();
    p.remove("families");
    p.remove("works");
    let migrated = migrate(23, v).unwrap();
    let st: State = serde_json::from_value(migrated["state"].clone()).unwrap();
    assert!(st.policy.families.instances.is_empty());
    let mut m = Engine::from_state(st, c.clone()).unwrap();
    assert_eq!(m.state().policy.families.decks, deck);
    months(&mut m, 1);
    assert!(!m.state().policy.families.instances.is_empty());
}

#[test]
fn drafting_needs_knowledge_and_composes_the_choices() {
    let c = content();
    let (mut e, id) = game_with(&c, "family.test_flood_wall");
    // Only heard of: refused (DEC-091).
    ready_to_draft(&mut e, id, Level::Heard);
    let err = draft(&mut e, id, &[], None).unwrap_err();
    assert!(err.contains("heard"), "{err}");
    ready_to_draft(&mut e, id, Level::Known);
    // An unknown option, or one of another axis, is refused.
    assert!(draft(&mut e, id, &[("where", "moon")], None).is_err());
    assert!(draft(&mut e, id, &[("colour", "red")], None).is_err());
    let bill = draft(
        &mut e,
        id,
        &[("where", "riverfront"), ("builder", "contractor")],
        None,
    )
    .unwrap();
    let s = e.state();
    let p = s.policy.get(bill).unwrap();
    assert_eq!(p.template, "family.test_flood_wall");
    assert_eq!(p.status, PolicyStatus::Draft);
    let d = p.family.as_ref().unwrap();
    assert_eq!(d.instance, id);
    assert_eq!(d.choice("where"), Some("riverfront"));
    let inst = s.policy.families.get(id).unwrap();
    assert!(inst.drafts.contains(&bill));
    // The area: the bound neighbourhood and those beside it.
    let hood = inst.hood("hood").unwrap();
    let area = p.area().expect("an area clause");
    assert!(area.contains(&hood));
    // Composition: the contractor's margin and speed, and `@hazard` bound.
    let t = mandate_sim::policy::resolve(&c, &s.policy, p).unwrap();
    let f = c.family("family.test_flood_wall").unwrap();
    assert_eq!(
        t.implementation_months,
        f.base.implementation_months + 6 - 6
    );
    assert!((t.cost.per_capita_per_unit - 110.0 * 1.25).abs() < 1e-9);
    let bound_hazard = match inst.bound("hazard") {
        Some(Bound::Hazard { hazard }) => *hazard,
        _ => panic!("hazard bound"),
    };
    assert!(t.mechanisms.iter().any(|m| matches!(
        m,
        mandate_content::MechanismDef::HazardProtection {
            hazard: mandate_content::HazardRef::Key(h),
            ..
        } if *h == bound_hazard
    )));
    // Per-capita costs count the area's residents only.
    let town = s
        .population
        .municipality_total(&s.world, s.player.as_ref().unwrap().home()) as f64;
    let res = mandate_sim::policy::cost_residents(&s.world, &s.population, p, town);
    assert!(res > 0.0 && res <= town);
    // A standard bill still drafts and resolves to its template.
    let std = mandate_sim::policy::probe(&c.policies["policy.parks"], 0, &BTreeMap::new());
    let r = mandate_sim::policy::resolve(&c, &s.policy, &std).unwrap();
    assert_eq!(r.template(), &c.policies["policy.parks"]);
}

#[test]
fn a_sponsors_version_pays_its_price_and_amending_costs_goodwill() {
    let c = content();
    // The builders' version: a favour; the residents': an endorsement.
    for seed in 1..40 {
        let mut e = game(&c, seed);
        months(&mut e, 2);
        let Some(id) = open_of(e.state(), "family.test_flood_wall") else {
            continue;
        };
        let inst = e.state().policy.families.get(id).unwrap().clone();
        let Some(person) = inst.sponsor("builders") else {
            continue;
        };
        ready_to_draft(&mut e, id, Level::Known);
        // A sponsor's version keeps its preset.
        assert!(draft(&mut e, id, &[("builder", "landowners")], Some("builders")).is_err());
        let me = mandate_sim::characters::player_character(e.state()).unwrap();
        let before = mandate_sim::characters::opinion(&e.state().politics, person, me);
        let owed = mandate_sim::characters::favours_owed(&e.state().politics, me, person);
        let bill = draft(&mut e, id, &[], Some("builders")).unwrap();
        let s = e.state();
        let p = s.policy.get(bill).unwrap();
        assert_eq!(
            p.family.as_ref().unwrap().choice("builder"),
            Some("contractor")
        );
        assert_eq!(p.param("height"), 2.0);
        assert_eq!(
            mandate_sim::characters::favours_owed(&s.politics, me, person),
            owed + 1,
            "the favour is owed"
        );
        let after = mandate_sim::characters::opinion(&s.politics, person, me);
        assert!(
            after > before,
            "adopting pleases the sponsor: {before} → {after}"
        );
        // Amending away from the preset costs goodwill.
        e.apply(Command::Player(PlayerCommand::AmendPolicy {
            policy: bill.0,
            params: BTreeMap::new(),
            clauses: Vec::new(),
            choices: Some([("builder".to_string(), "works_team".to_string())].into()),
        }))
        .unwrap();
        let s = e.state();
        let amended = mandate_sim::characters::opinion(&s.politics, person, me);
        assert!(
            amended < after,
            "amending costs goodwill: {after} → {amended}"
        );
        let d = s.policy.get(bill).unwrap().family.clone().unwrap();
        assert_eq!(d.choice("builder"), Some("works_team"));
        assert!(d.changed_axes >= 1);
        return;
    }
    panic!("no seed casts the builders");
}

#[test]
fn families_evaluate_under_a_renamed_constitution() {
    let c = renamed();
    let (mut e, id) = game_with(&c, "family.test_flood_wall");
    // The executive fallback reads the constitution's executive (a reeve).
    ready_to_draft(&mut e, id, Level::Known);
    let bill = draft(&mut e, id, &[], None).unwrap();
    assert!(e.state().policy.get(bill).is_some());
}

/// A family of another tier evaluates at its own tier (DEC-098): with
/// tier-generic conditions it binds across a region; with local-only
/// conditions it never does; and nothing opens outside the focus.
#[test]
fn other_tiers_evaluate_with_generic_conditions_only() {
    let c = content();
    let mut e = game(&c, 9);
    months(&mut e, 2);
    let s = e.state();
    let town = home(s);
    let region = town.parent(&s.world).unwrap();
    let regional = |id: &str| -> PolicyFamilyDef {
        let mut f = c.family(id).unwrap().clone();
        f.tier = TierKey::Regional;
        f
    };
    // The market family's trigger is tier-generic (`months_in_play`).
    let market = regional("family.test_market");
    let b = mandate_sim::policy::families::probe(s, &c, &market, region);
    assert!(b.is_ok(), "{b:?}");
    // The flood wall's trigger reads the town (local-only): never at a region.
    let wall = regional("family.test_flood_wall");
    assert_eq!(
        mandate_sim::policy::families::probe(s, &c, &wall, region),
        Err("trigger".into())
    );
    // The plant rescue's `place_open` (local-only) likewise.
    let plant = regional("family.test_plant_rescue");
    assert!(mandate_sim::policy::families::probe(s, &c, &plant, region).is_err());
    // A family is evaluated only at its own tier.
    assert_eq!(
        mandate_sim::policy::families::probe(s, &c, &market, town),
        Err("tier".into())
    );
    // The same family at the town binds there.
    let local = c.family("family.test_market").unwrap();
    if let Ok(b) = mandate_sim::policy::families::probe(s, &c, local, town) {
        assert!(!b.is_empty());
    }
    // Nothing opens outside the focus.
    assert!(s.policy.families.instances.iter().all(|i| i.at == town));
}

/// Draft a family bill and enact it, funded.
fn enacted(e: &mut Engine, id: FamilyInstanceId) -> PolicyInstanceId {
    ready_to_draft(e, id, Level::Known);
    let bill = draft(e, id, &[], None).unwrap();
    e.setup(|s| {
        let date = s.calendar.date;
        let p = s.policy.get_mut(bill).unwrap();
        p.status = PolicyStatus::Enacted;
        p.enacted_on = Some(date);
    });
    bill
}

fn fund(e: &mut Engine, bill: PolicyInstanceId) {
    e.setup(|s| {
        if let Some(a) = s.finance.appropriations.get_mut(&bill.0) {
            a.funded_share = 1.0;
        }
    });
}

#[test]
fn works_build_open_stall_and_decay_deterministically() {
    let c = content();
    let (mut e, id) = game_with(&c, "family.test_market");
    let bill = enacted(&mut e, id);
    months(&mut e, 1);
    fund(&mut e, bill);
    let w = &e.state().policy.works.projects;
    assert_eq!(w.len(), 1, "an enacted bill approves its works");
    assert_eq!(w[0].kind, "market_stalls");
    assert_eq!(w[0].policy, bill);
    // Fully funded, four months of building open it.
    for _ in 0..6 {
        months(&mut e, 1);
        fund(&mut e, bill);
    }
    let w = &e.state().policy.works.projects[0];
    assert!(matches!(w.stage, WorksStage::Open { .. }), "{:?}", w.stage);
    assert_eq!(w.progress, 1.0);
    // The read API: works by neighbourhood with stage and condition.
    let by = mandate_sim::policy::works::by_hood(&e.state().policy);
    let here = by.get(&w.hood).unwrap();
    assert_eq!(here[0].stage, "open");
    assert_eq!(here[0].condition, 1.0);

    // A second work, its law repealed while it is building: it stalls, then
    // decays, then is abandoned; twice the same.
    let c2 = content();
    let (mut a, id) = game_with(&c2, "family.test_flood_wall");
    let bill = enacted(&mut a, id);
    months(&mut a, 1);
    fund(&mut a, bill);
    months(&mut a, 1);
    a.setup(|s| {
        let p = s.policy.get_mut(bill).unwrap();
        p.status = PolicyStatus::Ended;
        p.end_reason = Some(mandate_sim::policy::EndReason::Repealed);
    });
    let mut b = Engine::from_state(a.state().clone(), c2.clone()).unwrap();
    months(&mut a, 2);
    let w = a
        .state()
        .policy
        .works
        .projects
        .iter()
        .find(|w| w.policy == bill)
        .unwrap()
        .clone();
    assert!(
        matches!(w.stage, WorksStage::Stalled { .. }),
        "{:?}",
        w.stage
    );
    assert!(w.progress < 1.0);
    assert!(w.condition < 1.0, "a stalled site decays");
    months(&mut a, 14);
    let w2 = a
        .state()
        .policy
        .works
        .projects
        .iter()
        .find(|w| w.policy == bill)
        .unwrap();
    assert!(
        matches!(w2.stage, WorksStage::Abandoned { .. }),
        "{:?}",
        w2.stage
    );
    assert!(w2.condition < w.condition);
    months(&mut b, 16);
    assert_eq!(b.state().policy.works, a.state().policy.works);
}

#[test]
fn missing_families_and_options_refuse_a_save() {
    let c = content();
    let (mut e, id) = game_with(&c, "family.test_flood_wall");
    ready_to_draft(&mut e, id, Level::Known);
    draft(&mut e, id, &[("builder", "landowners")], None).unwrap();
    // Without the family.
    let base = load(&base_sources()).unwrap().0;
    let missing = mandate_sim::engine::missing_references(e.state(), &base);
    assert!(
        missing.iter().any(|m| m == "family.test_flood_wall"),
        "{missing:?}"
    );
    // Without the chosen option (a patch removed it).
    let patch = r#"{ "kind": "policy_family_patches", "items": [
        { "op": "remove_option", "family": "family.test_flood_wall", "axis": "builder", "option": "landowners" } ] }"#;
    let mut src = with_families(base_sources());
    src.push(Source {
        path: "zz_patch/patch.jsonc".into(),
        text: patch.into(),
    });
    let patched = load(&src).unwrap().0;
    let missing = mandate_sim::engine::missing_references(e.state(), &patched);
    assert!(
        missing.iter().any(|m| m.contains("builder.landowners")),
        "{missing:?}"
    );
    // A disabled family's open proposals lapse on load; its bill stays.
    let patch = r#"{ "kind": "policy_family_patches", "items": [
        { "op": "disable", "family": "family.test_flood_wall" } ] }"#;
    let mut src = with_families(base_sources());
    src.push(Source {
        path: "zz_patch/patch.jsonc".into(),
        text: patch.into(),
    });
    let disabled = Arc::new(load(&src).unwrap().0);
    let mut st = e.state().clone();
    st.meta.content_hash = "changed".into();
    let m = Engine::from_state(st, disabled).unwrap();
    let inst = m.state().policy.families.get(id).unwrap();
    assert!(matches!(inst.status, FamilyStatus::Lapsed { .. }));
    assert!(
        m.state()
            .policy
            .instances
            .iter()
            .any(|p| p.family.is_some())
    );
    // The bill keeps composing under the disabled family (its rules remain).
    let p = m
        .state()
        .policy
        .instances
        .iter()
        .find(|p| p.family.is_some())
        .unwrap();
    assert!(mandate_sim::policy::resolve(m.content(), &m.state().policy, p).is_some());
}
