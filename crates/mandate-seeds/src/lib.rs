//! Test seeds that are cached, and found again when the world changes.
//!
//! Some tests need a world with a feature (a law that lapses in the first
//! skip, a town with a factory) and pick a seed that has it. A change that
//! moves outcomes on purpose can take the feature away from that seed. The
//! cache `tests/seeds.json` names each requirement with its seed and what it
//! needs; [`seed_for`] uses the cached seed when it still qualifies and
//! otherwise searches forward for the next one that does, so the test goes on
//! without a hunt by hand. `mandate-cli seeds refresh` rewrites the file;
//! `seeds check` fails, naming the requirement, when it is stale.
//!
//! This is dev-only support: no engine crate depends on it, and no game
//! behaviour reads it.
#![allow(clippy::print_stderr)]

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub mod requirements;

/// The seeds searched: `1..=9999`, wrapping (0 is avoided: "0" is falsy in web code).
pub const FIRST: u64 = 1;
pub const LAST: u64 = 9999;
/// At most this many candidates after the cached seed are tried.
pub const MAX_CANDIDATES: u32 = 64;

/// One requirement in the cache file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub seed: u64,
    pub needs: String,
}

/// The cache file: requirement name to entry, sorted by name.
pub type Cache = BTreeMap<String, Entry>;

/// `tests/seeds.json` at the repository root.
pub fn cache_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/seeds.json")
}

pub fn read_cache(path: &Path) -> Result<Cache, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

pub fn write_cache(path: &Path, cache: &Cache) -> Result<(), String> {
    let mut text = serde_json::to_string_pretty(cache).map_err(|e| e.to_string())?;
    text.push('\n');
    std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

/// The candidates after `cached`, in the order tried: the next seed upward,
/// wrapping from [`LAST`] to [`FIRST`], [`MAX_CANDIDATES`] of them.
pub fn candidates(cached: u64) -> impl Iterator<Item = u64> {
    let span = LAST - FIRST + 1;
    let base = cached.wrapping_sub(FIRST) % span;
    (1..=u64::from(MAX_CANDIDATES)).map(move |k| FIRST + (base + k) % span)
}

/// The first seed that satisfies `pred`, starting with `cached` itself and
/// then searching forward. `None` if none of them does.
pub fn search(cached: u64, pred: &dyn Fn(u64) -> bool) -> Option<u64> {
    if pred(cached) {
        return Some(cached);
    }
    candidates(cached).find(|s| pred(*s))
}

/// The seed for the requirement `name`: the cached one if `predicate` holds
/// for it, else the first later one that does (one line says the cache is
/// stale). Panics, naming the requirement, if none does: the game no longer
/// produces that situation.
pub fn seed_for(name: &str, predicate: impl Fn(u64) -> bool) -> u64 {
    seed_for_in(&cache_path(), name, predicate)
}

/// [`seed_for`] over a given cache file.
pub fn seed_for_in(path: &Path, name: &str, predicate: impl Fn(u64) -> bool) -> u64 {
    let cache = read_cache(path).unwrap_or_else(|e| panic!("seed cache unreadable: {e}"));
    let entry = cache.get(name).unwrap_or_else(|| {
        panic!("requirement `{name}` is not in the seed cache (tests/seeds.json)")
    });
    if predicate(entry.seed) {
        return entry.seed;
    }
    match candidates(entry.seed).find(|s| predicate(*s)) {
        Some(found) => {
            eprintln!(
                "seeds: `{name}`: cached seed {} no longer qualifies ({}); using {found} \
                 (run `mandate-cli seeds refresh` and commit tests/seeds.json)",
                entry.seed, entry.needs
            );
            found
        }
        None => panic!(
            "requirement `{name}` ({}): no seed from {} onwards ({} tried) qualifies. \
             The game no longer produces this situation",
            entry.needs,
            entry.seed,
            MAX_CANDIDATES + 1
        ),
    }
}

/// The seed for a requirement of [`requirements::ALL`], by name.
pub fn seed(name: &str) -> u64 {
    let r = requirements::ALL
        .iter()
        .find(|r| r.name == name)
        .unwrap_or_else(|| panic!("no requirement `{name}` in mandate-seeds"));
    seed_for(name, r.holds)
}

/// What `check` or `refresh` found for one requirement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub name: String,
    pub needs: String,
    /// The cached seed (`None`: not in the file yet).
    pub cached: Option<u64>,
    /// The seed that satisfies it now (`None`: none does).
    pub found: Option<u64>,
}

