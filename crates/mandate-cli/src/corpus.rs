//! `content doctor --corpus <dir>` (owner, 2026-11-20): every piece of
//! player-visible prose from simulated careers, as the player would see
//! it, with its situation, for a read-through audit ("NPCs never say
//! things that do not make sense"). Careers spread over scenarios,
//! storytellers and bot strategies; each item carries its seed, date,
//! speaker, place, topic and subject, template or line id, and a snapshot
//! of the truth at that moment (the worries, the crisis, the speaker's
//! convictions, whether a named law passed).
//!
//! The engine gives keys and arguments (DEC-054): the views the game shows
//! (`decision_view`, `article`, `story_view`, `exchange_view`,
//! `notebook_view`, the overheard draw, the action log), and, for
//! conversation coverage, every line that holds for every speaker in town
//! (passers-by met in a copy), with the arguments its conditions give. The
//! text is rendered by the game's own Fluent code (`web/src/dev/corpus/`,
//! the web `Translator`, run under Vitest). Writes `raw.jsonl` (keys and
//! arguments), then `corpus.jsonl`, `corpus.md` and `counts.json`.

use crate::bots;
use mandate_content::ContentBundle;
use mandate_content::conversation::TalkPart;
use mandate_protocol as proto;
use mandate_sim::characters::CharacterId;
use mandate_sim::explain;
use mandate_sim::places::Location;
use mandate_sim::{Command, Engine, State};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Arc;

/// Strategies the careers rotate through (the bots'; `explorer` talks).
const STRATEGIES: [&str; 6] = [
    "mixed",
    "explorer",
    "corrupt",
    "canvass",
    "media",
    "legislator",
];
/// (line id, voice) pairs a speaker could say.
type LineVoices = BTreeSet<(String, String)>;
/// Distinct argument fills kept per (id, part, voice).
const FILLS: usize = 6;
/// How often the notebook and the conversation sample are read (days).
const SAMPLE_DAYS: u32 = 61;

/// One piece of prose: how to render it and its situation.
#[derive(Clone, Debug)]
struct Item {
    /// `dilemma` | `scene` | `storyline` | `press` | `overheard` | `talk` |
    /// `notebook` | `outcome`.
    kind: &'static str,
    /// Within the kind: `title`, `situation`, `option`, `cost`, `gain`,
    /// `reason`, `aftermath`, `beat`, `stage`, `headline`, `line`, the
    /// reply's parts, `lead`, `learned`, `entry`, `gap`, `action`.
    part: String,
    /// The template or line id.
    id: String,
    /// How the renderer turns `view` into text (`event`, `title`, `beat`,
    /// `learned`, `entry`, `gap`).
    render: &'static str,
    /// The view the game renders (keys and arguments).
    view: Value,
    /// The voice a conversation line is said in, if any.
    voice: Option<String>,
    situation: Value,
}

impl Item {
    fn dedupe_key(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}",
            self.kind,
            self.part,
            self.id,
            self.voice.as_deref().unwrap_or(""),
            self.view
        )
    }
    fn slot_key(&self) -> String {
        format!(
            "{}|{}|{}|{}",
            self.kind,
            self.part,
            self.id,
            self.voice.as_deref().unwrap_or("")
        )
    }
}

/// A career's collection, with the argument-fill cap applied.
#[derive(Default)]
struct Bag {
    items: Vec<Item>,
    seen: BTreeSet<String>,
    fills: BTreeMap<String, usize>,
}

impl Bag {
    fn push(&mut self, it: Item) {
        let k = it.dedupe_key();
        if self.seen.contains(&k) {
            return;
        }
        let n = self.fills.entry(it.slot_key()).or_default();
        if *n >= FILLS {
            return;
        }
        *n += 1;
        self.seen.insert(k);
        self.items.push(it);
    }
}

/// The run a career belongs to.
struct Run {
    seed: u64,
    strategy: &'static str,
    storyteller: String,
    scenario: Option<String>,
}

fn date(st: &State) -> String {
    let c = st.calendar.date.calendar();
    format!("{:04}-{:02}-{:02}", c.year, c.month, c.day)
}

fn town(st: &State) -> Option<usize> {
    mandate_sim::player::home(st).map(|h| h.index())
}

fn name(st: &State, c: CharacterId) -> String {
    mandate_sim::characters::profile(&st.politics, &st.world, c)
        .map(|p| p.full_name())
        .unwrap_or_default()
}

