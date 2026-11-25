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
    /// The industry registry (DEC-110), in load order like sectors: a level
    /// under the sectors. The engine indexes industry data densely in this
    /// order. A sector with industries is a split sector.
    #[serde(default)]
    pub industries: Vec<IndustryDef>,
    /// The markets registry (DEC-110): where industries sell outside the
    /// country, in load order.
    #[serde(default)]
    pub markets: Vec<MarketDef>,
    /// The goods registry (DEC-110): what products are, in load order.
    #[serde(default)]
    pub goods: Vec<GoodDef>,
    /// The attitude registry (DEC-075), in load order like sectors. Cohorts,
    /// politicians, parties, and policy signatures are dense in this order.
    pub attitudes: Vec<AttitudeDef>,
    /// The community registry (DEC-075), in load order like sectors. Cohorts
    /// are dense in this order (`population::cohort_index`).
    #[serde(default)]
    pub communities: Vec<CommunityDef>,
    /// The concern registry (DEC-078 decision 3), in load order. Towns hold
    /// a level per concern and a salience per attitude derived from them.
    #[serde(default)]
    pub concerns: Vec<ConcernDef>,
    pub name_pools: BTreeMap<ContentKey, NamePoolDef>,
    /// The scenario registry (DEC-078 decision 4), in load order.
    #[serde(default)]
    pub scenarios: Vec<ScenarioDef>,
    /// The place-kind registry (DEC-079), in load order.
    #[serde(default)]
    pub places: Vec<PlaceKindDef>,
    pub economy: EconomyParams,
    pub constitutions: BTreeMap<ContentKey, ConstitutionDef>,
    pub policies: BTreeMap<ContentKey, PolicyTemplate>,
    pub politics: PoliticsParams,
    /// Balance parameter blocks (DEC-078 decision 1), one file each.
    pub opinion: OpinionParams,
    pub actions: ActionParams,
    pub council: CouncilParams,
    pub career: CareerParams,
    pub npc: NpcParams,
    /// Character trait tags (DEC-031), in load order.
    #[serde(default)]
    pub traits: Vec<TraitDef>,
    /// Occupations of Named characters (DEC-018), in load order.
    #[serde(default)]
    pub occupations: Vec<OccupationDef>,
    /// Kinds of live local crisis (DEC-077 §1), in load order.
    #[serde(default)]
    pub crises: Vec<CrisisDef>,
    /// Character, memory, and town-character numbers (DEC-031, DEC-077).
    pub characters: CharacterParams,
    /// The narrative director (DEC-033).
    pub director: crate::narrative::DirectorParams,
    /// Storyteller profiles, in load order.
    #[serde(default)]
    pub storytellers: Vec<crate::narrative::StorytellerDef>,
    /// Overheard talk (DEC-079), in load order.
    #[serde(default)]
    pub overheard: Vec<OverheardDef>,
    /// Dilemmas and requests (DEC-077 §4), in load order.
    #[serde(default)]
    pub dilemmas: Vec<crate::narrative::DilemmaDef>,
    /// NPC goal templates (DEC-032), in load order.
    #[serde(default)]
    pub goals: Vec<crate::narrative::GoalDef>,
    /// Scene templates (DEC-035), in load order.
    #[serde(default)]
    pub scenes: Vec<crate::narrative::SceneDef>,
    /// Outlet templates (DEC-034), in load order.
    #[serde(default)]
    pub outlets: Vec<crate::media::OutletDef>,
    /// Discovery, coverage, perception, responses, rumours, shady acts (DEC-034).
    pub media: crate::media::MediaParams,
    /// The legal path (DEC-006 Legal Jeopardy).
    pub justice: crate::media::JusticeParams,
    /// Knowledge is earned (DEC-091): start profiles, the option, the primer,
    /// introductions, and the effectiveness terms.
    #[serde(default)]
    pub knowledge: crate::knowledge::KnowledgeParams,
    /// Fluent sources by language, then by file path.
    pub locales: BTreeMap<String, BTreeMap<String, String>>,
    /// Hash over every source path and text, in path order (DEC-053).
    pub content_hash: String,
    /// Town layout v2 district types (DEC-090 D4), in load order. Presentation
    /// data for the client's generator: validated, never read by the simulation.
    #[serde(default)]
    pub town_districts: Vec<crate::layout::TownDistrictDef>,
    /// Town layout v2 terrain features, in load order.
    #[serde(default)]
    pub terrain_features: Vec<crate::layout::TerrainFeatureDef>,
    /// Town layout v2 connectors, in load order (the generator takes the first that fits).
    #[serde(default)]
    pub connectors: Vec<crate::layout::ConnectorDef>,
    /// The country map's layers (DEC-089), in load order (the picker's order).
    /// Presentation data: validated, never read by the simulation.
    #[serde(default)]
    pub map_layers: Vec<crate::map_layers::MapLayerDef>,
    /// The town map's layers (DEC-086, M6 note 11), in load order (the
    /// picker's order). Presentation data: validated, never read by the
    /// simulation.
    #[serde(default)]
    pub town_map_layers: Vec<crate::town_map_layers::TownMapLayerDef>,
    /// Interior recipes (DEC-088), in load order: presentation data for the
    /// client's room composer, validated, never read by the simulation.
    #[serde(default)]
    pub interiors: Vec<crate::interiors::InteriorRecipe>,
    /// Daily routines (DEC-031), in load order.
    #[serde(default)]
    pub routines: Vec<crate::routines::RoutineDef>,
    /// Conversation topics (DEC-093), in load order.
    #[serde(default)]
    pub talk_topics: Vec<crate::conversation::TopicDef>,
    /// Conversation lines, in load order.
    #[serde(default)]
    pub talk_lines: Vec<crate::conversation::TalkLineDef>,
    /// Conversation voices, in load order (ties in priority: the first).
    #[serde(default)]
    pub talk_voices: Vec<crate::conversation::VoiceDef>,
    /// Conversation rules (cost, discretion, speaker knowledge, the guard).
    #[serde(default)]
    pub talk: crate::conversation::TalkParams,
    /// The asks system's numbers (DEC-097 §2).
    #[serde(default)]
    pub ask_params: crate::asks::AskParams,
    /// Exchange scenes' constants (DEC-112; `exchange.jsonc`).
    pub exchange: crate::exchange::ExchangeParams,
    /// Ask templates, in load order.
    #[serde(default)]
    pub asks: Vec<crate::asks::AskDef>,
    /// Campaign promise templates, in load order.
    #[serde(default)]
    pub promises: Vec<crate::asks::PromiseDef>,
    /// The act flow (DEC-012): directives, interrupts, skip targets, the
    /// proxy and the Legacy record. Absent: no skips are offered.
    #[serde(default)]
    pub act_flow: crate::act::ActFlowParams,
    /// The endings registry (DEC-006), in load order (the first that holds
    /// ends the act).
    #[serde(default)]
    pub endings: Vec<crate::act::EndingDef>,
    /// Policy families (DEC-092), in load order, with every pack's patches
    /// applied (in pack order).
    #[serde(default)]
    pub policy_families: Vec<crate::policy_family::PolicyFamilyDef>,
    /// The family numbers (defaults when no pack defines them).
    #[serde(default)]
    pub policy_family_params: crate::policy_family::FamilyParams,
    /// Staged arcs (DEC-109), in load order.
    #[serde(default)]
    pub arcs: Vec<crate::arcs::ArcDef>,
}

