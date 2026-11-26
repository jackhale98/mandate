//! M5d part 2: situational policies (DEC-077 §3), the policy preview query,
//! the storyteller chosen at new game (DEC-033), and grounded overheard talk
//! (DEC-079). Native tests of the engine and the protocol the UI calls.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use mandate_content::{Condition, ContentBundle, WaterFeature};
use mandate_protocol::*;
use mandate_sim::actions::PlayerCommand;
use mandate_sim::places::{Location, Offer, locations_for};
use mandate_sim::{Command, Engine};
use mandate_wasm::session::Session;
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// The base pack with its situational templates on offer again: F4 retired
/// them into policy families (DEC-092 §2.20), and the mechanism they run on
/// (`unlock`) stays, so these tests keep proving it.
fn content() -> Arc<ContentBundle> {
    let mut src = mandate_content::embedded::sources();
    for s in src.iter_mut() {
        if s.path.ends_with("policies/situational.jsonc") {
            s.text = s.text.replace("\"retired\": true, ", "");
        }
    }
    Arc::new(mandate_content::load(&src).unwrap().0)
}

fn world(seed: u64) -> Engine {
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

fn req(id: u32, body: RequestBody) -> String {
    serde_json::to_string(&Request {
        protocol_version: PROTOCOL_VERSION,
        request_id: id,
        body,
    })
    .unwrap()
}

fn send(s: &mut Session, id: u32, body: RequestBody) -> Result<ResponseBody, ProtocolError> {
    let r: Response = serde_json::from_str(&s.handle_json(&req(id, body))).unwrap();
    match r.result {
        ResponseResult::Ok { body } => Ok(body),
        ResponseResult::Err { error } => Err(error),
    }
}

fn preview(s: &mut Session, template: &str, params: &[(&str, f64)]) -> PolicyPreviewView {
    match send(
        s,
        9,
        RequestBody::Query(QueryDto::PolicyPreview {
            template: template.into(),
            params: params
                .iter()
                .map(|(id, value)| ParamValue {
                    id: (*id).into(),
                    value: *value,
                })
                .collect(),
            proposal: None,
            choices: None,
        }),
    )
    .unwrap()
    {
        ResponseBody::PolicyPreview(p) => p,
        other => panic!("{other:?}"),
    }
}

fn new_game(s: &mut Session, seed: u64, storyteller: Option<&str>) -> OverviewView {
    match send(
        s,
        1,
        RequestBody::NewGame(NewGameParams {
            seed,
            size: WorldSizeDto::Small,
            scenario: None,
            storyteller: storyteller.map(String::from),
            local_knowledge: false,
            recent_openings: Vec::new(),
        }),
    )
    .unwrap()
    {
        ResponseBody::GameStarted(v) => v,
        other => panic!("{other:?}"),
    }
}

/// The preview query: the engine's cost follows the sliders, matches the
/// catalogue at the defaults, and reads parameters as a draft would.
#[test]
fn the_policy_preview_cost_follows_the_parameters() {
    let mut s = Session::new().unwrap();
    new_game(&mut s, 42, None);
    let office = match send(&mut s, 2, RequestBody::Query(QueryDto::Office)).unwrap() {
        ResponseBody::Office(o) => o,
        other => panic!("{other:?}"),
    };
    let transit = office
        .catalog
        .iter()
        .find(|t| t.id == "policy.transit_frequency")
        .unwrap();
    let at_default = preview(&mut s, &transit.id, &[]);
    assert!((at_default.est_annual_cost - transit.est_cost_default).abs() < 1e-6);
    let low = preview(&mut s, &transit.id, &[("level", 1.0)]);
    let high = preview(&mut s, &transit.id, &[("level", 5.0)]);
    assert!(
        high.est_annual_cost > low.est_annual_cost * 4.0,
        "{low:?} {high:?}"
    );
    assert!(high.budget_share > low.budget_share);
    // Disruption grows with the level.
    assert!(high.side_effects[0].amount > low.side_effects[0].amount);
    // Out of range is clamped, off-step is snapped (as a draft reads it).
    let clamped = preview(&mut s, &transit.id, &[("level", 99.0)]);
    assert_eq!(clamped.params[0].value, 5.0);
    assert_eq!(clamped.est_annual_cost, high.est_annual_cost);
    let snapped = preview(&mut s, &transit.id, &[("level", 2.6)]);
    assert_eq!(snapped.params[0].value, 3.0);
    // Two-parameter templates: each slider moves the estimate it drives.
    let rent_lo = preview(&mut s, "policy.emergency_rent", &[("relief", 100.0)]);
    let rent_hi = preview(&mut s, "policy.emergency_rent", &[("relief", 1500.0)]);
    assert!(rent_hi.est_annual_cost > rent_lo.est_annual_cost);
    let freeze0 = preview(&mut s, "policy.emergency_rent", &[("freeze", 0.0)]);
    let freeze3 = preview(&mut s, "policy.emergency_rent", &[("freeze", 3.0)]);
    let grievance = |p: &PolicyPreviewView| {
        p.side_effects
            .iter()
            .find(|x| x.kind == "grievance" && x.group == "high")
            .unwrap()
            .amount
    };
    assert!(grievance(&freeze3) > grievance(&freeze0));
    // Unknown templates are an error, not a panic.
    assert!(
        send(
            &mut s,
            3,
            RequestBody::Query(QueryDto::PolicyPreview {
                template: "policy.nope".into(),
                params: vec![],
                proposal: None,
                choices: None,
            }),
        )
        .is_err()
    );
}

/// DEC-077 §3: a situational policy is offered and draftable only while its
/// situation holds; then the same command succeeds.
#[test]
fn situational_policies_unlock_with_the_situation() {
    let content = content();
    let t = &content.policies["policy.fiscal_bridge"];
    // A world whose town is calm once its live crises are cleared.
    let (mut e, m) = [42u64, 7, 11, 19, 23, 29]
        .into_iter()
        .map(|seed| {
            let mut e = world(seed);
            let m = e.state().player.as_ref().unwrap().municipality;
            e.setup(|s| s.politics.characters.towns[m.index()].crises.clear());
            (e, m)
        })
        .find(|(e, m)| !mandate_sim::policy::unlocked(e.state(), &content, t, m.0))
        .expect("a calm town");
    let draft = |e: &mut Engine| {
        e.apply(e.route(PlayerCommand::DraftPolicy {
            template: "policy.fiscal_bridge".into(),
            params: BTreeMap::new(),
        }))
    };
    // Situational policies are learned (DEC-091): the player has not heard
    // of this one, so their catalogue leaves it out. The lab's omniscient
    // view lists it, locked, as M5d part 2 does.
    let view = mandate_protocol::office_view(&e).unwrap();
    assert!(view.catalog.iter().all(|x| x.id != "policy.fiscal_bridge"));
    e.set_lab_omniscient(true);
    let view = mandate_protocol::office_view(&e).unwrap();
    let row = view
        .catalog
        .iter()
        .find(|x| x.id == "policy.fiscal_bridge")
        .unwrap();
    assert!(row.situational && !row.available);
    assert_eq!(row.unlock_reason_key.as_deref(), Some("unlock-fiscal"));
    assert!(
        draft(&mut e).is_err(),
        "a locked template cannot be drafted"
    );
    // Ordinary templates stay on the table.
    assert!(
        view.catalog
            .iter()
            .filter(|x| !x.situational)
            .all(|x| x.available)
    );
    e.setup(|s| {
        s.politics.characters.towns[m.index()]
            .crises
            .push(mandate_sim::characters::ActiveCrisis {
                kind: "crisis.fiscal_hole".into(),
                severity: 0.5,
                since: s.calendar.date,
                history: vec![0.5],
            });
    });
    let view = mandate_protocol::office_view(&e).unwrap();
    assert!(
        view.catalog
            .iter()
            .find(|x| x.id == "policy.fiscal_bridge")
            .unwrap()
            .available
    );
    assert!(draft(&mut e).is_ok(), "on the table in a fiscal hole");
}

/// A real plant closure in the event log opens the closure package, for a
/// year; a closure elsewhere does not.
#[test]
fn a_plant_closure_opens_the_closure_package() {
    let content = content();
    let mut e = world(7);
    let m = e.state().player.as_ref().unwrap().municipality;
    let t = &content.policies["policy.closure_response"];
    e.setup(|s| s.politics.characters.towns[m.index()].crises.clear());
    let job_loss = mandate_sim::concerns::indicator(
        &mandate_sim::scheduler::Reads::of_state(e.state(), &content),
        m.index(),
        mandate_content::Indicator::JobLoss,
    );
    if job_loss >= 0.03 {
        return; // already open through the indicator in this world
    }
    let closed_on = |e: &mut Engine, town: u32| {
        e.setup(|s| {
            let date = s.calendar.date;
            s.events.record(
                date,
                mandate_sim::events::EventKind::PlantClosure {
                    municipality: town,
                    sector: "sector.manufacturing".into(),
                    jobs: 300,
                    industry: None,
                    plant: None,
                    factor: None,
                    upstream: None,
                },
                mandate_sim::spine::Cause::Lab,
            );
        })
    };
    assert!(!mandate_sim::policy::unlocked(e.state(), &content, t, m.0));
    closed_on(&mut e, m.0 + 1);
    assert!(!mandate_sim::policy::unlocked(e.state(), &content, t, m.0));
    closed_on(&mut e, m.0);
    assert!(mandate_sim::policy::unlocked(e.state(), &content, t, m.0));
    // The overheard closure line names the industry and the jobs.
    let loc = overheard_spots(&e)[0];
    let cond = Condition::PlantClosure { within_months: 12 };
    assert!(mandate_sim::overheard::condition_holds(
        e.state(),
        &content,
        loc,
        &cond
    ));
}

/// DEC-033: the storyteller is chosen at new game, kept in the run (state and
/// save), and shown in the run metadata; unknown ids are refused.
#[test]
fn the_storyteller_is_chosen_at_new_game_and_saved() {
    let mut s = Session::new().unwrap();
    let list = match send(&mut s, 1, RequestBody::Query(QueryDto::Storytellers)).unwrap() {
        ResponseBody::Storytellers(l) => l,
        other => panic!("{other:?}"),
    };
    assert_eq!(list.storytellers.len(), 3);
    assert_eq!(list.storytellers.iter().filter(|x| x.default).count(), 1);
    let v = new_game(&mut s, 42, Some("storyteller.steady"));
    assert_eq!(v.storyteller, "storyteller.steady");
    assert_eq!(v.storyteller_name_key, "storyteller-steady");
    let save = s
        .export_save(&SaveRequestDto {
            run_id: "cd".repeat(16),
            revision: 1,
            parent_revision: None,
            device_id: "d".into(),
            saved_at: "2026-11-19T00:00:00Z".into(),
            run_mode: RunModeDto::Ironman,
        })
        .unwrap();
    let mut s2 = Session::new().unwrap();
    assert_eq!(
        s2.import_save(&save).unwrap().storyteller,
        "storyteller.steady"
    );
    // The default when none is chosen.
    let d = new_game(&mut s2, 42, None);
    assert_eq!(d.storyteller, "storyteller.classic");
    let bad = send(
        &mut s2,
        5,
        RequestBody::NewGame(NewGameParams {
            seed: 1,
            size: WorldSizeDto::Small,
            scenario: None,
            storyteller: Some("storyteller.nope".into()),
            local_knowledge: false,
            recent_openings: Vec::new(),
        }),
    );
    assert!(bad.is_err());
}

fn overheard_spots(e: &Engine) -> Vec<Location> {
    locations_for(e.state(), e.content(), Offer::Service("overheard"), None)
}

/// Lines heard across a town's overheard places over some weeks.
fn heard(
    e: &mut Engine,
    weeks: u32,
) -> Vec<(u32, Location, Vec<mandate_sim::overheard::HeardLine>)> {
    let mut out = Vec::new();
    for w in 0..weeks {
        for loc in overheard_spots(e) {
            out.push((
                w,
                loc,
                mandate_sim::overheard::lines(e.state(), e.content(), loc),
            ));
        }
        e.apply(Command::AdvanceDays { days: 7 }).unwrap();
    }
    out
}

/// Overheard talk varies by town, place, and week; it is keyed (the same
/// state hears the same lines), and it avoids last week's lines.
#[test]
fn overheard_talk_varies_by_town_and_week() {
    let mut a = world(42);
    let mut b = world(9001);
    let ha = heard(&mut a, 6);
    let hb = heard(&mut b, 6);
    let set = |h: &[(u32, Location, Vec<mandate_sim::overheard::HeardLine>)]| {
        h.iter()
            .flat_map(|(_, _, l)| l.iter().map(|x| x.id.clone()))
            .collect::<BTreeSet<_>>()
    };
    let (sa, sb) = (set(&ha), set(&hb));
    assert!(sa.len() >= 5, "town A heard only {sa:?}");
    assert!(sb.len() >= 5, "town B heard only {sb:?}");
    assert_ne!(sa, sb, "two towns heard exactly the same lines");
    // Rendered lines (key + arguments) differ too: names and facts.
    let rendered = |h: &[(u32, Location, Vec<mandate_sim::overheard::HeardLine>)]| {
        h.iter()
            .flat_map(|(_, _, l)| l.iter().map(|x| (x.key.clone(), x.args.clone())))
            .collect::<BTreeSet<_>>()
    };
    assert!(rendered(&ha).is_disjoint(&rendered(&hb)));
    // Deterministic: the same state hears the same lines.
    let again = world(42);
    let loc = overheard_spots(&again)[0];
    assert_eq!(
        mandate_sim::overheard::lines(again.state(), again.content(), loc),
        mandate_sim::overheard::lines(world(42).state(), again.content(), loc)
    );
    // Week to week, a place rarely repeats itself.
    let mut repeats = 0;
    let mut pairs = 0;
    for (w, loc, lines) in &ha {
        if let Some((_, _, prev)) = ha.iter().find(|(w2, l2, _)| *w2 + 1 == *w && l2 == loc) {
            pairs += 1;
            repeats += lines
                .iter()
                .filter(|x| prev.iter().any(|p| p.id == x.id))
                .count();
        }
    }
    assert!(pairs > 0);
    assert!(
        (repeats as f64) / (pairs as f64 * 3.0) < 0.34,
        "{repeats} repeats over {pairs} week pairs"
    );
}

/// A town without a river never hears a river line; a river town can.
#[test]
fn no_river_no_river_talk() {
    let river = Condition::Water {
        feature: WaterFeature::River,
    };
    let mut checked_dry = 0;
    let mut checked_wet = 0;
    for seed in [1u64, 2, 3, 5, 8, 13, 21, 34, 55, 89] {
        let mut e = world(seed);
        let m = e.state().player.as_ref().unwrap().municipality;
        let on_river = e.state().world.municipalities[m.index()].on_river;
        for loc in overheard_spots(&e) {
            assert_eq!(
                mandate_sim::overheard::condition_holds(e.state(), e.content(), loc, &river),
                on_river
            );
        }
        if on_river {
            checked_wet += 1;
            continue;
        }
        checked_dry += 1;
        // Even where the environment is the top worry, no river line holds.
        e.setup(|s| {
            let env = s
                .population
                .concern_keys
                .iter()
                .position(|k| k == "concern.environment");
            if let (Some(i), Some(tc)) = (env, s.population.town_concerns.get_mut(m.index())) {
                for (j, l) in tc.levels.iter_mut().enumerate() {
                    *l = if j == i { 1.0 } else { 0.0 };
                }
            }
            // Every neighbourhood feels it too (overheard reads them).
            let hoods: Vec<usize> = s.world.hoods_of(m).collect();
            if let Some(i) = env {
                for h in hoods {
                    if let Some(hc) = s.population.hood_concerns.get_mut(h) {
                        for (j, l) in hc.levels.iter_mut().enumerate() {
                            *l = if j == i { 1.0 } else { 0.0 };
                        }
                    }
                }
            }
        });
        for (_, _, lines) in heard(&mut e, 1) {
            assert!(
                lines
                    .iter()
                    .all(|l| !l.key.contains("river") && !l.key.contains("weir"))
            );
        }
        for loc in overheard_spots(&e) {
            let avail = mandate_sim::overheard::available(e.state(), e.content(), loc);
            assert!(
                avail.iter().any(|l| l.key.starts_with("heard-environment")),
                "a worried dry town still talks about the environment"
            );
            assert!(
                avail
                    .iter()
                    .all(|l| !l.key.contains("river") && !l.key.contains("weir"))
            );
        }
    }
    assert!(
        checked_dry > 0 && checked_wet > 0,
        "dry {checked_dry}, river {checked_wet}"
    );
}
