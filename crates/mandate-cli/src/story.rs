//! `story --metrics` (ENGAGEMENT §7.1): the director's decision and pacing
//! metrics per career, and their spread across seeds.
//!
//! A [`Collector`] watches a career turn by turn, because the narrative
//! logs are bounded (resolved decisions, facts, finished storylines): every
//! decision, spotlit fact, storyline run and press story about the player is
//! recorded the first time it is seen. [`Collector::finish`] reads the
//! chapters and the director's phase log at the end. The E2 gates of §7.2
//! are reported beside the numbers, never enforced here (E12 owns the gate).

use mandate_content::ContentBundle;
use mandate_sim::Engine;
use mandate_sim::narrative::{ChapterKind, PhaseKind, Weight};
use std::collections::{BTreeMap, BTreeSet};

/// One decision as first seen.
#[derive(Clone, Debug)]
struct Seen {
    template: String,
    weight: Weight,
    opened: u32,
    chapter: u32,
    raised: bool,
}

/// Everything a career's metrics need, gathered as the career is played.
#[derive(Default)]
pub struct Collector {
    decisions: BTreeMap<u32, Seen>,
    /// Spotlit storyline stages (dates).
    stages: Vec<u32>,
    /// Spotlit developments: news of the town, crises, goals, the rival.
    developments: Vec<u32>,
    next_fact: u32,
    /// Storyline runs started: run id → template.
    runs: BTreeMap<u32, String>,
    /// Press stories about the player: story id → date.
    press: BTreeMap<u32, u32>,
}

impl Collector {
    /// Record what is new since the last call.
    pub fn observe(&mut self, e: &Engine) {
        let st = e.state();
        let n = &st.narrative;
        for d in &n.decisions {
            self.decisions.entry(d.id.0).or_insert_with(|| Seen {
                template: d.template.clone(),
                weight: d.weight,
                opened: d.opened.0,
                chapter: d.chapter,
                raised: d.about.is_some(),
            });
        }
        for f in n.facts.iter().filter(|f| f.id >= self.next_fact) {
            use mandate_sim::narrative::FactKind as F;
            if !f.spotlit {
                continue;
            }
            match f.kind {
                F::StorylineStage { .. } => self.stages.push(f.date.0),
                F::TownEvent { .. }
                | F::CrisisBegan { .. }
                | F::CrisisWorse { .. }
                | F::GoalTaken { .. }
                | F::RivalNamed { .. } => self.developments.push(f.date.0),
                _ => {}
            }
        }
        self.next_fact = n.next_fact;
        for r in &n.storylines {
            self.runs
                .entry(r.id.0)
                .or_insert_with(|| r.template.clone());
        }
        let me = st.player.as_ref().map(|p| p.politician);
        for s in &st.media.stories {
            if s.subject.is_some() && s.subject == me {
                self.press.entry(s.id).or_insert(s.date.0);
            }
        }
    }

