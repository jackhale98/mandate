//! The pocket rework's views (DEC-113, `POCKET_REWORK.md` task R1) and the
//! arc view (DEC-109, `STAGED_ARCS.md` task A3's protocol half):
//!
//! - one query lists every person the player has heard of, in sections and
//!   in order, and every count and reason comes with it;
//! - what needs the player, the arcs under way and what comes next are the
//!   engine's, ordered by it;
//! - the order is the same twice, and the same through a save and load;
//! - no reason, note or arc line names a person or a matter the player
//!   does not know (DEC-091); below `known` the arc view is a lead only;
//! - a query writes nothing, and with the same commands a ledger that
//!   knows everything and the default one leave identical worlds (K4);
//! - the fixtures the web task builds against are views the engine made.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use mandate_content::ContentBundle;
use mandate_content::arcs::BeatKind;
use mandate_content::knowledge::Level;
use mandate_protocol::*;
use mandate_sim::actions::{Action, PlayerCommand};
use mandate_sim::arcs::ArcActor;
use mandate_sim::arcs::beats::{self, BeatAction};
use mandate_sim::characters::{self, CharacterId};
use mandate_sim::environment::{DeferredHazard, HazardKind};
use mandate_sim::narrative::Actor;
use mandate_sim::player::knowledge::Subject;
use mandate_sim::{Command, Engine};
use mandate_wasm::session::Session;
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const FLOOD: &str = "arc.flood";
const SEVERITY: f64 = 0.9;

fn base() -> ContentBundle {
    mandate_content::load(&mandate_content::embedded::sources())
        .unwrap()
        .0
}

/// The base pack with the flood's first stage hidden and nothing in it
/// aimed at the player: its visit teaches `teaches` (none: the default,
/// that the matter exists).
fn hidden_flood(teaches: Option<Level>) -> ContentBundle {
    let mut b = base();
    let arc = b.arcs.iter_mut().find(|a| a.id == FLOOD).unwrap();
    let stage = &mut arc.stages[0];
    stage.hidden = true;
    stage
        .beats
        .retain(|x| !matches!(x.kind, BeatKind::Decision { .. } | BeatKind::Ask { .. }));
    for x in &mut stage.beats {
        if matches!(x.kind, BeatKind::Visit { .. }) {
            x.teaches = teaches;
        }
    }
    b
}

fn game(c: &Arc<ContentBundle>, seed: u64) -> Engine {
    generate(&WorldConfig::new(seed, WorldSize::Small), c.clone()).unwrap()
}

/// A flood strikes the player's town at the next monthly hazard step.
fn plant_flood(e: &mut Engine) {
    let town = mandate_sim::player::home(e.state()).unwrap().0;
    let max = e.content().director.chance.hazard_max_defer_days;
    e.setup(|st| {
        let drawn = mandate_core::SimDate(st.calendar.date.0.saturating_sub(max + 1));
        st.environment.deferred.push(DeferredHazard {
            municipality: town,
            kind: HazardKind::Flood,
            severity: SEVERITY,
            drawn,
            hold_event: None,
        });
    });
}

fn end_turn(e: &mut Engine) {
    e.apply(Command::EndTurn).unwrap();
}

fn flood_run(e: &Engine) -> Option<u32> {
    e.state()
        .arcs
        .runs
        .iter()
        .find(|r| r.template == FLOOD)
        .map(|r| r.id.0)
}

/// A game in which the flood arc has just been raised.
fn flooded(c: &Arc<ContentBundle>, seed: u64) -> Engine {
    let mut e = game(c, seed);
    plant_flood(&mut e);
    for _ in 0..10 {
        if flood_run(&e).is_some() {
            return e;
        }
        end_turn(&mut e);
    }
    panic!("seed {seed}: the flood raised no arc in ten turns");
}

/// A planted flood raises the arc in every small world tried (task A1's
/// acceptance); this one also lets it exit with nobody acting.
fn seed() -> u64 {
    7
}

fn me(e: &Engine) -> CharacterId {
    characters::player_character(e.state()).unwrap()
}

/// Forget everything (test setup): the default ledger, emptied.
fn wipe(e: &mut Engine) {
    e.setup(|st| {
        let k = &mut st.player.as_mut().unwrap().knowledge;
        k.lab_omniscient = false;
        k.entries.clear();
        k.leads.clear();
    });
}

fn json<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_string(v).unwrap()
}

/// Take the flood's visit beat on offer (quick mode: one routed request).
fn visit_beat(e: &mut Engine) -> bool {
    let run = flood_run(e);
    let verb = beats::offers(e.state(), e.content())
        .into_iter()
        .filter(|o| Some(o.run.0) == run)
        .find_map(|o| match o.action {
            BeatAction::Visit { verb, .. } => Some(verb),
            _ => None,
        });
    let Some(verb) = verb else {
        return false;
    };
    let cmd = e.route(PlayerCommand::Act {
        action: Action::Visit { verb },
        late_night: false,
    });
    e.apply(cmd).is_ok()
}

/// Every view this task adds or extends, as JSON: what a session would be
/// sent for the pocket, the people, the town, the notebook, the office and
/// every arc on record.
fn all_views(e: &Engine) -> Vec<(String, String)> {
    let mut out = vec![
        ("people".to_string(), json(&people_view(e))),
        ("pocket".to_string(), json(&pocket_view(e))),
        ("town".to_string(), json(&town_places_view(e))),
        ("notebook".to_string(), json(&notebook_view(e))),
        ("office".to_string(), json(&office_view(e))),
    ];
    let st = e.state();
    let ids = st
        .arcs
        .runs
        .iter()
        .map(|r| r.id.0)
        .chain(st.arcs.past.iter().map(|p| p.id.0));
    for id in ids {
        out.push((format!("arc {id}"), json(&arc_view(e, id))));
    }
    out
}

// --- People --------------------------------------------------------------------------------------

/// One query lists every person the player has heard of, once each, in
/// sections with contiguous ranks; the counts are the view's own.
#[test]
fn one_query_lists_every_person_in_sections_and_in_order() {
    let c = Arc::new(base());
    for seed in [seed(), 21, 2026] {
        let e = flooded(&c, seed);
        let v = people_view(&e).unwrap();
        let pl = e.state().player.as_ref().unwrap();
        // Everyone the ledger has at `heard` or more (the notebook's people).
        let known: BTreeSet<u32> = pl
            .knowledge
            .entries
            .iter()
            .filter_map(|(s, x)| match s {
                Subject::Person { person } if x.level >= Level::Heard => Some(person.0),
                _ => None,
            })
            .filter(|id| *id != me(&e).0)
            .collect();
        let listed: BTreeSet<u32> = v.rows.iter().map(|r| r.id).collect();
        assert!(!known.is_empty(), "seed {seed}: the player knows somebody");
        assert_eq!(listed.len(), v.rows.len(), "seed {seed}: one row a person");
        assert!(
            known.is_subset(&listed),
            "seed {seed}: missing {:?}",
            known.difference(&listed).collect::<Vec<_>>()
        );
        assert!(
            !listed.contains(&me(&e).0),
            "the player is not on their own list"
        );
        // The notebook's people are the same people.
        let nb = notebook_view(&e).unwrap();
        for entry in nb
            .sections
            .iter()
            .filter(|s| s.key == "people")
            .flat_map(|s| &s.entries)
        {
            let SubjectDto::Person { person } = entry.subject else {
                continue;
            };
            if person != me(&e).0 {
                assert!(
                    listed.contains(&person),
                    "seed {seed}: notebook person {person}"
                );
            }
        }
        // Sections in display order, ranks 0.. in each, counts that add up.
        let mut at = 0usize;
        let mut last = None;
        for g in &v.groups {
            assert!(last < Some(g.group), "seed {seed}: sections in order");
            last = Some(g.group);
            assert!(g.count > 0 && g.show > 0);
            for rank in 0..g.count {
                let row = &v.rows[at];
                assert_eq!((row.group, row.rank), (g.group, rank), "seed {seed}");
                assert_eq!(
                    row.now.is_empty(),
                    g.group != PersonGroup::Now,
                    "seed {seed}: reasons exactly in `now`: {row:?}"
                );
                at += 1;
            }
        }
        assert_eq!(at, v.rows.len(), "seed {seed}: the counts cover every row");
        for row in &v.rows {
            assert!(!row.name.is_empty(), "seed {seed}: a row has a name");
            assert_eq!(row.can_talk, row.here);
            assert_eq!(row.here, row.now.iter().any(|r| r.kind == NowKind::Here));
            // A reason's own line names nobody: it takes no arguments.
            for r in &row.now {
                assert!(r.key.starts_with("now-"), "{r:?}");
                assert!(r.args.is_empty(), "{r:?}");
            }
            if let Some(n) = &row.note {
                assert!(n.key.starts_with("know-"), "{n:?}");
            }
        }
    }
}

