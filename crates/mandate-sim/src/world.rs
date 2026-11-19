//! World and space (DATA_MODEL §1, DEC-021).
//!
//! Serialized world data holds everything the simulation reads. The terrain
//! raster is *derived*: it is regenerated deterministically from the seed and
//! cached outside the state (`TerrainData`), which keeps saves small. Town
//! baselines (TOWN_GENERATION §15 E1) are stored by their domains and read
//! through `State::town_baseline`.

use mandate_core::{SimDate, define_id};
use serde::{Deserialize, Serialize};

define_id!(
    /// Regional tier jurisdiction.
    RegionId
);
define_id!(
    /// Local tier jurisdiction.
    MunicipalityId
);
define_id!(
    /// Finest population unit.
    NeighborhoodId
);
define_id!(
    /// Electoral district.
    ConstituencyId
);
define_id!(
    /// A point of interest; every hub POI is a Place (DEC-009).
    PlaceId
);
define_id!(
    /// Transport graph edge.
    LinkId
);

/// Biome classes. Discriminants are the raster encoding sent to the client.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum Biome {
    Ocean = 0,
    Lake = 1,
    Beach = 2,
    Wetland = 3,
    Grassland = 4,
    TemperateForest = 5,
    Rainforest = 6,
    Boreal = 7,
    Shrubland = 8,
    Steppe = 9,
    Desert = 10,
    Tundra = 11,
    Alpine = 12,
    Rock = 13,
}

/// Land ownership classes in the terrain raster.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum Cover {
    Sea = 0,
    Foreign = 1,
    Country = 2,
    Lake = 3,
}

/// Derived terrain raster (not serialized; see module docs).
#[derive(Clone, Debug, PartialEq)]
pub struct TerrainData {
    pub w: usize,
    pub h: usize,
    /// Metres above sea level (negative below).
    pub elevation: Vec<f32>,
    pub biome: Vec<Biome>,
    pub cover: Vec<Cover>,
    /// River size class per cell: 0 none, 1 stream, 2 river, 3 major river.
    pub river: Vec<u8>,
    /// Owning municipality index, `u32::MAX` outside the country.
    pub owner: Vec<u32>,
    /// Mean temperature (°C) and precipitation (mm/yr).
    pub temperature: Vec<f32>,
    pub precipitation: Vec<f32>,
    /// Flow direction per cell as a 16-point compass (0 = north, clockwise
    /// in 22.5° steps; only the eight even values occur), or `COMPASS_NONE`
    /// for the sea and outlets (TOWN_GENERATION §4.1 `downstream`).
    pub downstream: Vec<u8>,
    /// Forest per cell, per mille (the timber layer; §4.1 `forest_pm`).
    pub forest_pm: Vec<u16>,
    /// Mineral endowment per cell, per mille (§4.1 `minerals_pm`).
    pub minerals_pm: Vec<u16>,
}

/// "No direction" on a 16-point compass (TOWN_GENERATION §4.1 `Compass16`).
pub const COMPASS_NONE: u8 = 255;

