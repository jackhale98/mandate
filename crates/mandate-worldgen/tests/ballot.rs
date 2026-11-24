//! Filing for a post and the party's nomination (play note 20): a refused
//! nomination leaves the player standing for the seat they hold, as if they
//! had not filed, and an event says why; a player with nothing to defend is
//! on no ballot, and told. Posts come from the constitution (DEC-098).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use mandate_content::{ContentBundle, bundle::read_dir_sources, load};
use mandate_sim::events::EventKind;
use mandate_sim::politics::elections::{Filing, executive_filing, seat_filing};
use mandate_sim::politics::{Post, Role};
use mandate_sim::{Command, Engine, State};
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

fn post_of(f: Option<Filing>) -> Post {
    match f {
        Some(Filing::Stand { post }) => post,
        _ => panic!("no post"),
    }
}

/// The executive office of the player's town and its holder.
fn executive(e: &Engine) -> (Post, Option<mandate_sim::politics::PoliticianId>) {
    let post = post_of(executive_filing(e.state(), e.content()));
    let holder = e
        .state()
        .politics
        .politicians
        .iter()
        .find(|p| p.role.post() == Some(post))
        .map(|p| p.id);
    (post, holder)
}

/// Make the executive's holder of the player's party (`same`) or another,
/// young and with no terms served, so only the party's hidden draw decides
/// whether they stand again.
fn set_holder(e: &mut Engine, same: bool) {
    let (_, holder) = executive(e);
    let holder = holder.expect("the executive office is held");
    e.setup(|st: &mut State| {
        let me = st.player.as_ref().unwrap().politician;
        let mine = st.politics.get(me).unwrap().party;
        let other = st
            .politics
            .parties
            .iter()
            .map(|p| p.id)
            .find(|p| Some(*p) != mine)
            .unwrap();
        let year = st.calendar.date.calendar().year;
        let h = st.politics.get_mut(holder).unwrap();
        h.party = if same { mine } else { Some(other) };
        h.birth_year = year - 45;
        h.terms_in_office = 0;
    });
}

fn set_standing(e: &mut Engine, standing: f32) {
    e.setup(|st: &mut State| st.player.as_mut().unwrap().tracks.standing = standing);
}

/// File (or not), then play until the campaign opens, holding Standing.
fn open_campaign(e: &mut Engine, filing: Option<Filing>, standing: f32) -> bool {
    if !e.state().politics.elections.races.is_empty() {
        return false;
    }
    if let Some(f) = filing {
        e.setup(|st: &mut State| st.player.as_mut().unwrap().filing = Some(f));
    }
    for _ in 0..80 {
        set_standing(e, standing);
        e.apply(Command::EndTurn).unwrap();
        if !e.state().politics.elections.races.is_empty() {
            return true;
        }
    }
    false
}

/// The post the player is a candidate for, if any.
fn standing_for(e: &Engine) -> Option<Post> {
    let me = e.state().player.as_ref().unwrap().politician;
    e.state()
        .politics
        .elections
        .races
        .iter()
        .find(|r| r.candidates.iter().any(|c| c.politician == me))
        .map(|r| r.seat)
}

struct Refused {
    post: Post,
    reason: String,
    nominee: Option<u32>,
    standing: u32,
    min_standing: Option<u32>,
    instead: Option<Post>,
}

fn refusals(e: &Engine) -> Vec<Refused> {
    e.state()
        .events
        .iter()
        .filter_map(|ev| match &ev.kind {
            EventKind::NominationRefused {
                post,
                reason,
                nominee,
                standing,
                min_standing,
                instead,
            } => Some(Refused {
                post: *post,
                reason: reason.clone(),
                nominee: *nominee,
                standing: *standing,
                min_standing: *min_standing,
                instead: *instead,
            }),
            _ => None,
        })
        .collect()
}

/// Who stands where (seat, candidates): the ballot, without the campaign's
/// figures.
fn ballot(e: &Engine) -> Vec<(Post, Vec<u32>)> {
    e.state()
        .politics
        .elections
        .races
        .iter()
        .map(|r| {
            (
                r.seat,
                r.candidates.iter().map(|c| c.politician.0).collect(),
            )
        })
        .collect()
}

#[test]
fn filing_against_the_party_s_own_holder_leaves_the_player_their_seat() {
    // The holder's draw (they usually stand again) decides; over seeds both
    // outcomes occur, and each is the rule.
    let (mut fell_back, mut nominated) = (0, 0);
    for seed in 1..=12u64 {
        let mut e = world(seed);
        let Some(seat) = seat_filing(e.state(), e.content()).map(|f| post_of(Some(f))) else {
            continue;
        };
        if e.state()
            .politics
            .get(e.state().player.as_ref().unwrap().politician)
            .unwrap()
            .role
            .post()
            != Some(seat)
        {
            continue;
        }
        set_holder(&mut e, true);
        let (exec, holder) = executive(&e);
        let mut twin = world(seed);
        set_holder(&mut twin, true);
        let filed = Some(Filing::Stand { post: exec });
        if !open_campaign(&mut e, filed, 60.0) || !open_campaign(&mut twin, None, 60.0) {
            continue;
        }
        let holder_runs =
            e.state().politics.elections.races.iter().any(|r| {
                r.seat == exec && r.candidates.iter().any(|c| Some(c.politician) == holder)
            });
        let refused = refusals(&e);
        if holder_runs {
            fell_back += 1;
            // Defending the seat, exactly as without the filing.
            assert_eq!(standing_for(&e), Some(seat), "seed {seed}");
            assert_eq!(
                ballot(&e),
                ballot(&twin),
                "seed {seed}: the ballot is as if not filed"
            );
            assert_eq!(refused.len(), 1, "seed {seed}");
            let r = &refused[0];
            assert_eq!(r.post, exec);
            assert_eq!(r.reason, "holder");
            assert_eq!(r.nominee, holder.map(|h| h.0));
            assert_eq!(r.instead, Some(seat));
            // Standing as the campaign opened (it moves a little within a turn).
            assert!(r.standing.abs_diff(60) <= 2);
        } else {
            nominated += 1;
            assert_eq!(standing_for(&e), Some(exec), "seed {seed}");
            assert!(refused.is_empty(), "seed {seed}");
        }
    }
    assert!(fell_back > 0, "no seed had the holder stand again");
    let _ = nominated;
}

