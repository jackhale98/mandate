//! Text rules: doubled punctuation, stray braces, empty arguments; and the
//! prose habits that read as machine-written (`prose-habit`, the style
//! bible's lint: `docs/design/STYLE_BIBLE.md`).

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

// --- Prose habits (the style bible) ---------------------------------------------------------

/// Stock words and phrases that mark text as machine-written, or as filler.
/// Lower case, matched on word boundaries. This table is meant to grow: when
/// a reviewer or the owner flags a phrase in a draft, add it here. Leave out
/// anything with an ordinary literal use in a town ("landscape", "journey",
/// "foster", "chapter"): a missed case is cheaper than a flagged good line.
pub const STOCK_PHRASES: &[&str] = &[
    "a testament to",
    "testament to the",
    "tapestry",
    "delve",
    "delving",
    "in the heart of",
    "nestled",
    "bustling",
    "vibrant",
    "palpable",
    "unwavering",
    "a stark reminder",
    "serves as a reminder",
    "a reminder that",
    "speaks volumes",
    "only time will tell",
    "at the end of the day",
    "a delicate balance",
    "navigate the",
    "navigating the",
    "the complexities of",
    "it's worth noting",
    "it is worth noting",
    "plays a crucial role",
    "a crucial role",
    "pivotal",
    "underscores",
    "the fabric of",
    "sense of community",
    "rich history",
    "deeply rooted",
    "resonates",
    "embark on",
    "embarked on",
    "a beacon of",
    "ever-changing",
    "ever-evolving",
    "in today's",
    "a myriad of",
    "multifaceted",
    "meticulous",
    "seamless",
    "showcasing",
    "fostering",
    "a whirlwind of",
    "hangs heavy",
    "hung heavy",
    "hangs in the air",
    "hung in the air",
    "couldn't help but",
    "can't help but",
    "little did",
    "make no mistake",
    "never been higher",
    "now more than ever",
    "in a world where",
    "game-changer",
    "game changer",
    "a perfect storm",
    "the elephant in the room",
    "going forward",
    "moving forward",
    "it goes without saying",
    "needless to say",
    "a double-edged sword",
    "the road ahead",
    "the weight of the world",
    "every voice matters",
    "come together as a community",
    // The pack's own reflexive closers (2026-11): an aftermath that ends on
    // someone's memory instead of what they did.
    "will not forget it",
    "won't forget it",
    "never forgets it",
    "will remember",
    "when it mattered",
    ", for now.",
];

/// How long a message of each kind should be, by key prefix (and suffix),
/// rendered with long sample names. Softer than `length_checks`' hard
/// limits: the style bible's targets plus room for long names. First match
/// wins.
const KIND_LIMITS: &[(&str, &str, usize, &str)] = &[
    ("headline-", "", 80, "headline"),
    ("talk-open-", "", 130, "opener"),
    ("talk-close-", "", 110, "closer"),
    ("talk-", "", 240, "conversation line"),
    ("heard-", "", 170, "overheard line"),
    ("scene-", "", 200, "scene beat"),
    ("story-", "", 180, "storyline step"),
    ("skip-item-", "", 110, "summary line"),
    ("legacy-", "", 110, "epilogue line"),
    ("dilemma-", "-situation", 460, "dilemma situation"),
    ("dilemma-", "-after", 300, "aftermath"),
    ("ask-", "-detail", 340, "ask"),
    ("ask-", "-kept", 170, "ask outcome"),
    ("ask-", "-broken", 170, "ask outcome"),
    ("ask-", "-need", 90, "ask step"),
];

/// The kind of text a key names, and its length target, when the key's
/// prefix says.
pub fn kind_limit(key: &str) -> Option<(usize, &'static str)> {
    KIND_LIMITS
        .iter()
        .find(|(p, s, _, _)| key.starts_with(p) && key.ends_with(s))
        .map(|(_, _, n, k)| (*n, *k))
}

