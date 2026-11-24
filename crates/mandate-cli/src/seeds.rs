//! `mandate-cli seeds check|refresh`: the test seeds cached in
//! `tests/seeds.json` (see `mandate-seeds`). `check` evaluates every
//! requirement and fails, naming each stale one; `refresh` rewrites the file
//! with the seeds the world needs now. Output is one line per requirement that
//! is not fresh: `stale <name> <cached> -> <found>`.

use mandate_seeds::{Outcome, cache_path, evaluate, refresh, requirements};
use std::io::Write;

pub fn run(refresh_file: bool) -> Result<(), String> {
    let path = cache_path();
    let all: Vec<_> = requirements::ALL
        .iter()
        .chain(crate::lab::seed_reqs::ALL)
        .copied()
        .collect();
    let (outcomes, unknown) = evaluate(&path, &all)?;
    let mut o = std::io::stdout().lock();
    let say = |o: &mut std::io::StdoutLock, line: String| {
        let _ = writeln!(o, "{line}");
    };
    let show = |x: &Option<u64>| x.map_or("none".to_string(), |s| s.to_string());
    let mut problems = Vec::new();
    for x in outcomes.iter().filter(|x| !x.fresh()) {
        say(
            &mut o,
            format!(
                "stale {} {} -> {}  ({})",
                x.name,
                show(&x.cached),
                show(&x.found),
                x.needs
            ),
        );
        if x.found.is_none() {
            problems.push(format!(
                "requirement `{}` ({}): no seed qualifies; the game no longer produces this situation",
                x.name, x.needs
            ));
        }
    }
    for k in &unknown {
        say(
            &mut o,
            format!("unknown {k} (in tests/seeds.json, named by no requirement)"),
        );
    }
    let described = |x: &Outcome| {
        let on_disk = mandate_seeds::read_cache(&path)
            .ok()
            .and_then(|c| c.get(&x.name).map(|e| e.needs.clone()));
        on_disk.as_deref() == Some(x.needs.as_str())
    };
    let stale_text = outcomes.iter().any(|x| !described(x));
    let n = outcomes.iter().filter(|x| !x.fresh()).count() + unknown.len();
    if refresh_file {
        refresh(&path, &outcomes)?;
        say(
            &mut o,
            format!(
                "refreshed tests/seeds.json: {n} changed, {} requirements",
                outcomes.len()
            ),
        );
        return Ok(());
    }
    if !problems.is_empty() {
        return Err(problems.join("\n"));
    }
    if n > 0 || stale_text {
        return Err(format!(
            "tests/seeds.json is stale ({n} requirement(s)): run `scripts/repin.sh` or `mandate-cli seeds refresh`"
        ));
    }
    say(
        &mut o,
        format!("seeds: {} requirements, all fresh", outcomes.len()),
    );
    Ok(())
}
