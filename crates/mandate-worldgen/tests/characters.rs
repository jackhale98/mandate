#![allow(clippy::unwrap_used, clippy::expect_used)]
//! DEC-031 characters, DEC-018 tiers, DEC-077 §1 town character: every
//! politician is a character; Named people are seeded at places and updated
//! lazily (order-independent); memory is bounded, betrayals persist and
//! trivia fades; towns get a defining industry, crises that come and go, a
//! power broker, and a rival. An expansion adds a trait, an occupation, and
//! a crisis kind purely in content.

use mandate_content::{ContentBundle, Source, bundle::read_dir_sources, load};
use mandate_sim::Command;
use mandate_sim::actions::PlayerCommand;
use mandate_sim::characters::{self, CharacterId, MemoryKind, Subject, Tier};
use mandate_sim::places::Location;
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::path::Path;
use std::sync::Arc;

fn base_sources() -> Vec<Source> {
    read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap()
}

fn base() -> Arc<ContentBundle> {
    Arc::new(load(&base_sources()).unwrap().0)
}

fn engine(c: &Arc<ContentBundle>, seed: u64) -> mandate_sim::Engine {
    generate(&WorldConfig::new(seed, WorldSize::Small), c.clone()).unwrap()
}

fn player_char(e: &mandate_sim::Engine) -> CharacterId {
    characters::player_character(e.state()).unwrap()
}

#[test]
fn every_politician_is_a_character_and_towns_have_people() {
    let c = base();
    let e = engine(&c, 5);
    let st = e.state();
    let pol = &st.politics;
    for p in &pol.politicians {
        let ch = pol.characters.get(p.character).expect("a character");
        assert_eq!(ch.politician, Some(p.id));
        assert!(ch.named.is_none());
        assert!(ch.traits.len() <= 3);
        for v in [
            ch.warmth,
            ch.boldness,
            ch.loyalty,
            ch.administration,
            ch.organising,
            ch.intrigue,
        ] {
            assert!((0.0..=1.0).contains(&v));
        }
        let prof = characters::profile(pol, &st.world, p.character).unwrap();
        assert_eq!(prof.personality.integrity, p.integrity);
        assert_eq!(prof.values, p.attitudes);
    }
    // Ids are dense and unique.
    for (i, ch) in pol.characters.people.iter().enumerate() {
        assert_eq!(ch.id.index(), i);
    }
    assert!(pol.characters.named_seeded);
    let named: Vec<_> = pol
        .characters
        .people
        .iter()
        .filter(|c| c.named.is_some())
        .collect();
    assert!(
        named.len() > st.world.municipalities.len() * 4,
        "{} named",
        named.len()
    );
    // Every town has its union steward or business chair where it has the
    // place, and the leaders are Principal characters.
    for ch in &named {
        let n = ch.named.as_deref().unwrap();
        let occ = c.occupation(&n.occupation).unwrap();
        if let Some(p) = n.place {
            let kind = st.world.places[p.index()].kind.key();
            assert!(occ.places.iter().any(|k| k == kind), "{} at {kind}", occ.id);
        }
        assert_eq!(n.values.len(), c.attitudes.len());
        let tier = characters::tier_of(pol, &c, ch.id);
        if occ.leader {
            assert_eq!(tier, Tier::Principal);
        }
    }
    // The player's colleagues are Principals.
    let pl = st.player.as_ref().unwrap();
    for id in &council_members_of(pol, pl.municipality.index()) {
        let ch = pol.get(*id).unwrap().character;
        assert_eq!(characters::tier_of(pol, &c, ch), Tier::Principal);
    }
}