/// A typical value for an argument, for the prose targets (the hard limits
/// use the long ones, `sample_arg`): a name of ordinary length, a town, a
/// small number, a policy title.
fn typical_arg(name: &str) -> String {
    let n = name.trim_end_matches("_key");
    match n {
        "n" | "jobs" | "days" | "amount" | "count" | "votes" | "seats" | "weeks" | "months"
        | "yes" | "no" | "years" | "turns" | "number" => "140".into(),
        "share" => "54%".into(),
        "year" => "2031".into(),
        "town" | "hood" | "outlet_place" | "asker_hood" | "constituency" => "Mireenwick".into(),
        "here" | "about" | "place" | "broker_place" => "the union hall".into(),
        "policy" | "bill" | "law" | "answer" => "Small business grants".into(),
        "concern_talk" => "the rent".into(),
        "fact" => "a cash bribe".into(),
        _ if n.ends_with("_role") => "Union steward".into(),
        _ => "Celia Janssenfield".into(),
    }
}

/// Every rendering of a message: each selector variant (capped), typical
/// arguments for variables, referenced messages expanded.
fn expand(msgs: &BTreeMap<String, String>, text: &str, depth: u32) -> Vec<String> {
    const CAP: usize = 48;
    let mut outs = vec![String::new()];
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '{' {
            let c = if chars[i] == '\n' { ' ' } else { chars[i] };
            for o in &mut outs {
                o.push(c);
            }
            i += 1;
            continue;
        }
        let mut level = 0;
        let mut j = i;
        while j < chars.len() {
            match chars[j] {
                '{' => level += 1,
                '}' => {
                    level -= 1;
                    if level == 0 {
                        break;
                    }
                }
                _ => {}
            }
            j += 1;
        }
        let inner: String = chars[i + 1..j.min(chars.len())].iter().collect();
        let inner = inner.trim();
        let fills: Vec<String> = if depth > 4 {
            vec![String::new()]
        } else if let Some(arrow) = inner.find("->") {
            variants(&inner[arrow + 2..])
                .iter()
                .flat_map(|v| expand(msgs, v, depth + 1))
                .collect()
        } else {
            let id: String = inner
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
                .collect();
            match msgs.get(&id) {
                Some(m) if !inner.starts_with('$') && !inner.starts_with('"') => {
                    expand(msgs, m, depth + 1)
                }
                _ => match inner.strip_prefix('$') {
                    Some(v) => vec![typical_arg(v.trim())],
                    None => vec![placeable(msgs, inner, depth)],
                },
            }
        };
        let mut next = Vec::new();
        'all: for o in &outs {
            for f in &fills {
                if next.len() >= CAP {
                    break 'all;
                }
                next.push(format!("{o}{f}"));
            }
        }
        outs = next;
        i = j + 1;
    }
    outs
}

/// A selector's variant bodies (`[key] body`, `*[other] body`).
fn variants(body: &str) -> Vec<String> {
    let chars: Vec<char> = body.chars().collect();
    let mut parts = Vec::new();
    let mut level = 0;
    let mut start: Option<usize> = None;
    let mut k = 0;
    while k < chars.len() {
        match chars[k] {
            '{' => level += 1,
            '}' => level -= 1,
            '[' if level == 0 => {
                if let Some(s) = start {
                    parts.push(chars[s..k].iter().collect::<String>());
                }
                while k < chars.len() && chars[k] != ']' {
                    k += 1;
                }
                start = Some(k + 1);
            }
            _ => {}
        }
        k += 1;
    }
    if let Some(s) = start {
        parts.push(chars[s.min(chars.len())..].iter().collect());
    }
    parts
        .into_iter()
        .map(|p| p.trim().trim_end_matches('*').trim().to_string())
        .collect()
}

