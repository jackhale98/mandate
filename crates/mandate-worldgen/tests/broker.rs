#![allow(clippy::unwrap_used, clippy::expect_used)]
//! The power broker matters (M6 note 27): they lean on votes on bills in
//! their interest and not on others, back a candidate with money and their
//! organisation, and once the player knows who they are a conversation
//! opens choices whose effects are what their previews say. Determinism
//! and save → load hold.

use mandate_content::{ContentBundle, PolicyTemplate, Source, bundle::read_dir_sources, load};
use mandate_sim::actions::PlayerCommand;
use mandate_sim::actions::broker::{self, Opening};
use mandate_sim::characters::town::{self, broker_pull, compliance};
use mandate_sim::characters::{self, CharacterId};
use mandate_sim::player::knowledge::{self, Level, Subject};
use mandate_sim::policy::{PolicyStatus, probe, yes_probability};
use mandate_sim::spine::Cause;
use mandate_sim::world::Jurisdiction;
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

fn home(e: &Engine) -> usize {
    mandate_sim::player::home(e.state()).unwrap().index()
}

/// The player's town's broker, when it is an organisation leader.
fn leader_broker(e: &Engine, c: &ContentBundle) -> Option<CharacterId> {
    let st = e.state();
    let b = st.politics.characters.towns.get(home(e))?.broker?;
    let occ = st
        .politics
        .characters
        .get(b)?
        .named
        .as_deref()?
        .occupation
        .clone();
    c.occupation(&occ).filter(|o| o.leader).map(|_| b)
}

/// Draftable local templates (no unlock, not retired), in id order.
fn local_templates(c: &ContentBundle) -> Vec<&PolicyTemplate> {
    c.policies
        .values()
        .filter(|t| t.tier == mandate_content::TierKey::Local && t.unlock.is_none() && !t.retired)
        .collect()
}

/// A template in the broker's interest and one outside it, as probes.
fn inside_and_outside<'c>(
    e: &Engine,
    c: &'c ContentBundle,
    b: CharacterId,
) -> Option<(&'c PolicyTemplate, &'c PolicyTemplate)> {
    let st = e.state();
    let tc = &st.politics.characters.towns[home(e)];
    let stake = town::stake_of(c, &st.politics, tc, b);
    let way = |t: &PolicyTemplate| {
        let inst = probe(t, home(e) as u32, &Default::default());
        town::interest(c, t, &inst, &stake)
    };
    let all = local_templates(c);
    let inside = all.iter().find(|t| way(t) > 0.0)?;
    let outside = all.iter().find(|t| way(t) == 0.0)?;
    Some((inside, outside))
}

/// The first small world whose town broker leads an organisation with a
/// draftable bill in their interest (searched in order: deterministic).
fn broker_world(c: &Arc<ContentBundle>) -> (Engine, CharacterId) {
    for seed in 1..200 {
        let e = engine(c, seed);
        if let Some(b) = leader_broker(&e, c)
            && inside_and_outside(&e, c, b).is_some()
        {
            return (e, b);
        }
    }
    panic!("no small world in 1..200 has a leader broker with a bill in their interest");
}

fn twin(e: &Engine, c: &Arc<ContentBundle>) -> Engine {
    Engine::from_state(e.state().clone(), c.clone()).unwrap()
}

fn know_broker(e: &mut Engine) {
    let m = mandate_sim::player::home(e.state()).unwrap();
    e.setup(|s| {
        knowledge::teach_one(
            s,
            Subject::Broker { town: m },
            Level::Known,
            knowledge::Source::Lab,
        );
    });
}

