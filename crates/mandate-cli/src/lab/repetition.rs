//! `mandate-cli lab repetition` (DEC-100 item 1): repetition across
//! careers.
//!
//! A returning player's consecutive careers, each in a new world, played
//! by the engaged stand-in. For each career: the share of its dilemmas,
//! arc stages, asks and conversation lines that an earlier career of the
//! same player already showed. Measured and printed with the decision's
//! target beside it (under 40% repeats in career 3); nothing is enforced.
//!
//! What a career "showed" is what reached the player: a decision opened
//! to them, a stage an arc in their focus entered, an ask offered, a line
//! said to them in a conversation. Content ids, so two tellings of one
//! template count as the same thing seen twice.

use super::runner::{self, Driver, Game, Setup};
use mandate_content::ContentBundle;
use mandate_sim::events::EventKind;
use mandate_sim::standin::Kind;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// DEC-100 item 1's target, as data: repeats in career 3.
pub const TARGET_CAREER: usize = 3;
pub const TARGET_REPEATS_UNDER: f64 = 0.40;

/// The kinds of content counted.
pub const KINDS: [&str; 4] = ["dilemmas", "arc_stages", "asks", "conversation_lines"];

pub struct RepetitionArgs {
    pub careers: u32,
    /// Returning players (each plays `careers` careers).
    pub seeds: u64,
    /// Years a career is played (it ends sooner if the act does).
    pub years: u32,
}

/// The world a player's `k`-th career is played in: a new seed each.
pub fn career_seed(player: u64, k: u32) -> u64 {
    player * 1000 + u64::from(k)
}

/// What one career showed, by kind.
type Shown = BTreeMap<&'static str, BTreeSet<String>>;

/// Play one career and collect what it showed.
fn career(content: &Arc<ContentBundle>, seed: u64, years: u32) -> Result<(Shown, Value), String> {
    let mut g = Game::new(
        content,
        &Setup::small(seed, None),
        Driver::standin(Kind::Engaged),
    )?;
    let end = g.start.add_days(years * 365);
    let mut shown: Shown = KINDS.iter().map(|k| (*k, BTreeSet::new())).collect();
    let mut cursor = g.e.state().events.iter().last().map_or(0, |e| e.id.0 + 1);
    let mut talked =
        g.e.state()
            .player
            .as_ref()
            .map_or(0, |p| p.talk.ids.peek_next());
    let mut turns = 0u32;
    // The log is pruned as it ages: read each turn's events as they come.
    loop {
        let st = g.e.state();
        let mut last = cursor;
        let mut add = |kind: &'static str, id: String| {
            if let Some(s) = shown.get_mut(kind) {
                s.insert(id);
            }
        };
        for ev in st.events.iter().filter(|e| e.id.0 >= cursor) {
            last = ev.id.0 + 1;
            match &ev.kind {
                EventKind::DecisionOpened { template, .. } => add("dilemmas", template.clone()),
                EventKind::ArcStage {
                    template, stage, ..
                } => add("arc_stages", format!("{template}:{stage}")),
                EventKind::ArcEnded { template, exit, .. } => {
                    add("arc_stages", format!("{template}:{exit}"))
                }
                EventKind::AskOffered { template, .. } => add("asks", template.clone()),
                _ => {}
            }
        }
        cursor = last;
        if let Some(pl) = st.player.as_ref() {
            for x in pl.talk.log.iter().filter(|x| x.id.0 >= talked) {
                for p in &x.parts {
                    add("conversation_lines", p.line.clone());
                }
            }
            talked = pl.talk.ids.peek_next();
        }
        if st.calendar.date >= end || st.act.over() || !g.step_turn() {
            break;
        }
        turns += 1;
    }
    let st = g.e.state();
    let info = json!({
        "seed": seed,
        "turns": turns,
        "years": f64::from(g.start.days_until(st.calendar.date)) / 365.0,
        "ending": st.act.ending.as_ref().map(|e| e.ending.clone()),
        "state_hash": g.e.state_hash(),
    });
    Ok((shown, info))
}

fn r3(x: f64) -> f64 {
    (x * 1000.0).round() / 1000.0
}

/// One returning player's careers: per career and kind, how much was
/// shown and how much of it an earlier career had shown.
fn player(content: &Arc<ContentBundle>, id: u64, a: &RepetitionArgs) -> Result<Value, String> {
    let mut before: Shown = KINDS.iter().map(|k| (*k, BTreeSet::new())).collect();
    let mut careers = Vec::new();
    for k in 1..=a.careers {
        let (shown, info) = career(content, career_seed(id, k), a.years)?;
        let mut kinds = serde_json::Map::new();
        let (mut all, mut repeats) = (0usize, 0usize);
        for kind in KINDS {
            let now = &shown[kind];
            let old = &before[kind];
            let rep = now.intersection(old).count();
            all += now.len();
            repeats += rep;
            kinds.insert(
                kind.into(),
                json!({ "shown": now.len(), "seen_before": rep,
                    "share": (!now.is_empty()).then(|| r3(rep as f64 / now.len() as f64)) }),
            );
        }
        careers.push(json!({
            "career": k,
            "world": info,
            "shown": all,
            "seen_before": repeats,
            "share": (all > 0).then(|| r3(repeats as f64 / all as f64)),
            "kinds": kinds,
        }));
        for kind in KINDS {
            if let Some(s) = before.get_mut(kind) {
                s.extend(shown[kind].iter().cloned());
            }
        }
    }
    Ok(json!({ "player": id, "careers": careers }))
}

pub fn repetition(content: &Arc<ContentBundle>, a: &RepetitionArgs) -> Result<Value, String> {
    let ids: Vec<u64> = (1..=a.seeds).collect();
    let players: Vec<Value> =
        runner::par_map(&ids, runner::threads(), |id| player(content, *id, a))
            .into_iter()
            .collect::<Result<_, _>>()?;
    // The mean share per career over the players, overall and by kind.
    let mean = |k: u32, path: &dyn Fn(&Value) -> Option<f64>| -> Option<f64> {
        let v: Vec<f64> = players
            .iter()
            .filter_map(|p| p["careers"].as_array()?.get(k as usize - 1))
            .filter_map(path)
            .collect();
        (!v.is_empty()).then(|| r3(v.iter().sum::<f64>() / v.len() as f64))
    };
    let by_career: Vec<Value> = (1..=a.careers)
        .map(|k| {
            let kinds: serde_json::Map<String, Value> = KINDS
                .iter()
                .map(|kind| {
                    (
                        (*kind).to_string(),
                        json!({
                            "share": mean(k, &|c| c["kinds"][kind]["share"].as_f64()),
                            "shown": mean(k, &|c| c["kinds"][kind]["shown"].as_f64()),
                        }),
                    )
                })
                .collect();
            json!({
                "career": k,
                "share_seen_before": mean(k, &|c| c["share"].as_f64()),
                "shown": mean(k, &|c| c["shown"].as_f64()),
                "kinds": kinds,
            })
        })
        .collect();
    let measured = by_career
        .get(TARGET_CAREER - 1)
        .and_then(|c| c["share_seen_before"].as_f64());
    Ok(json!({
        "players": a.seeds,
        "careers": a.careers,
        "years_per_career": a.years,
        "played_by": "standin:engaged",
        "by_career": by_career,
        "bar": {
            "source": "DEC-100 item 1",
            "target": format!("under {TARGET_REPEATS_UNDER} of career {TARGET_CAREER}'s content seen in the earlier careers"),
            "measured": measured,
            "pass": measured.map(|x| x < TARGET_REPEATS_UNDER),
            "enforced": false,
        },
        "rows": players,
    }))
}
