//! Position-preserving file wrappers used by the loader.

use super::*;

// ---- Position-preserving file wrappers -------------------------------------
// serde's internally tagged enums buffer their input and lose line numbers, so
// loading probes `kind` first, then parses into one of these structs directly.
// `ContentFile` (above) remains the schema source of truth.

#[derive(Deserialize)]
pub(crate) struct KindProbe {
    pub kind: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PackFile {
    #[allow(dead_code)]
    pub kind: String,
    pub id: ContentKey,
    pub version: String,
    pub name_key: String,
    #[serde(default)]
    pub load_order: i32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ValueFile<T> {
    #[allow(dead_code)]
    pub kind: String,
    pub value: T,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ItemsFile<T> {
    #[allow(dead_code)]
    pub kind: String,
    pub items: Vec<T>,
}

pub(crate) fn parse_content_file(json: &str) -> Result<ContentFile, serde_json::Error> {
    let probe: KindProbe = serde_json::from_str(json)?;
    Ok(match probe.kind.as_str() {
        "pack" => {
            let p: PackFile = serde_json::from_str(json)?;
            ContentFile::Pack(PackManifest {
                id: p.id,
                version: p.version,
                name_key: p.name_key,
                load_order: p.load_order,
            })
        }
        "tuning" => ContentFile::Tuning {
            value: serde_json::from_str::<ValueFile<Tuning>>(json)?.value,
        },
        "sectors" => ContentFile::Sectors {
            items: serde_json::from_str::<ItemsFile<SectorDef>>(json)?.items,
        },
        "attitudes" => ContentFile::Attitudes {
            items: serde_json::from_str::<ItemsFile<AttitudeDef>>(json)?.items,
        },
        "communities" => ContentFile::Communities {
            items: serde_json::from_str::<ItemsFile<CommunityDef>>(json)?.items,
        },
        "concerns" => ContentFile::Concerns {
            items: serde_json::from_str::<ItemsFile<ConcernDef>>(json)?.items,
        },
        "scenarios" => ContentFile::Scenarios {
            items: serde_json::from_str::<ItemsFile<ScenarioDef>>(json)?.items,
        },
        "name_pools" => ContentFile::NamePools {
            items: serde_json::from_str::<ItemsFile<NamePoolDef>>(json)?.items,
        },
        "places" => ContentFile::Places {
            items: serde_json::from_str::<ItemsFile<PlaceKindDef>>(json)?.items,
        },
        "economy" => ContentFile::Economy {
            value: Box::new(serde_json::from_str::<ValueFile<EconomyParams>>(json)?.value),
        },
        "constitution" => ContentFile::Constitution {
            value: serde_json::from_str::<ValueFile<ConstitutionDef>>(json)?.value,
        },
        "politics" => ContentFile::Politics {
            value: Box::new(serde_json::from_str::<ValueFile<PoliticsParams>>(json)?.value),
        },
        "policies" => ContentFile::Policies {
            items: serde_json::from_str::<ItemsFile<PolicyTemplate>>(json)?.items,
        },
        "opinion" => ContentFile::Opinion {
            value: serde_json::from_str::<ValueFile<OpinionParams>>(json)?.value,
        },
        "actions" => ContentFile::Actions {
            value: Box::new(serde_json::from_str::<ValueFile<ActionParams>>(json)?.value),
        },
        "council" => ContentFile::Council {
            value: serde_json::from_str::<ValueFile<CouncilParams>>(json)?.value,
        },
        "career" => ContentFile::Career {
            value: serde_json::from_str::<ValueFile<CareerParams>>(json)?.value,
        },
        "npc" => ContentFile::Npc {
            value: serde_json::from_str::<ValueFile<NpcParams>>(json)?.value,
        },
        "traits" => ContentFile::Traits {
            items: serde_json::from_str::<ItemsFile<TraitDef>>(json)?.items,
        },
        "occupations" => ContentFile::Occupations {
            items: serde_json::from_str::<ItemsFile<OccupationDef>>(json)?.items,
        },
        "crises" => ContentFile::Crises {
            items: serde_json::from_str::<ItemsFile<CrisisDef>>(json)?.items,
        },
        "characters" => ContentFile::Characters {
            value: Box::new(serde_json::from_str::<ValueFile<CharacterParams>>(json)?.value),
        },
        "director" => ContentFile::Director {
            value: Box::new(
                serde_json::from_str::<ValueFile<crate::narrative::DirectorParams>>(json)?.value,
            ),
        },
        "storytellers" => ContentFile::Storytellers {
            items: serde_json::from_str::<ItemsFile<crate::narrative::StorytellerDef>>(json)?.items,
        },
        "overheard" => ContentFile::Overheard {
            items: serde_json::from_str::<ItemsFile<OverheardDef>>(json)?.items,
        },
        "dilemmas" => ContentFile::Dilemmas {
            items: serde_json::from_str::<ItemsFile<crate::narrative::DilemmaDef>>(json)?.items,
        },
        "goals" => ContentFile::Goals {
            items: serde_json::from_str::<ItemsFile<crate::narrative::GoalDef>>(json)?.items,
        },
        "storylines" => ContentFile::Storylines {
            items: serde_json::from_str::<ItemsFile<crate::narrative::StorylineDef>>(json)?.items,
        },
        "scenes" => ContentFile::Scenes {
            items: serde_json::from_str::<ItemsFile<crate::narrative::SceneDef>>(json)?.items,
        },
        "outlets" => ContentFile::Outlets {
            items: serde_json::from_str::<ItemsFile<crate::media::OutletDef>>(json)?.items,
        },
        "media" => ContentFile::Media {
            value: Box::new(
                serde_json::from_str::<ValueFile<crate::media::MediaParams>>(json)?.value,
            ),
        },
        "justice" => ContentFile::Justice {
            value: Box::new(
                serde_json::from_str::<ValueFile<crate::media::JusticeParams>>(json)?.value,
            ),
        },
        "town_districts" => ContentFile::TownDistricts {
            items: serde_json::from_str::<ItemsFile<crate::layout::TownDistrictDef>>(json)?.items,
        },
        "terrain_features" => ContentFile::TerrainFeatures {
            items: serde_json::from_str::<ItemsFile<crate::layout::TerrainFeatureDef>>(json)?.items,
        },
        "connectors" => ContentFile::Connectors {
            items: serde_json::from_str::<ItemsFile<crate::layout::ConnectorDef>>(json)?.items,
        },
        "interiors" => ContentFile::Interiors {
            items: serde_json::from_str::<ItemsFile<crate::interiors::InteriorRecipe>>(json)?.items,
        },
        "map_layers" => ContentFile::MapLayers {
            items: serde_json::from_str::<ItemsFile<crate::map_layers::MapLayerDef>>(json)?.items,
        },
        "routines" => ContentFile::Routines {
            items: serde_json::from_str::<ItemsFile<crate::routines::RoutineDef>>(json)?.items,
        },
        "knowledge" => ContentFile::Knowledge {
            value: Box::new(
                serde_json::from_str::<ValueFile<crate::knowledge::KnowledgeParams>>(json)?.value,
            ),
        },
        "talk_topics" => ContentFile::TalkTopics {
            items: serde_json::from_str::<ItemsFile<crate::conversation::TopicDef>>(json)?.items,
        },
        "talk_lines" => ContentFile::TalkLines {
            items: serde_json::from_str::<ItemsFile<crate::conversation::TalkLineDef>>(json)?.items,
        },
        "talk_voices" => ContentFile::TalkVoices {
            items: serde_json::from_str::<ItemsFile<crate::conversation::VoiceDef>>(json)?.items,
        },
        "ask_params" => ContentFile::AskParams {
            value: Box::new(serde_json::from_str::<ValueFile<crate::asks::AskParams>>(json)?.value),
        },
        "asks" => ContentFile::Asks {
            items: serde_json::from_str::<ItemsFile<crate::asks::AskDef>>(json)?.items,
        },
        "promises" => ContentFile::Promises {
            items: serde_json::from_str::<ItemsFile<crate::asks::PromiseDef>>(json)?.items,
        },
        "talk_params" => ContentFile::TalkParams {
            value: Box::new(
                serde_json::from_str::<ValueFile<crate::conversation::TalkParams>>(json)?.value,
            ),
        },
        "act_flow" => ContentFile::ActFlow {
            value: Box::new(
                serde_json::from_str::<ValueFile<crate::act::ActFlowParams>>(json)?.value,
            ),
        },
        "endings" => ContentFile::Endings {
            items: serde_json::from_str::<ItemsFile<crate::act::EndingDef>>(json)?.items,
        },
        "policy_families" => ContentFile::PolicyFamilies {
            items: serde_json::from_str::<ItemsFile<crate::policy_family::PolicyFamilyDef>>(json)?
                .items,
        },
        "policy_family_params" => ContentFile::PolicyFamilyParams {
            value: Box::new(
                serde_json::from_str::<ValueFile<crate::policy_family::FamilyParams>>(json)?.value,
            ),
        },
        "policy_family_patches" => ContentFile::PolicyFamilyPatches {
            items: serde_json::from_str::<ItemsFile<crate::policy_family::FamilyPatch>>(json)?
                .items,
        },
        other => {
            use serde::de::Error;
            return Err(serde_json::Error::custom(format!(
                "unknown content kind `{other}`"
            )));
        }
    })
}