/// The sentences of a text, split after `.`, `!`, `?` or `…` (closing
/// quotes stay with their sentence; an ellipsis does not end one).
fn sentences(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let chars: Vec<char> = text.chars().collect();
    for (k, &c) in chars.iter().enumerate() {
        cur.push(c);
        if ".!?…".contains(c) {
            let next = chars[k + 1..]
                .iter()
                .find(|x| !"'\"‘’“”".contains(**x))
                .copied();
            if next.is_none_or(|n| n.is_whitespace()) && !(c == '.' && next == Some('.')) {
                out.push(std::mem::take(&mut cur));
            }
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur);
    }
    out
}

/// A sentence's words (lower case, `’` as `'`) and the punctuation that
/// joins clauses (`,`, `;`, `:`, a dash), as tokens.
fn tokens(sentence: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut word = String::new();
    let flush = |word: &mut String, out: &mut Vec<String>| {
        let w = word.trim_matches('\'').to_string();
        if !w.is_empty() {
            out.push(w);
        }
        word.clear();
    };
    for c in sentence.chars() {
        let c = if c == '’' { '\'' } else { c };
        if c.is_alphanumeric() || c == '\'' || (c == '-' && !word.is_empty()) {
            word.extend(c.to_lowercase());
        } else {
            flush(&mut word, &mut out);
            if ",;:—–".contains(c) {
                out.push(c.to_string());
            }
        }
    }
    flush(&mut word, &mut out);
    out
}

fn is_joint(t: &str) -> bool {
    matches!(t, "," | ";" | ":" | "—" | "–")
}

/// `it's`, `that's`, `they're`, or `it is`, `this was`…: how many tokens
/// the subject and its verb take (0 if none).
fn copula(t: &[String]) -> usize {
    const ONE: [&str; 3] = ["it's", "that's", "they're"];
    const SUBJ: [&str; 4] = ["it", "this", "that", "they"];
    const BE: [&str; 4] = ["is", "was", "are", "were"];
    match t {
        [a, ..] if ONE.contains(&a.as_str()) => 1,
        [a, b, ..] if SUBJ.contains(&a.as_str()) && BE.contains(&b.as_str()) => 2,
        _ => 0,
    }
}

/// The contrast frames: "not just X, it's Y", "not only X but Y", "it's not
/// X, it's Y", "it isn't about X, it's about Y", "not a X but a Y", "not
/// about X but Y".
pub fn contrast_frame(sentence: &str) -> bool {
    let t = tokens(sentence);
    let is = |k: usize, words: &[&str]| t.get(k).is_some_and(|w| words.contains(&w.as_str()));
    for i in 0..t.len() {
        // not just / only / merely / simply … but | …, it's
        if t[i] == "not" && is(i + 1, &["just", "only", "merely", "simply"]) {
            for j in i + 2..t.len() {
                if t[j] == "but" || (is_joint(&t[j - 1]) && copula(&t[j..]) > 0) {
                    return true;
                }
            }
        }
        // it's not X, it's Y / it isn't X; it's Y
        let c = copula(&t[i..]);
        let negated = (c > 0 && is(i + c, &["not"]))
            || (is(i, &["it", "this", "that", "they"])
                && is(i + 1, &["isn't", "wasn't", "aren't", "weren't"]));
        if negated && (i == 0 || is_joint(&t[i - 1]) || t[i - 1] == "and") {
            for j in i + 2..t.len() {
                let after = copula(&t[j..]);
                if is_joint(&t[j - 1]) && after > 0 && !is(j + after, &["not"]) {
                    return true;
                }
            }
        }
        // not a X but a Y, within a few words
        let but_a = |j: usize| t[j] == "but" && is(j + 1, &["a", "an"]);
        if t[i] == "not" && is(i + 1, &["a", "an"]) && (i + 2..(i + 7).min(t.len())).any(but_a) {
            return true;
        }
        // not about X but (about) Y
        if t[i] == "not" && is(i + 1, &["about"]) && t[i + 2..].iter().any(|w| w == "but") {
            return true;
        }
    }
    false
}

