#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Media, scandals, and the legal path (DEC-034, DEC-006, DEC-030, DEC-033):
//!
//! - every story traces to a fact (the grounding audit);
//! - perception changes only through coverage: a hidden fact changes no
//!   cohort's opinion until it is discovered and covered;
//! - the director shifts when a hidden fact comes out, within its window,
//!   never whether;
//! - NPC corruption uses the same mechanics as the player's;
//! - each stage of the legal path and each outcome; a warning precedes any
//!   career-ending sentence; a short sentence is time out and a comeback;
//! - determinism through saves.

use mandate_content::{
    ContentBundle, Discoverer, Gravity, HiddenSpec, Source, bundle::read_dir_sources, load,
};
use mandate_sim::actions::{Action, PlayerCommand};
use mandate_sim::justice::{Sentence, Stage};
use mandate_sim::media::{self, StorySource};
use mandate_sim::narrative::Fate;
use mandate_sim::player::RunStatus;
use mandate_sim::politics::PoliticianId;
use mandate_sim::{Command, Engine};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use serde_json::Value;
use std::path::Path;
use std::sync::Arc;

fn sources() -> Vec<Source> {
    read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap()
}

fn base() -> Arc<ContentBundle> {
    Arc::new(load(&sources()).unwrap().0)
}

/// Base content with values set in one parameter file (`file`, JSON pointer
/// under `value`, new value).
fn with(edits: &[(&str, &str, Value)]) -> Arc<ContentBundle> {
    let mut src = sources();
    for (file, ptr, v) in edits {
        let s = src
            .iter_mut()
            .find(|s| s.path == format!("base/{file}.jsonc"))
            .unwrap();
        let mut j: Value =
            serde_json::from_str(&mandate_content::jsonc::to_strict_json(&s.text)).unwrap();
        *j.pointer_mut(&format!("/value{ptr}")).unwrap() = v.clone();
        s.text = j.to_string();
    }
    Arc::new(load(&src).unwrap().0)
}

fn engine(c: &Arc<ContentBundle>, seed: u64) -> Engine {
    generate(&WorldConfig::new(seed, WorldSize::Small), c.clone()).unwrap()
}

fn me(e: &Engine) -> PoliticianId {
    e.state().player.as_ref().unwrap().politician
}

/// Plant a hidden fact that will come out between `days` and `days + 7`
/// from now (a test's stand-in for a shady act with its fate settled).
fn plant(e: &mut Engine, c: &ContentBundle, who: PoliticianId, sev: Gravity, days: u32) -> u32 {
    let spec = HiddenSpec {
        kind: "bribe".into(),
        severity: sev,
        evidence: 1.0,
        discoverers: vec![],
    };
    let mut id = 0;
    e.setup(|s| {
        id = media::hide(s, c, &spec, who, Vec::new(), mandate_sim::spine::Cause::Lab);
        let date = s.calendar.date;
        let h = s.narrative.hidden.iter_mut().find(|h| h.id == id).unwrap();
        h.window = [date.add_days(days), date.add_days(days + 7)];
        h.fate = Some(Fate {
            chance: 1.0,
            discovered: true,
            by: Discoverer::Journalist,
            who: None,
        });
    });
    id
}

/// The quick legal path: every scandal investigated, charged, and tried
/// within months (the stages and the odds are content).
fn quick(extra: &[(&str, &str, Value)]) -> Arc<ContentBundle> {
    let mut edits: Vec<(&str, &str, Value)> = vec![
        (
            "justice",
            "/open",
            serde_json::json!({ "ethics": 1.0, "misconduct": 1.0, "crime": 1.0 }),
        ),
        ("justice", "/open_days", serde_json::json!([7, 7])),
        (
            "justice",
            "/investigation_days",
            serde_json::json!([30, 30]),
        ),
        (
            "justice",
            "/charge",
            serde_json::json!({ "ethics": 1.0, "misconduct": 1.0, "crime": 1.0 }),
        ),
        ("justice", "/trial_days", serde_json::json!([60, 60])),
        ("justice", "/convict_base", serde_json::json!(6.0)),
        ("justice", "/disqualify", serde_json::json!(0.0)),
    ];
    edits.extend(extra.iter().cloned());
    with(&edits)
}

