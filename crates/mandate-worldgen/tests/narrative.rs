#![allow(clippy::unwrap_used, clippy::expect_used)]
//! The narrative director (DEC-033 B), dilemmas (DEC-077 §4), NPC goals and
//! storylines (DEC-032), and fairness warnings (DEC-006):
//!
//! - every emitted item is grounded in live actors and true conditions;
//! - pacing stays within the chapter budgets; cooldowns hold; the crisis cap
//!   holds hazards back (timing only, within the hazard model);
//! - a warning precedes every terminal ending and every deselection;
//! - the spotlight never changes an endogenous outcome;
//! - answers apply real effects, storylines branch, goals drive the AI;
//! - determinism, with answers and save/load/continue.

use mandate_content::{ContentBundle, Source, bundle::read_dir_sources, load};
use mandate_sim::actions::PlayerCommand;
use mandate_sim::events::EventKind;
use mandate_sim::narrative::{self, DecisionStatus, NarrativeState, Weight, audit};
use mandate_sim::{Command, Engine};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::path::Path;
use std::sync::Arc;

fn base_sources() -> Vec<Source> {
    read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap()
}

fn base() -> Arc<ContentBundle> {
    Arc::new(load(&base_sources()).unwrap().0)
}

fn engine(c: &Arc<ContentBundle>, seed: u64) -> Engine {
    generate(&WorldConfig::new(seed, WorldSize::Small), c.clone()).unwrap()
}

/// Answer every pending decision with its first affordable non-default
/// option (a stand-in for a player who engages).
fn answer_all(e: &mut Engine, c: &ContentBundle) {
    let pending: Vec<_> = e.state().narrative.pending().cloned().collect();
    for d in pending {
        let def = c.dilemma(&d.template).unwrap();
        for o in def.options.iter().filter(|o| !o.default) {
            let cmd = Command::Player(PlayerCommand::Answer {
                decision: d.id.0,
                option: o.id.clone(),
            });
            if e.apply(cmd).is_ok() {
                break;
            }
        }
    }
}

/// Advance to the next director day (the weekly run), so that the state
/// after the step is the state the director emitted from.
fn to_director_day(e: &mut Engine) {
    let d = e.state().calendar.date.0;
    let step = 7 - d % 7;
    e.apply(Command::AdvanceDays { days: step }).unwrap();
}

#[test]
fn every_emitted_item_is_grounded_in_live_state() {
    let c = base();
    let (mut decisions, mut facts) = (0usize, 0usize);
    let mut problems: Vec<String> = Vec::new();
    for seed in 1..=24u64 {
        let mut e = engine(&c, seed);
        let mut seen_decision = 0u32;
        let mut seen_fact = 0u32;
        let weeks = 10 * 52 + 2;
        for w in 0..weeks {
            to_director_day(&mut e);
            let st = e.state();
            for d in st
                .narrative
                .decisions
                .iter()
                .filter(|d| d.id.0 >= seen_decision)
            {
                if d.opened != st.calendar.date {
                    continue;
                }
                decisions += 1;
                for p in audit::decision(st, &c, d) {
                    problems.push(format!("seed {seed} week {w}: {p}"));
                }
            }
            seen_decision = st.narrative.decision_ids.peek_next();
            for f in st
                .narrative
                .facts
                .iter()
                .filter(|f| f.id >= seen_fact && f.spotlit)
            {
                facts += 1;
                for p in audit::fact(st, &c, f) {
                    problems.push(format!("seed {seed} week {w}: {p}"));
                }
            }
            seen_fact = st.narrative.next_fact;
            // Half the careers engage with their decisions; half let them pass.
            if seed % 2 == 0 {
                answer_all(&mut e, &c);
            }
        }
    }
    assert!(
        problems.is_empty(),
        "{:#?}",
        &problems[..problems.len().min(20)]
    );
    assert!(decisions > 24 * 10, "decisions emitted: {decisions}");
    assert!(facts > decisions, "facts: {facts}");
}

