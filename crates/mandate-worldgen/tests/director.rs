#![allow(clippy::unwrap_used, clippy::expect_used)]
//! The director's variety and pacing (DEC-097, ENGAGEMENT §4.1–4.4, E2):
//!
//! - cooldowns grow with each use, no template repeats within a chapter,
//!   and no climax repeats one of the last two chapters';
//! - the pacing cycle (build-up, peak, fade, relax) runs under Classic and
//!   is absent under Unfiltered; a relax ends when the player introduces a
//!   bill; a decision raised by a scandal opens during a relax;
//! - `answered` and `sponsored_law` read the player's record;
//! - what the player knows never changes what the director draws.

use mandate_content::{Condition, ContentBundle, LawStatus, bundle::read_dir_sources, load};
use mandate_sim::actions::PlayerCommand;
use mandate_sim::narrative::{
    self, Actor, AnswerRecord, CastEntry, ChapterKind, DecisionStatus, PhaseKind, Weight,
};
use mandate_sim::{Command, Engine};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

fn base() -> Arc<ContentBundle> {
    let src =
        read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap();
    Arc::new(load(&src).unwrap().0)
}

fn engine(c: &Arc<ContentBundle>, seed: u64) -> Engine {
    generate(&WorldConfig::new(seed, WorldSize::Small), c.clone()).unwrap()
}

