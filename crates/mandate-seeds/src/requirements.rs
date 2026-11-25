//! The seed requirements: each a name, one line on what the world must have,
//! and the predicate that says whether a seed's world has it. A test names a
//! requirement (`mandate_seeds::seed("name")`); a browser spec reads the same
//! name's seed from `tests/seeds.json`; `mandate-cli seeds check|refresh`
//! evaluates every predicate here, so the file can be refreshed and checked.
//!
//! A predicate plays the situation the way the test does, as far as the
//! test's setup needs it, and asserts nothing: it answers yes or no. The
//! test's own assertions stay in the test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use mandate_content::ContentBundle;
use mandate_content::act::SkipTargetKind;
use mandate_content::conversation::TalkPart;
use mandate_protocol::{PostKindDto, campaign, campaign_view, pocket_view, skip_summary_view};
use mandate_sim::actions::PlayerCommand;
use mandate_sim::asks::AskStatus;
use mandate_sim::characters::{self, CharacterId};
use mandate_sim::places::Location;
use mandate_sim::politics::elections::Filing;
use mandate_sim::talk::{self, TalkTarget};
use mandate_sim::world::{Jurisdiction, PlaceKind};
use mandate_sim::{Command, Engine};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::path::Path;
use std::sync::{Arc, OnceLock};

/// A requirement: see the module notes.
#[derive(Clone, Copy)]
pub struct Requirement {
    pub name: &'static str,
    pub needs: &'static str,
    pub holds: fn(u64) -> bool,
}

/// Every requirement, by name. `tests/seeds.json` holds exactly these.
pub const ALL: &[Requirement] = &[
    Requirement {
        name: "act_skip_spans_two_turns",
        needs: "the first NextSession skip, one turn in, spans at least two turns",
        holds: act_skip_spans_two_turns,
    },
    Requirement {
        name: "councillor_and_regular_at_a_place",
        needs: "a non-player councillor, and a regular whose own place is one in the town",
        holds: councillor_and_regular_at_a_place,
    },
    Requirement {
        name: "executive_post_open_to_file_for",
        needs: "the town's executive post is open, not held by the player, and fileable",
        holds: executive_post_open_to_file_for,
    },
    Requirement {
        name: "fresh_colleague_and_bill_before_council",
        needs: "a council colleague with no edge to the player, and a bill before the council",
        holds: fresh_colleague_and_bill_before_council,
    },
    Requirement {
        name: "introduction_ask_offered_early",
        needs: "an ask.introduce_me is offered within the first four turns (small world)",
        holds: introduction_ask_offered_early,
    },
    Requirement {
        name: "plant_in_trouble_listed_early",
        needs: "a small idle world where, within eight turns, the overview lists a plant on short time, under notice or shut, with what led to it",
        holds: plant_in_trouble_listed_early,
    },
    Requirement {
        name: "law_decided_early_with_its_chain",
        needs: "a small idle world where, within six turns, the overview lists a law passed or voted down and the town's paper a story, each with what led to it",
        holds: law_decided_early_with_its_chain,
    },
    Requirement {
        name: "law_lapses_in_first_skip",
        needs: "the first NextSession skip lists a lapsed law, with its policy named",
        holds: law_lapses_in_first_skip,
    },
    Requirement {
        name: "notebook_talk_reply_has_an_outcome",
        needs: "the first key figure answers topic.who_runs with a spoken reply",
        holds: notebook_talk_reply_has_an_outcome,
    },
    Requirement {
        name: "plant_closure_crisis_starts_clean",
        needs: "a town with an industry and no closure crisis at the start, one after a shock and none three years on",
        holds: plant_closure_crisis_starts_clean,
    },
    Requirement {
        name: "player_race_for_the_executive_within_30_turns",
        needs: "a medium world with a bill before the player's body at the start that the executive must sign, where filing for the executive office puts the player in its race within 30 turns",
        holds: player_race_for_the_executive_within_30_turns,
    },
    Requirement {
        name: "promises_reachable_by_skipping",
        needs: "a small world where the campaign promise picker opens within six rounds of skips",
        holds: promises_reachable_by_skipping,
    },
    Requirement {
        name: "regional_capital_medium",
        needs: "a medium world whose player's town has a regional assembly",
        holds: regional_capital_medium,
    },
    Requirement {
        name: "ask_town_hall_and_another_offered",
        needs: "a town-hall ask and one other are offered within the first four turns (small world)",
        holds: ask_town_hall_and_another_offered,
    },
    Requirement {
        name: "town_with_a_factory",
        needs: "the player's town has a factory (small world)",
        holds: town_with_a_factory,
    },
    Requirement {
        name: "voices_close_unequally",
        needs: "a chamber where the smooth and plain voices each close, and smooth closes more often",
        holds: voices_close_unequally,
    },
];

