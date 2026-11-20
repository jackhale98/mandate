//! The act flow in the lab (DEC-005, DEC-012, REQ-008).
//!
//! - `act-length`: how long Act 1 is. The informed bots play careers until
//!   the act ends (or `--years`), once playing every turn and once skipping
//!   every stretch out of a campaign (to the campaign, or the end of the
//!   term; after an interrupt the bot plays one turn, then skips again).
//!   Turns per chapter and per act, and minutes at a per-turn time
//!   assumption (`--minutes-per-turn`, default 2.0: the middle of DEC-005's
//!   1–3 min until playtests measure it; a skip costs `--minutes-per-skip`,
//!   default 1.0: directives, the target and reading the summary).
//! - `skip-determinism`: a run that skips, twice and through save/load,
//!   gives one state hash.

use crate::bots;
use mandate_content::ContentBundle;
use mandate_content::act::SkipTargetKind;
use mandate_sim::{Command, Engine};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::Arc;

/// The informed styles (ENGAGEMENT §2.4) the length is measured with.
pub const STYLES: [&str; 4] = [
    "explorer",
    "explorer_coalition",
    "explorer_legislator",
    "explorer_campaigner",
];

fn new_game(content: &Arc<ContentBundle>, seed: u64) -> Option<Engine> {
    generate(
        &WorldConfig {
            seed,
            size: WorldSize::Small,
            scenario: None,
        },
        content.clone(),
    )
    .ok()
}

fn campaigning(e: &Engine) -> bool {
    let me = e.state().player.as_ref().map(|p| p.politician);
    e.state()
        .politics
        .elections
        .races
        .iter()
        .any(|r| r.candidates.iter().any(|c| Some(c.politician) == me))
}

/// One chapter's turns: played by the player, and skipped.
#[derive(Clone, Debug, Default)]
struct Chapter {
    kind: String,
    played: u32,
    skipped: u32,
}

/// One Act 1 career's length.
struct Run {
    played: u32,
    skipped: u32,
    skips: u32,
    chapters: BTreeMap<u32, Chapter>,
    ending: String,
    days: u32,
}

fn chapter_of(e: &Engine) -> (u32, String) {
    e.state()
        .narrative
        .chapter
        .as_ref()
        .map(|c| (c.index, c.kind.key().to_string()))
        .unwrap_or((0, "session".into()))
}

/// Play one career's Act 1 with a style; `skip`: skip every stretch out of
/// a campaign.
fn run(
    content: &Arc<ContentBundle>,
    seed: u64,
    style: &str,
    skip: bool,
    years: u32,
) -> Option<Run> {
    let mut e = new_game(content, seed)?;
    let start = e.state().calendar.date;
    let end = start.add_days(years * 365);
    let start_turn = e.state().calendar.turn;
    let mut r = Run {
        played: 0,
        skipped: 0,
        skips: 0,
        chapters: BTreeMap::new(),
        ending: String::new(),
        days: 0,
    };
    // The bot's learning turns are played (an informed player learns first).
    let learned_by = start_turn + bots::EXPLORE_TURNS;
    let mut just_skipped = false;
    while e.state().calendar.date < end && !e.state().act.over() {
        let active = e.state().player.as_ref().is_some_and(|p| p.status.active());
        let can_skip = skip
            && active
            && !just_skipped
            && !campaigning(&e)
            && e.state().calendar.turn >= learned_by;
        if can_skip {
            let targets = mandate_sim::skip::targets(e.state(), e.content());
            let target = [SkipTargetKind::Campaign, SkipTargetKind::EndOfTerm]
                .into_iter()
                .find(|t| targets.iter().any(|x| x.target == *t));
            if let Some(target) = target {
                // In one-turn chunks, as the client does: each turn is
                // counted to its chapter.
                let mut any = false;
                loop {
                    let (ch, kind) = chapter_of(&e);
                    let before = e.state().calendar.turn;
                    let Ok(out) = e.apply(Command::Skip {
                        target,
                        limit: Some(1),
                    }) else {
                        break;
                    };
                    let n = e.state().calendar.turn - before;
                    if n > 0 {
                        any = true;
                        let c = r.chapters.entry(ch).or_default();
                        c.kind = kind;
                        c.skipped += n;
                        r.skipped += n;
                    }
                    if out.skip.is_none_or(|p| p.done) {
                        break;
                    }
                }
                if any {
                    r.skips += 1;
                    just_skipped = true;
                    continue;
                }
            }
        }
        just_skipped = false;
        let (ch, kind) = chapter_of(&e);
        bots::play_turn_from(&mut e, content, style, start_turn);
        if e.apply(Command::EndTurn).is_err() {
            break;
        }
        let c = r.chapters.entry(ch).or_default();
        c.kind = kind;
        c.played += 1;
        r.played += 1;
    }
    r.ending = e
        .state()
        .act
        .ending
        .as_ref()
        .map(|x| x.ending.clone())
        .unwrap_or_else(|| "none (time limit)".into());
    r.days = start.days_until(e.state().calendar.date);
    Some(r)
}