/// The 16-point compass bearing of a grid step (x east, y south), or
/// `COMPASS_NONE` for no step. Only the eight neighbour steps have a bearing.
pub fn compass8(dx: i32, dy: i32) -> u8 {
    match (dx.signum(), dy.signum()) {
        (0, -1) => 0,
        (1, -1) => 2,
        (1, 0) => 4,
        (1, 1) => 6,
        (0, 1) => 8,
        (-1, 1) => 10,
        (-1, 0) => 12,
        (-1, -1) => 14,
        _ => COMPASS_NONE,
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorldGeo {
    pub width: u32,
    pub height: u32,
    /// Approximate kilometres per cell edge.
    pub km_per_cell: f64,
    /// Hash of the derived terrain; checked when rebuilding after load.
    pub terrain_hash: String,
    /// Prevailing wind direction in degrees (0 = from west).
    pub wind_from_deg: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SettlementClass {
    Metro,
    City,
    Town,
    Rural,
}

/// Geographic profile of a municipality, aggregated from its cells. Indices
/// are normalised to `[0, 1]` unless a unit is given.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GeoProfile {
    pub elevation_m: f64,
    pub ruggedness: f64,
    pub temperature_c: f64,
    pub precipitation_mm: f64,
    pub forest_share: f64,
    pub fertility: f64,
    pub timber: f64,
    pub minerals: f64,
    pub fishing: f64,
    pub energy: f64,
    pub scenic: f64,
    pub flood: f64,
    pub drought: f64,
    pub wildfire: f64,
    pub seismic: f64,
    pub storm: f64,
    pub dominant_biome: Option<Biome>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Region {
    pub id: RegionId,
    pub name: String,
    pub capital: MunicipalityId,
    pub municipalities: Vec<MunicipalityId>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Municipality {
    pub id: MunicipalityId,
    pub name: String,
    pub region: RegionId,
    pub class: SettlementClass,
    /// Settlement centre cell (x, y).
    pub center: (u32, u32),
    pub cells: u32,
    pub coastal: bool,
    pub on_river: bool,
    pub port: bool,
    pub neighbors: Vec<MunicipalityId>,
    pub neighborhoods: Vec<NeighborhoodId>,
    pub places: Vec<PlaceId>,
    pub geo: GeoProfile,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NeighborhoodKind {
    Core,
    Inner,
    Waterfront,
    Industrial,
    Suburb,
    Rural,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Neighborhood {
    pub id: NeighborhoodId,
    pub municipality: MunicipalityId,
    pub name: String,
    pub kind: NeighborhoodKind,
    pub neighbors: Vec<NeighborhoodId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Tier {
    Local,
    Regional,
    National,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Constituency {
    pub id: ConstituencyId,
    pub tier: Tier,
    /// Municipality or region index for local/regional tiers; 0 for national.
    pub jurisdiction: u32,
    pub number: u32,
    pub neighborhoods: Vec<NeighborhoodId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LinkMode {
    Road,
    Highway,
    Rail,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransportLink {
    pub id: LinkId,
    pub a: MunicipalityId,
    pub b: MunicipalityId,
    pub mode: LinkMode,
    pub length_km: f64,
    /// Travel cost (hours, approximate).
    pub cost: f64,
    /// Cell path for rendering.
    pub path: Vec<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PlaceKind {
    TownHall,
    CouncilChamber,
    Diner,
    Market,
    School,
    Clinic,
    CommunityCentre,
    Park,
    NewsOffice,
    PartyOffice,
    UnionHall,
    BusinessAssociation,
    Factory,
    Port,
    Farm,
    Mine,
    Mill,
    Hotel,
    University,
    TechCampus,
    RegionalAssembly,
    Parliament,
    ExecutiveResidence,
}

impl PlaceKind {
    pub const ALL: [PlaceKind; 23] = [
        PlaceKind::TownHall,
        PlaceKind::CouncilChamber,
        PlaceKind::Diner,
        PlaceKind::Market,
        PlaceKind::School,
        PlaceKind::Clinic,
        PlaceKind::CommunityCentre,
        PlaceKind::Park,
        PlaceKind::NewsOffice,
        PlaceKind::PartyOffice,
        PlaceKind::UnionHall,
        PlaceKind::BusinessAssociation,
        PlaceKind::Factory,
        PlaceKind::Port,
        PlaceKind::Farm,
        PlaceKind::Mine,
        PlaceKind::Mill,
        PlaceKind::Hotel,
        PlaceKind::University,
        PlaceKind::TechCampus,
        PlaceKind::RegionalAssembly,
        PlaceKind::Parliament,
        PlaceKind::ExecutiveResidence,
    ];

    /// Snake-case key: the place-kind registry key (DEC-079) and Fluent suffix.
    pub fn key(self) -> &'static str {
        match self {
            PlaceKind::TownHall => "town_hall",
            PlaceKind::CouncilChamber => "council_chamber",
            PlaceKind::Diner => "diner",
            PlaceKind::Market => "market",
            PlaceKind::School => "school",
            PlaceKind::Clinic => "clinic",
            PlaceKind::CommunityCentre => "community_centre",
            PlaceKind::Park => "park",
            PlaceKind::NewsOffice => "news_office",
            PlaceKind::PartyOffice => "party_office",
            PlaceKind::UnionHall => "union_hall",
            PlaceKind::BusinessAssociation => "business_association",
            PlaceKind::Factory => "factory",
            PlaceKind::Port => "port",
            PlaceKind::Farm => "farm",
            PlaceKind::Mine => "mine",
            PlaceKind::Mill => "mill",
            PlaceKind::Hotel => "hotel",
            PlaceKind::University => "university",
            PlaceKind::TechCampus => "tech_campus",
            PlaceKind::RegionalAssembly => "regional_assembly",
            PlaceKind::Parliament => "parliament",
            PlaceKind::ExecutiveResidence => "executive_residence",
        }
    }
}

/// Whether a place is in use (TOWN_GENERATION §4.2, DEC-090 D3). Places are
/// never deleted during a run: a place that shuts is marked closed and keeps
/// its id, so a town's layout never rearranges (DEC-085).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PlaceStatus {
    #[default]
    Open,
    Closed {
        since: SimDate,
    },
}

impl PlaceStatus {
    pub fn is_open(&self) -> bool {
        matches!(self, PlaceStatus::Open)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Place {
    pub id: PlaceId,
    pub kind: PlaceKind,
    pub municipality: MunicipalityId,
    pub neighborhood: NeighborhoodId,
    pub name: String,
    /// Open unless it shut during the run. Not serialized while open, so
    /// saves and state hashes are unchanged until a place closes.
    #[serde(default, skip_serializing_if = "PlaceStatus::is_open")]
    pub status: PlaceStatus,
    /// When it opened, for a place added during the run; `None` for the
    /// places that stood as play began (the layout plans those).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opened: Option<SimDate>,
}

impl Place {
    pub fn is_open(&self) -> bool {
        self.status.is_open()
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct World {
    pub geo: Option<WorldGeo>,
    pub capital: Option<MunicipalityId>,
    pub regions: Vec<Region>,
    pub municipalities: Vec<Municipality>,
    pub neighborhoods: Vec<Neighborhood>,
    pub constituencies: Vec<Constituency>,
    pub links: Vec<TransportLink>,
    pub places: Vec<Place>,
}

impl World {
    /// Neighbourhood indices of a municipality.
    pub fn hoods_of(&self, m: MunicipalityId) -> impl Iterator<Item = usize> + '_ {
        self.municipalities
            .get(m.index())
            .into_iter()
            .flat_map(|mu| mu.neighborhoods.iter().map(|n| n.index()))
    }

    /// Mark a place closed (DEC-090 D3): it keeps its id and its slot in
    /// the town; nothing is removed. Returns false for an unknown place or
    /// one already closed.
    pub fn close_place(&mut self, id: PlaceId, date: SimDate) -> bool {
        match self.places.get_mut(id.index()) {
            Some(p) if p.is_open() => {
                p.status = PlaceStatus::Closed { since: date };
                true
            }
            _ => false,
        }
    }

    /// Reopen a closed place. Returns false for an unknown or open place.
    pub fn reopen_place(&mut self, id: PlaceId) -> bool {
        match self.places.get_mut(id.index()) {
            Some(p) if !p.is_open() => {
                p.status = PlaceStatus::Open;
                true
            }
            _ => false,
        }
    }

    /// Add a place during the run with the next id (ids are never reused).
    /// It is listed last in its municipality, after the places the town
    /// began with; `None` if the neighbourhood is not in the municipality.
    pub fn open_place(
        &mut self,
        kind: PlaceKind,
        neighborhood: NeighborhoodId,
        name: String,
        date: SimDate,
    ) -> Option<PlaceId> {
        let municipality = self.neighborhoods.get(neighborhood.index())?.municipality;
        let mu = self.municipalities.get_mut(municipality.index())?;
        let id = PlaceId(u32::try_from(self.places.len()).ok()?);
        mu.places.push(id);
        self.places.push(Place {
            id,
            kind,
            municipality,
            neighborhood,
            name,
            status: PlaceStatus::Open,
            opened: Some(date),
        });
        Some(id)
    }

    /// Neighbourhood indices of a region.
    pub fn hoods_of_region(&self, r: RegionId) -> Vec<usize> {
        self.regions
            .get(r.index())
            .map(|re| {
                re.municipalities
                    .iter()
                    .flat_map(|m| self.hoods_of(*m))
                    .collect()
            })
            .unwrap_or_default()
    }
}
