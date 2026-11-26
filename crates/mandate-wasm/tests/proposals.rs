//! Policy-family proposals in the protocol (DEC-092, task F3;
//! M6_ENGINE_CONTRACTS §2.12), with the sample families of
//! `mandate-content/tests/fixtures/families`:
//!
//! - knowledge gates the views (DEC-091): unknown proposals are left out, a
//!   heard one is a stub with its lead, a known one has the full menu, and
//!   support follows the `policy_support` subject;
//! - every option's cost, support and impact are the engine's, for the
//!   package with that option swapped in (at most one composition each),
//!   and the cost shown is the cost the drafted bill carries (shown = done);
//! - who may draft comes from the constitution (DEC-098), and the view's
//!   `can_draft` agrees with the `DraftFamily` command;
//! - views never write; the commands and queries round-trip as JSON.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use mandate_content::{ContentBundle, Source};
use mandate_protocol::*;
use mandate_sim::actions::PlayerCommand;
use mandate_sim::places::Location;
use mandate_sim::player::knowledge::{Level, PolicyRef, Source as Learned, Subject};
use mandate_sim::policy::families::{Bound, FamilyInstanceId};
use mandate_sim::world::PlaceKind;
use mandate_sim::{Command, Engine};
use mandate_wasm::session::Session;
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::collections::BTreeMap;
use std::sync::Arc;

const FAMILIES: &str =
    include_str!("../../mandate-content/tests/fixtures/families/test_families.jsonc");
const FTL: &str = include_str!("../../mandate-content/tests/fixtures/families/test_families.ftl");
const RESCUE: &str = "family.test_plant_rescue";
const WALL: &str = "family.test_flood_wall";

fn load(families: &str) -> Arc<ContentBundle> {
    let mut src = mandate_content::embedded::sources();
    src.push(Source {
        path: "zz_test/policy_families/test.jsonc".into(),
        text: families.into(),
    });
    src.push(Source {
        path: "zz_test/locale/en/test.ftl".into(),
        text: FTL.into(),
    });
    Arc::new(
        mandate_content::load(&src)
            .unwrap_or_else(|e| panic!("{e:#?}"))
            .0,
    )
}

fn content() -> Arc<ContentBundle> {
    load(FAMILIES)
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

/// The first seed (from `from`) whose home town opens `family` within two
/// months, with the proposal.
fn game_with(c: &Arc<ContentBundle>, family: &str, from: u64) -> (u64, Engine, FamilyInstanceId) {
    for seed in from..from + 40 {
        let mut e = game(c, seed);
        e.apply(Command::AdvanceDays { days: 62 }).unwrap();
        let found = e
            .state()
            .policy
            .families
            .instances
            .iter()
            .find(|i| i.is_open() && i.family == family)
            .map(|i| i.id);
        if let Some(id) = found {
            return (seed, e, id);
        }
    }
    panic!("no seed opens {family}");
}

/// Put a subject at exactly `level`, and nothing known by the lab switch.
fn set(e: &mut Engine, s: Subject, level: Level) {
    e.setup(|st| {
        let on = st.calendar.date;
        let k = &mut st.player.as_mut().unwrap().knowledge;
        k.lab_omniscient = false;
        k.entries.remove(&s);
        if level > Level::Unknown {
            k.learn(s, level, on, Learned::Lab);
        }
    });
}

fn proposal_subject(id: FamilyInstanceId) -> Subject {
    Subject::Proposal { proposal: id }
}

fn support_subject(e: &Engine, family: &str) -> Subject {
    Subject::PolicySupport {
        town: e.state().player.as_ref().unwrap().home(),
        policy: PolicyRef::Template {
            template: family.into(),
        },
    }
}

fn at_town_hall(e: &mut Engine) {
    e.setup(|s| {
        let town = s.player.as_ref().unwrap().home();
        let hall = s
            .world
            .places
            .iter()
            .find(|p| p.municipality == town && p.kind == PlaceKind::TownHall)
            .map(|p| p.id)
            .unwrap();
        s.player.as_mut().unwrap().location = Location::Place { place: hall };
    });
}

fn preview(
    e: &Engine,
    id: FamilyInstanceId,
    choices: &[(&str, &str)],
) -> Option<PolicyPreviewView> {
    let choices: Vec<ChoiceValue> = choices
        .iter()
        .map(|(a, o)| ChoiceValue {
            axis: (*a).into(),
            option: (*o).into(),
        })
        .collect();
    policy_preview_of(e, "", &[], Some(id.0), &choices)
}

fn office_proposal(e: &Engine, id: FamilyInstanceId) -> Option<ProposalView> {
    office_view(e)
        .unwrap()
        .proposals
        .into_iter()
        .find(|p| p.id == id.0)
}

fn option<'a>(v: &'a PolicyPreviewView, axis: &str, opt: &str) -> &'a OptionPreviewView {
    v.family
        .as_ref()
        .unwrap()
        .axes
        .iter()
        .find(|a| a.id == axis)
        .and_then(|a| a.options.iter().find(|o| o.id == opt))
        .unwrap_or_else(|| panic!("no option {axis}.{opt}"))
}

