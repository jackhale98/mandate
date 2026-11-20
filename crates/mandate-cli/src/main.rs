//! Mandate simulation lab CLI (DEC-058). Output is JSON for scripting.

mod bots;
mod corpus;
mod doctor;
mod lab;
mod mapimg;
mod png;
mod replay;
mod routines;
mod sweep;

use clap::{Parser, Subcommand, ValueEnum};
use mandate_content::{ContentBundle, Severity, bundle::read_dir_sources};
use mandate_sim::{Command, Engine};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "mandate", about = "Mandate simulation lab")]
struct Cli {
    /// Content root directory.
    #[arg(long, default_value = "content", global = true)]
    content: PathBuf,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Clone, Copy, ValueEnum)]
enum Size {
    Small,
    Medium,
    Large,
}

impl From<Size> for WorldSize {
    fn from(s: Size) -> Self {
        match s {
            Size::Small => WorldSize::Small,
            Size::Medium => WorldSize::Medium,
            Size::Large => WorldSize::Large,
        }
    }
}

#[derive(Subcommand)]
enum Cmd {
    /// Content tools.
    Content {
        #[command(subcommand)]
        cmd: ContentCmd,
    },
    /// Generate a world, play N turns, and print the overview and state hash.
    Run {
        #[arg(long, default_value_t = 1)]
        seed: u64,
        #[arg(long, value_enum, default_value_t = Size::Medium)]
        size: Size,
        #[arg(long, default_value_t = 0)]
        turns: u32,
        /// Extra days to advance after the turns.
        #[arg(long, default_value_t = 0)]
        days: u32,
        /// Print only the full state hash.
        #[arg(long)]
        hash_only: bool,
        /// Write a save file here after running.
        #[arg(long)]
        save: Option<PathBuf>,
    },
    /// Run the same scenario twice (and through save/load) and compare hashes.
    Determinism {
        #[arg(long, default_value_t = 1)]
        seed: u64,
        #[arg(long, default_value_t = 50)]
        turns: u32,
    },
    /// Inspect a save file's metadata.
    SaveInfo { path: PathBuf },
    /// Rebuild a tester's run from a bug report (`mandate-report-….json`):
    /// check the state hash at every checkpoint and at the end, and that the
    /// bundled save loads to it; report the first turn where they differ.
    Replay {
        report: PathBuf,
        /// Stop at the start of this turn and write a save there (`--out`).
        #[arg(long)]
        until: Option<u32>,
        /// Where `--until` writes: a `.mandate` run file (importable on the
        /// title screen) or, with any other extension, a raw engine save.
        /// Default `mandate-replay-<seed>-turn<n>.mandate`.
        #[arg(long)]
        out: Option<PathBuf>,
        /// Print the player's views where the replay stops (pocket, office,
        /// place) as JSON.
        #[arg(long)]
        dump: bool,
        /// Also replay through the WASM build with node and compare
        /// (`scripts/wasm-replay.mjs`; `MANDATE_WASM_NODE` picks the build).
        #[arg(long)]
        wasm: bool,
    },
    /// Monte Carlo health sweep (DEC-059): N seeds, burn-in plus Y years, NPC-only.
    Sweep {
        #[arg(long, default_value_t = 12)]
        seeds: u64,
        #[arg(long, default_value_t = 30)]
        years: u32,
        #[arg(long, value_enum, default_value_t = Size::Small)]
        size: Size,
        /// Scenario id from the content registry (e.g. `scenario.wealthy_coast`
        /// or just `wealthy_coast`); omitted = a random country.
        #[arg(long)]
        scenario: Option<String>,
        #[command(flatten)]
        lab: LabArgs,
    },
    /// Bot players (M4): strategies play the first local election across seeds.
    Bots {
        #[arg(long, default_value_t = 16)]
        seeds: u64,
        /// Print per-seed results for these strategies instead of the report.
        #[arg(long)]
        detail: Vec<String>,
        /// Play careers this many years long with these strategies and print
        /// their outcome distribution (elections, scandals, cases, prison).
        #[arg(long)]
        careers: Option<u32>,
        #[arg(long, default_values_t = vec!["corrupt".to_string(), "mixed".to_string()])]
        career_strategies: Vec<String>,
        /// Lab switch (DEC-091): every bot's player knows every subject in
        /// detail, for baselines against the knowledge ledger.
        #[arg(long)]
        omniscient: bool,
        #[command(flatten)]
        lab: LabArgs,
    },
    /// Play a career with a bot strategy and print its story (DEC-033):
    /// chapters, every decision as the player reads it (Fluent keys and
    /// arguments), storylines, and the facts log.
    Story {
        #[arg(long, default_value_t = 777)]
        seed: u64,
        #[arg(long, value_enum, default_value_t = Size::Small)]
        size: Size,
        #[arg(long, default_value_t = 4)]
        years: u32,
        /// A bot strategy (`idle` never answers).
        #[arg(long, default_value = "mixed")]
        strategy: String,
        /// Storyteller profile id (default: the content's).
        #[arg(long)]
        profile: Option<String>,
    },
    /// Daily routines (DEC-031): where the town's key people are in each part
    /// of the day, one turn's representative day after another.
    Routines {
        #[arg(long, default_value_t = 1)]
        seed: u64,
        /// Turns (representative days) to follow.
        #[arg(long, default_value_t = 7)]
        days: u32,
        #[arg(long, value_enum, default_value_t = Size::Small)]
        size: Size,
        /// Everyone in town, not one of each kind.
        #[arg(long)]
        all: bool,
        /// One line per person and day instead of JSON.
        #[arg(long)]
        text: bool,
    },
    /// Yearly economic trace (balancing diagnostics): jobs by sector, labour
    /// force, prices, and demand drivers.
    TraceEcon {
        #[arg(long, default_value_t = 1)]
        seed: u64,
        #[arg(long, default_value_t = 10)]
        years: u32,
        #[arg(long, value_enum, default_value_t = Size::Medium)]
        size: Size,
    },
    /// Compare two runs' health metrics side by side.
    Diff {
        #[arg(long)]
        seed_a: u64,
        #[arg(long)]
        seed_b: u64,
        #[arg(long, default_value_t = 10)]
        years: u32,
        #[arg(long, value_enum, default_value_t = Size::Small)]
        size: Size,
    },
    /// Generate a world; print a summary and optionally export a map PNG.
    Gen {
        #[arg(long, default_value_t = 1)]
        seed: u64,
        #[arg(long, value_enum, default_value_t = Size::Medium)]
        size: Size,
        /// Scenario id (see `sweep --scenario`).
        #[arg(long)]
        scenario: Option<String>,
        /// Write a map image here.
        #[arg(long)]
        png: Option<PathBuf>,
        #[arg(long, default_value_t = 3)]
        scale: usize,
        /// Print the full world JSON instead of a summary.
        #[arg(long)]
        full: bool,
    },
    /// Show details for one municipality or region of a generated world.
    Inspect {
        #[arg(long, default_value_t = 1)]
        seed: u64,
        #[arg(long, value_enum, default_value_t = Size::Medium)]
        size: Size,
        #[arg(long)]
        municipality: Option<u32>,
        #[arg(long)]
        region: Option<u32>,
    },
}

