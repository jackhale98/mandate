//! `mandate-cli lab audition --seed S [--size small|medium|large]`
//! (DEC-101 §5; STAGED_ARCS §6): the draft a new game would offer, with
//! every candidate's four terms, variety and weighted score, and why the
//! three were chosen.
//!
//! The audition is the engine's (`mandate_sim::arcs::audition`): this
//! only prints it. With `--cost` it also times it (the cost table of
//! §6.8): world generation, the scan, each fork and the whole, and the
//! memory a state and a fork take. The figures are the lab's and vary by
//! machine; everything else is a pure function of the seed and the
//! arguments.
//!
//! The wall clock (DEC-041 forbids it in the simulation) is read here
//! only to time the engine from outside, under `--cost`: no figure it
//! gives reaches a state, a draw or the draft.
#![allow(clippy::disallowed_types)]

use mandate_content::ContentBundle;
use mandate_sim::arcs::audition::{self, Auditioned, Draft, Params, SizeClass};
use mandate_sim::arcs::score::{Escalation, Pacing, Visibility};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Instant;

pub struct AuditionArgs {
    pub seed: u64,
    pub size: WorldSize,
    pub scenario: Option<String>,
    /// Overrides of the size's defaults.
    pub candidates: Option<usize>,
    pub second_pass: Option<usize>,
    pub horizon_weeks: Option<u32>,
    pub enforce_floors: bool,
    pub cost: bool,
}

fn r3(x: f64) -> f64 {
    (x * 1000.0).round() / 1000.0
}

fn ms(t: Instant) -> f64 {
    (t.elapsed().as_secs_f64() * 1e5).round() / 100.0
}

fn size_class(s: WorldSize) -> SizeClass {
    match s {
        WorldSize::Small => SizeClass::Small,
        WorldSize::Medium => SizeClass::Medium,
        WorldSize::Large => SizeClass::Large,
    }
}

fn candidate(i: usize, a: &Auditioned, offered: Option<usize>) -> Value {
    let c = &a.candidate;
    let s = a.score.as_ref();
    let term = |e: &Escalation, v: &Visibility, p: &Pacing| {
        json!({
            "escalation": { "value": r3(e.value()), "stages": e.stages, "peak_heat": r3(f64::from(e.peak_heat)), "turned": e.turned },
            "visibility": { "value": r3(v.value()), "entries": v.entries, "seen": v.seen },
            "pacing": { "value": r3(p.value()), "beat_on_turn_1": p.beat_on_turn_1, "turns_to_turn": p.turned, "longest_gap": p.longest_gap, "turns_to_exit": p.exit, "horizon_turns": p.horizon },
        })
    };
    json!({
        "index": i,
        "offered": offered,
        "town": c.town.0,
        "template": c.template,
        "title_key": c.title_key,
        "situation": match &c.situation {
            audition::Situation::Live { run, stage } => json!({ "live": { "run": run, "stage": stage } }),
            audition::Situation::Holds => json!("holds"),
        },
        "args": c.args,
        "opening": { "matter": c.opening.matter, "post": c.opening.post, "town_class": c.opening.town_class },
        "material": c.material,
        "second_pass": a.second_pass(),
        "error": a.error,
        "exits": a.forks.iter().map(|f| json!({ "who": f.kind.key(), "raised": f.trace.raised, "path": f.trace.entries.iter().map(|e| e.stage.clone()).collect::<Vec<_>>(), "exit": f.trace.exit_id() })).collect::<Vec<_>>(),
        "score": s.map(|s| {
            let mut t = term(&s.escalation, &s.visibility, &s.pacing);
            t["agency"] = match &s.agency {
                Some(a) => json!({ "value": r3(a.value()), "passive_engaged_exits_differ": a.passive_engaged_differ, "contrarian_engaged_exits_differ": a.contrarian_engaged_differ, "distance": r3(a.distance) }),
                None => Value::Null,
            };
            t["variety"] = json!(r3(s.variety));
            t["first_pass"] = json!(r3(s.first_pass));
            t["total"] = json!(r3(s.total));
            t["escalates"] = json!(s.escalates);
            t["floors"] = json!({ "agency": s.floors.agency, "visibility": s.floors.visibility, "first_beat": s.floors.first_beat });
            t
        }),
    })
}

fn report(d: &Draft) -> Value {
    json!({
        "months_ahead": d.months_ahead,
        "found": d.found,
        "forks": d.forks,
        "fork_days": d.fork_days,
        "digest": audition::digest(d),
        "draft": d.offered().map(|a| json!({ "town": a.candidate.town.0, "template": a.candidate.template, "total": a.score.as_ref().map(|s| r3(s.total)) })).collect::<Vec<_>>(),
        "candidates": d.auditioned.iter().enumerate().map(|(i, a)| candidate(i, a, d.openings.iter().position(|x| *x == i))).collect::<Vec<_>>(),
    })
}