#[test]
fn knowledge_gates_the_proposal_and_its_sheet() {
    let c = content();
    let (_, mut e, id) = game_with(&c, RESCUE, 1);

    // Unknown: on no desk, at no place, and no sheet.
    set(&mut e, proposal_subject(id), Level::Unknown);
    assert!(office_proposal(&e, id).is_none());
    assert!(preview(&e, id, &[]).is_none());

    // Heard: a stub with the reason and the lead, no menu, no figures.
    set(&mut e, proposal_subject(id), Level::Heard);
    let p = office_proposal(&e, id).expect("a heard proposal is listed");
    assert_eq!(p.level, KnowledgeLevelDto::Heard);
    assert_eq!(p.family, RESCUE);
    assert!(p.reason_key.is_some());
    assert!(p.bindings.is_empty());
    assert!(p.est_annual_cost.is_none());
    assert!(!p.can_draft);
    assert_eq!(p.blocked_key.as_deref(), Some("proposal-blocked-heard"));
    assert!(
        p.args.get("plant").is_some_and(|n| !n.is_empty()) || p.args.contains_key("plant_key"),
        "the title names the plant"
    );
    let v = preview(&e, id, &[]).expect("a heard proposal has a stub sheet");
    let fam = v.family.as_ref().unwrap();
    assert!(fam.axes.is_empty() && fam.sponsors.is_empty() && fam.param_defs.is_empty());
    assert!(v.impact.is_none());
    assert!(!v.available);
    assert_eq!(v.est_annual_cost, 0.0);
    assert!(v.support.overall.is_none());

    // Known: the menu, every option costed, the sponsors' terms. Support
    // is still a word until the player learns who backs it.
    set(&mut e, proposal_subject(id), Level::Known);
    let s = support_subject(&e, RESCUE);
    set(&mut e, s, Level::Unknown);
    let p = office_proposal(&e, id).unwrap();
    assert_eq!(p.level, KnowledgeLevelDto::Known);
    assert!(
        p.bindings
            .iter()
            .any(|b| b.slot == "plant" && b.location.is_some())
    );
    assert!(p.est_annual_cost.unwrap() > 0.0);
    let v = preview(&e, id, &[]).unwrap();
    let fam = v.family.as_ref().unwrap();
    assert_eq!(fam.axes.len(), 2);
    assert!(!fam.param_defs.is_empty());
    for a in &fam.axes {
        assert_eq!(a.options.iter().filter(|o| o.chosen).count(), 1);
        for o in &a.options {
            assert!(o.available, "{}.{}", a.id, o.id);
            let s = o.support.as_ref().unwrap();
            assert!(s.overall.is_none() && s.low.is_none(), "support is a word");
        }
    }
    assert!(v.support.overall.is_none());
    assert!(!fam.sponsors.is_empty());
    for s in &fam.sponsors {
        assert!(!s.preset.is_empty());
        assert!(!s.price.key.is_empty());
    }

    // Who backs it, in detail: exact support by class for every option.
    let s = support_subject(&e, RESCUE);
    set(&mut e, s, Level::Detailed);
    let v = preview(&e, id, &[]).unwrap();
    assert_eq!(v.support.level, KnowledgeLevelDto::Detailed);
    assert!(v.support.overall.is_some());
    for a in &v.family.as_ref().unwrap().axes {
        for o in &a.options {
            assert!(o.support.as_ref().unwrap().low.is_some());
        }
    }
}

