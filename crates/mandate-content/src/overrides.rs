//! Lab parameter overrides (DEC-078 decision 2): `--set <path>=<value>`
//! applied to content sources before they are loaded and validated, so a
//! sweep or bot tournament can try a balance number without editing files.
//!
//! A path starts with a content kind. For a single-value kind (`tuning`,
//! `economy`, `politics`, `opinion`, `actions`, `council`, `career`, `npc`)
//! it continues into the file's `value`: `politics.vote.party_weight`. For
//! a registry kind it names an entry by id in brackets:
//! `sectors[sector.retail].base_wage`, `concerns[concern.jobs].slope`.
//! Array elements are `[n]` or `.n`: `actions.fundraise.raised[0]`.
//!
//! Only values written in a content file can be overridden; an unknown path
//! is an error that lists what exists at the point it went wrong. The new
//! value must have the same JSON type as the old one (a number for a number).
//! The overridden sources hash differently, so a run records which rules it
//! used.

use crate::bundle::Source;
use crate::jsonc::to_strict_json;
use serde_json::Value;

/// One `path=value` override.
#[derive(Clone, Debug, PartialEq)]
pub struct Override {
    pub path: String,
    pub value: Value,
}

impl Override {
    /// Parse `path=value`. The value is JSON (`1.6`, `true`, `[1, 2]`), or a
    /// plain string when it does not parse as JSON.
    pub fn parse(s: &str) -> Result<Override, String> {
        let (path, raw) = s
            .split_once('=')
            .ok_or_else(|| format!("override `{s}` must be <path>=<value>"))?;
        let path = path.trim();
        if path.is_empty() {
            return Err(format!("override `{s}` has an empty path"));
        }
        let raw = raw.trim();
        let value = serde_json::from_str(raw).unwrap_or_else(|_| Value::String(raw.to_string()));
        Ok(Override {
            path: path.to_string(),
            value,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Seg {
    Key(String),
    /// `[...]`: an array index, or a registry entry id.
    Bracket(String),
}

fn segments(path: &str) -> Result<(String, Vec<Seg>), String> {
    let bad = || format!("malformed parameter path `{path}`");
    let mut chars = path.chars().peekable();
    let mut kind = String::new();
    while let Some(&c) = chars.peek() {
        if c == '.' || c == '[' {
            break;
        }
        kind.push(c);
        chars.next();
    }
    if kind.is_empty() {
        return Err(bad());
    }
    let mut out = Vec::new();
    while let Some(c) = chars.next() {
        match c {
            '.' => {
                let mut key = String::new();
                while let Some(&c) = chars.peek() {
                    if c == '.' || c == '[' {
                        break;
                    }
                    key.push(c);
                    chars.next();
                }
                if key.is_empty() {
                    return Err(bad());
                }
                out.push(Seg::Key(key));
            }
            '[' => {
                let mut inner = String::new();
                loop {
                    match chars.next() {
                        Some(']') => break,
                        Some(c) => inner.push(c),
                        None => return Err(bad()),
                    }
                }
                if inner.is_empty() {
                    return Err(bad());
                }
                out.push(Seg::Bracket(inner));
            }
            _ => return Err(bad()),
        }
    }
    Ok((kind, out))
}

fn kind_of(v: &Value) -> Option<&str> {
    v.get("kind").and_then(|k| k.as_str())
}

fn type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}

/// Walk `segs` from `node`; `at` is the path so far (for errors).
fn walk<'a>(
    mut node: &'a mut Value,
    segs: &[Seg],
    mut at: String,
) -> Result<&'a mut Value, String> {
    for seg in segs {
        node = match (seg, node) {
            (Seg::Key(k), Value::Object(o)) => {
                if !o.contains_key(k) {
                    let known: Vec<&str> = o.keys().map(String::as_str).collect();
                    return Err(format!(
                        "unknown parameter `{at}.{k}`: `{at}` has {} (only values written in a content file can be overridden)",
                        known.join(", ")
                    ));
                }
                at = format!("{at}.{k}");
                o.get_mut(k)
                    .ok_or_else(|| format!("unknown parameter `{at}`"))?
            }
            (Seg::Key(k) | Seg::Bracket(k), Value::Array(a)) => {
                let i: usize = k.parse().map_err(|_| {
                    format!("`{at}` is an array: index it with a number, not `{k}`")
                })?;
                let n = a.len();
                at = format!("{at}[{i}]");
                a.get_mut(i)
                    .ok_or_else(|| format!("`{at}` is out of range (the array has {n} elements)"))?
            }
            (Seg::Bracket(k), other) => {
                return Err(format!(
                    "`{at}` is {}, not an array: `[{k}]` does not apply",
                    type_name(other)
                ));
            }
            (Seg::Key(k), other) => {
                return Err(format!(
                    "`{at}` is {}, so it has no `{k}`",
                    type_name(other)
                ));
            }
        };
    }
    Ok(node)
}

/// Apply overrides to content sources in order. Sources that are touched are
/// rewritten as plain JSON. Errors name the override and what went wrong.
pub fn apply(sources: &mut [Source], overrides: &[Override]) -> Result<(), String> {
    for o in overrides {
        apply_one(sources, o).map_err(|e| format!("--set {}: {e}", o.path))?;
    }
    Ok(())
}

fn apply_one(sources: &mut [Source], o: &Override) -> Result<(), String> {
    let (kind, segs) = segments(&o.path)?;
    let mut kinds: Vec<String> = Vec::new();
    let mut seen_kind = false;
    for src in sources.iter_mut() {
        if !src.path.ends_with(".jsonc") {
            continue;
        }
        let Ok(mut root) = serde_json::from_str::<Value>(&to_strict_json(&src.text)) else {
            continue;
        };
        let Some(k) = kind_of(&root).map(str::to_string) else {
            continue;
        };
        if k != kind {
            if !kinds.contains(&k) {
                kinds.push(k);
            }
            continue;
        }
        seen_kind = true;
        let target = if root.get("value").is_some() {
            let value = root.get_mut("value").ok_or("no value")?;
            walk(value, &segs, kind.clone())?
        } else {
            // A registry: the first segment names an entry by id.
            let Some((Seg::Bracket(id), rest)) = segs.split_first() else {
                return Err(format!(
                    "`{kind}` is a registry: name an entry, e.g. `{kind}[<id>].<field>`"
                ));
            };
            let items = root
                .get_mut("items")
                .and_then(Value::as_array_mut)
                .ok_or_else(|| format!("`{}` has no items", src.path))?;
            let Some(item) = items
                .iter_mut()
                .find(|x| x.get("id").and_then(Value::as_str) == Some(id.as_str()))
            else {
                continue;
            };
            walk(item, rest, format!("{kind}[{id}]"))?
        };
        let same_type = std::mem::discriminant(target) == std::mem::discriminant(&o.value);
        if !same_type {
            return Err(format!(
                "`{}` is {}, but the override is {}",
                o.path,
                type_name(target),
                type_name(&o.value)
            ));
        }
        *target = o.value.clone();
        src.text = serde_json::to_string_pretty(&root).map_err(|e| e.to_string())?;
        return Ok(());
    }
    kinds.sort();
    match segs.first() {
        Some(Seg::Bracket(id)) if !seen_kind => Err(format!(
            "no content of kind `{kind}` defines `{id}` (kinds: {})",
            kinds.join(", ")
        )),
        Some(Seg::Bracket(id)) => Err(format!("no `{kind}` entry has id `{id}`")),
        _ => Err(format!(
            "no content file of kind `{kind}` (kinds: {})",
            kinds.join(", ")
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn src() -> Vec<Source> {
        vec![
            Source {
                path: "p/politics.jsonc".into(),
                text: "// c\n{\"kind\":\"politics\",\"value\":{\"vote\":{\"party_weight\":1.4,\"flag\":true},\"ages\":[0.1,0.2],}}".into(),
            },
            Source {
                path: "p/sectors.jsonc".into(),
                text: "{\"kind\":\"sectors\",\"items\":[{\"id\":\"sector.retail\",\"base_wage\":30000}]}".into(),
            },
            Source {
                path: "p/locale/en/a.ftl".into(),
                text: "x = y\n".into(),
            },
        ]
    }

    fn value(s: &[Source], i: usize) -> Value {
        serde_json::from_str(&to_strict_json(&s[i].text)).unwrap_or(Value::Null)
    }

    #[test]
    fn parses_paths_and_values() {
        let o = Override::parse("politics.vote.party_weight=1.6").unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(o.value, serde_json::json!(1.6));
        assert_eq!(
            Override::parse("a.b=[1, 2]").map(|o| o.value),
            Ok(serde_json::json!([1, 2]))
        );
        assert_eq!(
            Override::parse("a.b=hello").map(|o| o.value),
            Ok(Value::String("hello".into()))
        );
        assert!(Override::parse("no-equals").is_err());
        assert_eq!(
            segments("sectors[sector.retail].base_wage"),
            Ok((
                "sectors".into(),
                vec![
                    Seg::Bracket("sector.retail".into()),
                    Seg::Key("base_wage".into())
                ]
            ))
        );
        assert!(segments("a..b").is_err());
        assert!(segments("a[x").is_err());
    }

    #[test]
    fn applies_to_values_registries_and_arrays() {
        let mut s = src();
        let set = |p: &str| Override::parse(p).unwrap_or_else(|e| panic!("{e}"));
        apply(
            &mut s,
            &[
                set("politics.vote.party_weight=1.6"),
                set("politics.ages[1]=0.5"),
                set("politics.ages.0=0.3"),
                set("sectors[sector.retail].base_wage=31000"),
            ],
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let p = value(&s, 0);
        assert_eq!(p["value"]["vote"]["party_weight"], 1.6);
        assert_eq!(p["value"]["ages"], serde_json::json!([0.3, 0.5]));
        assert_eq!(value(&s, 1)["items"][0]["base_wage"], 31000);
        assert_eq!(s[2].text, "x = y\n");
    }

    #[test]
    fn rejects_unknown_paths_and_wrong_types() {
        let err = |p: &str| {
            let mut s = src();
            apply(
                &mut s,
                &[Override::parse(p).unwrap_or_else(|e| panic!("{e}"))],
            )
            .expect_err(p)
        };
        let e = err("politics.vote.party_wieght=1.6");
        assert!(
            e.contains("unknown parameter `politics.vote.party_wieght`"),
            "{e}"
        );
        assert!(e.contains("party_weight"), "{e}");
        assert!(err("politix.vote=1").contains("no content file of kind `politix`"));
        assert!(
            err("politics.vote.party_weight=true")
                .contains("is a number, but the override is a boolean")
        );
        assert!(err("politics.ages[5]=1").contains("out of range"));
        assert!(err("sectors.base_wage=1").contains("is a registry"));
        assert!(err("sectors[sector.mining].base_wage=1").contains("no `sectors` entry has id"));
        assert!(err("politics.vote.party_weight.x=1").contains("is a number, so it has no `x`"));
    }
}