/// A scope's `n` top worries by a reading (`concerns::ranked`), as ids.
fn worries(
    st: &State,
    content: &ContentBundle,
    scope: mandate_sim::concerns::Scope<'_>,
    by: mandate_content::condition::Worry,
    n: usize,
) -> Vec<String> {
    let ranking = mandate_sim::concerns::ranked(&st.population, content, &st.world, scope, by);
    mandate_sim::concerns::top(&ranking, n)
        .filter_map(|(i, _)| content.concerns.get(i).map(|c| c.id.clone()))
        .collect()
}

/// A neighbourhood's real top worries (felt, strongest first).
fn hood_worries(st: &State, content: &ContentBundle, hood: usize) -> Vec<String> {
    if st.world.neighborhoods.get(hood).is_none() {
        return Vec::new();
    }
    let scope = mandate_sim::concerns::Scope::Neighbourhood(mandate_sim::world::NeighborhoodId(
        hood as u32,
    ));
    worries(
        st,
        content,
        scope,
        mandate_content::condition::Worry::Felt,
        3,
    )
}

fn opinion_band(o: f32) -> &'static str {
    match o {
        x if x <= -40.0 => "hostile",
        x if x <= -10.0 => "cool",
        x if x < 10.0 => "neutral",
        x if x < 40.0 => "warm",
        _ => "friendly",
    }
}

/// Who someone is, as the situation records it: role (post word or
/// occupation), traits, opinion of the player, strongest convictions,
/// home neighbourhood and its worries.
fn person(st: &State, content: &ContentBundle, c: CharacterId) -> Value {
    let me = mandate_sim::characters::player_character(st);
    let p = mandate_sim::characters::profile(&st.politics, &st.world, c);
    let op = me.map(|m| mandate_sim::characters::opinion(&st.politics, c, m));
    let convictions: Vec<String> = p
        .as_ref()
        .map(|p| {
            let mut v: Vec<(usize, f32)> = p.values.iter().copied().enumerate().collect();
            v.sort_by(|a, b| b.1.abs().total_cmp(&a.1.abs()));
            v.into_iter()
                .take(2)
                .filter_map(|(i, x)| {
                    content
                        .attitudes
                        .get(i)
                        .map(|a| format!("{} {}", if x >= 0.0 { "pro" } else { "anti" }, a.key))
                })
                .collect()
        })
        .unwrap_or_default();
    let home = p.as_ref().map(|p| p.home as usize);
    json!({
        "name": name(st, c),
        "role": mandate_sim::characters::knows::role_key(st, content, c),
        "traits": st.politics.characters.get(c).map(|x| x.traits.clone()).unwrap_or_default(),
        "opinion": op.map(|o| (o as f64 * 10.0).round() / 10.0),
        "opinion_band": op.map(opinion_band),
        "convictions": convictions,
        "home": home.and_then(|h| st.world.neighborhoods.get(h)).map(|n| n.name.clone()),
        "home_worries": home.map(|h| hood_worries(st, content, h)).unwrap_or_default(),
    })
}

/// A place, its neighbourhood and that neighbourhood's worries.
fn place(st: &State, content: &ContentBundle, loc: Location) -> Value {
    let hood = mandate_sim::places::hood_of(st, loc);
    let label = match loc {
        Location::Place { place } => st
            .world
            .places
            .get(place.index())
            .map(|p| {
                if p.name.is_empty() {
                    p.kind.key().to_string()
                } else {
                    format!("{} ({})", p.name, p.kind.key())
                }
            })
            .unwrap_or_default(),
        Location::Street { .. } => "streets".into(),
        Location::Home => "home".into(),
    };
    json!({
        "place": label,
        "hood": hood.and_then(|h| st.world.neighborhoods.get(h.index())).map(|n| n.name.clone()),
        "hood_worries": hood.map(|h| hood_worries(st, content, h.index())).unwrap_or_default(),
    })
}

