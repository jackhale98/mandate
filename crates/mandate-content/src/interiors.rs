//! Interior recipes (DEC-088, DEC-098 §4): what an enterable place's room is
//! like (its size, furniture kit and arrangements, where the people present
//! stand or sit, which actions sit at which hotspot), in
//! `content/base/interiors.jsonc`.
//!
//! These are presentation data. The client composes the room from a recipe
//! and a keyed seed (the pixel-art pack's composer, `tools/pixelkit`, reads
//! the same file); the engine only validates them here and never interprets
//! them, so they cannot change a simulation result. The part ids stay in the
//! art pack: the tile rules are checked against a generated catalogue of the
//! pack's parts (`interior_parts.json`, kept equal to the pack by the
//! pixelkit tests), ported from the pack's own `validateRecipe`.

use crate::bundle::ContentBundle;
use crate::types::ContentKey;
use crate::validate::{Issue, valid_key};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::OnceLock;

/// How the engine's place view says someone is here (`PlacePanelView.here_as`).
pub const HERE_AS: &[&str] = &["presiding", "member", "works", "visitor"];

/// Spot roles the client knows how to dress and place.
pub const SPOT_ROLES: &[&str] = &[
    "chair", "clerk", "member", "official", "player", "staff", "public", "owner", "cook", "waiter",
    "patron",
];

/// The variable material roles a recipe picks ramps for (`@wood` refers to one).
const MATERIAL_ROLES: &[&str] = &["wood", "uph", "acc", "acc2", "wall", "trim"];

/// The largest room that fits a phone (tiles).
const MAX_COLS: i32 = 14;
const MAX_ROWS: i32 = 13;

/// A value, or a value with a weight for the seed's pick (default weight 1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Weighted<T> {
    Weighted(WeightedValue<T>),
    Plain(T),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WeightedValue<T> {
    pub value: T,
    pub weight: f64,
}

impl<T> Weighted<T> {
    pub fn value(&self) -> &T {
        match self {
            Weighted::Weighted(w) => &w.value,
            Weighted::Plain(v) => v,
        }
    }
    pub fn weight(&self) -> f64 {
        match self {
            Weighted::Weighted(w) => w.weight,
            Weighted::Plain(_) => 1.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Facing {
    Down,
    Up,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Pose {
    Stand,
    Sit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Lamp {
    Warm,
    White,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PatchKind {
    Surface,
    Rug,
    Dais,
}

/// A floor tile.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TilePos {
    pub x: i32,
    pub y: i32,
}

/// The room's light: the lamps' colour, how much of the night's grade reaches
/// inside (0..1), and unseen ceiling lights.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InteriorLight {
    pub lamp: Lamp,
    pub night: f64,
    #[serde(default)]
    pub ceiling: Vec<TilePos>,
}

/// A surface of the art pack and its ramps (a second ramp for two-colour surfaces).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SurfaceRef {
    pub surface: String,
    /// Ramp references: a palette ramp (`stone`, `grass+1`) or a material role (`@wood`).
    pub ramps: Vec<String>,
}

/// A lower band of the back wall (dado panelling, tiles), `h` px high.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WallBand {
    pub surface: String,
    pub ramps: Vec<String>,
    pub h: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WallStyle {
    pub id: String,
    pub upper: SurfaceRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lower: Option<WallBand>,
    /// The cornice, dado rail, skirting and window frames' ramp.
    pub trim: String,
}

/// The variable material roles' ramps, one picked per room by the seed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RoleRamps {
    pub wood: Vec<Weighted<String>>,
    pub uph: Vec<Weighted<String>>,
    pub acc: Vec<Weighted<String>>,
    pub acc2: Vec<Weighted<String>>,
}

/// A part id, or alternatives the seed picks between.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum PartRef {
    One(String),
    Pick(Vec<Weighted<String>>),
}

impl PartRef {
    pub fn alternatives(&self) -> Vec<&str> {
        match self {
            PartRef::One(p) => vec![p.as_str()],
            PartRef::Pick(list) => list.iter().map(|w| w.value().as_str()).collect(),
        }
    }
}

/// One placed part: on floor tiles, or on the back wall (a column and a
/// height `wy`, or standing on the floor line), or over the entrance (`front`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PieceDef {
    pub part: PartRef,
    pub x: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<i32>,
    /// Strips: length in tiles.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub len: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mirror: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dx: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dy: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wy: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub floor: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub front: Option<bool>,
    /// Present with this chance (0..1), keyed by the seed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chance: Option<f64>,
}