/// The flood's asker matters now because of the ask, with its deadline and
/// what it opens; a row replaces the sheet a card used to fetch.
#[test]
fn an_asker_matters_now_and_the_row_agrees_with_their_sheet() {
    let c = Arc::new(base());
    let e = flooded(&c, seed());
    let st = e.state();
    let ask = st
        .asks
        .open()
        .find(|a| {
            a.arc
                .as_ref()
                .is_some_and(|x| Some(x.run.0) == flood_run(&e))
        })
        .expect("the flood offers its ask as it is raised");
    let asker = ask.asker_person().unwrap();
    let v = people_view(&e).unwrap();
    let row = v
        .rows
        .iter()
        .find(|r| r.id == asker.0)
        .expect("the asker has a row");
    assert_eq!(row.group, PersonGroup::Now);
    let why = row
        .now
        .iter()
        .find(|r| r.kind == NowKind::Ask && r.ask == Some(ask.id.0))
        .expect("the ask is the reason");
    assert_eq!(why.key, "now-ask-offer");
    assert_eq!(why.arc, flood_run(&e));
    assert_eq!(
        why.due.as_ref().map(|d| d.day_number),
        Some(ask.respond_by.0)
    );
    assert_eq!(
        why.days_left,
        Some(st.calendar.date.days_until(ask.respond_by))
    );
    assert!(
        why.title.is_some(),
        "the opened row has the ask's own title"
    );
    // The same facts the character sheet gives.
    let sheet = character_view(&e, asker.0).unwrap();
    assert_eq!(row.name, sheet.name);
    assert_eq!(row.role_key, sheet.role_key);
    assert_eq!(
        (
            &row.opinion_band,
            &row.trust_band,
            row.owes_you,
            row.you_owe
        ),
        (
            &sheet.opinion_band,
            &sheet.trust_band,
            sheet.owes_you,
            sheet.you_owe
        )
    );
    assert_eq!(row.stance.band, sheet.opinion_band);
    assert!(["up", "level", "down"].contains(&row.stance.shape.as_str()));
    // "Go" walks to where their day has them.
    let at = characters::routines::whereabouts(st, e.content(), asker);
    assert_eq!(
        row.go
            .as_ref()
            .and_then(|g| g.to.as_ref())
            .map(|l| l.location),
        at.map(LocationDto::of)
    );
    // Whoever is where the player is says so.
    for r in &v.rows {
        assert_eq!(
            r.here,
            mandate_sim::talk::can_talk(st, e.content(), CharacterId(r.id)),
            "{}",
            r.name
        );
    }
}

// --- Needs you, under way, next up ---------------------------------------------------------------

#[test]
fn needs_you_is_the_engines_list_soonest_first() {
    let c = Arc::new(base());
    for seed in [seed(), 21, 2026] {
        let e = flooded(&c, seed);
        let st = e.state();
        let p = pocket_view(&e).unwrap();
        assert_eq!(p.needs_count as usize, p.needs_you.len());
        // Every pending decision and every offer is a need.
        for d in st.narrative.pending() {
            assert!(
                p.needs_you.iter().any(|n| n.kind == NeedKind::Decision
                    && n.target == NeedTargetDto::Decision { id: d.id.0 }),
                "seed {seed}: decision {}",
                d.id.0
            );
        }
        for a in st.asks.open().filter(|a| a.offered()) {
            let n = p
                .needs_you
                .iter()
                .find(|n| n.target == NeedTargetDto::Ask { id: a.id.0 })
                .unwrap_or_else(|| panic!("seed {seed}: offer {}", a.id.0));
            assert_eq!(n.kind, NeedKind::AskOffer);
            assert_eq!(n.arc, a.arc.as_ref().map(|x| x.run.0));
            assert_eq!(n.due.as_ref().map(|d| d.day_number), Some(a.respond_by.0));
        }
        for b in p.bills.iter().filter(|b| b.can_vote) {
            assert!(
                p.needs_you
                    .iter()
                    .any(|n| n.kind == NeedKind::Vote
                        && n.target == NeedTargetDto::Bill { id: b.id }),
                "seed {seed}: a vote to cast on bill {}",
                b.id
            );
        }
        // Soonest first; what has no date last.
        let days: Vec<u32> = p
            .needs_you
            .iter()
            .map(|n| n.due.as_ref().map_or(u32::MAX, |d| d.day_number))
            .collect();
        assert!(
            days.windows(2).all(|w| w[0] <= w[1]),
            "seed {seed}: {days:?}"
        );
        for n in &p.needs_you {
            assert!(!n.title_key.is_empty());
            assert_eq!(
                n.days_left,
                n.due.as_ref().map(|d| d.day_number - st.calendar.date.0)
            );
        }
        assert!(
            p.needs_you.iter().any(|n| n.arc == flood_run(&e)),
            "seed {seed}: the flood's ask or decision needs the player"
        );
    }
}

/// A beat that is the player's to take needs them in its last turn before
/// the clock; before that it is counted on the arc's row.
#[test]
fn an_arcs_beat_needs_you_in_its_last_turn() {
    let c = Arc::new(base());
    let mut e = flooded(&c, seed());
    let id = flood_run(&e).unwrap();
    let beat_need = |e: &Engine| {
        pocket_view(e)
            .unwrap()
            .needs_you
            .into_iter()
            .find(|n| matches!(&n.target, NeedTargetDto::Beat { arc, .. } if *arc == id))
    };
    let row = |e: &Engine| {
        pocket_view(e)
            .unwrap()
            .arcs
            .into_iter()
            .find(|a| a.id == id)
            .expect("a known arc is a row")
    };
    assert!(row(&e).open_beats >= 1, "the visit is there to take");
    assert!(beat_need(&e).is_none(), "weeks left: not yet a need");
    // Bring the clock inside the turn (test setup: the run's own date).
    let turn = e.turn_days();
    e.setup(|st| {
        let today = st.calendar.date;
        st.arcs.run_mut(mandate_sim::arcs::ArcId(id)).unwrap().clock = Some(today.add_days(turn));
    });
    let n = beat_need(&e).expect("the last turn: a need");
    assert_eq!(n.kind, NeedKind::Beat);
    assert_eq!(n.arc, Some(id));
    assert_eq!(n.days_left, Some(turn));
    assert!(
        n.title_key.starts_with("visit-arc-"),
        "the beat's button: {}",
        n.title_key
    );
    let clock = row(&e).clock.unwrap();
    assert_eq!(clock.days_left, turn);
    assert_eq!(clock.text.key, "now-clock-days");
    assert_eq!(clock.text.args.get("days"), Some(&turn.to_string()));
}

