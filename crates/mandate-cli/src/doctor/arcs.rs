//! The doctor's arc lints (STAGED_ARCS §4.5): the ones content validation
//! does not make (task A1 put the graph's hard errors there), and the
//! dynamic ones, which play the arc grader.
//!
//! Static, each a warning (a finding to read, never a failed build):
//! - `arc-no-passive-exit`: no exit can be reached with no act of the
//!   player's (the stages a player who does nothing passes through go
//!   round and never end);
//! - `arc-no-lever`: a stage no way out of which reads anything an act
//!   produces (a mark a beat sets, the run's heat, a law in force, repairs,
//!   the room's support, the race);
//! - `arc-nothing-to-do`: a stage the player can see with no beat they can
//!   take;
//! - `arc-heat-unreachable`: an edge asks for more heat than the run can
//!   have by then;
//! - `arc-exit-flavour`: an exit whose effects are opinions only, with no
//!   follow-up;
//! - `arc-names-office`: text that names an office or a body, instead of
//!   the role word the constitution gives it;
//! - `arc-role-uncast`: text given to a stage that names an optional role.
//!
//! The design's `arc-no-drift` (a stage with no way out when nobody acts)
//! is not here: validation's `arc-stuck` refuses any stage whose last edge
//! is not on time alone, and that edge is its drift. Nor is `arc-tier`:
//! validation refuses a condition that reads a town in a template listed
//! for a wider tier (DEC-098).
//!
//! Dynamic (`--seeds`): `arc-never-raised`, `arc-cast-fails`,
//! `arc-stage-unvisited`, `arc-exit-unvisited`, `arc-too-long`, from the
//! grader's worlds (`crate::arcs_grade`).

use super::{Out, Sev};
use mandate_content::arcs::{ArcBeat, ArcDef, ArcEffect, BeatKind, StageDef};
use mandate_content::{Condition, ContentBundle};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// The marks of a template by who can set them.
struct Marks {
    /// Set by an act of the player's (a visit, a reply, an answer, an ask
    /// kept).
    player: BTreeSet<String>,
    /// Set with no act of the player's (a cast person's move, a story, a
    /// decision's default at its deadline).
    passive: BTreeSet<String>,
}

fn marks(b: &ContentBundle, def: &ArcDef) -> Marks {
    let mut m = Marks {
        player: BTreeSet::new(),
        passive: BTreeSet::new(),
    };
    for beat in def.stages.iter().flat_map(|s| &s.beats) {
        let own = beat.sets.clone();
        match &beat.kind {
            BeatKind::Decision { dilemma, on, .. } => {
                let default = b
                    .dilemma(dilemma)
                    .and_then(|d| d.options.iter().find(|o| o.default))
                    .map(|o| o.id.as_str());
                for (option, outcome) in on {
                    if let Some(s) = &outcome.sets {
                        if Some(option.as_str()) == default {
                            m.passive.insert(s.clone());
                        } else {
                            m.player.insert(s.clone());
                        }
                    }
                }
                // The beat's own mark is set when it is settled, by an
                // answer or by the default.
                m.passive.extend(own.clone());
                m.player.extend(own);
            }
            BeatKind::Talk { choice, .. } => {
                m.player
                    .extend(choice.replies.iter().filter_map(|r| r.sets.clone()));
                m.player.extend(own);
            }
            BeatKind::Visit { .. } | BeatKind::Ask { .. } => m.player.extend(own),
            BeatKind::Press { .. } | BeatKind::Heard { .. } | BeatKind::Move { .. } => {
                m.passive.extend(own)
            }
        }
    }
    m
}

/// Whether these conditions can only hold after an act of the player's:
/// one of them asks for a mark nothing else sets.
fn needs_player(cs: &[Condition], m: &Marks) -> bool {
    fn one(c: &Condition, m: &Marks) -> bool {
        match c {
            Condition::ArcMark { mark, is: true } => !m.passive.contains(mark),
            Condition::Any { of } => !of.is_empty() && of.iter().all(|c| one(c, m)),
            _ => false,
        }
    }
    cs.iter().any(|c| one(c, m))
}