#[test]
fn opinion_and_favours_live_on_relationship_edges() {
    let c = base();
    let mut e = engine(&c, 37);
    let me = player_char(&e);
    let st = e.state();
    let pl = st.player.as_ref().unwrap();
    let colleague = *council_members(st, pl.municipality.index())
        .iter()
        .find(|id| **id != pl.politician)
        .unwrap();
    let cc = st.politics.get(colleague).unwrap().character;
    // No history: their first impression, no edge.
    assert!(st.politics.characters.edge(cc, me).is_none());
    let first = st.politics.characters.get(cc).unwrap().first_impression;
    assert_eq!(characters::opinion(&st.politics, cc, me), first);
    // A favour traded: the ledger records it, and they remember the promise.
    let bill = st
        .policy
        .instances
        .iter()
        .find(|p| {
            p.jurisdiction == pl.municipality.0
                && p.status == mandate_sim::policy::PolicyStatus::UnderConsideration
        })
        .map(|p| p.id.0);
    let Some(bill) = bill else { return };
    let r = e.apply(e.route(PlayerCommand::Act {
        action: mandate_sim::actions::Action::TradeFavor {
            politician: colleague.0,
            policy: bill,
        },
        late_night: false,
    }));
    assert!(r.is_ok(), "{r:?}");
    let pol = &e.state().politics;
    assert_eq!(characters::favours_owed(pol, me, cc), 1);
    assert_eq!(characters::favour_balance(pol, cc, me), -1);
    let f = &pol.characters.favours[0];
    assert_eq!(f.context, "favour-bill");
    assert_eq!(f.subject, Some(Subject::Policy { id: bill }));
    let edge = pol.characters.edge(cc, me).unwrap();
    // The colleague's warmth and traits scale the goodwill (DEC-031).
    let d = c.actions.lobby.opinion_success;
    let scaled = d * characters::opinion_scale(pol, &c, cc, d);
    let expected = (first as f64 + scaled).clamp(-100.0, 100.0) as f32;
    assert_eq!(edge.opinion, expected);
    let mem = characters::salient_memories(pol, cc, Some(me), e.state().calendar.date, 3);
    assert_eq!(mem[0].kind, MemoryKind::PromisedFavour);
}

#[test]
fn betrayal_is_remembered_years_later_while_trivia_fades() {
    let c = base();
    let mut e = engine(&c, 41);
    let me = player_char(&e);
    let st = e.state();
    let pl = st.player.as_ref().unwrap();
    let colleague = *council_members(st, pl.municipality.index())
        .iter()
        .find(|id| **id != pl.politician)
        .unwrap();
    let cc = st.politics.get(colleague).unwrap().character;
    let content = c.clone();
    e.setup(|st| {
        let d = st.calendar.date;
        let pol = &mut st.politics;
        characters::remember(
            pol,
            &content,
            d,
            cc,
            me,
            MemoryKind::LobbiedMe,
            None,
            Some(0.0),
        );
        characters::remember(
            pol,
            &content,
            d,
            cc,
            me,
            MemoryKind::BrokePromise,
            None,
            Some(-10.0),
        );
        characters::remember(pol, &content, d, cc, me, MemoryKind::Met, None, None);
    });
    let before = characters::opinion(&e.state().politics, cc, me);
    e.apply(Command::AdvanceDays { days: 6 * 365 }).unwrap();
    let pol = &e.state().politics;
    let kinds: Vec<MemoryKind> =
        characters::salient_memories(pol, cc, Some(me), e.state().calendar.date, 20)
            .into_iter()
            .map(|m| m.kind)
            .collect();
    assert!(kinds.contains(&MemoryKind::BrokePromise), "{kinds:?}");
    assert!(
        !kinds.contains(&MemoryKind::LobbiedMe),
        "trivia fades: {kinds:?}"
    );
    assert!(!kinds.contains(&MemoryKind::Met), "trivia fades: {kinds:?}");
    // Faded memories leave the opinion where they put it (the baseline).
    assert_eq!(characters::opinion(pol, cc, me), before);
}

#[test]
fn a_grudge_holder_forgives_slowly() {
    let c = base();
    let mut e = engine(&c, 43);
    let me = player_char(&e);
    // Two colleagues, one made a grudge-holder.
    let (a, b) = {
        let st = e.state();
        let pl = st.player.as_ref().unwrap();
        let ids: Vec<CharacterId> = council_members(st, pl.municipality.index())
            .iter()
            .filter(|id| **id != pl.politician)
            .map(|id| st.politics.get(*id).unwrap().character)
            .collect();
        (ids[0], ids[1])
    };
    let content = c.clone();
    e.setup(|st| {
        let d = st.calendar.date;
        let pol = &mut st.politics;
        pol.characters.get_mut(a).unwrap().traits = vec!["trait.grudge_holder".into()];
        pol.characters.get_mut(b).unwrap().traits = vec![];
        for h in [a, b] {
            characters::remember(
                pol,
                &content,
                d,
                h,
                me,
                MemoryKind::OpposedMyBill,
                None,
                Some(0.0),
            );
        }
    });
    let d = e.state().calendar.date.add_days(2 * 365);
    let pol = &e.state().politics;
    let s = |h| {
        pol.characters
            .get(h)
            .unwrap()
            .memories
            .iter()
            .find(|m| m.kind == MemoryKind::OpposedMyBill && m.actor == me)
            .unwrap()
            .salience(d)
    };
    assert!(s(a) > s(b) * 1.5, "grudge {} vs {}", s(a), s(b));
}