impl ContentBundle {
    /// An arc template by id.
    pub fn arc(&self, id: &str) -> Option<&crate::arcs::ArcDef> {
        self.arcs.iter().find(|a| a.id == id)
    }

    /// An ask template by id.
    pub fn ask(&self, id: &str) -> Option<&crate::asks::AskDef> {
        self.asks.iter().find(|a| a.id == id)
    }

    /// A promise template by id.
    pub fn promise(&self, id: &str) -> Option<&crate::asks::PromiseDef> {
        self.promises.iter().find(|a| a.id == id)
    }

    /// A policy family by id (disabled ones included: bills and laws of a
    /// disabled family still compose).
    pub fn family(&self, id: &str) -> Option<&crate::policy_family::PolicyFamilyDef> {
        self.policy_families.iter().find(|f| f.id == id)
    }

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

    /// Registry index of an industry by content key.
    pub fn industry_index(&self, key: &str) -> Option<usize> {
        self.industries.iter().position(|s| s.id == key)
    }

    /// An industry definition by content key.
    pub fn industry(&self, key: &str) -> Option<&IndustryDef> {
        self.industries.iter().find(|s| s.id == key)
    }

    /// Industry keys in registry order.
    pub fn industry_keys(&self) -> impl Iterator<Item = &ContentKey> {
        self.industries.iter().map(|s| &s.id)
    }