/// Whether these conditions read something an act of the player's
/// produces.
fn reads_lever(cs: &[Condition], m: &Marks) -> bool {
    fn one(c: &Condition, m: &Marks) -> bool {
        match c {
            Condition::ArcMark { mark, is: true } => m.player.contains(mark),
            Condition::ArcHeat { .. }
            | Condition::LawInForce { absent: false, .. }
            | Condition::Recovered { .. }
            | Condition::BodySupport { .. }
            | Condition::LastRace { .. } => true,
            Condition::Any { of } => of.iter().any(|c| one(c, m)),
            _ => false,
        }
    }
    cs.iter().any(|c| one(c, m))
}

/// The stages that can lead to `stage` by the template's edges.
fn before<'a>(def: &'a ArcDef, stage: &str) -> BTreeSet<&'a str> {
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut stack = vec![stage.to_string()];
    while let Some(to) = stack.pop() {
        for s in &def.stages {
            if s.next.iter().any(|e| e.to == to) && seen.insert(s.id.as_str()) {
                stack.push(s.id.clone());
            }
        }
    }
    seen
}

/// The most heat a stage can add: its weekly rise for as long as it can
/// last, and every beat that raises it. `None`: no bound (it rises with
/// no end in sight).
fn heat_gain(def: &ArcDef, s: &StageDef) -> Option<f64> {
    let mut gain = 0.0;
    if s.rise > 0.0 {
        // No longer than the longest clock of the template, or its own
        // slowest edge.
        let clocks = def
            .stages
            .iter()
            .filter_map(|x| x.clock.as_ref())
            .flat_map(|c| [c.days.map(|d| d[1]), c.before_budget_year.map(|d| d[1])])
            .flatten()
            .max();
        let days = s.next.iter().map(|e| e.after_days).max().unwrap_or(0);
        let days = clocks.unwrap_or(0).max(days);
        if days == 0 {
            return None;
        }
        gain += f64::from(s.rise) * f64::from(days) / 7.0;
    }
    let effect = |e: &ArcEffect| match e {
        ArcEffect::Heat { delta } if *delta > 0.0 => f64::from(*delta),
        _ => 0.0,
    };
    for beat in &s.beats {
        gain += beat.effects.iter().map(|g| effect(&g.effect)).sum::<f64>();
        match &beat.kind {
            BeatKind::Decision { on, .. } => {
                gain += on
                    .values()
                    .map(|o| {
                        f64::from(o.heat.max(0.0))
                            + o.effects.iter().map(|g| effect(&g.effect)).sum::<f64>()
                    })
                    .fold(0.0, f64::max);
            }
            BeatKind::Talk { choice, .. } => {
                gain += choice
                    .replies
                    .iter()
                    .map(|r| r.effects.iter().map(|g| effect(&g.effect)).sum::<f64>())
                    .fold(0.0, f64::max);
            }
            _ => {}
        }
    }
    Some(gain)
}

/// Every Fluent key a template's text uses, with the stage it belongs to
/// (none: the template's own).
fn text_keys(def: &ArcDef) -> Vec<(Option<&StageDef>, &str)> {
    let mut out: Vec<(Option<&StageDef>, &str)> = vec![(None, def.title_key.as_str())];
    for s in &def.stages {
        out.push((Some(s), s.key.as_str()));
        if let Some(x) = &s.exit {
            out.extend(
                [&x.headline_key, &x.legacy_key]
                    .into_iter()
                    .flatten()
                    .map(|k| (Some(s), k.as_str())),
            );
        }
        for beat in &s.beats {
            let keys: Vec<&str> = match &beat.kind {
                BeatKind::Visit {
                    verb_key,
                    outcome_key,
                    ..
                } => vec![verb_key, outcome_key],
                BeatKind::Press { headline_key } => vec![headline_key],
                BeatKind::Move { line_key, .. } => vec![line_key],
                BeatKind::Talk { choice, .. } => [&choice.label_key, &choice.say_key]
                    .into_iter()
                    .chain(choice.replies.iter().map(|r| &r.key))
                    .collect(),
                _ => Vec::new(),
            }
            .into_iter()
            .map(String::as_str)
            .collect();
            out.extend(keys.into_iter().map(|k| (Some(s), k)));
        }
    }
    out
}