#[test]
fn memory_stays_bounded_over_a_long_run() {
    let c = base();
    let mut e = engine(&c, 9);
    for _ in 0..12 {
        e.apply(Command::AdvanceDays { days: 365 }).unwrap();
        // NPC turns (lobbying memories) as in play, every quarter of a year.
        for _ in 0..4 {
            for _ in 0..6 {
                e.apply(Command::EndTurn).unwrap();
            }
        }
        let st = e.state();
        for ch in &st.politics.characters.people {
            let cap = match characters::tier_of(&st.politics, &c, ch.id) {
                Tier::Principal => c.characters.memory.principal,
                Tier::Named => c.characters.memory.named,
            } as usize;
            // Tiers can drop (a councillor losing their seat) until the next
            // monthly upkeep trims them; never beyond the Principal cap.
            assert!(ch.memories.len() <= cap.max(c.characters.memory.principal as usize));
        }
    }
    let total: usize = e
        .state()
        .politics
        .characters
        .people
        .iter()
        .map(|c| c.memories.len())
        .sum();
    assert!(total > 0, "memories are recorded in play");
}

#[test]
fn elections_leave_rivalries() {
    let c = base();
    let mut e = engine(&c, 21);
    e.apply(Command::AdvanceDays { days: 5 * 365 }).unwrap();
    let pol = &e.state().politics;
    let defeats = pol
        .characters
        .people
        .iter()
        .flat_map(|c| c.memories.iter())
        .filter(|m| m.kind == MemoryKind::DefeatedMe)
        .count();
    assert!(defeats > 0);
    let rivals = pol
        .characters
        .edges
        .values()
        .filter(|r| r.bonds.contains(&characters::Bond::Rival))
        .count();
    assert!(rivals > 0);
}

#[test]
fn lazy_named_updates_are_order_independent() {
    let c = base();
    let mut e = engine(&c, 13);
    e.apply(Command::AdvanceDays { days: 4 * 365 }).unwrap();
    let st = e.state().clone();
    let named: Vec<CharacterId> = st
        .politics
        .characters
        .people
        .iter()
        .filter(|c| c.named.is_some())
        .map(|c| c.id)
        .take(2)
        .collect();
    let (x, y) = (named[0], named[1]);
    let seed = st.meta.seed;
    let now = st.calendar.date;
    let mut a = st.clone();
    characters::touch(&mut a.politics, &c, seed, x, now);
    characters::touch(&mut a.politics, &c, seed, y, now);
    let mut b = st.clone();
    characters::touch(&mut b.politics, &c, seed, y, now);
    characters::touch(&mut b.politics, &c, seed, x, now);
    assert_eq!(a.hash(), b.hash());
    // In steps or at once: the same person.
    let mid = now.add_days(2 * 365);
    let end = now.add_days(5 * 365);
    let mut steps = st.clone();
    characters::touch(&mut steps.politics, &c, seed, x, mid);
    characters::touch(&mut steps.politics, &c, seed, x, end);
    let mut once = st.clone();
    characters::touch(&mut once.politics, &c, seed, x, end);
    assert_eq!(
        steps.politics.characters.get(x),
        once.politics.characters.get(x)
    );
    assert_ne!(
        once.politics
            .characters
            .get(x)
            .unwrap()
            .named
            .as_deref()
            .unwrap()
            .values,
        st.politics
            .characters
            .get(x)
            .unwrap()
            .named
            .as_deref()
            .unwrap()
            .values,
        "values drift over the years"
    );
    // Evaluating (a view) without storing changes nothing.
    let year = end.calendar().year;
    let view = characters::caught_up(st.politics.characters.get(x).unwrap(), seed, &c, year);
    assert_eq!(Some(&view), once.politics.characters.get(x));
    assert_eq!(e.state().hash(), st.hash());
}