#[test]
fn each_option_is_the_package_with_it_swapped_in() {
    let c = content();
    let (_, mut e, id) = game_with(&c, RESCUE, 1);
    set(&mut e, proposal_subject(id), Level::Known);
    let v = preview(&e, id, &[]).unwrap();
    // The chosen option of each axis is the package itself.
    let chosen = option(&v, "terms", "no_strings");
    assert!(chosen.chosen && chosen.is_default);
    assert_eq!(chosen.est_annual_cost, v.est_annual_cost);
    assert_eq!(chosen.impact, v.impact);
    // Clawback pays three quarters of the wage aid.
    let claw = option(&v, "terms", "clawback");
    assert!(
        (claw.est_annual_cost - 0.75 * v.est_annual_cost).abs() < 1e-6 * v.est_annual_cost.max(1.0)
    );
    // Each option is the package with only it changed: the same as a
    // preview with that option chosen.
    for (axis, opt) in [("terms", "job_guarantee"), ("pay", "levy"), ("pay", "bond")] {
        let swapped = preview(&e, id, &[(axis, opt)]).unwrap();
        let o = option(&v, axis, opt);
        assert_eq!(o.est_annual_cost, swapped.est_annual_cost, "{axis}.{opt}");
        assert_eq!(o.side_effects, swapped.side_effects, "{axis}.{opt}");
        assert_eq!(o.impact, swapped.impact, "{axis}.{opt}");
        assert_eq!(o.support.as_ref(), Some(&swapped.support), "{axis}.{opt}");
    }
    // The impact estimate covers the local tier: every option says what it
    // would do, and a bond is paid by borrowing.
    for a in &v.family.as_ref().unwrap().axes {
        for o in &a.options {
            assert!(o.impact.is_some(), "{}.{}", a.id, o.id);
        }
    }
    let bond = option(&v, "pay", "bond").impact.as_ref().unwrap();
    assert!(bond.cost.borrowed_pct > 0);
    // Deterministic, and no query writes.
    let before = e.state_hash();
    assert_eq!(preview(&e, id, &[]), Some(v));
    let _ = office_view(&e);
    assert_eq!(e.state_hash(), before);
}

#[test]
fn the_cost_shown_is_the_cost_the_bill_carries() {
    let c = content();
    let (_, mut e, id) = game_with(&c, RESCUE, 1);
    set(&mut e, proposal_subject(id), Level::Known);
    at_town_hall(&mut e);
    let v = preview(&e, id, &[("terms", "clawback")]).unwrap();
    let p = office_proposal(&e, id).unwrap();
    assert!(p.can_draft, "{:?}", p.blocked_key);
    assert!(!p.proposers.is_empty());
    let out = e
        .apply(Command::Player(PlayerCommand::DraftFamily {
            proposal: id.0,
            choices: BTreeMap::from([("terms".to_string(), "clawback".to_string())]),
            params: BTreeMap::new(),
            sponsor: None,
        }))
        .unwrap();
    let bill = out.result.unwrap().created.unwrap();
    let office = office_view(&e).unwrap();
    let b = office.bills.iter().find(|b| b.id == bill).unwrap();
    assert_eq!(b.est_annual_cost, v.est_annual_cost);
    let fam = b
        .family
        .as_ref()
        .expect("a family bill says what it was drafted with");
    assert_eq!(fam.proposal, id.0);
    assert!(
        fam.choices
            .iter()
            .any(|c| c.axis == "terms" && c.option == "clawback")
    );
    assert!(fam.args.contains_key("plant") || fam.args.contains_key("plant_key"));
    // The proposal lists the bill drafted from it.
    assert!(office_proposal(&e, id).unwrap().bills.contains(&bill));
}