#[test]
fn arcs_under_way_and_what_comes_next() {
    let c = Arc::new(base());
    let e = flooded(&c, seed());
    let st = e.state();
    let p = pocket_view(&e).unwrap();
    // Every running arc the player knows is a row, the soonest clock first.
    assert!(!p.arcs.is_empty());
    let clocks: Vec<u32> = p
        .arcs
        .iter()
        .map(|a| a.clock.as_ref().map_or(u32::MAX, |c| c.due.day_number))
        .collect();
    assert!(clocks.windows(2).all(|w| w[0] <= w[1]), "{clocks:?}");
    for a in &p.arcs {
        let run = st.arcs.run(mandate_sim::arcs::ArcId(a.id)).unwrap();
        let def = e.content().arc(&run.template).unwrap();
        assert_eq!(a.title_key, def.title_key);
        assert_eq!(a.stage.key, def.stage(&run.stage).unwrap().key);
        assert_eq!(
            a.open_beats as usize,
            beats::offers(st, e.content())
                .iter()
                .filter(|o| o.run == run.id)
                .count()
        );
        assert!(!a.place.is_empty());
    }
    // What comes next is dated, ahead, and the soonest of its kind.
    let next = p.next_up.expect("a running arc has a clock ahead");
    assert!(next.key.starts_with("now-next-"));
    assert!(next.date.day_number >= st.calendar.date.0);
    assert_eq!(next.days_left, next.date.day_number - st.calendar.date.0);
    let soonest_clock = clocks.iter().min().copied().unwrap();
    assert!(next.date.day_number <= soonest_clock);
}

// --- Places --------------------------------------------------------------------------------------

#[test]
fn a_place_has_its_reasons_and_its_order() {
    let c = Arc::new(base());
    for seed in [seed(), 21] {
        let e = flooded(&c, seed);
        let st = e.state();
        let t = town_places_view(&e).unwrap();
        let spots: Vec<&SpotView> = t.neighborhoods.iter().flat_map(|h| &h.spots).collect();
        // The ranks are an order of the whole town.
        let mut ranks: Vec<u32> = spots.iter().map(|s| s.rank).collect();
        ranks.sort();
        assert_eq!(ranks, (0..spots.len() as u32).collect::<Vec<_>>());
        // Places that matter come first.
        let last_now = spots
            .iter()
            .filter(|s| !s.now.is_empty())
            .map(|s| s.rank)
            .max();
        let first_rest = spots
            .iter()
            .filter(|s| s.now.is_empty())
            .map(|s| s.rank)
            .min();
        assert!(
            last_now < first_rest,
            "seed {seed}: {last_now:?} {first_rest:?}"
        );
        // The player is somewhere, and it says so.
        let here: Vec<&&SpotView> = spots.iter().filter(|s| s.here).collect();
        assert_eq!(here.len(), 1);
        assert!(here[0].now.iter().any(|r| r.key == "now-place-here"));
        // The flood's visit is done at its place, and the place says so.
        let visit = beats::offers(st, e.content())
            .into_iter()
            .find_map(|o| match o.action {
                BeatAction::Visit { location, .. } if Some(o.run.0) == flood_run(&e) => {
                    Some(location)
                }
                _ => None,
            })
            .expect("the flood's visit is on offer");
        let spot = spots
            .iter()
            .find(|s| s.location.location == LocationDto::of(visit))
            .unwrap();
        let why = spot.now.iter().find(|r| r.kind == NowKind::Arc).unwrap();
        assert_eq!(why.key, "now-place-beat");
        assert_eq!(why.arc, flood_run(&e));
        // A pending decision's scene is where it is.
        for d in st.narrative.pending() {
            if let Some(loc) = d.place {
                let spot = spots
                    .iter()
                    .find(|s| s.location.location == LocationDto::of(loc))
                    .unwrap();
                assert!(
                    spot.now
                        .iter()
                        .any(|r| r.key == "now-place-scene" && r.decision == Some(d.id.0)),
                    "seed {seed}: decision {}",
                    d.id.0
                );
            }
        }
        for s in &spots {
            for r in &s.now {
                assert!(
                    r.key.starts_with("now-place-") && r.args.is_empty(),
                    "{r:?}"
                );
            }
        }
    }
}

// --- Notes: knowledge in words ---------------------------------------------------------------------

/// A note is there only where the gap bears on something in hand, and says
/// nothing where it does not (DEC-091: knowledge is not a checklist).
#[test]
fn a_note_is_there_only_where_it_changes_something() {
    let c = Arc::new(base());
    let mut e = flooded(&c, seed());
    wipe(&mut e);
    let t = town_places_view(&e).unwrap();
    let here = t
        .neighborhoods
        .iter()
        .find(|h| h.spots.iter().any(|s| s.here))
        .unwrap();
    // Where the player stands a canvass is in hand, and its worries are
    // unknown: one sentence.
    let note = here
        .note
        .as_ref()
        .expect("the neighbourhood the player is in");
    assert_eq!(note.key, "know-hood-worries");
    assert_eq!(note.args.get("hood"), Some(&here.name));
    // Nothing counts the rest up: a neighbourhood with nothing in hand
    // says nothing, however little is known of it.
    let st = e.state();
    let in_hand: BTreeSet<u32> = st
        .asks
        .open()
        .flat_map(|a| a.needs.iter().skip(a.step as usize))
        .flat_map(|n| match n {
            mandate_sim::asks::Need::Canvass { hood, .. } => vec![hood.0],
            mandate_sim::asks::Need::TownHall { hoods } => hoods.iter().map(|h| h.0).collect(),
            _ => Vec::new(),
        })
        .chain(
            st.politics
                .elections
                .races
                .iter()
                .flat_map(|r| r.hoods.iter().copied()),
        )
        .collect();
    let quiet: Vec<&TownHoodView> = t
        .neighborhoods
        .iter()
        .filter(|h| h.id != here.id && !in_hand.contains(&h.id))
        .collect();
    assert!(!quiet.is_empty());
    for h in quiet {
        assert_eq!(h.note, None, "{}", h.name);
        assert_eq!(h.concern_knowledge.level, KnowledgeLevelDto::Unknown);
    }
    // Once known, the note goes: the facts are the display.
    let hood = mandate_sim::world::NeighborhoodId(here.id);
    e.setup(|st| {
        let on = st.calendar.date;
        st.player.as_mut().unwrap().knowledge.learn(
            Subject::HoodConcerns { hood },
            Level::Known,
            on,
            mandate_sim::player::knowledge::Source::Lab,
        );
    });
    let t = town_places_view(&e).unwrap();
    let again = t.neighborhoods.iter().find(|h| h.id == here.id).unwrap();
    assert_eq!(again.note, None);
    // A place says nothing unless a fact changes what can be done there.
    for s in t.neighborhoods.iter().flat_map(|h| &h.spots) {
        let shut = match s.location.location {
            LocationDto::Place { place } => !e.state().world.places[place as usize].is_open(),
            _ => false,
        };
        assert_eq!(
            s.note.as_ref().map(|n| n.key.as_str()),
            shut.then_some("know-place-shut")
        );
    }
}

/// Someone who matters now and is only a name says so; someone who does
/// not matter says nothing, whatever the level.
#[test]
fn a_person_only_known_by_name_says_so_when_they_matter() {
    let c = Arc::new(base());
    let mut e = flooded(&c, seed());
    let st = e.state();
    let asker = st
        .asks
        .open()
        .find_map(|a| a.asker_person())
        .expect("an ask from a person is open");
    // The asker at `heard`: they matter (the ask), and are only a name.
    e.setup(|st| {
        let on = st.calendar.date;
        let k = &mut st.player.as_mut().unwrap().knowledge;
        k.lab_omniscient = false;
        k.entries.remove(&Subject::Person { person: asker });
        k.learn(
            Subject::Person { person: asker },
            Level::Heard,
            on,
            mandate_sim::player::knowledge::Source::Lab,
        );
    });
    let v = people_view(&e).unwrap();
    let row = v.rows.iter().find(|r| r.id == asker.0).unwrap();
    assert_eq!(row.level, KnowledgeLevelDto::Heard);
    assert_eq!(
        row.note.as_ref().map(|n| n.key.as_str()),
        Some("know-person-name-only")
    );
    // The notebook's entry for them carries the same sentence.
    let nb = notebook_view(&e).unwrap();
    let entry = nb
        .sections
        .iter()
        .flat_map(|s| &s.entries)
        .find(|x| x.subject == SubjectDto::Person { person: asker.0 })
        .unwrap();
    assert_eq!(entry.note, row.note);
    // Known: the note goes.
    e.setup(|st| {
        let on = st.calendar.date;
        st.player.as_mut().unwrap().knowledge.learn(
            Subject::Person { person: asker },
            Level::Known,
            on,
            mandate_sim::player::knowledge::Source::Lab,
        );
    });
    let v = people_view(&e).unwrap();
    assert_eq!(v.rows.iter().find(|r| r.id == asker.0).unwrap().note, None);
    // Nobody outside "matter now" has a note.
    for r in v.rows.iter().filter(|r| r.group != PersonGroup::Now) {
        assert_eq!(r.note, None, "{}", r.name);
    }
}

