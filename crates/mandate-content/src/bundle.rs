//! Loading content sources into a validated `ContentBundle` (DEC-053).

use crate::jsonc::to_strict_json;
use crate::types::*;
use crate::validate::{Issue, Severity, validate_bundle};
use mandate_core::hash::StableHash;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A content source file: path relative to the content root, and its text.
#[derive(Clone, Debug)]
pub struct Source {
    pub path: String,
    pub text: String,
}

/// Fully loaded, validated content for one game session.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContentBundle {
    pub packs: Vec<PackManifest>,
    pub tuning: Tuning,
    /// The sector registry (DEC-075), in load order: files by path, then
    /// item order. The engine indexes sector data densely in this order.
    pub sectors: Vec<SectorDef>,
    /// The attitude registry (DEC-075), in load order like sectors. Cohorts,
    /// politicians, parties, and policy signatures are dense in this order.
    pub attitudes: Vec<AttitudeDef>,
    pub name_pools: BTreeMap<ContentKey, NamePoolDef>,
    /// The place-kind registry (DEC-079), in load order.
    #[serde(default)]
    pub places: Vec<PlaceKindDef>,
    pub economy: EconomyParams,
    pub constitutions: BTreeMap<ContentKey, ConstitutionDef>,
    pub policies: BTreeMap<ContentKey, PolicyTemplate>,
    pub politics: PoliticsParams,
    /// Fluent sources by language, then by file path.
    pub locales: BTreeMap<String, BTreeMap<String, String>>,
    /// Hash over every source path and text, in path order (DEC-053).
    pub content_hash: String,
}

impl ContentBundle {
    /// Registry index of a sector by content key.
    pub fn sector_index(&self, key: &str) -> Option<usize> {
        self.sectors.iter().position(|s| s.id == key)
    }

    /// A sector definition by content key.
    pub fn sector(&self, key: &str) -> Option<&SectorDef> {
        self.sectors.iter().find(|s| s.id == key)
    }

    /// Sector keys in registry order.
    pub fn sector_keys(&self) -> impl Iterator<Item = &ContentKey> {
        self.sectors.iter().map(|s| &s.id)
    }

    /// Registry indices and definitions of the sectors with a driver.
    pub fn sectors_with(&self, driver: SectorDriver) -> impl Iterator<Item = (usize, &SectorDef)> {
        self.sectors
            .iter()
            .enumerate()
            .filter(move |(_, s)| s.driver == driver)
    }

    /// Registry index of an attitude by its short key (e.g. `law_and_order`).
    pub fn attitude_index(&self, key: &str) -> Option<usize> {
        self.attitudes.iter().position(|a| a.key == key)
    }

    /// Attitude short keys in registry order.
    pub fn attitude_keys(&self) -> impl Iterator<Item = &String> {
        self.attitudes.iter().map(|a| &a.key)
    }

    /// A place kind by its snake-case key (e.g. `council_chamber`).
    pub fn place_kind(&self, key: &str) -> Option<&PlaceKindDef> {
        self.places.iter().find(|p| p.key == key)
    }

    /// All Fluent text for a language, concatenated in path order.
    pub fn locale_text(&self, lang: &str) -> String {
        self.locales
            .get(lang)
            .map(|files| files.values().cloned().collect::<Vec<_>>().join("\n"))
            .unwrap_or_default()
    }
}

