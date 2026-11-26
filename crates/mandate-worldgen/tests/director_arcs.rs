#![allow(clippy::unwrap_used, clippy::expect_used, clippy::print_stdout)]
//! Where arcs meet (DEC-109 §4.2, STAGED_ARCS §3.1): the director's real
//! interaction between arcs, kept here and nowhere else. Every arc's own
//! tests use worlds that hold that arc alone (`mandate-testkit`'s `only`),
//! so adding an arc to the pack does not reach them; what does reach this
//! file is what arcs do to each other:
//!
//! - the places for arcs (`director.arcs_active`) go by rank (the arc
//!   template's `rank`): a crisis is never kept out by matters raised
//!   before it, and a matter is;
//! - a crisis runs beside a standing arc and leads the chapter over it;
//! - one scene a chapter.

use mandate_content::ContentBundle;
use mandate_content::arcs::ArcRank;
use mandate_sim::Engine;
use mandate_sim::world::Jurisdiction;
use mandate_testkit::{self as kit, end_turn, plant_flood, run_of, summary_of, world};
use std::sync::Arc;

const FLOOD: &str = "arc.flood";
const ZONING: &str = "arc.zoning";
const FAVOUR: &str = "arc.favour_debt";
const ROUND: &str = "arc.budget_round";

/// A world whose home town has housing as its strongest worry from the
/// start: the zoning fight is raised in the first turns (the zoning fight's
/// own seed, `arcs_zoning.rs`).
const ZONING_SEED: u64 = 2;

/// The player owes a favour to a member of the town's body: the favour arc
/// (a matter) is raised.
fn owe_a_favour(e: &mut Engine) {
    let town = kit::home(e);
    let date = e.state().calendar.date;
    let me = kit::me(e);
    let creditor = e
        .state()
        .politics
        .politicians
        .iter()
        .find(|p| !p.is_player && p.municipality() == town.0 && p.role.post().is_some())
        .map(|p| p.character)
        .unwrap();
    e.setup(|st| {
        mandate_sim::characters::owe(&mut st.politics, me, creditor, "favour-lab", None, date);
    });
}

/// Live runs at the player's home that hold a place (standing arcs aside).
fn holding(e: &Engine) -> Vec<String> {
    let at: Jurisdiction = kit::town(e);
    e.state()
        .arcs
        .at(at)
        .filter(|r| {
            e.content()
                .arc(&r.template)
                .is_some_and(|d| d.standing.is_none())
        })
        .map(|r| r.template.clone())
        .collect()
}

/// Two matters hold both places; then a flood strikes. Returns whether a
/// flood run was raised (live or ended) and the places held as it came.
fn crowded_flood(c: &Arc<ContentBundle>) -> (bool, Vec<String>) {
    let mut e = world(c, ZONING_SEED);
    kit::raised(&mut e, ZONING);
    owe_a_favour(&mut e);
    kit::raised(&mut e, FAVOUR);
    let full = holding(&e);
    assert_eq!(
        full.len(),
        usize::from(c.director.arcs_active),
        "both places held by matters: {full:?}"
    );
    plant_flood(&mut e, 0.9);
    for _ in 0..6 {
        if run_of(&e, FLOOD).is_some() || summary_of(&e, FLOOD).is_some() {
            return (true, full);
        }
        end_turn(&mut e);
    }
    (false, full)
}

#[test]
fn a_crisis_is_never_kept_out_of_a_place_by_matters() {
    let c = kit::only(&[ZONING, FAVOUR, FLOOD]);
    assert_eq!(c.arc(FLOOD).unwrap().rank, ArcRank::Crisis);
    assert_eq!(c.arc(ZONING).unwrap().rank, ArcRank::Matter);
    let (raised, full) = crowded_flood(&c);
    assert!(raised, "the flood was kept out by {full:?}");

    // The same world with the flood ranked as a matter: the places are
    // full, and it waits (its owning system resolves the water alone).
    let mut b = (*c).clone();
    b.arcs.iter_mut().find(|a| a.id == FLOOD).unwrap().rank = ArcRank::Matter;
    let (raised, _) = crowded_flood(&Arc::new(b));
    assert!(!raised, "a matter is held back when the places are full");
}

/// A crisis runs beside the standing arc, and outranks it for the
/// chapter's lead while it can still turn: the flood's meeting is the
/// chapter's climax, though the budget round turns in the same chapter.
/// The round is raised all the same (a standing arc is outside the cap),
/// and each ends by its own rules. (Moved from `budget_round.rs`.)
#[test]
fn a_flood_runs_beside_the_round_and_leads_the_chapter() {
    let c = kit::only(&[ROUND, FLOOD]);
    let flood_def = c.arc(FLOOD).unwrap();
    // A world where the flood's turn comes before the round's (seed 7 before
    // industries, 99 after: one of eleven tried; 106 since the power broker
    // leans on votes, M6 note 27, which puts flood defences in force in
    // seed 99's town before the flood, so its arc has no meeting).
    let mut e = world(&c, 106);
    plant_flood(&mut e, 0.9);
    let mut together = false;
    let mut flood_led = false;
    for _ in 0..40 {
        end_turn(&mut e);
        let st = e.state();
        let flood = run_of(&e, FLOOD);
        together |= flood.is_some() && run_of(&e, ROUND).is_some();
        // While the flood has not turned and its turn is ahead, it leads,
        // whatever else runs.
        if let Some(f) = flood
            && f.turn.is_none()
            && mandate_sim::arcs::turn_ahead(flood_def, &f.stage)
        {
            let lead = st.narrative.chapter.as_ref().and_then(|ch| ch.lead);
            assert_eq!(lead, Some(f.id.0), "{:?}", st.narrative.chapter);
            flood_led = true;
        }
    }
    assert!(together, "the flood and the round ran at once");
    assert!(flood_led);
    let st = e.state();
    let rounds: Vec<_> = st
        .arcs
        .past
        .iter()
        .filter(|p| p.template == ROUND)
        .collect();
    assert!(!rounds.is_empty(), "the round ended by its own rules");
    // The first chapter's climax was the flood's turn, and the round
    // turned inside the same chapter without being it.
    let first = &st.narrative.chapters[0];
    println!("{first:?}");
    assert_eq!(first.climaxes, 1);
    assert_eq!(first.turn.as_deref(), Some("arc.flood/meeting"));
    let turned = rounds[0].turn.as_ref().unwrap().on;
    assert!(turned >= first.start && Some(turned) <= first.closed);
    // One scene a chapter, and one climax.
    assert!(st.narrative.chapters.iter().all(|ch| ch.climaxes <= 1));
    for ch in &st.narrative.chapters {
        let end = ch.closed.unwrap_or(st.calendar.date);
        let scenes = st
            .arcs
            .runs
            .iter()
            .flat_map(|r| r.played.iter())
            .filter(|p| p.on >= ch.start && p.on <= end && p.outcome.key() != "absent")
            .count();
        assert!(scenes <= 1, "chapter {}: {scenes} scenes played", ch.index);
    }
}