    /// Registry indices and definitions of a sector's industries (none:
    /// the sector is not split).
    pub fn industries_of(&self, sector: usize) -> impl Iterator<Item = (usize, &IndustryDef)> {
        self.industries
            .iter()
            .enumerate()
            .filter(move |(_, d)| d.sector_index == sector)
    }

    /// Whether a sector has industries (DEC-110): its state is their sum.
    pub fn is_split(&self, sector: usize) -> bool {
        self.industries.iter().any(|d| d.sector_index == sector)
    }

    /// A market definition by content key.
    pub fn market(&self, key: &str) -> Option<&MarketDef> {
        self.markets.iter().find(|s| s.id == key)
    }

    /// A good definition by content key.
    pub fn good(&self, key: &str) -> Option<&GoodDef> {
        self.goods.iter().find(|s| s.id == key)
    }

    /// The gateway a kind of place is, by the kind's key.
    pub fn gateway_of(&self, place_kind: &str) -> Option<GatewayKind> {
        self.places
            .iter()
            .find(|p| p.key == place_kind)
            .and_then(|p| p.gateway)
    }

    /// An industry's shares across the markets, in registry order: its own
    /// weights where it sets them, else each market's, over their sum. One
    /// market has share 1 exactly.
    pub fn market_shares(&self, industry: &IndustryDef) -> Vec<f64> {
        let w: Vec<f64> = self
            .markets
            .iter()
            .map(|m| {
                industry
                    .exposure
                    .markets
                    .get(&m.id)
                    .copied()
                    .unwrap_or(m.share)
                    .max(0.0)
            })
            .collect();
        let sum: f64 = w.iter().sum();
        if sum > 0.0 {
            w.iter().map(|x| x / sum).collect()
        } else {
            w
        }
    }

    /// A product and its industry's registry index, by the product's id.
    pub fn product(&self, key: &str) -> Option<(usize, &ProductDef)> {
        self.industries
            .iter()
            .enumerate()
            .find_map(|(i, d)| d.products.iter().find(|p| p.id == key).map(|p| (i, p)))
    }

    /// Registry index of an attitude by its short key (e.g. `law_and_order`).
    pub fn attitude_index(&self, key: &str) -> Option<usize> {
        self.attitudes.iter().position(|a| a.key == key)
    }

    /// Attitude short keys in registry order.
    pub fn attitude_keys(&self) -> impl Iterator<Item = &String> {
        self.attitudes.iter().map(|a| &a.key)
    }

    /// Registry index of a community by content id (e.g. `community.general`).
    pub fn community_index(&self, id: &str) -> Option<usize> {
        self.communities.iter().position(|c| c.id == id)
    }

    /// Community ids in registry order.
    pub fn community_keys(&self) -> impl Iterator<Item = &ContentKey> {
        self.communities.iter().map(|c| &c.id)
    }

    /// Registry index of a concern by content id (e.g. `concern.jobs`).
    pub fn concern_index(&self, id: &str) -> Option<usize> {
        self.concerns.iter().position(|c| c.id == id)
    }

    /// Concern ids in registry order.
    pub fn concern_keys(&self) -> impl Iterator<Item = &ContentKey> {
        self.concerns.iter().map(|c| &c.id)
    }

    /// The community without a share model, which takes each
    /// neighbourhood's remainder (validation requires exactly one).
    pub fn remainder_community(&self) -> usize {
        self.communities
            .iter()
            .position(|c| c.share.is_none())
            .unwrap_or(0)
    }

    /// A scenario by content id.
    pub fn scenario(&self, id: &str) -> Option<&ScenarioDef> {
        self.scenarios.iter().find(|s| s.id == id)
    }