/// Answer every pending decision with its first non-default option that is
/// neither borderline nor illegal (an honest player who engages).
fn answer_all(e: &mut Engine, c: &ContentBundle) {
    let pending: Vec<_> = e.state().narrative.pending().cloned().collect();
    for d in pending {
        let def = c.dilemma(&d.template).unwrap();
        for o in def.options.iter().filter(|o| {
            !o.default
                && !o
                    .effects
                    .iter()
                    .any(|x| matches!(x, mandate_content::EffectDef::Hidden { .. }))
        }) {
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

/// Advance to the next director day (the weekly run).
fn to_director_day(e: &mut Engine) {
    let d = e.state().calendar.date.0;
    e.apply(Command::AdvanceDays { days: 7 - d % 7 }).unwrap();
}

/// Variety across careers (ENGAGEMENT §4.1): at most three uses of a
/// template in four years, each return after its growing cooldown, no
/// template twice in a chapter, no climax twice in a row.
#[test]
fn variety_rules_hold_over_careers() {
    let c = base();
    let d = &c.director;
    for seed in [3u64, 8, 13, 21] {
        let mut e = engine(&c, seed);
        let mut seen: BTreeMap<u32, (String, Weight, u32, u32, bool)> = BTreeMap::new();
        for _ in 0..(4 * 26) {
            answer_all(&mut e, &c);
            e.apply(Command::EndTurn).unwrap();
            for x in &e.state().narrative.decisions {
                seen.entry(x.id.0).or_insert((
                    x.template.clone(),
                    x.weight,
                    x.opened.0,
                    x.chapter,
                    x.about.is_some(),
                ));
            }
        }
        let spot: Vec<_> = seen.values().filter(|x| !x.4).collect();
        let mut uses: BTreeMap<&str, Vec<u32>> = BTreeMap::new();
        let mut in_chapter: Vec<(u32, &str)> = Vec::new();
        for (t, _, opened, ch, _) in &spot {
            uses.entry(t.as_str()).or_default().push(*opened);
            assert!(
                !in_chapter.contains(&(*ch, t.as_str())),
                "seed {seed}: {t} twice in chapter {ch}"
            );
            in_chapter.push((*ch, t.as_str()));
        }
        for (t, dates) in &uses {
            assert!(
                dates.len() <= 3,
                "seed {seed}: {t} used {} times",
                dates.len()
            );
            let cd = c.dilemma(t).unwrap().cooldown_days;
            for (i, w) in dates.windows(2).enumerate() {
                let need = narrative::director::cooldown_days(d, cd, i as u16 + 1);
                // A storyline's stage presents its decision when the story
                // reaches it (story first); standalone returns wait.
                assert!(
                    w[1] - w[0] >= need || c.dilemma(t).unwrap().storyline_only,
                    "seed {seed}: {t} returned after {} days (cooldown {need})",
                    w[1] - w[0]
                );
            }
        }
        let mut climax: Vec<(u32, &str)> = spot
            .iter()
            .filter(|x| x.1 == Weight::Climax)
            .map(|x| (x.3, x.0.as_str()))
            .collect();
        climax.sort();
        for w in climax.windows(2) {
            assert!(
                w[0].1 != w[1].1 || w[1].0 - w[0].0 > d.climax_memory_chapters as u32,
                "seed {seed}: {} was the climax of chapters {} and {}",
                w[0].1,
                w[0].0,
                w[1].0
            );
        }
        // Every session chapter took a theme the live state supported.
        let n = &e.state().narrative;
        for ch in n.chapters.iter().chain(n.chapter.as_ref()) {
            let theme = ch.theme.as_deref().unwrap();
            match ch.kind {
                ChapterKind::Campaign => assert_eq!(theme, "campaign"),
                ChapterKind::Session => assert!(d.themes.contains_key(theme), "{theme}"),
            }
        }
    }
}

/// Classic paces in cycles; Unfiltered has no phases (pure chance within
/// the budgets, as before the cycle).
#[test]
fn classic_paces_in_cycles_and_unfiltered_has_no_phases() {
    let c = base();
    let mut classic = engine(&c, 5);
    let mut unfiltered = engine(&c, 5);
    unfiltered.setup(|s| s.narrative.profile = "storyteller.unfiltered".into());
    for _ in 0..(2 * 26) {
        answer_all(&mut classic, &c);
        answer_all(&mut unfiltered, &c);
        classic.apply(Command::EndTurn).unwrap();
        unfiltered.apply(Command::EndTurn).unwrap();
    }
    let u = &unfiltered.state().narrative;
    assert!(u.phase.is_none() && u.phase_log.is_empty() && u.setback.is_none());
    assert!(!u.decisions.is_empty());
    let n = &classic.state().narrative;
    assert!(n.phase.is_some());
    let kinds: Vec<PhaseKind> = n.phase_log.iter().map(|p| p.kind).collect();
    // Phases follow each other in order.
    for w in kinds.windows(2) {
        let next = match w[0] {
            PhaseKind::BuildUp => vec![PhaseKind::Peak],
            PhaseKind::Peak => vec![PhaseKind::PeakFade],
            PhaseKind::PeakFade => vec![PhaseKind::Relax],
            // A forced climax may cut a relax short.
            PhaseKind::Relax => vec![PhaseKind::BuildUp, PhaseKind::Peak],
        };
        assert!(next.contains(&w[1]), "{:?} then {:?}", w[0], w[1]);
    }
    // Significant decisions (storyline stages included) open only at peaks;
    // raised ones (a story, a case) open whenever the world raises them.
    for d in n
        .decisions
        .iter()
        .filter(|d| d.weight != Weight::Minor && d.about.is_none())
    {
        let at = n
            .phase_log
            .iter()
            .chain(n.phase.as_ref())
            .find(|p| p.since <= d.opened && p.until.is_none_or(|u| d.opened < u))
            .map(|p| p.kind);
        assert_eq!(at, Some(PhaseKind::Peak), "{} opened in {at:?}", d.template);
    }
    let peaks = kinds.iter().filter(|k| **k == PhaseKind::Peak).count();
    // A cycle is about 60–110 days.
    assert!((5..=13).contains(&peaks), "peaks in two years: {peaks}");
    // A session runs over the budget year of the player's main post.
    let st = classic.state();
    let r = mandate_sim::scheduler::Reads::of_state(st, &c);
    let v = narrative::cast::View::new(&r, &st.politics);
    // (Out of office, their home town's.)
    let expect = mandate_sim::player::posts(st)
        .first()
        .map(|p| p.at())
        .or(mandate_sim::player::home(st).map(mandate_sim::world::Jurisdiction::town));
    assert_eq!(narrative::director::session_jurisdiction(&v), expect);
}

/// Run a Classic career on director days until `want` holds.
fn until_phase(e: &mut Engine, c: &ContentBundle, want: &dyn Fn(&Engine) -> bool) {
    for _ in 0..400 {
        if want(e) {
            return;
        }
        answer_all(e, c);
        to_director_day(e);
    }
    panic!("the phase never came");
}

/// A relax ends when the player starts something: introducing a bill.
#[test]
fn a_relax_ends_when_the_player_introduces_a_bill() {
    let c = base();
    let mut e = engine(&c, 9);
    // A relax with at least two more weeks to run.
    until_phase(&mut e, &c, &|e| {
        let st = e.state();
        st.narrative.phase.as_ref().is_some_and(|p| {
            p.kind == PhaseKind::Relax
                && p.since < st.calendar.date
                && p.until.is_some_and(|u| u.0 >= st.calendar.date.0 + 14)
        })
    });
    let relax = e.state().narrative.phase.clone().unwrap();
    let t = c
        .policies
        .values()
        .find(|t| t.tier == mandate_content::TierKey::Local)
        .unwrap()
        .id
        .clone();
    let id = e
        .apply(e.route(PlayerCommand::DraftPolicy {
            template: t,
            params: Default::default(),
        }))
        .unwrap()
        .result
        .unwrap()
        .created
        .unwrap();
    e.apply(e.route(PlayerCommand::IntroducePolicy { policy: id }))
        .unwrap();
    to_director_day(&mut e);
    let n = &e.state().narrative;
    let ended = n
        .phase_log
        .iter()
        .rev()
        .find(|p| p.kind == PhaseKind::Relax && p.since == relax.since)
        .expect("the relax ended");
    assert!(ended.until.unwrap() < relax.until.unwrap());
    assert_eq!(n.phase.as_ref().unwrap().kind, PhaseKind::BuildUp);
}

/// The world is never hidden (DEC-033 B): a story about the player raises
/// its decision during a relax, and the director eases off afterwards.
#[test]
fn a_scandal_raises_its_decision_during_a_relax() {
    let c = base();
    let mut e = engine(&c, 4);
    until_phase(&mut e, &c, &|e| {
        let st = e.state();
        st.narrative.phase.as_ref().is_some_and(|p| {
            p.kind == PhaseKind::Relax && p.until.is_some_and(|u| u.0 >= st.calendar.date.0 + 10)
        }) && st.narrative.pending().count() == 0
    });
    e.setup(|s| {
        let me = s.player.as_ref().unwrap().politician;
        let town = s.player.as_ref().unwrap().home().0;
        let date = s.calendar.date;
        let id = s.media.scandals.len() as u32;
        s.media.scandals.push(mandate_sim::media::Scandal {
            id,
            hidden: 0,
            subject: me,
            kind: "back_room_deal".into(),
            severity: mandate_content::Gravity::Ethics,
            evidence: 0.5,
            town,
            broke: date,
            by: mandate_content::Discoverer::Journalist,
            who: None,
            outlet: None,
            heat: 1.0,
            stories: 1,
            response: None,
            responded: None,
            backfire_on: None,
            ended: None,
            leaked: false,
        });
    });
    to_director_day(&mut e);
    let n = &e.state().narrative;
    let raised: Vec<_> = n.pending().filter(|d| d.about.is_some()).collect();
    assert_eq!(raised.len(), 1, "the story raised its decision");
    assert_eq!(n.phase.as_ref().unwrap().kind, PhaseKind::Relax);
    assert!(
        n.setback.is_some(),
        "a scandal about the player is a setback"
    );
}

/// `answered` reads the player's answers (window, option, carried cast);
/// `sponsored_law` reads their laws, the same law giving the same answer.
#[test]
fn answered_and_sponsored_law_read_the_players_record() {
    let c = base();
    let mut e = engine(&c, 6);
    e.apply(Command::AdvanceDays { days: 30 }).unwrap();
    let me = e.state().player.as_ref().unwrap().politician;
    let date = e.state().calendar.date;
    let voice = Actor::Person {
        id: mandate_sim::characters::CharacterId(3),
    };
    let law = e.state().policy.instances.len() as u32;
    e.setup(|s| {
        s.narrative.answers.push(AnswerRecord {
            decision: narrative::DecisionId(900),
            template: "dilemma.jobs_plea".into(),
            option: "wage_subsidy".into(),
            date: mandate_core::SimDate(date.0 - 200),
            by_default: false,
            cast: vec![CastEntry {
                role: "voice".into(),
                actor: voice,
            }],
        });
        let mut p = s.policy.instances[0].clone();
        p.id = mandate_sim::policy::PolicyInstanceId(law);
        p.sponsor = Some(me);
        p.status = mandate_sim::policy::PolicyStatus::InEffect;
        p.enacted_on = Some(mandate_core::SimDate(date.0 - 400));
        p.introduced_on = Some(mandate_core::SimDate(date.0 - 430));
        s.policy.instances.push(p);
    });
    let st = e.state();
    let r = mandate_sim::scheduler::Reads::of_state(st, &c);
    let v = narrative::cast::View::new(&r, &st.politics);
    let holds = |cond: &Condition, cast: &[CastEntry]| narrative::cast::holds(&v, cond, cast);
    let answered =
        |option: Option<&str>, min: u32, max: Option<u32>, same: &[&str]| Condition::Answered {
            template: "dilemma.jobs_plea".into(),
            option: option.map(String::from),
            min_days: min,
            max_days: max,
            same_cast: same.iter().map(|s| s.to_string()).collect(),
        };
    let with_voice = [CastEntry {
        role: "voice".into(),
        actor: voice,
    }];
    assert!(holds(
        &answered(Some("wage_subsidy"), 120, Some(720), &[]),
        &[]
    ));
    assert!(holds(&answered(None, 0, None, &[]), &[]));
    assert!(!holds(&answered(Some("retrain"), 0, None, &[]), &[]));
    assert!(!holds(&answered(None, 300, None, &[]), &[]), "too recent");
    assert!(
        !holds(&answered(None, 0, Some(100), &[]), &[]),
        "too long ago"
    );
    assert!(holds(&answered(None, 0, None, &["voice"]), &with_voice));
    assert!(
        !holds(&answered(None, 0, None, &["voice"]), &[]),
        "another voice"
    );
    let law_cond = |min: u32, max: Option<u32>, chance: f64| Condition::SponsoredLaw {
        status: LawStatus::InForce,
        min_age_days: min,
        max_age_days: max,
        chance,
        role: None,
    };
    assert!(holds(&law_cond(365, Some(730), 1.0), &[]));
    assert!(!holds(&law_cond(500, None, 1.0), &[]), "not old enough");
    assert!(!holds(&law_cond(0, None, 0.0), &[]), "chance 0");
    let pending = Condition::SponsoredLaw {
        status: LawStatus::Pending,
        min_age_days: 0,
        max_age_days: None,
        chance: 1.0,
        role: None,
    };
    let any_pending = st.policy.instances.iter().any(|p| {
        p.sponsor == Some(me) && p.status == mandate_sim::policy::PolicyStatus::UnderConsideration
    });
    assert_eq!(holds(&pending, &[]), any_pending);
    // The law cast in a role, and the keyed chance: always the same answer.
    let cast = [CastEntry {
        role: "law".into(),
        actor: Actor::Policy { id: law },
    }];
    let half = Condition::SponsoredLaw {
        status: LawStatus::InForce,
        min_age_days: 0,
        max_age_days: None,
        chance: 0.5,
        role: Some("law".into()),
    };
    let first = holds(&half, &cast);
    for _ in 0..5 {
        assert_eq!(holds(&half, &cast), first);
    }
    let other = [CastEntry {
        role: "law".into(),
        actor: Actor::Policy { id: 0 },
    }];
    assert!(!holds(&half, &other) || st.policy.instances[0].sponsor == Some(me));
}

/// The director's draws are keyed: what the player knows (the lab's
/// omniscient switch) never changes what it draws or spotlights.
#[test]
fn what_the_player_knows_never_changes_the_draws() {
    let c = base();
    let mut blind = engine(&c, 11);
    let mut knowing = engine(&c, 11);
    knowing.set_lab_omniscient(true);
    for _ in 0..(2 * 26) {
        answer_all(&mut blind, &c);
        answer_all(&mut knowing, &c);
        blind.apply(Command::EndTurn).unwrap();
        knowing.apply(Command::EndTurn).unwrap();
    }
    let a = serde_json::to_value(&blind.state().narrative).unwrap();
    let b = serde_json::to_value(&knowing.state().narrative).unwrap();
    assert_eq!(a, b);
    assert_eq!(
        serde_json::to_value(&blind.state().rng).unwrap(),
        serde_json::to_value(&knowing.state().rng).unwrap()
    );
    let n = &blind.state().narrative;
    assert!(
        n.decisions
            .iter()
            .any(|d| d.status != DecisionStatus::Pending)
    );
    // The lab's view of a week's candidates is pure.
    let before = blind.state_hash();
    let _ = narrative::director::week_candidates(blind.state(), &c);
    assert_eq!(blind.state_hash(), before);
}