/// The situation every item carries: the run, the date, the town and its
/// truth (class, crisis, worries, broker, rival), the player's post.
fn situation(e: &Engine, run: &Run, extra: Value) -> Value {
    let st = e.state();
    let content = e.content();
    let m = town(st);
    let mu = m.and_then(|m| st.world.municipalities.get(m));
    let tc = m.and_then(|m| st.politics.characters.towns.get(m));
    let post = st.player.as_ref().and_then(|pl| {
        let p = st.politics.get(pl.politician)?;
        let c = content.constitutions.get(&st.meta.constitution)?;
        mandate_sim::politics::role_word(c, p).map(str::to_string)
    });
    let mut s = json!({
        "seed": run.seed,
        "date": date(st),
        "strategy": run.strategy,
        "storyteller": run.storyteller,
        "scenario": run.scenario,
        "town": mu.map(|x| x.name.clone()),
        "town_class": mu.map(|x| format!("{:?}", x.class).to_lowercase()),
        "crisis": tc.and_then(|t| t.crisis()).map(|c| format!("{} {:.2}", c.kind, c.severity)),
        // By level alone (the town's priorities).
        "town_worries": m
            .map(|m| {
                let town = mandate_sim::world::MunicipalityId(m as u32);
                let scope = mandate_sim::concerns::Scope::town(town);
                worries(st, content, scope, mandate_content::condition::Worry::Measured, 3)
            })
            .unwrap_or_default(),
        // As the notebook records them (felt across the town's neighbourhoods).
        "town_worries_noted": m
            .map(|m| {
                let town = mandate_sim::world::MunicipalityId(m as u32);
                let scope = mandate_sim::concerns::Scope::town(town);
                worries(st, content, scope, mandate_content::condition::Worry::Felt, 3)
            })
            .unwrap_or_default(),
        "broker": tc.and_then(|t| t.broker).map(|b| name(st, b)),
        "rival": tc.and_then(|t| t.rival).map(|r| name(st, r)),
        "player_post": post.unwrap_or_else(|| "out of office".into()),
    });
    if let (Value::Object(a), Value::Object(b)) = (&mut s, extra) {
        a.extend(b);
    }
    s
}

/// What a conversation is about, with the truth a line about it should fit:
/// the person, the bill's status and the town's support for it, the
/// neighbourhood's worries.
fn about_truth(st: &State, content: &ContentBundle, a: &mandate_sim::talk::AboutRef) -> Value {
    use mandate_sim::talk::AboutRef as A;
    let support = |template: &str| -> Option<f64> {
        let t = content.policies.get(template)?;
        let m = mandate_sim::player::home(st)?;
        let hoods: Vec<usize> = st.world.hoods_of(m).collect();
        let v = mandate_sim::policy::public_support(t, &st.population, &hoods);
        Some((v * 100.0).round() / 100.0)
    };
    match a {
        A::Person { person: p } => json!({ "person": person(st, content, *p) }),
        A::Rival => json!({
            "rival": mandate_sim::player::home(st)
                .and_then(|m| st.politics.characters.towns.get(m.index()))
                .and_then(|t| t.rival)
                .map(|r| person(st, content, r)),
        }),
        A::Bill { policy } => {
            let template = st
                .policy
                .instances
                .get(policy.index())
                .map(|p| p.template.clone());
            json!({
                "bill": policy_status(st, policy.0),
                "town_support": template.as_deref().and_then(support),
            })
        }
        A::Template { template } => {
            json!({ "template": template, "town_support": support(template) })
        }
        A::Hood { hood } => json!({
            "hood": st.world.neighborhoods.get(hood.index()).map(|n| n.name.clone()),
            "hood_worries": hood_worries(st, content, hood.index()),
        }),
        A::Place { place: p } => place(st, content, Location::Place { place: *p }),
        A::Proposal { .. } | A::Jurisdiction { .. } => serde_json::to_value(a).unwrap_or_default(),
    }
}

fn policy_status(st: &State, id: u32) -> Value {
    st.policy
        .instances
        .get(id as usize)
        .map(|p| {
            json!({
                "template": p.template,
                "status": serde_json::to_value(p.status).unwrap_or_default(),
            })
        })
        .unwrap_or(Value::Null)
}

/// The cast of a decision as the truth has it.
fn cast(st: &State, content: &ContentBundle, c: &[mandate_sim::narrative::CastEntry]) -> Value {
    use mandate_sim::narrative::Actor;
    Value::Array(
        c.iter()
            .map(|e| match e.actor {
                Actor::Person { id } => {
                    json!({ "role": e.role, "person": person(st, content, id) })
                }
                Actor::Neighborhood { id } => json!({
                    "role": e.role,
                    "hood": st.world.neighborhoods.get(id as usize).map(|n| n.name.clone()),
                    "hood_worries": hood_worries(st, content, id as usize),
                }),
                Actor::Policy { id } => json!({ "role": e.role, "policy": policy_status(st, id) }),
            })
            .collect(),
    )
}

fn event_item(
    kind: &'static str,
    part: &str,
    id: &str,
    key: &str,
    args: &BTreeMap<String, String>,
    sit: &Value,
) -> Item {
    Item {
        kind,
        part: part.into(),
        id: id.into(),
        render: "event",
        view: json!({ "key": key, "args": args }),
        voice: None,
        situation: sit.clone(),
    }
}