fn turns_until(e: &mut Engine, n: u32, f: &dyn Fn(&Engine) -> bool) -> bool {
    for _ in 0..n {
        if f(e) {
            return true;
        }
        if e.apply(Command::EndTurn).is_err() {
            return f(e);
        }
    }
    f(e)
}

/// REQ-003, DEC-034: every story that runs traces to a fact, checked on the
/// day it runs, across careers that take shady options and do shady things.
#[test]
fn every_story_traces_to_a_fact() {
    let c = base();
    let (mut stories, mut scandals) = (0usize, 0usize);
    let mut problems = Vec::new();
    for seed in 1..=10u64 {
        let mut e = engine(&c, seed);
        let mut seen = 0u32;
        for t in 0..(4 * 26) {
            // A shady streak: an envelope now and then, a leak, the first
            // option of each decision.
            if t % 9 == 3 {
                let _ = e.apply(e.route(PlayerCommand::Act {
                    action: Action::TakeBribe,
                    late_night: true,
                }));
            }
            if t % 17 == 5 {
                let _ = e.apply(e.route(PlayerCommand::Act {
                    action: Action::LeakFile,
                    late_night: false,
                }));
            }
            let pending: Vec<_> = e.state().narrative.pending().cloned().collect();
            for d in pending {
                let def = c.dilemma(&d.template).unwrap();
                for o in &def.options {
                    if e.apply(Command::Player(PlayerCommand::Answer {
                        decision: d.id.0,
                        option: o.id.clone(),
                    }))
                    .is_ok()
                    {
                        break;
                    }
                }
            }
            // Day by day, so each story is audited the day it runs.
            for _ in 0..14 {
                e.apply(Command::AdvanceDays { days: 1 }).unwrap();
                let st = e.state();
                for s in st.media.stories.iter().filter(|s| s.id >= seen) {
                    stories += 1;
                    for p in media::audit_story(st, s) {
                        problems.push(format!("seed {seed}: {p}"));
                    }
                }
                seen = st.media.next_story;
            }
        }
        scandals += e.state().media.scandals.len();
    }
    assert!(
        problems.is_empty(),
        "{:#?}",
        &problems[..problems.len().min(20)]
    );
    eprintln!("stories audited {stories}, scandals {scandals}");
    assert!(stories > 100 && scandals > 10);
}

/// DEC-034: perception changes only through coverage. The same career with
/// and without a hidden fact: every cohort's view of the player is the same
/// until the first story about it runs, and differs after.
#[test]
fn a_hidden_fact_changes_nothing_until_it_is_covered() {
    let c = base();
    let seed = 5;
    let mut a = engine(&c, seed);
    let mut b = engine(&c, seed);
    for e in [&mut a, &mut b] {
        e.setup(|s| s.narrative.resolve_defaults = false);
    }
    let p = me(&a);
    plant(&mut a, &c, p, Gravity::Crime, 40);
    let view = |e: &Engine| -> Vec<(f32, f32)> {
        e.state()
            .population
            .hoods
            .iter()
            .flat_map(|h| {
                h.cohorts
                    .iter()
                    .map(|c| (c.player_approval, c.player_awareness))
            })
            .collect()
    };
    let mut covered = None;
    for day in 0..120 {
        a.apply(Command::AdvanceDays { days: 1 }).unwrap();
        b.apply(Command::AdvanceDays { days: 1 }).unwrap();
        let about_me =
            a.state().media.stories.iter().any(|s| {
                s.subject == Some(p) && !matches!(s.source, StorySource::Statement { .. })
            });
        if covered.is_none() {
            if about_me {
                covered = Some(day);
            } else {
                assert_eq!(
                    view(&a),
                    view(&b),
                    "day {day}: opinion moved before any coverage"
                );
            }
        }
    }
    let day = covered.expect("the story ran");
    assert!(day >= 40, "it came out at {day}, before its window");
    assert_ne!(view(&a), view(&b), "coverage moved opinion");
    // The scandal hurt: the player's approval in town fell relative to the twin.
    let mean = |e: &Engine| {
        let st = e.state();
        let m = st.player.as_ref().unwrap().municipality;
        let hs: Vec<usize> = st.world.hoods_of(m).collect();
        let (mut x, mut w) = (0.0, 0.0);
        for h in hs {
            for c in &st.population.hoods[h].cohorts {
                x += c.player_approval as f64 * c.count as f64;
                w += c.count as f64;
            }
        }
        x / w
    };
    assert!(mean(&a) < mean(&b));
    assert!(
        a.state().politics.get(p).unwrap().scandal > b.state().politics.get(p).unwrap().scandal
    );
}