/// Budgets, per chapter, across careers; climaxes; cooldowns.
#[test]
fn pacing_stays_within_budgets_and_cooldowns_hold() {
    let c = base();
    let d = &c.director;
    let mut chapters = Vec::new();
    for seed in 1..=12u64 {
        let mut e = engine(&c, seed);
        for _ in 0..(6 * 30) {
            answer_all(&mut e, &c);
            e.apply(Command::EndTurn).unwrap();
        }
        let n: &NarrativeState = &e.state().narrative;
        let b = narrative::director::budget(d, n.profile(&c));
        for ch in &n.chapters {
            assert!(ch.climaxes <= b.climax, "seed {seed}: {ch:?}");
            assert!(ch.significant <= b.sig_max, "seed {seed}: {ch:?}");
            assert!(ch.minor <= b.minor_max, "seed {seed}: {ch:?}");
            chapters.push(ch.clone());
        }
        // Cooldowns: the same dilemma returns only after its cooldown (a
        // climax may revisit a theme earlier).
        let ds = &n.decisions;
        for (i, a) in ds.iter().enumerate() {
            for b2 in ds.iter().skip(i + 1) {
                if a.template != b2.template || b2.weight == Weight::Climax {
                    continue;
                }
                let cd = c.dilemma(&a.template).unwrap().cooldown_days;
                assert!(
                    a.opened.add_days(cd) <= b2.opened,
                    "seed {seed}: {} returned after {} days",
                    a.template,
                    a.opened.days_until(b2.opened)
                );
            }
        }
        // Spacing: decisions of weight climax/significant are spaced out
        // (a storyline's turn has priority and may come sooner).
        let mut opened: Vec<_> = ds
            .iter()
            .filter(|x| x.weight != Weight::Minor && x.storyline.is_none())
            .map(|x| x.opened)
            .collect();
        opened.sort();
        for w in opened.windows(2) {
            assert!(w[0].days_until(w[1]) >= d.spacing_days, "seed {seed}");
        }
    }
    let sessions: Vec<_> = chapters
        .iter()
        .filter(|c| c.kind == narrative::ChapterKind::Session)
        .collect();
    let campaigns = chapters.len() - sessions.len();
    let climax = chapters.iter().filter(|c| c.climaxes == 1).count();
    let sig: f64 =
        chapters.iter().map(|c| c.significant as f64).sum::<f64>() / chapters.len() as f64;
    let minor: f64 = chapters.iter().map(|c| c.minor as f64).sum::<f64>() / chapters.len() as f64;
    eprintln!(
        "chapters {} (sessions {}, campaigns {campaigns}): climax in {climax}, mean significant {sig:.2}, mean minor {minor:.2}",
        chapters.len(),
        sessions.len()
    );
    // About one climax per chapter (DEC-005): nearly every chapter has one.
    assert!(climax as f64 >= 0.85 * chapters.len() as f64);
    assert!((1.5..=4.0).contains(&sig), "significant per chapter {sig}");
}