/// Resolve a scenario argument to a registry id (`wealthy_coast` or
/// `scenario.wealthy_coast`), listing the registry when it is unknown.
fn resolve_scenario(content: &ContentBundle, arg: Option<&str>) -> Result<Option<String>, String> {
    let Some(a) = arg else { return Ok(None) };
    let full = if a.contains('.') {
        a.to_string()
    } else {
        format!("scenario.{a}")
    };
    if content.scenario(&full).is_some() {
        return Ok(Some(full));
    }
    let known: Vec<&str> = content.scenarios.iter().map(|x| x.id.as_str()).collect();
    Err(format!(
        "unknown scenario `{a}` (known: {})",
        known.join(", ")
    ))
}

/// Content overrides for lab runs (DEC-078 decision 2).
#[derive(clap::Args, Clone, Default)]
struct LabArgs {
    /// Override a content value before validation, e.g.
    /// `--set politics.vote.party_weight=1.6` or
    /// `--set sectors[sector.retail].base_wage=31000` (repeatable).
    #[arg(long = "set", value_name = "PATH=VALUE")]
    set: Vec<String>,
    /// Run every combination of up to two parameters, e.g.
    /// `--grid politics.vote.party_weight=1.0,1.4,1.8`, printing one summary
    /// row per combination (gates are reported, not enforced).
    #[arg(long, value_name = "PATH=V1,V2,…")]
    grid: Vec<String>,
}

