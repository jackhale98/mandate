//! M6 note 19 in whole games: the person speaks as the conversation opens
//! (the greeting is the opener the first reply would have started with,
//! and that reply leaves it out), nobody says they never go somewhere they
//! live in, work in, hold office over or are standing in, officeholders
//! answer for every neighbourhood of their jurisdiction, and "the whole
//! town" is a subject of its own.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use mandate_content::conversation::TalkPart;
use mandate_content::{ContentBundle, bundle::read_dir_sources, load};
use mandate_sim::actions::{ActionResult, PlayerCommand};
use mandate_sim::characters::knows::{self, Openness};
use mandate_sim::characters::{self, CharacterId};
use mandate_sim::places::Location;
use mandate_sim::player::knowledge::Level;
use mandate_sim::talk::context::Ctx;
use mandate_sim::talk::{self, AboutRef, TalkTarget, reply};
use mandate_sim::world::{MunicipalityId, NeighborhoodId};
use mandate_sim::{Command, Engine, EngineError};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

fn content() -> Arc<ContentBundle> {
    let src =
        read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap();
    Arc::new(load(&src).unwrap().0)
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

fn me(e: &Engine) -> CharacterId {
    characters::player_character(e.state()).unwrap()
}

fn town(e: &Engine) -> MunicipalityId {
    mandate_sim::player::home(e.state()).unwrap()
}

/// A councillor (not the player), and the player where they are now.
fn colleague_here(e: &mut Engine) -> CharacterId {
    let at = mandate_sim::world::Jurisdiction::town(town(e));
    let c = mandate_sim::politics::body_members_at(&e.state().politics, at)
        .into_iter()
        .filter_map(|p| e.state().politics.get(p))
        .find(|p| !p.is_player)
        .map(|p| p.character)
        .unwrap();
    if !talk::can_talk(e.state(), e.content(), c) {
        let l = characters::routines::whereabouts(e.state(), e.content(), c).unwrap();
        e.apply(Command::Player(PlayerCommand::MoveTo { location: l }))
            .unwrap();
    }
    assert!(talk::can_talk(e.state(), e.content(), c));
    c
}

fn say(
    e: &mut Engine,
    person: CharacterId,
    topic: &str,
    about: Option<AboutRef>,
) -> Result<ActionResult, EngineError> {
    e.apply(Command::Player(PlayerCommand::Talk {
        to: TalkTarget::Person { person },
        topic: topic.into(),
        about,
        late_night: false,
    }))
    .map(|o| o.result.unwrap())
}

fn exchange(e: &Engine, r: &ActionResult) -> talk::Exchange {
    let id = talk::ExchangeId(r.created.unwrap());
    e.state()
        .player
        .as_ref()
        .unwrap()
        .talk
        .exchange(id)
        .unwrap()
        .clone()
}

#[test]
fn the_greeting_is_the_first_replys_opener_and_the_reply_leaves_it_out() {
    let c = content();
    for seed in [3, 17, 41] {
        let mut e = game(seed, &c);
        let who = colleague_here(&mut e);
        let st = e.state();
        let greeting = talk::greeting(st, &c, who).expect("a greeting as the conversation opens");
        assert_eq!(greeting.part, TalkPart::Opener);
        // What a reply with an opener would have opened with, now: the same line.
        let pl = st.player.as_ref().unwrap();
        let open = Openness::default();
        let ctx = Ctx {
            st,
            content: &c,
            town: town(&e),
            me: me(&e),
            speaker: who,
            about: None,
            here: pl.location,
            cap: knows::discretion_cap(st, &c, who, me(&e), open),
        };
        let topic = c.talk_topic("topic.worries").unwrap();
        let week = talk::week_of(st.calendar.date);
        let composed = reply::compose(&ctx, topic, pl.talk.used_by(who), week, 0);
        let opener = composed
            .parts
            .iter()
            .find(|p| p.part == TalkPart::Opener)
            .expect("a reply with an opener opens");
        assert_eq!(opener.line.id, greeting.line, "seed {seed}");
        // The first reply: no opener (the greeting took its place); the
        // greeting is remembered as said, and is not given again today.
        let r = say(&mut e, who, "topic.worries", None).unwrap();
        let x = exchange(&e, &r);
        assert!(
            x.parts.iter().all(|p| p.part != TalkPart::Opener),
            "seed {seed}: the first reply repeats an opener: {:?}",
            x.parts
        );
        let used = e.state().player.as_ref().unwrap().talk.used_by(who);
        assert!(used.iter().any(|u| u.line == greeting.line));
        assert!(talk::greeting(e.state(), &c, who).is_none());
        assert!(!talk::opens_now(e.state(), who));
        // A later conversation the same day opens as the greeting would.
        let r = say(&mut e, who, "topic.who_runs", None).unwrap();
        let x = exchange(&e, &r);
        if let Some(o) = x.parts.iter().find(|p| p.part == TalkPart::Opener) {
            assert_ne!(o.line, greeting.line, "the guard: another opener");
        }
    }
}

#[test]
fn a_hooks_choice_after_the_greeting_takes_its_place_too() {
    // A conversation that opens with a choice also uses up the greeting:
    // the engine knows from the day's log, whatever was asked.
    let c = content();
    let mut e = game(5, &c);
    let who = colleague_here(&mut e);
    assert!(talk::opens_now(e.state(), who));
    say(&mut e, who, "topic.small_talk", None).unwrap();
    assert!(!talk::opens_now(e.state(), who));
    assert!(talk::greeting(e.state(), &c, who).is_none());
}

/// The keys of the lines that say the speaker does not know or go
/// somewhere, found by their English text.
fn stranger_lines(c: &ContentBundle) -> BTreeSet<String> {
    let ftl = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/base/locale/en/talk.ftl"),
    )
    .unwrap();
    let said = |t: &str| {
        let t = t.to_lowercase();
        [
            "not my end of town",
            "never over that way",
            "only know my own street",
        ]
        .iter()
        .any(|p| t.contains(p))
    };
    let keys: BTreeSet<String> = ftl
        .lines()
        .filter_map(|l| l.split_once(" = "))
        .filter(|(_, t)| said(t))
        .map(|(k, _)| k.trim().to_string())
        .collect();
    assert!(keys.len() >= 4, "the stranger lines are found: {keys:?}");
    c.talk_lines
        .iter()
        .filter(|l| keys.contains(&l.key))
        .map(|l| l.id.clone())
        .collect()
}