/// The crisis cap: while the player's town carries its cap of crises, a
/// fresh hazard does not strike it (it is held, and strikes later). Timing
/// only: the same hazards, drawn from the same stream, strike under Classic
/// and Unfiltered, each no later than the longest deferral.
#[test]
fn the_crisis_cap_holds_hazards_back_and_only_shifts_them() {
    // A pack whose two crisis kinds every town always has: every town is at
    // the cap, so every hazard on the player's town is held.
    let mut src = base_sources();
    src.push(Source {
        path: "aa_cap/crises.jsonc".into(),
        text: r#"{"kind":"crises","items":[
            {"id":"crisis.smog_a","name_key":"crisis-smog","indicator":"pollution","begin":0.0,"end":0.0,"severe":3.0},
            {"id":"crisis.smog_b","name_key":"crisis-smog","indicator":"pollution","begin":0.0,"end":0.0,"severe":3.0}]}"#
            .into(),
    });
    src.push(Source {
        path: "aa_cap/locale/en/cap.ftl".into(),
        text: "crisis-smog = Smog\n".into(),
    });
    let c = Arc::new(load(&src).unwrap().0);
    let max = c.director.chance.hazard_max_defer_days;
    let mut held_total = 0;
    for seed in 1..=24u64 {
        let mut classic = engine(&c, seed);
        let mut unfiltered = engine(&c, seed);
        unfiltered.setup(|s| s.narrative.profile = "storyteller.unfiltered".into());
        let m = classic.state().player.as_ref().unwrap().municipality.0;
        let mut struck_c = Vec::new();
        let mut struck_u = Vec::new();
        let mut last = classic
            .state()
            .events
            .iter()
            .last()
            .map(|e| e.id.0)
            .unwrap_or(0);
        let mut last_u = last;
        for _ in 0..(10 * 52) {
            let hold = classic.state().narrative.chance.hazard_hold;
            to_director_day(&mut classic);
            to_director_day(&mut unfiltered);
            let st = classic.state();
            let mut late = 0;
            for ev in st.events.iter().filter(|x| x.id.0 > last) {
                match &ev.kind {
                    EventKind::DirectorIntervention {
                        lever,
                        municipality,
                    } if *municipality == m => {
                        if lever == "hazard_struck_late" {
                            late += 1;
                        }
                        if lever == "hazard_held" {
                            held_total += 1;
                        }
                    }
                    EventKind::Disaster {
                        municipality,
                        hazard,
                        severity_pct,
                    } if *municipality == m
                        && *hazard != mandate_sim::environment::HazardKind::Drought =>
                    {
                        struck_c.push((ev.date, *hazard, *severity_pct));
                        let active = hold.is_some_and(|(t, until)| t == m && ev.date < until);
                        if active {
                            assert!(
                                late > 0,
                                "seed {seed}: a fresh hazard struck a town at its cap"
                            );
                            late -= 1;
                        }
                    }
                    _ => {}
                }
            }
            last = st.events.iter().last().map(|e| e.id.0).unwrap_or(last);
            let su = unfiltered.state();
            for ev in su.events.iter().filter(|x| x.id.0 > last_u) {
                if let EventKind::Disaster {
                    municipality,
                    hazard,
                    severity_pct,
                } = &ev.kind
                    && *municipality == m
                    && *hazard != mandate_sim::environment::HazardKind::Drought
                {
                    struck_u.push((ev.date, *hazard, *severity_pct));
                }
            }
            last_u = su.events.iter().last().map(|e| e.id.0).unwrap_or(last_u);
        }
        // Still held at the end are the difference.
        let pending = classic
            .state()
            .environment
            .deferred
            .iter()
            .filter(|h| h.municipality == m)
            .count();
        assert_eq!(struck_c.len() + pending, struck_u.len(), "seed {seed}");
        for (a, b) in struck_c.iter().zip(&struck_u) {
            assert_eq!(
                (a.1, a.2),
                (b.1, b.2),
                "seed {seed}: the same hazard, as drawn"
            );
            assert!(
                b.0 <= a.0 && b.0.days_until(a.0) <= max + 31,
                "seed {seed}: held too long"
            );
        }
    }
    eprintln!("hazards held across 24 careers: {held_total}");
    assert!(held_total > 0, "the cap held something back");
}

/// DEC-006: a warning precedes every terminal ending (the tracks' ends) and
/// every deselection, and an ending is allowed only after the response time.
#[test]
fn a_warning_precedes_every_terminal_ending_and_deselection() {
    let c = base();
    let f = &c.director.fairness;
    let mut deselections = 0;
    for seed in 1..=12u64 {
        let mut e = engine(&c, seed);
        // A career that drifts toward the edge: low standing and health, set
        // after the first month (when the last election lands on the tracks).
        e.apply(Command::AdvanceDays { days: 40 }).unwrap();
        e.setup(|s| {
            let p = s.player.as_mut().unwrap();
            p.tracks.standing = 27.0;
            p.tracks.health = 30.0;
        });
        for _ in 0..(4 * 365) {
            e.apply(Command::AdvanceDays { days: 1 }).unwrap();
            let st = e.state();
            let pl = st.player.as_ref().unwrap();
            let n = &st.narrative;
            for (track, at_end) in [
                ("standing", pl.tracks.standing <= 0.0),
                ("viability", pl.tracks.viability <= 0.0),
                ("health", pl.tracks.health <= 0.0),
                ("legal_jeopardy", pl.tracks.legal_jeopardy >= 100.0),
                ("threat", pl.tracks.threat >= 100.0),
            ] {
                if at_end {
                    assert!(
                        n.warnings.iter().any(|w| w.track == track),
                        "seed {seed}: {track} ended unwarned"
                    );
                }
            }
            if pl.tracks.standing as f64 <= f.standing {
                assert!(n.warnings.iter().any(|w| w.track == "standing"));
            }
        }
        let st = e.state();
        for ev in st.events.iter() {
            if matches!(ev.kind, EventKind::PlayerDeselected) {
                deselections += 1;
                assert!(
                    st.narrative
                        .warnings
                        .iter()
                        .any(|w| w.track == "standing" && w.date < ev.date),
                    "seed {seed}: deselected without a warning"
                );
            }
        }
        // The ending gate: not before the warning has had time to work.
        let w = st
            .narrative
            .warnings
            .iter()
            .find(|w| w.track == "standing")
            .unwrap();
        assert!(!st.narrative.ending_allowed(&c, "standing", w.date));
        assert!(
            st.narrative
                .ending_allowed(&c, "standing", w.date.add_days(f.response_days))
        );
    }
    eprintln!("deselections checked: {deselections}");
    assert!(deselections > 0);
}

