#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Voters weigh their own district's worries (DEC-092 note, N2): issue
//! salience is computed per constituency from its neighbourhoods' concern
//! levels (each town's measured levels shifted by how much more or less the
//! constituency feels each worry), so a jobs platform plays better on the
//! estate than in the suburbs. A whole town (a mayoral race, a town agenda)
//! reads the town's own figure; constituencies of any tier work the same.

use mandate_content::{ContentBundle, bundle::read_dir_sources, load};
use mandate_sim::concerns::{compute_all, political_levels, salience, salience_from, salience_in};
use mandate_sim::politics::elections::{Candidate, Race, expected_shares};
use mandate_sim::politics::{PoliticianId, Post};
use mandate_sim::scheduler::Reads;
use mandate_sim::world::{Jurisdiction, MunicipalityId, NeighborhoodKind, RegionId, Tier};
use mandate_sim::{Command, Engine, State};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::path::Path;
use std::sync::Arc;

fn base() -> Arc<ContentBundle> {
    let src =
        read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap();
    Arc::new(load(&src).unwrap().0)
}

fn engine(seed: u64, size: WorldSize, turns: u32) -> Engine {
    let mut e = generate(
        &WorldConfig {
            seed,
            size,
            scenario: None,
        },
        base(),
    )
    .unwrap();
    for _ in 0..turns {
        e.apply(Command::EndTurn).unwrap();
    }
    e
}

/// Recompute every concern from the state as it stands (as the monthly
/// refresh does).
fn recompute(e: &mut Engine) {
    let c = e.content().clone();
    e.setup(|s| {
        let (towns, hoods) = compute_all(&Reads::of_state(s, &c), &s.population);
        s.population.town_concerns = towns;
        s.population.hood_concerns = hoods;
    });
}

/// Each local ward of a town: its neighbourhoods (world indices).
fn wards(st: &State, m: usize) -> Vec<Vec<usize>> {
    Jurisdiction::town(MunicipalityId(m as u32))
        .constituencies(&st.world)
        .map(|w| w.neighborhoods.iter().map(|h| h.index()).collect())
        .collect()
}

fn town_hoods(st: &State, m: usize) -> Vec<usize> {
    st.world.hoods_of(MunicipalityId(m as u32)).collect()
}

/// How much a constituency weighs a concern's issues: the concern's
/// attitude weights' share of its salience.
fn concern_salience(c: &ContentBundle, s: &[f32], concern: usize) -> f64 {
    let w = &c.concerns[concern].attitude_weights;
    let norm: f64 = w.iter().map(|x| x.abs()).sum();
    w.iter()
        .zip(s)
        .map(|(w, s)| w.abs() * *s as f64)
        .sum::<f64>()
        / norm
}

fn kind_share(st: &State, hoods: &[usize], kind: NeighborhoodKind) -> f64 {
    let pop = &st.population;
    let all: f64 = hoods.iter().map(|&h| pop.hood_total(h) as f64).sum();
    let of: f64 = hoods
        .iter()
        .filter(|&&h| st.world.neighborhoods[h].kind == kind)
        .map(|&h| pop.hood_total(h) as f64)
        .sum();
    of / all.max(1.0)
}

/// A non-partisan two-way race over `hoods` for `seat`: two otherwise equal
/// candidates, the first at the voters' mean (in step with them on every
/// issue), the second the same but `shift` away along attitude `a` (out of
/// step on that one issue). Returns the state with the candidates set up,
/// and the race.
fn single_issue_race(
    st: &State,
    seat: Post,
    hoods: &[usize],
    a: usize,
    shift: f32,
) -> (State, Race) {
    let mut st = st.clone();
    let mean = mandate_sim::politics::mean_attitudes(&st.population, hoods);
    let ids: Vec<PoliticianId> = st
        .politics
        .politicians
        .iter()
        .filter(|p| !p.is_player)
        .take(2)
        .map(|p| p.id)
        .collect();
    for (k, id) in ids.iter().enumerate() {
        let p = st.politics.get_mut(*id).unwrap();
        p.attitudes = mean.clone();
        if k == 1 {
            p.attitudes[a] = (p.attitudes[a] + shift).clamp(-1.0, 1.0);
        }
        p.party = None;
        p.oratory = 0.5;
        p.integrity = 0.5;
        p.scandal = 0.0;
        p.record = 0.5;
        p.home = u32::MAX;
    }
    let n = hoods.len();
    let race = Race {
        seat,
        opened: st.calendar.date,
        election_day: st.calendar.date,
        hoods: hoods.iter().map(|h| *h as u32).collect(),
        incumbent: None,
        candidates: ids
            .iter()
            .map(|id| Candidate {
                politician: *id,
                party: None,
                awareness: vec![0.5; n],
                awareness_start: vec![0.5; n],
                persuasion: vec![0.0; n],
                mobilisation: vec![0.0; n],
                contacts: vec![0.0; n],
                spent: 0.0,
            })
            .collect(),
        partisan: false,
        shock: vec![0.0; 2],
        poll_bias_z: vec![0.0; 2],
    };
    (st, race)
}

