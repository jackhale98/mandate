//! Lab parameter overrides and grid sweeps (DEC-078 decision 2): `--set
//! <path>=<value>` (repeatable) changes content before it is loaded and
//! validated; `--grid <path>=<v1>,<v2>,…` (once or twice) runs every
//! combination and prints one summary row each. See
//! `mandate_content::overrides` for the path syntax.

use mandate_content::ContentBundle;
use mandate_content::bundle::read_dir_sources;
use mandate_content::overrides::{self, Override};
use serde_json::Value;
use std::path::Path;
use std::sync::Arc;

/// Parse `--set` arguments.
pub fn parse_sets(sets: &[String]) -> Result<Vec<Override>, String> {
    sets.iter().map(|s| Override::parse(s)).collect()
}

/// Parse `--grid` arguments: at most two, each `path=v1,v2,…` with scalar
/// values (numbers, booleans, or strings).
pub fn parse_grid(grid: &[String]) -> Result<Vec<(String, Vec<Value>)>, String> {
    if grid.len() > 2 {
        return Err("--grid takes at most two parameters".into());
    }
    grid.iter()
        .map(|g| {
            let (path, values) = g
                .split_once('=')
                .ok_or_else(|| format!("--grid `{g}` must be <path>=<v1>,<v2>,…"))?;
            let values: Vec<Value> = values
                .split(',')
                .map(|v| {
                    let v = v.trim();
                    serde_json::from_str(v).unwrap_or_else(|_| Value::String(v.to_string()))
                })
                .collect();
            if values.is_empty() || path.trim().is_empty() {
                return Err(format!("--grid `{g}` needs a path and at least one value"));
            }
            Ok((path.trim().to_string(), values))
        })
        .collect()
}

/// Every combination of grid values (the first parameter varies slowest),
/// as override lists. No grid gives one empty combination.
pub fn combinations(grid: &[(String, Vec<Value>)]) -> Vec<Vec<Override>> {
    let mut out: Vec<Vec<Override>> = vec![Vec::new()];
    for (path, values) in grid {
        out = out
            .into_iter()
            .flat_map(|combo| {
                values.iter().map(move |v| {
                    let mut c = combo.clone();
                    c.push(Override {
                        path: path.clone(),
                        value: v.clone(),
                    });
                    c
                })
            })
            .collect();
    }
    out
}

/// Load content from a directory with overrides applied before validation.
pub fn load(root: &Path, sets: &[Override]) -> Result<Arc<ContentBundle>, String> {
    let mut sources =
        read_dir_sources(root).map_err(|e| format!("reading {}: {e}", root.display()))?;
    overrides::apply(&mut sources, sets)?;
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

/// `{path: value}` for a row label.
pub fn label(sets: &[Override]) -> Value {
    Value::Object(
        sets.iter()
            .map(|o| (o.path.clone(), o.value.clone()))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_combinations_are_ordered_and_complete() {
        let g = parse_grid(&["a.b=1,2,3".into(), "c.d=true,false".into()]).unwrap_or_default();
        let c = combinations(&g);
        assert_eq!(c.len(), 6);
        assert_eq!(label(&c[0]), serde_json::json!({ "a.b": 1, "c.d": true }));
        assert_eq!(label(&c[1]), serde_json::json!({ "a.b": 1, "c.d": false }));
        assert_eq!(label(&c[5]), serde_json::json!({ "a.b": 3, "c.d": false }));
        assert_eq!(combinations(&[]).len(), 1);
        assert!(parse_grid(&["a=1".into(), "b=1".into(), "c=1".into()]).is_err());
        assert!(parse_grid(&["nothing".into()]).is_err());
    }

    #[test]
    fn overrides_apply_to_the_content_pack_and_change_its_hash() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
        let base = load(&root, &[]).unwrap_or_else(|e| panic!("{e}"));
        let set = parse_sets(&["politics.vote.party_weight=1.6".into()]).unwrap_or_default();
        let a = load(&root, &set).unwrap_or_else(|e| panic!("{e}"));
        let b = load(&root, &set).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(a.politics.vote.party_weight, 1.6);
        assert_ne!(a.content_hash, base.content_hash);
        assert_eq!(a.content_hash, b.content_hash);
        // Unknown paths and out-of-range values are refused with a clear error.
        let e = load(
            &root,
            &parse_sets(&["politics.vote.party_wieght=1.6".into()]).unwrap_or_default(),
        )
        .err()
        .unwrap_or_default();
        assert!(
            e.contains("unknown parameter `politics.vote.party_wieght`"),
            "{e}"
        );
        let e = load(
            &root,
            &parse_sets(&["opinion.gov_approval_rate=3".into()]).unwrap_or_default(),
        )
        .err()
        .unwrap_or_default();
        assert!(
            e.contains("opinion.gov_approval_rate = 3 outside [0, 1]"),
            "{e}"
        );
    }
}