/// Everything the engine computes except the narrative domain and the
/// event log (whose ids shift with the director's own entries).
fn endogenous(e: &Engine) -> serde_json::Value {
    let st = e.state();
    let mut policy = serde_json::to_value(&st.policy).unwrap();
    for p in policy["instances"].as_array_mut().unwrap() {
        p["cause"] = serde_json::Value::Null;
    }
    let mut economy = serde_json::to_value(&st.economy).unwrap();
    economy["global"]["recession"]["event"] = serde_json::Value::Null;
    serde_json::json!({
        "elections": st.politics.elections,
        "politicians": st.politics.politicians,
        "councils": st.politics.councils,
        "declared": serde_json::to_value(st.politics.declared_votes.iter().collect::<Vec<_>>()).unwrap(),
        "characters": st.politics.characters,
        "policy": policy,
        "population": st.population,
        "economy": economy,
        "finance": st.finance,
        "environment": st.environment,
        "player": st.player,
    })
}

fn first_diff(a: &serde_json::Value, b: &serde_json::Value, path: String) -> String {
    use serde_json::Value;
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            for (k, v) in x {
                if y.get(k) != Some(v) {
                    return first_diff(v, y.get(k).unwrap_or(&Value::Null), format!("{path}.{k}"));
                }
            }
            format!("{path}: keys differ")
        }
        (Value::Array(x), Value::Array(y)) => {
            for (i, (v, w)) in x.iter().zip(y).enumerate() {
                if v != w {
                    return first_diff(v, w, format!("{path}[{i}]"));
                }
            }
            format!("{path}: lengths {} vs {}", x.len(), y.len())
        }
        _ => format!("{path}: {a} vs {b}"),
    }
}

/// DEC-033 B, structurally: the spotlight (decisions, storylines,
/// developments) never changes an endogenous outcome. Spotlight on and off,
/// for the same seed and the same exogenous draws (the levers read the
/// simulation alone), with decisions left unapplied: every election result,
/// policy, and domain is identical.
#[test]
fn the_spotlight_never_changes_endogenous_outcomes() {
    let c = base();
    for seed in [3u64, 7, 11, 19] {
        let mut on = engine(&c, seed);
        let mut off = engine(&c, seed);
        on.setup(|s| s.narrative.resolve_defaults = false);
        off.setup(|s| {
            s.narrative.resolve_defaults = false;
            s.narrative.spotlight = false;
        });
        for _ in 0..(3 * 30) {
            on.apply(Command::EndTurn).unwrap();
            off.apply(Command::EndTurn).unwrap();
        }
        assert!(on.state().narrative.decisions.len() > 3, "seed {seed}");
        assert!(off.state().narrative.decisions.is_empty());
        assert_eq!(on.state().narrative.chance, off.state().narrative.chance);
        let (a, b) = (endogenous(&on), endogenous(&off));
        if a != b {
            panic!("seed {seed}: {}", first_diff(&a, &b, String::new()));
        }
    }
}

