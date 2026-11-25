//! Fluent checks: keys content or the engine names, missing, unused or unsupplied.

use super::*;

/// Fluent keys content references, with the item that references each.
pub(super) fn content_keys(b: &ContentBundle) -> BTreeMap<String, String> {
    fn looks(s: &str) -> bool {
        s.contains('-')
            && s.starts_with(|c: char| c.is_ascii_lowercase())
            && s.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
    }
    fn walk(v: &serde_json::Value, owner: &str, field: &str, out: &mut BTreeMap<String, String>) {
        match v {
            serde_json::Value::Object(m) => {
                let id = m
                    .get("id")
                    .and_then(|x| x.as_str())
                    .filter(|s| s.contains('.'))
                    .unwrap_or(owner)
                    .to_string();
                for (k, x) in m {
                    walk(x, &id, k, out);
                }
            }
            serde_json::Value::Array(xs) => {
                for x in xs {
                    walk(x, owner, field, out);
                }
            }
            serde_json::Value::String(s) => {
                let keyish = field.ends_with("_key")
                    || field.ends_with("_keys")
                    || matches!(field, "key" | "line" | "quote_fallback");
                if keyish && looks(s) {
                    out.entry(s.clone()).or_insert_with(|| owner.to_string());
                }
            }
            _ => {}
        }
    }
    let mut v = serde_json::to_value(b).unwrap_or_default();
    if let Some(m) = v.as_object_mut() {
        m.remove("locales");
        m.remove("content_hash");
        // Presentation registries name pixel-art keys, not Fluent keys.
        for k in ["town_districts", "terrain_features", "connectors"] {
            m.remove(k);
        }
    }
    let mut out = BTreeMap::new();
    walk(&v, "content", "", &mut out);
    out
}

/// A Fluent message: its value text (continuation lines included,
/// attributes left out).
pub(super) fn messages(ftl: &str) -> BTreeMap<String, String> {
    let mut out: BTreeMap<String, String> = BTreeMap::new();
    let mut cur: Option<String> = None;
    for line in ftl.lines() {
        let first = line.chars().next();
        if first.is_some_and(|c| c.is_ascii_alphabetic() || c == '-')
            && let Some((id, rest)) = line.split_once('=')
            && id
                .trim()
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            let id = id.trim().to_string();
            out.insert(id.clone(), rest.trim().to_string());
            cur = Some(id);
            continue;
        }
        if line.starts_with(' ') || line.starts_with('\t') {
            if line.trim_start().starts_with('.') {
                // An attribute: not part of the value.
                cur = None;
                continue;
            }
            if let Some(id) = &cur
                && let Some(v) = out.get_mut(id)
            {
                v.push('\n');
                v.push_str(line.trim());
            }
            continue;
        }
        cur = None;
    }
    out
}

/// Message references (`{ other-key }`, `{ -term }`) in a value.
pub(super) fn references(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(i) = rest.find('{') {
        rest = &rest[i + 1..];
        let t = rest.trim_start();
        let id: String = t
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        if !id.is_empty() && !t.starts_with('$') {
            out.push(id);
        }
    }
    out
}

/// Quoted identifier-like strings in a source file (`"talk-runs-self"`,
/// `'ui-yes'`), wherever they sit, even inside a template literal; code
/// only (Rust test modules and line comments are skipped). As (string, line).
pub(super) fn literals(text: &str, rust: bool) -> Vec<(String, usize)> {
    let mut out = Vec::new();
    let word = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_';
    for (n, line) in text.lines().enumerate() {
        if rust && line.trim_start().starts_with("#[cfg(test)]") {
            break;
        }
        if line.trim_start().starts_with("//") {
            continue;
        }
        let cs: Vec<char> = line.chars().collect();
        for i in 0..cs.len() {
            let q = cs[i];
            if !(q == '"' || (!rust && (q == '\'' || q == '`'))) {
                continue;
            }
            let s: String = cs[i + 1..].iter().take_while(|c| word(**c)).collect();
            if !s.is_empty() && cs.get(i + 1 + s.chars().count()) == Some(&q) {
                out.push((s, n + 1));
            }
        }
    }
    out
}

pub(super) fn source_files(dir: &Path, exts: &[&str], out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).collect();
    entries.sort();
    for p in entries {
        let name = p.file_name().and_then(|x| x.to_str()).unwrap_or("");
        if p.is_dir() {
            // Generated types, builds, and the dev tools themselves.
            if !matches!(
                name,
                "node_modules" | "pkg" | "protocol" | "dev" | "fixtures" | "target"
            ) {
                source_files(&p, exts, out);
            }
        } else if exts.iter().any(|x| name.ends_with(x)) && !name.contains(".test.") {
            out.push(p);
        }
    }
}

