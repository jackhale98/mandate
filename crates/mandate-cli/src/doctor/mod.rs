//! `mandate-cli content doctor` (the content workbench, owner 2026-11-19): a
//! lab report on the content pack. It finds
//!
//! - content that can never fire: contradictory or impossible bounds,
//!   conditions that contradict each other, references to ids that do not
//!   exist, and roles or places no generated town has; and content that
//!   never fired across N bot-played careers (dilemmas, situational
//!   policies, scenes, overheard lines, topics,
//!   conversation lines and voices);
//! - content that fires too often (in most careers, or far above its share
//!   of the picks of its kind);
//! - Fluent problems: keys referenced by content or named by the engine but
//!   missing from English, keys nothing references, and arguments a message
//!   uses that its caller never supplies (where that is static);
//! - text too long for a phone, rendered with representative arguments
//!   (the longest variant of every selector);
//! - voice problems: lines tagged with a voice or a role that the speakers
//!   who can say them cannot have;
//! - text that reads wrong (text audit 1): doubled punctuation and stray
//!   braces in a message, a line asserting a worry its conditions do not
//!   make real, and, in a rendered corpus, arguments that came out empty;
//! - prose habits that read as machine-written (`prose-habit`, info, or
//!   warn under `--strict-prose`; `docs/design/STYLE_BIBLE.md`).
//!
//! Findings have a severity (error, warn, info); `--strict` exits non-zero
//! on errors. Read-only: careers are played on throwaway engines.

use crate::bots;
use mandate_content::condition::Worry;
use mandate_content::conversation::{
    ConcernOf, LeadToDef, TalkLineDef, TalkPart, fluent_placeholders,
};
use mandate_content::validate::fluent_keys;
use mandate_content::{Cast, Condition, ContentBundle, ScenePlace, Source};
use mandate_sim::explain;
use mandate_sim::{Command, Engine};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(clap::Args, Clone, Debug)]
pub struct DoctorArgs {
    /// Bot-played careers to look for content that never (or too often) fires.
    #[arg(long, default_value_t = 16)]
    pub seeds: u64,
    /// Years per career.
    #[arg(long, default_value_t = 4)]
    pub years: u32,
    /// The bots' strategy.
    #[arg(long, default_value = "mixed")]
    pub strategy: String,
    /// Print the full report as JSON.
    #[arg(long)]
    pub json: bool,
    /// Exit non-zero when there are errors.
    #[arg(long)]
    pub strict: bool,
    /// Report the prose habits (`prose-habit`) as warnings, not infos.
    #[arg(long)]
    pub strict_prose: bool,
    /// Longest title (dilemma, topic), in characters.
    #[arg(long, default_value_t = 60)]
    pub max_title: usize,
    /// Longest line (overheard, conversation, scene beat, story step).
    #[arg(long, default_value_t = 280)]
    pub max_line: usize,
    /// Longest option label (a button), in characters.
    #[arg(long, default_value_t = 90)]
    pub max_label: usize,
    /// Longest body (a dilemma's situation or aftermath).
    #[arg(long, default_value_t = 600)]
    pub max_body: usize,
    /// Share of careers above which an item fires too often.
    #[arg(long, default_value_t = 0.6)]
    pub often_careers: f64,
    /// Share of all picks of its kind above which an item fires too often
    /// (and at least 2.5 times its fair share).
    #[arg(long, default_value_t = 0.25)]
    pub often_share: f64,
    /// Source tree scanned for the Fluent keys the engine and the UI name
    /// (`crates/*/src`, `web/src`); empty to skip.
    #[arg(long, default_value = ".")]
    pub source: PathBuf,
    /// Write the rendered-text corpus here instead of the report: every
    /// piece of player-visible prose from the careers, as the player sees
    /// it, with its situation (JSONL, Markdown, counts).
    #[arg(long, value_name = "DIR")]
    pub corpus: Option<PathBuf>,
    /// Read a corpus `--corpus` wrote and report what reads wrong in it
    /// (empty arguments, doubled punctuation, raw placeables), without
    /// playing or rendering again.
    #[arg(long, value_name = "DIR")]
    pub check_corpus: Option<PathBuf>,
}

