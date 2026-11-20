//! M4 election pipeline: the burn-in holds real elections (DEC-022, DEC-028).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use mandate_content::{ContentBundle, bundle::read_dir_sources, load};
use mandate_sim::Engine;
use mandate_sim::politics::Role;
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::path::Path;
use std::sync::{Arc, OnceLock};

fn content() -> Arc<ContentBundle> {
    static C: OnceLock<Arc<ContentBundle>> = OnceLock::new();
    C.get_or_init(|| {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
        Arc::new(load(&read_dir_sources(&root).unwrap()).unwrap().0)
    })
    .clone()
}

fn world(seed: u64) -> Engine {
    generate(&WorldConfig::new(seed, WorldSize::Small), content()).unwrap()
}

#[derive(Debug, Default)]
struct Stats {
    races: usize,
    incumbent_ran: usize,
    incumbent_won: usize,
    mayor_incumbent_ran: usize,
    mayor_incumbent_won: usize,
    turnout_sum: f64,
    seats_by_party: Vec<usize>,
    margin_sum: f64,
    elections: usize,
}

fn stats(e: &Engine) -> Stats {
    let st = e.state();
    let mut s = Stats {
        seats_by_party: vec![0; st.politics.parties.len()],
        ..Default::default()
    };
    let mut dates: Vec<_> = st
        .politics
        .elections
        .results
        .iter()
        .map(|r| r.date)
        .collect();
    dates.dedup();
    s.elections = dates.len();
    for r in &st.politics.elections.results {
        s.races += 1;
        s.turnout_sum += r.turnout as f64;
        let mut sh: Vec<f32> = r.candidates.iter().map(|c| c.share).collect();
        sh.sort_by(|a, b| b.total_cmp(a));
        s.margin_sum += (sh[0] - sh.get(1).copied().unwrap_or(0.0)) as f64;
        if let Some(i) = r.incumbent
            && r.candidates.iter().any(|c| c.politician == i)
        {
            let mayor = r.seat.is_office();
            s.incumbent_ran += 1;
            if mayor {
                s.mayor_incumbent_ran += 1;
            }
            if r.winner == i {
                s.incumbent_won += 1;
                if mayor {
                    s.mayor_incumbent_won += 1;
                }
            }
        }
        if dates.last() == Some(&r.date)
            && let Some(p) = r
                .candidates
                .iter()
                .find(|c| c.politician == r.winner)
                .and_then(|c| c.party)
        {
            s.seats_by_party[p.index()] += 1;
        }
    }
    s
}

#[test]
fn burn_in_holds_plausible_elections() {
    for seed in [11, 12, 13] {
        let e = world(seed);
        let s = stats(&e);
        let reelect = s.incumbent_won as f64 / s.incumbent_ran.max(1) as f64;
        let turnout = s.turnout_sum / s.races.max(1) as f64;
        let margin = s.margin_sum / s.races.max(1) as f64;
        println!(
            "seed {seed}: elections {} races {} reelect {:.2} (mayors {}/{}) turnout {:.2} margin {:.2} seats {:?} parties {:?}",
            s.elections,
            s.races,
            reelect,
            s.mayor_incumbent_won,
            s.mayor_incumbent_ran,
            turnout,
            margin,
            s.seats_by_party,
            e.state()
                .politics
                .parties
                .iter()
                .map(|p| p.name.clone())
                .collect::<Vec<_>>()
        );
        assert!(
            s.elections >= 3,
            "burn-in should hold several local elections"
        );
        assert!(
            (0.5..=0.92).contains(&reelect),
            "incumbent re-election rate {reelect}"
        );
        assert!((0.25..=0.8).contains(&turnout), "turnout {turnout}");
        assert!(
            s.seats_by_party.iter().take(2).all(|n| *n > 0),
            "both major parties hold seats"
        );
    }
}