/// DEC-033 B: the director chooses when a hidden fact comes out within its
/// evidence window, never whether. With the director's shaping on and off,
/// the same facts (same id, date, kind, and subject) have the same fate, and
/// every date is inside its window; some dates differ.
#[test]
fn the_director_shifts_when_never_whether() {
    let c = base();
    let (mut compared, mut shifted) = (0, 0);
    for seed in 1..=8u64 {
        let mut on = engine(&c, seed);
        let mut off = engine(&c, seed);
        off.setup(|s| s.narrative.shape_discovery = false);
        for e in [&mut on, &mut off] {
            for t in 0..60 {
                if t % 6 == 1 {
                    let _ = e.apply(e.route(PlayerCommand::Act {
                        action: Action::TakeBribe,
                        late_night: true,
                    }));
                }
                // (A career may end in prison along the way.)
                let _ = e.apply(Command::EndTurn);
            }
        }
        let (x, y) = (&on.state().narrative.hidden, &off.state().narrative.hidden);
        for h in x {
            if let Some(d) = h.discoverable_on {
                assert!(
                    d >= h.window[0] && d <= h.window[1],
                    "seed {seed}: outside its window"
                );
            }
            let Some(g) = y.iter().find(|g| g.id == h.id) else {
                continue;
            };
            if g.date != h.date || g.kind != h.kind || g.subject != h.subject {
                continue;
            }
            compared += 1;
            assert_eq!(g.fate, h.fate, "seed {seed}: the director changed whether");
            assert_eq!(g.window, h.window);
            if g.discoverable_on != h.discoverable_on {
                shifted += 1;
            }
        }
    }
    eprintln!("hidden facts compared {compared}, timing shifted {shifted}");
    assert!(compared > 20);
    assert!(shifted > 0, "the director shaped some timing");
}