/// An answer applies real effects (DEC-073), records an aftermath, and its
/// memories are what the person later quotes.
#[test]
fn answers_apply_real_effects_and_are_remembered() {
    let c = base();
    let mut found = false;
    for seed in 1..=12u64 {
        let mut e = engine(&c, seed);
        for _ in 0..120 {
            let pending = e.state().narrative.pending().next().cloned();
            if let Some(d) = pending {
                let def = c.dilemma(&d.template).unwrap();
                let Some(o) = def.options.iter().find(|o| {
                    o.effects.iter().any(|x| {
                        matches!(
                            x,
                            mandate_content::EffectDef::Opinion {
                                memory: Some(_),
                                ..
                            }
                        )
                    }) && !o
                        .effects
                        .iter()
                        .any(|x| matches!(x, mandate_content::EffectDef::Slots { .. }))
                }) else {
                    let _ = e.apply(Command::EndTurn);
                    continue;
                };
                let before = e.state().player.as_ref().unwrap().tracks.standing;
                e.apply(Command::Player(PlayerCommand::Answer {
                    decision: d.id.0,
                    option: o.id.clone(),
                }))
                .unwrap();
                let st = e.state();
                let x = st.narrative.decision(d.id).unwrap();
                assert_eq!(x.status, DecisionStatus::Answered);
                let res = x.resolution.as_ref().unwrap();
                assert_eq!(res.applied.len(), o.effects.len());
                let me = mandate_sim::characters::player_character(st).unwrap();
                let rememberers: Vec<_> = res
                    .applied
                    .iter()
                    .filter_map(|a| match a {
                        narrative::Applied::Opinion { person, .. } => Some(*person),
                        _ => None,
                    })
                    .collect();
                assert!(rememberers.iter().any(|p| {
                    st.politics
                        .characters
                        .get(*p)
                        .unwrap()
                        .memories
                        .iter()
                        .any(|m| {
                            m.actor == me
                                && m.subject
                                    == Some(mandate_sim::characters::Subject::Decision {
                                        id: d.id.0,
                                    })
                        })
                }));
                let standing = res.applied.iter().find_map(|a| match a {
                    narrative::Applied::Standing { delta } => Some(*delta),
                    _ => None,
                });
                if let Some(s) = standing {
                    let now = st.player.as_ref().unwrap().tracks.standing;
                    assert!((now - before - s).abs() < 1e-3);
                }
                // Answering twice is refused.
                assert!(
                    e.apply(Command::Player(PlayerCommand::Answer {
                        decision: d.id.0,
                        option: o.id.clone(),
                    }))
                    .is_err()
                );
                found = true;
                break;
            }
            e.apply(Command::EndTurn).unwrap();
        }
        if found {
            break;
        }
    }
    assert!(found);
}

/// Idle players let decisions pass: the default happens at the deadline,
/// and it costs someone.
#[test]
fn deadlines_apply_the_default() {
    let c = base();
    let mut e = engine(&c, 4);
    for _ in 0..80 {
        e.apply(Command::EndTurn).unwrap();
    }
    let n = &e.state().narrative;
    let defaulted: Vec<_> = n
        .decisions
        .iter()
        .filter(|d| d.status == DecisionStatus::Defaulted)
        .collect();
    assert!(!defaulted.is_empty());
    for d in defaulted {
        let r = d.resolution.as_ref().unwrap();
        assert!(r.by_default);
        assert!(r.date <= d.deadline.add_days(14));
        let def = c.dilemma(&d.template).unwrap();
        assert_eq!(def.options.iter().find(|o| o.default).unwrap().id, r.option);
    }
}

/// Storylines advance through stages and branch on the player's answer.
#[test]
fn storylines_advance_and_branch() {
    let c = base();
    let mut stages = 0;
    let mut branched = false;
    for seed in 1..=8u64 {
        let mut e = engine(&c, seed);
        for _ in 0..(4 * 30) {
            // Answer with the last option that is not the default.
            let pending: Vec<_> = e.state().narrative.pending().cloned().collect();
            for d in pending {
                let def = c.dilemma(&d.template).unwrap();
                for o in def.options.iter().rev().filter(|o| !o.default) {
                    let cmd = Command::Player(PlayerCommand::Answer {
                        decision: d.id.0,
                        option: o.id.clone(),
                    });
                    if e.apply(cmd).is_ok() {
                        break;
                    }
                }
            }
            e.apply(Command::EndTurn).unwrap();
        }
        for r in &e.state().narrative.storylines {
            stages += r.stages.len();
            let def = c.storyline(&r.template).unwrap();
            // Stages were reached in order, except where an answer branched.
            for w in r.stages.windows(2) {
                let a = def.stages.iter().position(|s| s.id == w[0].0).unwrap();
                let b = def.stages.iter().position(|s| s.id == w[1].0).unwrap();
                if b != a + 1 {
                    branched = true;
                    assert!(def.stages[a].decision.is_some());
                }
                assert!(w[0].1 <= w[1].1);
            }
        }
    }
    assert!(stages > 8, "storyline stages reached: {stages}");
    assert!(branched, "a branch was taken");
}