/// The expected share of a single-issue platform: the candidate in step
/// with the voters on attitude `a` against one 0.5 out of step on it (and
/// equal on everything else). The more the constituency weighs `a`, the
/// more the platform wins by.
fn platform_share(st: &State, c: &ContentBundle, seat: Post, hoods: &[usize], a: usize) -> f64 {
    let (s, race) = single_issue_race(st, seat, hoods, a, 0.5);
    expected_shares(&s.politics, &Reads::of_state(&s, c), &race)[0]
}

fn ward_seat(st: &State, m: usize, ward: usize) -> Post {
    let at = Jurisdiction::town(MunicipalityId(m as u32));
    let body = st
        .politics
        .bodies
        .iter()
        .find(|b| b.at == at)
        .map(|b| b.body)
        .unwrap();
    let id = at.constituencies(&st.world).nth(ward).unwrap().id;
    Post::Seat {
        body,
        at,
        constituency: Some(id),
    }
}

#[test]
fn an_industrial_ward_weighs_jobs_above_a_suburban_ward_of_the_same_town() {
    let c = base();
    let jobs = c.concern_index("concern.jobs").unwrap();
    let manufacturing = c
        .sectors
        .iter()
        .position(|s| s.id == "sector.manufacturing")
        .unwrap();
    // A town with a mostly industrial ward and a mostly suburban one.
    let mut found = None;
    'seeds: for seed in [4242u64, 777, 2026, 61, 5, 6] {
        let e = engine(seed, WorldSize::Medium, 0);
        let st = e.state();
        for m in 0..st.world.municipalities.len() {
            let ws = wards(st, m);
            let ind = ws
                .iter()
                .position(|w| kind_share(st, w, NeighborhoodKind::Industrial) > 0.5);
            let sub = ws
                .iter()
                .position(|w| kind_share(st, w, NeighborhoodKind::Suburb) > 0.5);
            if let (Some(i), Some(s)) = (ind, sub) {
                found = Some((e, m, i, s));
                break 'seeds;
            }
        }
    }
    let (mut e, m, ind, sub) = found.expect("a town with an industrial and a suburban ward");
    // The town's manufacturing loses a third of its jobs.
    e.setup(|s| {
        let sec = &mut s.economy.munis[m].sectors[manufacturing];
        sec.jobs = sec.baseline_jobs as f64 * 0.66;
    });
    recompute(&mut e);
    let st = e.state();
    let ws = wards(st, m);
    let pop = &st.population;
    let li = political_levels(pop, &c, &st.world, &ws[ind]);
    let ls = political_levels(pop, &c, &st.world, &ws[sub]);
    assert!(
        li[jobs] > ls[jobs],
        "jobs level {} vs {}",
        li[jobs],
        ls[jobs]
    );
    let si = salience_in(pop, &c, &st.world, &ws[ind]).unwrap();
    let ss = salience_in(pop, &c, &st.world, &ws[sub]).unwrap();
    let (ji, js) = (
        concern_salience(&c, &si, jobs),
        concern_salience(&c, &ss, jobs),
    );
    assert!(ji > js, "jobs salience industrial {ji} vs suburban {js}");
    // Salience keeps a mean of 1 in every ward.
    for s in [&si, &ss] {
        let mean = s.iter().map(|x| *x as f64).sum::<f64>() / s.len() as f64;
        assert!((mean - 1.0).abs() < 1e-5, "{mean}");
    }
}