/// DEC-030: an NPC's wrongdoing goes through the same mechanics as the
/// player's: the same exposure function, the same scandal, the same case
/// stages and sentence, and the seat is filled when they are removed.
#[test]
fn npc_corruption_uses_the_same_mechanics() {
    let c = quick(&[("justice", "/prison_years", serde_json::json!([2, 2]))]);
    let mut e = engine(&c, 9);
    let p = me(&e);
    let m = e.state().player.as_ref().unwrap().municipality.0;
    let npc = council_members(e.state(), m as usize)
        .iter()
        .copied()
        .find(|id| *id != p)
        .unwrap();
    let party = e.state().politics.get(npc).unwrap().party;
    let spec = HiddenSpec {
        kind: "bribe".into(),
        severity: Gravity::Crime,
        evidence: 0.5,
        discoverers: vec![],
    };
    // The same function prices the same act for both.
    let (xp, xn) = (
        media::exposure(e.state(), &c, &spec, p),
        media::exposure(e.state(), &c, &spec, npc),
    );
    assert_eq!(xp.window, xn.window);
    assert!(xp.chance > 0.0 && xn.chance > 0.0);
    plant(&mut e, &c, npc, Gravity::Crime, 20);
    plant(&mut e, &c, p, Gravity::Crime, 20);
    turns_until(&mut e, 60, &|e| {
        e.state()
            .justice
            .cases
            .iter()
            .filter(|x| x.stage == Stage::Convicted)
            .count()
            >= 2
    });
    let st = e.state();
    for who in [npc, p] {
        assert!(
            st.media.scandals.iter().any(|s| s.subject == who),
            "{who:?}: no scandal"
        );
        let case = st
            .justice
            .cases
            .iter()
            .find(|x| x.subject == who)
            .expect("a case");
        let stages: Vec<Stage> = case.history.iter().map(|h| h.0).collect();
        assert_eq!(
            stages,
            vec![Stage::Investigation, Stage::Charged, Stage::Convicted],
            "{who:?}"
        );
        assert_eq!(case.sentence, Some(Sentence::Prison { years: 2 }));
        assert!(
            !st.politics.get(who).unwrap().in_office(),
            "{who:?} still in office"
        );
        assert!(st.justice.barred(who, st.calendar.date));
        assert!(st.politics.get(who).unwrap().scandal > 0.0);
    }
    // The NPC's seat went to their party.
    let council = council_members(st, m as usize);
    let seat = council
        .iter()
        .find(|id| st.politics.get(**id).unwrap().party == party && **id != npc);
    assert!(seat.is_some());
    assert!(!council.contains(&npc) && !council.contains(&p));
    // NPCs take bribes on their own too (by personality), with fates.
    let mut e = engine(&base(), 11);
    for _ in 0..(5 * 26) {
        e.apply(Command::EndTurn).unwrap();
    }
    let npc_facts: Vec<_> = e
        .state()
        .narrative
        .hidden
        .iter()
        .filter(|h| h.subject.is_some_and(|s| s != me(&e)))
        .collect();
    assert!(!npc_facts.is_empty(), "no NPC was tempted in five years");
    assert!(npc_facts.iter().all(|h| h.fate.is_some()));
}

/// The legal path's stages and outcomes, for the player: investigation,
/// charges, trial, conviction; a short sentence takes the player out of
/// politics, time passes, and they can run again.
#[test]
fn a_short_sentence_is_time_out_and_a_comeback() {
    let c = quick(&[("justice", "/prison_years", serde_json::json!([2, 2]))]);
    let mut e = engine(&c, 3);
    let p = me(&e);
    plant(&mut e, &c, p, Gravity::Crime, 15);
    let mut saw_investigation = false;
    let mut saw_charge_track = false;
    let mut decisions = Vec::new();
    for _ in 0..60 {
        let st = e.state();
        if st
            .justice
            .open_against(p)
            .any(|x| x.stage == Stage::Investigation)
        {
            saw_investigation = true;
        }
        if st
            .justice
            .open_against(p)
            .any(|x| x.stage == Stage::Charged)
        {
            saw_charge_track |= st.player.as_ref().unwrap().tracks.legal_jeopardy >= 80.0;
        }
        decisions.extend(
            st.narrative
                .pending()
                .filter(|d| d.about.is_some())
                .map(|d| d.template.clone()),
        );
        if !st.player.as_ref().unwrap().status.active() {
            break;
        }
        e.apply(Command::EndTurn).unwrap();
    }
    assert!(saw_investigation && saw_charge_track);
    assert!(decisions.iter().any(|d| d == "dilemma.scandal_response"));
    assert!(decisions.iter().any(|d| d == "dilemma.legal_response"));
    let RunStatus::Imprisoned { until, .. } = e.state().player.as_ref().unwrap().status else {
        panic!(
            "not in prison: {:?}",
            e.state().player.as_ref().unwrap().status
        );
    };
    assert!(!e.state().politics.get(p).unwrap().in_office());
    // In prison, nothing to do but wait; ending the turn skips the sentence.
    assert!(
        e.apply(Command::Player(PlayerCommand::MoveTo {
            location: mandate_sim::places::Location::Home
        }))
        .is_err()
    );
    let jailed = e.state().calendar.date;
    let out = e.apply(Command::EndTurn).unwrap();
    assert!(
        out.days_advanced > 300,
        "a time skip: {}",
        out.days_advanced
    );
    let st = e.state();
    assert!(st.calendar.date >= until);
    // Nothing was put to the player in prison.
    assert!(
        st.narrative
            .decisions
            .iter()
            .all(|d| d.opened <= jailed.add_days(7) || d.opened >= until),
        "a decision opened in prison"
    );
    let pl = st.player.as_ref().unwrap();
    assert_eq!(pl.status, RunStatus::Active);
    assert!(pl.tracks.standing as f64 <= c.justice.release_standing);
    // A comeback path: they may file for their old ward again.
    let r = e.apply(e.route(PlayerCommand::File {
        filing: mandate_sim::politics::elections::seat_filing(e.state(), e.content()).unwrap(),
    }));
    assert!(r.is_ok(), "{r:?}");
}

