//! "The world does not wait for the player" (K6; DEC-091): knowledge gates
//! what the player sees and how well they act, never what happens.
//! Validation refuses knowledge conditions where a condition decides
//! whether something happens, and an omniscient player and a default one
//! with the same commands live in the same world.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use mandate_content::conversation::SubjectRef;
use mandate_content::knowledge::Level;
use mandate_content::types::SituationCondition;
use mandate_content::validate::validate_bundle;
use mandate_content::{
    Condition, ContentBundle, Severity, SubjectDef, bundle::read_dir_sources, load,
};
use mandate_sim::{Command, Engine};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::path::Path;
use std::sync::Arc;

fn base() -> ContentBundle {
    let src =
        read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap();
    load(&src).unwrap().0
}

/// The errors about `owner` that cite DEC-091.
fn refusals(b: &ContentBundle, owner: &str) -> usize {
    validate_bundle(b)
        .iter()
        .filter(|i| i.severity == Severity::Error && i.file == owner)
        .filter(|i| i.message.contains("DEC-091"))
        .count()
}

fn knows() -> Condition {
    Condition::Knows {
        subject: SubjectDef::Broker,
        min: Level::Known,
    }
}

#[test]
fn knows_never_decides_what_happens() {
    let b = base();
    assert!(
        validate_bundle(&b)
            .iter()
            .all(|i| i.severity != Severity::Error),
        "base content is valid"
    );
    // A dilemma's triggers.
    let mut x = b.clone();
    x.dilemmas[0].triggers.push(knows());
    let owner = x.dilemmas[0].id.clone();
    assert_eq!(refusals(&x, &owner), 1, "dilemma trigger");
    // A goal's conditions (NPCs pursue goals whatever the player knows).
    let mut x = b.clone();
    x.goals[0].conditions.push(knows());
    let owner = x.goals[0].id.clone();
    assert_eq!(refusals(&x, &owner), 1, "goal condition");
    // A storyline's start and a stage's advance.
    let mut x = b.clone();
    x.storylines[0].start.push(knows());
    x.storylines[0].stages[0].advance.push(knows());
    let owner = x.storylines[0].id.clone();
    assert_eq!(refusals(&x, &owner), 2, "storyline start and advance");
    // A situational policy's unlock: the ledger, what a speaker would
    // tell, and either nested in an `any`.
    let player_knows = SituationCondition::PlayerKnows {
        subject: SubjectRef::Broker,
        min: Some(Level::Known),
        max: None,
    };
    let tells = SituationCondition::Tells {
        subject: SubjectRef::Broker,
        min: Level::Heard,
    };
    let nested = SituationCondition::Any {
        of: vec![
            SituationCondition::Disaster { within_months: 6 },
            player_knows.clone(),
        ],
    };
    for c in [player_knows, tells, nested] {
        let mut x = b.clone();
        let (id, t) = x
            .policies
            .iter_mut()
            .find(|(_, t)| t.unlock.is_some())
            .expect("a situational policy");
        let id = id.clone();
        t.unlock.as_mut().unwrap().any = vec![c];
        assert_eq!(refusals(&x, &id), 1, "policy unlock");
    }
    // A family trigger binds slots (DEC-092); the ledger is refused there
    // too, while a conversation line with the same slots may use it.
    let mut issues = Vec::new();
    mandate_content::validate::check_trigger_condition(
        &b,
        "family.test",
        &SituationCondition::PlayerKnows {
            subject: SubjectRef::Person {
                slot: "speaker".into(),
            },
            min: Some(Level::Heard),
            max: None,
        },
        Some(&["speaker", "about", "player"]),
        &mut issues,
    );
    assert_eq!(issues.len(), 1, "{issues:?}");
    // Where knowledge frames what the player is shown, it stays allowed:
    // the base content validates with it in topics and lines.
    assert!(
        b.talk_topics.iter().any(|t| t
            .offered
            .when
            .iter()
            .any(|c| matches!(c, SituationCondition::PlayerKnows { .. }))),
        "topics are offered by what the player knows"
    );
    assert!(
        b.talk_lines.iter().any(|l| l
            .when
            .iter()
            .any(|c| matches!(c, SituationCondition::Tells { .. }))),
        "lines are chosen by what the speaker will tell"
    );
}

fn game(seed: u64, c: &Arc<ContentBundle>) -> Engine {
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

/// The state without the player's ledger.
fn world(e: &Engine) -> serde_json::Value {
    let mut v = serde_json::to_value(e.state()).unwrap();
    v.pointer_mut("/player")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .remove("knowledge");
    v
}

/// The lab guard: with the same commands (the idle bot's: none but ending
/// turns, so every decision takes its default), an omniscient player and a
/// default one produce identical states except `/player/knowledge`. Crises,
/// NPC goals and bills, decisions, storylines, stories and elections
/// happen whatever the player has learned.
#[test]
fn an_omniscient_and_a_default_idle_player_live_in_the_same_world() {
    let c = Arc::new(base());
    for seed in [5u64, 61, 2026] {
        let mut a = game(seed, &c);
        let mut lab = game(seed, &c);
        lab.set_lab_omniscient(true);
        for turn in 0..60 {
            a.apply(Command::EndTurn).unwrap();
            lab.apply(Command::EndTurn).unwrap();
            if turn % 20 == 19 {
                assert_eq!(world(&a), world(&lab), "seed {seed}, turn {turn}");
            }
        }
        // Things happened that the default player had not all learned of.
        let st = a.state();
        assert!(!st.narrative.decisions.is_empty(), "seed {seed}: decisions");
        assert_ne!(
            a.state().player.as_ref().unwrap().knowledge,
            lab.state().player.as_ref().unwrap().knowledge,
            "seed {seed}: the ledgers differ"
        );
    }
}
