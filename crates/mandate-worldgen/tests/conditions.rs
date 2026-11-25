#![allow(clippy::unwrap_used, clippy::expect_used)]
//! The one condition evaluator (DEC-098 §5, task U1) in a synthetic state:
//! every variant of the one `Condition` evaluates in the context its uses
//! bind (a town's situation, a place, a conversation, a cast), holds where
//! the state makes it true, and gives the Fluent arguments it documents; a
//! local-only variant never holds for a region or the nation; and
//! `holds_power` reads the powers through the constitution, so a renamed
//! office still evaluates.

use mandate_content::condition::CondTier;
use mandate_content::conversation::ConcernOf;
use mandate_content::{
    Condition, ContentBundle, HeardSubject, Power, TrackKey, bundle::read_dir_sources, load,
};
use mandate_sim::characters::{self, CharacterId, MemoryKind, Relationship};
use mandate_sim::condition::{self, Args, Ctx};
use mandate_sim::narrative::cast::View;
use mandate_sim::narrative::{Actor, AnswerRecord, CastEntry, DecisionId};
use mandate_sim::places::Location;
use mandate_sim::player::knowledge::{self, Level, Source, Subject};
use mandate_sim::politics::Role;
use mandate_sim::scheduler::Reads;
use mandate_sim::state::State;
use mandate_sim::talk::AboutRef;
use mandate_sim::talk::context::Ctx as TalkCtx;
use mandate_sim::world::{Jurisdiction, MunicipalityId, NeighborhoodId};
use mandate_sim::{Command, Engine};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Arc;

fn base() -> Arc<ContentBundle> {
    let src =
        read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap();
    Arc::new(load(&src).unwrap().0)
}

fn parse(s: &str) -> Condition {
    serde_json::from_str(&mandate_content::jsonc::to_strict_json(s))
        .unwrap_or_else(|e| panic!("{s}: {e}"))
}

/// Where a probe is evaluated.
#[derive(Clone)]
enum At {
    /// A town's situation (a policy's unlock).
    Town,
    /// A place, as a visit or an overheard line binds it.
    Here(Location),
    /// A conversation with the speaker, about a subject.
    Talk(Option<AboutRef>),
    /// A cast (dilemmas, goals, storylines), inside the simulation.
    Cast(Vec<CastEntry>),
}

struct World {
    e: Engine,
    town: MunicipalityId,
    hood: NeighborhoodId,
    me: CharacterId,
    speaker: CharacterId,
    rival: CharacterId,
}

/// A small world a few weeks in, with a crisis, an industry, a broker, a
/// rival, a closure on record, a disaster, a law, a known broker, residents
/// who know the player, and a speaker with traits, a goal, a favour and a
/// memory of meeting the player.
fn world() -> World {
    let c = base();
    let mut e = generate(&WorldConfig::new(7, WorldSize::Small), c.clone()).unwrap();
    for _ in 0..3 {
        e.apply(Command::EndTurn).unwrap();
    }
    let st = e.state();
    let town = st.player.as_ref().unwrap().home();
    let me = characters::player_character(st).unwrap();
    let hood = NeighborhoodId(st.world.hoods_of(town).next().unwrap() as u32);
    let in_town = |id: CharacterId| {
        characters::municipality_of(&st.politics, &st.world, id) == Some(town.0) && id != me
    };
    let speaker = st
        .politics
        .characters
        .people
        .iter()
        .find(|p| p.named.is_some() && p.politician.is_none() && in_town(p.id))
        .unwrap()
        .id;
    let rival = st
        .politics
        .politicians
        .iter()
        .find(|p| !p.is_player && in_town(p.character))
        .unwrap()
        .character;
    let sector = c.sectors[0].id.clone();
    let goal = c.goals[0].id.clone();
    let template = c
        .policies
        .values()
        .find(|t| t.unlock.is_none() && t.tier == mandate_content::TierKey::Local)
        .unwrap()
        .clone();
    e.setup(|st| {
        let date = st.calendar.date;
        let t = &mut st.politics.characters.towns[town.index()];
        t.crises.push(characters::ActiveCrisis {
            kind: "crisis.plant_closure".into(),
            severity: 0.6,
            since: date,
            history: vec![0.6],
        });
        t.industry = Some(sector.clone());
        t.broker = Some(speaker);
        t.rival = Some(rival);
        st.economy.munis[town.index()].months_since_disaster = 0;
        st.events.record(
            date,
            mandate_sim::events::EventKind::PlantClosure {
                municipality: town.0,
                sector: sector.clone(),
                jobs: 120,
                industry: None,
                plant: None,
                factor: None,
                upstream: None,
            },
            mandate_sim::spine::Cause::Lab,
        );
        for x in &mut st.population.hoods[hood.index()].cohorts {
            x.player_awareness = 0.6;
            x.player_approval = 0.3;
        }
        let pol = &mut st.politics;
        pol.characters.get_mut(speaker).unwrap().traits = vec!["trait.vain".into()];
        pol.characters.goals.push(characters::goals::Goal {
            holder: speaker,
            goal: goal.clone(),
            since: date,
            lean: 0.0,
            champion: 0.0,
        });
        pol.characters.edges.insert(
            (speaker.0, me.0),
            Relationship {
                opinion: 50.0,
                trust: 0.8,
                bonds: Vec::new(),
            },
        );
        pol.characters.favours.push(characters::Favour {
            debtor: me,
            creditor: speaker,
            count: 1,
            context: "favour-bill".into(),
            subject: None,
            since: date,
        });
        // The player holds the town's executive office (its powers are the
        // constitution's).
        let pid = st.player.as_ref().unwrap().politician;
        st.politics.get_mut(pid).unwrap().role = Role::Office {
            office: mandate_content::PostKey::new("mayor").unwrap(),
            at: Jurisdiction::town(town),
        };
        // A law the player sponsored, in force.
        let mut law = mandate_sim::policy::probe(&template, town.0, &BTreeMap::new());
        law.id = mandate_sim::policy::PolicyInstanceId(st.policy.instances.len() as u32);
        law.sponsor = Some(pid);
        law.status = mandate_sim::policy::PolicyStatus::InEffect;
        law.introduced_on = Some(mandate_core::SimDate(date.0 - 40));
        law.enacted_on = Some(mandate_core::SimDate(date.0 - 20));
        st.policy.instances.push(law);
    });
    let c2 = e.content_arc();
    e.setup(|st| {
        let date = st.calendar.date;
        characters::remember(
            &mut st.politics,
            &c2,
            mandate_core::SimDate(date.0 - 7),
            speaker,
            me,
            MemoryKind::Met,
            None,
            None,
        );
        knowledge::teach_one(
            st,
            Subject::Broker { town },
            Level::Known,
            Source::LocalOption,
        );
    });
    World {
        e,
        town,
        hood,
        me,
        speaker,
        rival,
    }
}