impl Default for DoctorArgs {
    fn default() -> Self {
        DoctorArgs {
            seeds: 16,
            years: 4,
            strategy: "mixed".into(),
            json: false,
            strict: false,
            strict_prose: false,
            max_title: 60,
            max_line: 280,
            max_body: 600,
            max_label: 90,
            often_careers: 0.6,
            often_share: 0.25,
            source: PathBuf::from("."),
            corpus: None,
            check_corpus: None,
        }
    }
}

// --- Findings ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Sev {
    Error,
    Warn,
    Info,
}

impl Sev {
    fn key(self) -> &'static str {
        match self {
            Sev::Error => "error",
            Sev::Warn => "warn",
            Sev::Info => "info",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Finding {
    pub severity: Sev,
    /// `validate` | `unsatisfiable` | `contradiction` | `unknown-ref` |
    /// `no-town-has` | `never-fired` | `too-often` | `fluent-missing` |
    /// `fluent-unused` | `fluent-args` | `too-long` | `voice` |
    /// `text-raw` | `text-empty-arg` | `text-doubled-punct` |
    /// `worry-unconditioned` | `subject-unread` | `subject-drift` |
    /// `subject-stranger` |
    /// `talk-off-subject` (corpus) | `prose-habit`.
    pub check: String,
    /// The content file (or `.ftl`) that defines the item.
    pub file: String,
    pub item: String,
    pub message: String,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct KindStats {
    pub kind: String,
    pub items: usize,
    /// Items seen firing in at least one career.
    pub fired: usize,
    pub never: usize,
    pub too_often: usize,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Report {
    pub content_hash: String,
    pub seeds: u64,
    pub years: u32,
    pub strategy: String,
    pub careers: usize,
    pub coverage: Vec<KindStats>,
    /// (check, errors, warns, infos).
    pub summary: Vec<(String, usize, usize, usize)>,
    pub findings: Vec<Finding>,
}

impl Report {
    pub fn errors(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == Sev::Error)
            .count()
    }

    #[cfg(test)]
    pub fn has(&self, check: &str, item: &str) -> bool {
        self.findings
            .iter()
            .any(|f| f.check == check && f.item == item)
    }
}

struct Out {
    findings: Vec<Finding>,
    files: BTreeMap<String, String>,
    /// The constitution's role words (`office` conditions; DEC-098).
    role_words: Vec<String>,
}

impl Out {
    fn file_of(&self, item: &str) -> String {
        self.files.get(item).cloned().unwrap_or_else(|| "?".into())
    }

    fn add(&mut self, sev: Sev, check: &str, item: &str, message: impl Into<String>) {
        let file = self.file_of(item);
        self.findings.push(Finding {
            severity: sev,
            check: check.into(),
            file,
            item: item.into(),
            message: message.into(),
        });
    }
}

/// Where each id (`"id": "…"`) and each Fluent key is defined.
fn origins(sources: &[Source]) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for s in sources {
        if s.path.ends_with(".ftl") {
            for k in fluent_keys(&s.text) {
                out.entry(k).or_insert_with(|| s.path.clone());
            }
            continue;
        }
        let mut rest = s.text.as_str();
        while let Some(i) = rest.find("\"id\"") {
            rest = &rest[i + 4..];
            let after = rest.trim_start();
            let Some(after) = after.strip_prefix(':') else {
                continue;
            };
            let after = after.trim_start();
            if let Some(v) = after.strip_prefix('"')
                && let Some(end) = v.find('"')
            {
                out.entry(v[..end].to_string())
                    .or_insert_with(|| s.path.clone());
            }
        }
    }
    out
}

// --- Loading ---------------------------------------------------------------------------

const STUBS: &str = "locale/en/zz-doctor-stubs.ftl";

/// Load the pack. Missing English keys stop a normal load; the doctor stubs
/// them (and reports each) so the rest can be checked. Any other error is
/// reported and ends the run.
fn load(sources: &[Source], out: &mut Out) -> Option<Arc<ContentBundle>> {
    match mandate_content::load(sources) {
        Ok((b, warnings)) => {
            for w in warnings {
                out.add(Sev::Warn, "validate", &w.file, w.message);
            }
            Some(Arc::new(b))
        }
        Err(issues) => {
            let mut missing = BTreeSet::new();
            let mut fatal = false;
            for i in &issues {
                let key = i
                    .message
                    .strip_prefix("missing Fluent key `")
                    .and_then(|r| r.split('`').next());
                match (i.severity, key) {
                    (_, Some(k)) => {
                        missing.insert(k.to_string());
                        out.add(
                            Sev::Error,
                            "fluent-missing",
                            &i.file,
                            format!("`{k}` is referenced but missing in English"),
                        );
                    }
                    (mandate_content::Severity::Error, None) => {
                        fatal = true;
                        out.add(Sev::Error, "validate", &i.file, i.message.clone());
                    }
                    (mandate_content::Severity::Warning, None) => {
                        out.add(Sev::Warn, "validate", &i.file, i.message.clone())
                    }
                }
            }
            if fatal || missing.is_empty() {
                return None;
            }
            let mut stub = String::new();
            for k in &missing {
                stub.push_str(&format!("{k} = ⟦{k}⟧\n"));
            }
            let mut with = sources.to_vec();
            with.push(Source {
                path: STUBS.into(),
                text: stub,
            });
            match mandate_content::load(&with) {
                Ok((b, _)) => Some(Arc::new(b)),
                Err(issues) => {
                    for i in issues {
                        out.add(Sev::Error, "validate", &i.file, i.message);
                    }
                    None
                }
            }
        }
    }
}

// --- Static: bounds and contradictions -----------------------------------------------------

mod conditions;
mod corpus;
mod fluent;
mod frequency;
mod length;
mod references;
mod text;
mod worry;
#[cfg(test)]
mod testkit {
    use super::*;

    pub(super) fn base_sources() -> Vec<Source> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
        mandate_content::bundle::read_dir_sources(&root).unwrap()
    }

    /// The base pack plus a planted dilemma that can never fire (its
    /// triggers contradict each other), an option whose aftermath key is
    /// missing from English, and an overheard line far too long for a phone.
    pub(super) fn planted() -> Vec<Source> {
        let mut s = base_sources();
        s.push(Source {
            path: "base/planted_dilemmas.jsonc".into(),
            text: r#"{
  "kind": "dilemmas",
  "items": [
    {
      "id": "dilemma.planted_never", "category": "significant",
      "title_key": "dilemma-planted-never", "situation_key": "dilemma-planted-never-situation",
      "roles": [ { "id": "rival", "cast": { "type": "rival" } } ],
      "triggers": [
        { "type": "track", "track": "standing", "min": 80 },
        { "type": "track", "track": "standing", "max": 20 }
      ],
      "cooldown_days": 30,
      "options": [
        { "id": "a", "label_key": "dilemma-planted-never-a", "aftermath_key": "dilemma-planted-missing-after", "default": true,
          "effects": [ { "type": "standing", "delta": 2 }, { "type": "health", "delta": -2 } ] },
        { "id": "b", "label_key": "dilemma-planted-never-b", "aftermath_key": "dilemma-planted-never-b-after",
          "effects": [ { "type": "standing", "delta": -2 }, { "type": "health", "delta": 2 } ] }
      ]
    }
  ]
}"#
            .into(),
        });
        s.push(Source {
            path: "base/planted_overheard.jsonc".into(),
            text: r#"{ "kind": "overheard", "items": [ { "id": "overheard.planted_long", "key": "overheard-planted-long" } ] }"#.into(),
        });
        let long = "On and on it goes about the { $town } bypass, ".repeat(9);
        s.push(Source {
            path: "base/locale/en/planted.ftl".into(),
            text: format!(
                "dilemma-planted-never = A planted dilemma\n\
                 dilemma-planted-never-situation = {{ $rival }} waits.\n\
                 dilemma-planted-never-a = Wait\n\
                 dilemma-planted-never-b = Act\n\
                 dilemma-planted-never-b-after = You acted.\n\
                 overheard-planted-long = {long}\n"
            ),
        });
        s
    }