/// A decision's texts: on opening, its title, situation, options with their
/// trade-off lines, reasons and scene; once answered, its aftermath.
fn decision_items(
    e: &Engine,
    run: &Run,
    d: &mandate_sim::narrative::Decision,
    bag: &mut Bag,
    opened: bool,
) {
    let st = e.state();
    let content = e.content();
    let Some(v) = proto::decision_view(e, d) else {
        return;
    };
    let sit = situation(
        e,
        run,
        json!({
            "template": d.template,
            "cast": cast(st, content, &d.cast),
            "status": v.status,
            "place": d.place.map(|l| place(st, content, l)),
        }),
    );
    let id = d.template.as_str();
    if opened {
        bag.push(Item {
            render: "title",
            ..event_item("dilemma", "title", id, &v.title_key, &v.args, &sit)
        });
        bag.push(event_item(
            "dilemma",
            "situation",
            id,
            &v.situation_key,
            &v.args,
            &sit,
        ));
        for o in &v.options {
            bag.push(event_item(
                "dilemma",
                &format!("option {}", o.id),
                id,
                &o.label_key,
                &v.args,
                &sit,
            ));
            for l in &o.costs {
                bag.push(event_item("dilemma", "cost", id, &l.key, &l.args, &sit));
            }
            for l in &o.gains {
                bag.push(event_item("dilemma", "gain", id, &l.key, &l.args, &sit));
            }
        }
        for r in &v.reasons {
            bag.push(event_item("dilemma", "reason", id, &r.key, &r.args, &sit));
        }
        // The scene's beats exactly as the place shows them: the decision
        // seen by a player standing there (a copy; the career is untouched).
        if let (Some(at), Some(sc)) = (
            d.place,
            content
                .dilemma(id)
                .and_then(|x| x.scene.as_deref())
                .and_then(|s| content.scene(s)),
        ) {
            let mut there = st.clone();
            if let Some(pl) = there.player.as_mut() {
                pl.location = at;
            }
            if let Ok(e2) = Engine::from_state(there, e.content_arc())
                && let Some(v2) = proto::decision_view(&e2, d)
            {
                for (b, def) in v2.scene.beats.iter().zip(&sc.beats) {
                    let mut s = sit.clone();
                    if let (Some(w), Value::Object(o)) = (
                        mandate_sim::narrative::person_of(&d.cast, &def.speaker),
                        &mut s,
                    ) {
                        o.insert("speaker".into(), person(st, content, w));
                    }
                    bag.push(Item {
                        kind: "scene",
                        part: "beat".into(),
                        id: sc.id.clone(),
                        render: "beat",
                        view: json!({ "key": b.key, "args": v.args, "quote": b.quote }),
                        voice: None,
                        situation: s,
                    });
                }
            }
        }
    } else if let Some(a) = &v.aftermath {
        if !a.by_default {
            bag.push(event_item(
                "dilemma",
                "aftermath",
                id,
                &a.key,
                &v.args,
                &sit,
            ));
        }
        for l in &a.lines {
            bag.push(event_item(
                "outcome",
                "aftermath line",
                id,
                &l.key,
                &l.args,
                &sit,
            ));
        }
    }
}

/// Every line that holds for a speaker on every topic (each subject kind
/// the topic takes), with the arguments its conditions give.
fn talk_sample(
    e: &Engine,
    run: &Run,
    st: &State,
    bag: &mut Bag,
    lines_voices: &mut BTreeSet<(String, String)>,
) {
    let content = e.content();
    for s in explain::speakers(st, content) {
        let role = mandate_sim::characters::knows::role_key(st, content, s);
        let Some(ctx0) = explain::talk_ctx(st, content, s, None) else {
            continue;
        };
        let voice = mandate_sim::talk::reply::voice(&ctx0);
        let vid = voice.map(|v| v.id.clone()).unwrap_or_default();
        let who = person(st, content, s);
        for t in &content.talk_topics {
            let o = &t.offered;
            if !o.roles.is_empty() && !role.as_ref().is_some_and(|r| o.roles.contains(r)) {
                continue;
            }
            for about in explain::sample_abouts_for(st, content, s, &t.about, t.about_required) {
                let Some(ctx) = explain::talk_ctx(st, content, s, about.as_ref()) else {
                    continue;
                };
                if !ctx.all_hold(&o.when, &mut Default::default()) {
                    continue;
                }
                let base = mandate_sim::talk::context_args(&ctx);
                let mut sit: Option<Value> = None;
                for part in TalkPart::ALL {
                    let (cands, _) = mandate_sim::talk::reply::candidates(&ctx, t, part, voice);
                    for (l, a) in cands {
                        lines_voices.insert((l.id.clone(), vid.clone()));
                        let mut args = base.clone();
                        args.extend(a);
                        let sit = sit.get_or_insert_with(|| {
                            let mut x = situation(
                                e,
                                run,
                                json!({
                                    "speaker": who,
                                    "voice": vid,
                                    "topic": t.id,
                                    "about": about.as_ref().map(|a| about_truth(st, content, a)),
                                    "sampled": true,
                                }),
                            );
                            if let Value::Object(m) = &mut x {
                                m.extend(
                                    place(
                                        st,
                                        content,
                                        st.player.as_ref().map(|p| p.location).unwrap_or_default(),
                                    )
                                    .as_object()
                                    .cloned()
                                    .unwrap_or_default(),
                                );
                            }
                            x
                        });
                        bag.push(Item {
                            kind: "talk",
                            part: part.key().into(),
                            id: l.id.clone(),
                            render: "event",
                            view: json!({ "key": l.key, "args": args }),
                            voice: Some(vid.clone()),
                            situation: sit.clone(),
                        });
                    }
                }
            }
        }
    }
}

