//! The filing view and election night (play note 20): before filing, each
//! post names its holder, their party, whether they usually stand again (in
//! words) and the Standing the party asks; when the party refuses the
//! player, the campaign view, the pocket's alerts and the event log say so;
//! after the election, what became of the post the player held.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use mandate_content::ContentBundle;
use mandate_protocol::*;
use mandate_sim::politics::elections::{Filing, executive_filing};
use mandate_sim::politics::{PoliticianId, Post};
use mandate_sim::{Command, Engine, State};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::sync::{Arc, OnceLock};

fn content() -> Arc<ContentBundle> {
    static C: OnceLock<Arc<ContentBundle>> = OnceLock::new();
    C.get_or_init(|| {
        Arc::new(
            mandate_content::load(&mandate_content::embedded::sources())
                .unwrap()
                .0,
        )
    })
    .clone()
}

fn game(seed: u64, size: WorldSize) -> Engine {
    generate(&WorldConfig::new(seed, size), content()).unwrap()
}

fn executive(e: &Engine) -> (Post, PoliticianId) {
    let Some(Filing::Stand { post }) = executive_filing(e.state(), e.content()) else {
        panic!("no executive");
    };
    let h = e
        .state()
        .politics
        .politicians
        .iter()
        .find(|p| p.role.post() == Some(post))
        .unwrap()
        .id;
    (post, h)
}

/// The executive's holder of the player's party or not, `age` years old,
/// with `terms` served.
fn set_holder(e: &mut Engine, same: bool, age: u32, terms: u8) {
    let (_, h) = executive(e);
    e.setup(|st: &mut State| {
        let me = st.player.as_ref().unwrap().politician;
        let mine = st.politics.get(me).unwrap().party;
        let other = st
            .politics
            .parties
            .iter()
            .map(|p| p.id)
            .find(|p| Some(*p) != mine);
        let year = st.calendar.date.calendar().year;
        let p = st.politics.get_mut(h).unwrap();
        p.party = if same { mine } else { other };
        p.birth_year = year - age;
        p.terms_in_office = terms;
    });
}

fn office_nomination(e: &Engine) -> NominationView {
    campaign_view(e)
        .unwrap()
        .nominations
        .into_iter()
        .find(|n| n.post.kind == PostKindDto::Office)
        .unwrap()
}

#[test]
fn the_filing_view_names_the_holder_the_rule_and_the_gate() {
    let mut e = game(1, WorldSize::Small);
    let (_, h) = executive(&e);
    let name = e.state().politics.get(h).unwrap().full_name();
    set_holder(&mut e, true, 45, 0);
    let standing = e.state().player.as_ref().unwrap().tracks.standing as f64;
    let n = office_nomination(&e);
    assert!(!n.held);
    assert_eq!(n.holder.as_deref(), Some(name.as_str()));
    let mine = e
        .state()
        .politics
        .get(e.state().player.as_ref().unwrap().politician)
        .unwrap()
        .party;
    assert_eq!(n.holder_party, mine.map(|p| p.0));
    assert!(n.holder_same_party);
    assert_eq!(
        n.holder_outlook_key.as_deref(),
        Some("ui-holder-usually-stands")
    );
    assert_eq!(n.gate_standing, Some(40.0));
    assert_eq!(n.standing, standing);
    // A councillor whom the party renominates keeps their seat.
    assert_eq!(n.warning_key.as_deref(), Some("ui-nomination-warn-seat"));
    // The seat held: no holder but the player, no gate, no warning.
    let seat = campaign_view(&e)
        .unwrap()
        .nominations
        .into_iter()
        .find(|n| n.held)
        .unwrap();
    assert_eq!(seat.holder, None);
    assert_eq!((seat.gate_standing, seat.warning_key), (None, None));

    // A holder of age to retire: said so, and no warning.
    set_holder(&mut e, true, 80, 0);
    let n = office_nomination(&e);
    assert_eq!(n.holder_outlook_key.as_deref(), Some("ui-holder-retiring"));
    assert_eq!(n.warning_key, None);
    // Another party's holder: the party backs the player, no warning.
    set_holder(&mut e, false, 45, 0);
    let n = office_nomination(&e);
    assert!(!n.holder_same_party);
    assert_eq!(n.warning_key, None);
}

#[test]
fn a_refused_nomination_is_in_the_campaign_view_the_alerts_and_the_log() {
    let mut told = 0;
    for seed in 1..=10u64 {
        let mut e = game(seed, WorldSize::Small);
        if !e.state().politics.elections.races.is_empty() {
            continue;
        }
        set_holder(&mut e, true, 45, 0);
        let (exec, h) = executive(&e);
        e.setup(|st: &mut State| {
            st.player.as_mut().unwrap().filing = Some(Filing::Stand { post: exec })
        });
        for _ in 0..80 {
            e.setup(|st: &mut State| st.player.as_mut().unwrap().tracks.standing = 60.0);
            e.apply(Command::EndTurn).unwrap();
            if !e.state().politics.elections.races.is_empty() {
                break;
            }
        }
        let cv = campaign_view(&e).unwrap();
        let holder_runs = e
            .state()
            .politics
            .elections
            .races
            .iter()
            .any(|r| r.seat == exec && r.candidates.iter().any(|c| c.politician == h));
        if !holder_runs {
            assert_eq!(cv.refusal, None, "seed {seed}");
            continue;
        }
        told += 1;
        let r = cv.refusal.clone().expect("the refusal is in the view");
        assert_eq!(r.key, "nomination-refused");
        assert_eq!(r.args["why_key"], "nomination-why-holder");
        assert_eq!(r.args["then_key"], "nomination-then-seat");
        assert_eq!(
            r.args["holder"],
            e.state().politics.get(h).unwrap().full_name()
        );
        assert_eq!(r.args["post_key"], "office-mayor");
        assert_eq!(r.args["district_key"], "ui-post-seat");
        // The player's race is their seat.
        assert!(
            cv.races
                .iter()
                .any(|r| r.is_player_race && r.post.kind == PostKindDto::Seat)
        );
        // The pocket's alert, and the story's event, say the same.
        let pocket = pocket_view(&e).unwrap();
        assert!(
            pocket
                .alerts
                .iter()
                .any(|a| a.key == r.key && a.args == r.args)
        );
        let ev = e
            .state()
            .events
            .iter()
            .find(|ev| {
                matches!(
                    ev.kind,
                    mandate_sim::events::EventKind::NominationRefused { .. }
                )
            })
            .unwrap();
        let view = event_view(ev, &e);
        assert_eq!((view.text_key, view.args), (r.key.clone(), r.args.clone()));
    }
    assert!(told > 0, "no seed had the holder stand again");
}

#[test]
fn election_night_says_what_became_of_the_post_held() {
    let mut e = game(2, WorldSize::Small);
    e.setup(|st: &mut State| st.player.as_mut().unwrap().filing = Some(Filing::Retire));
    let before = campaign_view(&e).unwrap().last_election;
    for _ in 0..80 {
        e.apply(Command::EndTurn).unwrap();
        if campaign_view(&e).unwrap().last_election != before {
            break;
        }
    }
    let cv = campaign_view(&e).unwrap();
    assert_ne!(cv.last_election, before, "an election was held");
    assert!(!cv.results.is_empty());
    assert!(cv.results.iter().all(|r| !r.player_ran));
    let o = cv.seat_outcome.expect("the seat held");
    assert_eq!(o.key, "ui-seat-passed");
    assert_eq!(o.post.kind, PostKindDto::Seat);
    assert!(o.winner.is_some());
}