#[test]
fn the_broker_leans_on_bills_in_their_interest_and_not_on_others() {
    let c = base();
    let (e, b) = broker_world(&c);
    let st = e.state();
    let m = home(&e) as u32;
    let (inside, outside) = inside_and_outside(&e, &c, b).unwrap();
    let constitution = c.constitutions.get(&st.meta.constitution).unwrap();
    let leans = |t: &PolicyTemplate, pull| {
        let inst = probe(t, m, &Default::default());
        mandate_sim::policy::legislature_leans(
            &c.council,
            &c.politics.party_line,
            constitution,
            &st.world,
            &st.population,
            &st.politics,
            st.meta.seed,
            t,
            &inst,
            pull,
        )
    };
    let pull_of = |t: &PolicyTemplate| {
        broker_pull(
            &c,
            &st.politics,
            &st.world,
            &st.population,
            t,
            &probe(t, m, &Default::default()),
        )
    };
    // Outside their interest: no pull at all.
    assert_eq!(pull_of(outside), None, "{}", outside.id);
    // Inside: a named term, `vote_pull` × influence, felt by each member as
    // far as they go along with the broker.
    let pull = pull_of(inside).expect("a bill in their interest is leaned on");
    assert_eq!(pull.broker, b);
    let occ = st
        .politics
        .characters
        .get(b)
        .unwrap()
        .named
        .as_deref()
        .unwrap();
    let influence = c.occupation(&occ.occupation).unwrap().influence;
    assert!((pull.pull - c.characters.broker.vote_pull * influence).abs() < 1e-12);
    let with = leans(inside, Some(pull));
    let without = leans(inside, None);
    let mut moved = 0;
    for ((id, a), (_, z)) in with.iter().zip(&without) {
        let ch = st.politics.get(*id).unwrap().character;
        let expect = pull.pull * compliance(&st.politics, ch, b);
        if st.politics.get(*id).unwrap().is_player {
            continue;
        }
        assert!((a - z - expect).abs() < 1e-9, "{id:?}: {a} vs {z}");
        moved += (expect > 0.0) as u32;
    }
    assert!(moved > 0, "the broker moved somebody");
    let yes = |l: &[(mandate_sim::politics::PoliticianId, f64)]| -> f64 {
        l.iter().map(|(_, x)| yes_probability(&c.council, *x)).sum()
    };
    assert!(yes(&with) > yes(&without));
    // Someone who loathes the broker does not go along at all.
    let member = with
        .iter()
        .map(|(id, _)| st.politics.get(*id).unwrap())
        .find(|p| !p.is_player && p.character != b)
        .unwrap()
        .character;
    let mut s = st.clone();
    characters::change_opinion(&mut s.politics, &c, member, b, -200.0);
    assert_eq!(compliance(&s.politics, member, b), 0.0);
}

/// The broker is named among the causes of the vote they leaned on.
#[test]
fn the_broker_is_a_cause_of_the_vote_they_leaned_on() {
    let c = base();
    let (mut e, b) = broker_world(&c);
    let (inside, _) = inside_and_outside(&e, &c, b).unwrap();
    let id = e
        .apply(e.route(PlayerCommand::DraftPolicy {
            template: inside.id.clone(),
            params: Default::default(),
        }))
        .unwrap()
        .result
        .unwrap()
        .created
        .unwrap();
    e.apply(e.route(PlayerCommand::IntroducePolicy { policy: id }))
        .unwrap()
        .result
        .unwrap();
    let mut leaned_by = None;
    for _ in 0..120 {
        let st = e.state();
        let inst = &st.policy.instances[id as usize];
        if inst.status != PolicyStatus::UnderConsideration {
            break;
        }
        let t = mandate_sim::policy::resolve(&c, &st.policy, inst).unwrap();
        leaned_by = broker_pull(
            &c,
            &st.politics,
            &st.world,
            &st.population,
            t.template(),
            inst,
        )
        .map(|p| p.broker);
        e.apply(Command::AdvanceDays { days: 1 }).unwrap();
    }
    let st = e.state();
    let inst = &st.policy.instances[id as usize];
    assert!(matches!(
        inst.status,
        PolicyStatus::Enacted | PolicyStatus::Ended
    ));
    let who = leaned_by.expect("the broker leaned on it the day it was decided");
    let ev = st
        .events
        .iter()
        .find(|ev| match ev.kind {
            mandate_sim::events::EventKind::PolicyEnacted { policy, .. }
            | mandate_sim::events::EventKind::PolicyFailed { policy, .. } => policy == id,
            _ => false,
        })
        .expect("the outcome is in the log");
    assert!(ev.causes.contains(&Cause::Actor(who)), "{:?}", ev.causes);
    let _ = b;
}