/// The roles a beat needs bound (the engine offers it only then).
fn beat_roles(beat: &ArcBeat) -> Vec<&str> {
    match &beat.kind {
        BeatKind::Ask { asker, .. } => vec![asker.as_str()],
        BeatKind::Talk { with, .. } => vec![with.as_str()],
        BeatKind::Visit { at, .. } => vec![at.as_str()],
        BeatKind::Move { actor, .. } => vec![actor.as_str()],
        _ => Vec::new(),
    }
}

/// The `$arguments` a message uses.
fn arguments(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut rest = text;
    while let Some(i) = rest.find('$') {
        rest = &rest[i + 1..];
        let name: String = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        if !name.is_empty() {
            out.insert(name);
        }
    }
    out
}

/// The words of a text, lower case.
fn words(text: &str) -> BTreeSet<String> {
    text.split(|c: char| !c.is_alphabetic())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// What the constitutions call their posts: every body's and office's id
/// and member word, and the words of their English names.
fn post_words(b: &ContentBundle, en: &BTreeMap<String, String>) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for c in b.constitutions.values() {
        for t in &c.tiers {
            for body in &t.bodies {
                out.insert(body.id.to_lowercase());
                out.insert(body.member_key.to_lowercase());
                if let Some(name) = en.get(&body.name_key) {
                    out.extend(words(name));
                }
            }
            for office in &t.offices {
                out.insert(office.id.to_lowercase());
                if let Some(name) = en.get(&office.name_key) {
                    out.extend(words(name));
                }
            }
        }
    }
    // Words of a name that are not the post's own ("the", "of", a tier's
    // adjective) would flag ordinary prose.
    out.retain(|w| {
        w.len() > 3 && !["regional", "national", "local", "house"].contains(&w.as_str())
    });
    out
}

