//! Dynamic checks: what bot-played careers fire, and what never or too often.

use super::*;

/// What one career saw.
#[derive(Clone, Debug, Default)]
pub(super) struct Seen {
    /// Item key (`kind:id`) → picks.
    picks: BTreeMap<String, u64>,
    /// Lines whose conditions held for some speaker (not necessarily picked).
    held: BTreeSet<String>,
    place_kinds: BTreeSet<String>,
    roles: BTreeSet<String>,
    role_voice: BTreeSet<(String, String)>,
}

impl Seen {
    fn pick(&mut self, kind: &str, id: &str) {
        *self.picks.entry(format!("{kind}:{id}")).or_default() += 1;
    }
}

/// How often the talk sample runs (days).
pub(super) const TALK_SAMPLE_DAYS: u32 = 91;

/// A copy of the state in which everyone passing by in town has been met:
/// passers-by (residents, retirees, clergy, farmers…) are speakers too, and
/// only exist once met. The career's own engine is untouched.
pub(super) fn with_passers_by(e: &Engine, content: &ContentBundle) -> mandate_sim::State {
    use mandate_sim::places::Location;
    let mut st = e.state().clone();
    let Some(m) = mandate_sim::player::home(&st).map(|h| h.0) else {
        return st;
    };
    let mut locs: Vec<Location> = st
        .world
        .neighborhoods
        .iter()
        .enumerate()
        .filter(|(_, h)| h.municipality.0 == m)
        .map(|(i, _)| Location::Street {
            neighborhood: mandate_sim::world::NeighborhoodId(i as u32),
        })
        .collect();
    locs.extend(
        st.world
            .places
            .iter()
            .filter(|p| p.municipality.0 == m)
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

pub(super) fn sample_talk(e: &Engine, content: &ContentBundle, seen: &mut Seen) {
    let met = with_passers_by(e, content);
    let st = &met;
    let week = mandate_sim::talk::week_of(st.calendar.date);
    for s in explain::speakers(st, content) {
        let role = mandate_sim::characters::knows::role_key(st, content, s);
        if let Some(r) = &role {
            seen.roles.insert(r.clone());
        }
        let Some(ctx) = explain::talk_ctx(st, content, s, None) else {
            continue;
        };
        let voice = mandate_sim::talk::reply::voice(&ctx);
        if let Some(v) = voice {
            seen.pick("voice", &v.id);
            if let Some(r) = &role {
                seen.role_voice.insert((r.clone(), v.id.clone()));
            }
        }
        for t in &content.talk_topics {
            let o = &t.offered;
            if !o.roles.is_empty() && !role.as_ref().is_some_and(|r| o.roles.contains(r)) {
                continue;
            }
            // Without a subject (when it needs none), then with one of each
            // kind it takes.
            let mut asked = false;
            for about in explain::sample_abouts_for(st, content, s, &t.about, t.about_required) {
                let Some(ctx) = explain::talk_ctx(st, content, s, about.as_ref()) else {
                    continue;
                };
                if !ctx.all_hold(&o.when, &mut Default::default()) {
                    continue;
                }
                asked = true;
                for part in TalkPart::ALL {
                    let (cands, _) = mandate_sim::talk::reply::candidates(&ctx, t, part, voice);
                    for (l, _) in cands {
                        seen.held.insert(l.id.clone());
                    }
                }
                let reply = mandate_sim::talk::reply::compose(&ctx, t, &[], week, 0);
                for c in reply.parts {
                    seen.pick("line", &c.line.id);
                }
            }
            if asked {
                seen.pick("topic", &t.id);
            }
        }
    }
}

pub(super) fn career(
    content: &Arc<ContentBundle>,
    seed: u64,
    years: u32,
    strategy: &str,
) -> Option<Seen> {
    let mut e = generate(&WorldConfig::new(seed, WorldSize::Small), content.clone()).ok()?;
    let mut seen = Seen::default();
    {
        let st = e.state();
        // The town's place kinds (static for a run).
        if let Some(m) = mandate_sim::player::home(st).map(|h| h.0) {
            for p in st.world.places.iter().filter(|p| p.municipality.0 == m) {
                seen.place_kinds.insert(p.kind.key().to_string());
            }
        }
    }
    let start = e.state().calendar.date;
    let end = start.add_days(years * 365);
    let start_turn = e.state().calendar.turn;
    let mut last_decision = 0u32;
    let mut next_talk = start;
    while e.state().calendar.date < end {
        bots::play_turn_from(&mut e, content, strategy, start_turn);
        if e.apply(Command::EndTurn).is_err() {
            break;
        }
        let st = e.state();
        // Decisions opened (and the scenes that present them).
        for d in st
            .narrative
            .decisions
            .iter()
            .filter(|d| d.id.0 >= last_decision)
        {
            if d.id.0 < last_decision {
                continue;
            }
            seen.pick("dilemma", &d.template);
            if d.place.is_some()
                && let Some(s) = content.dilemma(&d.template).and_then(|x| x.scene.as_ref())
            {
                seen.pick("scene", s);
            }
        }
        last_decision = st
            .narrative
            .decisions
            .iter()
            .map(|d| d.id.0 + 1)
            .max()
            .unwrap_or(last_decision);
        // Situational policies on the table in the player's town.
        if let Some(m) = mandate_sim::player::home(st).map(|h| h.0) {
            for t in content.policies.values().filter(|p| p.unlock.is_some()) {
                if mandate_sim::policy::unlocked(st, content, t, m) {
                    seen.pick("situational", &t.id);
                }
            }
        }
        // This week's talk at every place where people talk.
        for p in explain::talk_places(st, content) {
            let loc = mandate_sim::places::Location::Place { place: p };
            for l in mandate_sim::overheard::lines(st, content, loc) {
                seen.pick("overheard", &l.id);
            }
            for l in mandate_sim::overheard::available(st, content, loc) {
                seen.held.insert(l.id);
            }
        }
        if st.calendar.date >= next_talk {
            sample_talk(&e, content, &mut seen);
            next_talk = e.state().calendar.date.add_days(TALK_SAMPLE_DAYS);
        }
    }
    Some(seen)
}

pub(super) fn careers(content: &Arc<ContentBundle>, args: &DoctorArgs) -> Vec<Seen> {
    let seeds: Vec<u64> = (1..=args.seeds).collect();
    let workers = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .clamp(1, 4);
    let mut results: Vec<(u64, Seen)> = Vec::new();
    for chunk in seeds.chunks(workers) {
        std::thread::scope(|s| {
            let hs: Vec<_> = chunk
                .iter()
                .map(|seed| {
                    let c = content.clone();
                    let seed = *seed;
                    let strategy = args.strategy.clone();
                    let years = args.years;
                    s.spawn(move || (seed, career(&c, seed, years, &strategy)))
                })
                .collect();
            for h in hs {
                if let Ok((seed, Some(x))) = h.join() {
                    results.push((seed, x));
                }
            }
        });
    }
    results.sort_by_key(|r| r.0);
    results.into_iter().map(|r| r.1).collect()
}

/// Items by kind, as the doctor counts them: (kind, id, file item, group for shares).
pub(super) fn all_items(b: &ContentBundle) -> Vec<(&'static str, String, String, Option<String>)> {
    let mut out = Vec::new();
    for d in &b.dilemmas {
        // Requests, raised dilemmas and an arc's own come up by their own
        // routes; the spotlight's share is among the spotlight's.
        let g = (d.category != mandate_content::DilemmaCategory::Request
            && !d.arc_only
            && d.raised_by.is_none())
        .then(|| "spotlight".to_string());
        out.push(("dilemma", d.id.clone(), d.id.clone(), g));
    }
    for p in b.policies.values().filter(|p| p.unlock.is_some()) {
        out.push(("situational", p.id.clone(), p.id.clone(), None));
    }
    for s in &b.scenes {
        out.push(("scene", s.id.clone(), s.id.clone(), None));
    }
    for o in &b.overheard {
        out.push((
            "overheard",
            o.id.clone(),
            o.id.clone(),
            Some("overheard".into()),
        ));
    }
    for t in &b.talk_topics {
        out.push(("topic", t.id.clone(), t.id.clone(), None));
    }
    for l in &b.talk_lines {
        let g = match l.part {
            TalkPart::Substance => format!("substance:{}", l.topic.clone().unwrap_or_default()),
            TalkPart::Closer => format!("closer:{}", l.voice.clone().unwrap_or_default()),
            p => p.key().to_string(),
        };
        out.push(("line", l.id.clone(), l.id.clone(), Some(g)));
    }
    for v in &b.talk_voices {
        out.push(("voice", v.id.clone(), v.id.clone(), None));
    }
    out
}

pub(super) fn dynamic_checks(
    b: &ContentBundle,
    seen: &[Seen],
    args: &DoctorArgs,
    out: &mut Out,
) -> Vec<KindStats> {
    let n = seen.len();
    let items = all_items(b);
    let mut stats: BTreeMap<&str, KindStats> = BTreeMap::new();
    // Picks per share group.
    let mut group_total: BTreeMap<String, u64> = BTreeMap::new();
    let mut group_size: BTreeMap<String, usize> = BTreeMap::new();
    for (kind, id, _, g) in &items {
        if let Some(g) = g {
            *group_size.entry(g.clone()).or_default() += 1;
            let k = format!("{kind}:{id}");
            *group_total.entry(g.clone()).or_default() += seen
                .iter()
                .map(|s| s.picks.get(&k).copied().unwrap_or(0))
                .sum::<u64>();
        }
    }
    for (kind, id, item, g) in &items {
        let st = stats.entry(kind).or_insert_with(|| KindStats {
            kind: kind.to_string(),
            ..Default::default()
        });
        st.items += 1;
        let k = format!("{kind}:{id}");
        let in_careers = seen.iter().filter(|s| s.picks.contains_key(&k)).count();
        let picks: u64 = seen
            .iter()
            .map(|s| s.picks.get(&k).copied().unwrap_or(0))
            .sum();
        if in_careers > 0 {
            st.fired += 1;
        } else {
            st.never += 1;
            let held = *kind == "line" && seen.iter().any(|s| s.held.contains(id));
            let (sev, msg) = if held {
                (
                    Sev::Info,
                    format!("held for some speaker but was never picked in {n} careers"),
                )
            } else {
                (Sev::Warn, format!("never fired in {n} careers"))
            };
            out.add(sev, "never-fired", item, msg);
            continue;
        }
        let share_careers = in_careers as f64 / n.max(1) as f64;
        let mut often = false;
        if matches!(*kind, "dilemma" | "scene" | "situational")
            && share_careers > args.often_careers
            && !(*kind == "dilemma" && g.is_none())
        {
            often = true;
            out.add(
                Sev::Warn,
                "too-often",
                item,
                format!(
                    "{kind} fired in {in_careers} of {n} careers ({:.0}%)",
                    share_careers * 100.0
                ),
            );
        }
        if let Some(g) = g {
            let total = group_total.get(g).copied().unwrap_or(0);
            let size = group_size.get(g).copied().unwrap_or(1).max(1);
            let share = picks as f64 / total.max(1) as f64;
            if total >= 20 && size >= 3 && share > args.often_share && share > 2.5 / size as f64 {
                often = true;
                out.add(
                    Sev::Warn,
                    "too-often",
                    item,
                    format!(
                        "{:.0}% of the {total} picks among {size} {kind}s in `{g}` (fair share {:.0}%)",
                        share * 100.0,
                        100.0 / size as f64
                    ),
                );
            }
        }
        if often {
            st.too_often += 1;
        }
    }
    // Roles and places no generated town had.
    let kinds: BTreeSet<&String> = seen.iter().flat_map(|s| s.place_kinds.iter()).collect();
    let roles: BTreeSet<&String> = seen.iter().flat_map(|s| s.roles.iter()).collect();
    let rv: BTreeSet<&(String, String)> = seen.iter().flat_map(|s| s.role_voice.iter()).collect();
    let no_town = |out: &mut Out, item: &str, what: String| {
        out.add(
            Sev::Error,
            "no-town-has",
            item,
            format!("{what} in none of the {n} generated towns"),
        );
    };
    let cast_roles = |out: &mut Out, item: &str, c: &Cast| {
        if let Cast::Occupation { occupations } = c
            && !occupations.iter().any(|o| roles.contains(o))
        {
            no_town(out, item, format!("a {} to cast", occupations.join(" or ")));
        }
    };
    for d in &b.dilemmas {
        for r in &d.roles {
            cast_roles(out, &d.id, &r.cast);
        }
    }
    for sc in &b.scenes {
        let k = match &sc.place {
            ScenePlace::Kind { kind } => kind,
            ScenePlace::Workplace { fallback, .. } => fallback,
        };
        if !mandate_content::PSEUDO_PLACES.contains(&k.as_str()) && !kinds.contains(k) {
            no_town(out, &sc.id, format!("a `{k}` for the scene"));
        }
    }
    for t in &b.talk_topics {
        let o = &t.offered;
        let places_ok = o.places.is_empty()
            || o.places
                .iter()
                .any(|k| mandate_content::PSEUDO_PLACES.contains(&k.as_str()) || kinds.contains(k));
        if !places_ok {
            no_town(
                out,
                &t.id,
                format!("a place where it is raised ({})", o.places.join(", ")),
            );
        }
        if !o.roles.is_empty() && !o.roles.iter().any(|r| roles.contains(r)) {
            no_town(
                out,
                &t.id,
                format!("someone it is offered to ({})", o.roles.join(", ")),
            );
        }
    }
    for l in &b.talk_lines {
        if let Some(m) = speaker_roles(&l.when)
            && !m.iter().any(|r| roles.contains(r))
        {
            no_town(
                out,
                &l.id,
                format!(
                    "a speaker with its role ({})",
                    m.into_iter().collect::<Vec<_>>().join(", ")
                ),
            );
        }
        if let Some(lead) = &l.lead
            && let LeadToDef::PlaceKind { kind } = &lead.to
            && !kinds.contains(kind)
        {
            no_town(out, &l.id, format!("a `{kind}` for its lead"));
        }
        // A voice's line for roles never heard in that voice.
        if l.part == TalkPart::Closer
            && let Some(v) = &l.voice
        {
            let allowed = speaker_roles(&l.when);
            let any = rv
                .iter()
                .any(|(r, vv)| vv == v && allowed.as_ref().is_none_or(|a| a.contains(r)));
            if !any && n > 0 {
                out.add(
                    Sev::Warn,
                    "voice",
                    &l.id,
                    format!(
                        "no speaker with voice `{v}`{} was met in {n} careers",
                        allowed
                            .map(|a| format!(
                                " and role {}",
                                a.into_iter().collect::<Vec<_>>().join("/")
                            ))
                            .unwrap_or_default()
                    ),
                );
            }
        }
    }
    let order = [
        "dilemma",
        "situational",
        "scene",
        "overheard",
        "topic",
        "line",
        "voice",
    ];
    order.iter().filter_map(|k| stats.get(k).cloned()).collect()
}

// --- Fluent: missing, unused, arguments ---------------------------------------------------

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::doctor::testkit::*;

    #[test]
    fn a_short_career_run_reports_coverage() {
        let args = DoctorArgs {
            seeds: 1,
            years: 1,
            source: PathBuf::new(),
            ..Default::default()
        };
        let r = diagnose(&base_sources(), &args);
        assert_eq!(r.careers, 1);
        let d = r.coverage.iter().find(|k| k.kind == "dilemma").unwrap();
        assert_eq!(d.items, d.fired + d.never);
        assert!(
            r.coverage
                .iter()
                .any(|k| k.kind == "overheard" && k.fired > 0)
        );
    }
}