#[test]
fn who_may_draft_comes_from_the_constitution() {
    let c = content();
    let (_, mut e, id) = game_with(&c, RESCUE, 1);
    set(&mut e, proposal_subject(id), Level::Known);
    let p = office_proposal(&e, id).unwrap();
    assert_eq!(p.requires, "propose_ordinance");
    // The posts listed are exactly the constitution's posts of the
    // proposal's tier holding the power.
    let cons = c.constitutions.get(&e.state().meta.constitution).unwrap();
    let tier = cons.tier(mandate_content::TierKey::Local).unwrap();
    let mut want: Vec<String> = tier
        .bodies
        .iter()
        .filter(|b| b.powers.contains(&mandate_content::Power::ProposeOrdinance))
        .map(|b| b.id.clone())
        .chain(
            tier.offices
                .iter()
                .filter(|o| o.powers.contains(&mandate_content::Power::ProposeOrdinance))
                .map(|o| o.id.clone()),
        )
        .collect();
    let mut got: Vec<String> = p.proposers.iter().map(|x| x.id.clone()).collect();
    want.sort();
    got.sort();
    assert_eq!(got, want);
    assert!(p.proposers.iter().all(|x| x.at == p.at));
    // Out of office the player cannot draft it, and the command agrees.
    e.setup(|s| {
        let me = s.player.as_ref().unwrap().politician;
        let home = s.player.as_ref().unwrap().home();
        if let Some(pol) = s.politics.politicians.iter_mut().find(|x| x.id == me) {
            pol.role = mandate_sim::politics::Role::Private { home };
        }
    });
    at_town_hall(&mut e);
    let p = office_proposal(&e, id).unwrap();
    assert!(!p.can_draft);
    assert_eq!(p.blocked_key.as_deref(), Some("proposal-blocked-no-power"));
    let r = e.apply(Command::Player(PlayerCommand::DraftFamily {
        proposal: id.0,
        choices: BTreeMap::new(),
        params: BTreeMap::new(),
        sponsor: None,
    }));
    assert!(r.is_err());
}

#[test]
fn an_option_that_cannot_go_with_another_says_so() {
    // The landowners cannot build the whole riverfront (a test-only rule).
    let families = FAMILIES.replace(
        r#""cost_factor": 0.4, "effect_factor": 0.7, "months": 6,"#,
        r#""cost_factor": 0.4, "effect_factor": 0.7, "months": 6, "excludes": ["where.riverfront"],"#,
    );
    assert_ne!(families, FAMILIES);
    let c = load(&families);
    let (_, mut e, id) = game_with(&c, WALL, 1);
    set(&mut e, proposal_subject(id), Level::Known);
    let v = preview(&e, id, &[("where", "riverfront")]).unwrap();
    let land = option(&v, "builder", "landowners");
    assert!(!land.available);
    assert_eq!(
        land.unavailable_key.as_deref(),
        Some("option-unavailable-excludes")
    );
    assert_eq!(
        land.unavailable_args.get("with_key").map(String::as_str),
        Some("opt-test-wall-riverfront")
    );
    assert!(land.support.is_none() && land.impact.is_none());
    // The package as chosen is fine; choosing both is not.
    assert!(v.available);
    let both = preview(
        &e,
        id,
        &[("where", "riverfront"), ("builder", "landowners")],
    )
    .unwrap();
    assert!(!both.available);
    assert!(both.family.as_ref().unwrap().unavailable_key.is_some());
}