/// Lists of three or more short items in one sentence, the last joined by
/// "and" ("the keys, the kettle and the grievances"): how many.
pub fn triplets(sentence: &str) -> usize {
    // Items are the comma-separated stretches. The first is the tail of
    // whatever precedes it; the middle and last items must be short, so a
    // run of clauses is not taken for a list.
    const SHORT: usize = 6;
    let t = tokens(sentence);
    let segs: Vec<&[String]> = t.split(|w| w == ",").collect();
    let mut found = 0;
    let mut k = 0;
    // A clause, not an item, starts with its subject: "…, and they need".
    const SUBJECTS: [&str; 10] = [
        "i", "you", "he", "she", "it", "we", "they", "there", "that", "this",
    ];
    let clause = |s: &[String]| s.first().is_some_and(|w| SUBJECTS.contains(&w.as_str()));
    while k + 1 < segs.len() {
        // The first item: what follows the last colon or dash, and short.
        let first = segs[k].rsplit(|w| is_joint(w)).next().unwrap_or_default();
        if first.is_empty() || first.len() > SHORT {
            k += 1;
            continue;
        }
        let mut m = k + 1;
        let mut items = 1;
        let mut closed = false;
        while m < segs.len() {
            let s = segs[m];
            if s.is_empty() || s.len() > SHORT + 1 || s.iter().any(|w| is_joint(w)) {
                break;
            }
            if s[0] == "and" {
                // The Oxford comma: ", and Y" after at least two items.
                closed = items >= 2 && s.len() > 1 && s.len() <= SHORT && !clause(&s[1..]);
                items += 1;
                break;
            }
            if let Some(a) = s.iter().position(|w| w == "and") {
                // "X, Y and Z": Y and Z both short and present.
                closed = a > 0
                    && a + 1 < s.len()
                    && a <= SHORT
                    && s.len() - a - 1 <= SHORT
                    && !clause(&s[a + 1..]);
                items += 2;
                break;
            }
            if s.len() > SHORT {
                break;
            }
            items += 1;
            m += 1;
        }
        if closed && items >= 3 {
            found += 1;
            k = m + 1;
        } else {
            k += 1;
        }
    }
    found
}

/// An exclamation mark outside double-quoted speech (narration, headlines,
/// summaries, labels).
pub fn bang_outside_speech(text: &str) -> bool {
    let mut inside = false;
    for c in text.chars() {
        match c {
            '"' => inside = !inside,
            '“' => inside = true,
            '”' => inside = false,
            '!' if !inside => return true,
            _ => {}
        }
    }
    false
}

/// The prose habits a message shows in any of its renderings, as short
/// descriptions. Every rule prefers missing a case to flagging a good line.
pub fn prose_habits(key: &str, renderings: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut push = |s: String| {
        if !out.contains(&s) {
            out.push(s);
        }
    };
    let boundary = |c: Option<char>| c.is_none_or(|c| !c.is_alphanumeric());
    for text in renderings {
        let lower = text.to_lowercase().replace('’', "'");
        for p in STOCK_PHRASES {
            let mut from = 0;
            while let Some(at) = lower[from..].find(p) {
                let (s, e) = (from + at, from + at + p.len());
                // A word boundary at each end that is a letter; a phrase
                // that starts or ends with punctuation carries its own.
                let word_at = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric());
                let left = !word_at(p.chars().next()) || boundary(lower[..s].chars().next_back());
                let right = !word_at(p.chars().last()) || boundary(lower[e..].chars().next());
                if left && right {
                    push(format!("stock phrase \"{p}\""));
                    break;
                }
                from = e;
            }
        }
        let ss = sentences(text);
        if let Some(s) = ss.iter().find(|s| contrast_frame(s)) {
            push(format!(
                "contrast frame (\"not X, but Y\"): \"{}\"",
                s.trim()
            ));
        }
        let lists: usize = ss.iter().map(|s| triplets(s)).sum();
        if lists >= 2 {
            push(format!("{lists} lists of three in one message"));
        }
        let dashes = text.matches('—').count() + text.matches(" – ").count();
        if dashes > 1 {
            push(format!("{dashes} dashes in one message"));
        }
        if bang_outside_speech(text) {
            push("an exclamation mark outside speech".into());
        }
        if let Some((limit, kind)) = kind_limit(key) {
            let n = text.chars().count();
            if n > limit {
                push(format!(
                    "long for a {kind}: {n} characters (target {limit})"
                ));
            }
        }
    }
    out
}