/// The broker's backing reaches the race: money and the organisation's
/// door-knocking for the candidate they favour, every campaign week.
#[test]
fn backing_changes_a_races_funds_and_contacts() {
    let c = base();
    let mut quiet = (*c).clone();
    quiet.characters.broker.back_money = 0.0;
    quiet.characters.broker.back_contacts = 0;
    let quiet = Arc::new(quiet);
    for seed in 1..60 {
        let mut a = engine(&c, seed);
        // Into the first campaign.
        let mut days = 0;
        while a.state().politics.elections.races.is_empty() && days < 1500 {
            a.apply(Command::AdvanceDays { days: 7 }).unwrap();
            days += 7;
        }
        let st = a.state();
        let Some((ri, (b, ci))) =
            st.politics
                .elections
                .races
                .iter()
                .enumerate()
                .find_map(|(i, r)| {
                    mandate_sim::politics::elections::backed(
                        &st.politics,
                        &st.world,
                        &st.population,
                        &c,
                        r,
                    )
                    .map(|x| (i, x))
                })
        else {
            continue;
        };
        let race = &st.politics.elections.races[ri];
        let who = race.candidates[ci].politician;
        if st.politics.get(who).unwrap().is_player {
            continue;
        }
        // The same world with the backing switched off.
        let mut z = Engine::from_state(a.state().clone(), quiet.clone()).unwrap();
        a.apply(Command::AdvanceDays { days: 7 }).unwrap();
        z.apply(Command::AdvanceDays { days: 7 }).unwrap();
        let worth = |e: &Engine| {
            let st = e.state();
            let r = &st.politics.elections.races[ri];
            let cand = r.candidates.iter().find(|x| x.politician == who).unwrap();
            let contacts: f32 = cand.contacts.iter().sum();
            (st.politics.get(who).unwrap().funds + cand.spent, contacts)
        };
        let (fa, ca) = worth(&a);
        let (fz, cz) = worth(&z);
        assert!(fa > fz, "seed {seed}: funds {fa} vs {fz} (broker {b:?})");
        assert!(ca > cz, "seed {seed}: contacts {ca} vs {cz}");
        return;
    }
    panic!("no small world in 1..60 has a broker backing an NPC candidate");
}

fn offered_ids(e: &Engine, c: &ContentBundle, speaker: CharacterId) -> Vec<String> {
    mandate_sim::talk::hooks::offered(e.state(), c, speaker)
        .into_iter()
        .map(|ch| ch.id)
        .filter(|id| id.starts_with("choice.broker."))
        .collect()
}

/// A bill before the body in the broker's interest, the player's.
fn pending_bill(e: &mut Engine, c: &ContentBundle, b: CharacterId) -> u32 {
    let (inside, _) = inside_and_outside(e, c, b).unwrap();
    let id = e
        .apply(e.route(PlayerCommand::DraftPolicy {
            template: inside.id.clone(),
            params: Default::default(),
        }))
        .unwrap()
        .result
        .unwrap()
        .created
        .unwrap();
    e.apply(e.route(PlayerCommand::IntroducePolicy { policy: id }))
        .unwrap()
        .result
        .unwrap();
    id
}

/// Mean approval of the player by income class across the town.
fn approval_by_class(e: &Engine) -> [f64; 3] {
    let st = e.state();
    let m = mandate_sim::player::home(st).unwrap();
    let mut acc = [0.0f64; 3];
    let mut n = [0.0f64; 3];
    for h in st.world.hoods_of(m) {
        for (ci, co) in st.population.hoods[h].cohorts.iter().enumerate() {
            let k = mandate_sim::population::class_of(ci);
            acc[k] += co.player_approval as f64;
            n[k] += 1.0;
        }
    }
    std::array::from_fn(|i| acc[i] / n[i].max(1.0))
}