#[test]
fn a_town_wide_race_reproduces_the_town_salience() {
    let mut e = engine(777, WorldSize::Medium, 24);
    recompute(&mut e);
    let st = e.state();
    let c = e.content();
    let pop = &st.population;
    let mut differs = 0;
    for m in 0..st.world.municipalities.len() {
        let all = town_hoods(st, m);
        let town = &pop.town_concerns[m];
        // Exactly the town's figures, in any order of its neighbourhoods.
        assert_eq!(
            salience_in(pop, c, &st.world, &all).unwrap(),
            salience(pop, m).unwrap()
        );
        let mut rev = all.clone();
        rev.reverse();
        assert_eq!(salience_in(pop, c, &st.world, &rev).unwrap(), town.salience);
        let measured: Vec<f64> = town.measured.iter().map(|l| *l as f64).collect();
        assert_eq!(political_levels(pop, c, &st.world, &all), measured);
        // The town agenda reads the town; a ward agenda its ward.
        for t in c.policies.values().take(3) {
            let town_bonus = mandate_sim::npc::agenda_bonus(st, c, m as u32, t);
            let all_bonus = mandate_sim::npc::agenda_bonus_in(st, c, m as u32, &all, t);
            assert_eq!(town_bonus, all_bonus);
        }
        for w in wards(st, m) {
            let s = salience_in(pop, c, &st.world, &w).unwrap();
            assert_eq!(s.len(), town.salience.len());
            if s != town.salience {
                differs += 1;
            }
        }
    }
    assert!(differs > 0, "every ward weighs issues as its town does");
}

#[test]
fn the_vote_model_weighs_a_wards_voters_by_the_wards_worries() {
    let mut e = engine(777, WorldSize::Medium, 24);
    recompute(&mut e);
    let c = e.content().clone();
    let env_concern = c.concern_index("concern.environment").unwrap();
    let env = c
        .attitudes
        .iter()
        .position(|a| a.id == "attitude.environment")
        .unwrap();
    let st = e.state().clone();
    let m = (0..st.world.municipalities.len())
        .find(|&m| wards(&st, m).len() >= 2)
        .unwrap();
    let ws = wards(&st, m);
    let all = town_hoods(&st, m);
    let at = Jurisdiction::town(MunicipalityId(m as u32));
    let mayor = st
        .politics
        .offices
        .iter()
        .find(|o| o.at == at)
        .map(|o| Post::Office {
            office: o.office,
            at,
        })
        .unwrap();
    let seat = ward_seat(&st, m, 0);
    let ward_before = platform_share(&st, &c, seat, &ws[0], env);
    let town_before = platform_share(&st, &c, mayor, &all, env);
    // The first ward alone comes to dread pollution; the town's own
    // (measured) figures are untouched.
    e.setup(|s| {
        for &h in &ws[0] {
            s.population.hood_concerns[h].levels[env_concern] = 1.0;
        }
    });
    let st = e.state().clone();
    let s0 = salience_in(&st.population, &c, &st.world, &ws[0]).unwrap();
    let s1 = salience_in(&st.population, &c, &st.world, &ws[1]).unwrap();
    assert!(s0[env] > s1[env], "{} vs {}", s0[env], s1[env]);
    let ward_after = platform_share(&st, &c, seat, &ws[0], env);
    let town_after = platform_share(&st, &c, mayor, &all, env);
    // An environment platform wins the ward by more once it dreads pollution.
    assert!(
        ward_before > 0.5 && ward_after > ward_before,
        "the ward's voters weigh its worry: {ward_before} → {ward_after}"
    );
    assert_eq!(town_before, town_after, "a town-wide race reads the town");
}

