#![allow(clippy::unwrap_used, clippy::expect_used, clippy::print_stdout)]
//! The budget round (DEC-109, task A2): the standing arc of a session.
//!
//! - the budget calendar raises it once a budget year in the town the
//!   player is watching, in eight seeds of eight, and it turns on a
//!   different variant in consecutive years (climax memory by variant);
//! - its cast binds through powers (whoever proposes the budget, another
//!   holder of the power to approve it), never an office's name;
//! - a passive and an engaged scripted play-through of one seed end at
//!   different exits, and the difference is in simulated state: a change
//!   voted into the next budget (and the budget itself once the year
//!   turns), a colleague's opinion, a favour owed;
//! - every beat a player takes is reached in quick mode and through Places.
//!
//! A crisis beside it, which leads, is `director_arcs.rs`.

use mandate_content::arcs::BeatKind;
use mandate_content::{ContentBundle, Power};
use mandate_sim::actions::{Action, PlayerCommand};
use mandate_sim::arcs::beats::{self, BeatAction, BeatOffer};
use mandate_sim::arcs::{ArcActor, ArcRun, ArcSummary};
use mandate_sim::characters::{self, CharacterId};
use mandate_sim::events::EventKind;
use mandate_sim::places::Location;
use mandate_sim::talk::TalkTarget;
use mandate_sim::world::Jurisdiction;
use mandate_sim::{Command, Engine};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

const ARC: &str = "arc.budget_round";
/// The seeds of the acceptance run (the flood's, in A1).
const SEEDS: [u64; 8] = [1, 2, 3, 7, 31, 99, 777, 2026];

/// The base pack with the round alone (the arc-test harness's `only`):
/// these are the round's tests. A flood beside it, and which leads, is the
/// director's (`director_arcs.rs`).
fn base() -> Arc<ContentBundle> {
    mandate_testkit::only(&[ARC])
}

use mandate_testkit::{town, world as engine};

/// The round in progress.
fn round(e: &Engine) -> Option<&ArcRun> {
    e.state().arcs.runs.iter().find(|r| r.template == ARC)
}

/// The rounds that ended, oldest first.
fn rounds(e: &Engine) -> Vec<&ArcSummary> {
    e.state()
        .arcs
        .past
        .iter()
        .filter(|p| p.template == ARC)
        .collect()
}

fn me(e: &Engine) -> CharacterId {
    let st = e.state();
    let pl = st.player.as_ref().unwrap();
    st.politics.get(pl.politician).unwrap().character
}

