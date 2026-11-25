//! Length checks: text too long for a phone, rendered with representative arguments.

use super::*;

/// A representative value for an argument: long names, a long town.
pub(super) fn sample_arg(name: &str) -> String {
    let n = name.trim_end_matches("_key");
    match n {
        "n" | "jobs" | "days" | "amount" | "count" | "votes" | "seats" | "weeks" | "months" => {
            "12,345".into()
        }
        "year" => "2031".into(),
        "town" | "hood" | "here" | "outlet_place" | "place" | "broker_place" => {
            "Saint-Aubin-des-Marais".into()
        }
        _ => "Bartholomew Okonkwo-Hale".into(),
    }
}

/// The longest rendering of a message (every selector's longest variant),
/// with representative arguments; `x_key` arguments render a message of
/// typical length.
pub(super) fn render(msgs: &BTreeMap<String, String>, text: &str, depth: u32) -> String {
    let mut out = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '{' {
            if chars[i] != '\n' {
                out.push(chars[i]);
            } else {
                out.push(' ');
            }
            i += 1;
            continue;
        }
        // The placeable up to its matching brace.
        let mut depth_b = 0;
        let mut j = i;
        while j < chars.len() {
            match chars[j] {
                '{' => depth_b += 1,
                '}' => {
                    depth_b -= 1;
                    if depth_b == 0 {
                        break;
                    }
                }
                _ => {}
            }
            j += 1;
        }
        let inner: String = chars[i + 1..j.min(chars.len())].iter().collect();
        out.push_str(&placeable(msgs, inner.trim(), depth));
        i = j + 1;
    }
    out
}

pub(super) fn placeable(msgs: &BTreeMap<String, String>, inner: &str, depth: u32) -> String {
    if depth > 6 {
        return String::new();
    }
    if let Some(arrow) = inner.find("->") {
        // A selector: the longest variant.
        let variants = &inner[arrow + 2..];
        let mut best = String::new();
        // Split on variant heads `[key]` at nesting level 0.
        let mut parts: Vec<String> = Vec::new();
        let chars: Vec<char> = variants.chars().collect();
        let mut k = 0;
        let mut lvl = 0;
        let mut start: Option<usize> = None;
        while k < chars.len() {
            match chars[k] {
                '{' => lvl += 1,
                '}' => lvl -= 1,
                '[' if lvl == 0 => {
                    if let Some(s) = start {
                        parts.push(chars[s..k].iter().collect());
                    }
                    // Skip the head.
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
        for p in parts {
            let p = p.trim().trim_end_matches('*').trim();
            let r = render(msgs, p, depth + 1);
            if r.chars().count() > best.chars().count() {
                best = r;
            }
        }
        return best;
    }
    if let Some(v) = inner.strip_prefix('$') {
        let name: String = v
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        return sample_arg(&name);
    }
    if let Some(s) = inner.strip_prefix('"') {
        return s.trim_end_matches('"').to_string();
    }
    let id: String = inner
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    if let Some(m) = msgs.get(&id) {
        return render(msgs, m, depth + 1);
    }
    // A function call (`NUMBER($n)`) or something else: a short number.
    "12,345".into()
}

pub(super) fn length_checks(b: &ContentBundle, args: &DoctorArgs, out: &mut Out) {
    let msgs = messages(&b.locale_text("en"));
    // `x_key` arguments are other messages: render $x as a typical one.
    let mut checks: Vec<(String, String, usize, &str)> = Vec::new();
    let title = |checks: &mut Vec<_>, item: &str, k: &str| {
        checks.push((item.to_string(), k.to_string(), args.max_title, "title"))
    };
    let label = |checks: &mut Vec<_>, item: &str, k: &str| {
        checks.push((
            item.to_string(),
            k.to_string(),
            args.max_label,
            "option label",
        ))
    };
    let line = |checks: &mut Vec<_>, item: &str, k: &str| {
        checks.push((item.to_string(), k.to_string(), args.max_line, "line"))
    };
    let body = |checks: &mut Vec<_>, item: &str, k: &str| {
        checks.push((item.to_string(), k.to_string(), args.max_body, "body"))
    };
    for d in &b.dilemmas {
        title(&mut checks, &d.id, &d.title_key);
        body(&mut checks, &d.id, &d.situation_key);
        for o in &d.options {
            label(&mut checks, &d.id, &o.label_key);
            body(&mut checks, &d.id, &o.aftermath_key);
        }
    }
    for sc in &b.scenes {
        for beat in &sc.beats {
            line(&mut checks, &sc.id, &beat.line);
        }
    }
    for o in &b.overheard {
        line(&mut checks, &o.id, &o.key);
    }
    for t in &b.talk_topics {
        title(&mut checks, &t.id, &t.name_key);
    }
    for l in &b.talk_lines {
        line(&mut checks, &l.id, &l.key);
        if let Some(lead) = &l.lead {
            line(&mut checks, &l.id, &lead.key);
        }
    }
    let mut seen = BTreeSet::new();
    for (item, key, max, what) in checks {
        if !seen.insert(key.clone()) {
            continue;
        }
        let Some(m) = msgs.get(&key) else { continue };
        let text = render(&msgs, m, 0);
        let n = text.chars().count();
        if n > max {
            out.add(
                Sev::Warn,
                "too-long",
                &item,
                format!(
                    "{what} `{key}` renders to {n} characters (limit {max}): \"{}…\"",
                    text.chars().take(60).collect::<String>()
                ),
            );
        }
    }
}

// --- Report ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn rendering_takes_the_longest_variant_and_references() {
        let msgs = messages(
            "a = { $n ->\n    [one] one\n   *[other] { $n } many many things\n}\nb = See { a } in { $town }\n",
        );
        let r = render(&msgs, msgs.get("b").unwrap(), 0);
        assert!(r.contains("many many things"), "{r}");
        assert!(r.contains("Saint-Aubin-des-Marais"), "{r}");
    }
}