/// Goals drive the NPC AI (DEC-032, DEC-030): an officeholder out to stop
/// the player leans against the player's bills.
#[test]
fn an_opposing_goal_turns_a_colleague_against_the_players_bills() {
    let c = base();
    let mut e = engine(&c, 9);
    let st = e.state();
    let pl = st.player.as_ref().unwrap();
    let m = pl.municipality.0;
    let t = c
        .policies
        .values()
        .find(|t| t.tier == mandate_content::TierKey::Local)
        .unwrap()
        .id
        .clone();
    let id = e
        .apply(e.route(PlayerCommand::DraftPolicy {
            template: t.clone(),
            params: Default::default(),
        }))
        .unwrap()
        .result
        .unwrap()
        .created
        .unwrap();
    let st = e.state();
    let colleague = st.politics.councils[m as usize]
        .members
        .iter()
        .copied()
        .find(|p| !st.politics.get(*p).unwrap().is_player)
        .unwrap();
    let leans = |e: &Engine| {
        let st = e.state();
        let inst = &st.policy.instances[id as usize];
        mandate_sim::policy::council_leans(
            &c.council,
            &st.world,
            &st.population,
            &st.politics,
            st.meta.seed,
            c.policies.get(&t).unwrap(),
            inst,
        )
        .into_iter()
        .find(|(p, _)| *p == colleague)
        .unwrap()
        .1
    };
    let before = leans(&e);
    let ch = e.state().politics.get(colleague).unwrap().character;
    e.setup(|s| {
        let d = s.calendar.date;
        s.politics
            .characters
            .goals
            .push(mandate_sim::characters::goals::Goal {
                holder: ch,
                goal: "goal.oppose_player".into(),
                since: d,
                lean: -0.35,
                champion: 0.0,
            });
    });
    let after = leans(&e);
    assert!(after < before, "{after} < {before}");
}

/// Same seed twice with answers, and save → load → continue: identical.
#[test]
fn narrative_play_is_deterministic_through_saves() {
    let c = base();
    let play = |turns: u32| {
        let mut e = engine(&c, 21);
        for _ in 0..turns {
            answer_all(&mut e, &c);
            e.apply(Command::EndTurn).unwrap();
        }
        e
    };
    let a = play(90);
    let b = play(90);
    assert_eq!(a.state_hash(), b.state_hash());
    let half = play(45);
    let json = serde_json::to_string(half.state()).unwrap();
    let state: mandate_sim::State = serde_json::from_str(&json).unwrap();
    let mut h = Engine::from_state(state, c.clone()).unwrap();
    for _ in 0..45 {
        answer_all(&mut h, &c);
        h.apply(Command::EndTurn).unwrap();
    }
    assert_eq!(a.state_hash(), h.state_hash());
    assert!(!a.state().narrative.decisions.is_empty());
}

/// Steady (DEC-033 addendum): fewer hazard strikes on the player's town and
/// smaller budgets; Unfiltered shapes nothing.
#[test]
fn storyteller_profiles_shape_chance_not_consequences() {
    let c = base();
    let steady = c.storyteller("storyteller.steady").unwrap();
    let classic = c.storyteller("storyteller.classic").unwrap();
    let bs = narrative::director::budget(&c.director, Some(steady));
    let bc = narrative::director::budget(&c.director, Some(classic));
    assert!(bs.sig_max < bc.sig_max && bs.minor_max < bc.minor_max);
    let (mut averted, mut struck_steady, mut struck_unfiltered) = (0, 0, 0);
    let in_town = |e: &Engine, from: mandate_core::SimDate| {
        let m = e.state().player.as_ref().unwrap().municipality.0;
        e.state()
            .events
            .iter()
            .filter(|ev| ev.date >= from)
            .filter(|ev| matches!(&ev.kind, EventKind::Disaster { municipality, hazard, .. } if *municipality == m && *hazard != mandate_sim::environment::HazardKind::Drought))
            .count()
    };
    for seed in 1..=16u64 {
        let mut e = engine(&c, seed);
        let mut u = engine(&c, seed);
        e.setup(|s| s.narrative.profile = "storyteller.steady".into());
        u.setup(|s| s.narrative.profile = "storyteller.unfiltered".into());
        // Count from the start of play (history before it is shared).
        let from = e.state().calendar.date;
        e.apply(Command::AdvanceDays { days: 365 * 10 }).unwrap();
        u.apply(Command::AdvanceDays { days: 365 * 10 }).unwrap();
        averted += e
            .state()
            .events
            .iter()
            .filter(|ev| matches!(&ev.kind, EventKind::DirectorIntervention { lever, .. } if lever == "hazard_averted"))
            .count();
        struck_steady += in_town(&e, from);
        struck_unfiltered += in_town(&u, from);
        assert_eq!(
            e.state().narrative.chance.hazard_keep.map(|k| k.1),
            Some(steady.hazard_rate as f32)
        );
        assert_eq!(u.state().narrative.chance, Default::default());
    }
    eprintln!(
        "hazards on the player's town (16 careers × 10 years): Unfiltered {struck_unfiltered}, Steady {struck_steady} ({averted} averted)"
    );
    assert!(struck_steady <= struck_unfiltered);
}