/// The content combinations a lab run covers: `--set` plus each grid point.
fn lab_runs(
    root: &Path,
    lab: &LabArgs,
) -> Result<Vec<(serde_json::Value, Arc<ContentBundle>)>, String> {
    let sets = lab::parse_sets(&lab.set)?;
    let grid = lab::parse_grid(&lab.grid)?;
    lab::combinations(&grid)
        .into_iter()
        .map(|point| {
            let all: Vec<_> = sets.iter().cloned().chain(point.iter().cloned()).collect();
            Ok((lab::label(&point), lab::load(root, &all)?))
        })
        .collect()
}

#[derive(Subcommand)]
enum ContentCmd {
    /// Validate all content; non-zero exit on errors.
    Validate,
    /// Write the JSON Schema for content files.
    Schema {
        #[arg(long, default_value = "content/schemas/content.schema.json")]
        out: PathBuf,
    },
    /// Report content that never fires, fires too often, Fluent and length
    /// problems (the content workbench); `--strict` fails on errors.
    Doctor(doctor::DoctorArgs),
}

fn load_content(root: &Path) -> Result<Arc<ContentBundle>, String> {
    let sources = read_dir_sources(root).map_err(|e| format!("reading {}: {e}", root.display()))?;
    mandate_content::load(&sources)
        .map(|(b, _)| Arc::new(b))
        .map_err(|issues| {
            issues
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join("\n")
        })
}

fn out(v: serde_json::Value) {
    let mut s = std::io::stdout().lock();
    let _ = writeln!(
        s,
        "{}",
        serde_json::to_string_pretty(&v).unwrap_or_default()
    );
}

fn play(
    content: Arc<ContentBundle>,
    seed: u64,
    size: WorldSize,
    turns: u32,
    days: u32,
) -> Result<Engine, String> {
    let mut e = generate(
        &WorldConfig {
            seed,
            size,
            scenario: None,
        },
        content,
    )
    .map_err(|e| e.to_string())?;
    for _ in 0..turns {
        e.apply(Command::EndTurn).map_err(|e| e.to_string())?;
    }
    if days > 0 {
        e.apply(Command::AdvanceDays { days })
            .map_err(|e| e.to_string())?;
    }
    Ok(e)
}