// --- The arc view --------------------------------------------------------------------------------

/// A known arc: its title and stage in words, its clock, its cast, the
/// beats that are the player's to take with what each needs, and what has
/// happened so far.
#[test]
fn a_known_arc_shows_where_it_stands_and_what_can_be_done() {
    let c = Arc::new(base());
    let mut e = flooded(&c, seed());
    wipe(&mut e);
    let id = flood_run(&e).unwrap();
    let st = e.state();
    let run = st.arcs.run(mandate_sim::arcs::ArcId(id)).unwrap();
    let def = e.content().arc(FLOOD).unwrap();
    let v = arc_view(&e, id).expect("the break is never hidden");
    assert_eq!(v.level, KnowledgeLevelDto::Known);
    assert_eq!(v.lead, None);
    assert_eq!(
        v.title.as_ref().map(|t| t.key.as_str()),
        Some(def.title_key.as_str())
    );
    let stage = v.stage.as_ref().unwrap();
    assert_eq!(stage.id, run.stage);
    assert_eq!(stage.text.key, def.stage(&run.stage).unwrap().key);
    assert!(stage.event.is_some(), "the stage's entry can be asked why");
    // The stage's line has what it needs: the cast by role and the readings.
    for role in ["hood", "organiser", "hall"] {
        assert!(
            stage.text.args.get(role).is_some_and(|x| !x.is_empty()),
            "{role}"
        );
    }
    assert!(stage.text.args.contains_key("homes") || stage.text.args.contains_key("away"));
    let clock = v.clock.as_ref().unwrap();
    assert_eq!(Some(clock.due.day_number), run.clock.map(|c| c.0));
    assert!(clock.text.key.starts_with("now-clock-"));
    assert_eq!(v.turn.as_ref().map(|t| t.state.as_str()), Some("ahead"));
    // The cast, by role, as the run binds it.
    assert_eq!(v.cast.len(), run.cast.len());
    for (cv, entry) in v.cast.iter().zip(&run.cast) {
        assert_eq!(cv.role, entry.role);
        assert!(!cv.name.is_empty() || matches!(entry.actor, ArcActor::Jurisdiction { .. }));
        match entry.actor {
            ArcActor::Person { id } => {
                assert_eq!(cv.kind, "person");
                assert_eq!(cv.person.as_ref().and_then(|p| p.id), Some(id.0));
            }
            ArcActor::Place { .. } => assert!(cv.kind == "place" && cv.at.is_some()),
            ArcActor::Area { .. } => assert_eq!(cv.kind, "area"),
            ArcActor::Jurisdiction { .. } => assert_eq!(cv.kind, "jurisdiction"),
        }
    }
    // What the player can do: every offer of the run, as the command that
    // takes it, with what it needs.
    let offers: Vec<_> = beats::offers(st, e.content())
        .into_iter()
        .filter(|o| o.run.0 == id)
        .collect();
    assert_eq!(v.beats.len(), offers.len());
    let kinds: BTreeSet<&str> = v.beats.iter().map(|b| b.kind.as_str()).collect();
    assert!(
        kinds.contains("visit") && kinds.contains("ask"),
        "{kinds:?}"
    );
    for b in &v.beats {
        assert!(!b.label.key.is_empty(), "{b:?}");
        match b.kind.as_str() {
            "visit" => {
                assert!(b.verb.as_deref().is_some_and(|x| x.starts_with("arc.")));
                assert!(b.needs.place.is_some() && b.needs.halves >= 1);
                assert!(b.route.as_ref().is_some_and(|r| r.action == "visit"));
            }
            "talk" => {
                assert!(b.topic.is_some() && b.needs.person.is_some());
                assert_eq!(b.needs.halves, 1);
            }
            "decision" => {
                assert!(b.decision.is_some());
                assert_eq!(b.needs.halves, 0);
                // The flood's money is the office's to move: the beat says so.
                assert_eq!(b.needs.power.as_deref(), Some("office"));
            }
            "ask" => {
                assert!(b.ask.is_some() && b.needs.person.is_some());
                assert_eq!(b.needs.halves, 0);
            }
            other => panic!("a beat kind nobody takes: {other}"),
        }
    }
    let days: Vec<u32> = v
        .beats
        .iter()
        .map(|b| b.due.as_ref().map_or(u32::MAX, |d| d.day_number))
        .collect();
    assert!(
        days.windows(2).all(|w| w[0] <= w[1]),
        "soonest first: {days:?}"
    );
    // What has happened: at least the stage it entered, newest first.
    assert!(v.history.iter().any(|h| h.text_key == stage.text.key));
    let dates: Vec<u32> = v.history.iter().map(|h| h.date.day_number).collect();
    assert!(dates.windows(2).all(|w| w[0] >= w[1]));
    // Taking the visit is in the history, and the beat is gone.
    assert!(visit_beat(&mut e));
    let after = arc_view(&e, id).unwrap();
    assert!(!after.beats.iter().any(|b| b.kind == "visit"));
    assert!(after.history.len() > v.history.len());
}

/// The talk beat (the flood's second stage) is listed with its person and
/// the route to them: every kind of beat a player takes has its row.
#[test]
fn a_talk_beat_is_listed_with_its_person() {
    let c = Arc::new(base());
    let mut e = flooded(&c, seed());
    let id = flood_run(&e).unwrap();
    let mut seen = false;
    for _ in 0..20 {
        let Some(v) = arc_view(&e, id) else { break };
        if v.ended.is_some() {
            break;
        }
        if let Some(b) = v.beats.iter().find(|b| b.kind == "talk") {
            seen = true;
            let who = b.needs.person.as_ref().unwrap().id.unwrap();
            assert!(
                b.topic
                    .as_deref()
                    .is_some_and(|t| t.starts_with("choice.arc."))
            );
            assert!(b.label.key.starts_with("talk-arc-"));
            // The person matters now for it, and their row opens the arc.
            let people = people_view(&e).unwrap();
            let row = people.rows.iter().find(|r| r.id == who).unwrap();
            let why = row.now.iter().find(|r| r.kind == NowKind::Arc).unwrap();
            assert_eq!((why.key.as_str(), why.arc), ("now-arc-talk", Some(id)));
            break;
        }
        end_turn(&mut e);
    }
    assert!(seen, "the flood reaches its talk beat within twenty turns");
}

/// An arc that ended can still be read: how it ended and what happened.
#[test]
fn an_ended_arc_can_be_read() {
    let c = Arc::new(base());
    let mut e = flooded(&c, seed());
    let id = flood_run(&e).unwrap();
    for _ in 0..80 {
        if flood_run(&e).is_none() {
            break;
        }
        end_turn(&mut e);
    }
    assert!(flood_run(&e).is_none(), "the idle game lets the flood exit");
    let v = arc_view(&e, id).expect("an ended arc is on record");
    let end = v.ended.expect("it ended");
    assert!(end.text.is_some_and(|t| t.key.starts_with("arc-flood-")));
    assert!(v.beats.is_empty() && v.stage.is_none() && v.clock.is_none());
    assert!(v.title.is_some() && !v.cast.is_empty() && !v.history.is_empty());
    assert!(!pocket_view(&e).unwrap().arcs.iter().any(|a| a.id == id));
    assert_eq!(arc_view(&e, 9_999), None);
}