impl Outcome {
    pub fn fresh(&self) -> bool {
        self.cached.is_some() && self.cached == self.found
    }
}

/// Evaluate every requirement in `all` against the cache at `path`, and list
/// the entries of the file that no requirement names.
pub fn evaluate(
    path: &Path,
    all: &[requirements::Requirement],
) -> Result<(Vec<Outcome>, Vec<String>), String> {
    let cache = read_cache(path)?;
    let outcomes = all
        .iter()
        .map(|r| {
            let cached = cache.get(r.name).map(|e| e.seed);
            let found = search(cached.unwrap_or(FIRST), &r.holds);
            Outcome {
                name: r.name.into(),
                needs: r.needs.into(),
                cached,
                found,
            }
        })
        .collect();
    let unknown = cache
        .keys()
        .filter(|k| !all.iter().any(|r| r.name == k.as_str()))
        .cloned()
        .collect();
    Ok((outcomes, unknown))
}

/// Rewrite the cache with the seed each requirement needs now. Entries no
/// requirement names are dropped. Fails (writing nothing) if one has no seed.
pub fn refresh(path: &Path, outcomes: &[Outcome]) -> Result<(), String> {
    let mut cache = Cache::new();
    for o in outcomes {
        let seed = o
            .found
            .ok_or_else(|| format!("requirement `{}` ({}): no seed qualifies", o.name, o.needs))?;
        cache.insert(
            o.name.clone(),
            Entry {
                seed,
                needs: o.needs.clone(),
            },
        );
    }
    write_cache(path, &cache)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    fn temp_cache(tag: &str, seed: u64) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("mandate-seeds-{tag}-{}.json", std::process::id()));
        let mut cache = Cache::new();
        cache.insert(
            "tens".into(),
            Entry {
                seed,
                needs: "a seed that is a multiple of ten".into(),
            },
        );
        write_cache(&path, &cache).unwrap();
        path
    }

    #[test]
    fn a_fresh_cache_is_used_without_searching() {
        let path = temp_cache("fresh", 20);
        let calls = Cell::new(0);
        let s = seed_for_in(&path, "tens", |s| {
            calls.set(calls.get() + 1);
            s % 10 == 0
        });
        assert_eq!(s, 20);
        assert_eq!(calls.get(), 1, "only the cached seed was tried");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn a_stale_cache_is_replaced_by_the_next_qualifying_seed() {
        let path = temp_cache("stale", 21);
        let tried = RefCell::new(Vec::new());
        let s = seed_for_in(&path, "tens", |s| {
            tried.borrow_mut().push(s);
            s % 10 == 0
        });
        assert_eq!(s, 30);
        assert_eq!(
            *tried.borrow(),
            (21..=30).collect::<Vec<_>>(),
            "in order, stopping at the first"
        );
        // The same again: deterministic.
        assert_eq!(seed_for_in(&path, "tens", |s| s % 10 == 0), 30);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn the_search_wraps_within_the_range_and_is_bounded() {
        assert_eq!(search(LAST, &|s| s == 3), Some(3));
        assert_eq!(candidates(LAST).next(), Some(FIRST));
        assert_eq!(candidates(5).count(), MAX_CANDIDATES as usize);
        let far = 5 + u64::from(MAX_CANDIDATES);
        assert_eq!(search(5, &|s| s == far), Some(far));
        assert_eq!(search(5, &|s| s == far + 1), None, "bounded");
    }

    #[test]
    #[should_panic(expected = "requirement `tens`")]
    fn an_unsatisfiable_requirement_fails_with_its_name() {
        let path = temp_cache("never", 21);
        seed_for_in(&path, "tens", |_| false);
    }

    #[test]
    #[should_panic(expected = "not in the seed cache")]
    fn an_unknown_requirement_fails() {
        let path = temp_cache("unknown", 1);
        seed_for_in(&path, "nope", |_| true);
    }

    #[test]
    fn refresh_writes_the_found_seeds() {
        let path = temp_cache("refresh", 21);
        let o = |found| Outcome {
            name: "tens".into(),
            needs: "ten".into(),
            cached: Some(21),
            found,
        };
        assert!(refresh(&path, &[o(None)]).is_err());
        refresh(&path, &[o(Some(30))]).unwrap();
        assert_eq!(read_cache(&path).unwrap()["tens"].seed, 30);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn the_committed_file_names_every_requirement_here() {
        let cache = read_cache(&cache_path()).unwrap();
        for r in requirements::ALL {
            assert!(
                cache.contains_key(r.name),
                "tests/seeds.json lacks `{}`",
                r.name
            );
        }
    }
}