fn run(cli: Cli) -> Result<(), String> {
    match cli.cmd {
        Cmd::Content {
            cmd: ContentCmd::Validate,
        } => {
            let sources = read_dir_sources(&cli.content).map_err(|e| e.to_string())?;
            let n = sources.len();
            match mandate_content::load(&sources) {
                Ok((b, warnings)) => {
                    for w in &warnings {
                        eprintln!("{w}");
                    }
                    // Every place kind worldgen can generate needs behaviour (DEC-079).
                    let places = mandate_sim::places::registry_issues(&b);
                    if !places.is_empty() {
                        for p in &places {
                            eprintln!("error: places: {p}");
                        }
                        return Err(format!("{} content error(s)", places.len()));
                    }
                    out(
                        serde_json::json!({ "ok": true, "files": n, "content_hash": b.content_hash, "warnings": warnings.len() }),
                    );
                    Ok(())
                }
                Err(issues) => {
                    for i in &issues {
                        eprintln!("{i}");
                    }
                    let errors = issues
                        .iter()
                        .filter(|i| i.severity == Severity::Error)
                        .count();
                    Err(format!("{errors} content error(s)"))
                }
            }
        }
        Cmd::Content {
            cmd: ContentCmd::Doctor(args),
        } => doctor::run(&cli.content, &args),
        Cmd::Content {
            cmd: ContentCmd::Schema { out: path },
        } => {
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
            }
            let schema = serde_json::to_string_pretty(&mandate_content::content_schema())
                .map_err(|e| e.to_string())?;
            std::fs::write(&path, schema + "\n").map_err(|e| e.to_string())?;
            eprintln!("wrote {}", path.display());
            Ok(())
        }
        Cmd::Run {
            seed,
            size,
            turns,
            days,
            hash_only,
            save,
        } => {
            let content = load_content(&cli.content)?;
            let e = play(content, seed, size.into(), turns, days)?;
            if let Some(path) = save {
                let req = mandate_save::SaveRequest {
                    run_id: format!("{seed:032x}"),
                    revision: 1,
                    parent_revision: None,
                    device_id: "cli".into(),
                    saved_at: "1970-01-01T00:00:00Z".into(),
                    protocol_version: mandate_protocol::PROTOCOL_VERSION,
                    run_mode: mandate_save::RunMode::Story,
                };
                let (bytes, _) =
                    mandate_save::encode(e.state(), &req).map_err(|e| e.to_string())?;
                std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
            }
            if hash_only {
                let mut s = std::io::stdout().lock();
                let _ = writeln!(s, "{}", e.state_hash());
            } else {
                let overview = mandate_protocol::overview(&e);
                out(serde_json::json!({ "state_hash": e.state_hash(), "overview": overview }));
            }
            Ok(())
        }
        Cmd::Story {
            seed,
            size,
            years,
            strategy,
            profile,
        } => {
            let content = load_content(&cli.content)?;
            let mut e = generate(
                &WorldConfig {
                    seed,
                    size: size.into(),
                    scenario: None,
                },
                content.clone(),
            )
            .map_err(|e| e.to_string())?;
            if let Some(p) = profile {
                e.setup(|s| s.narrative.profile = p);
            }
            let end = e.state().calendar.date.add_days(years * 365);
            let start_turn = e.state().calendar.turn;
            while e.state().calendar.date < end {
                bots::play_turn_from(&mut e, &content, &strategy, start_turn);
                if e.apply(Command::EndTurn).is_err() {
                    // The career ended (DEC-006).
                    break;
                }
            }
            let me = e.state().player.as_ref().map(|p| p.politician);
            // The press about the player, oldest first (DEC-034).
            let press: Vec<_> = e
                .state()
                .media
                .stories
                .iter()
                .filter(|s| s.subject.is_some() && s.subject == me)
                .filter_map(|s| mandate_protocol::article(&e, s))
                .collect();
            let legal_events: Vec<_> = e
                .state()
                .events
                .iter()
                .filter(|ev| {
                    matches!(
                        ev.kind,
                        mandate_sim::events::EventKind::ScandalBroke { .. }
                            | mandate_sim::events::EventKind::CaseOpened { .. }
                            | mandate_sim::events::EventKind::CaseDeveloped { .. }
                            | mandate_sim::events::EventKind::PlayerImprisoned { .. }
                            | mandate_sim::events::EventKind::PlayerReleased
                            | mandate_sim::events::EventKind::CareerEnded { .. }
                    )
                })
                .map(|ev| mandate_protocol::event_view(ev, &e))
                .filter(|v| {
                    me.is_some_and(|m| {
                        v.args.get("name").map(|n| n.as_str())
                            == e.state().politics.get(m).map(|p| p.full_name()).as_deref()
                            || !v.args.contains_key("name")
                    })
                })
                .collect();
            let decisions: Vec<_> = e
                .state()
                .narrative
                .decisions
                .iter()
                .filter_map(|d| mandate_protocol::decision_view(&e, d))
                .collect();
            out(serde_json::json!({
                "seed": seed,
                "story": mandate_protocol::story_view(&e),
                "decisions": decisions,
                "press": press,
                "legal": mandate_protocol::legal_view(&e),
                "legal_events": legal_events,
                "chapters": e.state().narrative.chapters,
                "facts": e.state().narrative.facts.len(),
            }));
            Ok(())
        }
        Cmd::Determinism { seed, turns } => {
            let content = load_content(&cli.content)?;
            let a = play(content.clone(), seed, WorldSize::Medium, turns, 0)?;
            let b = play(content.clone(), seed, WorldSize::Medium, turns, 0)?;
            let mut half = play(content.clone(), seed, WorldSize::Medium, turns / 2, 0)?;
            let req = mandate_save::SaveRequest {
                run_id: "0".repeat(32),
                revision: 1,
                parent_revision: None,
                device_id: "cli".into(),
                saved_at: "1970-01-01T00:00:00Z".into(),
                protocol_version: mandate_protocol::PROTOCOL_VERSION,
                run_mode: mandate_save::RunMode::Story,
            };
            let (bytes, _) = mandate_save::encode(half.state(), &req).map_err(|e| e.to_string())?;
            let (state, _) = mandate_save::decode(&bytes).map_err(|e| e.to_string())?;
            half = Engine::from_state(state, content).map_err(|e| e.to_string())?;
            for _ in 0..(turns - turns / 2) {
                half.apply(Command::EndTurn).map_err(|e| e.to_string())?;
            }
            let ok = a.state_hash() == b.state_hash() && a.state_hash() == half.state_hash();
            out(
                serde_json::json!({ "ok": ok, "run_a": a.state_hash(), "run_b": b.state_hash(), "save_load_continue": half.state_hash() }),
            );
            if ok {
                Ok(())
            } else {
                Err("determinism violation".into())
            }
        }
        Cmd::Gen {
            seed,
            size,
            scenario,
            png: png_path,
            scale,
            full,
        } => {
            let content = load_content(&cli.content)?;
            let cfg = WorldConfig {
                seed,
                size: size.into(),
                scenario: resolve_scenario(&content, scenario.as_deref())?,
            };
            let g = mandate_worldgen::generate_world(&cfg, &content, &mut |_, _| {})
                .map_err(|e| e.to_string())?;
            if let Some(path) = png_path {
                let (w, h, rgb) = mapimg::render(&g.terrain, &g.world, scale.max(1));
                std::fs::write(&path, png::encode_rgb(w, h, &rgb)).map_err(|e| e.to_string())?;
                eprintln!("wrote {}", path.display());
            }
            if full {
                out(serde_json::to_value(&g.world).map_err(|e| e.to_string())?);
            } else {
                out(summary(&g.world, &g.hood_population));
            }
            Ok(())
        }
        Cmd::Inspect {
            seed,
            size,
            municipality,
            region,
        } => {
            let content = load_content(&cli.content)?;
            let e = generate(
                &WorldConfig {
                    seed,
                    size: size.into(),
                    scenario: None,
                },
                content,
            )
            .map_err(|e| e.to_string())?;
            if let Some(m) = municipality {
                let v = mandate_protocol::municipality_view(&e, m).ok_or("no such municipality")?;
                out(serde_json::to_value(v).map_err(|e| e.to_string())?);
            } else if let Some(r) = region {
                let st = e.state();
                let re = st.world.regions.get(r as usize).ok_or("no such region")?;
                out(serde_json::json!({
                    "region": re,
                    "population": st.population.region_total(&st.world, re.id),
                    "environment": st.environment.regions.get(r as usize),
                    "treasury": st.finance.regional.get(r as usize).map(|t| &t.last_year),
                }));
            } else {
                let st = e.state();
                let mut v = serde_json::to_value(mandate_protocol::overview(&e))
                    .map_err(|e| e.to_string())?;
                v["national_treasury"] =
                    serde_json::to_value(&st.finance.national).map_err(|e| e.to_string())?;
                out(v);
            }
            Ok(())
        }
        Cmd::Sweep {
            seeds,
            years,
            size,
            scenario,
            lab,
        } => {
            let runs = lab_runs(&cli.content, &lab)?;
            if !lab.grid.is_empty() {
                // One summary row per combination: bands and floors, no runs.
                let mut rows = Vec::new();
                for (point, content) in &runs {
                    let scenario = resolve_scenario(content, scenario.as_deref())?;
                    let r = sweep::report(
                        &sweep::run(content, seeds, years, size.into(), scenario.as_deref()),
                        years,
                    );
                    let row = serde_json::json!({ "set": point, "pass": r["pass"], "bands": r["bands"], "drama_floors": r["drama_floors"] });
                    eprintln!("{}", serde_json::to_string(&row).unwrap_or_default());
                    rows.push(row);
                }
                out(
                    serde_json::json!({ "seeds": seeds, "years": years, "base_set": lab.set, "rows": rows }),
                );
                return Ok(());
            }
            let content = runs.into_iter().next().map(|r| r.1).ok_or("no content")?;
            let scenario = resolve_scenario(&content, scenario.as_deref())?;
            let rows = sweep::run(&content, seeds, years, size.into(), scenario.as_deref());
            let report = sweep::report(&rows, years);
            let ok = report["pass"].as_bool().unwrap_or(false);
            out(report);
            if ok {
                Ok(())
            } else {
                Err("Monte Carlo bands or drama floors failed".into())
            }
        }
        Cmd::Bots {
            seeds,
            detail,
            careers,
            career_strategies,
            omniscient,
            lab,
        } => {
            bots::set_omniscient(omniscient);
            let runs = lab_runs(&cli.content, &lab)?;
            if !lab.grid.is_empty() {
                // One summary row per combination: win rates and the gates.
                let mut rows = Vec::new();
                for (point, content) in &runs {
                    let r = bots::run(content, seeds);
                    let rates: serde_json::Map<String, serde_json::Value> = r["strategies"]
                        .as_object()
                        .map(|o| {
                            o.iter()
                                .map(|(k, v)| (k.clone(), v["win_rate"].clone()))
                                .collect()
                        })
                        .unwrap_or_default();
                    let mut row = serde_json::json!({
                        "set": point,
                        "win_rates": rates,
                        "best_minus_median": r["best_minus_median"],
                        "distinct_best": r["variety"]["distinct_best"],
                        "top_best_share": r["variety"]["top_best_share"],
                        "explorer_minus_mixed_share": r["explorer_minus_mixed_share"],
                    });
                    for g in bots::GATES {
                        row[g] = r[g].clone();
                    }
                    eprintln!("{}", serde_json::to_string(&row).unwrap_or_default());
                    rows.push(row);
                }
                out(serde_json::json!({ "seeds": seeds, "base_set": lab.set, "rows": rows }));
                return Ok(());
            }
            let content = runs.into_iter().next().map(|r| r.1).ok_or("no content")?;
            if let Some(years) = careers {
                let rows: Vec<serde_json::Value> = career_strategies
                    .iter()
                    .map(|s| bots::careers(&content, seeds, s, years))
                    .collect();
                out(serde_json::Value::Array(rows));
                return Ok(());
            }
            if !detail.is_empty() {
                let mut rows = Vec::new();
                for seed in 1..=seeds {
                    let mut row = serde_json::Map::new();
                    row.insert("seed".into(), seed.into());
                    for s in &detail {
                        row.insert(
                            s.clone(),
                            match bots::play(&content, seed, s) {
                                Some((won, share)) => {
                                    serde_json::json!({ "won": won, "share": share })
                                }
                                None => serde_json::Value::Null,
                            },
                        );
                    }
                    rows.push(serde_json::Value::Object(row));
                }
                out(serde_json::Value::Array(rows));
                return Ok(());
            }
            let report = bots::run(&content, seeds);
            let failed = bots::failed_gates(&report);
            out(report);
            if failed.is_empty() {
                Ok(())
            } else {
                Err(format!("bot gates failed: {}", failed.join(", ")))
            }
        }
        Cmd::Routines {
            seed,
            days,
            size,
            all,
            text,
        } => {
            let content = load_content(&cli.content)?;
            let v = routines::run(&content, seed, days, size.into(), all, text)?;
            if !text {
                out(v);
            }
            Ok(())
        }
        Cmd::TraceEcon { seed, years, size } => {
            let content = load_content(&cli.content)?;
            let mut e = generate(
                &WorldConfig {
                    seed,
                    size: size.into(),
                    scenario: None,
                },
                content,
            )
            .map_err(|e| e.to_string())?;
            let mut rows = Vec::new();
            for y in 0..=years {
                if y > 0 {
                    e.apply(Command::AdvanceDays { days: 365 })
                        .map_err(|e| e.to_string())?;
                }
                let st = e.state();
                let ec = &st.economy;
                let n = ec.sector_keys.len();
                let mut jobs = vec![0.0f64; n];
                let mut cap = vec![0.0f64; n];
                for m in &ec.munis {
                    for (k, sct) in m.sectors.iter().enumerate() {
                        jobs[k] += sct.jobs;
                        cap[k] += sct.capacity;
                    }
                }
                let lf: f64 = ec.munis.iter().map(|m| m.labour_force).sum();
                let cons: f64 = ec.munis.iter().map(|m| m.consumption).sum();
                let base: f64 = ec.munis.iter().map(|m| m.base_consumption).sum();
                rows.push(serde_json::json!({
                    "year": st.calendar.date.calendar().year,
                    "lf_k": (lf / 1000.0).round(),
                    "jobs_k": jobs.iter().map(|j| (j / 1000.0).round()).collect::<Vec<_>>(),
                    "jobs_over_capacity": jobs.iter().zip(cap.iter()).map(|(j, c)| (j / c.max(1.0) * 100.0).round() / 100.0).collect::<Vec<_>>(),
                    "unemployment": (ec.national.unemployment * 1000.0).round() / 1000.0,
                    "price_level": (ec.national.price_level * 1000.0).round() / 1000.0,
                    "real_consumption_ratio": ((cons / ec.national.price_level / base.max(1.0)) * 1000.0).round() / 1000.0,
                    "public_index": (st.finance.public_spending_index(0) * 1000.0).round() / 1000.0,
                    "global_demand": (ec.global.demand * 1000.0).round() / 1000.0,
                }));
            }
            out(serde_json::Value::Array(rows));
            Ok(())
        }
        Cmd::Diff {
            seed_a,
            seed_b,
            years,
            size,
        } => {
            let content = load_content(&cli.content)?;
            let a = sweep::run_one(&content, seed_a, years, size.into(), None);
            let b = sweep::run_one(&content, seed_b, years, size.into(), None);
            out(serde_json::json!({ "a": a, "b": b }));
            Ok(())
        }
        Cmd::Replay {
            report,
            until,
            out: out_path,
            dump,
            wasm,
        } => {
            let content = load_content(&cli.content)?;
            let checked = replay::run(
                content,
                &report,
                &replay::Options {
                    until,
                    out: out_path,
                    dump,
                    wasm,
                },
            )?;
            out(checked.summary);
            if checked.ok {
                Ok(())
            } else {
                Err("the replay does not match the report (see `first_divergence`)".into())
            }
        }
        Cmd::SaveInfo { path } => {
            let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
            let meta = mandate_save::read_meta(&bytes).map_err(|e| e.to_string())?;
            out(serde_json::to_value(meta).map_err(|e| e.to_string())?);
            Ok(())
        }
    }
}