/// Below `known` the arc view shows a lead only (A3's acceptance), and
/// nothing else in any view speaks of the arc.
#[test]
fn below_known_the_arc_view_is_a_lead_only() {
    let c = Arc::new(hidden_flood(None));
    let mut e = flooded(&c, seed());
    wipe(&mut e);
    let id = flood_run(&e).unwrap();
    let st = e.state();
    let run = st.arcs.run(mandate_sim::arcs::ArcId(id)).unwrap().clone();
    let def = e.content().arc(FLOOD).unwrap().clone();
    assert!(def.stage(&run.stage).unwrap().hidden);
    let people: Vec<String> = run
        .cast
        .iter()
        .filter_map(|x| match x.actor {
            ArcActor::Person { id } => {
                characters::profile(&st.politics, &st.world, id).map(|p| p.full_name())
            }
            _ => None,
        })
        .collect();
    assert!(!people.is_empty());
    let speaks_of_it = |text: &str| {
        text.contains(&format!("\"{}\"", def.title_key))
            || def
                .stages
                .iter()
                .any(|s| text.contains(&format!("\"{}\"", s.key)))
    };

    // Unknown: no such arc, as far as any view says.
    assert_eq!(
        mandate_protocol::arcs::arc_level(&e, &run, &def),
        Level::Unknown
    );
    assert_eq!(arc_view(&e, id), None);
    let pocket = pocket_view(&e).unwrap();
    assert!(!pocket.arcs.iter().any(|a| a.id == id));
    assert!(!pocket.needs_you.iter().any(|n| n.arc == Some(id)));
    assert!(pocket.next_up.as_ref().is_none_or(|n| n.arc != Some(id)));
    assert_eq!(notebook_view(&e).unwrap().arc_leads, None);
    for (name, text) in [
        ("people", json(&people_view(&e))),
        ("pocket", json(&pocket)),
        ("notebook", json(&notebook_view(&e))),
    ] {
        assert!(!speaks_of_it(&text), "{name} speaks of an arc nobody knows");
    }
    let town = town_places_view(&e).unwrap();
    for s in town.neighborhoods.iter().flat_map(|h| &h.spots) {
        assert!(
            !s.now.iter().any(|r| r.kind == NowKind::Arc),
            "{:?}",
            s.location
        );
    }
    for r in people_view(&e).unwrap().rows {
        assert!(!r.now.iter().any(|x| x.kind == NowKind::Arc), "{}", r.name);
    }

    // The player takes the visit the place lists: now they have heard of
    // it, and the view is a lead and nothing else.
    assert!(
        visit_beat(&mut e),
        "the place lists the arc's verb, known or not"
    );
    let run = e
        .state()
        .arcs
        .run(mandate_sim::arcs::ArcId(id))
        .unwrap()
        .clone();
    assert_eq!(
        mandate_protocol::arcs::arc_level(&e, &run, &def),
        Level::Heard
    );
    let v = arc_view(&e, id).expect("heard of: a lead");
    assert_eq!(v.level, KnowledgeLevelDto::Heard);
    let lead = v.lead.clone().expect("a lead");
    assert_eq!(lead.text.key, "know-arc-lead");
    assert!(lead.text.args.get("where").is_some_and(|w| !w.is_empty()));
    assert!(lead.route.is_some(), "it says where to go and ask");
    assert_eq!(
        (
            &v.title, &v.at, &v.began, &v.days, &v.stage, &v.clock, &v.turn
        ),
        (&None, &None, &None, &None, &None, &None, &None)
    );
    assert!(v.cast.is_empty() && v.beats.is_empty() && v.history.is_empty());
    assert!(v.headline.is_none() && v.ended.is_none());
    // It names nobody and nothing of the matter.
    let text = json(&v);
    assert!(!speaks_of_it(&text), "{text}");
    for name in &people {
        assert!(!text.contains(name.as_str()), "the lead names {name}");
    }
    // The notebook has the same lead; Today has no row for it.
    assert_eq!(notebook_view(&e).unwrap().arc_leads, Some(vec![lead]));
    let pocket = pocket_view(&e).unwrap();
    assert!(!pocket.arcs.iter().any(|a| a.id == id));
    assert!(!pocket.needs_you.iter().any(|n| n.arc == Some(id)));
    assert!(!speaks_of_it(&json(&pocket)));
    assert!(!speaks_of_it(&json(&people_view(&e))));

    // A beat that teaches `known` opens the whole view.
    let c = Arc::new(hidden_flood(Some(Level::Known)));
    let mut e = flooded(&c, seed());
    wipe(&mut e);
    let id = flood_run(&e).unwrap();
    assert_eq!(arc_view(&e, id), None);
    assert!(visit_beat(&mut e));
    let v = arc_view(&e, id).unwrap();
    assert_eq!(v.level, KnowledgeLevelDto::Known);
    assert!(v.lead.is_none() && v.title.is_some() && v.stage.is_some() && !v.cast.is_empty());
    assert_eq!(notebook_view(&e).unwrap().arc_leads, None);
    assert!(pocket_view(&e).unwrap().arcs.iter().any(|a| a.id == id));
}

// --- Redaction -----------------------------------------------------------------------------------

/// Everyone a view may name with nothing in the ledger: those named by
/// what is aimed at the player (an open ask, a pending decision), the cast
/// of an arc in the open, and the rival.
fn named_to_the_player(e: &Engine) -> BTreeSet<CharacterId> {
    let st = e.state();
    let mut out = BTreeSet::new();
    for a in st.asks.open() {
        out.extend(a.asker_person());
        for x in &a.cast {
            if let Actor::Person { id } = x.actor {
                out.insert(id);
            }
        }
        for n in &a.needs {
            match n {
                mandate_sim::asks::Need::Talk { person }
                | mandate_sim::asks::Need::Quiet { person } => {
                    out.insert(*person);
                }
                _ => {}
            }
        }
    }
    for d in st.narrative.pending() {
        for x in &d.cast {
            if let Actor::Person { id } = x.actor {
                out.insert(id);
            }
        }
    }
    for run in &st.arcs.runs {
        let def = e.content().arc(&run.template).unwrap();
        if mandate_protocol::arcs::arc_level(e, run, def) >= Level::Known {
            for x in &run.cast {
                if let ArcActor::Person { id } = x.actor {
                    out.insert(id);
                }
            }
        }
    }
    let home = mandate_sim::player::home(st).unwrap();
    out.extend(st.politics.characters.towns[home.index()].rival);
    out
}