fn notebook_items(e: &Engine, run: &Run, bag: &mut Bag) {
    let Some(nb) = proto::notebook_view(e) else {
        return;
    };
    let sit = situation(e, run, json!({}));
    for sec in &nb.sections {
        for en in &sec.entries {
            let mut s = sit.clone();
            if let Value::Object(o) = &mut s {
                o.insert(
                    "subject".into(),
                    serde_json::to_value(&en.subject).unwrap_or_default(),
                );
                o.insert(
                    "level".into(),
                    serde_json::to_value(en.level).unwrap_or_default(),
                );
            }
            bag.push(Item {
                kind: "notebook",
                part: format!("entry {}", sec.key),
                id: en.title_key.clone(),
                render: "entry",
                view: serde_json::to_value(en).unwrap_or_default(),
                voice: None,
                situation: s.clone(),
            });
            for h in &en.history {
                bag.push(event_item(
                    "notebook", "source", &h.key, &h.key, &h.args, &s,
                ));
            }
        }
    }
    for g in &nb.gaps {
        let mut s = sit.clone();
        if let Value::Object(o) = &mut s {
            o.insert(
                "subject".into(),
                serde_json::to_value(&g.subject).unwrap_or_default(),
            );
        }
        bag.push(Item {
            kind: "notebook",
            part: "gap".into(),
            id: g.title_key.clone(),
            render: "gap",
            view: serde_json::to_value(g).unwrap_or_default(),
            voice: None,
            situation: s,
        });
    }
    for l in &nb.leads {
        bag.push(event_item(
            "notebook", "lead", &l.key, &l.key, &l.args, &sit,
        ));
    }
}

fn exchange_items(e: &Engine, run: &Run, x: &mandate_sim::talk::state::Exchange, bag: &mut Bag) {
    let st = e.state();
    let content = e.content();
    let Some(v) = proto::exchange_view(e, x.id.0) else {
        return;
    };
    let voice = explain::talk_ctx(st, content, x.speaker, x.about.as_ref())
        .and_then(|c| mandate_sim::talk::reply::voice(&c).map(|v| v.id.clone()))
        .unwrap_or_default();
    let loc = x
        .place
        .map(|p| Location::Place { place: p })
        .unwrap_or_default();
    let mut sit = situation(
        e,
        run,
        json!({
            "speaker": person(st, content, x.speaker),
            "voice": voice,
            "topic": x.topic,
            "about": x.about.as_ref().map(|a| about_truth(st, content, a)),
            "exchange": x.id.0,
        }),
    );
    if let Value::Object(m) = &mut sit {
        m.extend(
            place(st, content, loc)
                .as_object()
                .cloned()
                .unwrap_or_default(),
        );
    }
    for (p, sp) in v.parts.iter().zip(&x.parts) {
        bag.push(Item {
            kind: "talk",
            part: p.part.clone(),
            id: sp.line.clone(),
            render: "event",
            view: json!({ "key": p.key, "args": p.args }),
            voice: Some(voice.clone()),
            situation: sit.clone(),
        });
    }
    for l in &v.leads {
        bag.push(event_item("talk", "lead", &l.key, &l.key, &l.args, &sit));
    }
    for l in &v.learned {
        bag.push(Item {
            kind: "talk",
            part: "learned".into(),
            id: l.title_key.clone(),
            render: "learned",
            view: serde_json::to_value(l).unwrap_or_default(),
            voice: None,
            situation: sit.clone(),
        });
    }
    bag.push(event_item(
        "outcome",
        "talk",
        &v.outcome_key,
        &v.outcome_key,
        &v.outcome_args,
        &sit,
    ));
}

