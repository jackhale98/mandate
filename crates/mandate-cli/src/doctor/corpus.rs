//! Corpus checks: rendered corpora read for text problems and subject relevance.

use super::*;

/// The text checks on a rendered corpus (`--corpus` writes one, and
/// `--check-corpus` reads one back): every item's text through
/// `text_problems`, one finding per check and template, with how many
/// renderings had it and the first one.
pub fn corpus_text_findings(dir: &Path) -> Result<Vec<Finding>, String> {
    let path = dir.join("corpus.jsonl");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut seen: BTreeMap<(String, String), (usize, String, String)> = BTreeMap::new();
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let v: serde_json::Value =
            serde_json::from_str(line).map_err(|e| format!("{}: {e}", path.display()))?;
        let s = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
        let item = format!("{} {}", s("kind"), s("id"));
        for (check, what) in text_problems(&s("text")) {
            let e = seen
                .entry((check.to_string(), item.clone()))
                .or_insert_with(|| (0, s("key"), what));
            e.0 += 1;
        }
    }
    Ok(seen
        .into_iter()
        .map(|((check, item), (n, key, what))| Finding {
            severity: Sev::Warn,
            message: format!(
                "{n} rendering(s) of `{key}` with {}, e.g. \"{what}\"",
                label(&check)
            ),
            check,
            file: "corpus".into(),
            item,
        })
        .collect())
}

/// Replies that answer the question (owner report 2026-11-20: "npcs respond
/// with answers that aren't really relevant"), statically:
///
/// - `subject-unread`: a line of a topic that is always about something
///   (`about_required`, an `inform` topic: "Ask about…") whose conditions
///   never read the subject. It can win over the subject's own lines and
///   answer something else. Fallbacks are deflections and exempt.
/// - `subject-drift`: a line of a topic that can be about a place, reading
///   the subject only as a neighbourhood's worries (`hood_concerns` of
///   `about`, which a place resolves to its neighbourhood) without an
///   `about` condition that keeps places out: asked about the paper, it
///   answers with the cost of living.
/// - `subject-stranger`: a line for a neighbourhood the speaker does not
///   live in (`home: false`) that does not also rule out the other ties
///   (`tied: false`, or `office`/`works` given): "I'm never over that way"
///   would be said by the mayor of the town, or by someone who works there
///   (M6 note 19).
pub(super) fn relevance_checks(b: &ContentBundle, out: &mut Out) {
    use mandate_content::conversation::{AboutKind, HoodOf, SubjectRef, TalkMechanic, reads_about};
    for l in &b.talk_lines {
        if l.part != TalkPart::Substance || l.fallback {
            continue;
        }
        let stranger = l.when.iter().any(|c| {
            matches!(
                c,
                Condition::About {
                    home: Some(false),
                    office: None,
                    works: None,
                    tied: None,
                    is: true,
                    ..
                }
            )
        });
        if stranger {
            out.add(
                Sev::Warn,
                "subject-stranger",
                &l.id,
                format!(
                    "`{}` is for a neighbourhood the speaker does not live in, but not for one they have no tie to; add `\"tied\": false` (or say `office`/`works`)",
                    l.id
                ),
            );
        }
        let Some(t) = l
            .topic
            .as_deref()
            .and_then(|t| b.talk_topics.iter().find(|x| x.id == t))
        else {
            continue;
        };
        if t.mechanic != TalkMechanic::Inform || t.about.is_empty() {
            continue;
        }
        if t.about_required && !reads_about(&l.when) {
            out.add(
                Sev::Warn,
                "subject-unread",
                &l.id,
                format!(
                    "`{}` answers `{}` (always about something) without reading the subject",
                    l.id, t.id
                ),
            );
        }
        let hood_only = l.when.iter().any(|c| {
            matches!(
                c,
                Condition::Tells {
                    subject: SubjectRef::HoodConcerns { of: HoodOf::About },
                    ..
                }
            )
        });
        let keeps_out = l.when.iter().any(|c| match c {
            Condition::About {
                kinds, is: true, ..
            } => !kinds.is_empty() && !kinds.contains(&AboutKind::Place),
            _ => false,
        });
        if t.about.contains(&AboutKind::Place) && hood_only && !keeps_out {
            out.add(
                Sev::Warn,
                "subject-drift",
                &l.id,
                format!(
                    "`{}` reads a place subject as its neighbourhood's worries; add `{{ \"type\": \"about\", \"kinds\": [\"hood\"] }}`",
                    l.id
                ),
            );
        }
    }
}

