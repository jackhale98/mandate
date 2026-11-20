//! The base conversation lines in real towns (DEC-093, task C3): every line
//! the engine picks renders in English with every placeholder filled, across
//! towns, people, topics and relationships; and a sample of rendered
//! exchanges for two towns (`cargo test -p mandate-save --test talk_lines --
//! --nocapture` prints them).
//!
//! The English is rendered here by a small Fluent subset (placeables,
//! `x_key` arguments as message references, plural selects): enough to read
//! the base lines, not a Fluent implementation.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::print_stdout)]
use mandate_content::conversation::{AboutKind, TalkPart};
use mandate_content::{ContentBundle, bundle::read_dir_sources, load};
use mandate_sim::actions::PlayerCommand;
use mandate_sim::characters::knows::{self, Openness};
use mandate_sim::characters::{self, CharacterId, Relationship};
use mandate_sim::places::Location;
use mandate_sim::talk::context::Ctx;
use mandate_sim::talk::{self, AboutRef, TalkTarget, reply};
use mandate_sim::world::{NeighborhoodId, PlaceKind};
use mandate_sim::{Command, Engine};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Arc;

// --- A small Fluent subset -----------------------------------------------------------

struct Ftl(BTreeMap<String, String>);

impl Ftl {
    fn load() -> Ftl {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/base/locale/en");
        let mut files: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|x| x == "ftl"))
            .collect();
        files.sort();
        let mut out = BTreeMap::new();
        for f in files {
            let text = std::fs::read_to_string(f).unwrap();
            let mut cur: Option<String> = None;
            for line in text.lines() {
                let id_line = line.chars().next().is_some_and(|c| c.is_ascii_alphabetic());
                if id_line && let Some((id, rest)) = line.split_once('=') {
                    let id = id.trim().to_string();
                    out.insert(id.clone(), rest.trim_start().to_string());
                    cur = Some(id);
                } else if line.starts_with(' ') && !line.trim_start().starts_with('.') {
                    if let Some(id) = &cur {
                        let v: &mut String = out.get_mut(id).unwrap();
                        v.push('\n');
                        v.push_str(line);
                    }
                } else {
                    cur = None;
                }
            }
        }
        Ftl(out)
    }

    fn message(&self, key: &str, args: &BTreeMap<String, String>) -> String {
        match self.0.get(key) {
            Some(v) => self.render(v, args).trim().to_string(),
            None => format!("<missing message {key}>"),
        }
    }

    fn arg(&self, name: &str, args: &BTreeMap<String, String>) -> String {
        if let Some(v) = args.get(name) {
            // An empty name (an unnamed place) would leave a hole in the line.
            if v.is_empty() {
                return format!("<missing ${name} (empty)>");
            }
            return v.clone();
        }
        if let Some(k) = args.get(&format!("{name}_key")) {
            return self.message(k, args);
        }
        format!("<missing ${name}>")
    }

    fn render(&self, text: &str, args: &BTreeMap<String, String>) -> String {
        let mut out = String::new();
        let cs: Vec<char> = text.chars().collect();
        let mut i = 0;
        while i < cs.len() {
            if cs[i] != '{' {
                out.push(cs[i]);
                i += 1;
                continue;
            }
            let mut depth = 0;
            let mut j = i;
            while j < cs.len() {
                match cs[j] {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
                j += 1;
            }
            let inner: String = cs[i + 1..j].iter().collect();
            out.push_str(&self.placeable(&inner, args));
            i = j + 1;
        }
        out
    }

    fn placeable(&self, inner: &str, args: &BTreeMap<String, String>) -> String {
        if let Some((sel, variants)) = inner.split_once("->") {
            let v = self.arg(sel.trim().trim_start_matches('$'), args);
            let mut items: Vec<(bool, String, String)> = Vec::new();
            for l in variants.lines() {
                let t = l.trim();
                let (def, t) = match t.strip_prefix('*') {
                    Some(r) => (true, r),
                    None => (false, t),
                };
                if let Some(r) = t.strip_prefix('[')
                    && let Some((k, body)) = r.split_once(']')
                {
                    items.push((def, k.trim().to_string(), body.trim().to_string()));
                } else if let Some(last) = items.last_mut() {
                    last.2.push(' ');
                    last.2.push_str(t);
                }
            }
            let cat = match v.parse::<i64>() {
                Ok(1) => "one",
                Ok(_) => "other",
                Err(_) => "",
            };
            let pick = items
                .iter()
                .find(|x| x.1 == v)
                .or_else(|| items.iter().find(|x| x.1 == cat))
                .or_else(|| items.iter().find(|x| x.0))
                .map(|x| x.2.clone())
                .unwrap_or_default();
            return self.render(&pick, args);
        }
        let t = inner.trim();
        if let Some(name) = t.strip_prefix('$') {
            self.arg(name, args)
        } else if let Some(lit) = t.strip_prefix('"').and_then(|x| x.strip_suffix('"')) {
            lit.to_string()
        } else {
            self.message(t, args)
        }
    }
}