#[test]
fn proposals_are_listed_where_they_are_about() {
    let c = content();
    let (_, mut e, id) = game_with(&c, RESCUE, 1);
    set(&mut e, proposal_subject(id), Level::Heard);
    let plant = e
        .state()
        .policy
        .families
        .get(id)
        .and_then(|i| i.place("plant"))
        .unwrap();
    let panel = place_panel_view(&e, LocationDto::of(Location::Place { place: plant })).unwrap();
    assert!(panel.proposals_here.iter().any(|p| p.id == id.0));
    // Somewhere it is not about, it is not listed.
    let elsewhere = e
        .state()
        .world
        .places
        .iter()
        .filter(|p| p.municipality == e.state().player.as_ref().unwrap().home())
        .find(|p| {
            p.id != plant
                && !e
                    .state()
                    .policy
                    .families
                    .get(id)
                    .unwrap()
                    .sponsors
                    .iter()
                    .any(|s| {
                        mandate_sim::characters::routines::workplace(e.state(), &c, s.person)
                            == Some(Location::Place { place: p.id })
                    })
        })
        .map(|p| p.id)
        .unwrap();
    let panel =
        place_panel_view(&e, LocationDto::of(Location::Place { place: elsewhere })).unwrap();
    assert!(!panel.proposals_here.iter().any(|p| p.id == id.0));
    // Unknown, it is listed nowhere.
    set(&mut e, proposal_subject(id), Level::Unknown);
    let panel = place_panel_view(&e, LocationDto::of(Location::Place { place: plant })).unwrap();
    assert!(panel.proposals_here.is_empty());
    // A bound neighbourhood's streets list a flood wall.
    let (_, mut e, wall) = game_with(&c, WALL, 1);
    set(&mut e, proposal_subject(wall), Level::Heard);
    let hood = match e.state().policy.families.get(wall).unwrap().bound("hood") {
        Some(Bound::Neighborhood { hood }) => *hood,
        other => panic!("{other:?}"),
    };
    let streets = Location::Street { neighborhood: hood };
    let panel = place_panel_view(&e, LocationDto::of(streets)).unwrap();
    assert!(panel.proposals_here.iter().any(|p| p.id == wall.0));
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
fn the_sheet_and_the_draft_round_trip_as_json() {
    // The query and the commands F6 sends, as the client writes them.
    let q = QueryDto::PolicyPreview {
        template: RESCUE.into(),
        params: vec![],
        proposal: Some(3),
        choices: Some(vec![ChoiceValue {
            axis: "terms".into(),
            option: "clawback".into(),
        }]),
    };
    let j = serde_json::to_string(&q).unwrap();
    assert_eq!(serde_json::from_str::<QueryDto>(&j).unwrap(), q);
    // An old client's preview (no proposal, no choices) still parses.
    let old: QueryDto =
        serde_json::from_str(r#"{"type":"policy_preview","template":"policy.x","params":[]}"#)
            .unwrap();
    assert!(matches!(
        old,
        QueryDto::PolicyPreview {
            proposal: None,
            choices: None,
            ..
        }
    ));
    let amend: CommandDto = serde_json::from_str(
        r#"{"type":"amend_policy","policy":1,"params":[],"clauses":[],"choices":[{"axis":"pay","option":"bond"}]}"#,
    )
    .unwrap();
    assert!(matches!(amend, CommandDto::AmendPolicy { choices: Some(ref c), .. } if c.len() == 1));
    // Through a session with the shipped pack (no families yet): a preview
    // of a proposal that does not exist and a draft of one are refused
    // cleanly.
    let mut s = Session::new().unwrap();
    let start = RequestBody::NewGame(NewGameParams {
        seed: 2026,
        size: WorldSizeDto::Small,
        scenario: None,
        storyteller: None,
        local_knowledge: false,
        recent_openings: Vec::new(),
    });
    let r: Response = serde_json::from_str(&s.handle_json(&req(1, start))).unwrap();
    assert!(matches!(r.result, ResponseResult::Ok { .. }));
    let r: Response = serde_json::from_str(&s.handle_json(&req(2, RequestBody::Query(q)))).unwrap();
    assert!(matches!(r.result, ResponseResult::Err { .. }));
    let draft = CommandDto::DraftFamily {
        proposal: 3,
        choices: vec![],
        params: vec![],
        sponsor: None,
    };
    let r: Response =
        serde_json::from_str(&s.handle_json(&req(3, RequestBody::Command(draft)))).unwrap();
    assert!(matches!(r.result, ResponseResult::Err { .. }));
    // The desk lists no proposals without families.
    let r: Response =
        serde_json::from_str(&s.handle_json(&req(4, RequestBody::Query(QueryDto::Office))))
            .unwrap();
    match r.result {
        ResponseResult::Ok {
            body: ResponseBody::Office(o),
        } => assert!(o.proposals.is_empty()),
        other => panic!("{other:?}"),
    }
}

/// The report's worked example (`MANDATE_PRINT=1 cargo test -p mandate-wasm
/// --test proposals worked_example -- --nocapture`): a plant rescue as a
/// player who knows little and as one who knows a lot.
#[test]
fn worked_example() {
    let c = content();
    let (seed, mut e, id) = game_with(&c, RESCUE, 1);
    let print = std::env::var("MANDATE_PRINT").is_ok();
    let line = |label: &str, v: &PolicyPreviewView| {
        if !print {
            return;
        }
        let fam = v.family.as_ref().unwrap();
        let p = &fam.proposal;
        println!(
            "== {label}: seed {seed}, proposal {} {} {:?} at {} ({}), level {:?}, can_draft {} {:?}",
            p.id, p.family, p.args, p.at.name, p.at.tier, p.level, p.can_draft, p.blocked_key
        );
        println!(
            "   package {:?}: cost {:.0} ({:.2}% of budget), support {:?} {:?}, months {}",
            fam.choices
                .iter()
                .map(|c| format!("{}={}", c.axis, c.option))
                .collect::<Vec<_>>(),
            v.est_annual_cost,
            v.budget_share * 100.0,
            v.support.band,
            v.support.overall,
            fam.months
        );
        for a in &fam.axes {
            for o in &a.options {
                let lead: Vec<String> = o
                    .impact
                    .iter()
                    .flat_map(|i| i.lines.iter().filter(|l| l.lead))
                    .map(|l| {
                        format!(
                            "{} {} {:?}-{:?} {}",
                            l.outcome, l.direction, l.lo, l.hi, l.unit
                        )
                    })
                    .collect();
                println!(
                    "   {}.{}{}: cost {:.0}, support {} {:?} (low {:?}), borrowed {:?}%, lead {:?}",
                    a.id,
                    o.id,
                    if o.chosen { "*" } else { "" },
                    o.est_annual_cost,
                    o.support.as_ref().map(|s| s.band.as_str()).unwrap_or("-"),
                    o.support.as_ref().and_then(|s| s.overall),
                    o.support.as_ref().and_then(|s| s.low),
                    o.impact.as_ref().map(|i| i.cost.borrowed_pct),
                    lead
                );
            }
        }
        for s in &fam.sponsors {
            println!(
                "   sponsor {} ({:?}, {:?}): {:?} price {} {:?} matches {}",
                s.id,
                s.person.name,
                s.person.role_key,
                s.preset
                    .iter()
                    .map(|c| format!("{}={}", c.axis, c.option))
                    .collect::<Vec<_>>(),
                s.price.key,
                s.price.args,
                s.matches_current
            );
        }
    };
    set(&mut e, proposal_subject(id), Level::Heard);
    let little = preview(&e, id, &[]).unwrap();
    line("knows little (heard)", &little);
    set(&mut e, proposal_subject(id), Level::Known);
    let s = support_subject(&e, RESCUE);
    set(&mut e, s, Level::Unknown);
    let some = preview(&e, id, &[]).unwrap();
    line("knows the proposal (known), not who backs it", &some);
    set(&mut e, proposal_subject(id), Level::Detailed);
    let s = support_subject(&e, RESCUE);
    set(&mut e, s, Level::Detailed);
    let lot = preview(&e, id, &[]).unwrap();
    line("knows a lot (detailed, support detailed)", &lot);
    // Knowing more never changes a figure, only how much of it is shown.
    assert_eq!(some.est_annual_cost, lot.est_annual_cost);
    assert_eq!(some.impact, lot.impact);
    assert_eq!(some.support.band, lot.support.band);
    assert_eq!(little.support.band, lot.support.band);
}