/// The same in a rendered corpus (`talk-off-subject`): a reply to "Ask
/// about…", or to "What are people worried about?" asked of a
/// neighbourhood, must name what was asked about (`$about`: a person, the
/// rival, a neighbourhood, a place), unless it comes from a fallback (a
/// deflection). Bills and templates are named by their own arguments and
/// are not checked here.
pub fn corpus_relevance_findings(dir: &Path, b: &ContentBundle) -> Result<Vec<Finding>, String> {
    let path = dir.join("corpus.jsonl");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let msgs = messages(&b.locale_text("en"));
    let fallback: BTreeSet<&str> = b
        .talk_lines
        .iter()
        .filter(|l| l.fallback)
        .map(|l| l.id.as_str())
        .collect();
    let mut seen: BTreeMap<String, (usize, String, String)> = BTreeMap::new();
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let v: serde_json::Value =
            serde_json::from_str(line).map_err(|e| format!("{}: {e}", path.display()))?;
        let s = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
        if s("kind") != "talk" || s("part") != "substance" || fallback.contains(s("id").as_str()) {
            continue;
        }
        let topic = v
            .pointer("/situation/topic")
            .and_then(|x| x.as_str())
            .unwrap_or("");
        let about = v.pointer("/situation/about");
        let hood_asked = about.and_then(|a| a.get("hood")).is_some()
            && about.and_then(|a| a.get("place")).is_none();
        if !(topic == "topic.about" || (topic == "topic.worries" && hood_asked)) {
            continue;
        }
        let args = v.get("args");
        let arg = |k: &str| args.and_then(|a| a.get(k)).and_then(|x| x.as_str());
        let named = match (arg("about"), arg("about_key")) {
            (Some(n), _) => n.to_string(),
            (None, Some(k)) => msgs.get(k).cloned().unwrap_or_default(),
            (None, None) => continue,
        };
        if named.is_empty() || s("text").to_lowercase().contains(&named.to_lowercase()) {
            continue;
        }
        let e = seen
            .entry(s("id"))
            .or_insert_with(|| (0, s("key"), format!("asked about {named}: {}", s("text"))));
        e.0 += 1;
    }
    Ok(seen
        .into_iter()
        .map(|(id, (n, key, what))| Finding {
            severity: Sev::Warn,
            check: "talk-off-subject".into(),
            file: "corpus".into(),
            item: format!("talk {id}"),
            message: format!("{n} reply(ies) from `{key}` do not name what was asked, e.g. {what}"),
        })
        .collect())
}