#[test]
fn standing_below_the_gate_leaves_the_player_their_seat_with_both_numbers() {
    let mut e = world(3);
    set_holder(&mut e, false);
    let (exec, _) = executive(&e);
    let seat = post_of(seat_filing(e.state(), e.content()));
    // Above the party's renomination line (25), below the office's gate (40).
    assert!(open_campaign(
        &mut e,
        Some(Filing::Stand { post: exec }),
        30.0
    ));
    assert_eq!(standing_for(&e), Some(seat));
    let r = refusals(&e);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].reason, "standing");
    assert!(r[0].standing.abs_diff(30) <= 2 && r[0].standing < 40);
    assert_eq!(r[0].min_standing, Some(40));
    assert_eq!(r[0].instead, Some(seat));
    // On the spine: the parties' rule and the Standing behind it.
    let ev = e
        .state()
        .events
        .iter()
        .find(|ev| matches!(ev.kind, EventKind::NominationRefused { .. }))
        .unwrap();
    assert!(!ev.causes.is_empty());
}

#[test]
fn an_office_held_by_another_party_or_open_takes_the_player_as_nominee() {
    for open in [false, true] {
        let mut e = world(4);
        set_holder(&mut e, false);
        let (exec, holder) = executive(&e);
        if open {
            // The office is open: its holder steps out of politics.
            let h = holder.unwrap();
            e.setup(|st: &mut State| {
                for o in st.politics.offices.iter_mut() {
                    if o.holder == Some(h) {
                        o.holder = None;
                    }
                }
                let p = st.politics.get_mut(h).unwrap();
                let home = p.role.post().map(|q| q.at()).and_then(|a| a.municipality());
                p.role = Role::Private {
                    home: home.unwrap(),
                };
            });
        }
        assert!(open_campaign(
            &mut e,
            Some(Filing::Stand { post: exec }),
            60.0
        ));
        assert_eq!(standing_for(&e), Some(exec), "open {open}");
        assert!(refusals(&e).is_empty(), "open {open}");
    }
}

#[test]
fn a_player_with_no_seat_refused_is_on_no_ballot_and_told() {
    let mut e = world(5);
    set_holder(&mut e, false);
    let (exec, _) = executive(&e);
    // Out of office: an NPC takes the player's seat.
    e.setup(|st: &mut State| {
        let me = st.player.as_ref().unwrap().politician;
        let role = st.politics.get(me).unwrap().role;
        let home = st.player.as_ref().unwrap().home();
        let npc = st
            .politics
            .politicians
            .iter()
            .find(|p| !p.is_player && !p.in_office() && p.municipality() == home.0)
            .map(|p| p.id)
            .unwrap();
        for b in st.politics.bodies.iter_mut() {
            for m in b.members.iter_mut() {
                if *m == me {
                    *m = npc;
                }
            }
        }
        st.politics.get_mut(npc).unwrap().role = role;
        st.politics.get_mut(me).unwrap().role = Role::Private { home };
    });
    assert!(open_campaign(
        &mut e,
        Some(Filing::Stand { post: exec }),
        30.0
    ));
    assert_eq!(standing_for(&e), None);
    let r = refusals(&e);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].reason, "standing");
    assert_eq!(r[0].instead, None);
}

/// After any campaign opens, a player in office who did not stand down and
/// whom the party still renominates is a candidate somewhere, or an event
/// says why not.
#[test]
fn a_player_in_office_is_on_a_ballot_or_told_why_not() {
    for seed in 6..=11u64 {
        for (k, standing) in [(0u8, 60.0f32), (1, 30.0), (2, 60.0)] {
            let mut e = world(seed);
            set_holder(&mut e, k == 2);
            let filing = match k {
                0 => None,
                _ => Some(Filing::Stand {
                    post: executive(&e).0,
                }),
            };
            if !open_campaign(&mut e, filing, standing) {
                continue;
            }
            let st = e.state();
            let me = st.player.as_ref().unwrap().politician;
            if !st.politics.get(me).unwrap().in_office() {
                continue;
            }
            assert!(
                standing_for(&e).is_some() || !refusals(&e).is_empty(),
                "seed {seed}, case {k}: in office, on no ballot, and not told"
            );
        }
    }
}