#[test]
fn the_openings_appear_only_once_the_broker_is_known() {
    let c = base();
    let (mut e, b) = broker_world(&c);
    let _ = pending_bill(&mut e, &c, b);
    assert!(
        offered_ids(&e, &c, b).is_empty(),
        "nothing is offered before the player knows who the broker is"
    );
    // Heard of is not enough.
    let m = mandate_sim::player::home(e.state()).unwrap();
    e.setup(|s| {
        knowledge::teach_one(
            s,
            Subject::Broker { town: m },
            Level::Heard,
            knowledge::Source::Lab,
        );
    });
    assert!(offered_ids(&e, &c, b).is_empty());
    know_broker(&mut e);
    let ids = offered_ids(&e, &c, b);
    assert!(
        ids.iter().any(|i| i.starts_with("choice.broker.lobby.")),
        "{ids:?}"
    );
    assert!(ids.contains(&Opening::Favour.id()), "{ids:?}");
    assert!(ids.contains(&Opening::Oppose.id()), "{ids:?}");
    // The press offers an exposé; nobody else does.
    let st = e.state();
    let press = st.politics.characters.people.iter().find(|p| {
        p.named.as_deref().is_some_and(|n| {
            c.characters
                .broker
                .press_occupations
                .contains(&n.occupation)
                && characters::municipality_of(&st.politics, &st.world, p.id) == Some(m.0)
        })
    });
    if let Some(p) = press {
        assert_eq!(offered_ids(&e, &c, p.id), vec![Opening::Expose.id()]);
    }
}

#[test]
fn each_opening_does_what_its_preview_says() {
    let c = base();
    let (mut e, b) = broker_world(&c);
    let bill = pending_bill(&mut e, &c, b);
    know_broker(&mut e);
    let at = Jurisdiction::town(mandate_sim::player::home(e.state()).unwrap());
    let me = characters::player_character(e.state()).unwrap();
    let (depend, resent) = town::depend_and_resent(&c, &e.state().politics, b);
    // Lobbying the broker: the sway shown lands on each member, and the
    // player owes the broker a favour.
    {
        let mut x = e.state().clone();
        let o = Opening::Lobby { policy: bill };
        let p = broker::preview(&x, &c, o, b, at).expect("lobbying is open");
        assert!(p.owe && p.votes > 0.0 && !p.sway.is_empty());
        let before: Vec<f32> = p
            .sway
            .iter()
            .map(|(id, _)| x.politics.sway(*id, bill))
            .collect();
        let owed = characters::favours_owed(&x.politics, me, b);
        broker::apply(&mut x, &c, o, &p, false);
        for ((id, amount), was) in p.sway.iter().zip(before) {
            let now = x.politics.sway(*id, bill);
            assert!((now - was - *amount as f32).abs() < 1e-6, "{id:?}");
        }
        assert_eq!(characters::favours_owed(&x.politics, me, b), owed + 1);
    }
    // A favour: the broker owes the player and thinks better of them; those
    // who resent the broker think a little less of the player.
    let mut favoured = twin(&e, &c);
    {
        let before = approval_by_class(&favoured);
        let op = characters::opinion(&favoured.state().politics, b, me);
        let p = broker::preview(favoured.state(), &c, Opening::Favour, b, at).unwrap();
        assert!(p.broker_owes);
        favoured.setup(|s| broker::apply(s, &c, Opening::Favour, &p, false));
        let st = favoured.state();
        assert_eq!(characters::favours_owed(&st.politics, b, me), 1);
        assert!(characters::opinion(&st.politics, b, me) > op);
        let after = approval_by_class(&favoured);
        for k in &resent {
            assert!(after[*k as usize] < before[*k as usize], "class {k}");
        }
        // Lobbying them now calls the favour in instead.
        let p =
            broker::preview(favoured.state(), &c, Opening::Lobby { policy: bill }, b, at).unwrap();
        assert!(!p.owe);
    }
    // Opposing them openly: popular with those who resent them, costly with
    // those who depend on them; the broker cools; not offered again soon.
    {
        let mut x = twin(&e, &c);
        let before = approval_by_class(&x);
        let op = characters::opinion(&x.state().politics, b, me);
        let p = broker::preview(x.state(), &c, Opening::Oppose, b, at).unwrap();
        x.setup(|s| broker::apply(s, &c, Opening::Oppose, &p, false));
        let after = approval_by_class(&x);
        for k in &resent {
            assert!(after[*k as usize] > before[*k as usize], "resent {k}");
        }
        for k in &depend {
            assert!(after[*k as usize] < before[*k as usize], "depend {k}");
        }
        assert!(characters::opinion(&x.state().politics, b, me) < op);
        assert!(broker::preview(x.state(), &c, Opening::Oppose, b, at).is_none());
        assert!(broker::preview(x.state(), &c, Opening::Expose, b, at).is_none());
    }
    // Exposing them: the risk shown is the press's gaffe risk; a story that
    // stands up turns the members against the broker (who then moves them
    // less); one that falls apart costs the player approval everywhere.
    {
        let p = broker::preview(e.state(), &c, Opening::Expose, b, at).unwrap();
        assert_eq!(
            p.risk,
            Some(mandate_sim::actions::press_gaffe_risk(e.state(), &c))
        );
        let mut ok = twin(&e, &c);
        let member = p
            .opinions
            .iter()
            .find(|(h, t, _)| *t == b && *h != me)
            .map(|x| x.0)
            .expect("members distance themselves");
        let was = compliance(&ok.state().politics, member, b);
        ok.setup(|s| broker::apply(s, &c, Opening::Expose, &p, false));
        assert!(compliance(&ok.state().politics, member, b) < was);
        let mut bad = twin(&e, &c);
        let before = approval_by_class(&bad);
        bad.setup(|s| broker::apply(s, &c, Opening::Expose, &p, true));
        let after = approval_by_class(&bad);
        for k in 0..3 {
            assert!(after[k] <= before[k], "class {k}");
        }
        assert_eq!(compliance(&bad.state().politics, member, b), was);
    }
}