/// A long sentence ends the career, and only after the Legal Jeopardy
/// warning and its response window (DEC-006 fairness); with no warning
/// possible, the trial waits instead.
#[test]
fn a_warning_precedes_any_career_ending_sentence() {
    let c = quick(&[("justice", "/prison_years", serde_json::json!([8, 8]))]);
    let w = c.director.fairness.response_days;
    for seed in [3u64, 4] {
        let mut e = engine(&c, seed);
        let p = me(&e);
        plant(&mut e, &c, p, Gravity::Crime, 15);
        let ended = turns_until(&mut e, 80, &|e| {
            matches!(
                e.state().player.as_ref().unwrap().status,
                RunStatus::Ended { .. }
            )
        });
        assert!(ended, "seed {seed}: the career did not end");
        let st = e.state();
        let RunStatus::Ended { date, .. } = st.player.as_ref().unwrap().status else {
            unreachable!()
        };
        let warned = st
            .narrative
            .warnings
            .iter()
            .find(|x| x.track == "legal_jeopardy")
            .expect("warned");
        assert!(
            warned.date.add_days(w) <= date,
            "seed {seed}: no time to respond"
        );
        assert!(st.narrative.ending_allowed(&c, "legal_jeopardy", date));
        assert!(e.apply(Command::EndTurn).is_err(), "the run is over");
    }
    // Without a possible warning (the warning line above any case's track),
    // a career-ending trial is adjourned, never held.
    let c = quick(&[
        ("justice", "/prison_years", serde_json::json!([8, 8])),
        (
            "director",
            "/fairness/legal_jeopardy",
            serde_json::json!(99.9),
        ),
    ]);
    let mut e = engine(&c, 3);
    let p = me(&e);
    plant(&mut e, &c, p, Gravity::Crime, 15);
    for _ in 0..40 {
        e.apply(Command::EndTurn).unwrap();
    }
    let st = e.state();
    assert!(st.player.as_ref().unwrap().status.active());
    let case = st.justice.cases.iter().find(|x| x.subject == p).unwrap();
    assert_eq!(case.stage, Stage::Adjourned);
}

