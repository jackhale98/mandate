//! Fluent keys: parsing `.ftl` sources and checking every referenced key exists.

use super::*;

/// Fluent message identifiers defined in a language's `.ftl` sources.
pub fn fluent_keys(text: &str) -> BTreeSet<String> {
    text.lines()
        .filter_map(|line| {
            let first = line.chars().next()?;
            if !first.is_ascii_alphabetic() {
                return None;
            }
            let (id, _) = line.split_once('=')?;
            let id = id.trim();
            id.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
                .then(|| id.to_string())
        })
        .collect()
}

/// The variables (`{ $x }`, also inside selectors) each Fluent message of
/// a language's `.ftl` sources uses, by message id. A message runs from its
/// `id = …` line through the indented lines after it.
pub fn fluent_placeholders(text: &str) -> BTreeMap<String, BTreeSet<String>> {
    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut current: Option<String> = None;
    let scan = |line: &str, vars: &mut BTreeSet<String>| {
        let mut rest = line;
        while let Some(i) = rest.find('$') {
            let name: String = rest[i + 1..]
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
                .collect();
            if !name.is_empty() {
                vars.insert(name.clone());
            }
            rest = &rest[i + 1 + name.len()..];
        }
    };
    for line in text.lines() {
        let first = line.chars().next();
        match first {
            Some(c) if c.is_ascii_alphabetic() => {
                current = line.split_once('=').map(|(id, _)| id.trim().to_string());
                if let (Some(id), Some((_, value))) = (&current, line.split_once('=')) {
                    scan(value, out.entry(id.clone()).or_default());
                }
            }
            Some(c) if c.is_whitespace() => {
                if let Some(id) = &current {
                    scan(line, out.entry(id.clone()).or_default());
                }
            }
            _ => current = None,
        }
    }
    out
}

/// Every referenced Fluent key must exist in English (the reference locale).
pub(super) fn validate_fluent_refs(b: &ContentBundle, issues: &mut Vec<Issue>) {
    // Every referenced Fluent key must exist in English (the reference locale).
    let en = fluent_keys(&b.locale_text("en"));
    let mut referenced: Vec<(&str, &str)> = Vec::new();
    for p in &b.packs {
        referenced.push((&p.id, &p.name_key));
    }
    for s in &b.sectors {
        referenced.push((&s.id, &s.name_key));
    }
    for c in &b.communities {
        referenced.push((&c.id, &c.name_key));
    }
    for x in &b.scenarios {
        referenced.push((&x.id, &x.name_key));
    }
    for c in &b.concerns {
        referenced.push((&c.id, &c.name_key));
    }
    for o in &b.overheard {
        referenced.push((&o.id, &o.key));
    }
    for c in b.constitutions.values() {
        referenced.push((&c.id, &c.name_key));
        for t in &c.tiers {
            for x in &t.bodies {
                referenced.push((&c.id, &x.name_key));
            }
            for x in &t.offices {
                referenced.push((&c.id, &x.name_key));
            }
        }
    }
    for p in &b.places {
        referenced.push((&p.id, &p.name_key));
    }
    for t in &b.traits {
        referenced.push((&t.id, &t.name_key));
    }
    for o in &b.occupations {
        referenced.push((&o.id, &o.name_key));
    }
    for c in &b.crises {
        referenced.push((&c.id, &c.name_key));
    }
    // Memories are quoted through `memory-<kind>`.
    let memory_keys: Vec<String> = crate::types::MEMORY_KINDS
        .iter()
        .map(|k| format!("memory-{}", k.replace('_', "-")))
        .collect();
    for key in &memory_keys {
        referenced.push(("characters", key));
    }
    for p in b.policies.values() {
        referenced.push((&p.id, &p.name_key));
        referenced.push((&p.id, &p.desc_key));
        if let Some(u) = &p.unlock {
            referenced.push((&p.id, &u.reason_key));
        }
        for k in &p.secondary_keys {
            referenced.push((&p.id, k));
        }
        for prm in &p.params {
            referenced.push((&p.id, &prm.name_key));
            referenced.push((&p.id, &prm.unit_key));
        }
    }
    // Attitudes are named in the UI through their conviction keys.
    let convictions: Vec<(&str, String)> = b
        .attitudes
        .iter()
        .flat_map(|a| {
            ["pro", "anti"].map(|side| (a.id.as_str(), format!("conviction-{side}-{}", a.key)))
        })
        // A character's expertise names the issue (DEC-031).
        .chain(
            b.attitudes
                .iter()
                .map(|a| (a.id.as_str(), format!("issue-{}", a.key))),
        )
        .collect();
    for (owner, key) in &convictions {
        referenced.push((owner, key));
    }
    // Residents name a town's industry in overheard talk (`talk-<name key>`).
    for s in &b.sectors {
        if let Some(k) = &s.talk_key {
            referenced.push((&s.id, k));
        }
    }
    // And its worries in conversation.
    for c in &b.concerns {
        if let Some(k) = &c.talk_key {
            referenced.push((&c.id, k));
        }
    }
    referenced.extend(crate::map_layers::fluent_refs(b));
    referenced.extend(crate::act::fluent_refs(b));
    for (owner, key) in referenced {
        if !en.contains(key) {
            issues.push(Issue::error(
                owner,
                &format!("missing Fluent key `{key}` in locale en"),
            ));
        }
    }
}