/// The whole path, as the player takes it: talk to the broker and pick
/// the choice; the outcome is the opening's, and it is deterministic and
/// survives save → load.
#[test]
fn an_opening_through_a_conversation_is_deterministic_through_saves() {
    let c = base();
    let play = |days_after: u32| {
        let (mut e, b) = broker_world(&c);
        know_broker(&mut e);
        // Go where they are now, then talk.
        let talk = PlayerCommand::Talk {
            to: mandate_sim::talk::TalkTarget::Person { person: b },
            topic: Opening::Favour.id(),
            about: None,
            late_night: false,
        };
        let cmd = match mandate_sim::characters::routines::whereabouts(e.state(), &c, b) {
            Some(location) => PlayerCommand::Go {
                location,
                then: Some(Box::new(talk)),
            },
            None => talk,
        };
        let out = e.apply(Command::Player(cmd)).ok().and_then(|o| o.result);
        (e, b, out, days_after)
    };
    let (mut a, b, out, _) = play(0);
    let me = characters::player_character(a.state()).unwrap();
    match out {
        Some(r) => {
            assert_eq!(r.outcome_key, "outcome-broker-favour");
            assert_eq!(characters::favours_owed(&a.state().politics, b, me), 1);
        }
        None => panic!("the favour could not be done"),
    }
    let (mut z, _, _, _) = play(0);
    for _ in 0..8 {
        a.apply(Command::EndTurn).unwrap();
        z.apply(Command::EndTurn).unwrap();
    }
    assert_eq!(a.state_hash(), z.state_hash());
    let json = serde_json::to_string(a.state()).unwrap();
    let state: mandate_sim::State = serde_json::from_str(&json).unwrap();
    let mut h = Engine::from_state(state, c.clone()).unwrap();
    for _ in 0..8 {
        a.apply(Command::EndTurn).unwrap();
        h.apply(Command::EndTurn).unwrap();
    }
    assert_eq!(a.state_hash(), h.state_hash());
}