// --- Games ---------------------------------------------------------------------------

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

fn town(e: &Engine) -> mandate_sim::world::MunicipalityId {
    e.state().player.as_ref().unwrap().municipality
}

fn go(e: &mut Engine, loc: Location) {
    e.apply(Command::Player(PlayerCommand::MoveTo { location: loc }))
        .unwrap();
}

fn relate(e: &mut Engine, holder: CharacterId, opinion: f32, trust: f32) {
    let me = me(e);
    e.setup(|st| {
        st.politics.characters.edges.insert(
            (holder.0, me.0),
            Relationship {
                opinion,
                trust,
                bonds: Vec::new(),
            },
        );
    });
}

/// Every place in the player's town, and the streets of each neighbourhood.
fn spots(e: &Engine) -> Vec<Location> {
    let t = town(e);
    let st = e.state();
    let mut v: Vec<Location> = st
        .world
        .places
        .iter()
        .filter(|p| p.municipality == t)
        .map(|p| Location::Place { place: p.id })
        .collect();
    v.extend(st.world.hoods_of(t).map(|h| Location::Street {
        neighborhood: NeighborhoodId(h as u32),
    }));
    v
}

fn role(e: &Engine, s: CharacterId) -> String {
    knows::role_key(e.state(), e.content(), s).unwrap_or_else(|| "none".into())
}

fn standard_template(c: &ContentBundle) -> String {
    c.policies
        .values()
        .find(|t| t.tier == mandate_content::TierKey::Local && t.unlock.is_none())
        .unwrap()
        .id
        .clone()
}

/// The subjects to try for a topic with this speaker.
fn abouts(e: &Engine, c: &ContentBundle, s: CharacterId, kinds: &[AboutKind]) -> Vec<AboutRef> {
    let st = e.state();
    let t = town(e);
    let mut v = Vec::new();
    for k in kinds {
        match k {
            AboutKind::Person => {
                if let Some(p) = talk::present(st).into_iter().find(|p| *p != s) {
                    v.push(AboutRef::Person { person: p });
                }
            }
            AboutKind::Rival => {
                if st.politics.characters.towns[t.index()].rival.is_some() {
                    v.push(AboutRef::Rival);
                }
            }
            AboutKind::Hood => {
                if let Some(p) = characters::profile(&st.politics, &st.world, s) {
                    v.push(AboutRef::Hood {
                        hood: NeighborhoodId(p.home),
                    });
                }
                if let Some(h) = st.world.hoods_of(t).next() {
                    v.push(AboutRef::Hood {
                        hood: NeighborhoodId(h as u32),
                    });
                }
            }
            AboutKind::Place => {
                if let Some(p) = st.world.places.iter().find(|p| p.municipality == t) {
                    v.push(AboutRef::Place { place: p.id });
                }
            }
            AboutKind::Template => v.push(AboutRef::Template {
                template: standard_template(c),
            }),
            AboutKind::Bill => {
                if let Some(p) = st.policy.instances.first() {
                    v.push(AboutRef::Bill { policy: p.id });
                }
            }
            AboutKind::Proposal => {}
        }
    }
    v
}

/// Render one composed reply.
fn render_parts(ftl: &Ftl, parts: &[(String, BTreeMap<String, String>)]) -> String {
    parts
        .iter()
        .map(|(k, a)| ftl.message(k, a))
        .collect::<Vec<_>>()
        .join(" ")
}

