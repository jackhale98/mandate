//! Points of interest per municipality (DATA_MODEL §1). Every hub POI is a
//! Place; organisations attach to them in later milestones.

use mandate_core::SimRng;
use mandate_sim::world::{GeoProfile, NeighborhoodKind, PlaceKind, SettlementClass};

pub struct PlacePlan {
    pub kind: PlaceKind,
    /// Preferred neighbourhood kind (falls back to the core).
    pub area: NeighborhoodKind,
    pub name: String,
}

const PAPERS: [&str; 6] = [
    "Courier",
    "Herald",
    "Gazette",
    "Post",
    "Ledger",
    "Chronicle",
];

pub struct Ctx<'a> {
    pub muni_name: &'a str,
    pub class: SettlementClass,
    pub geo: &'a GeoProfile,
    pub port: bool,
    pub industrial: bool,
    pub regional_capital: bool,
    pub national_capital: bool,
    pub given_names: &'a [String],
}

pub fn plan(rng: &mut SimRng, c: &Ctx<'_>) -> Vec<PlacePlan> {
    use NeighborhoodKind as A;
    use PlaceKind as K;
    let big = matches!(c.class, SettlementClass::Metro | SettlementClass::City);
    let mut out = Vec::new();
    let mut add = |kind, area, name: String| out.push(PlacePlan { kind, area, name });
    add(K::TownHall, A::Core, String::new());
    add(K::CouncilChamber, A::Core, String::new());
    // The player's party keeps a local office in every town (DEC-079): no draw.
    add(K::PartyOffice, A::Core, String::new());
    let diner = rng
        .pick(c.given_names)
        .cloned()
        .unwrap_or_else(|| "Corner".into());
    add(K::Diner, A::Core, format!("{diner}'s"));
    add(K::Market, A::Core, String::new());
    add(K::School, A::Suburb, String::new());
    add(K::Clinic, A::Inner, String::new());
    add(K::CommunityCentre, A::Suburb, String::new());
    add(K::Park, A::Inner, String::new());
    if c.class != SettlementClass::Rural {
        let paper = rng.pick(&PAPERS).copied().unwrap_or("Post");
        add(
            K::NewsOffice,
            A::Core,
            format!("The {} {paper}", c.muni_name),
        );
        add(K::BusinessAssociation, A::Core, String::new());
    }
    if c.industrial || big {
        add(K::UnionHall, A::Industrial, String::new());
    }
    if c.industrial {
        add(K::Factory, A::Industrial, String::new());
    }
    if c.port {
        add(K::Port, A::Waterfront, String::new());
    }
    if c.geo.fertility > 0.45 && !big {
        add(K::Farm, A::Rural, String::new());
    }
    if c.geo.minerals > 0.45 {
        add(K::Mine, A::Rural, String::new());
    }
    if c.geo.timber > 0.5 {
        add(K::Mill, A::Rural, String::new());
    }
    if c.geo.scenic > 0.45 || big {
        add(K::Hotel, A::Waterfront, String::new());
    }
    if big && (c.class == SettlementClass::Metro || rng.chance(0.5)) {
        add(K::University, A::Inner, String::new());
    }
    if c.class == SettlementClass::Metro {
        add(K::TechCampus, A::Suburb, String::new());
    }
    if c.regional_capital {
        add(K::RegionalAssembly, A::Core, String::new());
    }
    if c.national_capital {
        add(K::Parliament, A::Core, String::new());
        add(K::ExecutiveResidence, A::Inner, String::new());
    }
    out
}