    /// The generator biases for a world: its scenario's, or none (a random
    /// country) when the world has no scenario or the id is unknown.
    pub fn scenario_or_random(&self, id: Option<&str>) -> ScenarioDef {
        id.and_then(|k| self.scenario(k))
            .cloned()
            .unwrap_or_default()
    }

    /// A place kind by its snake-case key (e.g. `council_chamber`).
    pub fn place_kind(&self, key: &str) -> Option<&PlaceKindDef> {
        self.places.iter().find(|p| p.key == key)
    }

    /// A trait by content id.
    pub fn trait_def(&self, id: &str) -> Option<&TraitDef> {
        self.traits.iter().find(|t| t.id == id)
    }

    /// An occupation by content id.
    pub fn occupation(&self, id: &str) -> Option<&OccupationDef> {
        self.occupations.iter().find(|o| o.id == id)
    }

    /// Registry index of a crisis kind by content id.
    pub fn crisis_index(&self, id: &str) -> Option<usize> {
        self.crises.iter().position(|c| c.id == id)
    }

    /// Crisis ids in registry order.
    pub fn crisis_keys(&self) -> impl Iterator<Item = &ContentKey> {
        self.crises.iter().map(|c| &c.id)
    }

    /// A storyteller profile by id.
    pub fn storyteller(&self, id: &str) -> Option<&crate::narrative::StorytellerDef> {
        self.storytellers.iter().find(|s| s.id == id)
    }

    /// A dilemma (or request) by id.
    pub fn dilemma(&self, id: &str) -> Option<&crate::narrative::DilemmaDef> {
        self.dilemmas.iter().find(|d| d.id == id)
    }

    /// An outlet template by id.
    pub fn outlet(&self, id: &str) -> Option<&crate::media::OutletDef> {
        self.outlets.iter().find(|o| o.id == id)
    }

    /// A goal template by id.
    pub fn goal(&self, id: &str) -> Option<&crate::narrative::GoalDef> {
        self.goals.iter().find(|g| g.id == id)
    }