#[test]
fn constituencies_of_any_tier_weigh_their_towns_by_residents() {
    // A region's (or the nation's) constituency covers whole towns: its
    // levels are their measured levels weighted by residents, and a part of
    // one town shifts that town's share only.
    let mut e = engine(2026, WorldSize::Medium, 12);
    recompute(&mut e);
    let st = e.state();
    let c = e.content();
    let pop = &st.population;
    let region = (0..st.world.regions.len())
        .find(|&r| st.world.regions[r].municipalities.len() >= 2)
        .unwrap();
    let at = Jurisdiction::of(Tier::Regional.into(), region as u32);
    assert_eq!(
        at,
        Jurisdiction::Region {
            id: RegionId(region as u32)
        }
    );
    let hoods = at.hoods(&st.world);
    let towns = &st.world.regions[region].municipalities;
    let mut sum = vec![0.0; c.concerns.len()];
    let mut w = 0.0;
    for t in towns {
        let n = pop.municipality_total(&st.world, *t) as f64;
        for (s, l) in sum.iter_mut().zip(&pop.town_concerns[t.index()].measured) {
            *s += n * *l as f64;
        }
        w += n;
    }
    let expect: Vec<f64> = sum.iter().map(|s| s / w).collect();
    let got = political_levels(pop, c, &st.world, &hoods);
    for (a, b) in got.iter().zip(&expect) {
        assert!((a - b).abs() < 1e-9, "{a} vs {b}");
    }
    assert_eq!(
        salience_in(pop, c, &st.world, &hoods).unwrap(),
        salience_from(c, &got)
    );
    // The nation, too.
    let nation = Jurisdiction::Nation.hoods(&st.world);
    assert_eq!(
        political_levels(pop, c, &st.world, &nation).len(),
        c.concerns.len()
    );
    // No neighbourhoods: no salience (every issue weighs the same).
    assert!(salience_in(pop, c, &st.world, &[]).is_none());
}

#[test]
fn ward_salience_is_deterministic_and_survives_a_save() {
    let digest = |st: &State, c: &ContentBundle| -> Vec<Vec<u32>> {
        st.world
            .constituencies
            .iter()
            .map(|w| {
                let hs: Vec<usize> = w.neighborhoods.iter().map(|h| h.index()).collect();
                salience_in(&st.population, c, &st.world, &hs)
                    .unwrap()
                    .iter()
                    .map(|x| x.to_bits())
                    .collect()
            })
            .collect()
    };
    let a = engine(2026, WorldSize::Medium, 14);
    let b = engine(2026, WorldSize::Medium, 14);
    let c = a.content().clone();
    let da = digest(a.state(), &c);
    assert!(!da.is_empty());
    assert_eq!(da, digest(b.state(), &c));
    // Derived from saved state: a round trip of the state gives the same.
    let json = serde_json::to_string(a.state()).unwrap();
    let back: State = serde_json::from_str(&json).unwrap();
    let e = Engine::from_state(back, Arc::new(c.clone())).unwrap();
    assert_eq!(da, digest(e.state(), &c));
}

/// Lab evidence (N2): for seeds 777, 2026 and 4242, each ward of the
/// player's town: its top three issues by salience, and the expected share
/// of a single-issue platform on three issues (`platform_share`). Run with
/// `cargo test -p mandate-worldgen --test ward_salience -- --ignored --nocapture`.
#[test]
#[ignore]
fn evidence_ward_salience_and_single_issue_shares() {
    for seed in [777u64, 2026, 4242] {
        let e = engine(seed, WorldSize::Medium, 0);
        let st = e.state();
        let c = e.content();
        let pop = &st.population;
        let m = st.player.as_ref().unwrap().home().index();
        let attr = |id: &str| c.attitudes.iter().position(|a| a.id == id).unwrap();
        let issues = [
            ("redistribution", attr("attitude.redistribution")),
            ("environment", attr("attitude.environment")),
            ("tax_tolerance", attr("attitude.tax_tolerance")),
        ];
        let top3 = |s: &[f32]| -> String {
            let mut v: Vec<(usize, f32)> = s.iter().copied().enumerate().collect();
            v.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
            v.iter()
                .take(3)
                .map(|(i, x)| {
                    let id = c.attitudes[*i].id.as_str();
                    format!("{} {x:.3}", id.trim_start_matches("attitude."))
                })
                .collect::<Vec<_>>()
                .join(", ")
        };
        println!(
            "seed {seed} town {m} ({}): town salience top-3 [{}]",
            st.world.municipalities[m].name,
            top3(&pop.town_concerns[m].salience)
        );
        for (wi, w) in wards(st, m).iter().enumerate() {
            let s = salience_in(pop, c, &st.world, w).unwrap();
            let kinds: Vec<String> = w
                .iter()
                .map(|h| format!("{:?}", st.world.neighborhoods[*h].kind))
                .collect();
            let seat = ward_seat(st, m, wi);
            let shares: Vec<String> = issues
                .iter()
                .map(|(n, a)| format!("{n} {:.4}", platform_share(st, c, seat, w, *a)))
                .collect();
            println!(
                "  ward {} [{}]: top-3 [{}]; single-issue share: {}",
                wi + 1,
                kinds.join("/"),
                top3(&s),
                shares.join(", ")
            );
        }
    }
}