/// Evaluate a condition where a probe says, for jurisdiction `at`.
fn eval(
    w: &World,
    content: &ContentBundle,
    at: &At,
    juris: Option<Jurisdiction>,
    answers: Option<Vec<AnswerRecord>>,
    c: &Condition,
) -> (bool, Args) {
    let st: &State = w.e.state();
    let r = Reads::of_state(st, content);
    let mut v = View::new(&r, &st.politics);
    v.answers = answers;
    let talk = TalkCtx {
        st,
        content,
        town: w.town,
        me: w.me,
        speaker: w.speaker,
        about: match at {
            At::Talk(a) => a.as_ref(),
            _ => None,
        },
        here: Location::Street {
            neighborhood: w.hood,
        },
        cap: Level::Detailed,
    };
    let mut cx = match at {
        At::Town => Ctx::town(&v, st, w.town),
        At::Here(loc) => {
            let mut cx = Ctx::town(&v, st, w.town);
            cx.here = Some(*loc);
            cx
        }
        At::Talk(_) => Ctx::talk(&v, &talk),
        At::Cast(cast) => Ctx::narrative(&v, cast),
    };
    if juris.is_some() {
        cx.at = juris;
    }
    let mut args = Args::new();
    let ok = condition::holds(&cx, c, &mut args);
    (ok, args)
}

/// Every variant of the one language, as the schema lists them (the old
/// `live_crisis` spelling aside).
fn every_variant() -> BTreeSet<String> {
    let v = mandate_content::content_schema();
    v.pointer("/$defs/Condition/oneOf")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|x| x.pointer("/properties/type/const").and_then(|c| c.as_str()))
        .filter(|k| *k != "live_crisis")
        .map(str::to_string)
        .collect()
}