fn summary(world: &mandate_sim::world::World, hood_pop: &[u64]) -> serde_json::Value {
    use mandate_sim::world::SettlementClass as S;
    use mandate_sim::world::Tier;
    let classes = |c| world.municipalities.iter().filter(|m| m.class == c).count();
    let muni_pop = |m: &mandate_sim::world::Municipality| -> u64 {
        m.neighborhoods.iter().map(|h| hood_pop[h.index()]).sum()
    };
    serde_json::json!({
        "population": hood_pop.iter().sum::<u64>(),
        "regions": world.regions.iter().map(|r| serde_json::json!({
            "name": r.name, "municipalities": r.municipalities.len(),
            "population": r.municipalities.iter().map(|m| muni_pop(&world.municipalities[m.index()])).sum::<u64>(),
            "capital": world.municipalities[r.capital.index()].name,
        })).collect::<Vec<_>>(),
        "municipalities": world.municipalities.len(),
        "classes": { "metro": classes(S::Metro), "city": classes(S::City), "town": classes(S::Town), "rural": classes(S::Rural) },
        "capital": world.capital.map(|c| world.municipalities[c.index()].name.clone()),
        "neighborhoods": world.neighborhoods.len(),
        "constituencies": {
            "local": world.constituencies.iter().filter(|c| c.tier == Tier::Local).count(),
            "regional": world.constituencies.iter().filter(|c| c.tier == Tier::Regional).count(),
            "national": world.constituencies.iter().filter(|c| c.tier == Tier::National).count(),
        },
        "links": world.links.len(),
        "ports": world.municipalities.iter().filter(|m| m.port).count(),
        "places": world.places.len(),
        "largest": world.municipalities.iter().take(5).map(|m| serde_json::json!({
            "name": m.name, "class": m.class, "population": muni_pop(m), "biome": m.geo.dominant_biome,
        })).collect::<Vec<_>>(),
    })
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