/// A patch of floor: another surface, a rug or a dais.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PatchDef {
    pub kind: PatchKind,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surface: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ramps: Option<Vec<String>>,
}

/// Where a person present stands or sits, by role.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SpotDef {
    pub id: String,
    pub role: String,
    pub x: i32,
    pub y: i32,
    pub facing: Facing,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pose: Option<Pose>,
}

/// A hotspot: a rectangle of tiles, the actions and services offered there,
/// and the tile the player stands on to use them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HotspotDef {
    pub id: String,
    /// Action keys from this place kind's `actions` (or `late_night`, for a late hotspot).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<String>,
    /// Services from this place kind's `services`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub services: Vec<String>,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub stand: TilePos,
    pub facing: Facing,
    /// Only offered late at night.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub late: bool,
}

/// One arrangement of a part of the room.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OptionDef {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
    pub pieces: Vec<PieceDef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub patches: Vec<PatchDef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub spots: Vec<SpotDef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hotspots: Vec<HotspotDef>,
}

/// Arrangements the seed picks between.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ChoiceDef {
    pub id: String,
    pub options: Vec<OptionDef>,
}

/// One interior recipe and version (DEC-088).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InteriorRecipe {
    /// Content id, e.g. `interior.diner`; with `version`, unique.
    pub id: ContentKey,
    /// A change that moves anything for the same seed bumps it (DEC-085).
    pub version: u32,
    /// The place kind (`places.jsonc` key) it furnishes.
    pub place: String,
    /// Floor tiles across and deep.
    pub cols: i32,
    pub rows: i32,
    /// The back wall face's height, px.
    pub wall_h: i32,
    /// The entrance's column in the front wall.
    pub door: i32,
    pub light: InteriorLight,
    pub floors: Vec<Weighted<SurfaceRef>>,
    pub walls: Vec<Weighted<WallStyle>>,
    pub roles: RoleRamps,
    pub pieces: Vec<PieceDef>,
    #[serde(default)]
    pub patches: Vec<PatchDef>,
    #[serde(default)]
    pub choices: Vec<ChoiceDef>,
    #[serde(default)]
    pub spots: Vec<SpotDef>,
    #[serde(default)]
    pub hotspots: Vec<HotspotDef>,
    /// How someone is here (the place view's `here_as`: presiding, member,
    /// works, visitor) → the spot roles they take, in order of preference.
    pub cast: BTreeMap<String, Vec<String>>,
    /// Staff roles drawn as scenery where nobody present takes them.
    #[serde(default)]
    pub extras: Vec<String>,
}

// ---- The art pack's part catalogue --------------------------------------------

#[derive(Clone, Debug, Deserialize)]
struct CatalogPart {
    kind: String,
    foot: [i32; 2],
    strip: Option<String>,
    h: i32,
}

#[derive(Clone, Debug, Deserialize)]
struct Catalog {
    parts: BTreeMap<String, CatalogPart>,
    surfaces: BTreeMap<String, String>,
    ramps: BTreeSet<String>,
}

fn catalog() -> Option<&'static Catalog> {
    static CAT: OnceLock<Option<Catalog>> = OnceLock::new();
    CAT.get_or_init(|| serde_json::from_str(include_str!("interior_parts.json")).ok())
        .as_ref()
}