/// How many of the world's towns pass each indicator threshold the lines
/// use (a calibration report for line conditions, not a gate).
fn report_indicators(e: &Engine, c: &ContentBundle, seed: u64) {
    use mandate_content::{Indicator, SituationCondition};
    let st = e.state();
    let n = st.world.municipalities.len() as u32;
    let mut out = Vec::new();
    for (ind, thresholds) in [
        (Indicator::JobLoss, &[0.01, 0.02, 0.03][..]),
        (Indicator::Unemployment, &[0.05, 0.08, 0.12][..]),
        (Indicator::RentBurden, &[0.15, 0.22, 0.3][..]),
        (Indicator::Pollution, &[0.2, 0.4, 0.5, 0.8][..]),
        (Indicator::TaxBurden, &[0.05, 0.06, 0.065, 0.07, 0.08][..]),
        (Indicator::BudgetStress, &[0.2, 0.3, 0.4][..]),
        (Indicator::RecentDisaster, &[0.3, 0.5][..]),
    ] {
        let counts: Vec<String> = thresholds
            .iter()
            .map(|t| {
                let cond = SituationCondition::Indicator {
                    indicator: ind,
                    min: Some(*t),
                    max: None,
                };
                let k = (0..n)
                    .filter(|m| mandate_sim::policy::situation_holds(st, c, &cond, *m))
                    .count();
                format!("≥{t}: {k}")
            })
            .collect();
        out.push(format!("{ind:?} [{}]", counts.join(", ")));
    }
    println!("seed {seed}, {n} towns: {}", out.join("; "));
}