#[test]
fn meeting_a_passer_by_makes_them_named() {
    let c = base();
    let mut e = engine(&c, 17);
    let st = e.state();
    let pl = st.player.as_ref().unwrap();
    let market = st.world.municipalities[pl.municipality.index()]
        .places
        .iter()
        .copied()
        .find(|p| st.world.places[p.index()].kind == mandate_sim::world::PlaceKind::Market)
        .unwrap();
    let loc = Location::Place { place: market };
    let ambient = characters::ambient::ambient_at(st, &c, loc);
    assert!(!ambient.is_empty());
    // Pure: the same people twice.
    assert_eq!(ambient, characters::ambient::ambient_at(st, &c, loc));
    let (slot, who) = ambient[0].clone();
    let before = st.politics.characters.people.len();
    let r = e
        .apply(Command::Player(PlayerCommand::Go {
            location: loc,
            then: Some(Box::new(PlayerCommand::Meet { slot })),
        }))
        .unwrap();
    let id = CharacterId(r.result.unwrap().created.unwrap());
    let st = e.state();
    assert_eq!(st.politics.characters.people.len(), before + 1);
    let met = st.politics.characters.get(id).unwrap();
    assert_eq!(
        met.named.as_deref().unwrap().given,
        who.named.as_deref().unwrap().given
    );
    assert_eq!(met.named.as_deref().unwrap().place, Some(market));
    assert_eq!(
        characters::salient_memories(&st.politics, id, None, st.calendar.date, 1)[0].kind,
        MemoryKind::Met
    );
    // They are no longer a passer-by, but a regular here.
    assert!(
        characters::ambient::ambient_at(st, &c, loc)
            .iter()
            .all(|(s, _)| *s != slot)
    );
    assert!(characters::ambient::stored_at(&st.politics, st, loc).contains(&id));
    // Nobody in a slot that is not there.
    assert!(
        e.apply(Command::Player(PlayerCommand::Meet { slot: 99 }))
            .is_err()
    );
}

#[test]
fn every_town_has_a_character() {
    let c = base();
    for seed in [3, 4] {
        let e = engine(&c, seed);
        let st = e.state();
        let towns = &st.politics.characters.towns;
        assert_eq!(towns.len(), st.world.municipalities.len());
        for t in towns {
            let k = t.industry.as_deref().expect("a defining industry");
            assert!(
                c.sector(k)
                    .is_some_and(|s| s.driver != mandate_content::SectorDriver::Public)
            );
            assert!(t.industry_share > 0.0 && t.industry_share <= 1.0);
            assert!(t.broker.is_some());
            assert!(t.priorities.len() <= 3);
        }
        let pl = st.player.as_ref().unwrap();
        let mine = &towns[pl.municipality.index()];
        let rival = mine.rival.expect("the player has a rival");
        assert_ne!(Some(rival), characters::player_character(st));
    }
}

#[test]
fn the_rival_follows_the_players_plans() {
    let c = base();
    let mut e = engine(&c, 7);
    let m = e.state().player.as_ref().unwrap().municipality.index();
    let before = e.state().politics.characters.towns[m].rival;
    e.apply(e.route(PlayerCommand::File {
        filing: mandate_sim::politics::elections::executive_filing(e.state(), e.content()).unwrap(),
    }))
    .unwrap();
    let st = e.state();
    let after = st.politics.characters.towns[m].rival;
    let mayor = mayor_of(st, m)
        .and_then(|id| st.politics.get(id))
        .map(|p| p.character);
    assert_eq!(after, mayor, "running for mayor makes the mayor the rival");
    let _ = before;
}

/// A forced plant closure (a lasting demand shock on the town's industry) puts
/// the town in a crisis that clears once jobs recover.
#[test]
fn a_plant_closure_crisis_comes_and_goes() {
    let c = base();
    let mut e = engine(&c, 11);
    let m = e.state().player.as_ref().unwrap().municipality.index();
    let industry = e.state().politics.characters.towns[m]
        .industry
        .clone()
        .unwrap();
    let crisis_of = |e: &mandate_sim::Engine, k: &str| {
        e.state().politics.characters.towns[m]
            .crises
            .iter()
            .find(|x| x.kind == k)
            .cloned()
    };
    assert!(crisis_of(&e, "crisis.plant_closure").is_none());
    e.apply(Command::LabSectorShock {
        municipality: m as u32,
        sector: industry,
        factor: 0.25,
        months: 10,
    })
    .unwrap();
    e.apply(Command::AdvanceDays { days: 240 }).unwrap();
    let live = crisis_of(&e, "crisis.plant_closure").expect("a plant-closure crisis");
    assert!(live.severity > 0.0);
    // The shock also counts as a disaster in the economy (months since).
    assert!(crisis_of(&e, "crisis.disaster_recovery").is_some());
    let t = &e.state().politics.characters.towns[m];
    assert!(t.crisis().is_some());
    // Severity history is kept for the trend.
    assert!(
        !live.history.is_empty() && live.history.len() <= c.characters.town.trend_months as usize
    );
    e.apply(Command::AdvanceDays { days: 3 * 365 }).unwrap();
    assert!(
        crisis_of(&e, "crisis.plant_closure").is_none(),
        "jobs recover, the crisis ends"
    );
    assert!(crisis_of(&e, "crisis.disaster_recovery").is_none());
}