#[test]
fn every_variant_evaluates_in_a_synthetic_state() {
    let w = world();
    let c = w.e.content_arc();
    let st = w.e.state();
    let r = Reads::of_state(st, &c);
    let v = View::new(&r, &st.politics);
    let street = Location::Street {
        neighborhood: w.hood,
    };
    let law = st.policy.instances.last().unwrap();
    let law_id = law.id.0;
    let law_template = law.template.clone();
    let cast = vec![
        CastEntry {
            role: "who".into(),
            actor: Actor::Person { id: w.speaker },
        },
        CastEntry {
            role: "bill".into(),
            actor: Actor::Policy { id: law_id },
        },
    ];
    // What the state reads, computed independently, for the probes whose
    // answer is not forced by the setup.
    let unemployment = mandate_sim::concerns::indicator(
        &r,
        w.town.index(),
        mandate_content::Indicator::Unemployment,
    );
    let town_cx = Ctx::town(&v, st, w.town);
    let felt = condition::ranked_worries(
        &town_cx,
        ConcernOf::Town,
        mandate_content::condition::Worry::Felt,
    );
    let priorities = st.politics.characters.towns[w.town.index()]
        .priorities
        .clone();
    let mut here_cx = Ctx::town(&v, st, w.town);
    here_cx.here = Some(street);
    let here_measured = condition::ranked_worries(
        &here_cx,
        ConcernOf::Here,
        mandate_content::condition::Worry::Measured,
    );
    let mu = &st.world.municipalities[w.town.index()];
    let wet = mu.on_river
        || mu.coastal
        || st.world.neighborhoods[w.hood.index()].kind
            == mandate_sim::world::NeighborhoodKind::Waterfront;
    let scandal = st
        .media
        .scandals
        .iter()
        .any(|x| x.running() && x.town == w.town.0);
    let headline = st.media.stories.iter().any(|s| {
        s.town == w.town.0
            && s.date.days_until(st.calendar.date) <= condition::NEWS_DAYS
            && st
                .media
                .outlet(s.outlet)
                .is_some_and(|o| o.covers(&st.world, w.town.0))
    });
    let campaign = !st.politics.elections.races.is_empty();
    let speaker_role = condition::role_key(&town_cx, w.speaker).unwrap();
    let trust = characters::trust(&st.politics, &c, w.speaker, w.me) as f64;
    let party_line = v.party_line(law_id) != 0;
    let can_vote = v.can_vote(law_id);
    // A place in town with someone of an occupation there now, and an
    // industry that sheds jobs (set below on a copy).
    let now = characters::routines::Now::of(st, &c);
    let (present_at, present_occ) = st
        .world
        .places
        .iter()
        .filter(|p| p.municipality == w.town)
        .find_map(|p| {
            let loc = Location::Place { place: p.id };
            characters::routines::present(st, &c, &now, loc)
                .into_iter()
                .filter(|x| *x != w.me)
                .find_map(|x| {
                    st.politics
                        .characters
                        .get(x)?
                        .named
                        .as_deref()
                        .map(|n| n.occupation.clone())
                })
                .map(|o| (loc, o))
        })
        .expect("someone of an occupation somewhere in town");
    let tells_town = {
        let t = TalkCtx {
            st,
            content: &c,
            town: w.town,
            me: w.me,
            speaker: w.speaker,
            about: None,
            here: street,
            cap: Level::Detailed,
        };
        t.subject(&mandate_content::conversation::SubjectRef::TownConcerns)
            .is_some_and(|s| t.told(&s) >= Level::Heard)
            && !felt.is_empty()
    };
    let voice = {
        let t = TalkCtx {
            st,
            content: &c,
            town: w.town,
            me: w.me,
            speaker: w.speaker,
            about: None,
            here: street,
            cap: Level::Detailed,
        };
        mandate_sim::talk::reply::voice(&t).unwrap().id.clone()
    };
    let first_goal = c.goals[0].id.clone();
    let answers = vec![AnswerRecord {
        decision: DecisionId(1),
        template: c.dilemmas[0].id.clone(),
        option: c.dilemmas[0].options[0].id.clone(),
        date: mandate_core::SimDate(st.calendar.date.0 - 30),
        by_default: false,
        cast: cast.clone(),
    }];
    let sp = "speaker";
    let mut probes: Vec<(Condition, At, bool)> = vec![
        (
            parse(r#"{ "type": "crisis", "kind": "crisis.plant_closure", "min_severity": 0.5 }"#),
            At::Town,
            true,
        ),
        (
            parse(r#"{ "type": "crisis", "kind": "crisis.plant_closure", "min_severity": 0.9 }"#),
            At::Town,
            false,
        ),
        (
            parse(r#"{ "type": "live_crisis", "min_severity": 0.5 }"#),
            At::Cast(cast.clone()),
            true,
        ),
        (
            Condition::Indicator {
                indicator: mandate_content::Indicator::Unemployment,
                min: Some(unemployment - 0.001),
                max: None,
            },
            At::Cast(cast.clone()),
            true,
        ),
        (parse(r#"{ "type": "industry" }"#), At::Here(street), true),
        (
            parse(r#"{ "type": "plant_closure", "within_months": 3 }"#),
            At::Here(street),
            true,
        ),
        (
            parse(r#"{ "type": "plant_closure", "within_months": 3 }"#),
            At::Cast(cast.clone()),
            false,
        ),
        (
            parse(r#"{ "type": "disaster", "within_months": 1 }"#),
            At::Town,
            true,
        ),
        (
            Condition::LawInForce {
                templates: vec![law_template.clone()],
                absent: false,
            },
            At::Cast(cast.clone()),
            true,
        ),
        (
            parse(r#"{ "type": "water", "feature": "any" }"#),
            At::Here(street),
            wet,
        ),
        (parse(r#"{ "type": "dry" }"#), At::Here(street), !wet),
        (parse(r#"{ "type": "broker" }"#), At::Here(street), true),
        (parse(r#"{ "type": "rival" }"#), At::Here(street), true),
        (
            parse(r#"{ "type": "rival_record", "min": 0, "max": 1 }"#),
            At::Here(street),
            true,
        ),
        (
            Condition::Scandal {
                about: HeardSubject::Anyone,
            },
            At::Here(street),
            scandal,
        ),
        (
            parse(r#"{ "type": "headline" }"#),
            At::Here(street),
            headline,
        ),
        (
            parse(r#"{ "type": "player_awareness", "min": 0.5 }"#),
            At::Here(street),
            true,
        ),
        (
            parse(r#"{ "type": "player_approval", "min": 0.2 }"#),
            At::Here(street),
            true,
        ),
        (
            parse(r#"{ "type": "player_awareness", "min": 0.5 }"#),
            At::Town,
            false,
        ),
        (
            Condition::Present {
                occupations: vec![present_occ.clone()],
            },
            At::Here(present_at),
            true,
        ),
        (
            Condition::Present {
                occupations: vec![present_occ],
            },
            At::Cast(cast.clone()),
            false,
        ),
        (
            parse(
                r#"{ "type": "day_part", "parts": ["morning", "afternoon", "evening", "late_night"] }"#,
            ),
            At::Here(street),
            true,
        ),
        (
            parse(r#"{ "type": "day_part", "parts": ["morning"] }"#),
            At::Cast(cast.clone()),
            false,
        ),
        (
            Condition::Phase {
                phase: mandate_content::condition::Phase::Campaign,
            },
            At::Town,
            campaign,
        ),
        (
            Condition::Phase {
                phase: mandate_content::condition::Phase::Governing,
            },
            At::Cast(cast.clone()),
            !campaign,
        ),
        (
            parse(r#"{ "type": "election_within", "days": 3650 }"#),
            At::Cast(cast.clone()),
            true,
        ),
        (
            parse(r#"{ "type": "months_in_play", "min": 0 }"#),
            At::Cast(cast.clone()),
            true,
        ),
        (
            Condition::Campaigning {
                is: v.campaigning(),
            },
            At::Cast(cast.clone()),
            true,
        ),
        (
            parse(r#"{ "type": "office", "role": "mayor" }"#),
            At::Cast(cast.clone()),
            true,
        ),
        (
            parse(r#"{ "type": "office", "posts": ["none"] }"#),
            At::Cast(cast.clone()),
            false,
        ),
        (
            Condition::HoldsPower { power: Power::Veto },
            At::Cast(cast.clone()),
            true,
        ),
        (
            Condition::HoldsPower {
                power: Power::ApproveOrdinance,
            },
            At::Town,
            false,
        ),
        (
            Condition::Track {
                track: TrackKey::Viability,
                min: Some(0.0),
                max: None,
            },
            At::Cast(cast.clone()),
            true,
        ),
        (
            parse(r#"{ "type": "money", "min": -1000000000 }"#),
            At::Cast(cast.clone()),
            true,
        ),
        (
            Condition::Answered {
                template: c.dilemmas[0].id.clone(),
                option: None,
                min_days: 10,
                max_days: Some(60),
                same_cast: vec!["who".into()],
            },
            At::Cast(cast.clone()),
            true,
        ),
        (
            parse(r#"{ "type": "sponsored_law", "role": "bill", "min_age_days": 10 }"#),
            At::Cast(cast.clone()),
            true,
        ),
        (
            parse(r#"{ "type": "sponsored_law", "status": "pending" }"#),
            At::Cast(cast.clone()),
            false,
        ),
        (
            parse(r#"{ "type": "knows", "subject": { "type": "broker" }, "min": "known" }"#),
            At::Cast(cast.clone()),
            true,
        ),
        (
            parse(r#"{ "type": "player_knows", "subject": { "type": "broker" }, "min": "known" }"#),
            At::Talk(None),
            true,
        ),
        (
            Condition::Occupation {
                slot: sp.into(),
                occupations: vec![speaker_role.clone()],
                is: true,
            },
            At::Talk(None),
            true,
        ),
        (
            Condition::Occupation {
                slot: "who".into(),
                occupations: vec![speaker_role],
                is: true,
            },
            At::Cast(cast.clone()),
            true,
        ),
        (
            parse(r#"{ "type": "trait", "slot": "speaker", "traits": ["trait.vain"] }"#),
            At::Talk(None),
            true,
        ),
        (
            parse(r#"{ "type": "trait", "role": "who", "traits": ["trait.vain"] }"#),
            At::Cast(cast.clone()),
            true,
        ),
        (
            parse(r#"{ "type": "personality", "slot": "speaker", "axis": "warmth", "min": 0 }"#),
            At::Talk(None),
            true,
        ),
        (
            Condition::Goal {
                slot: "who".into(),
                goal: first_goal,
            },
            At::Cast(cast.clone()),
            true,
        ),
        (
            parse(r#"{ "type": "opinion", "slot": "speaker", "min": 40 }"#),
            At::Talk(None),
            true,
        ),
        (
            parse(r#"{ "type": "opinion", "role": "who", "max": 40 }"#),
            At::Cast(cast.clone()),
            false,
        ),
        (
            Condition::Trust {
                slot: sp.into(),
                min: Some(trust - 0.01),
                max: None,
            },
            At::Talk(None),
            true,
        ),
        (
            parse(r#"{ "type": "remembers", "slot": "speaker", "kinds": ["met"] }"#),
            At::Talk(None),
            true,
        ),
        (
            parse(r#"{ "type": "favour", "slot": "speaker" }"#),
            At::Talk(None),
            true,
        ),
        (
            parse(r#"{ "type": "favour", "role": "who", "owed_to_player": true }"#),
            At::Cast(cast.clone()),
            false,
        ),
        (
            parse(r#"{ "type": "met", "slot": "speaker" }"#),
            At::Talk(None),
            true,
        ),
        (
            parse(r#"{ "type": "is_broker", "slot": "speaker" }"#),
            At::Talk(None),
            true,
        ),
        (
            parse(r#"{ "type": "is_rival", "slot": "speaker", "is": false }"#),
            At::Talk(None),
            true,
        ),
        (
            parse(r#"{ "type": "is_rival", "slot": "about" }"#),
            At::Talk(Some(AboutRef::Rival)),
            true,
        ),
        (
            parse(r#"{ "type": "party_line_against_ward", "slot": "bill" }"#),
            At::Cast(cast.clone()),
            party_line && {
                let lean = v.ward_lean(law_id);
                lean.abs() > 0.02 && (v.party_line(law_id) as f64) * lean < 0.0
            },
        ),
        (
            parse(r#"{ "type": "can_vote", "slot": "bill" }"#),
            At::Cast(cast.clone()),
            can_vote,
        ),
        (
            parse(r#"{ "type": "tells", "subject": { "type": "town_concerns" }, "min": "heard" }"#),
            At::Talk(None),
            tells_town,
        ),
        (
            Condition::Voice {
                voices: vec![voice],
            },
            At::Talk(None),
            true,
        ),
        (
            parse(r#"{ "type": "about", "kinds": ["rival"] }"#),
            At::Talk(Some(AboutRef::Rival)),
            true,
        ),
        (
            parse(r#"{ "type": "about", "kinds": ["rival"] }"#),
            At::Talk(None),
            false,
        ),
        (
            Condition::Any {
                of: vec![
                    parse(r#"{ "type": "disaster", "within_months": 0 }"#),
                    parse(r#"{ "type": "crisis" }"#),
                ],
            },
            At::Town,
            true,
        ),
    ];
    // Worries: the town's felt top one, its measured priorities, the
    // speaker's home, and the neighbourhood here.
    if let Some(i) = felt.first() {
        probes.push((
            Condition::Concern {
                of: ConcernOf::Town,
                concern: None,
                concerns: vec![c.concerns[*i].id.clone()],
                top: None,
                by: None,
            },
            At::Talk(None),
            true,
        ));
    }
    if let Some(p) = priorities.first() {
        probes.push((
            parse(&format!(
                r#"{{ "type": "concern", "concern": "{p}", "top": 1 }}"#
            )),
            At::Cast(cast.clone()),
            true,
        ));
    }
    if let Some(i) = here_measured.first() {
        probes.push((
            Condition::Concern {
                of: ConcernOf::Here,
                concern: None,
                concerns: vec![c.concerns[*i].id.clone()],
                top: Some(1),
                by: Some(mandate_content::condition::Worry::Measured),
            },
            At::Here(street),
            true,
        ));
    }
    probes.push((
        Condition::Concern {
            of: ConcernOf::Speaker,
            concern: None,
            concerns: c.concerns.iter().map(|x| x.id.clone()).collect(),
            top: Some(6),
            by: None,
        },
        At::Talk(None),
        true,
    ));
    let mut seen = BTreeSet::new();
    for (cond, at, expect) in &probes {
        let answers = matches!(cond, Condition::Answered { .. }).then(|| answers.clone());
        let (ok, args) = eval(&w, &c, at, None, answers, cond);
        assert_eq!(ok, *expect, "{cond:?}");
        seen.insert(cond.key().to_string());
        if ok {
            // The arguments it documents, and no others (a place without a
            // proper name comes as its kind's key, `x_key` for `x`).
            let stem = |k: &String| k.strip_suffix("_key").unwrap_or(k).to_string();
            if let Some(p) = cond.provides() {
                let p: BTreeSet<String> = p.iter().map(stem).collect();
                let got: BTreeSet<String> = args.keys().map(stem).collect();
                let narrative_broker =
                    matches!(cond, Condition::IsBroker { .. }) && matches!(at, At::Cast(_));
                if matches!(cond, Condition::Any { .. }) {
                    // An `any` guarantees what every alternative gives.
                    assert!(p.is_subset(&got), "{cond:?}: {got:?}");
                } else if !narrative_broker {
                    assert_eq!(got, p, "{cond:?}");
                }
            }
            // DEC-098: a local-only variant never holds for a region, the
            // nation, or no jurisdiction.
            if cond.tier() == CondTier::Local {
                let region = st.world.municipalities[w.town.index()].region;
                for j in [Jurisdiction::Region { id: region }, Jurisdiction::Nation] {
                    let (ok, _) = eval(&w, &c, at, Some(j), None, cond);
                    assert!(!ok, "{cond:?} held for {j:?}");
                }
            }
        }
    }
    // Shedding: the place's industry letting people go (on a copy).
    let mut w2 = world();
    let (works, sector) = {
        let st = w2.e.state();
        st.world
            .places
            .iter()
            .filter(|p| p.municipality == w2.town)
            .find_map(|p| {
                let s = c.place_kind(p.kind.key())?.sector.clone()?;
                let k = c.sector_index(&s)?;
                (st.economy.munis[w2.town.index()].sectors[k].capacity > 0.0).then_some((p.id, k))
            })
            .expect("a place with an industry")
    };
    let town = w2.town;
    w2.e.setup(|st| {
        let s = &mut st.economy.munis[town.index()].sectors[sector];
        s.jobs = s.capacity * 0.5;
    });
    let shed = parse(r#"{ "type": "shedding", "min": 0.5 }"#);
    let (ok, args) = eval(
        &w2,
        &c,
        &At::Here(Location::Place { place: works }),
        None,
        None,
        &shed,
    );
    assert!(ok && args.contains_key("industry_key"));
    let (ok, _) = eval(&w, &c, &At::Here(street), None, None, &shed);
    assert!(!ok, "a street has no industry");
    seen.insert("shedding".into());
    // A policy family's bound slots (DEC-092): the neighbourhood, the place
    // with an industry and its sector, bound as a family would bind them.
    {
        use mandate_sim::policy::families::{Bound, SlotBinding};
        let sector_key = c.sectors[sector].id.clone();
        w2.e.setup(|st| {
            let s = &mut st.economy.munis[town.index()].sectors[sector];
            s.baseline_jobs = (s.jobs * 2.0) as u32;
        });
        let bindings = vec![
            SlotBinding {
                slot: "hood".into(),
                bound: Bound::Neighborhood { hood: w2.hood },
            },
            SlotBinding {
                slot: "plant".into(),
                bound: Bound::Place { place: works },
            },
            SlotBinding {
                slot: "sector".into(),
                bound: Bound::Sector { sector: sector_key },
            },
        ];
        let fam = |w: &World, j: Option<Jurisdiction>, cond: &Condition| -> bool {
            let st: &State = w.e.state();
            let r = Reads::of_state(st, &c);
            let v = View::new(&r, &st.politics);
            let mut cx = Ctx::town(&v, st, w.town);
            if j.is_some() {
                cx.at = j;
            }
            cx.family = Some(&bindings);
            condition::holds(&cx, cond, &mut Args::new())
        };
        let st = w2.e.state();
        let levels = &st.population.hood_concerns[w2.hood.index()].levels;
        let top = (0..levels.len())
            .max_by(|a, b| levels[*a].total_cmp(&levels[*b]).then(b.cmp(a)))
            .unwrap();
        let low = (0..levels.len())
            .min_by(|a, b| levels[*a].total_cmp(&levels[*b]))
            .unwrap();
        let concern = |i: usize, n: u8| {
            parse(&format!(
                r#"{{ "type": "hood_concern", "slot": "hood", "concern": "{}", "top": {n} }}"#,
                c.concerns[i].id
            ))
        };
        assert!(fam(&w2, None, &concern(top, 1)));
        if levels[low] < levels[top] {
            assert!(!fam(&w2, None, &concern(low, 1)));
        }
        let hazard =
            parse(r#"{ "type": "hood_hazard", "slot": "hood", "hazard": "flood", "min": 0.0 }"#);
        assert!(fam(&w2, None, &hazard));
        assert!(
            !fam(
                &w2,
                None,
                &parse(
                    r#"{ "type": "hood_hazard", "slot": "hood", "hazard": "flood", "min": 1.0 }"#
                )
            ) || st
                .environment
                .exposure(w2.hood)
                .is_some_and(|e| e.flood >= 1000)
        );
        let decline = parse(r#"{ "type": "sector_decline", "slot": "sector", "min": 0.3 }"#);
        assert!(fam(&w2, None, &decline));
        assert!(!fam(
            &w2,
            None,
            &parse(r#"{ "type": "sector_decline", "slot": "sector", "min": 0.9 }"#)
        ));
        let open = parse(r#"{ "type": "place_open", "slot": "plant" }"#);
        let closed = parse(r#"{ "type": "place_closed", "slot": "plant" }"#);
        assert!(fam(&w2, None, &open) && !fam(&w2, None, &closed));
        // A slot the family did not bind never holds.
        assert!(!fam(
            &w2,
            None,
            &parse(r#"{ "type": "place_open", "slot": "market" }"#)
        ));
        // Local-only: never for a region or the nation.
        let region = st.world.municipalities[town.index()].region;
        for j in [Jurisdiction::Region { id: region }, Jurisdiction::Nation] {
            for cond in [&concern(top, 3), &hazard, &decline, &open] {
                assert!(!fam(&w2, Some(j), cond), "{cond:?} held for {j:?}");
            }
        }
        // Closed, the place reads the other way.
        let date = st.calendar.date;
        w2.e.setup(|st| {
            st.world.close_place(works, date);
        });
        assert!(!fam(&w2, None, &open) && fam(&w2, None, &closed));
        for k in [
            "hood_concern",
            "hood_hazard",
            "sector_decline",
            "place_open",
            "place_closed",
        ] {
            seen.insert(k.into());
        }
    }
    // --- Staged arcs (DEC-109): a run's own readings, and `arc` -------------------
    {
        use mandate_sim::arcs::{ArcActor, ArcCastEntry, ArcId, ArcRun};
        use mandate_sim::world::Area;
        let mut w3 = world();
        let content = w3.e.content_arc();
        let today = w3.e.state().calendar.date;
        let (town, hood) = (w3.town, w3.hood);
        let at = Jurisdiction::town(town);
        let since = mandate_core::SimDate(today.0 - 10);
        let run = ArcRun {
            id: ArcId(0),
            template: "arc.flood".into(),
            at,
            cast: vec![ArcCastEntry {
                role: "hood".into(),
                actor: ArcActor::Area {
                    area: Area::Hood { id: hood },
                },
            }],
            stage: "counting".into(),
            since,
            started: since,
            began: since,
            clock: None,
            heat: 0.5,
            marks: vec!["seen".into()],
            done: Vec::new(),
            open: Vec::new(),
            base: vec![("hood.damage".into(), 1000), ("hood.homes".into(), 40)],
            root: None,
            turn: None,
            before: Some("turn_hole".into()),
            exchange: None,
            played: Vec::new(),
        };
        let held = |e: &Engine, s: &str, run: Option<&ArcRun>, here: Option<Location>| {
            let st = e.state();
            let r = Reads::of_state(st, &content);
            let v = View::new(&r, &st.politics);
            let mut cx = Ctx::new(&v, Some(st), Some(at));
            cx.arc = run;
            cx.here = here;
            let mut a = Args::new();
            (condition::holds(&cx, &parse(s), &mut a), a)
        };
        let on = |s: &str| held(&w3.e, s, Some(&run), None);
        assert!(on(r#"{ "type": "arc_mark", "mark": "seen" }"#).0);
        assert!(!on(r#"{ "type": "arc_mark", "mark": "other" }"#).0);
        assert!(on(r#"{ "type": "arc_mark", "mark": "other", "is": false }"#).0);
        assert!(on(r#"{ "type": "arc_heat", "min": 0.4, "max": 0.6 }"#).0);
        assert!(!on(r#"{ "type": "arc_heat", "min": 0.6 }"#).0);
        assert!(on(r#"{ "type": "arc_days", "min": 10, "max": 10 }"#).0);
        assert!(!on(r#"{ "type": "arc_days", "min": 11 }"#).0);
        // Nobody is away and nothing is damaged now: all of what was
        // damaged as the run was raised is repaired.
        let (ok, a) = on(r#"{ "type": "displaced", "slot": "hood", "max": 0.0 }"#);
        assert!(ok && a["away"] == "0", "{a:?}");
        assert!(!on(r#"{ "type": "displaced", "slot": "hood", "min": 0.01 }"#).0);
        let (ok, a) = on(r#"{ "type": "recovered", "slot": "hood", "min": 1.0 }"#);
        assert!(ok && a["homes"] == "40" && a["repaired"] == "40", "{a:?}");
        // Without bounds they are readings: they always hold.
        assert!(on(r#"{ "type": "displaced", "slot": "hood" }"#).0);
        assert!(on(r#"{ "type": "recovered", "slot": "hood" }"#).0);
        // A role the run does not have, or no run at all: never.
        assert!(!on(r#"{ "type": "displaced", "slot": "nowhere" }"#).0);
        for s in [
            r#"{ "type": "arc_mark", "mark": "seen" }"#,
            r#"{ "type": "arc_heat", "min": 0.0 }"#,
            r#"{ "type": "arc_days", "min": 0 }"#,
            r#"{ "type": "displaced", "slot": "hood" }"#,
            r#"{ "type": "recovered", "slot": "hood" }"#,
        ] {
            assert!(!held(&w3.e, s, None, None).0, "{s} held with no run");
        }
        // `arc`: a run of this arc at one of these stages here; with
        // `here`, only where that role of the run is.
        let arc = r#"{ "type": "arc", "arc": "arc.flood", "stages": ["counting"] }"#;
        let here = r#"{ "type": "arc", "arc": "arc.flood", "here": "hood" }"#;
        let street = |h: NeighborhoodId| Some(Location::Street { neighborhood: h });
        assert!(!held(&w3.e, arc, None, None).0, "no run yet");
        w3.e.setup(|st| st.arcs.runs.push(run.clone()));
        assert!(held(&w3.e, arc, None, None).0);
        assert!(
            !held(
                &w3.e,
                r#"{ "type": "arc", "arc": "arc.flood", "stages": ["struck"] }"#,
                None,
                None
            )
            .0
        );
        assert!(held(&w3.e, here, None, street(hood)).0);
        let other = w3.e.state().world.municipalities[town.index()]
            .neighborhoods
            .iter()
            .copied()
            .find(|h| *h != hood)
            .unwrap();
        assert!(!held(&w3.e, here, None, street(other)).0);
        // `arc_last_turn`: the turn the last run of the template here
        // reached (climax memory by variant).
        let last = |s: &str| held(&w3.e, s, Some(&run), None).0;
        assert!(last(r#"{ "type": "arc_last_turn", "stage": "turn_hole" }"#));
        assert!(!last(
            r#"{ "type": "arc_last_turn", "stage": "turn_line" }"#
        ));
        assert!(last(
            r#"{ "type": "arc_last_turn", "stage": "turn_line", "is": false }"#
        ));
        assert!(
            !held(
                &w3.e,
                r#"{ "type": "arc_last_turn", "stage": "turn_hole" }"#,
                None,
                None
            )
            .0
        );
        for k in [
            "arc",
            "arc_mark",
            "arc_heat",
            "arc_days",
            "displaced",
            "recovered",
            "arc_last_turn",
        ] {
            seen.insert(k.into());
        }

        // --- The budget and the body that votes it (tier-generic) ------------
        let at_any = |e: &Engine, s: &str, j: Jurisdiction| {
            let st = e.state();
            let r = Reads::of_state(st, &content);
            let v = View::new(&r, &st.politics);
            let cx = Ctx::new(&v, Some(st), Some(j));
            let mut a = Args::new();
            (condition::holds(&cx, &parse(s), &mut a), a)
        };
        // `budget_year`: days until the budget year turns (1 January).
        let left = today.days_until(mandate_core::SimDate::from_ymd(
            today.calendar().year + 1,
            1,
            1,
        ));
        assert!(
            at_any(
                &w3.e,
                &format!(r#"{{ "type": "budget_year", "min": {left}, "max": {left} }}"#),
                at
            )
            .0
        );
        assert!(
            !at_any(
                &w3.e,
                &format!(r#"{{ "type": "budget_year", "max": {} }}"#, left - 1),
                at
            )
            .0
        );
        // `budget_balance`: last year's balance over its revenue, for the
        // treasury of the jurisdiction (a town's, a region's, the nation's).
        w3.e.setup(|st| {
            let t = &mut st.finance.local[town.index()];
            t.last_year.year = 1;
            t.last_year.revenue = mandate_core::Money::from_units(1000);
            t.last_year.balance = mandate_core::Money::from_units(-50);
        });
        let (ok, a) = at_any(&w3.e, r#"{ "type": "budget_balance", "max": -0.02 }"#, at);
        assert!(ok && a["books"] == "short" && a["gap"] == "5", "{a:?}");
        assert!(!at_any(&w3.e, r#"{ "type": "budget_balance", "min": 0.0 }"#, at).0);
        assert!(
            at_any(&w3.e, r#"{ "type": "budget_balance" }"#, at).0,
            "a reading always holds"
        );
        w3.e.setup(|st| {
            st.finance.local[town.index()].last_year.balance = mandate_core::Money::from_units(70);
        });
        let (ok, a) = at_any(&w3.e, r#"{ "type": "budget_balance", "min": 0.05 }"#, at);
        assert!(ok && a["books"] == "over" && a["gap"] == "7", "{a:?}");
        let region = w3.e.state().world.municipalities[town.index()].region;
        for j in [Jurisdiction::Region { id: region }, Jurisdiction::Nation] {
            let (ok, a) = at_any(&w3.e, r#"{ "type": "budget_balance" }"#, j);
            assert!(ok && a.contains_key("books"), "{j:?}: {a:?}");
        }
        // `body_majority`: one party with more than half (or a share) of
        // the posts that list the power, read from the bodies as seated.
        // (A member who cannot use the power now, one in custody say, is
        // not counted: the condition reads who holds it, not who is listed.)
        let st = w3.e.state();
        let constitution =
            w3.e.content()
                .constitutions
                .get(&st.meta.constitution)
                .unwrap();
        let listed = st
            .politics
            .bodies
            .iter()
            .filter(|b| b.at == at)
            .flat_map(|b| b.members.iter())
            .count();
        let holders: Vec<mandate_sim::politics::PoliticianId> = st
            .politics
            .bodies
            .iter()
            .filter(|b| b.at == at)
            .flat_map(|b| b.members.iter().copied())
            .filter(|m| {
                mandate_sim::politics::holds_power_at(
                    &st.politics,
                    constitution,
                    *m,
                    mandate_content::Power::ApproveBudget,
                    at,
                )
            })
            .collect();
        let seats: Vec<Option<mandate_sim::politics::PartyId>> = holders
            .iter()
            .map(|m| st.politics.get(*m).unwrap().party)
            .collect();
        eprintln!(
            "body_majority: {listed} listed, {} hold the power",
            seats.len()
        );
        let most = (0..8)
            .map(|k| {
                seats
                    .iter()
                    .filter(|p| **p == Some(mandate_sim::politics::PartyId(k)))
                    .count()
            })
            .max()
            .unwrap();
        let majority = most * 2 > seats.len();
        let has = |s: &str| at_any(&w3.e, s, at).0;
        assert_eq!(
            has(r#"{ "type": "body_majority", "power": "approve_budget" }"#),
            majority
        );
        assert_eq!(
            has(r#"{ "type": "body_majority", "power": "approve_budget", "is": false }"#),
            !majority
        );
        assert_eq!(
            has(r#"{ "type": "body_majority", "power": "approve_budget", "share": 1.0 }"#),
            most == seats.len()
        );
        // `body_support`: who of the others with the power would back the
        // player: their party, those who think well of them, those in
        // their debt.
        let (ok, a) = at_any(
            &w3.e,
            r#"{ "type": "body_support", "power": "approve_budget" }"#,
            at,
        );
        let pl = st.player.as_ref().unwrap().politician;
        let sits = holders.contains(&pl);
        let (backers, members): (usize, usize) =
            (a["backers"].parse().unwrap(), a["members"].parse().unwrap());
        assert!(ok && members == seats.len() - usize::from(sits) && backers <= members);
        let share = backers as f64 / members as f64;
        assert!(has(&format!(
            r#"{{ "type": "body_support", "power": "approve_budget", "min": {share}, "max": {share} }}"#
        )));
        assert!(
            !has(&format!(
                r#"{{ "type": "body_support", "power": "approve_budget", "min": {} }}"#,
                (share + 0.01).min(1.0) + 0.001
            )) || share >= 0.99
        );
        // An opinion nobody could hold: only the party and debts count.
        let (_, strict) = at_any(
            &w3.e,
            r#"{ "type": "body_support", "power": "approve_budget", "opinion": 100 }"#,
            at,
        );
        assert!(strict["backers"].parse::<usize>().unwrap() <= backers);
        // `last_race`: the player's last decided race, won or lost.
        let stood = st
            .politics
            .elections
            .results
            .iter()
            .filter(|r| r.candidates.iter().any(|c| c.politician == pl))
            .max_by_key(|r| r.date)
            .map(|r| r.winner == pl)
            .filter(|_| !mandate_sim::characters::is_candidate(&st.politics, pl));
        assert_eq!(
            has(r#"{ "type": "last_race", "won": true }"#),
            stood == Some(true)
        );
        assert_eq!(
            has(r#"{ "type": "last_race", "won": false }"#),
            stood == Some(false)
        );
        // `sponsors_bill`: the person in the slot has a bill the player
        // can still vote on (nobody without a slot does).
        let sponsors = |who: mandate_sim::characters::CharacterId| {
            let st = w3.e.state();
            let r = Reads::of_state(st, &content);
            let v = View::new(&r, &st.politics);
            let cast = vec![CastEntry {
                role: "who".into(),
                actor: Actor::Person { id: who },
            }];
            let mut cx = Ctx::new(&v, Some(st), Some(at));
            cx.cast = &cast;
            let want = st.policy.instances.iter().any(|b| {
                b.status == mandate_sim::policy::PolicyStatus::UnderConsideration
                    && b.sponsor
                        .and_then(|s| st.politics.get(s))
                        .is_some_and(|s| s.character == who)
                    && v.can_vote(b.id.0)
            });
            let got = condition::holds(
                &cx,
                &parse(r#"{ "type": "sponsors_bill", "slot": "who" }"#),
                &mut Args::new(),
            );
            (got, want)
        };
        for m in st
            .politics
            .bodies
            .iter()
            .filter(|b| b.at == at)
            .flat_map(|b| b.members.iter())
        {
            let (got, want) = sponsors(st.politics.get(*m).unwrap().character);
            assert_eq!(got, want);
        }
        assert!(!has(r#"{ "type": "sponsors_bill", "slot": "nobody" }"#));
        // `bill_costs`: a bill of theirs a yes would cost the player is a
        // bill of theirs the player can vote on.
        for m in st
            .politics
            .bodies
            .iter()
            .filter(|b| b.at == at)
            .flat_map(|b| b.members.iter())
        {
            let who = st.politics.get(*m).unwrap().character;
            let st = w3.e.state();
            let r = Reads::of_state(st, &content);
            let v = View::new(&r, &st.politics);
            let cast = vec![CastEntry {
                role: "who".into(),
                actor: Actor::Person { id: who },
            }];
            let mut cx = Ctx::new(&v, Some(st), Some(at));
            cx.cast = &cast;
            let mut a = Args::new();
            let costs = condition::holds(
                &cx,
                &parse(r#"{ "type": "bill_costs", "slot": "who" }"#),
                &mut a,
            );
            assert!(!costs || sponsors(who).1);
            if costs {
                assert!(["seat", "party", "promise"].contains(&a["bind"].as_str()));
            }
        }
        // `plant` with no slot: a plant of the jurisdiction in one of the
        // states asked for, with its name parts and jobs.
        let in_town = |s: &[&str]| {
            st.economy.plants.iter().any(|p| {
                st.world
                    .places
                    .get(p.place.index())
                    .is_some_and(|pl| at.contains_hood(&st.world, pl.neighborhood))
                    && s.contains(&mandate_sim::condition::plant_state(&p.status).key())
            })
        };
        let (any, args) = at_any(&w3.e, r#"{ "type": "plant" }"#, at);
        assert_eq!(
            any,
            in_town(&["running", "short_time", "threatened", "closed"])
        );
        if any {
            assert!(args.contains_key("plant_key") && args.contains_key("jobs"));
        }
        for s in ["running", "short_time", "threatened", "closed"] {
            assert_eq!(
                has(&format!(r#"{{ "type": "plant", "status": ["{s}"] }}"#)),
                in_town(&[s]),
                "{s}"
            );
        }
        for k in [
            "budget_year",
            "budget_balance",
            "body_majority",
            "body_support",
            "last_race",
            "sponsors_bill",
            "bill_costs",
            "plant",
        ] {
            seen.insert(k.into());
        }
    }
    // Every variant was evaluated.
    let missing: Vec<String> = every_variant().difference(&seen).cloned().collect();
    assert!(missing.is_empty(), "not evaluated: {missing:?}");
    let _ = w.rival;
}

/// `holds_power` reads the powers through the constitution (DEC-098): with
/// the executive office renamed, the player in it still holds its powers;
/// no code names the office.
#[test]
fn a_renamed_constitution_still_evaluates_requires_power() {
    let w = world();
    let mut c = (*w.e.content_arc()).clone();
    let id = w.e.state().meta.constitution.clone();
    let con = c.constitutions.get_mut(&id).unwrap();
    let office = con
        .tiers
        .iter_mut()
        .flat_map(|t| t.offices.iter_mut())
        .find(|o| o.id == "mayor")
        .unwrap();
    office.id = "burgomaster".into();
    let mut w = w;
    let town = w.town;
    w.e.setup(|st| {
        let pid = st.player.as_ref().unwrap().politician;
        st.politics.get_mut(pid).unwrap().role = Role::Office {
            office: mandate_content::PostKey::new("burgomaster").unwrap(),
            at: Jurisdiction::town(town),
        };
    });
    let veto = Condition::HoldsPower { power: Power::Veto };
    let approve = Condition::HoldsPower {
        power: Power::ApproveOrdinance,
    };
    assert!(eval(&w, &c, &At::Town, None, None, &veto).0);
    assert!(!eval(&w, &c, &At::Town, None, None, &approve).0);
    // The post serves the town, not the region.
    let region = w.e.state().world.municipalities[town.index()].region;
    assert!(
        !eval(
            &w,
            &c,
            &At::Town,
            Some(Jurisdiction::Region { id: region }),
            None,
            &veto
        )
        .0
    );
    // The old name is gone: a constitution word condition no longer matches it.
    assert!(
        !eval(
            &w,
            &c,
            &At::Town,
            None,
            None,
            &parse(r#"{ "type": "office", "role": "mayor" }"#)
        )
        .0
    );
    assert!(
        eval(
            &w,
            &c,
            &At::Town,
            None,
            None,
            &parse(r#"{ "type": "office", "role": "burgomaster" }"#)
        )
        .0
    );
}

/// The visit verbs' short spellings are the one language's conditions: the
/// verb's `parts` is a `day_part` and its `with` a `present`, with the
/// same answers.
#[test]
fn verb_fields_are_conditions() {
    let w = world();
    let c = w.e.content_arc();
    let st = w.e.state();
    let part = characters::routines::day_part(st);
    let other = mandate_content::routines::DayPart::ALL
        .into_iter()
        .find(|p| *p != part && *p != mandate_content::routines::DayPart::LateNight)
        .unwrap();
    let street = At::Here(Location::Street {
        neighborhood: w.hood,
    });
    assert!(
        eval(
            &w,
            &c,
            &street,
            None,
            None,
            &Condition::DayPart { parts: vec![part] }
        )
        .0
    );
    assert!(
        !eval(
            &w,
            &c,
            &street,
            None,
            None,
            &Condition::DayPart { parts: vec![other] }
        )
        .0
    );
}