#[test]
fn councils_and_roles_stay_consistent() {
    let e = world(14);
    let st = e.state();
    // Every body's members and every office's holder hold that post (DEC-098).
    for b in &st.politics.bodies {
        for id in &b.members {
            let p = st.politics.get(*id).unwrap();
            assert!(
                matches!(p.role, Role::Seat { body, at, .. } if body == b.body && at == b.at),
                "{:?}",
                p.role
            );
        }
    }
    for o in &st.politics.offices {
        let holder = st.politics.get(o.holder.unwrap()).unwrap();
        assert!(
            matches!(holder.role, Role::Office { office, at } if office == o.office && at == o.at)
        );
    }
    let seated = st
        .politics
        .politicians
        .iter()
        .filter(|p| p.in_office())
        .count();
    let seats: usize = st
        .politics
        .bodies
        .iter()
        .map(|b| b.members.len())
        .sum::<usize>()
        + st.politics.offices.len();
    assert_eq!(seated, seats, "every officeholder holds exactly one seat");
    // The player sits on a council after the burn-in.
    let pl = st.player.as_ref().unwrap();
    assert!(st.politics.get(pl.politician).unwrap().is_player);
}

#[test]
#[ignore]
fn debug_party_shares() {
    let e = world(13);
    let st = e.state();
    let np = st.politics.parties.len();
    let mut acc = vec![0.0f64; np];
    let mut n = 0.0;
    for h in &st.population.hoods {
        for c in &h.cohorts {
            for (i, v) in c.party_id.iter().enumerate() {
                acc[i] += *v as f64 * c.count as f64;
            }
            n += c.count as f64;
        }
    }
    println!(
        "attachment {:?}",
        acc.iter().map(|a| a / n).collect::<Vec<_>>()
    );
    for p in &st.politics.parties {
        println!("{} {:?}", p.name, p.platform);
    }
    let last = st.politics.elections.results.last().unwrap().date;
    let mut ran = vec![0; np];
    for r in st
        .politics
        .elections
        .results
        .iter()
        .filter(|r| r.date == last)
    {
        for c in &r.candidates {
            if let Some(p) = c.party {
                ran[p.index()] += 1;
            }
        }
    }
    println!("candidates per party {:?}", ran);
}

#[test]
#[ignore]
fn debug_medium_world_policies() {
    // Time with `time cargo test … -- --ignored` (no wall clock in the workspace, DEC-041).
    let e = generate(&WorldConfig::new(5, WorldSize::Medium), content()).unwrap();
    let st = e.state();
    let mut by_status = std::collections::BTreeMap::new();
    for p in &st.policy.instances {
        *by_status.entry(format!("{:?}", p.status)).or_insert(0) += 1;
    }
    println!(
        "policies {:?} politicians {}",
        by_status,
        st.politics.politicians.len()
    );
}

#[test]
fn player_runs_a_full_local_election() {
    use mandate_sim::Command;
    use mandate_sim::state::GamePhase;
    let mut e = world(21);
    let me = e.state().player.as_ref().unwrap().politician;
    let mut saw_campaign = false;
    let mut saw_polls = false;
    let mut turns = 0;
    while e
        .state()
        .politics
        .elections
        .last_election
        .is_none_or(|d| d < mandate_core::SimDate::from_ymd(2020, 1, 1))
    {
        e.apply(Command::EndTurn).unwrap();
        turns += 1;
        let st = e.state();
        if st.calendar.phase == GamePhase::Campaign {
            saw_campaign = true;
            assert!(
                st.politics
                    .elections
                    .races
                    .iter()
                    .any(|r| r.candidates.iter().any(|c| c.politician == me))
            );
        }
        saw_polls |= !st.politics.elections.polls.is_empty();
        assert!(turns < 60, "election should arrive within the first year");
    }
    // Let the monthly player update apply the result.
    for _ in 0..3 {
        e.apply(Command::EndTurn).unwrap();
    }
    let st = e.state();
    assert!(saw_campaign && saw_polls);
    assert_eq!(st.calendar.phase, GamePhase::Governing);
    let res = st
        .politics
        .elections
        .results
        .iter()
        .rev()
        .find(|r| r.candidates.iter().any(|c| c.politician == me))
        .expect("the player ran");
    assert_eq!(
        res.decomposition.len(),
        6,
        "player-era results are explained"
    );
    assert!(
        res.candidates.iter().all(|c| c.final_poll.is_some()),
        "the player's race was polled"
    );
    assert_eq!(
        st.player.as_ref().unwrap().last_election_seen,
        st.politics.elections.last_election
    );
    println!(
        "turns {turns}, player {} with {:?}, decomposition {:?}",
        if res.winner == me { "won" } else { "lost" },
        res.candidates
            .iter()
            .map(|c| (c.share, c.final_poll))
            .collect::<Vec<_>>(),
        res.decomposition
    );
}