/// An expansion adds a trait, an occupation, and a crisis kind in content.
fn with_expansion() -> Arc<ContentBundle> {
    let mut src = base_sources();
    src.push(Source {
        // Sorts before `base/`: first in the trait registry.
        path: "aa_harbour/traits.jsonc".into(),
        text: r#"{ "kind": "traits", "items": [
            { "id": "trait.sentimental", "name_key": "trait-sentimental", "chance": 1.0,
              "memory_decay": { "positive": 0.2 } } ] }"#
            .into(),
    });
    src.push(Source {
        path: "zz_harbour/occupations.jsonc".into(),
        text: r#"{ "kind": "occupations", "items": [
            { "id": "occupation.harbour_master", "name_key": "occupation-harbour_master",
              "places": ["market"], "chance": 1.0, "leader": true, "influence": 1.0,
              "stake": { "crises": ["crisis.cost_of_living"] },
              "class": [0.0, 1.0, 0.0], "age": [40, 60] } ] }"#
            .into(),
    });
    src.push(Source {
        path: "zz_harbour/crises.jsonc".into(),
        text: r#"{ "kind": "crises", "items": [
            { "id": "crisis.cost_of_living", "name_key": "crisis-cost_of_living",
              "indicator": "rent_burden", "begin": 0.0, "end": 0.0, "severe": 1.0 } ] }"#
            .into(),
    });
    src.push(Source {
        path: "zz_harbour/locale/en/harbour.ftl".into(),
        text: "trait-sentimental = Sentimental\noccupation-harbour_master = Harbour master\ncrisis-cost_of_living = Cost of living\n".into(),
    });
    Arc::new(load(&src).unwrap().0)
}

#[test]
fn an_expansion_adds_a_trait_an_occupation_and_a_crisis() {
    let c = with_expansion();
    assert!(c.trait_def("trait.sentimental").is_some());
    let a = engine(&c, 23);
    let b = engine(&c, 23);
    assert_eq!(a.state_hash(), b.state_hash());
    let st = a.state();
    // Everyone has the trait (chance 1), respecting the three-trait cap.
    assert!(
        st.politics
            .characters
            .people
            .iter()
            .all(|p| p.traits.iter().any(|t| t == "trait.sentimental"))
    );
    // Harbour masters at every market, as Principal leaders.
    let hm: Vec<_> = st
        .politics
        .characters
        .people
        .iter()
        .filter(|p| {
            p.named
                .as_deref()
                .is_some_and(|n| n.occupation == "occupation.harbour_master")
        })
        .collect();
    assert!(!hm.is_empty());
    // The always-on crisis is live everywhere, and harbour masters (with a
    // stake in it) broker the towns.
    for t in &st.politics.characters.towns {
        assert!(t.crises.iter().any(|x| x.kind == "crisis.cost_of_living"));
    }
    let brokers = st
        .politics
        .characters
        .towns
        .iter()
        .filter(|t| t.broker.is_some_and(|b| hm.iter().any(|h| h.id == b)))
        .count();
    assert!(brokers > 0);
    // Bad validation.
    let mut src = base_sources();
    src.push(Source {
        path: "zz_bad/occupations.jsonc".into(),
        text: r#"{ "kind": "occupations", "items": [
            { "id": "occupation.ghost", "name_key": "occupation-ghost",
              "places": ["haunted_house"], "chance": 2.0,
              "class": [0.0, 0.0, 0.0], "age": [10, 5], "expertise": "astrology" } ] }"#
            .into(),
    });
    let err = load(&src).unwrap_err();
    let text: Vec<String> = err.iter().map(|i| i.to_string()).collect();
    for needle in [
        "unknown place kind",
        "chance",
        "class weights",
        "age",
        "expertise",
        "occupation-ghost",
    ] {
        assert!(
            text.iter().any(|t| t.contains(needle)),
            "{needle}: {text:?}"
        );
    }
}