pub(super) fn static_checks(b: &ContentBundle, out: &mut Out) {
    // The English messages, by key.
    let en: BTreeMap<String, String> = b
        .locales
        .get("en")
        .into_iter()
        .flat_map(|files| files.values())
        .flat_map(|ftl| super::fluent::messages(ftl))
        .collect();
    let en = &en;
    let posts = post_words(b, en);
    for def in &b.arcs {
        let item = def.id.as_str();
        let m = marks(b, def);
        // --- The graph, as a player who does nothing meets it.
        let mut reached: BTreeSet<&str> = BTreeSet::new();
        let mut stack: Vec<&StageDef> = def.stages.first().into_iter().collect();
        while let Some(s) = stack.pop() {
            if !reached.insert(s.id.as_str()) {
                continue;
            }
            for e in s.next.iter().filter(|e| !needs_player(&e.when, &m)) {
                if let Some(t) = def.stage(&e.to) {
                    stack.push(t);
                }
            }
        }
        if !def
            .stages
            .iter()
            .any(|s| s.exit.is_some() && reached.contains(s.id.as_str()))
        {
            out.add(
                Sev::Warn,
                "arc-no-passive-exit",
                item,
                "no exit can be reached with no act of the player's: a player who does nothing must still see it end",
            );
        }
        let floor = def.heat.floor.is_some();
        for s in def.stages.iter().filter(|s| s.exit.is_none()) {
            let at = |what: &str| format!("stage `{}`: {what}", s.id);
            if !s.next.iter().any(|e| reads_lever(&e.when, &m)) {
                out.add(
                    Sev::Warn,
                    "arc-no-lever",
                    item,
                    at("no way out reads anything an act produces (a mark, heat, a law, repairs, support, the race)"),
                );
            }
            if !s.hidden && !s.beats.iter().any(|x| x.kind.player()) {
                out.add(
                    Sev::Warn,
                    "arc-nothing-to-do",
                    item,
                    at("the player can see it and has no beat to take"),
                );
            }
            // A heat the run cannot have by then (a floor from the
            // simulation can lift it past any bound).
            if !floor {
                let mut bound = Some(f64::from(def.heat.start));
                let earlier = before(def, &s.id);
                for t in def
                    .stages
                    .iter()
                    .filter(|t| t.id == s.id || earlier.contains(t.id.as_str()))
                {
                    bound = bound.zip(heat_gain(def, t)).map(|(a, g)| a + g);
                }
                for e in &s.next {
                    for c in &e.when {
                        if let (Condition::ArcHeat { min: Some(x), .. }, Some(most)) = (c, bound)
                            && most.min(1.0) + 1e-9 < *x
                        {
                            out.add(
                                Sev::Warn,
                                "arc-heat-unreachable",
                                item,
                                at(&format!(
                                    "the edge to `{}` asks for heat {x}; the run can have {:.2} at most by then",
                                    e.to,
                                    most.min(1.0)
                                )),
                            );
                        }
                    }
                }
            }
        }
        // --- Consequences.
        for s in &def.stages {
            if let Some(x) = &s.exit
                && x.follow_up.is_none()
                && !x.effects.is_empty()
                && x.effects.iter().all(|g| !g.effect.material())
            {
                out.add(
                    Sev::Warn,
                    "arc-exit-flavour",
                    item,
                    format!(
                        "exit `{}` changes opinions and favours only: it needs a material effect or a follow-up",
                        s.id
                    ),
                );
            }
        }
        // --- Binding and text.
        let optional: BTreeSet<&str> = def
            .roles
            .iter()
            .filter(|r| r.optional)
            .map(|r| r.id.as_str())
            .collect();
        for (stage, key) in text_keys(def) {
            let Some(text) = en.get(key) else { continue };
            let named: Vec<String> = words(text).intersection(&posts).cloned().collect();
            if !named.is_empty() {
                out.add(
                    Sev::Warn,
                    "arc-names-office",
                    item,
                    format!(
                        "`{key}` names {}: a post is named by the role word the constitution gives (`$<role>_role`), a body not at all",
                        named.join(", ")
                    ),
                );
            }
            // A stage's own text, read whether or not an optional role is
            // bound (a beat's text is read only when its roles are).
            let own = stage.is_some_and(|s| {
                s.key == key
                    || s.exit.as_ref().is_some_and(|x| {
                        x.headline_key.as_deref() == Some(key)
                            || x.legacy_key.as_deref() == Some(key)
                    })
            });
            let guarded: BTreeSet<&str> = stage
                .map(|s| {
                    s.beats
                        .iter()
                        .filter(|beat| text_keys_of(beat).contains(&key))
                        .flat_map(beat_roles)
                        .collect()
                })
                .unwrap_or_default();
            for a in arguments(text) {
                let role = a.strip_suffix("_role").unwrap_or(&a);
                if optional.contains(role) && (own || stage.is_none() || !guarded.contains(role)) {
                    out.add(
                        Sev::Warn,
                        "arc-role-uncast",
                        item,
                        format!("`{key}` uses `${a}`, and the role `{role}` is optional: unbound, the text shows a gap"),
                    );
                }
            }
        }
    }
}

/// The keys of one beat's text.
fn text_keys_of(beat: &ArcBeat) -> Vec<&str> {
    match &beat.kind {
        BeatKind::Visit {
            verb_key,
            outcome_key,
            ..
        } => vec![verb_key.as_str(), outcome_key.as_str()],
        BeatKind::Press { headline_key } => vec![headline_key.as_str()],
        BeatKind::Move { line_key, .. } => vec![line_key.as_str()],
        BeatKind::Talk { choice, .. } => [choice.label_key.as_str(), choice.say_key.as_str()]
            .into_iter()
            .chain(choice.replies.iter().map(|r| r.key.as_str()))
            .collect(),
        _ => Vec::new(),
    }
}