fn career(
    content: &Arc<ContentBundle>,
    run: Run,
    years: u32,
) -> Option<(Bag, BTreeSet<(String, String)>)> {
    let mut e = generate(
        &WorldConfig {
            seed: run.seed,
            size: WorldSize::Small,
            scenario: run.scenario.clone(),
        },
        content.clone(),
    )
    .ok()?;
    let profile = run.storyteller.clone();
    e.setup(|s| s.narrative.profile = profile);
    let mut bag = Bag::default();
    let mut lv: BTreeSet<(String, String)> = BTreeSet::new();
    let start = e.state().calendar.date;
    let end = start.add_days(years * 365);
    let start_turn = e.state().calendar.turn;
    let mut opened: BTreeSet<u32> = BTreeSet::new();
    let mut closed: BTreeSet<u32> = BTreeSet::new();
    let mut asks_opened: BTreeSet<u32> = BTreeSet::new();
    let mut asks_closed: BTreeSet<u32> = BTreeSet::new();
    let mut stories = 0u32;
    let mut actions = 0usize;
    let mut exchanges: BTreeSet<u32> = BTreeSet::new();
    let mut next_sample = start;
    while e.state().calendar.date < end {
        bots::play_turn_from(&mut e, content, run.strategy, start_turn);
        // What the turn's actions said (the action log), before time passes.
        {
            let st = e.state();
            if let Some(pl) = st.player.as_ref() {
                if pl.actions.len() < actions {
                    actions = 0;
                }
                for a in &pl.actions[actions..] {
                    let args: BTreeMap<String, String> = a.args.iter().cloned().collect();
                    let sit = situation(
                        &e,
                        &run,
                        json!({ "action": a.action, "where": place(st, content, pl.location) }),
                    );
                    bag.push(event_item(
                        "outcome", "action", &a.outcome, &a.outcome, &args, &sit,
                    ));
                }
                actions = pl.actions.len();
                for x in &pl.talk.log {
                    if exchanges.insert(x.id.0) {
                        exchange_items(&e, &run, x, &mut bag);
                    }
                }
            }
        }
        if e.apply(Command::EndTurn).is_err() {
            break;
        }
        if e.state()
            .player
            .as_ref()
            .is_some_and(|p| p.actions.len() < actions)
        {
            actions = 0;
        }
        let st = e.state();
        // Decisions opened and answered.
        for d in &st.narrative.decisions {
            if opened.insert(d.id.0) {
                decision_items(&e, &run, d, &mut bag, true);
            }
            if d.status != mandate_sim::narrative::DecisionStatus::Pending && closed.insert(d.id.0)
            {
                decision_items(&e, &run, d, &mut bag, false);
            }
        }
        // Asks and promises (DEC-097 §2): their texts when made, and how
        // they ended.
        for a in &st.asks.asks {
            let (name, detail, need, kept, broken) =
                match (content.ask(&a.template), content.promise(&a.template)) {
                    (Some(t), _) => (
                        &t.name_key,
                        &t.detail_key,
                        &t.need_key,
                        &t.kept_key,
                        &t.broken_key,
                    ),
                    (None, Some(t)) => (
                        &t.name_key,
                        &t.detail_key,
                        &t.need_key,
                        &t.kept_key,
                        &t.broken_key,
                    ),
                    _ => continue,
                };
            let args = proto::narrative::ask_args(&e, a);
            let sit = situation(
                &e,
                &run,
                json!({ "ask": a.template, "status": a.status.key(), "cast": cast(st, content, &a.cast) }),
            );
            if asks_opened.insert(a.id.0) {
                for (part, key) in [("title", name), ("detail", detail), ("need", need)] {
                    bag.push(event_item("ask", part, &a.template, key, &args, &sit));
                }
            }
            let end = match a.status {
                mandate_sim::asks::AskStatus::Kept { .. } => Some(("kept", kept)),
                mandate_sim::asks::AskStatus::Broken { .. } => Some(("broken", broken)),
                _ => None,
            };
            if let Some((part, key)) = end
                && asks_closed.insert(a.id.0)
            {
                bag.push(event_item("ask", part, &a.template, key, &args, &sit));
            }
        }
        // Storylines as the story so far tells them.
        let story = proto::story_view(&e);
        // `story_view` lists the newest twelve runs with a template, newest first.
        let shown: Vec<&mandate_sim::narrative::StorylineRun> = st
            .narrative
            .storylines
            .iter()
            .rev()
            .take(12)
            .filter(|r| content.storyline(&r.template).is_some())
            .collect();
        for (s, run_def) in story.storylines.iter().zip(shown) {
            let sit = situation(
                &e,
                &run,
                json!({ "storyline": run_def.template, "cast": cast(st, content, &run_def.cast) }),
            );
            bag.push(Item {
                render: "title",
                ..event_item(
                    "storyline",
                    "title",
                    &run_def.template,
                    &s.title_key,
                    &s.args,
                    &sit,
                )
            });
            for ev in &s.stages {
                bag.push(event_item(
                    "storyline",
                    "stage",
                    &run_def.template,
                    &ev.text_key,
                    &ev.args,
                    &sit,
                ));
            }
        }
        // The press in town and about the player.
        let me = st.player.as_ref().map(|p| p.politician);
        let m = town(st).map(|m| m as u32);
        for s in st.media.stories.iter().filter(|s| s.id >= stories) {
            if Some(s.town) != m && (s.subject.is_none() || s.subject != me) {
                continue;
            }
            if let Some(a) = proto::article(&e, s) {
                let sit = situation(
                    &e,
                    &run,
                    json!({
                        "source": serde_json::to_value(&s.source).unwrap_or_default(),
                        "subject": s.subject.and_then(|p| st.politics.get(p)).map(|p| person(st, content, p.character)),
                        "framing": a.framing,
                        "outlet": a.outlet.name_key,
                    }),
                );
                bag.push(event_item(
                    "press",
                    "headline",
                    &a.headline_key,
                    &a.headline_key,
                    &a.args,
                    &sit,
                ));
            }
        }
        stories = st
            .media
            .stories
            .iter()
            .map(|s| s.id + 1)
            .max()
            .unwrap_or(stories)
            .max(stories);
        // The week's talk at every place where people talk.
        for p in explain::talk_places(st, content) {
            let loc = Location::Place { place: p };
            let def_of = |id: &str| content.overheard.iter().find(|d| d.id == id);
            for l in mandate_sim::overheard::lines(st, content, loc) {
                let mut args = l.args.clone();
                if let Some(a) = l
                    .story
                    .and_then(|sid| st.media.stories.iter().find(|s| s.id == sid))
                    .and_then(|s| proto::article(&e, s))
                {
                    for (k, v) in a.args {
                        args.entry(k).or_insert(v);
                    }
                    args.insert("headline_key".into(), a.headline_key);
                    args.insert("outlet_key".into(), a.outlet.name_key);
                    args.insert("outlet_place".into(), a.outlet.place);
                }
                let mut sit = situation(
                    &e,
                    &run,
                    json!({ "line": l.id, "concern": def_of(&l.id).and_then(|d| d.concern.clone()) }),
                );
                if let Value::Object(mm) = &mut sit {
                    mm.extend(
                        place(st, content, loc)
                            .as_object()
                            .cloned()
                            .unwrap_or_default(),
                    );
                }
                bag.push(event_item("overheard", "line", &l.id, &l.key, &args, &sit));
            }
        }
        if st.calendar.date >= next_sample {
            notebook_items(&e, &run, &mut bag);
            let met = with_passers_by(&e, content);
            talk_sample(&e, &run, &met, &mut bag, &mut lv);
            next_sample = st.calendar.date.add_days(SAMPLE_DAYS);
        }
    }
    Some((bag, lv))
}

