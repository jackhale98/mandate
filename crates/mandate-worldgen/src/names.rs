//! Syllable-based fictional name generation from content pools (REQ-001).

use mandate_content::NamePoolDef;
use mandate_core::SimRng;
use std::collections::BTreeSet;

pub fn generate(pool: &NamePoolDef, rng: &mut SimRng) -> String {
    if !pool.words.is_empty()
        && rng.chance(0.15)
        && let Some(w) = rng.pick(&pool.words)
    {
        return w.clone();
    }
    let p = rng.pick(&pool.prefixes).cloned().unwrap_or_default();
    let m = rng.pick(&pool.middles).cloned().unwrap_or_default();
    let s = rng.pick(&pool.suffixes).cloned().unwrap_or_default();
    let mut name = format!("{p}{m}{s}");
    if let Some(first) = name.get(..1) {
        name = first.to_uppercase() + &name[1..];
    }
    name
}

/// Generate a name not yet in `used` (falls back to a numbered variant).
pub fn unique(pool: &NamePoolDef, rng: &mut SimRng, used: &mut BTreeSet<String>) -> String {
    for _ in 0..64 {
        let n = generate(pool, rng);
        if used.insert(n.clone()) {
            return n;
        }
    }
    let base = generate(pool, rng);
    let mut i = 2;
    loop {
        let n = format!("{base} {i}");
        if used.insert(n.clone()) {
            return n;
        }
        i += 1;
    }
}