    pub(super) fn quick() -> DoctorArgs {
        DoctorArgs {
            seeds: 0,
            source: PathBuf::new(),
            ..Default::default()
        }
    }
}

use self::conditions::*;
pub use self::corpus::*;
use self::fluent::*;
use self::frequency::*;
use self::length::*;
use self::references::*;
pub use self::text::*;
use self::worry::*;

/// Run every check on a set of sources.
pub fn diagnose(sources: &[Source], args: &DoctorArgs) -> Report {
    let mut out = Out {
        findings: Vec::new(),
        files: origins(sources),
        role_words: Vec::new(),
    };
    let mut report = Report {
        seeds: args.seeds,
        years: args.years,
        strategy: args.strategy.clone(),
        ..Default::default()
    };
    if let Some(b) = load(sources, &mut out) {
        report.content_hash = b.content_hash.clone();
        out.role_words = b.role_words();
        static_checks(&b, &mut out);
        let scan = scan_sources(&args.source);
        fluent_checks(&b, sources, scan.as_ref(), &mut out);
        length_checks(&b, args, &mut out);
        text_checks(&b, &mut out);
        prose_checks(&b, args, &mut out);
        worry_checks(&b, &mut out);
        relevance_checks(&b, &mut out);
        if args.seeds > 0 {
            let seen = careers(&b, args);
            report.careers = seen.len();
            report.coverage = dynamic_checks(&b, &seen, args, &mut out);
        }
    }
    // The stubs file is the doctor's own.
    out.findings.retain(|f| f.file != STUBS);
    out.findings.sort_by(|a, b| {
        (
            a.file.as_str(),
            a.severity,
            a.item.as_str(),
            a.check.as_str(),
        )
            .cmp(&(
                b.file.as_str(),
                b.severity,
                b.item.as_str(),
                b.check.as_str(),
            ))
    });
    out.findings.dedup();
    let mut summary: BTreeMap<String, (usize, usize, usize)> = BTreeMap::new();
    for f in &out.findings {
        let e = summary.entry(f.check.clone()).or_default();
        match f.severity {
            Sev::Error => e.0 += 1,
            Sev::Warn => e.1 += 1,
            Sev::Info => e.2 += 1,
        }
    }
    report.summary = summary
        .into_iter()
        .map(|(k, (e, w, i))| (k, e, w, i))
        .collect();
    report.findings = out.findings;
    report
}