/// The share of worlds above which an uncastable template is a finding
/// (§4.5: "over a fifth").
const CAST_FAILS_SHARE: f64 = 0.2;

/// The dynamic lints: the grader's worlds, read as findings.
pub(super) fn dynamic_checks(b: &Arc<ContentBundle>, seeds: u64, out: &mut Out) {
    let args = crate::arcs_grade::GradeArgs {
        seeds,
        ..Default::default()
    };
    let Ok(grades) = crate::arcs_grade::grade_all(b, &args) else {
        return;
    };
    for g in grades {
        let item = g.arc.as_str();
        let count = |why: &str| g.missed.iter().filter(|m| m.reason == why).count();
        let planted = g.missed.iter().any(|m| m.planted);
        let cast_fails = count("cast_fails");
        if cast_fails as f64 > CAST_FAILS_SHARE * seeds as f64 {
            out.add(
                Sev::Warn,
                "arc-cast-fails",
                item,
                format!(
                    "its `raise` held and no cast could be bound in {cast_fails} of {seeds} worlds{}",
                    crate::arcs_grade::unvisited_reason(&g.arc, "*")
                        .map(|r| format!(" ({r})"))
                        .unwrap_or_default()
                ),
            );
        }
        if g.found == 0 {
            if cast_fails == 0 {
                out.add(
                    Sev::Warn,
                    "arc-never-raised",
                    item,
                    format!(
                        "no world raised it in {seeds} seeds ({}; `raise` never held in {}, held back in {})",
                        if planted { "its cause planted by the lab" } else { "the lab has no intervention that plants its cause" },
                        count("never_raised"),
                        count("held_back"),
                    ),
                );
            }
            continue;
        }
        let reason = |s: &str| crate::arcs_grade::unvisited_reason(&g.arc, s);
        let mut unvisited = |check: &str, list: Vec<&str>| {
            for s in list {
                // Listed with its reason: known, and reported as such.
                let (sev, why) = match reason(s) {
                    Some(r) => (Sev::Info, format!(" ({r})")),
                    None => (Sev::Warn, String::new()),
                };
                out.add(
                    sev,
                    check,
                    item,
                    format!("no stand-in visited `{s}` in {} worlds{why}", g.found),
                );
            }
        };
        unvisited("arc-stage-unvisited", g.coverage.unvisited_stages());
        unvisited("arc-exit-unvisited", g.coverage.unvisited_exits());
        if let Some(t) = g.engaged_turns_to_turn
            && t > g.chapter_turns
        {
            out.add(
                Sev::Warn,
                "arc-too-long",
                item,
                format!(
                    "the engaged median reaches the turn after {t} turns; a chapter is {:.0}",
                    g.chapter_turns
                ),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use crate::doctor::testkit::*;
    use crate::doctor::{Sev, diagnose};
    use mandate_content::Source;

    /// The base pack plus one arc with every defect the static lints name:
    /// - a player who does nothing goes round `a`, `b`, `c` for ever: both
    ///   exits need the mark only a visit sets (no passive exit);
    /// - `c` is left on time alone (no lever);
    /// - `b` shows and offers the player nothing;
    /// - `b`'s edge to `y` asks for heat 0.9, and the run can have 0.24;
    /// - exit `x` changes an opinion and nothing else;
    /// - its title names an office, and stage `b`'s text an optional role.
    fn planted_arc() -> Vec<Source> {
        let mut s = base_sources();
        s.push(Source {
            path: "base/arcs/planted.jsonc".into(),
            text: r#"{
  "kind": "arcs",
  "items": [
    {
      "id": "arc.planted", "title_key": "arc-planted", "tiers": ["local"],
      "raise": [ { "type": "disaster", "within_months": 1 } ],
      "roles": [
        { "id": "hood", "cast": { "type": "area", "pick": "least_pleased" } },
        { "id": "official", "cast": { "type": "post", "power": "propose_budget" }, "optional": true }
      ],
      "heat": { "start": 0.2 },
      "stages": [
        { "id": "a", "key": "arc-planted-a",
          "beats": [ { "id": "seen", "type": "visit", "at": "hood", "verb_key": "visit-arc-planted", "outcome_key": "outcome-arc-planted", "sets": "seen" } ],
          "next": [
            { "to": "x", "when": [ { "type": "arc_mark", "mark": "seen" } ] },
            { "to": "b", "after_days": 7 } ] },
        { "id": "b", "key": "arc-planted-b", "rise": 0.01, "clock": { "days": [14, 28] },
          "next": [
            { "to": "y", "when": [ { "type": "arc_heat", "min": 0.9 }, { "type": "arc_mark", "mark": "seen" } ] },
            { "to": "c", "clock_within_days": 0 } ] },
        { "id": "c", "key": "arc-planted-c", "hidden": true,
          "next": [ { "to": "a", "after_days": 7 } ] },
        { "id": "x", "key": "arc-planted-x", "exit": { "effects": [ { "type": "opinion", "role": "official", "delta": 2 } ] } },
        { "id": "y", "key": "arc-planted-y", "exit": { "effects": [ { "type": "standing", "delta": 1 } ] } }
      ]
    }
  ]
}"#
            .into(),
        });
        s.push(Source {
            path: "base/locale/en/planted_arc.ftl".into(),
            text: "arc-planted = The mayor and the water\n\
                   arc-planted-a = The water is in { $hood }.\n\
                   arc-planted-b = { $official } has not been seen in { $hood }.\n\
                   arc-planted-c = The water stands.\n\
                   arc-planted-x = It ended one way.\n\
                   arc-planted-y = It ended another.\n\
                   visit-arc-planted = Look at the water\n\
                   outcome-arc-planted = You looked at the water.\n"
                .into(),
        });
        s
    }

    #[test]
    fn each_static_arc_lint_has_a_failing_fixture() {
        let r = diagnose(&planted_arc(), &quick());
        let found = |check: &str, text: &str| {
            r.findings
                .iter()
                .any(|f| f.check == check && f.item == "arc.planted" && f.message.contains(text))
        };
        let show = || {
            r.findings
                .iter()
                .filter(|f| f.item == "arc.planted" || f.file.contains("planted"))
                .map(|f| format!("{} {:?} {}: {}", f.check, f.severity, f.item, f.message))
                .collect::<Vec<_>>()
                .join("\n")
        };
        for (check, text) in [
            ("arc-no-passive-exit", "no exit"),
            ("arc-no-lever", "stage `c`"),
            ("arc-nothing-to-do", "stage `b`"),
            ("arc-heat-unreachable", "heat 0.9"),
            ("arc-exit-flavour", "exit `x`"),
            ("arc-names-office", "mayor"),
            ("arc-role-uncast", "$official"),
        ] {
            assert!(
                found(check, text),
                "{check} ({text}) not found in:\n{}",
                show()
            );
        }
        // Report-only: every one is a warning.
        assert!(
            r.findings
                .iter()
                .filter(|f| f.check.starts_with("arc-") && f.item == "arc.planted")
                .all(|f| f.severity == Sev::Warn),
            "{}",
            show()
        );
        // What is sound is not flagged: stage `a` has a lever and a beat.
        assert!(!found("arc-no-lever", "stage `a`"), "{}", show());
        assert!(!found("arc-nothing-to-do", "stage `a`"), "{}", show());
        assert!(!found("arc-exit-flavour", "exit `y`"), "{}", show());
    }

    #[test]
    fn the_shipped_arcs_have_a_passive_exit_and_name_no_office() {
        let r = diagnose(&base_sources(), &quick());
        let any = |check: &str| r.findings.iter().any(|f| f.check == check);
        assert!(!any("arc-no-passive-exit"));
        assert!(!any("arc-names-office"));
        assert!(!any("arc-heat-unreachable"));
    }
}