fn median(mut v: Vec<f64>) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    let n = v.len();
    if n.is_multiple_of(2) {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    } else {
        v[n / 2]
    }
}

/// `act-length`: the REQ-008 table.
pub fn act_length(
    content: &Arc<ContentBundle>,
    seeds: u64,
    years: u32,
    per_turn: f64,
    per_skip: f64,
) -> serde_json::Value {
    let mut modes = serde_json::Map::new();
    for skip in [false, true] {
        let mut act_turns = Vec::new();
        let mut act_played = Vec::new();
        let mut act_minutes = Vec::new();
        let mut years_v = Vec::new();
        let mut by_kind: BTreeMap<String, (Vec<f64>, Vec<f64>)> = BTreeMap::new();
        let mut endings: BTreeMap<String, u32> = BTreeMap::new();
        let mut rows = Vec::new();
        for style in STYLES {
            for seed in 1..=seeds {
                let Some(r) = run(content, seed, style, skip, years) else {
                    continue;
                };
                let minutes = r.played as f64 * per_turn + r.skips as f64 * per_skip;
                act_turns.push((r.played + r.skipped) as f64);
                act_played.push(r.played as f64);
                act_minutes.push(minutes);
                years_v.push(r.days as f64 / 365.0);
                *endings.entry(r.ending.clone()).or_default() += 1;
                for c in r.chapters.values() {
                    let e = by_kind.entry(c.kind.clone()).or_default();
                    e.0.push(c.played as f64);
                    e.1.push(c.played as f64 * per_turn);
                }
                rows.push(json!({
                    "style": style, "seed": seed, "turns": r.played + r.skipped,
                    "played": r.played, "skipped": r.skipped, "skips": r.skips,
                    "chapters": r.chapters.len(), "minutes": minutes,
                    "years": r.days as f64 / 365.0, "ending": r.ending,
                }));
            }
        }
        let chapters: serde_json::Map<String, serde_json::Value> = by_kind
            .into_iter()
            .map(|(k, (t, m))| {
                (
                    k,
                    json!({ "chapters": t.len(), "median_played_turns": median(t), "median_minutes": median(m) }),
                )
            })
            .collect();
        // By ending: a full act (served out) against one cut short.
        let mut by_ending = serde_json::Map::new();
        for e in endings.keys() {
            let rs: Vec<&serde_json::Value> = rows.iter().filter(|r| r["ending"] == *e).collect();
            let col = |k: &str| median(rs.iter().filter_map(|r| r[k].as_f64()).collect());
            by_ending.insert(
                e.clone(),
                json!({
                    "careers": rs.len(), "median_turns": col("turns"), "median_played": col("played"),
                    "median_skips": col("skips"), "median_minutes": col("minutes"),
                    "median_hours": col("minutes") / 60.0, "median_years": col("years"),
                }),
            );
        }
        let m = median(act_minutes.clone());
        modes.insert(
            if skip { "with_skips" } else { "without_skips" }.into(),
            json!({
                "careers": act_turns.len(),
                "act_median_turns": median(act_turns),
                "act_median_played_turns": median(act_played),
                "act_median_minutes": m,
                "act_median_hours": m / 60.0,
                "act_median_years": median(years_v),
                "within_req_008": (90.0..=150.0).contains(&m),
                "chapters_by_kind": chapters,
                "endings": endings,
                "by_ending": by_ending,
                "rows": rows,
            }),
        );
    }
    json!({
        "seeds": seeds,
        "styles": STYLES,
        "max_years": years,
        "assumption": {
            "minutes_per_played_turn": per_turn,
            "minutes_per_skip": per_skip,
            "source": "DEC-005 turn target 1-3 min (middle); replace with the M6 playtest's measured turn time (REQ-008)",
        },
        "targets": { "act_minutes": [90, 150], "chapter_minutes": [25, 45] },
        "modes": modes,
    })
}