pub(super) struct SourceScan {
    /// Literals that are Fluent-key shaped, with where (Rust only).
    engine: Vec<(String, String)>,
    /// Every literal (Rust and TypeScript).
    all: BTreeSet<String>,
    /// Static prefixes of keys built at run time (`fact-{…}`, `` `place-${…}` ``).
    prefixes: BTreeSet<String>,
}

pub(super) fn scan_sources(root: &Path) -> Option<SourceScan> {
    if root.as_os_str().is_empty() || !root.join("crates").is_dir() {
        return None;
    }
    let mut rs = Vec::new();
    for c in [
        "mandate-sim",
        "mandate-protocol",
        "mandate-content",
        "mandate-worldgen",
        "mandate-wasm",
    ] {
        source_files(&root.join("crates").join(c).join("src"), &[".rs"], &mut rs);
    }
    let mut ts = Vec::new();
    source_files(&root.join("web/src"), &[".ts", ".tsx"], &mut ts);
    let mut scan = SourceScan {
        engine: Vec::new(),
        all: BTreeSet::new(),
        prefixes: BTreeSet::new(),
    };
    let keyish = |s: &str| {
        s.contains('-')
            && s.starts_with(|c: char| c.is_ascii_lowercase())
            && s.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
            && !s.ends_with('-')
    };
    for (files, rust) in [(&rs, true), (&ts, false)] {
        for f in files {
            let Ok(text) = std::fs::read_to_string(f) else {
                continue;
            };
            let rel = f
                .strip_prefix(root)
                .unwrap_or(f)
                .to_string_lossy()
                .replace('\\', "/");
            // Keys built at run time, wherever the placeholder sits (nested
            // template literals defeat the literal scan): `fact-{…}` in Rust
            // format strings, `` `lean-${…}` `` in TypeScript.
            let open = if rust { "-{" } else { "-${" };
            let mut from = 0;
            while let Some(i) = text[from..].find(open) {
                let end = from + i + 1;
                let head: String = text[..end]
                    .chars()
                    .rev()
                    .take_while(|c| {
                        c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-' || *c == '_'
                    })
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect();
                if head.len() > 2 && head.starts_with(|c: char| c.is_ascii_lowercase()) {
                    scan.prefixes.insert(head);
                }
                from = end;
            }
            for (lit, line) in literals(&text, rust) {
                // A prefix keys are concatenated onto (`"place-" + kind`).
                if lit.ends_with('-') && lit.len() > 2 {
                    scan.prefixes.insert(lit.clone());
                }
                if keyish(&lit) {
                    if rust {
                        scan.engine.push((lit.clone(), format!("{rel}:{line}")));
                    }
                    scan.all.insert(lit);
                }
            }
        }
    }
    Some(scan)
}

/// `x_key` also provides `x` (the client renders the key as `$x`).
pub(super) fn with_stems(mut s: BTreeSet<String>) -> BTreeSet<String> {
    let stems: Vec<String> = s
        .iter()
        .filter_map(|a| a.strip_suffix("_key").map(str::to_string))
        .collect();
    s.extend(stems);
    s
}