/// The other outcomes: a dropped case, an acquittal, and removal from office
/// for misconduct (the seat goes to the player's party).
#[test]
fn cases_can_be_dropped_end_in_acquittal_or_remove_from_office() {
    let run = |c: &Arc<ContentBundle>, sev: Gravity| {
        let mut e = engine(c, 4);
        let p = me(&e);
        plant(&mut e, c, p, sev, 15);
        turns_until(&mut e, 40, &|e| {
            e.state()
                .justice
                .cases
                .iter()
                .any(|x| x.subject == p && !x.stage.open())
        });
        e
    };
    let e = run(
        &quick(&[(
            "justice",
            "/charge",
            serde_json::json!({ "ethics": 0.0, "misconduct": 0.0, "crime": 0.0 }),
        )]),
        Gravity::Crime,
    );
    assert_eq!(e.state().justice.cases[0].stage, Stage::Dropped);
    assert!(e.state().politics.get(me(&e)).unwrap().in_office());
    let e = run(
        &quick(&[("justice", "/convict_base", serde_json::json!(-9.0))]),
        Gravity::Crime,
    );
    assert_eq!(e.state().justice.cases[0].stage, Stage::Acquitted);
    let e = run(
        &quick(&[("justice", "/removal_misconduct", serde_json::json!(1.0))]),
        Gravity::Misconduct,
    );
    let st = e.state();
    let p = me(&e);
    assert!(matches!(
        st.justice.cases[0].sentence,
        Some(Sentence::Removed { .. })
    ));
    assert!(!st.politics.get(p).unwrap().in_office());
    assert!(
        st.player.as_ref().unwrap().status.active(),
        "removal is survivable"
    );
    let party = st.politics.get(p).unwrap().party;
    let ward = st.player.as_ref().unwrap().ward;
    let holder = st
        .politics
        .politicians
        .iter()
        .find(|x| x.role.constituency() == Some(ward))
        .unwrap();
    assert_eq!(holder.party, party, "the party keeps the seat");
}

/// Shady options show their risk before the choice (DEC-073) and apply a
/// real payoff with a hidden fact when chosen.
#[test]
fn shady_actions_pay_now_and_leave_a_hidden_fact() {
    let c = base();
    let mut e = engine(&c, 2);
    let money = e.state().player.as_ref().unwrap().money;
    let hidden = e.state().narrative.next_hidden;
    let r = e.apply(e.route(PlayerCommand::Act {
        action: Action::TakeBribe,
        late_night: true,
    }));
    assert!(r.is_ok(), "{r:?}");
    let st = e.state();
    assert!(st.player.as_ref().unwrap().money > money);
    let h = st.narrative.hidden.iter().find(|h| h.id == hidden).unwrap();
    assert_eq!(h.kind, "bribe");
    assert_eq!(h.severity, Gravity::Crime);
    assert!(h.fate.is_some());
    // Not by day.
    assert!(
        e.apply(e.route(PlayerCommand::Act {
            action: Action::TakeBribe,
            late_night: false,
        }))
        .is_err()
    );
}

/// Determinism with the press and the courts at work: twice, and through
/// save, load, and continue.
#[test]
fn media_play_is_deterministic_through_saves() {
    let c = quick(&[]);
    let play = |turns: u32| {
        let mut e = engine(&c, 21);
        let p = me(&e);
        plant(&mut e, &c, p, Gravity::Misconduct, 20);
        for t in 0..turns {
            if t % 7 == 2 {
                let _ = e.apply(e.route(PlayerCommand::Act {
                    action: Action::LeakFile,
                    late_night: false,
                }));
            }
            let _ = e.apply(Command::EndTurn);
        }
        e
    };
    let a = play(60);
    let b = play(60);
    assert_eq!(a.state_hash(), b.state_hash());
    assert!(!a.state().media.stories.is_empty() && !a.state().justice.cases.is_empty());
    let half = play(30);
    let json = serde_json::to_string(half.state()).unwrap();
    let state: mandate_sim::State = serde_json::from_str(&json).unwrap();
    let mut h = Engine::from_state(state, c.clone()).unwrap();
    for t in 30..60 {
        if t % 7 == 2 {
            let _ = h.apply(h.route(PlayerCommand::Act {
                action: Action::LeakFile,
                late_night: false,
            }));
        }
        let _ = h.apply(Command::EndTurn);
    }
    assert_eq!(a.state_hash(), h.state_hash());
}

/// The members of a town's bodies (the base constitution's council; DEC-098).
fn council_members(st: &mandate_sim::State, m: usize) -> Vec<mandate_sim::politics::PoliticianId> {
    let at = mandate_sim::world::Jurisdiction::town(mandate_sim::world::MunicipalityId(m as u32));
    mandate_sim::politics::body_members_at(&st.politics, at)
}