// --- Length -------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::doctor::testkit::*;

    #[test]
    fn a_corpus_is_checked_for_how_it_reads() {
        let dir = std::env::temp_dir().join(format!("doctor-corpus-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let line = |id: &str, text: &str| {
            serde_json::json!({ "kind": "scene", "part": "beat", "id": id, "text": text, "key": "k" })
                .to_string()
        };
        std::fs::write(
            dir.join("corpus.jsonl"),
            [
                line(
                    "scene.donor_offer",
                    "\"You pressed them on  and got nowhere (2020).\"",
                ),
                line(
                    "scene.donor_offer",
                    "\"You pressed them on  and got nowhere (2021).\"",
                ),
                line(
                    "scene.fine",
                    "\"You leaned on me once, and it didn't work.\"",
                ),
            ]
            .join("\n"),
        )
        .unwrap();
        let found = corpus_text_findings(&dir).unwrap();
        std::fs::remove_dir_all(&dir).ok();
        assert_eq!(found.len(), 1, "{found:#?}");
        assert_eq!(found[0].check, "text-empty-arg");
        assert_eq!(found[0].item, "scene scene.donor_offer");
        assert!(
            found[0].message.starts_with("2 rendering(s)"),
            "{}",
            found[0].message
        );
    }

    #[test]
    fn replies_that_ignore_the_subject_are_found() {
        let mut s = base_sources();
        s.push(Source {
            path: "base/planted_lines.jsonc".into(),
            text: r#"{ "kind": "talk_lines", "items": [
  { "id": "line.planted.about_anything", "part": "substance", "topic": "topic.about",
    "key": "talk-planted-anything",
    "when": [ { "type": "opinion", "slot": "speaker", "min": -15 } ] },
  { "id": "line.planted.about_hood_drift", "part": "substance", "topic": "topic.about",
    "key": "talk-planted-drift",
    "when": [ { "type": "tells", "subject": { "type": "hood_concerns", "of": "about" }, "min": "known" } ] },
  { "id": "line.planted.about_hood_ok", "part": "substance", "topic": "topic.about",
    "key": "talk-planted-drift",
    "when": [ { "type": "about", "kinds": ["hood"] },
              { "type": "tells", "subject": { "type": "hood_concerns", "of": "about" }, "min": "known" } ] },
  { "id": "line.planted.never_there", "part": "substance", "topic": "topic.worries",
    "key": "talk-planted-anything",
    "when": [ { "type": "about", "kinds": ["hood"], "home": false } ] },
  { "id": "line.planted.never_there_ok", "part": "substance", "topic": "topic.worries",
    "key": "talk-planted-anything",
    "when": [ { "type": "about", "kinds": ["hood"], "home": false, "tied": false } ] }
] }"#
                .into(),
        });
        s.push(Source {
            path: "base/locale/en/planted.ftl".into(),
            text: "talk-planted-anything = \"Lovely weather.\"\n\
                   talk-planted-drift = \"Round { $hood } it's { $concern_talk }.\"\n"
                .into(),
        });
        let r = diagnose(&s, &quick());
        assert!(
            r.has("subject-unread", "line.planted.about_anything"),
            "{}",
            human(&r)
        );
        assert!(
            r.has("subject-drift", "line.planted.about_hood_drift"),
            "{}",
            human(&r)
        );
        assert!(!r.has("subject-drift", "line.planted.about_hood_ok"));
        // "Never over that way", said by the mayor of the place (M6 note 19).
        assert!(
            r.has("subject-stranger", "line.planted.never_there"),
            "{}",
            human(&r)
        );
        assert!(!r.has("subject-stranger", "line.planted.never_there_ok"));
        // The base pack answers what it is asked.
        let base = diagnose(&base_sources(), &quick());
        let off: Vec<_> = base
            .findings
            .iter()
            .filter(|f| {
                f.check == "subject-unread"
                    || f.check == "subject-drift"
                    || f.check == "subject-stranger"
            })
            .collect();
        assert!(off.is_empty(), "{off:#?}");
    }

    /// The owner's report (2026-11-20): asked about the newspaper office, a
    /// reporter talked about the cost of living. In a corpus, a reply to
    /// "Ask about…" that does not name the place is found; a fallback (a
    /// deflection) is not.

    #[test]
    fn a_corpus_reply_that_does_not_name_the_place_asked_about_is_found() {
        let dir = std::env::temp_dir().join(format!("doctor-relevance-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let item = |id: &str, text: &str| {
            serde_json::json!({
                "kind": "talk", "part": "substance", "id": id, "text": text, "key": "k",
                "args": { "about_key": "talk-place-news_office", "speaker": "Ada Moss" },
                "situation": { "topic": "topic.about", "about": { "hood": "Old Quay", "place": "news_office" } }
            })
            .to_string()
        };
        std::fs::write(
            dir.join("corpus.jsonl"),
            [
                item(
                    "line.about.hood",
                    "\"Old Quay's my end of town. It's the rent.\"",
                ),
                item(
                    "line.about.place.own.press",
                    "\"You mean the newspaper office? Two desks, one kettle.\"",
                ),
                item("line.about.shrug", "\"Can't say I know much about that.\""),
            ]
            .join("\n"),
        )
        .unwrap();
        let b = mandate_content::load(&base_sources()).unwrap().0;
        let found = corpus_relevance_findings(&dir, &b).unwrap();
        std::fs::remove_dir_all(&dir).ok();
        assert_eq!(found.len(), 1, "{found:#?}");
        assert_eq!(found[0].item, "talk line.about.hood");
    }
}