/// `skip-determinism`: a scripted run with skips (every other ten turns
/// skipped in chunks, the proxy on bolder directives), twice and through
/// save/load at the half.
pub fn skip_determinism(
    content: &Arc<ContentBundle>,
    seed: u64,
    turns: u32,
) -> Result<serde_json::Value, String> {
    fn step(e: &mut Engine, i: u32) -> Result<(), String> {
        if i == 0 {
            e.apply(Command::SetDirectives {
                choices: vec![
                    ("directive.ground".into(), "lots".into()),
                    ("directive.caution".into(), "bold".into()),
                ],
            })
            .map_err(|x| x.to_string())?;
        }
        let skip = (i / 10) % 2 == 1;
        if skip
            && e.apply(Command::Skip {
                target: SkipTargetKind::EndOfTerm,
                limit: Some(1),
            })
            .is_ok()
        {
            return Ok(());
        }
        e.apply(Command::EndTurn)
            .map(|_| ())
            .map_err(|x| x.to_string())
    }
    let play = |n: u32| -> Result<Engine, String> {
        let mut e = generate(
            &WorldConfig {
                seed,
                size: WorldSize::Medium,
                scenario: None,
            },
            content.clone(),
        )
        .map_err(|x| x.to_string())?;
        for i in 0..n {
            step(&mut e, i)?;
        }
        Ok(e)
    };
    let a = play(turns)?;
    let b = play(turns)?;
    let mut half = play(turns / 2)?;
    let req = mandate_save::SaveRequest {
        run_id: "0".repeat(32),
        revision: 1,
        parent_revision: None,
        device_id: "cli".into(),
        saved_at: "1970-01-01T00:00:00Z".into(),
        protocol_version: mandate_protocol::PROTOCOL_VERSION,
        run_mode: mandate_save::RunMode::Story,
    };
    let (bytes, _) = mandate_save::encode(half.state(), &req).map_err(|x| x.to_string())?;
    let (state, _) = mandate_save::decode(&bytes).map_err(|x| x.to_string())?;
    half = Engine::from_state(state, content.clone()).map_err(|x| x.to_string())?;
    for i in turns / 2..turns {
        step(&mut half, i)?;
    }
    let skipped = a
        .command_log()
        .iter()
        .filter(|c| matches!(c.command, Command::Skip { .. }))
        .count();
    let ok = a.state_hash() == b.state_hash() && a.state_hash() == half.state_hash();
    Ok(json!({
        "ok": ok,
        "skip_commands": skipped,
        "turn": a.state().calendar.turn,
        "run_a": a.state_hash(),
        "run_b": b.state_hash(),
        "save_load_continue": half.state_hash(),
    }))
}