fn person(cast: &[mandate_sim::arcs::ArcCastEntry], role: &str) -> CharacterId {
    cast.iter()
        .find_map(|c| match c.actor {
            ArcActor::Person { id } if c.role == role => Some(id),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no {role}"))
}

/// The beats of a run that happened, as `stage.beat:kind:ok`, from the log.
fn beats_done(e: &Engine, run: u32) -> BTreeSet<String> {
    e.state()
        .events
        .iter()
        .filter_map(|ev| match &ev.kind {
            EventKind::ArcBeat {
                arc,
                stage,
                beat,
                kind,
                ok,
                ..
            } if *arc == run => Some(format!(
                "{stage}.{beat}:{kind}:{}",
                if *ok { "done" } else { "not" }
            )),
            _ => None,
        })
        .collect()
}

/// A scripted player of the round: which of its beats it takes, and how
/// it gets to them. Nothing else is touched (other decisions take their
/// defaults), so two scripts differ only in what they did about the round.
#[derive(Clone, Copy)]
struct Script {
    /// Take every beat the round offers (`false`: every turn just ends).
    acts: bool,
    /// Through Places: walk there (`MoveTo`), then do it. Otherwise quick
    /// mode: one request, routed.
    places: bool,
}

const PASSIVE: Script = Script {
    acts: false,
    places: false,
};
const ENGAGED: Script = Script {
    acts: true,
    places: false,
};

/// The engaged answers: press the neighbourhood's case, then move a change
/// at the turn, whichever turn it is.
fn answer(dilemma: &str) -> &'static str {
    match dilemma {
        "dilemma.budget_line" => "press",
        "dilemma.budget_hole" => "cuts",
        "dilemma.budget_surplus" => "spend",
        "dilemma.budget_deal" => "broker",
        "dilemma.budget_row" => "back_member",
        other => panic!("no scripted answer for {other}"),
    }
}

impl Script {
    /// The commands that take an offered beat now. `None`: not this turn.
    fn commands(&self, e: &Engine, o: &BeatOffer) -> Option<Vec<Command>> {
        let st = e.state();
        let located = |at: Location, then: PlayerCommand| -> Vec<Command> {
            if self.places {
                vec![
                    Command::Player(PlayerCommand::MoveTo { location: at }),
                    Command::Player(then),
                ]
            } else {
                vec![Command::Player(PlayerCommand::Go {
                    location: at,
                    then: Some(Box::new(then)),
                })]
            }
        };
        let talk = |person: CharacterId, topic: String| -> Option<Vec<Command>> {
            let at = characters::routines::whereabouts(st, e.content(), person)?;
            Some(located(
                at,
                PlayerCommand::Talk {
                    to: TalkTarget::Person { person },
                    topic,
                    about: None,
                    late_night: false,
                },
            ))
        };
        match &o.action {
            BeatAction::Visit { verb, location } => {
                let act = PlayerCommand::Act {
                    action: Action::Visit { verb: verb.clone() },
                    late_night: false,
                };
                if self.places {
                    Some(located(*location, act))
                } else {
                    Some(vec![e.route(act)])
                }
            }
            BeatAction::Talk { person, topic } => talk(*person, topic.clone()),
            BeatAction::Decision { decision } => {
                let d = st
                    .narrative
                    .decision(mandate_sim::narrative::DecisionId(*decision))?;
                Some(vec![Command::Player(PlayerCommand::Answer {
                    decision: *decision,
                    option: answer(&d.template).to_string(),
                })])
            }
            BeatAction::Ask { ask } => {
                let a = st.asks.get(mandate_sim::asks::AskId(*ask))?;
                if a.offered() {
                    return Some(vec![Command::Player(PlayerCommand::RespondAsk {
                        ask: *ask,
                        accept: true,
                    })]);
                }
                match a.current()? {
                    mandate_sim::asks::Need::Talk { person } => {
                        talk(*person, mandate_sim::asks::hook::choice_id(a.id))
                    }
                    _ => None,
                }
            }
            // The budget round holds no scene.
            BeatAction::Exchange { .. } => None,
        }
    }

    /// One turn: take what the round offers (if the script acts), then end
    /// the turn. Returns the commands given, in order.
    fn turn(&self, e: &mut Engine) -> Vec<Command> {
        let mut given = Vec::new();
        let mut tried: BTreeSet<String> = BTreeSet::new();
        for _ in 0..16 {
            let Some(id) = round(e).map(|r| r.id).filter(|_| self.acts) else {
                break;
            };
            let offers = beats::offers(e.state(), e.content());
            let Some((key, cmds)) = offers.iter().filter(|o| o.run == id).find_map(|o| {
                let key = format!("{:?}", o.action);
                (!tried.contains(&key)).then(|| (key, self.commands(e, o)))
            }) else {
                break;
            };
            tried.insert(key);
            let Some(cmds) = cmds else { continue };
            let mut ok = true;
            for c in cmds {
                if ok && e.apply(c.clone()).is_ok() {
                    given.push(c);
                } else {
                    ok = false;
                }
            }
            if ok {
                // An ask accepted has a next step this turn.
                tried.retain(|k| !k.starts_with("Ask"));
            }
        }
        e.apply(Command::EndTurn).unwrap();
        given.push(Command::EndTurn);
        given
    }

    /// Play until `n` rounds have ended (at most `turns` turns).
    fn play(&self, e: &mut Engine, n: usize, turns: u32) -> Vec<Command> {
        let mut list = Vec::new();
        for _ in 0..turns {
            if rounds(e).len() >= n {
                break;
            }
            list.extend(self.turn(e));
        }
        list
    }
}

/// The acceptance run: the calendar raises one round a budget year in the
/// town the player is watching, eight seeds of eight; each round reaches
/// its turn; consecutive years turn on different variants.
#[test]
fn the_calendar_raises_one_round_a_budget_year_and_the_variants_differ() {
    let c = base();
    let def = c.arc(ARC).unwrap();
    let mut ok = 0;
    let mut variants: BTreeMap<String, u32> = BTreeMap::new();
    for seed in SEEDS {
        let mut e = engine(&c, seed);
        let at = town(&e);
        let first = e.state().calendar.date.calendar().year;
        // Four budget years, nobody playing.
        while e.state().calendar.date.calendar().year < first + 4 {
            e.apply(Command::EndTurn).unwrap();
        }
        let st = e.state();
        let mut raised: BTreeMap<u32, u32> = BTreeMap::new();
        for ev in st.events.iter() {
            if let EventKind::ArcRaised {
                template, at: a, ..
            } = &ev.kind
                && template == ARC
            {
                assert_eq!(*a, at, "seed {seed}: raised where the player is watching");
                *raised.entry(ev.date.calendar().year).or_default() += 1;
            }
        }
        let want: BTreeMap<u32, u32> = (first..first + 4).map(|y| (y, 1)).collect();
        println!(
            "seed {seed}: raised {raised:?}, turns {:?}",
            rounds(&e)
                .iter()
                .map(|p| p.turn.as_ref().map(|t| t.stage.clone()))
                .collect::<Vec<_>>()
        );
        if raised == want {
            ok += 1;
        }
        // Every round reached its turn, a variant of the template, and no
        // two running are the same.
        let turns: Vec<&str> = rounds(&e)
            .iter()
            .map(|p| {
                p.turn
                    .as_ref()
                    .expect("a round reaches its turn")
                    .stage
                    .as_str()
            })
            .collect();
        assert_eq!(turns.len(), 4, "seed {seed}: four rounds ended");
        for t in &turns {
            assert!(def.stage(t).is_some_and(|s| s.climax), "{t}");
            *variants.entry((*t).to_string()).or_default() += 1;
        }
        for w in turns.windows(2) {
            assert_ne!(w[0], w[1], "seed {seed}: the same turn two years running");
        }
        // The vote fell before the budget year turned, every year.
        for p in rounds(&e) {
            assert_eq!(
                p.started.calendar().year,
                p.ended.calendar().year,
                "seed {seed}: {p:?}"
            );
        }
        // An exit with nobody playing: it passed, or it was late.
        for p in rounds(&e) {
            assert!(["passed", "late"].contains(&p.exit.as_str()), "{p:?}");
        }
    }
    assert_eq!(ok, 8, "one round a budget year in every seed");
    // More than one variant is seen across the eight towns.
    println!("variants: {variants:?}");
    assert!(variants.len() >= 3, "{variants:?}");
}

/// The cast binds through powers (DEC-098): the proposer holds the power
/// to propose the budget, the colleague the power to approve it, neither
/// is the player, and the hall is where the proposer's post sits.
#[test]
fn the_cast_binds_through_the_budget_powers() {
    let c = base();
    for seed in [1u64, 7, 99] {
        let mut e = engine(&c, seed);
        until_raised(&mut e);
        let st = e.state();
        let run = round(&e).expect("a round");
        assert_eq!(run.at, town(&e));
        let con = c.constitutions.get(&st.meta.constitution).unwrap();
        let holds = |who: CharacterId, power: Power| {
            let p = st
                .politics
                .characters
                .get(who)
                .and_then(|x| x.politician)
                .unwrap();
            mandate_sim::politics::holds_power_at(&st.politics, con, p, power, run.at)
        };
        let (proposer, member) = (person(&run.cast, "proposer"), person(&run.cast, "member"));
        assert!(holds(proposer, Power::ProposeBudget));
        assert!(holds(member, Power::ApproveBudget));
        assert!(proposer != me(&e) && member != me(&e) && proposer != member);
        let Some(ArcActor::Place { id }) = run.actor("hall") else {
            panic!("no hall");
        };
        let place = &st.world.places[id.index()];
        assert!(Jurisdiction::town(place.municipality) == run.at && place.is_open());
        assert!(matches!(run.actor("hood"), Some(ArcActor::Area { .. })));
        // The vote's day is set, inside the budget year.
        let clock = run.clock.expect("the vote has a day");
        assert_eq!(clock.calendar().year, st.calendar.date.calendar().year);
        assert!(clock > st.calendar.date);
    }
}

/// End turns until a round is raised (at most forty).
fn until_raised(e: &mut Engine) {
    for _ in 0..40 {
        if round(e).is_some() {
            return;
        }
        e.apply(Command::EndTurn).unwrap();
    }
    panic!("no round was raised in forty turns");
}

/// The acceptance pairs: in each seed of the band, one round, two scripts.
/// The passive player's budget passes as written; the engaged player moves
/// a change, and the room's arithmetic decides it. Where it carries, the
/// two worlds differ in what the treasury will spend; where the colleague's
/// ask was kept, in what the colleague thinks and in who owes whom.
/// (A fixed band, not a picked seed: which turn a town's first round takes
/// and whether its room backs the player are the town's own.)
#[test]
fn passive_and_engaged_scripts_reach_different_exits() {
    let c = base();
    let (mut differ, mut carried, mut spent, mut owing) = (0, 0, 0, 0);
    for seed in SEEDS {
        let mut passive = engine(&c, seed);
        let mut engaged = engine(&c, seed);
        PASSIVE.play(&mut passive, 1, 60);
        let list = ENGAGED.play(&mut engaged, 1, 60);
        let (p, g) = (rounds(&passive)[0].clone(), rounds(&engaged)[0].clone());
        println!(
            "seed {seed}: passive {} (turn {:?}, marks {:?}); engaged {} (turn {:?}, marks {:?})",
            p.exit, p.turn, p.marks, g.exit, g.turn, g.marks
        );
        // The same matter in both worlds: the same cast, the same day.
        assert_eq!((p.started, &p.cast), (g.started, &g.cast), "seed {seed}");
        // Nobody moved anything: it passed as written, or it was late.
        assert!(
            ["passed", "late"].contains(&p.exit.as_str()),
            "seed {seed}: {p:?}"
        );
        assert!(p.marks.is_empty(), "seed {seed}: {:?}", p.marks);
        // The engaged player gave commands a player has, and nothing else,
        // and the run remembers what they did.
        assert!(
            list.iter()
                .any(|x| matches!(x, Command::Player(PlayerCommand::Answer { .. }))),
            "seed {seed}"
        );
        assert!(
            g.marks.iter().any(|m| m == "pressed"),
            "seed {seed}: {:?}",
            g.marks
        );
        // A change moved is carried or defeated by the room (or, with no
        // majority and no deal, the budget is late): never just passed.
        assert!(
            ["carried", "defeated", "late"].contains(&g.exit.as_str()),
            "seed {seed}: {g:?}"
        );
        if g.exit != p.exit {
            differ += 1;
        }

        // The change the room carried is voted into the next budget;
        // nothing is voted where nobody moved anything, or where the room
        // said no.
        let has = |m: &str| g.marks.iter().any(|x| x == m);
        let factor = if g.exit != "carried" {
            None
        } else if has("cut") {
            Some(0.97)
        } else if has("spend") {
            Some(1.03)
        } else if has("backed") {
            Some(1.01)
        } else {
            None
        };
        let year = p.ended.calendar().year;
        let member = person(&p.cast, "member");
        let m = mandate_sim::player::home(passive.state()).unwrap().index();
        let late = |x: &ArcSummary| usize::from(x.exit == "late");
        assert_eq!(
            passive.state().finance.local[m].voted.len(),
            late(&p),
            "seed {seed}"
        );
        assert_eq!(
            engaged.state().finance.local[m].voted.len(),
            usize::from(factor.is_some()) + late(&g),
            "seed {seed}"
        );
        // Play both to the same day: past the day the budget year turns.
        for e in [&mut passive, &mut engaged] {
            while e.state().calendar.date.calendar().year == year {
                e.apply(Command::EndTurn).unwrap();
            }
        }
        assert_eq!(passive.state().calendar.date, engaged.state().calendar.date);
        for e in [&passive, &engaged] {
            assert!(
                e.state().finance.local[m].voted.is_empty(),
                "seed {seed}: spent as the year turned"
            );
        }
        if g.exit == "carried" {
            carried += 1;
        }
        if let (Some(f), "passed") = (factor, p.exit.as_str()) {
            let budget = |e: &Engine| e.state().finance.local[m].budget_total().units() as f64;
            let (bp, bg) = (budget(&passive), budget(&engaged));
            println!(
                "  next year's budget: passive {bp:.0}, engaged {bg:.0} ({:+.2}%, moved {:+.0}%)",
                (bg / bp - 1.0) * 100.0,
                (f - 1.0) * 100.0
            );
            // As moved (the two towns are otherwise the same town; a
            // little drift is allowed).
            assert!(
                (bg / bp - f).abs() < 0.008,
                "seed {seed}: {bp} vs {bg}, moved {f}"
            );
            spent += 1;
        }

        // The colleague whose ask the player kept thinks better of them,
        // and owes them one; the passive player is owed nothing.
        let owed = |e: &Engine| characters::favours_owed(&e.state().politics, member, me(e));
        assert_eq!(owed(&passive), 0, "seed {seed}");
        if has("seconded") {
            let opinion = |e: &Engine| characters::opinion(&e.state().politics, member, me(e));
            assert!(
                opinion(&engaged) > opinion(&passive) + 5.0,
                "seed {seed}: {} vs {}",
                opinion(&engaged),
                opinion(&passive)
            );
            assert!(owed(&engaged) >= 1, "seed {seed}");
            // And the record of visible work: the ask kept shows (an act;
            // no exit moves the record).
            let record = |e: &Engine| {
                let st = e.state();
                st.politics
                    .get(st.player.as_ref().unwrap().politician)
                    .unwrap()
                    .record
            };
            assert!(record(&engaged) > record(&passive), "seed {seed}");
            owing += 1;
        }
    }
    println!(
        "of 8: exits differ in {differ}, carried in {carried}, a budget changed in {spent}, a favour owed in {owing}"
    );
    assert_eq!(differ, 8, "the engaged script ends elsewhere in every seed");
    // (Measured: carried 6, a budget changed 5, a favour owed 3; before
    // audit fixes 3, 5 and 8 were merged, 7, 7 and 6.)
    assert!(carried >= 4 && spent >= 3 && owing >= 2);
}

/// DEC-051: every beat of the round a player takes is taken with commands
/// that exist, in quick mode (one routed request) and through Places
/// (walk there, then act), and the two reach the same beats.
#[test]
fn every_beat_is_reached_in_quick_mode_and_through_places() {
    let c = base();
    let def = c.arc(ARC).unwrap();
    // A town of the band where the colleague's ask can be kept (whether
    // the proposer will hear it out is the town's own): found by play, so
    // a change to the world finds another.
    let seed = SEEDS
        .into_iter()
        .find(|s| {
            let mut e = engine(&c, *s);
            ENGAGED.play(&mut e, 1, 60);
            rounds(&e)[0].marks.iter().any(|m| m == "seconded")
        })
        .expect("a seed of the band where the colleague's ask is kept");
    let mut seen: Vec<BTreeSet<String>> = Vec::new();
    for places in [false, true] {
        let script = Script { acts: true, places };
        let mut e = engine(&c, seed);
        let list = script.play(&mut e, 1, 60);
        let done = rounds(&e)[0];
        if places {
            assert!(
                list.iter()
                    .any(|x| matches!(x, Command::Player(PlayerCommand::MoveTo { .. })))
                    && !list
                        .iter()
                        .any(|x| matches!(x, Command::Player(PlayerCommand::Go { .. })))
            );
        }
        let all = beats_done(&e, done.id.0);
        // Every beat of the stages the round went through was taken: the
        // estimates read, the proposer asked, the colleague's ask kept,
        // the stand taken, the vote cast; the story ran.
        let turn = done.turn.as_ref().unwrap().stage.as_str();
        for (stage, beat) in def.beats() {
            if !["estimates", "line", turn].contains(&stage.id.as_str()) {
                continue;
            }
            let key = format!("{}.{}", stage.id, beat.id);
            let reached = all
                .iter()
                .any(|x| x.starts_with(&format!("{key}:")) && x.ends_with(":done"));
            match &beat.kind {
                // The colleague's own move is theirs to make (a keyed
                // draw), and is not made once the ask is kept; the
                // proposer is asked once (in the first stage here).
                BeatKind::Move { .. } => {}
                BeatKind::Talk { .. } if stage.id == "line" => {}
                _ => assert!(
                    reached,
                    "{key} ({}) was not reached: {all:#?}",
                    beat.kind.key()
                ),
            }
        }
        seen.push(all);
    }
    assert_eq!(
        seen[0], seen[1],
        "quick mode and Places reach the same beats"
    );
}