    /// The career's metrics (one row).
    pub fn finish(&self, e: &Engine, content: &ContentBundle, seed: u64) -> serde_json::Value {
        let st = e.state();
        let n = &st.narrative;
        let end = st.calendar.date.0;
        let began = n.play_began.map(|d| d.0).unwrap_or(end);
        let mut chapters: Vec<&mandate_sim::narrative::Chapter> = n.chapters.iter().collect();
        chapters.extend(n.chapter.as_ref());

        // --- Decisions and variety (raised decisions are fairness: apart).
        let spot: Vec<&Seen> = self.decisions.values().filter(|d| !d.raised).collect();
        let mut uses: BTreeMap<&str, u32> = BTreeMap::new();
        let mut per_chapter: BTreeSet<(u32, &str)> = BTreeSet::new();
        let mut repeats = 0u32;
        for d in &spot {
            *uses.entry(d.template.as_str()).or_default() += 1;
            if !per_chapter.insert((d.chapter, d.template.as_str())) {
                repeats += 1;
            }
        }
        let (top_template, most_uses) = uses
            .iter()
            .max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(a.0)))
            .map(|(k, v)| (k.to_string(), *v))
            .unwrap_or_default();
        // Climaxes, in chapter order.
        let mut climax_by_chapter: BTreeMap<u32, &str> = BTreeMap::new();
        for d in spot.iter().filter(|d| d.weight == Weight::Climax) {
            climax_by_chapter.entry(d.chapter).or_insert(&d.template);
        }
        let climax_seq: Vec<&str> = climax_by_chapter.values().copied().collect();
        let same_climax = climax_seq.windows(2).filter(|w| w[0] == w[1]).count();
        let climax_distinct: BTreeSet<&str> = climax_seq.iter().copied().collect();

        // --- Chapters: themes, peaks, budgets.
        let d = &content.director;
        let b = mandate_sim::narrative::director::budget(d, n.profile(content));
        let over_budget = chapters
            .iter()
            .filter(|c| c.climaxes > b.climax || c.significant > b.sig_max || c.minor > b.minor_max)
            .count();
        let sessions: Vec<_> = chapters
            .iter()
            .filter(|c| c.kind == ChapterKind::Session)
            .collect();
        let themes: Vec<serde_json::Value> = chapters
            .iter()
            .map(|c| serde_json::json!(c.theme.clone()))
            .collect();
        let mut same_theme = 0u32;
        for w in sessions.windows(2) {
            if w[0].theme.is_some() && w[0].theme == w[1].theme && w[1].theme_options > 1 {
                same_theme += 1;
            }
        }
        // Peaks per closed session chapter (the running one is partial).
        let closed_sessions: Vec<_> = n
            .chapters
            .iter()
            .filter(|c| c.kind == ChapterKind::Session)
            .collect();
        let peaks: Vec<u8> = closed_sessions.iter().map(|c| c.peaks).collect();
        let sig_per_chapter: Vec<u8> = closed_sessions
            .iter()
            .map(|c| c.significant + c.climaxes)
            .collect();

        // --- The pacing cycle (phase log).
        let mut relax_days = 0u32;
        let (mut relaxes, mut relax_beats) = (0u32, 0u32);
        for p in n.phase_log.iter().filter(|p| p.kind == PhaseKind::Relax) {
            let to = p.until.map(|u| u.0).unwrap_or(end);
            relax_days = relax_days.max(to.saturating_sub(p.since.0));
            relaxes += 1;
            if p.beats > 0 {
                relax_beats += 1;
            }
        }
        // Peaks that spotlighted nothing (the budget was spent, nothing
        // qualified, or the tension curve held it back).
        let peaks_all = n
            .phase_log
            .iter()
            .filter(|p| p.kind == PhaseKind::Peak)
            .count();
        let peaks_empty = n
            .phase_log
            .iter()
            .filter(|p| p.kind == PhaseKind::Peak && p.decision.is_none())
            .count();
        // Climaxes that came by the fallback (template cooldowns waived).
        let climax_fallbacks = n
            .phase_log
            .iter()
            .chain(n.phase.as_ref())
            .filter(|p| p.kind == PhaseKind::Peak && p.climax_fallback)
            .count();
        let relax_share = if relaxes == 0 {
            serde_json::Value::Null
        } else {
            serde_json::json!(relax_beats as f64 / relaxes as f64)
        };

        // --- Gaps: governing (session chapters) and campaign time, apart.
        let kind_on = |day: u32| -> ChapterKind {
            chapters
                .iter()
                .rev()
                .find(|c| c.start.0 <= day)
                .map(|c| c.kind)
                .unwrap_or(ChapterKind::Session)
        };
        // Intervals of one kind: [from, to).
        let mut spans: Vec<(ChapterKind, u32, u32)> = Vec::new();
        for c in &chapters {
            let from = c.start.0.max(began);
            let to = c.closed.map(|x| x.0).unwrap_or(end).min(end);
            if to <= from {
                continue;
            }
            match spans.last_mut() {
                Some(last) if last.0 == c.kind && last.2 >= from => last.2 = to,
                _ => spans.push((c.kind, from, to)),
            }
        }
        let gap = |beats: &[u32], kind: ChapterKind| -> u32 {
            let mut best = 0;
            for (k, from, to) in spans.iter().filter(|s| s.0 == kind) {
                let mut at: Vec<u32> = beats
                    .iter()
                    .copied()
                    .filter(|x| x >= from && x < to)
                    .collect();
                at.sort_unstable();
                let _ = k;
                let mut last = *from;
                for x in at {
                    best = best.max(x - last);
                    last = x;
                }
                best = best.max(to - last);
            }
            best
        };
        let opened: Vec<u32> = self.decisions.values().map(|d| d.opened).collect();
        let significant: Vec<u32> = self
            .decisions
            .values()
            .filter(|d| d.weight != Weight::Minor && !d.raised)
            .map(|d| d.opened)
            .collect();
        let press: Vec<u32> = self.press.values().copied().collect();
        // A beat (§7.1): a decision opened, a storyline stage, press about
        // the player (asks come with E6).
        let mut beats: Vec<u32> = opened.clone();
        beats.extend(&self.stages);
        beats.extend(&press);
        let mut beats_dev = beats.clone();
        beats_dev.extend(&self.developments);
        let _ = kind_on;

        let started: BTreeSet<&str> = self.runs.values().map(|s| s.as_str()).collect();
        serde_json::json!({
            "seed": seed,
            "decisions": self.decisions.len(),
            "raised": self.decisions.values().filter(|d| d.raised).count(),
            "distinct_templates": uses.len(),
            "most_uses": most_uses,
            "most_used": top_template,
            "repeats_in_chapter": repeats,
            "climaxes": climax_seq,
            "climax_distinct": climax_distinct.len(),
            "same_climax_consecutive": same_climax,
            "themes": themes,
            "same_theme_consecutive": same_theme,
            "peaks_per_session": peaks,
            "decisions_per_session": sig_per_chapter,
            "chapters": chapters.len(),
            "chapters_over_budget": over_budget,
            "peaks": peaks_all,
            "peaks_empty": peaks_empty,
            "climax_fallbacks": climax_fallbacks,
            "longest_relax": relax_days,
            "relax_phases": relaxes,
            "relax_with_minor_beat": relax_share,
            "gap_decisions_governing": gap(&opened, ChapterKind::Session),
            "gap_decisions_campaign": gap(&opened, ChapterKind::Campaign),
            "gap_significant_governing": gap(&significant, ChapterKind::Session),
            "gap_quiet_governing": gap(&beats, ChapterKind::Session),
            "gap_quiet_campaign": gap(&beats, ChapterKind::Campaign),
            "gap_quiet_with_developments_governing": gap(&beats_dev, ChapterKind::Session),
            "storyline_templates_started": started.len(),
            "storyline_runs": self.runs.len(),
            "press_about_player": self.press.len(),
            "uses": uses,
        })
    }
}

