//! The views the web's proposal sheet (DEC-092, task F6) is built against,
//! as the engine made them from the base pack: the desk's proposals (one
//! known, and a stub where a second is open), the preview of a known
//! proposal at its defaults, with another option chosen, and with a
//! sponsor's version, the stub's preview, and the bill drafted from it.
//! Written with `MANDATE_WRITE_FIXTURES=1 cargo test -p mandate-wasm --test
//! proposal_sheet`; otherwise the files on disk must still read as today's
//! views (a protocol change that leaves them behind fails here).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use mandate_protocol::*;
use mandate_sim::actions::PlayerCommand;
use mandate_sim::places::Location;
use mandate_sim::player::knowledge::{Level, Source as Learned, Subject};
use mandate_sim::policy::families::FamilyInstanceId;
use mandate_sim::world::PlaceKind;
use mandate_sim::{Command, Engine};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../web/src/components/fixtures/proposal")
}

/// Put a proposal at exactly `level` (the lab switch off).
fn teach(e: &mut Engine, id: FamilyInstanceId, level: Level) {
    e.setup(|st| {
        let on = st.calendar.date;
        let k = &mut st.player.as_mut().unwrap().knowledge;
        k.lab_omniscient = false;
        let s = Subject::Proposal { proposal: id };
        k.entries.remove(&s);
        if level > Level::Unknown {
            k.learn(s, level, on, Learned::Lab);
        }
    });
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
    choices: Vec<ChoiceValue>,
    params: &[ParamValue],
) -> PolicyPreviewView {
    policy_preview_of(e, "", params, Some(id.0), &choices).expect("a preview")
}

/// Open proposals of the player's focus, in the order they opened.
fn open_here(e: &Engine) -> Vec<FamilyInstanceId> {
    let st = e.state();
    let focus = st.focus();
    st.policy
        .families
        .instances
        .iter()
        .filter(|i| i.is_open() && focus.contains(&i.at))
        .map(|i| i.id)
        .collect()
}

/// A small world whose home town has, within two months, an open proposal
/// with sponsors, two axes or more, sliders and options that cost
/// differently: the first seed from 1 with all of it.
fn world() -> (u64, Engine, FamilyInstanceId) {
    let c = Arc::new(
        mandate_content::load(&mandate_content::embedded::sources())
            .unwrap()
            .0,
    );
    for seed in 1..80u64 {
        let mut e = generate(&WorldConfig::new(seed, WorldSize::Small), c.clone()).unwrap();
        e.apply(Command::AdvanceDays { days: 62 }).unwrap();
        for id in open_here(&e) {
            teach(&mut e, id, Level::Known);
            let v = preview(&e, id, Vec::new(), &[]);
            let fam = v.family.as_ref().unwrap();
            let varied = fam.axes.iter().any(|a| {
                a.options
                    .iter()
                    .any(|o| o.est_annual_cost != v.est_annual_cost)
            });
            if fam.axes.len() >= 2
                && !fam.sponsors.is_empty()
                && !fam.param_defs.is_empty()
                && varied
            {
                return (seed, e, id);
            }
            teach(&mut e, id, Level::Unknown);
        }
    }
    panic!("no small world opens a proposal with all of it in two months");
}

