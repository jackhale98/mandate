//! Mandate simulation lab CLI (DEC-058). Output is JSON for scripting.

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
    let mut e = generate(&WorldConfig { seed, size }, content).map_err(|e| e.to_string())?;
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
        Cmd::SaveInfo { path } => {
            let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
            let meta = mandate_save::read_meta(&bytes).map_err(|e| e.to_string())?;
            out(serde_json::to_value(meta).map_err(|e| e.to_string())?);
            Ok(())
        }
    }
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