/// Every line the engine picks renders, in several towns, for everyone the
/// player can reach, on every topic they offer, at every relationship band.
/// Prints the lines never picked (a coverage report, not a gate).
#[test]
fn every_picked_line_renders() {
    let c = content();
    let ftl = Ftl::load();
    let mut picked: BTreeSet<String> = BTreeSet::new();
    let mut fell_back: BTreeMap<String, (u32, u32)> = BTreeMap::new();
    let mut exchanges = 0u32;
    let mut distinct: BTreeSet<String> = BTreeSet::new();
    for seed in [42u64, 9001, 7, 2026] {
        let mut e = game(seed, &c);
        // Let the town move a little first.
        for _ in 0..3 {
            e.apply(Command::EndTurn).unwrap();
        }
        let me = me(&e);
        report_indicators(&e, &c, seed);
        // A bill before the council, to talk and remember things about.
        e.apply(e.route(PlayerCommand::DraftPolicy {
            template: standard_template(&c),
            params: Default::default(),
        }))
        .unwrap();
        let bill = e.state().policy.instances.last().map(|p| p.id.0);
        let mut nth = 0usize;
        for loc in spots(&e) {
            go(&mut e, loc);
            // Passers-by too (promoted, as talking to them does).
            let slots: Vec<u8> = characters::ambient::ambient_at(e.state(), &c, loc)
                .into_iter()
                .map(|(s, _)| s)
                .collect();
            for slot in slots {
                e.setup(|st| {
                    characters::ambient::promote(st, &c, loc, slot);
                });
            }
            let people = talk::present(e.state());
            for s in people {
                let r = role(&e, s);
                // Each person remembers something of the player, and some
                // trade favours either way (so quotes and favour lines hold).
                let kind = characters::MemoryKind::ALL[nth % characters::MemoryKind::ALL.len()];
                let favour = nth % 3;
                nth += 1;
                e.setup(|st| {
                    let date = st.calendar.date;
                    let subject = bill.map(|id| characters::Subject::Policy { id });
                    characters::remember(&mut st.politics, &c, date, s, me, kind, subject, None);
                    if favour > 0 {
                        let (debtor, creditor) = if favour == 1 { (me, s) } else { (s, me) };
                        st.politics.characters.favours.push(characters::Favour {
                            debtor,
                            creditor,
                            count: 1,
                            context: "favour-bill".into(),
                            subject,
                            since: date,
                        });
                    }
                });
                for (op, trust) in [
                    (-60.0, 0.3),
                    (-25.0, 0.5),
                    (0.0, 0.5),
                    (30.0, 0.5),
                    (70.0, 0.8),
                ] {
                    relate(&mut e, s, op, trust);
                    for topic in &c.talk_topics {
                        if !topic.offered.roles.is_empty() && !topic.offered.roles.contains(&r) {
                            continue;
                        }
                        let mut subjects: Vec<Option<AboutRef>> = abouts(&e, &c, s, &topic.about)
                            .into_iter()
                            .map(Some)
                            .collect();
                        if !topic.about_required {
                            subjects.push(None);
                        }
                        for about in subjects {
                            let st = e.state();
                            let pl = st.player.as_ref().unwrap();
                            let cap = knows::discretion_cap(st, &c, s, me, Openness::default());
                            let ctx = Ctx {
                                st,
                                content: &c,
                                town: pl.municipality,
                                me,
                                speaker: s,
                                about: about.as_ref(),
                                here: pl.location,
                                cap,
                            };
                            for n in 0..3 {
                                let x = reply::compose(&ctx, topic, &[], 5, n);
                                // The context arguments, as the `Talk` handler
                                // gives them (unnamed places by their kind).
                                let base = talk::context_args(&ctx);
                                let parts: Vec<(String, BTreeMap<String, String>)> = x
                                    .parts
                                    .iter()
                                    .map(|p| {
                                        let mut a = base.clone();
                                        a.extend(p.args.clone());
                                        (p.line.key.clone(), a)
                                    })
                                    .collect();
                                let text = render_parts(&ftl, &parts);
                                assert!(
                                    !text.contains("<missing"),
                                    "seed {seed}, {r}, {}: {text}\n{:?}",
                                    topic.id,
                                    x.parts.iter().map(|p| &p.line.id).collect::<Vec<_>>()
                                );
                                assert!(!text.contains("  "), "double space: {text}");
                                assert!(
                                    x.parts.iter().any(|p| p.part == TalkPart::Substance),
                                    "{r} {}: a substance",
                                    topic.id
                                );
                                for p in &x.parts {
                                    picked.insert(p.line.id.clone());
                                }
                                let f = fell_back.entry(format!("{} {r}", topic.id)).or_default();
                                f.1 += 1;
                                if x.fell_back {
                                    f.0 += 1;
                                }
                                exchanges += 1;
                                distinct.insert(text);
                            }
                        }
                    }
                }
            }
        }
    }
    let never: Vec<&String> = c
        .talk_lines
        .iter()
        .map(|l| &l.id)
        .filter(|l| !picked.contains(*l))
        .collect();
    println!(
        "{exchanges} replies, {} distinct; {} of {} lines picked",
        distinct.len(),
        picked.len(),
        c.talk_lines.len()
    );
    println!("never picked here: {never:?}");
    // A spread of what was said, to read.
    let step = (distinct.len() / 40).max(1);
    for t in distinct.iter().step_by(step) {
        println!("  | {t}");
    }
    for (k, (f, n)) in &fell_back {
        println!("fallback share {k}: {f}/{n}");
    }
    assert!(
        picked.len() * 2 > c.talk_lines.len(),
        "most lines are reachable"
    );
}

// --- Samples -------------------------------------------------------------------------

/// Talk for real (the `Talk` command), and render what was said.
fn talk_render(
    e: &mut Engine,
    ftl: &Ftl,
    s: CharacterId,
    topic: &str,
    about: Option<AboutRef>,
) -> String {
    e.setup(|st| st.player.as_mut().unwrap().slots_used = 0);
    let r = e
        .apply(Command::Player(PlayerCommand::Talk {
            to: TalkTarget::Person { person: s },
            topic: topic.into(),
            about,
            late_night: false,
        }))
        .unwrap_or_else(|x| panic!("{topic}: {x}"))
        .result
        .unwrap();
    let id = talk::ExchangeId(r.created.unwrap());
    let x = e
        .state()
        .player
        .as_ref()
        .unwrap()
        .talk
        .exchange(id)
        .unwrap()
        .clone();
    let parts: Vec<(String, BTreeMap<String, String>)> = x
        .parts
        .iter()
        .map(|p| (p.key.clone(), p.args.iter().cloned().collect()))
        .collect();
    let text = render_parts(ftl, &parts);
    assert!(!text.contains("<missing"), "{text}");
    let ids: Vec<&str> = x.parts.iter().map(|p| p.line.as_str()).collect();
    let outcome = ftl.message(&r.outcome_key, &r.args.clone().into_iter().collect());
    format!("{text}\n      [{}] -> {outcome}", ids.join(", "))
}

