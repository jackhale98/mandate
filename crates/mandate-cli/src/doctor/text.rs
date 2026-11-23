//! Text rules: doubled punctuation, stray braces, empty arguments.

use super::*;

/// What a reader would trip over in rendered text, as (check, what):
///
/// - `text-raw`: a placeable or a missing key left in the text (`{ $x }`,
///   `⟦key⟧`), or a stray brace;
/// - `text-empty-arg`: an argument that came out empty (two spaces, a space
///   before punctuation, empty brackets or quotes: "pressed them on  and");
/// - `text-doubled-punct`: two stops where one belongs ("..'", ".,'"),
///   quotation marks between them notwithstanding; an ellipsis ("...") and
///   "?!" are fine.
pub fn text_problems(text: &str) -> Vec<(&'static str, String)> {
    let mut out = Vec::new();
    let excerpt = |at: usize| -> String {
        let start = text[..at]
            .char_indices()
            .rev()
            .nth(24)
            .map(|(i, _)| i)
            .unwrap_or(0);
        let end = text[at..]
            .char_indices()
            .nth(24)
            .map(|(i, _)| at + i)
            .unwrap_or(text.len());
        format!("…{}…", &text[start..end])
    };
    if let Some(at) = text.find(['{', '}', '⟦']) {
        out.push(("text-raw", excerpt(at)));
    }
    const EMPTY: [&str; 13] = [
        "  ", " ,", " .", " ;", " :", " !", " ?", " )", "( ", "()", "''", "\"\"", "\" \"",
    ];
    if let Some(at) = EMPTY.iter().filter_map(|p| text.find(p)).min() {
        out.push(("text-empty-arg", excerpt(at)));
    } else if text.starts_with("\" ") || text.ends_with(" \"") || text.trim() != text {
        out.push(("text-empty-arg", excerpt(0)));
    }
    let stop = |c: char| ".,;:!?".contains(c);
    let quote = |c: char| "'\"‘’“”".contains(c);
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut prev: Option<(usize, char)> = None;
    for (k, &(at, c)) in chars.iter().enumerate() {
        if quote(c) {
            continue;
        }
        if stop(c)
            && let Some((_, p)) = prev
            && stop(p)
        {
            let ellipsis = c == '.' && p == '.' && {
                // Three or more dots in a row (quotes aside) is an ellipsis.
                let dots = chars
                    .iter()
                    .skip(k.saturating_sub(2))
                    .take(5)
                    .filter(|(_, x)| *x == '.')
                    .count();
                dots >= 3
            };
            let mixed = matches!((p, c), ('?', '!') | ('!', '?'));
            if !ellipsis && !mixed {
                out.push(("text-doubled-punct", excerpt(at)));
                break;
            }
        }
        prev = Some((at, c));
    }
    out
}

/// Every English message rendered with representative arguments (the
/// longest variant of each selector): doubled punctuation and stray braces
/// written into the text itself. (Empty arguments only show with real
/// arguments: the corpus check below.)
pub(super) fn text_checks(b: &ContentBundle, out: &mut Out) {
    let msgs = messages(&b.locale_text("en"));
    for (key, m) in &msgs {
        let text = render(&msgs, m, 0);
        for (check, what) in text_problems(&text) {
            if check == "text-empty-arg" {
                continue;
            }
            out.add(
                Sev::Warn,
                check,
                key,
                format!("`{key}` renders with {}: \"{what}\"", label(check)),
            );
        }
    }
}

pub(super) fn label(check: &str) -> &'static str {
    match check {
        "text-raw" => "a raw placeable or missing key",
        "text-empty-arg" => "an empty argument",
        "text-doubled-punct" => "doubled punctuation",
        _ => "a problem",
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::doctor::testkit::*;

    #[test]
    fn text_problems_find_empty_arguments() {
        // A memory with no subject, quoted as it was before the fix.
        let p = text_problems("You pressed them on  and got nowhere (2020).");
        assert!(p.iter().any(|(c, _)| *c == "text-empty-arg"), "{p:?}");
        assert!(
            text_problems("\" Put my ward in your bill.\"")
                .iter()
                .any(|(c, _)| *c == "text-empty-arg")
        );
        assert!(
            text_problems("Who supports : nobody")
                .iter()
                .any(|(c, _)| *c == "text-empty-arg")
        );
        assert!(text_problems("You pressed them on the bill and got nowhere (2020).").is_empty());
    }

    #[test]
    fn text_problems_find_doubled_punctuation_but_not_an_ellipsis() {
        // The retold headline before the fix: its own stop, then the line's.
        for bad in [
            "\"Did you see it? 'Mireenwick passed the bill (1–0)..' It's all anyone's on about.\"",
            "\"'Mireenwick passed the bill (1–0).,' says the Chronicle.\"",
            "Well,. there it is.",
        ] {
            assert!(
                text_problems(bad)
                    .iter()
                    .any(|(c, _)| *c == "text-doubled-punct"),
                "{bad}"
            );
        }
        for good in [
            "\"Sorry. Was that... sorry. I'll let you get on.\"",
            "\"Oh! Sorry,\" says Ada. \"Is this – are you after me?\"",
            "\"'Wage floor bill passes, 4–3,' says the Chronicle. Really?!\"",
            "\"Did you see it? 'Wage floor bill passes.' It's all anyone's on about.\"",
        ] {
            assert!(
                text_problems(good).is_empty(),
                "{good}: {:?}",
                text_problems(good)
            );
        }
    }

    #[test]
    fn text_problems_find_raw_placeables_and_missing_keys() {
        for bad in [
            "'Mireenwick passed {$policy} (1–0).'",
            "Hello { $name }",
            "A ⟦talk-missing⟧ line",
        ] {
            assert!(
                text_problems(bad).iter().any(|(c, _)| *c == "text-raw"),
                "{bad}"
            );
        }
    }

    #[test]
    fn doubled_punctuation_written_into_a_message_is_found() {
        let mut s = base_sources();
        s.push(Source {
            path: "base/planted_overheard.jsonc".into(),
            text: r#"{ "kind": "overheard", "items": [ { "id": "overheard.planted_stops", "key": "heard-planted-stops" } ] }"#.into(),
        });
        s.push(Source {
            path: "base/locale/en/planted.ftl".into(),
            text: "heard-planted-stops = \"They closed the library..\"\n".into(),
        });
        let r = diagnose(&s, &quick());
        assert!(
            r.findings
                .iter()
                .any(|f| f.check == "text-doubled-punct" && f.item == "heard-planted-stops"),
            "{}",
            human(&r)
        );
    }
}