// ---- shared setup -------------------------------------------------------

/// The base pack, from the content folder.
pub fn content() -> Arc<ContentBundle> {
    static C: OnceLock<Arc<ContentBundle>> = OnceLock::new();
    C.get_or_init(|| {
        let src = mandate_content::bundle::read_dir_sources(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"),
        )
        .unwrap();
        Arc::new(mandate_content::load(&src).unwrap().0)
    })
    .clone()
}

fn world(seed: u64, size: WorldSize) -> Engine {
    generate(&WorldConfig::new(seed, size), content()).unwrap()
}

fn small(seed: u64) -> Engine {
    world(seed, WorldSize::Small)
}

fn town(e: &Engine) -> mandate_sim::world::MunicipalityId {
    e.state().player.as_ref().unwrap().municipality
}

fn end_turn(e: &mut Engine) -> bool {
    e.apply(Command::EndTurn).is_ok()
}

fn skip_next_session(e: &mut Engine) -> bool {
    e.apply(Command::Skip {
        target: SkipTargetKind::NextSession,
        limit: None,
    })
    .is_ok()
}

fn offered(e: &Engine, template: &str) -> bool {
    e.state()
        .asks
        .asks
        .iter()
        .any(|a| a.template.as_str() == template && a.status == AskStatus::Offered)
}

// ---- the predicates -----------------------------------------------------

/// The overview's recent events list a line with one of these text keys
/// whose "Why?" has at least two steps (the event and what led to it).
fn listed_with_a_chain(e: &Engine, text_keys: &[&str]) -> bool {
    mandate_protocol::overview(e)
        .recent_events
        .iter()
        .any(|ev| {
            text_keys.contains(&ev.text_key.as_str())
                && mandate_protocol::why_view(e, ev.id, None).is_some_and(|v| v.chain.len() >= 2)
        })
}

/// The town's paper carries a story that names an event with what led to it.
fn paper_names_an_event(e: &Engine) -> bool {
    mandate_protocol::newspaper_view(e, 24).is_some_and(|n| {
        n.articles
            .iter()
            .filter_map(|a| a.event)
            .any(|ev| mandate_protocol::why_view(e, ev, None).is_some_and(|v| v.chain.len() >= 2))
    })
}

fn law_decided_early_with_its_chain(seed: u64) -> bool {
    let mut e = small(seed);
    (0..6).any(|_| {
        end_turn(&mut e)
            && listed_with_a_chain(&e, &["event-policy-enacted", "event-policy-defeated"])
            && paper_names_an_event(&e)
    })
}

/// Since industries (DEC-110) a works closes by stages (short time, a dated
/// notice, then the gates), so an early game lists the first stages.
fn plant_in_trouble_listed_early(seed: u64) -> bool {
    let mut e = small(seed);
    (0..8).any(|_| {
        end_turn(&mut e)
            && listed_with_a_chain(
                &e,
                &[
                    "event-plant-short-time",
                    "event-plant-threatened",
                    "event-plant-closed",
                ],
            )
    })
}

fn law_lapses_in_first_skip(seed: u64) -> bool {
    let mut e = small(seed);
    if !(end_turn(&mut e) && skip_next_session(&mut e)) {
        return false;
    }
    skip_summary_view(&e).is_some_and(|s| {
        s.items.iter().any(|i| {
            i.text_key == "skip-item-law-expired"
                && i.args.get("policy_key").is_some_and(|k| !k.is_empty())
        })
    })
}

fn act_skip_spans_two_turns(seed: u64) -> bool {
    let mut e = small(seed);
    if !(end_turn(&mut e) && skip_next_session(&mut e)) {
        return false;
    }
    e.state()
        .act
        .last_skip
        .as_ref()
        .is_some_and(|s| s.turns >= 2)
}

fn councillor_and_regular_at_a_place(seed: u64) -> bool {
    let e = small(seed);
    let st = e.state();
    let council =
        mandate_sim::politics::body_members_at(&st.politics, Jurisdiction::town(town(&e)));
    let member = council
        .iter()
        .filter_map(|id| st.politics.get(*id))
        .any(|p| !p.is_player);
    member
        && st
            .world
            .places
            .iter()
            .filter(|p| p.municipality == town(&e))
            .any(|p| {
                characters::ambient::regulars_at(&st.politics, st, Location::Place { place: p.id })
                    .into_iter()
                    .any(|c| {
                        characters::profile(&st.politics, &st.world, c)
                            .is_some_and(|x| x.place == Some(p.id))
                    })
            })
}