fn find_named(e: &Engine, occupation: &str) -> Option<(CharacterId, Location)> {
    let t = town(e);
    let st = e.state();
    st.politics.characters.people.iter().find_map(|c| {
        let n = c.named.as_deref()?;
        if n.occupation != occupation {
            return None;
        }
        let p = st.world.places.get(n.place?.index())?;
        (p.municipality == t).then_some((c.id, Location::Place { place: p.id }))
    })
}

fn describe(e: &Engine, c: &ContentBundle, s: CharacterId) -> String {
    let st = e.state();
    let p = characters::profile(&st.politics, &st.world, s).unwrap();
    let pl = st.player.as_ref().unwrap();
    let ctx = Ctx {
        st,
        content: c,
        town: pl.municipality,
        me: me(e),
        speaker: s,
        about: None,
        here: pl.location,
        cap: mandate_sim::player::knowledge::Level::Unknown,
    };
    let v = reply::voice(&ctx).map(|v| v.id.clone()).unwrap_or_default();
    format!(
        "{} ({}, {v}; warmth {:.2}, boldness {:.2}, traits {:?}; opinion {:.0}, trust {:.2})",
        p.full_name(),
        role(e, s),
        p.personality.warmth,
        p.personality.boldness,
        p.traits,
        characters::opinion(&st.politics, s, me(e)),
        characters::trust(&st.politics, c, s, me(e)),
    )
}

/// Three people in each of two towns, two or three topics each, rendered.
#[test]
fn sample_exchanges() {
    let c = content();
    let ftl = Ftl::load();
    for seed in [42u64, 9001] {
        let mut e = game(seed, &c);
        for _ in 0..3 {
            e.apply(Command::EndTurn).unwrap();
        }
        let tname = e.state().world.municipalities[town(&e).index()]
            .name
            .clone();
        println!("\n=== Seed {seed}: {tname} ===");
        // A cool union steward.
        if let Some((s, at)) = find_named(&e, "occupation.union_steward") {
            go(&mut e, at);
            relate(&mut e, s, -30.0, 0.4);
            println!("\n-- {}", describe(&e, &c, s));
            for (t, a) in [
                ("topic.worries", None),
                ("topic.who_runs", None),
                ("topic.small_talk", None),
            ] {
                println!("  [{t}] {}", talk_render(&mut e, &ftl, s, t, a));
            }
        }
        // A warm shopkeeper (trusting).
        if let Some((s, at)) = find_named(&e, "occupation.shopkeeper") {
            go(&mut e, at);
            relate(&mut e, s, 62.0, 0.7);
            println!("\n-- {}", describe(&e, &c, s));
            let home = characters::profile(&e.state().politics, &e.state().world, s)
                .unwrap()
                .home;
            for (t, a) in [
                ("topic.worries", None),
                ("topic.who_runs", None),
                (
                    "topic.about",
                    Some(AboutRef::Hood {
                        hood: NeighborhoodId(home),
                    }),
                ),
            ] {
                println!("  [{t}] {}", talk_render(&mut e, &ftl, s, t, a));
            }
        }
        // A guarded councillor (cautious): friendly, but one level more discreet.
        let chamber = e
            .state()
            .world
            .places
            .iter()
            .find(|p| p.municipality == town(&e) && p.kind == PlaceKind::CouncilChamber)
            .map(|p| Location::Place { place: p.id })
            .unwrap();
        go(&mut e, chamber);
        let rival = e.state().politics.characters.towns[town(&e).index()].rival;
        let s = talk::present(e.state())
            .into_iter()
            .find(|p| Some(*p) != rival && role(&e, *p) == "councillor")
            .unwrap();
        e.setup(|st| {
            let ch = st.politics.characters.get_mut(s).unwrap();
            ch.traits = vec!["trait.cautious".into()];
        });
        relate(&mut e, s, 55.0, 0.6);
        println!("\n-- {}", describe(&e, &c, s));
        let template = standard_template(&c);
        for (t, a) in [
            ("topic.about", Some(AboutRef::Rival)),
            ("topic.who_runs", None),
            ("topic.pitch", Some(AboutRef::Template { template })),
        ] {
            println!("  [{t}] {}", talk_render(&mut e, &ftl, s, t, a));
        }
    }
}