/// The corpus text findings for a terminal.
fn print_text_findings(found: &[Finding]) {
    use std::io::Write;
    let mut o = std::io::stdout().lock();
    let _ = writeln!(o, "Text in the corpus: {} problem(s)", found.len());
    for f in found {
        let _ = writeln!(o, "  {} {}: {}", f.check, f.item, f.message);
    }
}

/// The report as text for a terminal.
pub fn human(r: &Report) -> String {
    let mut s = String::new();
    let hash: String = r.content_hash.chars().take(12).collect();
    s.push_str(&format!(
        "Content doctor: {} careers × {} years (strategy {}), content {hash}\n\n",
        r.careers, r.years, r.strategy
    ));
    s.push_str("Summary\n");
    s.push_str(&format!(
        "  {:<16} {:>6} {:>6} {:>6}\n",
        "check", "errors", "warns", "infos"
    ));
    let (mut te, mut tw, mut ti) = (0, 0, 0);
    for (k, e, w, i) in &r.summary {
        s.push_str(&format!("  {k:<16} {e:>6} {w:>6} {i:>6}\n"));
        te += e;
        tw += w;
        ti += i;
    }
    s.push_str(&format!("  {:<16} {te:>6} {tw:>6} {ti:>6}\n", "total"));
    if !r.coverage.is_empty() {
        s.push_str(&format!(
            "\nCoverage in {} careers\n  {:<12} {:>6} {:>6} {:>6} {:>9}\n",
            r.careers, "kind", "items", "fired", "never", "too often"
        ));
        for k in &r.coverage {
            s.push_str(&format!(
                "  {:<12} {:>6} {:>6} {:>6} {:>9}\n",
                k.kind, k.items, k.fired, k.never, k.too_often
            ));
        }
    }
    let mut file = "";
    let mut infos = 0usize;
    const INFOS_PER_FILE: usize = 12;
    for f in &r.findings {
        if f.file != file {
            if infos > INFOS_PER_FILE {
                s.push_str(&format!(
                    "  … {} more infos (--json for all)\n",
                    infos - INFOS_PER_FILE
                ));
            }
            file = &f.file;
            infos = 0;
            s.push_str(&format!("\n{file}\n"));
        }
        if f.severity == Sev::Info {
            infos += 1;
            if infos > INFOS_PER_FILE {
                continue;
            }
        }
        s.push_str(&format!(
            "  {:<5} {:<14} {}: {}\n",
            f.severity.key(),
            f.check,
            f.item,
            f.message
        ));
    }
    if infos > INFOS_PER_FILE {
        s.push_str(&format!(
            "  … {} more infos (--json for all)\n",
            infos - INFOS_PER_FILE
        ));
    }
    s
}