/// Personality reaches the NPC AI with the same rules for everyone (DEC-030):
/// integrity resists lobbying, loyalty weighs the party line, and agendas
/// follow the town's live crisis (DEC-077 §1).
#[test]
fn personality_and_town_character_reach_the_npc_ai() {
    let c = base();
    let e = engine(&c, 19);
    let st = e.state();
    let pl = st.player.as_ref().unwrap();
    let m = pl.municipality.index();
    let target = *council_members(st, m)
        .iter()
        .find(|id| **id != pl.politician)
        .unwrap();
    // Integrity: the same pitch works less on the principled.
    let odds = |integrity: f32| {
        let mut s = st.clone();
        s.politics.get_mut(target).unwrap().integrity = integrity;
        mandate_sim::npc::lobby_odds_between(&s, &c, pl.politician, target, false)
    };
    assert!(odds(0.9) < odds(0.2), "{} vs {}", odds(0.9), odds(0.2));
    // Loyalty: party discipline counts for more with the loyal.
    let p = st.politics.get(target).unwrap();
    let t = c.policies.values().next().unwrap();
    let inst = st.policy.instances.first().unwrap();
    let lean = |loyalty: f64| {
        mandate_sim::policy::vote_lean(
            &c.council,
            p,
            &mandate_sim::policy::VoteInputs {
                template: t,
                instance: inst,
                ward_support: 0.0,
                signer_lean: 0.0,
                sway: 0.0,
                sponsor_is_player: false,
                player_opinion: 0.0,
                loyalty,
                sponsor_party: p.party,
                declared: None,
                noise: 0.0,
            },
        )
    };
    if p.party.is_some() {
        assert!(lean(0.9) > lean(0.1));
    }
    // A live crisis favours the templates that answer it.
    let grants = c.policies.get("policy.small_business_grants").unwrap();
    let calm = mandate_sim::npc::agenda_bonus(st, &c, m as u32, grants);
    let mut s = st.clone();
    s.politics.characters.towns[m].crises = vec![characters::ActiveCrisis {
        kind: "crisis.plant_closure".into(),
        severity: 1.0,
        since: s.calendar.date,
        history: vec![1.0],
    }];
    let hit = mandate_sim::npc::agenda_bonus(&s, &c, m as u32, grants);
    assert!(
        (hit - calm - c.npc.crisis_agenda).abs() < 1e-9,
        "{calm} → {hit}"
    );
    let parks = c.policies.get("policy.parks").unwrap();
    assert_eq!(
        mandate_sim::npc::agenda_bonus(&s, &c, m as u32, parks),
        mandate_sim::npc::agenda_bonus(st, &c, m as u32, parks)
    );
}

/// Warmth and traits scale opinion changes (DEC-031).
#[test]
fn warmth_and_traits_scale_opinion_changes() {
    let c = base();
    let e = engine(&c, 29);
    let st = e.state();
    let me = player_char(&e);
    let pl = st.player.as_ref().unwrap();
    let who = council_members(st, pl.municipality.index())
        .iter()
        .find(|id| **id != pl.politician)
        .map(|id| st.politics.get(*id).unwrap().character)
        .unwrap();
    let gain = |warmth: f32, traits: Vec<String>| {
        let mut s = st.clone();
        let ch = s.politics.characters.get_mut(who).unwrap();
        ch.warmth = warmth;
        ch.traits = traits;
        characters::change_opinion(&mut s.politics, &c, who, me, 5.0)
    };
    assert!(gain(0.9, vec![]) > gain(0.1, vec![]));
    assert!(gain(0.5, vec!["trait.grudge_holder".into()]) < gain(0.5, vec![]));
}

/// The members of a town's bodies (the base constitution's council; DEC-098).
fn council_members(st: &mandate_sim::State, m: usize) -> Vec<mandate_sim::politics::PoliticianId> {
    let at = mandate_sim::world::Jurisdiction::town(mandate_sim::world::MunicipalityId(m as u32));
    mandate_sim::politics::body_members_at(&st.politics, at)
}

/// The holder of a town's office (the base constitution's mayor; DEC-098).
fn mayor_of(st: &mandate_sim::State, m: usize) -> Option<mandate_sim::politics::PoliticianId> {
    let at = mandate_sim::world::Jurisdiction::town(mandate_sim::world::MunicipalityId(m as u32));
    mandate_sim::politics::office_holders_at(&st.politics, at)
        .first()
        .copied()
}

/// The members of a town's bodies, from the politics state.
fn council_members_of(
    pol: &mandate_sim::politics::PoliticsState,
    m: usize,
) -> Vec<mandate_sim::politics::PoliticianId> {
    let at = mandate_sim::world::Jurisdiction::town(mandate_sim::world::MunicipalityId(m as u32));
    mandate_sim::politics::body_members_at(pol, at)
}