/// `prose-habit`: every English message, in every variant, against the
/// habits above. Info (a report for writers), or warn under
/// `--strict-prose`.
pub(super) fn prose_checks(b: &ContentBundle, args: &DoctorArgs, out: &mut Out) {
    let sev = if args.strict_prose {
        Sev::Warn
    } else {
        Sev::Info
    };
    let msgs = messages(&b.locale_text("en"));
    for (key, m) in &msgs {
        let renderings = expand(&msgs, m, 0);
        for what in prose_habits(key, &renderings) {
            out.add(sev, "prose-habit", key, format!("`{key}`: {what}"));
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

    fn habits(key: &str, text: &str) -> Vec<String> {
        prose_habits(key, &[text.to_string()])
    }

    #[test]
    fn stock_phrases_match_whole_words_only() {
        let p = habits(
            "talk-x",
            "\"In the heart of Mireenwick, a vibrant market.\"",
        );
        assert!(p.iter().any(|h| h.contains("in the heart of")), "{p:?}");
        assert!(p.iter().any(|h| h.contains("vibrant")), "{p:?}");
        // "delve" inside another word, "bustling" as a name: not flagged.
        assert!(habits("talk-x", "\"Ask at Bustlingham. The delved-out quarry.\"").is_empty());
        assert!(habits("talk-x", "\"The Vibrantes run the chip shop.\"").is_empty());
        // A phrase edged with punctuation carries its own boundary.
        assert!(!habits("story-x", "The pact held, for now.").is_empty());
        assert!(habits("ui-x", "Called for now: the floods.").is_empty());
    }

    #[test]
    fn contrast_frames_are_found() {
        for bad in [
            "It's not just about the rent, it's about respect.",
            "This is not only a budget but a promise.",
            "It isn't the money; it's the principle.",
            "It's not a crisis but a choice.",
            "That was not about jobs but about pride.",
            "We want not merely talk, but a bill.",
        ] {
            assert!(contrast_frame(bad), "{bad}");
        }
        for good in [
            "I'm not sure, but I think the bus goes at six.",
            "It's not my department, so ask at the desk.",
            "Not tonight. It's raining.",
            "We could use your hands, not just your sympathy.",
            "It's not somewhere I spend my days.",
            "That's not much, is it?",
            "It isn't open on Sundays, and it's shut by four the rest of the week.",
        ] {
            assert!(!contrast_frame(good), "{good}");
        }
        // ", it's not" in the second clause does not complete the frame.
        assert!(!contrast_frame("It isn't fair, it's not right either."));
    }

    #[test]
    fn lists_of_three_are_counted_and_flagged_only_in_pairs() {
        assert_eq!(triplets("I've the keys, the kettle and the grievances."), 1);
        assert_eq!(triplets("Tired, frightened, and angry."), 1);
        assert_eq!(triplets("Bread and butter."), 0);
        // A run of clauses is not a list.
        assert_eq!(
            triplets(
                "If you're asking what keeps people up at night in the east end, it's the rent, and nobody will say otherwise to your face."
            ),
            0
        );
        let one = habits("talk-x", "\"The keys, the kettle and the grievances.\"");
        assert!(!one.iter().any(|h| h.contains("lists of three")), "{one:?}");
        let two = habits(
            "talk-x",
            "\"They are tired, frightened and angry. They want answers, action and accountability.\"",
        );
        assert!(
            two.iter().any(|h| h.contains("2 lists of three")),
            "{two:?}"
        );
    }

    #[test]
    fn dashes_and_exclamations_and_length_by_kind() {
        let p = habits("story-x", "The mayor — never shy — arrived — early.");
        assert!(p.iter().any(|h| h.contains("3 dashes")), "{p:?}");
        assert!(habits("story-x", "Is this – are you after me?").is_empty());
        assert!(
            habits("skip-item-x", "Your bill passed!")
                .iter()
                .any(|h| h.contains("exclamation"))
        );
        // Inside speech an exclamation is a speaker's.
        assert!(habits("talk-x", "{ x } starts. \"Oh! Sorry.\"").is_empty());
        let long = "A ".repeat(60);
        assert!(
            habits("headline-x", &long)
                .iter()
                .any(|h| h.contains("long for a headline"))
        );
        // A key whose prefix names no kind has no target.
        assert!(habits("ui-x-help", &long).is_empty());
        assert_eq!(
            kind_limit("dilemma-x-situation").map(|k| k.1),
            Some("dilemma situation")
        );
        assert_eq!(kind_limit("dilemma-x-label"), None);
    }

    #[test]
    fn every_variant_of_a_selector_is_read() {
        let mut msgs = BTreeMap::new();
        msgs.insert("other".to_string(), "a vibrant town".to_string());
        let text = "{ $kind ->\n [a] Quiet.\n *[b] Your bill passed!\n} See { other }.";
        let r = expand(&msgs, text, 0);
        assert_eq!(r.len(), 2, "{r:?}");
        let h = prose_habits("skip-item-x", &r);
        assert!(h.iter().any(|x| x.contains("exclamation")), "{h:?}");
        assert!(h.iter().any(|x| x.contains("vibrant")), "{h:?}");
    }

    #[test]
    fn prose_habits_are_infos_or_warnings_under_strict_prose() {
        let mut s = base_sources();
        s.push(Source {
            path: "base/planted_overheard.jsonc".into(),
            text: r#"{ "kind": "overheard", "items": [ { "id": "overheard.planted_prose", "key": "heard-planted-prose" } ] }"#.into(),
        });
        s.push(Source {
            path: "base/locale/en/planted.ftl".into(),
            text:
                "heard-planted-prose = \"It's not just the rent, it's the tapestry of it all.\"\n"
                    .into(),
        });
        let r = diagnose(&s, &quick());
        let found: Vec<_> = r
            .findings
            .iter()
            .filter(|f| f.check == "prose-habit" && f.item == "heard-planted-prose")
            .collect();
        assert_eq!(found.len(), 2, "{}", human(&r));
        assert!(found.iter().all(|f| f.severity == Sev::Info));
        let strict = DoctorArgs {
            strict_prose: true,
            ..quick()
        };
        let r = diagnose(&s, &strict);
        assert!(
            r.findings
                .iter()
                .filter(|f| f.check == "prose-habit")
                .all(|f| f.severity == Sev::Warn)
        );
        // Prose habits never fail `--strict`.
        assert!(
            !r.findings
                .iter()
                .any(|f| f.check == "prose-habit" && f.severity == Sev::Error)
        );
    }

    /// The base pack's prose census (not a gate): `cargo test -p mandate-cli
    /// prose_census -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn prose_census() {
        let b = mandate_content::load(&base_sources()).unwrap().0;
        let msgs = messages(&b.locale_text("en"));
        let mut one_list = Vec::new();
        for (key, m) in &msgs {
            for r in expand(&msgs, m, 0) {
                let n: usize = sentences(&r).iter().map(|s| triplets(s)).sum();
                if n == 1 {
                    one_list.push(format!("{key}: {r}"));
                    break;
                }
            }
        }
        println!(
            "messages with exactly one list of three: {}",
            one_list.len()
        );
        for l in one_list.iter().take(12) {
            println!("  {l}");
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