// --- Follow-ups: voices, unnamed places, concerns -------------------------------------

fn ctx_for<'a>(
    e: &'a Engine,
    c: &'a ContentBundle,
    s: CharacterId,
    about: Option<&'a AboutRef>,
) -> Ctx<'a> {
    let st = e.state();
    let pl = st.player.as_ref().unwrap();
    let me = me(e);
    Ctx {
        st,
        content: c,
        town: pl.municipality,
        me,
        speaker: s,
        about,
        here: pl.location,
        cap: knows::discretion_cap(st, c, s, me, Openness::default()),
    }
}

/// The first spot in town with someone to talk to, and who.
fn someone(e: &mut Engine) -> CharacterId {
    for loc in spots(e) {
        go(e, loc);
        if let Some(s) = talk::present(e.state()).first() {
            return *s;
        }
    }
    panic!("nobody to talk to");
}

/// Openers follow the chosen voice: a cautious charmer speaks smoothly
/// (the higher-priority voice), so no guarded opener goes with a smooth
/// closer, whichever lines are drawn.
#[test]
fn openers_follow_the_chosen_voice() {
    use mandate_content::SituationCondition as U;
    let c = content();
    let ftl = Ftl::load();
    let mut e = game(42, &c);
    let s = someone(&mut e);
    e.setup(|st| {
        st.politics.characters.get_mut(s).unwrap().traits =
            vec!["trait.charmer".into(), "trait.cautious".into()];
    });
    relate(&mut e, s, 0.0, 0.5);
    let topic = c.talk_topic("topic.small_talk").unwrap();
    let ctx = ctx_for(&e, &c, s, None);
    assert_eq!(
        reply::voice(&ctx).map(|v| v.id.as_str()),
        Some("voice.smooth")
    );
    let mut smooth = 0;
    for week in 0..30 {
        for n in 0..3 {
            let x = reply::compose(&ctx, topic, &[], week, n);
            for p in &x.parts {
                assert!(!p.line.id.starts_with("line.open.guarded"), "{}", p.line.id);
                if p.part == TalkPart::Closer {
                    assert_eq!(p.line.voice.as_deref(), Some("voice.smooth"));
                }
                if p.line.id.starts_with("line.open.smooth") {
                    smooth += 1;
                }
                // Every line that names a voice names the speaker's.
                for w in &p.line.when {
                    if let U::Voice { voices } = w {
                        assert!(voices.iter().any(|v| v == "voice.smooth"));
                    }
                }
            }
            let base = talk::context_args(&ctx);
            let parts: Vec<_> = x
                .parts
                .iter()
                .map(|p| {
                    let mut a = base.clone();
                    a.extend(p.args.clone());
                    (p.line.key.clone(), a)
                })
                .collect();
            assert!(!render_parts(&ftl, &parts).contains("<missing"));
        }
    }
    assert!(smooth > 0, "smooth openers are drawn");
}