#[test]
fn fixtures_are_views_the_engine_made() {
    let (seed, mut e, id) = world();
    at_town_hall(&mut e);
    // A second open proposal, only heard of: the desk's stub.
    let stub = open_here(&e).into_iter().find(|x| *x != id);
    if let Some(s) = stub {
        teach(&mut e, s, Level::Heard);
    }
    let default = preview(&e, id, Vec::new(), &[]);
    let fam = default.family.clone().unwrap();
    // Another option of the first axis whose cost differs.
    let (axis, other) = fam
        .axes
        .iter()
        .find_map(|a| {
            a.options
                .iter()
                .find(|o| !o.chosen && o.available && o.est_annual_cost != default.est_annual_cost)
                .map(|o| (a.id.clone(), o.id.clone()))
        })
        .unwrap();
    // Drafted with the other option: the bill on the desk.
    let out = e
        .apply(Command::Player(PlayerCommand::DraftFamily {
            proposal: id.0,
            choices: BTreeMap::from([(axis.clone(), other.clone())]),
            params: BTreeMap::new(),
            sponsor: None,
        }))
        .unwrap();
    let bill_id = out.result.unwrap().created.unwrap();
    // The sheet after the draft: the proposal lists its bill.
    let default = preview(&e, id, Vec::new(), &[]);
    let fam = default.family.clone().unwrap();
    let swapped = preview(
        &e,
        id,
        vec![ChoiceValue {
            axis: axis.clone(),
            option: other.clone(),
        }],
        &[],
    );
    let sponsor = fam.sponsors[0].clone();
    let adopted = preview(&e, id, sponsor.preset.clone(), &sponsor.params);
    let heard = stub.map(|s| preview(&e, s, Vec::new(), &[]));
    let office = office_view(&e).unwrap();
    let bill = office
        .bills
        .iter()
        .find(|b| b.id == bill_id)
        .cloned()
        .unwrap();
    let proposals = office.proposals.clone();

    let mut files: Vec<(&str, serde_json::Value)> = vec![
        ("proposals.json", serde_json::to_value(&proposals).unwrap()),
        ("preview.json", serde_json::to_value(&default).unwrap()),
        (
            "preview-option.json",
            serde_json::to_value(&swapped).unwrap(),
        ),
        (
            "preview-sponsor.json",
            serde_json::to_value(&adopted).unwrap(),
        ),
        ("bill.json", serde_json::to_value(&bill).unwrap()),
    ];
    if let Some(h) = &heard {
        files.push(("preview-heard.json", serde_json::to_value(h).unwrap()));
    }
    let dir = fixture_dir();
    if std::env::var_os("MANDATE_WRITE_FIXTURES").is_some() {
        std::fs::create_dir_all(&dir).unwrap();
        for (name, v) in &files {
            let mut text = serde_json::to_string_pretty(v).unwrap();
            text.push('\n');
            std::fs::write(dir.join(name), text).unwrap();
        }
        eprintln!(
            "seed {seed}: proposal {} ({}), stub {stub:?}",
            id.0, fam.proposal.family
        );
    }
    let read = |name: &str| -> String {
        std::fs::read_to_string(dir.join(name)).unwrap_or_else(|_| {
            panic!("{name} is missing: MANDATE_WRITE_FIXTURES=1 cargo test -p mandate-wasm --test proposal_sheet")
        })
    };
    let stale = "the fixture no longer reads as the view: write the fixtures again";
    let proposals: Vec<ProposalView> = serde_json::from_str(&read("proposals.json")).expect(stale);
    let default: PolicyPreviewView = serde_json::from_str(&read("preview.json")).expect(stale);
    let swapped: PolicyPreviewView =
        serde_json::from_str(&read("preview-option.json")).expect(stale);
    let adopted: PolicyPreviewView =
        serde_json::from_str(&read("preview-sponsor.json")).expect(stale);
    let bill: BillView = serde_json::from_str(&read("bill.json")).expect(stale);
    // What the sheet needs each to show.
    let known = proposals
        .iter()
        .find(|p| p.level == KnowledgeLevelDto::Known)
        .expect("a known proposal");
    assert!(known.can_draft && known.est_annual_cost.is_some() && !known.sponsors.is_empty());
    let fam = default.family.as_ref().unwrap();
    assert!(fam.axes.len() >= 2 && !fam.param_defs.is_empty() && !fam.sponsors.is_empty());
    assert!(fam.sponsors.iter().all(|s| !s.matches_current) || fam.sponsors.len() > 1);
    assert_ne!(swapped.est_annual_cost, default.est_annual_cost);
    assert!(
        adopted
            .family
            .as_ref()
            .unwrap()
            .sponsors
            .iter()
            .any(|s| s.matches_current)
    );
    assert_eq!(bill.family.as_ref().map(|f| f.proposal), Some(known.id));
    assert_eq!(
        bill.est_annual_cost, swapped.est_annual_cost,
        "shown = done"
    );
    assert!(
        fam.proposal.bills.contains(&bill.id),
        "the sheet lists the bill drafted from it"
    );
    if dir.join("preview-heard.json").exists() {
        let h: PolicyPreviewView = serde_json::from_str(&read("preview-heard.json")).expect(stale);
        let hf = h.family.as_ref().unwrap();
        assert!(hf.axes.is_empty() && hf.proposal.level == KnowledgeLevelDto::Heard);
    }
}