    /// A scene template by id.
    pub fn scene(&self, id: &str) -> Option<&crate::narrative::SceneDef> {
        self.scenes.iter().find(|s| s.id == id)
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
    let mut opinion: Option<(OpinionParams, String)> = None;
    let mut actions: Option<(ActionParams, String)> = None;
    let mut council: Option<(CouncilParams, String)> = None;
    let mut career: Option<(CareerParams, String)> = None;
    let mut npc: Option<(NpcParams, String)> = None;
    let mut characters: Option<(CharacterParams, String)> = None;
    let mut director: Option<(crate::narrative::DirectorParams, String)> = None;
    let mut storytellers: Vec<crate::narrative::StorytellerDef> = Vec::new();
    let mut dilemmas: Vec<crate::narrative::DilemmaDef> = Vec::new();
    let mut overheard: Vec<OverheardDef> = Vec::new();
    let mut goals: Vec<crate::narrative::GoalDef> = Vec::new();
    let mut scenes: Vec<crate::narrative::SceneDef> = Vec::new();
    let mut outlets: Vec<crate::media::OutletDef> = Vec::new();
    let mut media: Option<(crate::media::MediaParams, String)> = None;
    let mut justice: Option<(crate::media::JusticeParams, String)> = None;
    let mut knowledge: Option<(crate::knowledge::KnowledgeParams, String)> = None;
    let mut traits: Vec<TraitDef> = Vec::new();
    let mut occupations: Vec<OccupationDef> = Vec::new();
    let mut crises: Vec<CrisisDef> = Vec::new();
    let mut constitutions: BTreeMap<ContentKey, ConstitutionDef> = BTreeMap::new();
    let mut policies: BTreeMap<ContentKey, PolicyTemplate> = BTreeMap::new();
    let mut sectors: Vec<SectorDef> = Vec::new();
    let mut industries: Vec<IndustryDef> = Vec::new();
    let mut markets: Vec<MarketDef> = Vec::new();
    let mut goods: Vec<GoodDef> = Vec::new();
    let mut attitudes: Vec<AttitudeDef> = Vec::new();
    let mut communities: Vec<CommunityDef> = Vec::new();
    let mut concerns: Vec<ConcernDef> = Vec::new();
    let mut name_pools: BTreeMap<ContentKey, NamePoolDef> = BTreeMap::new();
    let mut scenarios: Vec<ScenarioDef> = Vec::new();
    let mut places: Vec<PlaceKindDef> = Vec::new();
    let mut locales: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    let mut origin: BTreeMap<String, String> = BTreeMap::new();
    let mut town_districts: Vec<crate::layout::TownDistrictDef> = Vec::new();
    let mut terrain_features: Vec<crate::layout::TerrainFeatureDef> = Vec::new();
    let mut connectors: Vec<crate::layout::ConnectorDef> = Vec::new();
    let mut map_layers: Vec<crate::map_layers::MapLayerDef> = Vec::new();
    let mut town_map_layers: Vec<crate::town_map_layers::TownMapLayerDef> = Vec::new();
    let mut interiors: Vec<crate::interiors::InteriorRecipe> = Vec::new();
    let mut routines: Vec<crate::routines::RoutineDef> = Vec::new();
    let mut talk_topics: Vec<crate::conversation::TopicDef> = Vec::new();
    let mut talk_lines: Vec<crate::conversation::TalkLineDef> = Vec::new();
    let mut talk_voices: Vec<crate::conversation::VoiceDef> = Vec::new();
    let mut talk: Option<(crate::conversation::TalkParams, String)> = None;
    let mut ask_params: Option<(crate::asks::AskParams, String)> = None;
    let mut exchange: Option<(crate::exchange::ExchangeParams, String)> = None;
    let mut asks: Vec<crate::asks::AskDef> = Vec::new();
    let mut promises: Vec<crate::asks::PromiseDef> = Vec::new();
    let mut act_flow: Option<(crate::act::ActFlowParams, String)> = None;
    let mut endings: Vec<crate::act::EndingDef> = Vec::new();
    let mut arcs: Vec<crate::arcs::ArcDef> = Vec::new();
    // What arc files carry for their beats joins each registry after every
    // other file's items: registry order breaks ties and keys draws (an
    // overheard line's index), so an arc's items never shift the rest.
    let mut arc_dilemmas: Vec<crate::narrative::DilemmaDef> = Vec::new();
    let mut arc_asks: Vec<crate::asks::AskDef> = Vec::new();
    let mut arc_scenes: Vec<crate::narrative::SceneDef> = Vec::new();
    let mut arc_overheard: Vec<OverheardDef> = Vec::new();
    let mut policy_families: Vec<crate::policy_family::PolicyFamilyDef> = Vec::new();
    let mut family_params: Option<(crate::policy_family::FamilyParams, String)> = None;
    // (pack directory, path, patch): applied after every family is loaded.
    let mut family_patches: Vec<(String, String, crate::policy_family::FamilyPatch)> = Vec::new();
    // (pack directory, load order) of each manifest.
    let mut manifests: Vec<(String, i32)> = Vec::new();

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
        // Single-value kinds (parameter blocks) may be defined once across packs.
        fn once<T>(
            slot: &mut Option<(T, String)>,
            v: T,
            kind: &str,
            path: &str,
            issues: &mut Vec<Issue>,
        ) {
            if let Some((_, prev)) = slot {
                issues.push(Issue::error(
                    path,
                    &format!("{kind} defined twice (first in {prev})"),
                ));
            } else {
                *slot = Some((v, path.to_string()));
            }
        }
        match file {
            ContentFile::Opinion { value } => {
                once(&mut opinion, value, "opinion", &src.path, &mut issues)
            }
            ContentFile::Actions { value } => {
                once(&mut actions, *value, "actions", &src.path, &mut issues)
            }
            ContentFile::Council { value } => {
                once(&mut council, value, "council", &src.path, &mut issues)
            }
            ContentFile::Career { value } => {
                once(&mut career, value, "career", &src.path, &mut issues)
            }
            ContentFile::Npc { value } => once(&mut npc, value, "npc", &src.path, &mut issues),
            ContentFile::Characters { value } => once(
                &mut characters,
                *value,
                "characters",
                &src.path,
                &mut issues,
            ),
            ContentFile::Director { value } => {
                once(&mut director, *value, "director", &src.path, &mut issues)
            }
            ContentFile::Storytellers { items } => {
                for x in items {
                    if claim(&x.id, &mut issues) {
                        storytellers.push(x);
                    }
                }
            }
            ContentFile::Overheard { items } => {
                for x in items {
                    if claim(&x.id, &mut issues) {
                        overheard.push(x);
                    }
                }
            }
            ContentFile::Dilemmas { items } => {
                for x in items {
                    if claim(&x.id, &mut issues) {
                        dilemmas.push(x);
                    }
                }
            }
            ContentFile::Goals { items } => {
                for x in items {
                    if claim(&x.id, &mut issues) {
                        goals.push(x);
                    }
                }
            }
            ContentFile::Scenes { items } => {
                for x in items {
                    if claim(&x.id, &mut issues) {
                        scenes.push(x);
                    }
                }
            }
            ContentFile::Outlets { items } => {
                for x in items {
                    if claim(&x.id, &mut issues) {
                        outlets.push(x);
                    }
                }
            }
            ContentFile::Media { value } => {
                once(&mut media, *value, "media", &src.path, &mut issues)
            }
            ContentFile::Justice { value } => {
                once(&mut justice, *value, "justice", &src.path, &mut issues)
            }
            ContentFile::Knowledge { value } => {
                once(&mut knowledge, *value, "knowledge", &src.path, &mut issues)
            }
            ContentFile::Traits { items } => {
                for t in items {
                    if claim(&t.id, &mut issues) {
                        traits.push(t);
                    }
                }
            }
            ContentFile::Occupations { items } => {
                for o in items {
                    if claim(&o.id, &mut issues) {
                        occupations.push(o);
                    }
                }
            }
            ContentFile::Crises { items } => {
                for c in items {
                    if claim(&c.id, &mut issues) {
                        crises.push(c);
                    }
                }
            }
            ContentFile::Pack(p) => {
                if claim(&p.id, &mut issues) {
                    let dir = src.path.split('/').next().unwrap_or_default().to_string();
                    manifests.push((dir, p.load_order));
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
                    economy = Some((*value, src.path.clone()));
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
            ContentFile::Industries { items } => {
                for mut d in items {
                    if claim(&d.id, &mut issues) {
                        // Products are ids too: one registry of names.
                        d.products.retain(|p| claim(&p.id, &mut issues));
                        industries.push(d);
                    }
                }
            }
            ContentFile::Markets { items } => {
                for x in items {
                    if claim(&x.id, &mut issues) {
                        markets.push(x);
                    }
                }
            }
            ContentFile::Goods { items } => {
                for x in items {
                    if claim(&x.id, &mut issues) {
                        goods.push(x);
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
            ContentFile::Communities { items } => {
                for c in items {
                    if claim(&c.id, &mut issues) {
                        communities.push(c);
                    }
                }
            }
            ContentFile::Concerns { items } => {
                for c in items {
                    if claim(&c.id, &mut issues) {
                        concerns.push(c);
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
            ContentFile::Scenarios { items } => {
                for x in items {
                    if claim(&x.id, &mut issues) {
                        scenarios.push(x);
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
            ContentFile::TownDistricts { items } => {
                for x in items {
                    if claim(&x.id, &mut issues) {
                        town_districts.push(x);
                    }
                }
            }
            ContentFile::TerrainFeatures { items } => {
                for x in items {
                    if claim(&x.id, &mut issues) {
                        terrain_features.push(x);
                    }
                }
            }
            ContentFile::Connectors { items } => {
                for x in items {
                    if claim(&x.id, &mut issues) {
                        connectors.push(x);
                    }
                }
            }
            ContentFile::Interiors { items } => {
                for x in items {
                    // One entry per recipe and version.
                    if claim(&format!("{}@{}", x.id, x.version), &mut issues) {
                        interiors.push(x);
                    }
                }
            }
            ContentFile::MapLayers { items } => {
                for x in items {
                    if claim(&x.id, &mut issues) {
                        map_layers.push(x);
                    }
                }
            }
            ContentFile::TownMapLayers { items } => {
                for x in items {
                    if claim(&x.id, &mut issues) {
                        town_map_layers.push(x);
                    }
                }
            }
            ContentFile::Routines { items } => {
                for x in items {
                    if claim(&x.id, &mut issues) {
                        routines.push(x);
                    }
                }
            }
            ContentFile::TalkTopics { items } => {
                for x in items {
                    if claim(&x.id, &mut issues) {
                        talk_topics.push(x);
                    }
                }
            }
            ContentFile::TalkLines { items } => {
                for x in items {
                    if claim(&x.id, &mut issues) {
                        talk_lines.push(x);
                    }
                }
            }
            ContentFile::TalkVoices { items } => {
                for x in items {
                    if claim(&x.id, &mut issues) {
                        talk_voices.push(x);
                    }
                }
            }
            ContentFile::TalkParams { value } => {
                once(&mut talk, *value, "talk_params", &src.path, &mut issues)
            }
            ContentFile::AskParams { value } => once(
                &mut ask_params,
                *value,
                "ask_params",
                &src.path,
                &mut issues,
            ),
            ContentFile::ExchangeParams { value } => once(
                &mut exchange,
                *value,
                "exchange_params",
                &src.path,
                &mut issues,
            ),
            ContentFile::Asks { items } => {
                for x in items {
                    if claim(&x.id, &mut issues) {
                        asks.push(x);
                    }
                }
            }
            ContentFile::Promises { items } => {
                for x in items {
                    if claim(&x.id, &mut issues) {
                        promises.push(x);
                    }
                }
            }
            ContentFile::ActFlow { value } => {
                once(&mut act_flow, *value, "act_flow", &src.path, &mut issues)
            }
            ContentFile::Endings { items } => {
                for x in items {
                    if claim(&x.id, &mut issues) {
                        endings.push(x);
                    }
                }
            }
            ContentFile::Arcs {
                items,
                dilemmas: ds,
                asks: aks,
                scenes: ss,
                overheard: os,
            } => {
                for x in items {
                    if claim(&x.id, &mut issues) {
                        arcs.push(x);
                    }
                }
                // What an arc's beats name, in the registries they ride on
                // (appended once every file is read).
                for x in ds {
                    if claim(&x.id, &mut issues) {
                        arc_dilemmas.push(x);
                    }
                }
                for x in aks {
                    if claim(&x.id, &mut issues) {
                        arc_asks.push(x);
                    }
                }
                for x in ss {
                    if claim(&x.id, &mut issues) {
                        arc_scenes.push(x);
                    }
                }
                for x in os {
                    if claim(&x.id, &mut issues) {
                        arc_overheard.push(x);
                    }
                }
            }
            ContentFile::PolicyFamilies { items } => {
                for x in items {
                    if claim(&x.id, &mut issues) {
                        policy_families.push(x);
                    }
                }
            }
            ContentFile::PolicyFamilyParams { value } => once(
                &mut family_params,
                *value,
                "policy_family_params",
                &src.path,
                &mut issues,
            ),
            ContentFile::PolicyFamilyPatches { items } => {
                let pack = src.path.split('/').next().unwrap_or_default().to_string();
                for x in items {
                    family_patches.push((pack.clone(), src.path.clone(), x));
                }
            }
        }
    }
    dilemmas.extend(arc_dilemmas);
    asks.extend(arc_asks);
    scenes.extend(arc_scenes);
    overheard.extend(arc_overheard);

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
    let (
        Some((opinion, _)),
        Some((actions, _)),
        Some((council, _)),
        Some((career, _)),
        Some((npc, _)),
        Some((characters, _)),
        Some((director, _)),
        Some((media, _)),
        Some((justice, _)),
    ) = (
        opinion, actions, council, career, npc, characters, director, media, justice,
    )
    else {
        issues.push(Issue::error(
            "<content>",
            "the opinion, actions, council, career, npc, characters, director, media, and justice parameter files are all required",
        ));
        return Err(issues);
    };
    let Some((knowledge, _)) = knowledge else {
        issues.push(Issue::error("<content>", "no knowledge file found"));
        return Err(issues);
    };
    let Some((talk, _)) = talk else {
        issues.push(Issue::error("<content>", "no talk_params file found"));
        return Err(issues);
    };
    packs.sort_by(|a, b| (a.load_order, &a.id).cmp(&(b.load_order, &b.id)));
    // Family patches (DEC-092), in pack order: a pack's load order (its
    // manifest's, by the directory it lives in), then path, then item order.
    let order_of = |dir: &str| -> i32 {
        manifests
            .iter()
            .find(|(d, _)| d == dir)
            .map(|(_, o)| *o)
            .unwrap_or(0)
    };
    family_patches.sort_by_key(|(dir, path, _)| (order_of(dir), path.clone()));
    for (_, path, patch) in &family_patches {
        if let Err(e) = patch.apply(&mut policy_families) {
            issues.push(Issue::error(path, &e));
        }
    }
    // Resolve keyed policy signatures to dense registry order (unknown keys
    // are reported by validation).
    for p in policies.values_mut() {
        p.signature_weights = attitudes
            .iter()
            .map(|a| p.signature.get(&a.key).copied().unwrap_or(0.0))
            .collect();
    }
    // Likewise concern weights.
    for c in concerns.iter_mut() {
        c.attitude_weights = attitudes
            .iter()
            .map(|a| c.attitudes.get(&a.key).copied().unwrap_or(0.0))
            .collect();
    }
    // Likewise community attitude offsets.
    for c in communities.iter_mut() {
        c.attitude_offset_weights = attitudes
            .iter()
            .map(|a| c.attitude_offsets.get(&a.key).copied().unwrap_or(0.0))
            .collect();
    }

    // Resolve each industry's sector, links and neighbours to registry
    // indices (unknown links are reported by validation; an industry whose
    // sector is unknown is dropped here so nothing indexes out of range).
    industries.retain_mut(|d| {
        let Some(k) = sectors.iter().position(|s| s.id == d.sector) else {
            issues.push(Issue::error(
                &d.id,
                &format!("industry names unknown sector `{}`", d.sector),
            ));
            return false;
        };
        d.sector_index = k;
        true
    });
    let kept: Vec<ContentKey> = industries.iter().map(|d| d.id.clone()).collect();
    for d in industries.iter_mut() {
        for l in d.links.iter_mut() {
            if let Some(i) = kept.iter().position(|k| *k == l.from) {
                l.from_index = i;
                l.from_sector = false;
            } else if let Some(k) = sectors.iter().position(|s| s.id == l.from) {
                l.from_index = k;
                l.from_sector = true;
            } else {
                // Reported by validation; an unresolved link reads nothing.
                l.from_index = usize::MAX;
            }
        }
        // Prices are read through the goods registry (an unknown or
        // unpriced good is reported by validation).
        d.exposure.price_index = d
            .exposure
            .prices
            .iter()
            .filter_map(|(k, x)| {
                goods
                    .iter()
                    .find(|g| g.id == *k)
                    .and_then(|g| g.price)
                    .map(|c| (c, *x))
            })
            .collect();
        d.near_index = d
            .siting
            .near
            .iter()
            .filter_map(|(k, f)| kept.iter().position(|x| x == k).map(|i| (i, *f)))
            .collect();
    }

    let bundle = ContentBundle {
        packs,
        tuning,
        sectors,
        industries,
        markets,
        goods,
        attitudes,
        communities,
        concerns,
        name_pools,
        scenarios,
        places,
        economy,
        constitutions,
        policies,
        politics,
        opinion,
        actions,
        council,
        career,
        npc,
        traits,
        occupations,
        crises,
        characters,
        director,
        storytellers,
        overheard,
        dilemmas,
        goals,
        scenes,
        outlets,
        media,
        justice,
        knowledge,
        locales,
        content_hash,
        town_districts,
        terrain_features,
        connectors,
        map_layers,
        town_map_layers,
        interiors,
        routines,
        talk_topics,
        talk_lines,
        talk_voices,
        talk,
        ask_params: ask_params.map(|(v, _)| v).unwrap_or_default(),
        exchange: exchange.map(|(v, _)| v).unwrap_or_default(),
        asks,
        promises,
        act_flow: act_flow.map(|(a, _)| a).unwrap_or_default(),
        endings,
        policy_families,
        policy_family_params: family_params.map(|(p, _)| p).unwrap_or_default(),
        arcs,
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