pub fn run(content: &Arc<ContentBundle>, a: &AuditionArgs) -> Result<Value, String> {
    let t0 = Instant::now();
    let e = generate(
        &WorldConfig {
            seed: a.seed,
            size: a.size,
            scenario: a.scenario.clone(),
        },
        content.clone(),
    )
    .map_err(|e| e.to_string())?;
    let worldgen_ms = ms(t0);
    let mut p = Params::for_size(size_class(a.size));
    if let Some(k) = a.candidates {
        p.candidates = k;
    }
    if let Some(k) = a.second_pass {
        p.second_pass = k;
    }
    if let Some(w) = a.horizon_weeks {
        p.horizon_weeks = w;
    }
    p.enforce_floors = a.enforce_floors;
    let params = json!({ "candidates": p.candidates, "second_pass": p.second_pass, "horizon_weeks": p.horizon_weeks, "scan_months": p.scan_months, "enforce_floors": p.enforce_floors });
    let hash_before = e.state_hash();
    let st = e.state();
    let t_scan = Instant::now();
    let found = audition::scan(st, content, &p.recent).len();
    let scan_ms = ms(t_scan);
    let t_all = Instant::now();
    let mut au = audition::Audition::begin(st, content, p)?;
    let begin_ms = ms(t_all);
    let mut fork_ms = Vec::new();
    loop {
        let t = Instant::now();
        let more = au.step();
        fork_ms.push(ms(t));
        if !more {
            break;
        }
    }
    let d = au.finish();
    let total_ms = ms(t_all);
    if e.state_hash() != hash_before {
        return Err("the audition changed the real state".into());
    }
    let mut out = json!({
        "seed": a.seed,
        "size": format!("{:?}", a.size).to_lowercase(),
        "home": mandate_sim::player::home(st).map(|m| m.0),
        "params": params,
        "found_at_start": found,
        "state_hash": hash_before,
        "audition": report(&d),
    });
    if a.cost {
        let forks = fork_ms.iter().filter(|x| **x > 0.0).count().max(1) as f64;
        let fork_total: f64 = fork_ms.iter().sum();
        let fork_months = d.fork_days as f64 / 30.0;
        let state_bytes = serde_json::to_vec(st).map(|v| v.len()).unwrap_or(0);
        out["cost"] = json!({
            "worldgen_ms": worldgen_ms,
            "scan_ms": scan_ms,
            "begin_ms": begin_ms,
            "forks": d.forks,
            "fork_ms_mean": r3(fork_total / forks),
            "fork_ms_max": fork_ms.iter().copied().fold(0.0, f64::max),
            "fork_month_ms": r3(fork_total / fork_months.max(1.0)),
            "audition_ms": total_ms,
            "state_json_bytes": state_bytes,
            "max_rss_kb": max_rss_kb(),
        });
    }
    Ok(out)
}

/// The process's peak resident memory (Linux; `None` elsewhere).
fn max_rss_kb() -> Option<u64> {
    let s = std::fs::read_to_string("/proc/self/status").ok()?;
    s.lines()
        .find(|l| l.starts_with("VmHWM:"))?
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()
}

/// One line a candidate, for `--text`.
pub fn text(v: &Value) -> String {
    let mut out = String::new();
    let a = &v["audition"];
    out.push_str(&format!(
        "seed {} ({}), home town {}: {} found, {} auditioned, {} forks, {} months ahead\n",
        v["seed"],
        v["size"].as_str().unwrap_or(""),
        v["home"],
        a["found"],
        a["candidates"].as_array().map_or(0, |x| x.len()),
        a["forks"],
        a["months_ahead"],
    ));
    out.push_str("  #  town  template               offer  E      V      P      A      N      total  esc  floors(A,V,1)\n");
    for c in a["candidates"].as_array().into_iter().flatten() {
        let s = &c["score"];
        let f = |k: &str| {
            s[k]["value"]
                .as_f64()
                .map_or("  -  ".to_string(), |x| format!("{x:.3}"))
        };
        out.push_str(&format!(
            "  {:<2} {:<5} {:<22} {:<6} {} {} {} {} {:.3}  {:.3}  {:<4} {}{}{}\n",
            c["index"],
            c["town"],
            c["template"].as_str().unwrap_or(""),
            c["offered"]
                .as_u64()
                .map_or("-".into(), |x| (x + 1).to_string()),
            f("escalation"),
            f("visibility"),
            f("pacing"),
            f("agency"),
            s["variety"].as_f64().unwrap_or(0.0),
            s["total"].as_f64().unwrap_or(0.0),
            if s["escalates"].as_bool().unwrap_or(false) {
                "yes"
            } else {
                "no"
            },
            if s["floors"]["agency"].as_bool().unwrap_or(false) {
                "y"
            } else {
                "n"
            },
            if s["floors"]["visibility"].as_bool().unwrap_or(false) {
                "y"
            } else {
                "n"
            },
            if s["floors"]["first_beat"].as_bool().unwrap_or(false) {
                "y"
            } else {
                "n"
            },
        ));
    }
    if let Some(c) = v.get("cost") {
        out.push_str(&format!("  cost: {c}\n"));
    }
    out
}