/// Load and validate content. Returns the bundle and any warnings, or all errors.
pub fn load(sources: &[Source]) -> Result<(ContentBundle, Vec<Issue>), Vec<Issue>> {
    let mut sorted: Vec<&Source> = sources.iter().collect();
    sorted.sort_by(|a, b| a.path.cmp(&b.path));

    let mut hasher_input = Vec::new();
    for s in &sorted {
        hasher_input.extend_from_slice(s.path.as_bytes());
        hasher_input.push(0);
        hasher_input.extend_from_slice(s.text.as_bytes());
        hasher_input.push(0);
    }
    let content_hash = StableHash::of_bytes(&hasher_input).hex();

    let mut issues = Vec::new();
    let mut packs = Vec::new();
    let mut tuning: Option<(Tuning, String)> = None;
    let mut economy: Option<(EconomyParams, String)> = None;
    let mut politics: Option<(PoliticsParams, String)> = None;
    let mut constitutions: BTreeMap<ContentKey, ConstitutionDef> = BTreeMap::new();
    let mut policies: BTreeMap<ContentKey, PolicyTemplate> = BTreeMap::new();
    let mut sectors: Vec<SectorDef> = Vec::new();
    let mut attitudes: Vec<AttitudeDef> = Vec::new();
    let mut name_pools: BTreeMap<ContentKey, NamePoolDef> = BTreeMap::new();
    let mut places: Vec<PlaceKindDef> = Vec::new();
    let mut locales: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    let mut origin: BTreeMap<String, String> = BTreeMap::new();

    for src in sorted {
        if src.path.ends_with(".ftl") {
            match locale_of(&src.path) {
                Some(lang) => {
                    locales
                        .entry(lang)
                        .or_default()
                        .insert(src.path.clone(), src.text.clone());
                }
                None => issues.push(Issue::error(
                    &src.path,
                    "Fluent files must live under <pack>/locale/<lang>/",
                )),
            }
            continue;
        }
        if !src.path.ends_with(".jsonc") {
            continue;
        }
        let json = to_strict_json(&src.text);
        let file: ContentFile = match parse_content_file(&json) {
            Ok(f) => f,
            Err(e) => {
                issues.push(Issue {
                    severity: Severity::Error,
                    file: src.path.clone(),
                    line: Some(e.line()),
                    column: Some(e.column()),
                    message: e.to_string(),
                });
                continue;
            }
        };
        let mut claim = |id: &str, issues: &mut Vec<Issue>| -> bool {
            if let Some(prev) = origin.get(id) {
                issues.push(Issue::error(
                    &src.path,
                    &format!("duplicate content id `{id}` (first defined in {prev})"),
                ));
                false
            } else {
                origin.insert(id.to_string(), src.path.clone());
                true
            }
        };
        match file {
            ContentFile::Pack(p) => {
                if claim(&p.id, &mut issues) {
                    packs.push(p);
                }
            }
            ContentFile::Tuning { value } => {
                if let Some((_, prev)) = &tuning {
                    issues.push(Issue::error(
                        &src.path,
                        &format!("tuning defined twice (first in {prev})"),
                    ));
                } else {
                    tuning = Some((value, src.path.clone()));
                }
            }
            ContentFile::Constitution { value } => {
                if claim(&value.id, &mut issues) {
                    constitutions.insert(value.id.clone(), value);
                }
            }
            ContentFile::Policies { items } => {
                for p in items {
                    if claim(&p.id, &mut issues) {
                        policies.insert(p.id.clone(), p);
                    }
                }
            }
            ContentFile::Economy { value } => {
                if let Some((_, prev)) = &economy {
                    issues.push(Issue::error(
                        &src.path,
                        &format!("economy defined twice (first in {prev})"),
                    ));
                } else {
                    economy = Some((value, src.path.clone()));
                }
            }
            ContentFile::Politics { value } => {
                if let Some((_, prev)) = &politics {
                    issues.push(Issue::error(
                        &src.path,
                        &format!("politics defined twice (first in {prev})"),
                    ));
                } else {
                    politics = Some((*value, src.path.clone()));
                }
            }
            ContentFile::Sectors { items } => {
                for s in items {
                    if claim(&s.id, &mut issues) {
                        sectors.push(s);
                    }
                }
            }
            ContentFile::Attitudes { items } => {
                for a in items {
                    if claim(&a.id, &mut issues) {
                        attitudes.push(a);
                    }
                }
            }
            ContentFile::Places { items } => {
                for p in items {
                    if claim(&p.id, &mut issues) {
                        places.push(p);
                    }
                }
            }
            ContentFile::NamePools { items } => {
                for n in items {
                    if claim(&n.id, &mut issues) {
                        name_pools.insert(n.id.clone(), n);
                    }
                }
            }
        }
    }

    let Some((tuning, _)) = tuning else {
        issues.push(Issue::error("<content>", "no tuning file found"));
        return Err(issues);
    };
    let Some((economy, _)) = economy else {
        issues.push(Issue::error("<content>", "no economy file found"));
        return Err(issues);
    };
    let Some((politics, _)) = politics else {
        issues.push(Issue::error("<content>", "no politics file found"));
        return Err(issues);
    };
    packs.sort_by(|a, b| (a.load_order, &a.id).cmp(&(b.load_order, &b.id)));
    // Resolve keyed policy signatures to dense registry order (unknown keys
    // are reported by validation).
    for p in policies.values_mut() {
        p.signature_weights = attitudes
            .iter()
            .map(|a| p.signature.get(&a.key).copied().unwrap_or(0.0))
            .collect();
    }

    let bundle = ContentBundle {
        packs,
        tuning,
        sectors,
        attitudes,
        name_pools,
        places,
        economy,
        constitutions,
        policies,
        politics,
        locales,
        content_hash,
    };
    issues.extend(validate_bundle(&bundle));
    if issues.iter().any(|i| i.severity == Severity::Error) {
        Err(issues)
    } else {
        Ok((bundle, issues))
    }
}

fn locale_of(path: &str) -> Option<String> {
    let parts: Vec<&str> = path.split('/').collect();
    let idx = parts.iter().position(|p| *p == "locale")?;
    parts.get(idx + 1).map(|s| s.to_string())
}

/// Load every `.jsonc` and `.ftl` file under a directory (CLI and tests).
#[cfg(feature = "fs")]
pub fn read_dir_sources(root: &std::path::Path) -> std::io::Result<Vec<Source>> {
    fn walk(
        root: &std::path::Path,
        dir: &std::path::Path,
        out: &mut Vec<Source>,
    ) -> std::io::Result<()> {
        let mut entries: Vec<_> = std::fs::read_dir(dir)?.collect::<Result<_, _>>()?;
        entries.sort_by_key(|e| e.path());
        for e in entries {
            let p = e.path();
            if p.is_dir() {
                walk(root, &p, out)?;
            } else if matches!(
                p.extension().and_then(|x| x.to_str()),
                Some("jsonc" | "ftl")
            ) {
                let rel = p
                    .strip_prefix(root)
                    .unwrap_or(&p)
                    .to_string_lossy()
                    .replace('\\', "/");
                out.push(Source {
                    path: rel,
                    text: std::fs::read_to_string(&p)?,
                });
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    walk(root, root, &mut out)?;
    Ok(out)
}