fn plant_closure_crisis_starts_clean(seed: u64) -> bool {
    let mut e = small(seed);
    let m = town(&e).index();
    let Some(industry) = e.state().politics.characters.towns[m].industry.clone() else {
        return false;
    };
    let crisis = |e: &Engine, k: &str| {
        e.state().politics.characters.towns[m]
            .crises
            .iter()
            .any(|x| x.kind == k)
    };
    if crisis(&e, "crisis.plant_closure") {
        return false;
    }
    let shock = e.apply(Command::LabSectorShock {
        municipality: m as u32,
        sector: industry,
        factor: 0.25,
        months: 10,
    });
    if shock.is_err() || e.apply(Command::AdvanceDays { days: 240 }).is_err() {
        return false;
    }
    if !crisis(&e, "crisis.plant_closure") {
        return false;
    }
    e.apply(Command::AdvanceDays { days: 3 * 365 }).is_ok()
        && !crisis(&e, "crisis.plant_closure")
        && !crisis(&e, "crisis.disaster_recovery")
}

fn fresh_colleague_and_bill_before_council(seed: u64) -> bool {
    let e = small(seed);
    let st = e.state();
    let Some(pl) = st.player.as_ref() else {
        return false;
    };
    let Some(me) = characters::player_character(st) else {
        return false;
    };
    let at = Jurisdiction::town(pl.municipality);
    let colleague = mandate_sim::politics::body_members_at(&st.politics, at)
        .into_iter()
        .any(|id| {
            id != pl.politician
                && st
                    .politics
                    .characters
                    .edge(st.politics.get(id).unwrap().character, me)
                    .is_none()
        });
    colleague
        && st.policy.instances.iter().any(|p| {
            p.jurisdiction == pl.municipality.0
                && p.status == mandate_sim::policy::PolicyStatus::UnderConsideration
        })
}

fn executive_post_open_to_file_for(seed: u64) -> bool {
    let e = small(seed);
    let Some(cv) = campaign_view(&e) else {
        return false;
    };
    cv.nominations
        .iter()
        .find(|n| n.post.kind == PostKindDto::Office)
        .is_some_and(|n| n.open && !n.held && campaign::fileable(&e, &n.post).is_some())
}

fn town_with_a_factory(seed: u64) -> bool {
    place_in(&small(seed), PlaceKind::Factory)
}

fn regional_capital_medium(seed: u64) -> bool {
    place_in(&world(seed, WorldSize::Medium), PlaceKind::RegionalAssembly)
}

fn place_in(e: &Engine, kind: PlaceKind) -> bool {
    let st = e.state();
    let Some(pl) = st.player.as_ref() else {
        return false;
    };
    st.world.municipalities[pl.home().index()]
        .places
        .iter()
        .any(|p| st.world.places[p.index()].kind == kind)
}

/// Turns taken (at most four) until something is offered, as the browser
/// spec does.
fn turns_until_an_offer(e: &mut Engine) {
    for _ in 0..4 {
        if e.state()
            .asks
            .asks
            .iter()
            .any(|a| a.status == AskStatus::Offered)
        {
            break;
        }
        end_turn(e);
    }
}

fn ask_town_hall_and_another_offered(seed: u64) -> bool {
    let mut e = small(seed);
    turns_until_an_offer(&mut e);
    offered(&e, "ask.town_hall_here")
        && e.state()
            .asks
            .asks
            .iter()
            .any(|a| a.status == AskStatus::Offered && a.template.as_str() != "ask.town_hall_here")
}

fn introduction_ask_offered_early(seed: u64) -> bool {
    let mut e = small(seed);
    for _ in 0..4 {
        if offered(&e, "ask.introduce_me") {
            break;
        }
        end_turn(&mut e);
    }
    offered(&e, "ask.introduce_me")
}

fn promises_reachable_by_skipping(seed: u64) -> bool {
    let mut e = small(seed);
    let can_promise = |e: &Engine| pocket_view(e).is_some_and(|p| p.can_promise);
    for _ in 0..6 {
        if can_promise(&e) {
            return true;
        }
        let has_campaign = mandate_sim::skip::targets(e.state(), e.content())
            .iter()
            .any(|t| t.target == SkipTargetKind::Campaign);
        let moved = if has_campaign {
            e.apply(Command::Skip {
                target: SkipTargetKind::Campaign,
                limit: None,
            })
            .is_ok()
        } else {
            end_turn(&mut e)
        };
        if !moved {
            return false;
        }
    }
    can_promise(&e)
}