// ---- The tile rules (ported from pixelkit's `validateRecipe`) ----------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tile {
    Blocked,
    Floor,
    Seat,
}

#[derive(Clone, Copy)]
struct Rect {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

fn footprint(p: &CatalogPart, piece: &PieceDef) -> Option<Rect> {
    if p.kind == "wall" || p.kind == "top" {
        return None;
    }
    let y = piece.y.unwrap_or(0);
    let len = piece.len.unwrap_or(1);
    match p.strip.as_deref() {
        Some("h") => Some(Rect {
            x: piece.x,
            y,
            w: len,
            h: p.foot[1],
        }),
        Some(_) => Some(Rect {
            x: piece.x,
            y,
            w: p.foot[0],
            h: len,
        }),
        None => Some(Rect {
            x: piece.x,
            y,
            w: p.foot[0],
            h: p.foot[1],
        }),
    }
}

/// Every combination of a recipe's options, as the chosen option per choice.
fn combos(r: &InteriorRecipe) -> Vec<Vec<&OptionDef>> {
    let mut out: Vec<Vec<&OptionDef>> = vec![Vec::new()];
    for c in &r.choices {
        out = out
            .into_iter()
            .flat_map(|acc| {
                c.options.iter().map(move |o| {
                    let mut v = acc.clone();
                    v.push(o);
                    v
                })
            })
            .collect();
    }
    out
}

fn check_ramp(r: &str, cat: &Catalog, what: &str, id: &str, issues: &mut Vec<Issue>) {
    if let Some(role) = r.strip_prefix('@') {
        if !MATERIAL_ROLES.contains(&role) {
            issues.push(Issue::error(
                id,
                &format!("{what}: unknown material role `{r}`"),
            ));
        }
        return;
    }
    let name = r
        .split_once(['+', '-'])
        .map(|(n, shift)| {
            if shift.is_empty() || !shift.chars().all(|c| c.is_ascii_digit()) {
                issues.push(Issue::error(
                    id,
                    &format!("{what}: bad ramp shift in `{r}`"),
                ));
            }
            n
        })
        .unwrap_or(r);
    if !cat.ramps.contains(name) {
        issues.push(Issue::error(
            id,
            &format!("{what}: `{name}` is not a palette ramp"),
        ));
    }
}

fn check_surface(s: &str, on: &str, cat: &Catalog, what: &str, id: &str, issues: &mut Vec<Issue>) {
    match cat.surfaces.get(s) {
        None => issues.push(Issue::error(id, &format!("{what}: unknown surface `{s}`"))),
        Some(k) if k != on => issues.push(Issue::error(
            id,
            &format!("{what}: surface `{s}` is a {k} surface, not a {on} one"),
        )),
        _ => {}
    }
}

/// Problems with one recipe's tiles, for every option combination and every alternative part.
fn tile_rules(r: &InteriorRecipe, cat: &Catalog, issues: &mut Vec<Issue>) {
    let id = format!("{}@{}", r.id, r.version);
    let id = id.as_str();
    let (cols, rows) = (r.cols, r.rows);
    let inside = |x: i32, y: i32| x >= 0 && y >= 0 && x < cols && y < rows;
    let mut errs: BTreeSet<String> = BTreeSet::new();
    if r.door < 0 || r.door >= cols {
        errs.insert(format!("door column {} outside the room", r.door));
    }
    for combo in combos(r) {
        let tag = if combo.is_empty() {
            "[base]".to_string()
        } else {
            format!(
                "[{}]",
                combo
                    .iter()
                    .map(|o| o.id.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
            )
        };
        let pieces: Vec<&PieceDef> = r
            .pieces
            .iter()
            .chain(combo.iter().flat_map(|o| o.pieces.iter()))
            .collect();
        let spots: Vec<&SpotDef> = r
            .spots
            .iter()
            .chain(combo.iter().flat_map(|o| o.spots.iter()))
            .collect();
        let hotspots: Vec<&HotspotDef> = r
            .hotspots
            .iter()
            .chain(combo.iter().flat_map(|o| o.hotspots.iter()))
            .collect();
        let mut walk = vec![Tile::Floor; (cols.max(0) * rows.max(0)) as usize];
        let mut taken: BTreeMap<(i32, i32), &str> = BTreeMap::new();
        for (i, p) in pieces.iter().enumerate() {
            let alts = p.part.alternatives();
            for (ai, a) in alts.iter().enumerate() {
                let Some(def) = cat.parts.get(*a) else {
                    errs.insert(format!("{tag} piece {i}: unknown part \"{a}\""));
                    continue;
                };
                if def.strip.is_some() && p.len.unwrap_or(1) < 1 {
                    errs.insert(format!("{tag} {a}: strip length {:?}", p.len));
                }
                if def.kind == "wall" && p.front != Some(true) {
                    if p.x < 0 || p.x >= cols {
                        errs.insert(format!("{tag} {a}: wall column {} outside the room", p.x));
                    }
                    let top = if p.floor == Some(true) {
                        r.wall_h - def.h
                    } else {
                        p.wy.unwrap_or(0) - def.h / 2
                    };
                    if top < 0 || top + def.h > r.wall_h {
                        errs.insert(format!(
                            "{tag} {a}: off the wall face (top {top}, height {}, wall {})",
                            def.h, r.wall_h
                        ));
                    }
                }
                if def.kind == "top" && !inside(p.x, p.y.unwrap_or(0)) {
                    errs.insert(format!(
                        "{tag} {a}: at ({}, {}) outside the room",
                        p.x,
                        p.y.unwrap_or(0)
                    ));
                }
                if let Some(t) = footprint(def, p) {
                    if !inside(t.x, t.y) || !inside(t.x + t.w - 1, t.y + t.h - 1) {
                        errs.insert(format!(
                            "{tag} {a}: footprint ({}, {}, {}×{}) outside the room",
                            t.x, t.y, t.w, t.h
                        ));
                    }
                    if def.kind != "decal" && ai == 0 {
                        for y in t.y..t.y + t.h {
                            for x in t.x..t.x + t.w {
                                if let Some(other) = taken.get(&(x, y)) {
                                    errs.insert(format!(
                                        "{tag} {a} overlaps {other} at ({x}, {y})"
                                    ));
                                }
                                taken.insert((x, y), a);
                            }
                        }
                    }
                }
            }
            // For reachability a piece blocks with its most blocking alternative.
            let defs: Vec<&CatalogPart> = alts.iter().filter_map(|a| cat.parts.get(*a)).collect();
            let Some(def) = defs.iter().find(|d| d.kind == "furniture").or(defs.first()) else {
                continue;
            };
            if def.kind == "decal" {
                continue;
            }
            if let Some(t) = footprint(def, p) {
                for y in t.y..t.y + t.h {
                    for x in t.x..t.x + t.w {
                        if inside(x, y) {
                            walk[(y * cols + x) as usize] = if def.kind == "seat" {
                                Tile::Seat
                            } else {
                                Tile::Blocked
                            };
                        }
                    }
                }
            }
        }
        if !inside(r.door, rows - 1) {
            continue;
        }
        let at = |x: i32, y: i32| (y * cols + x) as usize;
        if walk[at(r.door, rows - 1)] != Tile::Floor {
            errs.insert(format!("{tag} the entrance tile is not floor"));
        }
        // Tiles reachable from the door over floor and seats (4-neighbour).
        let mut reach = vec![false; walk.len()];
        let mut queue = VecDeque::new();
        if walk[at(r.door, rows - 1)] != Tile::Blocked {
            reach[at(r.door, rows - 1)] = true;
            queue.push_back((r.door, rows - 1));
        }
        while let Some((x, y)) = queue.pop_front() {
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let (nx, ny) = (x + dx, y + dy);
                if !inside(nx, ny) || reach[at(nx, ny)] || walk[at(nx, ny)] == Tile::Blocked {
                    continue;
                }
                reach[at(nx, ny)] = true;
                queue.push_back((nx, ny));
            }
        }
        let check = |what: &str, x: i32, y: i32, errs: &mut BTreeSet<String>| {
            if !inside(x, y) {
                errs.insert(format!("{tag} {what} at ({x}, {y}) outside the room"));
            } else if walk[at(x, y)] == Tile::Blocked {
                errs.insert(format!(
                    "{tag} {what} at ({x}, {y}) is not on walkable floor"
                ));
            } else if !reach[at(x, y)] {
                errs.insert(format!(
                    "{tag} {what} at ({x}, {y}) is not reachable from the door"
                ));
            }
        };
        let mut ids = BTreeSet::new();
        let mut tiles: BTreeMap<(i32, i32), &SpotDef> = BTreeMap::new();
        for s in &spots {
            if !ids.insert(s.id.as_str()) {
                errs.insert(format!("{tag} duplicate spot id {}", s.id));
            }
            check(&format!("spot {}", s.id), s.x, s.y, &mut errs);
            if let Some(o) = tiles.insert((s.x, s.y), s) {
                errs.insert(format!(
                    "{tag} spots {} and {} share ({}, {})",
                    o.id, s.id, s.x, s.y
                ));
            }
            if s.pose == Some(Pose::Sit) && (!inside(s.x, s.y) || walk[at(s.x, s.y)] != Tile::Seat)
            {
                errs.insert(format!(
                    "{tag} spot {} sits but ({}, {}) has no seat",
                    s.id, s.x, s.y
                ));
            }
        }
        let mut hids = BTreeSet::new();
        for h in &hotspots {
            if !hids.insert(h.id.as_str()) {
                errs.insert(format!("{tag} duplicate hotspot id {}", h.id));
            }
            if h.actions.is_empty() && h.services.is_empty() {
                errs.insert(format!("{tag} hotspot {} offers nothing", h.id));
            }
            if h.w < 1 || h.h < 1 || !inside(h.x, h.y) || !inside(h.x + h.w - 1, h.y + h.h - 1) {
                errs.insert(format!("{tag} hotspot {} outside the room", h.id));
            }
            check(
                &format!("hotspot {}'s stand", h.id),
                h.stand.x,
                h.stand.y,
                &mut errs,
            );
            let touches = h.stand.x >= h.x - 1
                && h.stand.x <= h.x + h.w
                && h.stand.y >= h.y - 1
                && h.stand.y <= h.y + h.h;
            if !touches {
                errs.insert(format!(
                    "{tag} hotspot {}: its stand ({}, {}) is not next to it",
                    h.id, h.stand.x, h.stand.y
                ));
            }
            // The player's own seat may be where the player stands to use a hotspot.
            if let Some(s) = tiles.get(&(h.stand.x, h.stand.y))
                && s.role != "player"
            {
                errs.insert(format!(
                    "{tag} hotspot {}'s stand is spot {}'s tile",
                    h.id, s.id
                ));
            }
        }
    }
    for e in errs {
        issues.push(Issue::error(id, &e));
    }
}

/// Interior recipes (DEC-088): ids, sizes, materials known to the art pack,
/// the tile rules for every option combination, hotspots offering only what
/// their place kind offers (late ones only its late-night actions), roles and
/// cast, and every place kind's `interior` naming a recipe for that kind.
pub(crate) fn validate(b: &ContentBundle, issues: &mut Vec<Issue>) {
    let Some(cat) = catalog() else {
        issues.push(Issue::error(
            "interiors",
            "the art pack's part catalogue (interior_parts.json) does not parse",
        ));
        return;
    };
    for r in &b.interiors {
        let id = format!("{}@{}", r.id, r.version);
        let id = id.as_str();
        if !valid_key(&r.id) || !r.id.starts_with("interior.") {
            issues.push(Issue::error(id, "id must be interior.<name>"));
        }
        if r.version == 0 {
            issues.push(Issue::error(id, "version starts at 1"));
        }
        if !(1..=MAX_COLS).contains(&r.cols) || !(1..=MAX_ROWS).contains(&r.rows) {
            issues.push(Issue::error(
                id,
                &format!(
                    "a room is 1..{MAX_COLS} tiles wide and 1..{MAX_ROWS} deep (a phone's screen)"
                ),
            ));
            continue;
        }
        if !(16..=96).contains(&r.wall_h) {
            issues.push(Issue::error(id, "wall_h must be 16..96 px"));
        }
        if !(r.light.night.is_finite() && (0.0..=1.0).contains(&r.light.night)) {
            issues.push(Issue::error(id, "light.night must be in [0, 1]"));
        }
        for c in &r.light.ceiling {
            if c.x < 0 || c.y < 0 || c.x >= r.cols || c.y >= r.rows {
                issues.push(Issue::error(id, "a ceiling light is outside the room"));
            }
        }
        // Materials.
        if r.floors.is_empty() || r.walls.is_empty() {
            issues.push(Issue::error(id, "floors and walls must not be empty"));
        }
        let weights = r
            .floors
            .iter()
            .map(Weighted::weight)
            .chain(r.walls.iter().map(Weighted::weight))
            .chain(
                [&r.roles.wood, &r.roles.uph, &r.roles.acc, &r.roles.acc2]
                    .into_iter()
                    .flatten()
                    .map(Weighted::weight),
            );
        for w in weights {
            if !(w.is_finite() && w > 0.0) {
                issues.push(Issue::error(id, "weights must be > 0"));
            }
        }
        for f in &r.floors {
            check_surface(&f.value().surface, "floor", cat, "floors", id, issues);
            for x in &f.value().ramps {
                check_ramp(x, cat, "floors", id, issues);
            }
        }
        for w in &r.walls {
            let w = w.value();
            check_surface(&w.upper.surface, "wall", cat, "walls", id, issues);
            for x in &w.upper.ramps {
                check_ramp(x, cat, "walls", id, issues);
            }
            if let Some(l) = &w.lower {
                check_surface(&l.surface, "wall", cat, "walls", id, issues);
                for x in &l.ramps {
                    check_ramp(x, cat, "walls", id, issues);
                }
                if l.h < 1 || l.h >= r.wall_h {
                    issues.push(Issue::error(id, "a wall's lower band must fit the wall"));
                }
            }
            check_ramp(&w.trim, cat, "walls", id, issues);
        }
        for (role, list) in [
            ("wood", &r.roles.wood),
            ("uph", &r.roles.uph),
            ("acc", &r.roles.acc),
            ("acc2", &r.roles.acc2),
        ] {
            if list.is_empty() {
                issues.push(Issue::error(id, &format!("roles.{role} is empty")));
            }
            for x in list {
                check_ramp(x.value(), cat, &format!("roles.{role}"), id, issues);
            }
        }
        let all_patches = r.patches.iter().chain(
            r.choices
                .iter()
                .flat_map(|c| c.options.iter().flat_map(|o| o.patches.iter())),
        );
        for p in all_patches {
            if p.w < 1 || p.h < 1 || p.x < 0 || p.y < 0 || p.x + p.w > r.cols || p.y + p.h > r.rows
            {
                issues.push(Issue::error(id, "a patch is outside the room"));
            }
            if let Some(s) = &p.surface {
                check_surface(s, "floor", cat, "patches", id, issues);
            }
            for x in p.ramps.iter().flatten() {
                check_ramp(x, cat, "patches", id, issues);
            }
        }
        let mut choice_ids = BTreeSet::new();
        for c in &r.choices {
            if !choice_ids.insert(c.id.as_str()) {
                issues.push(Issue::error(id, &format!("duplicate choice `{}`", c.id)));
            }
            if c.options.is_empty() {
                issues.push(Issue::error(
                    id,
                    &format!("choice `{}` has no options", c.id),
                ));
            }
            let mut opt_ids = BTreeSet::new();
            for o in &c.options {
                if !opt_ids.insert(o.id.as_str()) {
                    issues.push(Issue::error(
                        id,
                        &format!("choice `{}`: duplicate option `{}`", c.id, o.id),
                    ));
                }
                if o.weight.is_some_and(|w| !(w.is_finite() && w > 0.0)) {
                    issues.push(Issue::error(id, "weights must be > 0"));
                }
            }
        }
        for p in r.pieces.iter().chain(
            r.choices
                .iter()
                .flat_map(|c| c.options.iter().flat_map(|o| o.pieces.iter())),
        ) {
            if p.chance
                .is_some_and(|c| !(c.is_finite() && c > 0.0 && c <= 1.0))
            {
                issues.push(Issue::error(id, "a piece's chance must be in (0, 1]"));
            }
        }
        // People and hotspots.
        let all_spots: Vec<&SpotDef> = r
            .spots
            .iter()
            .chain(
                r.choices
                    .iter()
                    .flat_map(|c| c.options.iter().flat_map(|o| o.spots.iter())),
            )
            .collect();
        let roles_here: BTreeSet<&str> = all_spots.iter().map(|s| s.role.as_str()).collect();
        for s in &all_spots {
            if !SPOT_ROLES.contains(&s.role.as_str()) {
                issues.push(Issue::error(
                    id,
                    &format!("spot {}: unknown role `{}`", s.id, s.role),
                ));
            }
        }
        for (k, list) in &r.cast {
            if !HERE_AS.contains(&k.as_str()) {
                issues.push(Issue::error(
                    id,
                    &format!("cast: `{k}` is not one of {HERE_AS:?}"),
                ));
            }
            for role in list {
                if !roles_here.contains(role.as_str()) || role == "player" {
                    issues.push(Issue::error(
                        id,
                        &format!("cast `{k}`: no `{role}` spot to place anyone on"),
                    ));
                }
            }
        }
        for e in &r.extras {
            if !roles_here.contains(e.as_str()) || e == "player" {
                issues.push(Issue::error(id, &format!("extras: no `{e}` spot")));
            }
        }
        match b.place_kind(&r.place) {
            None => issues.push(Issue::error(
                id,
                &format!("place `{}` is not a place kind", r.place),
            )),
            Some(kind) => {
                let all_hotspots = r.hotspots.iter().chain(
                    r.choices
                        .iter()
                        .flat_map(|c| c.options.iter().flat_map(|o| o.hotspots.iter())),
                );
                for h in all_hotspots {
                    for a in &h.actions {
                        let ok = if h.late {
                            kind.late_night.contains(a)
                        } else {
                            kind.actions.contains(a)
                        };
                        if !ok {
                            issues.push(Issue::error(
                                id,
                                &format!(
                                    "hotspot {}: `{a}` is not {} action of {}",
                                    h.id,
                                    if h.late { "a late-night" } else { "an" },
                                    r.place
                                ),
                            ));
                        }
                    }
                    for s in &h.services {
                        if !kind.services.contains(s) {
                            issues.push(Issue::error(
                                id,
                                &format!("hotspot {}: `{s}` is not a service of {}", h.id, r.place),
                            ));
                        }
                    }
                }
            }
        }
        tile_rules(r, cat, issues);
    }
    for p in &b.places {
        let Some(i) = &p.interior else { continue };
        if !b.interiors.iter().any(|r| &r.id == i && r.place == p.key) {
            issues.push(Issue::error(
                &p.id,
                &format!("interior `{i}` is not a recipe for {}", p.key),
            ));
        }
    }
}