/// A place without a proper name is named by its kind: `$here` (and
/// `$about`) come as the Fluent key `talk-place-<kind>` ("the union hall"),
/// never as an empty name.
#[test]
fn unnamed_places_are_named_by_their_kind() {
    let c = content();
    let ftl = Ftl::load();
    let mut e = game(42, &c);
    let t = town(&e);
    let s = someone(&mut e);
    let (place, kind) = e
        .state()
        .world
        .places
        .iter()
        .find(|p| p.municipality == t && p.name.is_empty())
        .map(|p| (p.id, p.kind))
        .expect("an unnamed place");
    go(&mut e, Location::Place { place });
    let about = AboutRef::Place { place };
    let ctx = ctx_for(&e, &c, s, Some(&about));
    let a = talk::context_args(&ctx);
    let key = talk::context::place_talk_key(kind);
    assert!(!a.contains_key("here"), "{a:?}");
    assert_eq!(a.get("here_key"), Some(&key));
    assert_eq!(a.get("about_key"), Some(&key));
    let text = ftl.message("talk-open-guarded-here", &a);
    assert!(text.contains(" round the "), "{text}");
    assert!(!text.contains("<missing"), "{text}");
    // A named place keeps its name.
    let (named, name) = e
        .state()
        .world
        .places
        .iter()
        .find(|p| p.municipality == t && !p.name.is_empty())
        .map(|p| (p.id, p.name.clone()))
        .expect("a named place");
    go(&mut e, Location::Place { place: named });
    let ctx = ctx_for(&e, &c, s, None);
    let a = talk::context_args(&ctx);
    assert_eq!(a.get("here"), Some(&name));
    assert!(!a.contains_key("here_key"));
    // The broker's place, likewise: a name, or a kind's key.
    if let Some(b) = e.state().politics.characters.towns[t.index()].broker {
        let ctx = ctx_for(&e, &c, b, None);
        let mut args = BTreeMap::new();
        let holds = ctx.holds(
            &mandate_content::SituationCondition::IsBroker {
                slot: "speaker".into(),
                is: true,
            },
            &mut args,
        );
        assert!(holds);
        let named = args.get("broker_place").is_some_and(|x| !x.is_empty());
        let keyed = args
            .get("broker_place_key")
            .is_some_and(|k| k.starts_with("talk-place-"));
        assert!(named != keyed, "{args:?}");
        let text = ftl.message("talk-runs-named-where", &args);
        assert!(!text.contains("<missing"), "{text}");
    }
}

/// The `concern` condition: the speaker's street's or the town's top
/// concern, with its name and its noun phrase of talk.
#[test]
fn the_concern_condition_reads_the_top_concern() {
    use mandate_content::SituationCondition as U;
    use mandate_content::conversation::ConcernOf;
    let c = content();
    let ftl = Ftl::load();
    let mut held = 0;
    for seed in [42u64, 9001, 7, 2026] {
        let mut e = game(seed, &c);
        for _ in 0..3 {
            e.apply(Command::EndTurn).unwrap();
        }
        let t = town(&e);
        let top = mandate_sim::concerns::top_concerns(&e.state().population, t.index());
        let Some(&(i, level)) = top.first() else {
            continue;
        };
        let s = someone(&mut e);
        let ctx = ctx_for(&e, &c, s, None);
        let all: Vec<String> = c.concerns.iter().map(|x| x.id.clone()).collect();
        let others: Vec<String> = all
            .iter()
            .filter(|x| **x != c.concerns[i].id)
            .cloned()
            .collect();
        let town_is = |concerns: Vec<String>| U::Concern {
            of: ConcernOf::Town,
            concerns,
        };
        let mut args = BTreeMap::new();
        let holds = ctx.holds(&town_is(all.clone()), &mut args);
        assert_eq!(holds, level > 0.0, "seed {seed}");
        assert_eq!(
            talk::context::holds_in_town(e.state(), &c, &town_is(all.clone()), t.0),
            holds,
            "the town form reads the same in a town's situation"
        );
        if holds {
            held += 1;
            assert_eq!(args["concern_key"], c.concerns[i].name_key);
            assert_eq!(
                Some(&args["concern_talk_key"]),
                c.concerns[i].talk_key.as_ref()
            );
            let mut a = talk::context_args(&ctx);
            a.extend(args.clone());
            let text = ftl.message("talk-worries-town-talk", &a);
            assert!(!text.contains("<missing"), "{text}");
            assert!(!ctx.holds(&town_is(others), &mut BTreeMap::new()));
        }
        // The speaker's form holds for at most one concern.
        let speaker = all
            .iter()
            .filter(|k| {
                ctx.holds(
                    &U::Concern {
                        of: ConcernOf::Speaker,
                        concerns: vec![(*k).clone()],
                    },
                    &mut BTreeMap::new(),
                )
            })
            .count();
        assert!(speaker <= 1);
    }
    assert!(held > 0, "some town has a top concern");
}