/// Everyone the player could meet in their town: its politicians and named
/// characters.
fn townsfolk(e: &Engine) -> Vec<CharacterId> {
    let st = e.state();
    let me = me(e);
    (0..st.politics.characters.people.len() as u32)
        .map(CharacterId)
        .filter(|c| *c != me)
        .filter(|c| {
            characters::municipality_of(&st.politics, &st.world, *c) == Some(town(e).0)
                && characters::profile(&st.politics, &st.world, *c).is_some()
        })
        .collect()
}

#[test]
fn nobody_says_they_never_go_where_they_live_work_hold_office_or_stand() {
    let c = content();
    let stranger = stranger_lines(&c);
    let worries = c.talk_topic("topic.worries").unwrap();
    let mut checked = 0;
    let mut office_answers = 0;
    for seed in [1, 8, 23, 2026] {
        let e = game(seed, &c);
        let st = e.state();
        let pl = st.player.as_ref().unwrap();
        let hoods: Vec<NeighborhoodId> = st
            .world
            .hoods_of(town(&e))
            .map(|h| NeighborhoodId(h as u32))
            .collect();
        for who in townsfolk(&e) {
            for hood in &hoods {
                let about = AboutRef::Hood { hood: *hood };
                // Where the player is now, and standing in that neighbourhood.
                for here in [
                    pl.location,
                    Location::Street {
                        neighborhood: *hood,
                    },
                ] {
                    let ctx = Ctx {
                        st,
                        content: &c,
                        town: town(&e),
                        me: me(&e),
                        speaker: who,
                        about: Some(&about),
                        here,
                        cap: Level::Detailed,
                    };
                    let ties = ctx.ties(*hood);
                    let (cands, _) =
                        reply::candidates(&ctx, worries, TalkPart::Substance, reply::voice(&ctx));
                    let said: Vec<&str> = cands
                        .iter()
                        .map(|(l, _)| l.id.as_str())
                        .filter(|id| stranger.contains(*id))
                        .collect();
                    checked += 1;
                    if ties.any() {
                        assert!(
                            said.is_empty(),
                            "seed {seed}: {who:?} about {hood:?} ({ties:?}) could say {said:?}"
                        );
                    }
                    // Officeholders answer for their jurisdiction by report.
                    if ties.office
                        && !ties.home
                        && cands
                            .iter()
                            .any(|(l, _)| l.id.starts_with("line.worries.about_hood_office"))
                    {
                        office_answers += 1;
                    }
                }
            }
        }
    }
    assert!(
        checked > 100,
        "enough speakers and neighbourhoods: {checked}"
    );
    assert!(
        office_answers > 0,
        "officeholders answer for other neighbourhoods"
    );
}

#[test]
fn officeholders_know_their_jurisdiction_by_report() {
    let c = content();
    let mut e = game(17, &c);
    let who = colleague_here(&mut e);
    let st = e.state();
    let home = characters::profile(&st.politics, &st.world, who)
        .map(|p| NeighborhoodId(p.home))
        .unwrap();
    let other = st
        .world
        .hoods_of(town(&e))
        .map(|h| NeighborhoodId(h as u32))
        .find(|h| *h != home)
        .unwrap();
    let s = mandate_sim::player::knowledge::Subject::HoodConcerns { hood: other };
    assert_eq!(knows::speaker_knows(st, &c, who, &s), Level::Known);
    let t = mandate_sim::player::knowledge::Subject::TownConcerns { town: town(&e) };
    assert_eq!(knows::speaker_knows(st, &c, who, &t), Level::Detailed);
}

#[test]
fn the_whole_town_is_a_subject_of_its_own() {
    let c = content();
    let mut e = game(23, &c);
    let who = colleague_here(&mut e);
    let here = talk::jurisdictions_here(e.state());
    assert_eq!(
        here,
        vec![mandate_sim::world::Jurisdiction::town(town(&e))],
        "the town the conversation is in"
    );
    // Warm enough to tell plainly.
    let me = me(&e);
    e.setup(|st| {
        st.politics.characters.edges.insert(
            (who.0, me.0),
            characters::Relationship {
                opinion: 40.0,
                trust: 0.6,
                bonds: Vec::new(),
                familiar: 0,
            },
        );
    });
    let about = AboutRef::Jurisdiction { at: here[0] };
    let r = say(&mut e, who, "topic.worries", Some(about)).unwrap();
    let x = exchange(&e, &r);
    let substance = x
        .parts
        .iter()
        .find(|p| p.part == TalkPart::Substance)
        .unwrap();
    assert!(
        substance.line.starts_with("line.worries.whole"),
        "answered from the town's worries: {}",
        substance.line
    );
    // Another town's whole is not here.
    let elsewhere = e
        .state()
        .world
        .municipalities
        .iter()
        .find(|m| m.id != town(&e))
        .map(|m| mandate_sim::world::Jurisdiction::town(m.id))
        .unwrap();
    assert!(
        say(
            &mut e,
            who,
            "topic.worries",
            Some(AboutRef::Jurisdiction { at: elsewhere })
        )
        .is_err()
    );
}