/// No reason names a person or a matter the player does not know: with an
/// empty ledger the people list holds only those something aimed at the
/// player names, and no reason, note or need carries anyone else's name.
#[test]
fn no_reason_names_what_the_player_does_not_know() {
    let c = Arc::new(base());
    for seed in [seed(), 21, 2026] {
        let mut e = flooded(&c, seed);
        wipe(&mut e);
        let st = e.state();
        let allowed = named_to_the_player(&e);
        let named: BTreeSet<String> = allowed
            .iter()
            .chain([&me(&e)])
            .filter_map(|id| characters::profile(&st.politics, &st.world, *id))
            .map(|p| p.full_name())
            .collect();
        // Everyone else in the world: nobody may name them.
        let strangers: Vec<String> = (0..st.politics.characters.people.len() as u32)
            .map(CharacterId)
            .filter(|id| !allowed.contains(id) && *id != me(&e))
            .filter_map(|id| characters::profile(&st.politics, &st.world, id))
            .map(|p| p.full_name())
            // A namesake elsewhere in the country is not the person named.
            .filter(|n| !named.contains(n))
            .collect();
        assert!(strangers.len() > 20, "seed {seed}: a town of strangers");
        let people = people_view(&e).unwrap();
        for r in &people.rows {
            assert!(
                allowed.contains(&CharacterId(r.id)),
                "seed {seed}: {} is listed, and nothing names them to the player",
                r.name
            );
            assert_ne!(
                r.group,
                PersonGroup::Office,
                "nobody is known to hold office"
            );
        }
        let pocket = pocket_view(&e).unwrap();
        let town = town_places_view(&e).unwrap();
        let reasons = json(&(
            &people,
            &pocket.needs_you,
            &pocket.arcs,
            &pocket.next_up,
            town.neighborhoods
                .iter()
                .flat_map(|h| {
                    h.spots
                        .iter()
                        .map(|s| (&s.now, &s.note))
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>(),
            town.neighborhoods
                .iter()
                .map(|h| &h.note)
                .collect::<Vec<_>>(),
        ));
        // Every arc view (a lead below known, none for an arc unknown) and
        // the notebook's notes and leads.
        let arcs: Vec<_> = st.arcs.runs.iter().map(|r| arc_view(&e, r.id.0)).collect();
        let lines = json(&(&arcs, notebook_view(&e)));
        for name in &strangers {
            assert!(
                !reasons.contains(&format!("\"{name}\"")),
                "seed {seed}: a reason, a note or a need names {name}"
            );
            assert!(
                !lines.contains(&format!("\"{name}\"")),
                "seed {seed}: an arc line or a notebook note names {name}"
            );
        }
        // With the ledger back, the list grows and its people are known.
        let full = flooded(&c, seed);
        assert!(people_view(&full).unwrap().rows.len() > people.rows.len());
    }
}

// --- Determinism ---------------------------------------------------------------------------------

/// The order is the same twice, and the same through a save and load.
#[test]
fn the_views_are_the_same_twice_and_through_a_save_and_load() {
    let c = Arc::new(base());
    for seed in [seed(), 21] {
        let mut e = flooded(&c, seed);
        assert!(visit_beat(&mut e));
        for _ in 0..3 {
            end_turn(&mut e);
        }
        let first = all_views(&e);
        assert_eq!(first, all_views(&e), "seed {seed}: the same twice");
        let req = mandate_save::SaveRequest {
            run_id: "views".into(),
            revision: 1,
            parent_revision: None,
            device_id: "test".into(),
            saved_at: "2026-11-25T00:00:00Z".into(),
            protocol_version: PROTOCOL_VERSION,
            run_mode: mandate_save::RunMode::Ironman,
        };
        let (bytes, _) = mandate_save::encode(e.state(), &req).unwrap();
        let (state, _) = mandate_save::decode(&bytes).unwrap();
        let loaded = Engine::from_state(state, c.clone()).unwrap();
        assert_eq!(
            loaded.state_hash(),
            e.state_hash(),
            "seed {seed}: the save is whole"
        );
        for ((name, a), (_, b)) in first.iter().zip(all_views(&loaded)) {
            assert_eq!(a, &b, "seed {seed}: {name} through a save and load");
        }
        // And a second game from the same seed and commands sees the same.
        let mut twin = flooded(&c, seed);
        assert!(visit_beat(&mut twin));
        for _ in 0..3 {
            end_turn(&mut twin);
        }
        assert_eq!(first, all_views(&twin), "seed {seed}: the same game twice");
    }
}

// --- K4: views never write -----------------------------------------------------------------------

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

/// The K4 guard, for the new queries: with the same commands, a ledger that
/// knows everything and the default one leave identical worlds, queried at
/// every turn or never.
#[test]
fn a_query_writes_nothing_and_an_omniscient_ledger_leaves_the_same_world() {
    let c = Arc::new(base());
    for seed in [seed(), 21] {
        let mut plain = flooded(&c, seed);
        let mut lab = flooded(&c, seed);
        let mut quiet = flooded(&c, seed);
        lab.set_lab_omniscient(true);
        for turn in 0..24 {
            for e in [&plain, &lab] {
                let before = e.state_hash();
                let views = all_views(e);
                assert!(views.len() >= 6);
                assert_eq!(
                    e.state_hash(),
                    before,
                    "seed {seed}, turn {turn}: a view wrote"
                );
            }
            // The same commands in all three: a beat taken, then the turn.
            if turn == 1 {
                for e in [&mut plain, &mut lab, &mut quiet] {
                    assert!(visit_beat(e), "seed {seed}");
                }
            }
            for e in [&mut plain, &mut lab, &mut quiet] {
                end_turn(e);
            }
            if turn % 6 == 5 {
                assert_eq!(world(&plain), world(&lab), "seed {seed}, turn {turn}");
                assert_eq!(
                    plain.state_hash(),
                    quiet.state_hash(),
                    "seed {seed}, turn {turn}: queried or not, the same game"
                );
            }
        }
        // The two ledgers differ, and so do the views: one sees more.
        assert_ne!(
            plain.state().player.as_ref().unwrap().knowledge,
            lab.state().player.as_ref().unwrap().knowledge
        );
        let (a, b) = (people_view(&plain).unwrap(), people_view(&lab).unwrap());
        assert!(
            a.rows
                .iter()
                .any(|r| r.level != KnowledgeLevelDto::Detailed)
        );
        assert!(
            b.rows
                .iter()
                .all(|r| r.level == KnowledgeLevelDto::Detailed)
        );
    }
}

/// The session answers the two new queries, and neither changes the state.
#[test]
fn the_session_answers_people_and_arc() {
    let mut s = Session::new().unwrap();
    let send = |s: &mut Session, id: u32, body: RequestBody| -> Response {
        let req = serde_json::to_string(&Request {
            protocol_version: PROTOCOL_VERSION,
            request_id: id,
            body,
        })
        .unwrap();
        serde_json::from_str(&s.handle_json(&req)).unwrap()
    };
    send(
        &mut s,
        1,
        RequestBody::NewGame(NewGameParams {
            seed: 24,
            size: WorldSizeDto::Small,
            scenario: None,
            storyteller: None,
            local_knowledge: false,
        }),
    );
    for i in 0..4 {
        send(&mut s, 10 + i, RequestBody::Command(CommandDto::EndTurn));
    }
    let before = s.engine().unwrap().state_hash();
    let r = send(&mut s, 20, RequestBody::Query(QueryDto::People));
    let ResponseResult::Ok {
        body: ResponseBody::People(people),
    } = r.result
    else {
        panic!("people: {:?}", r.result);
    };
    assert!(!people.rows.is_empty());
    let r = send(&mut s, 21, RequestBody::Query(QueryDto::Pocket));
    let ResponseResult::Ok {
        body: ResponseBody::Pocket(pocket),
    } = r.result
    else {
        panic!("pocket: {:?}", r.result);
    };
    for a in &pocket.arcs {
        let r = send(
            &mut s,
            30 + a.id,
            RequestBody::Query(QueryDto::Arc { id: a.id }),
        );
        let ResponseResult::Ok {
            body: ResponseBody::Arc(v),
        } = r.result
        else {
            panic!("arc {}: {:?}", a.id, r.result);
        };
        assert_eq!(v.id, a.id);
        assert_eq!(v.title.map(|t| t.key), Some(a.title_key.clone()));
    }
    // No such arc is a bad request, as an arc nobody knows of would be.
    let r = send(&mut s, 90, RequestBody::Query(QueryDto::Arc { id: 9_999 }));
    assert!(matches!(
        r.result,
        ResponseResult::Err { error } if error.code == ErrorCode::BadRequest
    ));
    assert_eq!(s.engine().unwrap().state_hash(), before);
    // The wire names: `{"type":"people"}` and `{"type":"arc","id":3}`.
    assert_eq!(json(&QueryDto::People), r#"{"type":"people"}"#);
    assert_eq!(json(&QueryDto::Arc { id: 3 }), r#"{"type":"arc","id":3}"#);
}

// --- A vote on the player's bill, and a bill's note ------------------------------------------------

/// Draft and introduce a bill of the player's own.
fn own_bill(e: &mut Engine) -> u32 {
    let template = e
        .content()
        .policies
        .values()
        .find(|t| t.tier == mandate_content::TierKey::Local && t.unlock.is_none())
        .unwrap()
        .id
        .clone();
    let cmd = e.route(PlayerCommand::DraftPolicy {
        template,
        params: Default::default(),
    });
    let id = e.apply(cmd).unwrap().result.unwrap().created.unwrap();
    let cmd = e.route(PlayerCommand::IntroducePolicy { policy: id });
    e.apply(cmd).unwrap().result.unwrap();
    id
}

/// A vote on the player's own bill within the turn: away from the chamber
/// everyone with a vote on it matters, if the player has heard of them;
/// where the count is given member by member, only the undecided do. A
/// member the player hardly knows says what that costs.
#[test]
fn a_vote_on_the_players_bill_names_who_has_a_vote() {
    let c = Arc::new(base());
    let mut e = game(&c, 21);
    let bill = own_bill(&mut e);
    let turn = e.turn_days();
    // Until the vote falls within the turn.
    let mut due = None;
    for _ in 0..40 {
        let p = pocket_view(&e).unwrap();
        let Some(b) = p.bills.iter().find(|b| b.id == bill) else {
            break;
        };
        if b.status == "under_consideration"
            && let Some(d) = &b.next_vote
            && d.day_number <= e.state().calendar.date.0 + turn
        {
            due = Some(d.day_number);
            break;
        }
        end_turn(&mut e);
    }
    let due = due.expect("the bill comes to a vote within forty turns");
    // Away from the chamber (introducing the bill took the player there).
    e.apply(Command::Player(PlayerCommand::MoveTo {
        location: mandate_sim::places::Location::Home,
    }))
    .unwrap();
    let st = e.state();
    let constitution = e
        .content()
        .constitutions
        .get(&st.meta.constitution)
        .unwrap();
    let pl = st.player.as_ref().unwrap();
    let members: Vec<CharacterId> = mandate_sim::politics::legislature_of(
        &st.politics,
        constitution,
        mandate_sim::world::Jurisdiction::town(pl.home()),
    )
    .iter()
    .filter(|id| **id != pl.politician)
    .filter_map(|id| st.politics.get(*id).map(|p| p.character))
    .collect();
    assert!(members.len() >= 2);
    // One member the player has never heard of, one only by name.
    let (stranger, name_only) = (members[0], members[1]);
    e.setup(|st| {
        let on = st.calendar.date;
        let k = &mut st.player.as_mut().unwrap().knowledge;
        k.lab_omniscient = false;
        k.entries.remove(&Subject::Person { person: stranger });
        k.entries.remove(&Subject::Person { person: name_only });
        k.learn(
            Subject::Person { person: name_only },
            Level::Heard,
            on,
            mandate_sim::player::knowledge::Source::Lab,
        );
    });
    let named = named_to_the_player(&e);
    let v = people_view(&e).unwrap();
    let vote = |v: &PeopleView, who: CharacterId| {
        v.rows
            .iter()
            .find(|r| r.id == who.0)
            .and_then(|r| r.now.iter().find(|x| x.kind == NowKind::Vote).cloned())
    };
    for m in &members {
        let level = e
            .state()
            .player
            .as_ref()
            .unwrap()
            .knowledge
            .level(&Subject::Person { person: *m });
        match vote(&v, *m) {
            Some(r) => {
                assert!(level >= Level::Heard, "a vote names nobody unheard of");
                assert_eq!(r.key, "now-vote-member");
                assert_eq!(r.bill, Some(bill));
                assert_eq!(r.due.map(|d| d.day_number), Some(due));
                assert!(r.title.is_some());
            }
            None => assert!(level < Level::Heard, "every member heard of has the reason"),
        }
    }
    // The stranger is not listed for the vote (only if something aimed at
    // the player names them).
    assert_eq!(
        v.rows.iter().any(|r| r.id == stranger.0),
        named.contains(&stranger)
    );
    assert_eq!(vote(&v, stranger), None);
    // Lobbying someone known only by name works less often: the row says so.
    let row = v.rows.iter().find(|r| r.id == name_only.0).unwrap();
    assert_eq!(row.group, PersonGroup::Now);
    assert_eq!(
        row.note.as_ref().map(|n| n.key.as_str()),
        Some("know-person-lobby")
    );
    // In the chamber the count is member by member: only the undecided matter.
    let home = e.state().player.as_ref().unwrap().home();
    let places: Vec<_> = e.state().world.municipalities[home.index()].places.clone();
    let chamber = places.into_iter().find(|p| {
        let place = &e.state().world.places[p.index()];
        place.is_open()
            && e.content().place_kind(place.kind.key()).is_some_and(|d| {
                mandate_sim::places::kind_offers(
                    d,
                    mandate_sim::places::Offer::Service("whip_exact"),
                )
            })
    });
    let chamber = chamber.expect("a place gives the exact count");
    e.apply(Command::Player(PlayerCommand::MoveTo {
        location: mandate_sim::places::Location::Place { place: chamber },
    }))
    .unwrap();
    let office = office_view(&e).unwrap();
    let b = office.bills.iter().find(|b| b.id == bill).unwrap();
    assert!(b.whip_exact && !b.whip.is_empty());
    let v = people_view(&e).unwrap();
    for w in b.whip.iter().filter(|w| !w.is_player) {
        let who = e
            .state()
            .politics
            .get(mandate_sim::politics::PoliticianId(w.politician))
            .unwrap()
            .character;
        let heard = w.level >= KnowledgeLevelDto::Heard;
        assert_eq!(
            vote(&v, who).map(|r| r.key),
            (w.lean == "undecided" && heard).then(|| "now-vote-undecided".to_string()),
            "{} leans {}",
            w.name,
            w.lean
        );
    }
}

/// A bill that is the player's to vote on says, in one sentence, that
/// their voters' side is a rough estimate, until support for it is known;
/// an old reading says how old.
#[test]
fn a_bill_in_hand_says_what_is_not_known_of_its_support() {
    let c = Arc::new(base());
    // A game with a bill awaiting the player's vote (within thirty turns).
    let mut found = None;
    'seeds: for seed in [seed(), 21, 2026, 45] {
        let mut e = game(&c, seed);
        for _ in 0..30 {
            if pocket_view(&e).unwrap().bills.iter().any(|b| b.can_vote) {
                found = Some(e);
                break 'seeds;
            }
            end_turn(&mut e);
        }
    }
    let mut e = found.expect("a bill comes before the player in one of four worlds");
    wipe(&mut e);
    let bill = pocket_view(&e)
        .unwrap()
        .bills
        .into_iter()
        .find(|b| b.can_vote)
        .unwrap();
    assert_eq!(
        bill.note.as_ref().map(|n| n.key.as_str()),
        Some("know-bill-support")
    );
    let office = office_view(&e).unwrap();
    let full = office.bills.iter().find(|b| b.id == bill.id).unwrap();
    assert_eq!(full.note, bill.note);
    // A bill that is not the player's to act on says nothing.
    for b in office
        .bills
        .iter()
        .filter(|b| !b.can_vote && !b.sponsor_is_player)
    {
        assert_eq!(b.note, None, "bill {}", b.id);
    }
    // Known: nothing to say. Then the reading grows old: it says so.
    let town = e.state().player.as_ref().unwrap().home();
    let subject = Subject::PolicySupport {
        town,
        policy: mandate_sim::player::knowledge::PolicyRef::Template {
            template: full.template.clone(),
        },
    };
    let today = e.state().calendar.date;
    e.setup(|st| {
        let k = &mut st.player.as_mut().unwrap().knowledge;
        k.learn(
            subject.clone(),
            Level::Detailed,
            today,
            mandate_sim::player::knowledge::Source::Lab,
        );
        k.read(
            subject.clone(),
            mandate_sim::player::knowledge::Reading {
                on: today,
                source: mandate_sim::player::knowledge::Source::Lab,
                values: vec![("support".into(), 10)],
            },
        );
    });
    let note = |e: &Engine| {
        pocket_view(e)
            .unwrap()
            .bills
            .into_iter()
            .find(|b| b.id == bill.id)
            .unwrap()
            .note
    };
    assert_eq!(note(&e), None);
    let old = mandate_core::SimDate(today.0 - e.content().knowledge.fresh_days - 1);
    e.setup(|st| {
        let k = &mut st.player.as_mut().unwrap().knowledge;
        k.entries.get_mut(&subject).unwrap().reading =
            Some(mandate_sim::player::knowledge::Reading {
                on: old,
                source: mandate_sim::player::knowledge::Source::Lab,
                values: vec![("support".into(), 10)],
            });
    });
    let n = note(&e).expect("an old reading");
    assert_eq!(n.key, "know-figures-stale");
    let cal = old.calendar();
    assert_eq!(
        n.args.get("month_key"),
        Some(&format!("month-{}", cal.month))
    );
    assert_eq!(n.args.get("year"), Some(&cal.year.to_string()));
    // The notebook's entry for it shows the figure, and says the same.
    let nb = notebook_view(&e).unwrap();
    let entry = nb
        .sections
        .iter()
        .flat_map(|s| &s.entries)
        .find(|x| x.stale)
        .expect("the stale entry");
    assert_eq!(
        entry.note.as_ref().map(|n| n.key.as_str()),
        Some("know-figures-stale")
    );
}

/// A place that has shut says so: nothing can be done there.
#[test]
fn a_place_that_has_shut_says_so() {
    let c = Arc::new(base());
    let mut e = game(&c, 21);
    let t = town_places_view(&e).unwrap();
    let open = t
        .neighborhoods
        .iter()
        .flat_map(|h| &h.spots)
        .find_map(|s| match s.location.location {
            LocationDto::Place { place } if !s.here && s.note.is_none() => Some(place),
            _ => None,
        })
        .unwrap();
    e.setup(|st| {
        let since = st.calendar.date;
        st.world.places[open as usize].status = mandate_sim::world::PlaceStatus::Closed { since };
    });
    let t = town_places_view(&e).unwrap();
    let spot = t
        .neighborhoods
        .iter()
        .flat_map(|h| &h.spots)
        .find(|s| s.location.location == LocationDto::Place { place: open })
        .unwrap();
    assert_eq!(
        spot.note.as_ref().map(|n| n.key.as_str()),
        Some("know-place-shut")
    );
}

// --- Fixtures for the web task ---------------------------------------------------------------------

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../web/src/components/fixtures/pocket")
}

/// The fixtures the web task (R2) builds against: each new or extended
/// view as the engine made it, from one scripted game. Written with
/// `MANDATE_WRITE_FIXTURES=1 cargo test -p mandate-wasm --test pocket_views
/// fixtures`; otherwise the files on disk must still read as today's views
/// (a protocol change that leaves them behind fails here).
#[test]
fn fixtures_are_views_the_engine_made() {
    // A known arc on the day it is raised, its first beats open; the
    // player at the place its visit is done, and knowing forty of the
    // town's people at every level (a career's worth of rows, so the folds
    // have something to fold).
    let c = Arc::new(base());
    let mut e = flooded(&c, seed());
    let id = flood_run(&e).unwrap();
    let hall = beats::offers(e.state(), e.content())
        .into_iter()
        .find_map(|o| match o.action {
            BeatAction::Visit { location, .. } if o.run.0 == id => Some(location),
            _ => None,
        })
        .unwrap();
    e.apply(Command::Player(PlayerCommand::MoveTo { location: hall }))
        .unwrap();
    let home = mandate_sim::player::home(e.state()).unwrap();
    let townsfolk: Vec<CharacterId> = (0..e.state().politics.characters.people.len() as u32)
        .map(CharacterId)
        .filter(|id| {
            let st = e.state();
            *id != me(&e)
                && characters::municipality_of(&st.politics, &st.world, *id) == Some(home.0)
        })
        .take(40)
        .collect();
    e.setup(|st| {
        let on = st.calendar.date;
        let k = &mut st.player.as_mut().unwrap().knowledge;
        for (i, person) in townsfolk.iter().enumerate() {
            let level = [Level::Heard, Level::Known, Level::Detailed][i % 3];
            k.learn(
                Subject::Person { person: *person },
                level,
                on,
                mandate_sim::player::knowledge::Source::Lab,
            );
        }
    });
    // The same arc, only heard of (its first stage hidden, one visit made).
    let hc = Arc::new(hidden_flood(None));
    let mut heard = flooded(&hc, seed());
    wipe(&mut heard);
    assert!(visit_beat(&mut heard));
    let heard_id = flood_run(&heard).unwrap();
    // And ended.
    let mut over = flooded(&c, seed());
    let over_id = flood_run(&over).unwrap();
    for _ in 0..80 {
        if flood_run(&over).is_none() {
            break;
        }
        end_turn(&mut over);
    }
    let files: Vec<(&str, serde_json::Value)> = vec![
        (
            "people.json",
            serde_json::to_value(people_view(&e).unwrap()).unwrap(),
        ),
        (
            "pocket.json",
            serde_json::to_value(pocket_view(&e).unwrap()).unwrap(),
        ),
        (
            "town.json",
            serde_json::to_value(town_places_view(&e).unwrap()).unwrap(),
        ),
        (
            "notebook.json",
            serde_json::to_value(notebook_view(&e).unwrap()).unwrap(),
        ),
        (
            "arc.json",
            serde_json::to_value(arc_view(&e, id).unwrap()).unwrap(),
        ),
        (
            "arc-heard.json",
            serde_json::to_value(arc_view(&heard, heard_id).unwrap()).unwrap(),
        ),
        (
            "notebook-arc-lead.json",
            serde_json::to_value(notebook_view(&heard).unwrap()).unwrap(),
        ),
        (
            "arc-ended.json",
            serde_json::to_value(arc_view(&over, over_id).unwrap()).unwrap(),
        ),
    ];
    let dir = fixture_dir();
    if std::env::var_os("MANDATE_WRITE_FIXTURES").is_some() {
        std::fs::create_dir_all(&dir).unwrap();
        for (name, v) in &files {
            let mut text = serde_json::to_string_pretty(v).unwrap();
            text.push('\n');
            std::fs::write(dir.join(name), text).unwrap();
        }
    }
    let read = |name: &str| -> String {
        std::fs::read_to_string(dir.join(name)).unwrap_or_else(|_| {
            panic!("{name} is missing: MANDATE_WRITE_FIXTURES=1 cargo test -p mandate-wasm --test pocket_views fixtures")
        })
    };
    let stale = "the fixture no longer reads as the view: write the fixtures again";
    let people: PeopleView = serde_json::from_str(&read("people.json")).expect(stale);
    let pocket: PocketView = serde_json::from_str(&read("pocket.json")).expect(stale);
    let town: TownPlacesView = serde_json::from_str(&read("town.json")).expect(stale);
    let _: NotebookView = serde_json::from_str(&read("notebook.json")).expect(stale);
    let arc: ArcView = serde_json::from_str(&read("arc.json")).expect(stale);
    let lead: ArcView = serde_json::from_str(&read("arc-heard.json")).expect(stale);
    let nb: NotebookView = serde_json::from_str(&read("notebook-arc-lead.json")).expect(stale);
    let ended: ArcView = serde_json::from_str(&read("arc-ended.json")).expect(stale);
    // What the web task needs each to show.
    assert!(people.groups.iter().any(|g| g.group == PersonGroup::Now));
    assert!(people.groups.len() >= 3, "sections to fold");
    assert!(people.rows.len() > 12, "a fold with more than it shows");
    assert!(people.rows.iter().any(|r| r.here), "somebody is here");
    assert!(!pocket.needs_you.is_empty() && !pocket.arcs.is_empty());
    assert!(
        town.neighborhoods
            .iter()
            .flat_map(|h| &h.spots)
            .any(|s| s.now.len() > 1 || !s.here && !s.now.is_empty())
    );
    assert!(arc.lead.is_none() && arc.beats.len() >= 2 && !arc.history.is_empty());
    assert!(lead.lead.is_some() && lead.title.is_none());
    assert!(nb.arc_leads.is_some_and(|l| l.len() == 1));
    assert!(ended.ended.is_some());
}