pub(super) fn fluent_checks(
    b: &ContentBundle,
    sources: &[Source],
    scan: Option<&SourceScan>,
    out: &mut Out,
) {
    let en_text = b.locale_text("en");
    let msgs = messages(&en_text);
    let defined: BTreeSet<String> = fluent_keys(&en_text);
    let refs = content_keys(b);
    // Missing: content references (a normal load refuses them; the doctor
    // stubs them, so look in the real files) and keys the engine names.
    let real: BTreeSet<String> = sources
        .iter()
        .filter(|s| s.path.ends_with(".ftl") && s.path.contains("/en/"))
        .flat_map(|s| fluent_keys(&s.text))
        .collect();
    for (k, owner) in &refs {
        if !real.contains(k)
            && !out
                .findings
                .iter()
                .any(|f| f.check == "fluent-missing" && f.message.contains(&format!("`{k}`")))
        {
            out.add(
                Sev::Error,
                "fluent-missing",
                owner,
                format!("`{k}` is referenced but missing in English"),
            );
        }
    }
    let families: BTreeSet<&str> = defined.iter().filter_map(|k| k.split('-').next()).collect();
    if let Some(scan) = scan {
        let mut reported = BTreeSet::new();
        for (lit, at) in &scan.engine {
            let fam = lit.split('-').next().unwrap_or("");
            if families.contains(fam)
                && !defined.contains(lit)
                && !scan.prefixes.iter().any(|p| p == &format!("{lit}-"))
                // A suffix the engine puts after a prefix (`outcome-talk-` + `favour-offered`).
                && !defined.iter().any(|k| k.ends_with(&format!("-{lit}")))
                && reported.insert(lit.clone())
            {
                out.findings.push(Finding {
                    severity: Sev::Warn,
                    check: "fluent-missing".into(),
                    file: at.clone(),
                    item: lit.clone(),
                    message: format!("the engine names `{lit}`, which is not in English"),
                });
            }
        }
    }
    // Unused: nothing references the key (content, other messages, a
    // literal in the engine or the UI, or a prefix keys are built from).
    let mut referenced: BTreeSet<String> = refs.keys().cloned().collect();
    for v in msgs.values() {
        referenced.extend(references(v));
    }
    if let Some(scan) = scan {
        for k in &defined {
            if referenced.contains(k) {
                continue;
            }
            let used =
                scan.all.contains(k) || scan.prefixes.iter().any(|p| k.starts_with(p.as_str()));
            if !used {
                out.add(
                    Sev::Info,
                    "fluent-unused",
                    k,
                    format!("`{k}` is never referenced (content, messages, engine or UI)"),
                );
            }
        }
    }
    // Arguments a message uses that its caller never supplies.
    let vars = fluent_placeholders(&en_text);
    let check = |out: &mut Out, item: &str, key: &str, provided: &BTreeSet<String>, what: &str| {
        let Some(used) = vars.get(key) else { return };
        for v in used {
            if !provided.contains(v) {
                out.add(
                    Sev::Error,
                    "fluent-args",
                    item,
                    format!("`{key}` uses `${v}`, which {what} never supplies"),
                );
            }
        }
    };
    let base = |roles: &[&str], policy_roles: &[&str]| -> BTreeSet<String> {
        let mut s: BTreeSet<String> = ["town", "player", "crisis_key"]
            .iter()
            .map(|x| x.to_string())
            .collect();
        for r in roles {
            s.insert(r.to_string());
            s.insert(format!("{r}_role_key"));
        }
        for r in policy_roles {
            s.insert(format!("{r}_key"));
        }
        with_stems(s)
    };
    let split = |roles: &[mandate_content::RoleDef]| -> (Vec<String>, Vec<String>) {
        let mut people = Vec::new();
        let mut policies = Vec::new();
        for r in roles {
            match r.cast {
                Cast::Bill { .. } | Cast::Law { .. } => policies.push(r.id.clone()),
                _ => people.push(r.id.clone()),
            }
        }
        (people, policies)
    };
    for d in &b.dilemmas {
        let (p, q) = split(&d.roles);
        let pr: Vec<&str> = p.iter().map(|x| x.as_str()).collect();
        let qr: Vec<&str> = q.iter().map(|x| x.as_str()).collect();
        let mut provided = base(&pr, &qr);
        if d.raised_by.is_some() {
            provided.extend(with_stems(
                ["fact_key", "severity_key", "outlet_key", "outlet_place"]
                    .iter()
                    .map(|x| x.to_string())
                    .collect(),
            ));
        }
        let mut keys = vec![d.title_key.as_str(), d.situation_key.as_str()];
        for o in &d.options {
            keys.push(&o.label_key);
            keys.push(&o.aftermath_key);
        }
        for k in keys {
            check(out, &d.id, k, &provided, "the decision's cast");
        }
        // `$crisis` is there only while the town has a live crisis.
        let crisis_trigger = d
            .triggers
            .iter()
            .any(|c| matches!(c, Condition::Crisis { .. }));
        if !crisis_trigger
            && vars
                .get(&d.situation_key)
                .is_some_and(|v| v.contains("crisis"))
        {
            out.add(
                Sev::Warn,
                "fluent-args",
                &d.id,
                format!(
                    "`{}` uses `$crisis`, which is there only while the town has a live crisis (no crisis trigger)",
                    d.situation_key
                ),
            );
        }
    }
    for sc in &b.scenes {
        let mut roles = Vec::new();
        for d in b
            .dilemmas
            .iter()
            .filter(|d| d.scene.as_deref() == Some(sc.id.as_str()))
        {
            roles.extend(d.roles.iter().cloned());
        }
        let (p, q) = split(&roles);
        let pr: Vec<&str> = p.iter().map(|x| x.as_str()).collect();
        let qr: Vec<&str> = q.iter().map(|x| x.as_str()).collect();
        let mut provided = base(&pr, &qr);
        provided.extend(["memory", "subject", "year"].iter().map(|x| x.to_string()));
        for beat in &sc.beats {
            check(out, &sc.id, &beat.line, &provided, "the scene's cast");
        }
    }
    for o in &b.overheard {
        let mut provided: BTreeSet<String> =
            ["town", "hood"].iter().map(|x| x.to_string()).collect();
        let mut static_ok = true;
        for c in &o.when {
            match c.provides() {
                Some(a) => provided.extend(a),
                None => static_ok = false,
            }
        }
        if static_ok {
            check(out, &o.id, &o.key, &with_stems(provided), "its conditions");
        }
    }
}

// --- Text: how a line reads (text audit 1) ----------------------------------------------