fn player_race_for_the_executive_within_30_turns(seed: u64) -> bool {
    let mut e = world(seed, WorldSize::Medium);
    // The chamber shows the signer's lean on a bill awaiting a vote.
    if !mandate_protocol::office_view(&e).is_some_and(|o| {
        o.bills
            .iter()
            .any(|b| b.signer.is_some() && b.next_vote.is_some())
    }) {
        return false;
    }
    let Some(cv) = campaign_view(&e) else {
        return false;
    };
    let Some(post) = cv
        .nominations
        .iter()
        .find(|n| n.post.kind == PostKindDto::Office)
        .and_then(|n| campaign::fileable(&e, &n.post))
    else {
        return false;
    };
    let cmd = e.route(PlayerCommand::File {
        filing: Filing::Stand { post },
    });
    if e.apply(cmd).is_err() {
        return false;
    }
    for _ in 0..30 {
        if campaign_view(&e).is_some_and(|c| {
            c.races
                .iter()
                .any(|r| r.is_player_race && r.post.kind == PostKindDto::Office)
        }) {
            return true;
        }
        if !end_turn(&mut e) {
            return false;
        }
    }
    false
}

/// Go where the engine says `person` is, unless they are here.
fn find(e: &mut Engine, person: CharacterId) {
    if !talk::can_talk(e.state(), e.content(), person)
        && let Some(l) = characters::routines::whereabouts(e.state(), e.content(), person)
    {
        let _ = e.apply(Command::Player(PlayerCommand::MoveTo { location: l }));
    }
}

fn say(e: &mut Engine, person: CharacterId, topic: &str) -> Option<talk::Exchange> {
    find(e, person);
    let r = e
        .apply(Command::Player(PlayerCommand::Talk {
            to: TalkTarget::Person { person },
            topic: topic.into(),
            about: None,
            late_night: false,
        }))
        .ok()?
        .result?;
    let id = talk::ExchangeId(r.created?);
    e.state().player.as_ref()?.talk.exchange(id).cloned()
}

fn notebook_talk_reply_has_an_outcome(seed: u64) -> bool {
    let mut e = small(seed);
    // The first key figure the Notebook lists: the town's executive, else
    // the first member of the council (the player left out).
    let st = e.state();
    let at = Jurisdiction::town(town(&e));
    let figure = mandate_sim::politics::office_holders_at(&st.politics, at)
        .into_iter()
        .chain(mandate_sim::politics::body_members_at(&st.politics, at))
        .filter_map(|p| st.politics.get(p))
        .find(|p| !p.is_player)
        .map(|p| p.character);
    let Some(figure) = figure else {
        return false;
    };
    say(&mut e, figure, "topic.who_runs").is_some_and(|x| !x.parts.is_empty())
}

fn set_traits(e: &mut Engine, who: CharacterId, traits: &[&str], boldness: f32, warmth: f32) {
    e.setup(|st| {
        let c = st.politics.characters.get_mut(who).unwrap();
        c.traits = traits.iter().map(|t| t.to_string()).collect();
        c.boldness = boldness;
        c.warmth = warmth;
    });
}

fn voices_close_unequally(seed: u64) -> bool {
    let mut e = small(seed);
    let rival = e.state().politics.characters.towns[town(&e).index()].rival;
    let at = Jurisdiction::town(town(&e));
    let people: Vec<CharacterId> = mandate_sim::politics::body_members_at(&e.state().politics, at)
        .into_iter()
        .filter_map(|p| e.state().politics.get(p))
        .filter(|p| !p.is_player)
        .map(|p| p.character)
        .filter(|c| Some(*c) != rival)
        .collect();
    if people.len() < 2 {
        return false;
    }
    let (smooth, flat) = (people[0], people[1]);
    find(&mut e, smooth);
    set_traits(&mut e, smooth, &["trait.charmer"], 0.3, 0.6);
    set_traits(&mut e, flat, &[], 0.3, 0.6);
    let mut closers: [Vec<String>; 2] = [Vec::new(), Vec::new()];
    for turn in 0..12 {
        for (i, s) in [smooth, flat].into_iter().enumerate() {
            let topic = ["topic.worries", "topic.who_runs", "topic.small_talk"][turn % 3];
            let Some(x) = say(&mut e, s, topic) else {
                return false;
            };
            closers[i].extend(
                x.parts
                    .iter()
                    .filter(|p| p.part == TalkPart::Closer)
                    .map(|p| p.line.clone()),
            );
        }
        if !end_turn(&mut e) {
            return false;
        }
    }
    !closers[0].is_empty()
        && !closers[1].is_empty()
        && closers[0]
            .iter()
            .all(|l| l.starts_with("line.close.smooth."))
        && closers[1]
            .iter()
            .all(|l| l.starts_with("line.close.plain."))
        && closers[0].len() > closers[1].len()
}