/// A copy of the state in which every passer-by in town has been met (they
/// only exist once met), so their lines are sampled too.
fn with_passers_by(e: &Engine, content: &ContentBundle) -> State {
    let mut st = e.state().clone();
    let Some(m) = town(&st) else { return st };
    let mut locs: Vec<Location> = st
        .world
        .neighborhoods
        .iter()
        .enumerate()
        .filter(|(_, h)| h.municipality.index() == m)
        .map(|(i, _)| Location::Street {
            neighborhood: mandate_sim::world::NeighborhoodId(i as u32),
        })
        .collect();
    locs.extend(
        st.world
            .places
            .iter()
            .filter(|p| p.municipality.index() == m)
            .map(|p| Location::Place { place: p.id }),
    );
    for loc in locs {
        let slots: Vec<u8> = mandate_sim::characters::ambient::ambient_at(&st, content, loc)
            .into_iter()
            .map(|(s, _)| s)
            .collect();
        for s in slots {
            mandate_sim::characters::ambient::promote(&mut st, content, loc, s);
        }
    }
    st
}

/// Collect, write `raw.jsonl`, render, and report the counts.
pub fn run(
    content: Arc<ContentBundle>,
    dir: &Path,
    seeds: u64,
    years: u32,
    source: &Path,
) -> Result<Value, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut scenarios: Vec<Option<String>> = vec![None];
    scenarios.extend(content.scenarios.iter().map(|s| Some(s.id.clone())));
    let storytellers: Vec<String> = content.storytellers.iter().map(|s| s.id.clone()).collect();
    let runs: Vec<Run> = (1..=seeds)
        .map(|seed| {
            let i = (seed - 1) as usize;
            Run {
                seed,
                strategy: STRATEGIES[i % STRATEGIES.len()],
                storyteller: storytellers
                    .get(i % storytellers.len().max(1))
                    .cloned()
                    .unwrap_or_else(|| content.director.default_profile.clone()),
                scenario: scenarios[i % scenarios.len()].clone(),
            }
        })
        .collect();
    let workers = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .clamp(1, 2);
    let mut results: Vec<(u64, Bag, LineVoices)> = Vec::new();
    let mut runs = runs.into_iter().peekable();
    while runs.peek().is_some() {
        let chunk: Vec<Run> = runs.by_ref().take(workers).collect();
        std::thread::scope(|s| {
            let hs: Vec<_> = chunk
                .into_iter()
                .map(|r| {
                    let c = content.clone();
                    s.spawn(move || {
                        let seed = r.seed;
                        (seed, career(&c, r, years))
                    })
                })
                .collect();
            for h in hs {
                if let Ok((seed, Some((b, lv)))) = h.join() {
                    results.push((seed, b, lv));
                }
            }
        });
    }
    results.sort_by_key(|r| r.0);
    // Merge: across careers, the same rendering once; at most FILLS fills.
    let mut all = Bag::default();
    let mut held: BTreeSet<(String, String)> = BTreeSet::new();
    for (_, b, lv) in results {
        held.extend(lv);
        for it in b.items {
            all.push(it);
        }
    }
    let raw = dir.join("raw.jsonl");
    let mut text = String::new();
    for it in &all.items {
        let line = json!({
            "kind": it.kind,
            "part": it.part,
            "id": it.id,
            "voice": it.voice,
            "render": it.render,
            "view": it.view,
            "situation": it.situation,
        });
        text.push_str(&line.to_string());
        text.push('\n');
    }
    std::fs::write(&raw, text).map_err(|e| format!("{}: {e}", raw.display()))?;
    // Coverage of line × voice: every pair a speaker could say (held), and
    // the lines that never held for anyone.
    let sampled: BTreeSet<(String, String)> = all
        .items
        .iter()
        .filter(|i| i.kind == "talk")
        .map(|i| (i.id.clone(), i.voice.clone().unwrap_or_default()))
        .collect();
    let never: Vec<&str> = content
        .talk_lines
        .iter()
        .filter(|l| !held.iter().any(|(id, _)| id == &l.id))
        .map(|l| l.id.as_str())
        .collect();
    let coverage = json!({
        "line_voice_pairs_reachable": held.len(),
        "line_voice_pairs_sampled": held.intersection(&sampled).count(),
        "lines": content.talk_lines.len(),
        "lines_never_held": never,
    });
    std::fs::write(
        dir.join("coverage.json"),
        serde_json::to_string_pretty(&coverage).unwrap_or_default(),
    )
    .map_err(|e| e.to_string())?;
    let rendered = render(dir, source)?;
    let mut counts: Map<String, Value> = Map::new();
    counts.insert("careers".into(), json!(seeds));
    counts.insert("years".into(), json!(years));
    counts.insert("raw_items".into(), json!(all.items.len()));
    counts.insert("coverage".into(), coverage);
    if let Value::Object(r) = rendered {
        counts.extend(r);
    }
    let counts = Value::Object(counts);
    std::fs::write(
        dir.join("counts.json"),
        serde_json::to_string_pretty(&counts).unwrap_or_default(),
    )
    .map_err(|e| e.to_string())?;
    Ok(counts)
}

/// Render `raw.jsonl` with the game's own Fluent code (Vitest runs
/// `web/src/dev/corpus/corpus.test.ts` with `MANDATE_CORPUS` set). Returns
/// the renderer's counts.
fn render(dir: &Path, source: &Path) -> Result<Value, String> {
    let web = source.join("web");
    if !web.join("node_modules").exists() {
        return Err(format!(
            "wrote {}/raw.jsonl; rendering needs {}/node_modules (npm ci), then rerun",
            dir.display(),
            web.display()
        ));
    }
    let abs = std::fs::canonicalize(dir).map_err(|e| e.to_string())?;
    let status = std::process::Command::new("npx")
        .args(["vitest", "run", "src/dev/corpus/corpus.test.ts"])
        .current_dir(&web)
        .env("MANDATE_CORPUS", &abs)
        .stdout(std::process::Stdio::null())
        .status()
        .map_err(|e| format!("running the renderer: {e}"))?;
    if !status.success() {
        return Err("the corpus renderer failed (see its output)".into());
    }
    let text =
        std::fs::read_to_string(abs.join("rendered-counts.json")).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}