/// `content doctor`: print the report; `Err` under `--strict` with errors.
pub fn run(root: &Path, args: &DoctorArgs) -> Result<(), String> {
    let sources = mandate_content::bundle::read_dir_sources(root)
        .map_err(|e| format!("reading {}: {e}", root.display()))?;
    if let Some(dir) = &args.check_corpus {
        let mut found = corpus_text_findings(dir)?;
        if let Ok((b, _)) = mandate_content::load(&sources) {
            found.extend(corpus_relevance_findings(dir, &b)?);
        }
        print_text_findings(&found);
        if args.strict && !found.is_empty() {
            return Err(format!("content doctor: {} text problem(s)", found.len()));
        }
        return Ok(());
    }
    if let Some(dir) = &args.corpus {
        let content = mandate_content::load(&sources)
            .map(|(b, _)| Arc::new(b))
            .map_err(|issues| {
                format!("{} content error(s); run `content validate`", issues.len())
            })?;
        let counts =
            crate::corpus::run(content.clone(), dir, args.seeds, args.years, &args.source)?;
        use std::io::Write;
        let _ = writeln!(
            std::io::stdout().lock(),
            "{}",
            serde_json::to_string_pretty(&counts).unwrap_or_default()
        );
        // What reads wrong in it (only when it was rendered).
        if dir.join("corpus.jsonl").exists() {
            let mut found = corpus_text_findings(dir)?;
            found.extend(corpus_relevance_findings(dir, &content)?);
            print_text_findings(&found);
        }
        return Ok(());
    }
    let report = diagnose(&sources, args);
    {
        use std::io::Write;
        let mut o = std::io::stdout().lock();
        let text = if args.json {
            serde_json::to_string_pretty(&report).unwrap_or_default()
        } else {
            human(&report)
        };
        let _ = writeln!(o, "{text}");
    }
    if args.strict && report.errors() > 0 {
        return Err(format!("content doctor: {} error(s)", report.errors()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::doctor::testkit::*;

    #[test]
    fn the_doctor_finds_planted_problems() {
        let r = diagnose(&planted(), &quick());
        let show = || human(&r);
        assert!(
            r.has("contradiction", "dilemma.planted_never"),
            "{}",
            show()
        );
        let missing = r.findings.iter().find(|f| {
            f.check == "fluent-missing" && f.message.contains("dilemma-planted-missing-after")
        });
        assert!(missing.is_some(), "{}", show());
        assert_eq!(missing.unwrap().severity, Sev::Error);
        let long = r
            .findings
            .iter()
            .find(|f| f.check == "too-long" && f.item == "overheard.planted_long");
        assert!(long.is_some(), "{}", show());
        // Findings are filed where the item is defined.
        assert_eq!(long.unwrap().file, "base/planted_overheard.jsonc");
        assert!(r.errors() >= 2);
    }

    #[test]
    fn the_base_pack_loads_and_its_static_checks_run() {
        let r = diagnose(&base_sources(), &quick());
        assert!(!r.content_hash.is_empty(), "{}", human(&r));
        assert!(
            !r.findings
                .iter()
                .any(|f| f.check == "validate" && f.severity == Sev::Error)
        );
    }
}