fn median(mut v: Vec<f64>) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    let m = v.len() / 2;
    if v.len().is_multiple_of(2) {
        (v[m - 1] + v[m]) / 2.0
    } else {
        v[m]
    }
}

/// Median, min and max of each numeric metric across the rows, and the E2
/// gates of ENGAGEMENT §7.2 (reported, not enforced).
pub fn summary(rows: &[serde_json::Value]) -> serde_json::Value {
    let num = |k: &str| -> Vec<f64> { rows.iter().filter_map(|r| r[k].as_f64()).collect() };
    let mut stats = serde_json::Map::new();
    for k in [
        "decisions",
        "distinct_templates",
        "most_uses",
        "repeats_in_chapter",
        "climax_distinct",
        "same_climax_consecutive",
        "same_theme_consecutive",
        "chapters_over_budget",
        "peaks",
        "peaks_empty",
        "climax_fallbacks",
        "longest_relax",
        "relax_with_minor_beat",
        "gap_decisions_governing",
        "gap_decisions_campaign",
        "gap_significant_governing",
        "gap_quiet_governing",
        "gap_quiet_campaign",
        "gap_quiet_with_developments_governing",
        "storyline_templates_started",
        "press_about_player",
    ] {
        let v = num(k);
        if v.is_empty() {
            continue;
        }
        let lo = v.iter().copied().fold(f64::INFINITY, f64::min);
        let hi = v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        stats.insert(
            k.into(),
            serde_json::json!({ "median": median(v), "min": lo, "max": hi }),
        );
    }
    let peaks: Vec<f64> = rows
        .iter()
        .flat_map(|r| {
            r["peaks_per_session"]
                .as_array()
                .cloned()
                .unwrap_or_default()
        })
        .filter_map(|x| x.as_f64())
        .collect();
    let all = |k: &str, f: &dyn Fn(f64) -> bool| num(k).into_iter().all(f);
    let med = |k: &str| median(num(k));
    let mut gates = serde_json::Map::new();
    let mut gate = |name: &str, ok: bool| {
        gates.insert(name.into(), serde_json::json!(ok));
    };
    gate("most_uses_le_3", all("most_uses", &|x| x <= 3.0));
    gate(
        "no_repeat_in_chapter",
        all("repeats_in_chapter", &|x| x == 0.0),
    );
    gate(
        "no_climax_in_consecutive_chapters",
        all("same_climax_consecutive", &|x| x == 0.0),
    );
    gate(
        "no_theme_in_consecutive_sessions",
        all("same_theme_consecutive", &|x| x == 0.0),
    );
    gate(
        "peaks_3_to_6_per_session",
        !peaks.is_empty() && peaks.iter().all(|p| (3.0..=6.0).contains(p)),
    );
    gate(
        "decision_gap_median_le_120",
        med("gap_decisions_governing") <= 120.0,
    );
    gate(
        "decision_gap_every_seed_le_145",
        all("gap_decisions_governing", &|x| x <= 145.0),
    );
    gate("within_budgets", all("chapters_over_budget", &|x| x == 0.0));
    let lo = peaks.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = peaks.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    serde_json::json!({
        "careers": rows.len(),
        "stats": stats,
        "peaks_per_session": { "min": lo, "max": hi, "median": median(peaks) },
        "e2_gates": gates,
    })
}
