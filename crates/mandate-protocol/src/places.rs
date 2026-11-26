//! Places as the interface (M5a, DEC-079): the player's location, the town's
//! places, the pocket (always available, coarse), and place panels (precise
//! information where a place offers it). The engine decides precision; the
//! client only renders.

use crate::campaign::campaign_view;
use crate::knowledge::{Gate, gate_concerns};
use crate::map::SectorJobsView;
use crate::office::{ConcernView, TracksView, bill_view, coarse, concerns_for};
use crate::views::DateView;
use mandate_content::{LATE_NIGHT_ACTIONS, PLACE_ACTIONS};
use mandate_core::Stream;
use mandate_sim::places::{self, Location, NEIGHBORHOOD_ACTIONS};
use mandate_sim::player::knowledge::{Level, Subject};
use mandate_sim::policy::PolicyStatus;
use mandate_sim::world::{MunicipalityId, NeighborhoodId, PlaceId};
use mandate_sim::{Engine, State};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

/// A location in the player's town.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LocationDto {
    Home,
    Street { neighborhood: u32 },
    Place { place: u32 },
}

impl LocationDto {
    pub fn of(l: Location) -> Self {
        match l {
            Location::Home => LocationDto::Home,
            Location::Street { neighborhood } => LocationDto::Street {
                neighborhood: neighborhood.0,
            },
            Location::Place { place } => LocationDto::Place { place: place.0 },
        }
    }

    pub fn to_sim(self) -> Location {
        match self {
            LocationDto::Home => Location::Home,
            LocationDto::Street { neighborhood } => Location::Street {
                neighborhood: NeighborhoodId(neighborhood),
            },
            LocationDto::Place { place } => Location::Place {
                place: PlaceId(place),
            },
        }
    }
}

/// A location with what the client needs to name it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LocationView {
    pub location: LocationDto,
    /// Registry key: `home`, `streets`, or a place kind (`council_chamber`).
    /// Fluent: `place-<kind_key>`.
    pub kind_key: String,
    /// A generated proper name (e.g. "The Ashford Herald"), or empty.
    pub name: String,
    pub neighborhood: Option<u32>,
    pub neighborhood_name: String,
    /// For the centre (a neighbourhood with its town's name): the Fluent key
    /// that names it to the player in place of `neighborhood_name` ("Larkowick
    /// town centre"), with `neighborhood_args` (M6 note 19).
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub neighborhood_key: Option<String>,
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub neighborhood_args: Option<BTreeMap<String, String>>,
}

/// Where an action is done (auto-routing, DEC-079 rule 2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RouteView {
    /// Action key (`lobby`, `canvass`, `vote`, …).
    pub action: String,
    pub late_night: bool,
    /// Target neighbourhood for neighbourhood actions (canvass, town hall).
    pub neighborhood: Option<u32>,
    /// The best place for it: where the player is, if that works, else the
    /// first matching place in town. `None` if the town offers it nowhere.
    pub to: Option<LocationView>,
    /// The player is already there.
    pub here: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AlertView {
    /// Fluent key, e.g. `alert-vote-needed`.
    pub key: String,
    pub args: BTreeMap<String, String>,
}

/// A bill in the pocket: status and an approximate count only.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PocketBillView {
    pub id: u32,
    pub name_key: String,
    pub status: String,
    pub next_vote: Option<DateView>,
    pub next_stage_key: Option<String>,
    /// "About N of M": the expected yes count, rounded.
    pub about_yes: u32,
    pub seats: u32,
    pub needed: u32,
    pub sponsor_is_player: bool,
    /// The player may (and has not yet) declared a vote on it.
    pub can_vote: bool,
    pub my_vote: Option<String>,
    /// Knowledge in words (DEC-113): one sentence, only where a gap or an
    /// old reading bears on this bill (`crate::notebook`).
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<crate::pocket::TextView>,
}

/// Headline numbers in coarse form: rounded, as of the last monthly update.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct HeadlinesView {
    /// Unemployment, to the whole percent.
    pub unemployment: f64,
    /// Monthly rent, to 10.
    pub monthly_rent: f64,
    /// −1..1, to 0.05.
    pub gov_approval: f64,
    pub approval_ward: f64,
    pub awareness_ward: f64,
    pub approval_town: f64,
    /// The town's two strongest concerns (Fluent name keys, DEC-078), as
    /// known (DEC-091): none, the top one, or two.
    pub top_concern_keys: Vec<String>,
    pub concerns_knowledge: crate::knowledge::KnowledgeView,
}

/// The pocket (phone and notebook): always available (DEC-079 rule 3).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PocketView {
    pub date: DateView,
    pub turn: u32,
    pub turn_days: u32,
    pub slots_left: u32,
    pub slots_total: u32,
    pub slot_cap: u32,
    pub late_night_available: bool,
    pub money: f64,
    pub mandate: f64,
    pub tracks: TracksView,
    /// Visible work in office (0..100), DEC-076.
    pub record: f64,
    pub renomination_standing: f64,
    pub alerts: Vec<AlertView>,
    /// The player's live bills, then bills awaiting the player's vote.
    pub bills: Vec<PocketBillView>,
    pub next_election: Option<DateView>,
    pub days_to_election: Option<u32>,
    pub in_campaign: bool,
    pub headlines: HeadlinesView,
    pub location: LocationView,
    /// Where each action is done: town-wide actions once (and their late
    /// variant), neighbourhood actions once per neighbourhood.
    pub routes: Vec<RouteView>,
    /// Decisions waiting for the player, soonest deadline first (DEC-079:
    /// always listed here and answerable from here).
    pub decisions: Vec<crate::narrative::DecisionView>,
    /// The latest headlines in town (coarse: the newspaper has the rest).
    pub news: Vec<crate::media::ArticleView>,
    /// Scandals, cases, and where the career stands (DEC-006, DEC-034).
    pub legal: Option<crate::media::LegalView>,
    /// Subjects learned of this turn (the notebook's badge, DEC-091).
    pub notebook_new: u32,
    /// Asks and promises (DEC-097 §2): offers first, then accepted ones by
    /// due date (the Today block).
    pub asks: Vec<crate::asks::AskView>,
    /// The badge: offers to answer, plus asks and promises with something
    /// left to do that fall due within the turn.
    pub asks_due_soon: u32,
    /// Asks and promises resolved in the last four weeks, newest first (at
    /// most three): the Today block's last outcomes.
    pub asks_recent: Vec<crate::asks::AskView>,
    /// A campaign promise can be made now (`QueryDto::PromiseMenu`).
    pub can_promise: bool,
    /// What waits on the player, soonest first (DEC-113; Today's "Needs
    /// you"): decisions, asks to answer, votes to cast, asks due within
    /// the turn, and a known arc's beats in their last turn. The engine
    /// orders it; the UI sorts and counts nothing.
    pub needs_you: Vec<crate::pocket::NeedView>,
    /// The bar's "needs you" count: `needs_you.len()`.
    pub needs_count: u32,
    /// The running arcs the player knows, as rows (Today's "Under way"):
    /// the soonest clock first.
    pub arcs: Vec<crate::arcs::ArcSummaryView>,
    /// The next dated thing ahead (the empty state: "Nothing is waiting on
    /// you. Next: …"); none when nothing is dated.
    pub next_up: Option<crate::pocket::NextUpView>,
    /// This fortnight's calendar (DEC-114; Today's "This fortnight"): the
    /// events held on the day played, in the day's order, then any
    /// reported as having happened.
    pub fortnight: Vec<crate::calendar::CalendarEventView>,
    /// The next turn's event the player is expected at, with what staying
    /// away costs (shown a turn ahead).
    pub next_expected: Option<crate::calendar::CalendarEventView>,
}

/// One location on the town schematic, with what it offers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SpotView {
    pub location: LocationView,
    pub actions: Vec<String>,
    pub late_night: Vec<String>,
    pub services: Vec<String>,
    /// The visit verbs offered here, as Fluent name keys (places with
    /// purpose; `PlacePanelView.verbs` has the details).
    pub verbs: Vec<String>,
    pub here: bool,
    /// Why the place matters now, strongest first (DEC-113): a decision's
    /// scene is here, an ask's next need is done here, a known arc's beat
    /// is here, the player is here. Empty when nothing does. (Always
    /// sent; optional in the generated type so that older hand-built
    /// spots still type-check.)
    #[serde(default)]
    #[ts(as = "Option<Vec<crate::pocket::NowReasonView>>", optional)]
    pub now: Vec<crate::pocket::NowReasonView>,
    /// Its order among the town's places (0 first): those that matter
    /// now, then the rest as the town lists them. The UI sorts by nothing
    /// else. (Always sent; optional in the generated type, as `now`.)
    #[serde(default)]
    #[ts(as = "Option<u32>", optional)]
    pub rank: u32,
    /// Knowledge in words: one sentence, only where a fact changes what
    /// can be done here (`crate::notebook`).
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<crate::pocket::TextView>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TownHoodView {
    pub id: u32,
    pub name: String,
    /// For the centre (it has its town's name): the Fluent key that names it
    /// in place of `name` ("Larkowick town centre"), with `name_args`.
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_key: Option<String>,
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_args: Option<BTreeMap<String, String>>,
    /// Neighbourhood kind (Fluent `hood-<kind>`).
    pub kind: String,
    pub is_home: bool,
    pub in_ward: bool,
    /// The number of the local constituency (ward) this neighbourhood is in;
    /// the label is the constitution's (`ui-post-seat` / `ui-ward`).
    pub ward_number: Option<u32>,
    /// Home (in the home neighbourhood), the streets, then each place
    /// (hidden places only once found, DEC-090 D7).
    pub spots: Vec<SpotView>,
    /// Fluent name key of its strongest worry, once heard of (the map's
    /// concern overlay).
    pub top_concern_key: Option<String>,
    pub concern_knowledge: crate::knowledge::KnowledgeView,
    /// Knowledge in words (DEC-113): one sentence, only where not knowing
    /// its worries, or knowing them from an old reading, bears on
    /// something in hand (`crate::notebook`).
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<crate::pocket::TextView>,
}

/// The player's town as places, grouped by neighbourhood.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TownPlacesView {
    pub municipality: String,
    pub location: LocationDto,
    pub home_neighborhood: Option<u32>,
    pub neighborhoods: Vec<TownHoodView>,
    /// The town's character (DEC-077 §1): industry, crisis, broker, rival,
    /// priorities.
    pub character: Option<crate::characters::TownCharacterView>,
}

/// A verb of the visit action offered at a place (places with purpose):
/// its cost, whether it can be done now, and its risk (DEC-073: the odds
/// shown are the odds rolled).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct VerbOfferView {
    /// Verb id (`verb.tour_works`): `ActionDto::Visit { verb }`.
    pub verb: String,
    /// Fluent key of its label.
    pub name_key: String,
    /// Time in half slots (1: half a slot, 2: a slot).
    pub halves: u32,
    pub health: f64,
    /// It may also be done in the late-night slot.
    pub late_night: bool,
    /// Whether it can be done now (the player here, the time of day, the
    /// town's situation, who is here, the time left).
    pub available: bool,
    /// Why not, as a Fluent key (`visit-blocked-…`), when it cannot.
    pub blocked_key: Option<String>,
    /// Chance it goes wrong, where it can (computed as the command rolls it).
    pub risk: Option<f64>,
}

/// An action offered at a place, with its cost and risk (DEC-073).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct OfferView {
    pub action: String,
    pub late_night: bool,
    pub slots: u32,
    pub health: f64,
    /// Chance it goes wrong (a gaffe, a hostile crowd), where it can.
    pub risk: Option<f64>,
    /// A shady action's payoff and exposure (DEC-073 risk line).
    pub shady: Option<crate::media::ShadyOfferView>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct HoodDataView {
    pub id: u32,
    pub name: String,
    pub kind: String,
    pub population: f64,
    pub gov_approval: f64,
    pub player_awareness: f64,
    pub player_approval: f64,
    pub town_hall_risk: f64,
    /// Fluent name key of the neighbourhood's strongest concern, once the
    /// player has heard of its worries (DEC-091).
    pub top_concern_key: Option<String>,
}

/// A named plant as a place panel and the town's data show it (DEC-110,
/// P1): parts and Fluent keys, never prose. The client composes the name
/// from `name_key` (`plant-name-<form>`, or `plant-old` once it has shut),
/// `stem` and `works_key`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PlantView {
    pub id: u32,
    /// The place it is (a `Location::Place` id).
    pub place: u32,
    pub name_key: String,
    pub stem: String,
    /// The works' noun ("cannery").
    pub works_key: String,
    /// What it makes, a mass noun.
    pub product_key: String,
    pub industry_key: String,
    /// Its town's name (what a "why" line names).
    pub town: String,
    /// `running` | `short_time` | `threatened` | `closed`.
    pub status: String,
    /// `plant-status-<status>`, with `month_key` and `year`.
    pub status_key: String,
    /// Since when (short time, closed), or the closing date (under notice):
    /// the month (1-12) and the year. None while it runs full time.
    pub month: Option<u32>,
    pub year: Option<u32>,
    /// On short time or worse: why (`plant-why-<factor>`: the weakest term
    /// of its industry's demand), and for supply the upstream industry as
    /// residents name it.
    pub why_key: Option<String>,
    pub upstream_key: Option<String>,
    /// About how many work there, to two figures ("about 210"). 0 once shut.
    pub jobs_about: u32,
}

/// A count to two significant figures.
fn about(n: u32) -> u32 {
    let mut unit = 1;
    while n / unit >= 100 {
        unit *= 10;
    }
    (n + unit / 2) / unit * unit
}

/// A plant's view.
pub fn plant_view(
    engine: &Engine,
    plant: mandate_sim::economy::plants::PlantId,
) -> Option<PlantView> {
    use mandate_sim::economy::plants::{self, PlantStatus};
    let st = engine.state();
    let content = engine.content();
    let f = plants::facts(&st.economy, &st.world, content, plant)?;
    let (_, product) = content.product(&f.product)?;
    let industry = content.industry(&f.industry)?;
    let date = match f.status {
        PlantStatus::Running => None,
        PlantStatus::ShortTime { since } | PlantStatus::Closed { since, .. } => Some(since),
        PlantStatus::Threatened { closes_on, .. } => Some(closes_on),
    }
    .map(|d| d.calendar());
    let troubled = !matches!(f.status, PlantStatus::Running);
    let talk = |k: &str| {
        content
            .industry(k)
            .map(|d| d.talk_key.clone().unwrap_or_else(|| d.name_key.clone()))
            .or_else(|| {
                content
                    .sector(k)
                    .map(|d| d.talk_key.clone().unwrap_or_else(|| d.name_key.clone()))
            })
    };
    Some(PlantView {
        id: f.plant.0,
        place: f.place.0,
        name_key: if matches!(f.status, PlantStatus::Closed { .. }) {
            "plant-old".into()
        } else {
            format!("plant-name-{}", f.name.form.key())
        },
        stem: f.name.stem.clone(),
        works_key: product.works_key.clone(),
        product_key: product.product_key.clone(),
        industry_key: industry.name_key.clone(),
        town: st
            .world
            .municipalities
            .get(f.municipality.index())
            .map(|m| m.name.clone())
            .unwrap_or_default(),
        status: f.status.key().into(),
        status_key: format!("plant-status-{}", f.status.key()),
        month: date.map(|c| c.month),
        year: date.map(|c| c.year),
        why_key: troubled.then(|| format!("plant-why-{}", f.factor.key())),
        upstream_key: f.upstream.as_deref().and_then(talk),
        jobs_about: about(f.jobs),
    })
}

/// Detailed town data (the statistics desk).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TownDataView {
    pub population: f64,
    pub unemployment: f64,
    /// Monthly, oldest first (last 12 months).
    pub unemployment_history: Vec<f64>,
    pub typical_income: f64,
    pub monthly_rent: f64,
    pub home_price: f64,
    pub vacancy: f64,
    pub sectors: Vec<SectorJobsView>,
    /// The town's named plants, open and shut (DEC-110): its main
    /// employers, the largest first.
    pub plants: Vec<PlantView>,
    /// Live at the desk once the player has heard of the town's worries
    /// (DEC-091); empty before.
    pub concerns: Vec<ConcernView>,
    pub neighborhoods: Vec<HoodDataView>,
    /// Communities with adults in town and their shares (DEC-075).
    pub communities: Vec<crate::map::CommunityShareView>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProgrammeCostView {
    pub id: u32,
    pub name_key: String,
    pub status: String,
    pub est_annual_cost: f64,
    pub funded_pct: f64,
}

/// Budget papers (the clerk's office).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct BudgetPapersView {
    pub budget: f64,
    pub debt: f64,
    pub last_balance: f64,
    pub programmes: Vec<ProgrammeCostView>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct InternalShareView {
    pub name: String,
    pub party: Option<u32>,
    pub share: f64,
    pub is_player: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct InternalRaceView {
    /// The post contested.
    pub post: crate::post::PostDto,
    pub candidates: Vec<InternalShareView>,
}

/// The party's internal polling: tighter than public polls (DEC-079).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct InternalPollView {
    /// The player's race, during a campaign.
    pub race: Option<InternalRaceView>,
    pub ward_awareness: f64,
    pub ward_approval: f64,
    /// Typical error of these figures (share points, 0..1).
    pub margin: f64,
}

/// What people nearby are talking about: qualitative early warnings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct OverheardView {
    pub neighborhood: u32,
    pub neighborhood_name: String,
    /// What people are saying this week, grounded in facts.
    pub lines: Vec<OverheardLineView>,
}

/// One overheard line: a Fluent key with arguments (names and facts;
/// argument names ending in `_key` are Fluent keys, DEC-054).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct OverheardLineView {
    pub id: String,
    pub key: String,
    pub args: std::collections::BTreeMap<String, String>,
}

/// A place panel: what can be done and learned there.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PlacePanelView {
    pub location: LocationView,
    pub here: bool,
    /// The named plant this place is (DEC-110): the open one, else the
    /// last that shut here.
    pub plant: Option<PlantView>,
    pub actions: Vec<OfferView>,
    pub services: Vec<String>,
    /// The visit verbs offered here (places with purpose), in the kind's order.
    pub verbs: Vec<VerbOfferView>,
    /// Service payloads: only filled while the player is here.
    pub town_data: Option<TownDataView>,
    pub budget: Option<BudgetPapersView>,
    pub internal_poll: Option<InternalPollView>,
    pub overheard: Option<OverheardView>,
    /// The newspaper (DEC-034): only while the player is here.
    pub newspaper: Option<crate::media::NewspaperView>,
    /// Who is here (DEC-018): characters who work or sit here, then
    /// passers-by. Listed wherever the panel is opened; meeting someone
    /// needs the player there.
    pub people: Vec<crate::characters::PersonRefView>,
    /// How each of `people` is here, in the same order (DEC-088 interiors:
    /// the room puts people on its spots by this, never by office or name):
    /// `presiding` (heads the body or office that sits here), `member` (sits
    /// in that body), `works` (works here), or `visitor`.
    pub here_as: Vec<String>,
    /// The interior the player walks into from the street (DEC-088), when
    /// the place kind names one: its recipe (`interiors.jsonc`, the highest
    /// version of the kind's `interior` id) as the content's JSON, which the
    /// client composes the room from (presentation data the engine
    /// validates and never reads; the art pack's `InteriorRecipe` shape).
    #[ts(type = "unknown")]
    pub interior: Option<serde_json::Value>,
    /// Pending decisions whose scene happens here (DEC-079: going there adds
    /// the conversation; they can always be answered from the pocket).
    pub decisions_here: Vec<crate::narrative::DecisionView>,
    /// On a neighbourhood's streets: its faults, open, being repaired and
    /// lately repaired (DEC-114). (Always sent; optional in the generated
    /// type, as `SpotView.now`.)
    #[serde(default)]
    #[ts(as = "Option<Vec<crate::upkeep::FaultView>>", optional)]
    pub faults: Vec<crate::upkeep::FaultView>,
}

pub fn location_view(st: &State, loc: Location) -> LocationView {
    let hood = places::hood_of(st, loc);
    let centre = hood.and_then(|h| crate::talk::hood_centre_label(st, h));
    LocationView {
        location: LocationDto::of(loc),
        kind_key: places::kind_key(st, loc).into(),
        name: match loc {
            Location::Place { place } => st
                .world
                .places
                .get(place.index())
                .map(|p| p.name.clone())
                .unwrap_or_default(),
            _ => String::new(),
        },
        neighborhood: hood.map(|h| h.0),
        neighborhood_name: hood
            .and_then(|h| st.world.neighborhoods.get(h.index()))
            .map(|n| n.name.clone())
            .unwrap_or_default(),
        neighborhood_key: centre.as_ref().map(|(k, _)| k.clone()),
        neighborhood_args: centre.map(|(_, a)| a),
    }
}

fn weighted<F: Fn(&mandate_sim::population::Cohort) -> f64>(
    st: &State,
    hoods: &[usize],
    f: F,
) -> f64 {
    let mut acc = 0.0;
    let mut w = 0.0;
    for &h in hoods {
        for c in &st.population.hoods[h].cohorts {
            let weight = c.count as f64;
            acc += f(c) * weight;
            w += weight;
        }
    }
    if w > 0.0 { acc / w } else { 0.0 }
}

fn town_hoods(st: &State, m: MunicipalityId) -> Vec<usize> {
    st.world.hoods_of(m).collect()
}

/// Every route the agenda needs.
pub fn routes(engine: &Engine) -> Vec<RouteView> {
    let st = engine.state();
    let content = engine.content();
    let Some(pl) = st.player.as_ref() else {
        return Vec::new();
    };
    let hoods: Vec<u32> = town_hoods(st, pl.municipality)
        .into_iter()
        .map(|h| h as u32)
        .collect();
    let mut out = Vec::new();
    for key in PLACE_ACTIONS {
        let scopes: Vec<Option<u32>> = if NEIGHBORHOOD_ACTIONS.contains(&key) {
            hoods.iter().map(|h| Some(*h)).collect()
        } else {
            vec![None]
        };
        for late_night in [false, true] {
            if late_night && !LATE_NIGHT_ACTIONS.contains(&key) {
                continue;
            }
            for hood in &scopes {
                let to = places::best_for(st, content, key, late_night, *hood);
                out.push(RouteView {
                    action: key.into(),
                    late_night,
                    neighborhood: *hood,
                    here: to == Some(pl.location),
                    to: to.map(|l| location_view(st, l)),
                });
            }
        }
    }
    out
}

pub fn pocket_view(engine: &Engine) -> Option<PocketView> {
    let st = engine.state();
    let content = engine.content();
    let pl = st.player.as_ref()?;
    let me = st.politics.get(pl.politician)?;
    let m = pl.municipality;
    let hoods = town_hoods(st, m);
    let ward_hoods = mandate_sim::policy::ward_hoods(&st.world, pl.ward);
    let campaign = campaign_view(engine);
    let mut bills: Vec<PocketBillView> = st
        .policy
        .in_municipality(m.0)
        .filter(|p| {
            matches!(
                p.status,
                PolicyStatus::Draft | PolicyStatus::UnderConsideration
            )
        })
        .filter_map(|p| bill_view(engine, p))
        .filter(|b| b.sponsor_is_player || b.can_vote)
        .map(|b| PocketBillView {
            id: b.id,
            name_key: b.name_key,
            status: b.status,
            next_vote: b.next_vote,
            next_stage_key: b.next_stage_key,
            about_yes: b.expected_yes.round().max(0.0) as u32,
            seats: b.seats,
            needed: b.needed,
            sponsor_is_player: b.sponsor_is_player,
            can_vote: b.can_vote,
            my_vote: b.my_vote,
            note: b.note,
        })
        .collect();
    bills.sort_by_key(|b| (!b.sponsor_is_player, std::cmp::Reverse(b.id)));
    let mut alerts = Vec::new();
    let alert = |key: &str, args: &[(&str, String)]| AlertView {
        key: key.into(),
        args: args
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect(),
    };
    let votes = bills.iter().filter(|b| b.can_vote).count();
    if votes > 0 {
        alerts.push(alert("alert-vote-needed", &[("n", votes.to_string())]));
    }
    if pl.slot_cap() < pl.standard_slots {
        alerts.push(alert(
            "alert-tired",
            &[("n", (pl.standard_slots - pl.slot_cap()).to_string())],
        ));
    }
    let renom = content.politics.vote.renomination_standing;
    if (pl.tracks.standing as f64) < renom + 10.0 {
        alerts.push(alert("alert-party-patience", &[("n", renom.to_string())]));
    }
    if pl.tracks.legal_jeopardy >= 50.0 {
        alerts.push(alert("alert-legal", &[]));
    }
    // A story about you is running; a case against you is open (DEC-034, DEC-006).
    for sc in st.media.running_about(pl.politician) {
        alerts.push(alert(
            "alert-scandal",
            &[("fact_key", format!("fact-{}", sc.kind.replace('_', "-")))],
        ));
    }
    for c in st.justice.open_against(pl.politician) {
        alerts.push(alert(
            "alert-case",
            &[("stage_key", format!("case-stage-{}", c.stage.key()))],
        ));
    }
    let decisions = crate::narrative::pending_decisions(engine);
    if !decisions.is_empty() {
        alerts.push(alert(
            "alert-decisions",
            &[("n", decisions.len().to_string())],
        ));
    }
    // DEC-006: every track in danger is warned about (the director's warning
    // is also in the story so far).
    let sched = mandate_sim::scheduler::Reads::of_state(st, content);
    for (track, _, danger, _) in mandate_sim::narrative::director::track_dangers(&sched) {
        if danger {
            alerts.push(alert(
                "alert-track-warning",
                &[("track_key", format!("track-{track}"))],
            ));
        }
    }
    if let Some(c) = &campaign
        && c.in_campaign
        && let Some(d) = c.days_to_election
    {
        alerts.push(alert("alert-campaign", &[("n", d.to_string())]));
    }
    // The party did not nominate the player for the post they filed for
    // (play note 20): said for the whole campaign.
    if let Some(r) = campaign.as_ref().and_then(|c| c.refusal.clone()) {
        alerts.push(AlertView {
            key: r.key,
            args: r.args,
        });
    }
    let econ = st.economy.munis.get(m.index());
    let gate = Gate::of(engine);
    let concerns_subject = Subject::TownConcerns { town: m };
    let concerns = gate_concerns(
        gate.level(&concerns_subject),
        concerns_for(st, content, mandate_sim::concerns::Scope::town(m)),
        false,
        2,
    );
    let c5 = |v: f64| coarse(v, 0.05);
    let needs_you = crate::pocket::needs_you(engine);
    crate::pocket::drop_covered(&mut alerts, &needs_you);
    Some(PocketView {
        date: DateView::of(st.calendar.date),
        turn: st.calendar.turn,
        turn_days: engine.turn_days(),
        slots_left: pl.slots_left() as u32,
        slots_total: pl.standard_slots as u32,
        slot_cap: pl.slot_cap() as u32,
        late_night_available: !pl.late_night_used,
        money: pl.money.units() as f64,
        mandate: pl.mandate as f64,
        tracks: TracksView {
            viability: pl.tracks.viability as f64,
            legal_jeopardy: pl.tracks.legal_jeopardy as f64,
            threat: pl.tracks.threat as f64,
            standing: pl.tracks.standing as f64,
            health: pl.tracks.health as f64,
        },
        record: me.record as f64 * 100.0,
        renomination_standing: renom,
        alerts,
        bills,
        next_election: campaign.as_ref().and_then(|c| c.election_day.clone()),
        days_to_election: campaign.as_ref().and_then(|c| c.days_to_election),
        in_campaign: campaign.as_ref().is_some_and(|c| c.in_campaign),
        decisions,
        headlines: HeadlinesView {
            unemployment: econ.map(|e| coarse(e.unemployment, 0.01)).unwrap_or(0.0),
            monthly_rent: econ.map(|e| coarse(e.housing.rent, 10.0)).unwrap_or(0.0),
            gov_approval: c5(weighted(st, &hoods, |c| c.gov_approval as f64)),
            approval_ward: c5(weighted(st, &ward_hoods, |c| c.player_approval as f64)),
            awareness_ward: c5(weighted(st, &ward_hoods, |c| c.player_awareness as f64)),
            approval_town: c5(weighted(st, &hoods, |c| c.player_approval as f64)),
            top_concern_keys: concerns.into_iter().map(|c| c.name_key).collect(),
            concerns_knowledge: gate.view(&concerns_subject),
        },
        location: location_view(st, pl.location),
        routes: routes(engine),
        news: crate::media::newspaper_view(engine, 3)
            .map(|n| n.articles)
            .unwrap_or_default(),
        legal: crate::media::legal_view(engine),
        notebook_new: crate::knowledge::notebook_new(engine),
        asks: crate::asks::open_asks(engine),
        asks_due_soon: crate::asks::asks_due_soon(engine),
        asks_recent: crate::asks::recent_outcomes(engine, 28, 3),
        can_promise: crate::asks::can_promise(engine),
        needs_count: needs_you.len() as u32,
        needs_you,
        arcs: crate::arcs::arc_summaries(engine),
        next_up: crate::pocket::next_up(engine),
        fortnight: crate::calendar::fortnight(engine),
        next_expected: crate::calendar::next_expected(engine),
    })
}

fn spot(engine: &Engine, loc: Location) -> SpotView {
    let st = engine.state();
    let content = engine.content();
    let acts = places::actions_at(st, content, loc);
    SpotView {
        location: location_view(st, loc),
        actions: acts
            .iter()
            .filter(|(_, late)| !late)
            .map(|(k, _)| k.to_string())
            .collect(),
        late_night: acts
            .iter()
            .filter(|(_, late)| *late)
            .map(|(k, _)| k.to_string())
            .collect(),
        services: places::services_at(st, content, loc)
            .into_iter()
            .map(String::from)
            .collect(),
        verbs: mandate_sim::visits::verbs_at(st, content, loc)
            .into_iter()
            .map(|v| v.name_key.clone())
            // An arc's visit beats here (DEC-109).
            .chain(
                mandate_sim::arcs::beats::visits_at(st, content, loc)
                    .into_iter()
                    .map(|o| o.verb_key),
            )
            .collect(),
        here: st.player.as_ref().is_some_and(|p| p.location == loc),
        // Filled for the whole town at once (`pocket::rank_spots`).
        now: Vec::new(),
        rank: 0,
        note: None,
    }
}

/// The verbs offered at a location, with what each costs and risks now.
fn verb_offers(engine: &Engine, loc: Location) -> Vec<VerbOfferView> {
    use mandate_content::visits::VisitTime;
    let st = engine.state();
    let content = engine.content();
    let here = st.player.as_ref().is_some_and(|p| p.location == loc);
    let place = mandate_sim::visits::place_at(st, loc).map(|p| p.id);
    mandate_sim::visits::verbs_at(st, content, loc)
        .into_iter()
        .map(|v| {
            let why = if here {
                mandate_sim::visits::availability(st, content, loc, v, false)
                    .err()
                    .map(|b| b.key().to_string())
            } else {
                Some("visit-blocked-away".to_string())
            };
            VerbOfferView {
                verb: v.id.clone(),
                name_key: v.name_key.clone(),
                halves: match v.time {
                    VisitTime::Half => 1,
                    VisitTime::Slot => 2,
                },
                health: v.health,
                late_night: v.late_night,
                available: why.is_none(),
                blocked_key: why,
                risk: place.and_then(|p| mandate_sim::visits::risk(st, content, p, v)),
            }
        })
        // An arc's visit beats here (DEC-109): verbs like any other, done
        // with the same `Visit` action.
        .chain(
            mandate_sim::arcs::beats::visits_at(st, content, loc)
                .into_iter()
                .map(|o| {
                    let why = if !here {
                        Some("visit-blocked-away".to_string())
                    } else if !mandate_sim::arcs::beats::has_time(st, o.time) {
                        Some("visit-blocked-time".to_string())
                    } else {
                        None
                    };
                    VerbOfferView {
                        verb: o.verb,
                        name_key: o.verb_key,
                        halves: match o.time {
                            VisitTime::Half => 1,
                            VisitTime::Slot => 2,
                        },
                        health: 0.0,
                        late_night: false,
                        available: why.is_none(),
                        blocked_key: why,
                        risk: None,
                    }
                }),
        )
        .collect()
}

pub fn town_places_view(engine: &Engine) -> Option<TownPlacesView> {
    let st = engine.state();
    let content = engine.content();
    let gate = Gate::of(engine);
    let pl = st.player.as_ref()?;
    let mu = st.world.municipalities.get(pl.municipality.index())?;
    let home = places::home_neighborhood(st);
    let ward: Vec<usize> = mandate_sim::policy::ward_hoods(&st.world, pl.ward);
    let mut neighborhoods: Vec<TownHoodView> = mu
        .neighborhoods
        .iter()
        .filter_map(|h| st.world.neighborhoods.get(h.index()))
        .map(|n| {
            let mut spots = Vec::new();
            if home == Some(n.id) {
                spots.push(spot(engine, Location::Home));
            }
            spots.push(spot(engine, Location::Street { neighborhood: n.id }));
            for pid in &mu.places {
                if st
                    .world
                    .places
                    .get(pid.index())
                    .is_some_and(|p| p.neighborhood == n.id)
                    && crate::knowledge::place_listed(&gate, st, *pid)
                {
                    spots.push(spot(engine, Location::Place { place: *pid }));
                }
            }
            let concern_subject = Subject::HoodConcerns { hood: n.id };
            let top_concern_key = (gate.level(&concern_subject) >= Level::Heard)
                .then(|| {
                    concerns_for(
                        st,
                        content,
                        mandate_sim::concerns::Scope::Neighbourhood(n.id),
                    )
                    .into_iter()
                    .next()
                    .map(|c| c.name_key)
                })
                .flatten();
            let centre = crate::talk::hood_centre_label(st, n.id);
            TownHoodView {
                id: n.id.0,
                name: n.name.clone(),
                name_key: centre.as_ref().map(|(k, _)| k.clone()),
                name_args: centre.map(|(_, a)| a),
                kind: crate::office::snake(&n.kind),
                is_home: home == Some(n.id),
                in_ward: ward.contains(&n.id.index()),
                ward_number: st
                    .world
                    .constituencies
                    .iter()
                    .find(|c| {
                        c.tier == mandate_sim::world::Tier::Local
                            && c.jurisdiction == n.municipality.0
                            && c.neighborhoods.contains(&n.id)
                    })
                    .map(|c| c.number),
                spots,
                top_concern_key,
                concern_knowledge: gate.view(&concern_subject),
                note: None,
            }
        })
        .collect();
    // Why each place matters now, its order, and the notes (DEC-113).
    crate::pocket::rank_spots(engine, &mut neighborhoods);
    Some(TownPlacesView {
        municipality: mu.name.clone(),
        location: LocationDto::of(pl.location),
        home_neighborhood: home.map(|h| h.0),
        neighborhoods,
        character: crate::characters::town_character_view(engine, pl.municipality.index()),
    })
}

fn offer_view(engine: &Engine, loc: Location, key: &str, late_night: bool) -> OfferView {
    let st = engine.state();
    let cost = engine.content().tuning.action_cost(key);
    let risk = match key {
        "press" => Some(mandate_sim::actions::press_gaffe_risk(st, engine.content())),
        "town_hall" => places::hood_of(st, loc)
            .map(|h| mandate_sim::actions::town_hall_risk(st, engine.content(), h.0)),
        _ => None,
    };
    let shady = mandate_content::SHADY_ACTIONS
        .contains(&key)
        .then(|| {
            crate::media::shady_offers(engine)
                .into_iter()
                .find(|x| x.action == key)
        })
        .flatten();
    // Free commands (votes, drafting, filing) take no slot.
    let free = matches!(key, "vote" | "draft" | "file" | "withdraw");
    OfferView {
        action: key.into(),
        late_night,
        slots: if free || late_night {
            0
        } else {
            cost.slots as u32
        },
        health: if late_night {
            cost.health + 8.0
        } else {
            cost.health
        },
        risk,
        shady,
    }
}

fn town_data(engine: &Engine) -> Option<TownDataView> {
    let st = engine.state();
    let content = engine.content();
    let pl = st.player.as_ref()?;
    let m = pl.municipality;
    let e = st.economy.munis.get(m.index())?;
    let hoods = town_hoods(st, m);
    let hist = &e.history;
    let start = hist.len().saturating_sub(12);
    // At the desk the figures are live (DEC-079), for what the player has
    // at least heard of (DEC-091).
    let gate = Gate::of(engine);
    let heard = |s: Subject| gate.level(&s) >= Level::Heard;
    Some(TownDataView {
        population: st.population.municipality_total(&st.world, m) as f64,
        unemployment: e.unemployment,
        unemployment_history: hist[start..]
            .iter()
            .map(|h| h.unemployment as f64)
            .collect(),
        typical_income: e.class_income[1],
        monthly_rent: e.housing.rent,
        home_price: e.housing.price,
        vacancy: e.housing.vacancy,
        sectors: content
            .sectors
            .iter()
            .zip(&e.sectors)
            .map(|(d, s)| SectorJobsView {
                id: d.id.clone(),
                name_key: d.name_key.clone(),
                jobs: s.jobs.round(),
                wage: s.wage.round(),
            })
            .collect(),
        plants: {
            let mut v: Vec<PlantView> = mandate_sim::economy::plants::of_town(&st.economy, m)
                .filter_map(|p| plant_view(engine, p.id))
                .collect();
            v.sort_by(|a, b| b.jobs_about.cmp(&a.jobs_about).then(a.id.cmp(&b.id)));
            v
        },
        concerns: if heard(Subject::TownConcerns { town: m }) {
            concerns_for(st, content, mandate_sim::concerns::Scope::town(m))
        } else {
            Vec::new()
        },
        communities: crate::map::community_shares(engine, m),
        neighborhoods: hoods
            .iter()
            .filter_map(|&h| {
                let n = st.world.neighborhoods.get(h)?;
                Some(HoodDataView {
                    id: n.id.0,
                    name: n.name.clone(),
                    kind: crate::office::snake(&n.kind),
                    population: st.population.hood_total(h) as f64,
                    gov_approval: weighted(st, &[h], |c| c.gov_approval as f64),
                    player_awareness: weighted(st, &[h], |c| c.player_awareness as f64),
                    player_approval: weighted(st, &[h], |c| c.player_approval as f64),
                    town_hall_risk: mandate_sim::actions::town_hall_risk(st, content, h as u32),
                    top_concern_key: heard(Subject::HoodConcerns { hood: n.id })
                        .then(|| {
                            concerns_for(
                                st,
                                content,
                                mandate_sim::concerns::Scope::Neighbourhood(n.id),
                            )
                            .first()
                            .map(|c| c.name_key.clone())
                        })
                        .flatten(),
                })
            })
            .collect(),
    })
}

fn budget_papers(engine: &Engine) -> Option<BudgetPapersView> {
    let st = engine.state();
    let pl = st.player.as_ref()?;
    let m = pl.municipality;
    let t = st.finance.local.get(m.index())?;
    Some(BudgetPapersView {
        budget: t.budget_total().units() as f64,
        debt: t.debt.units() as f64,
        last_balance: t.last_year.balance.units() as f64,
        programmes: st
            .policy
            .in_municipality(m.0)
            .filter(|p| matches!(p.status, PolicyStatus::Enacted | PolicyStatus::InEffect))
            .filter_map(|p| {
                let t = mandate_sim::policy::resolve(engine.content(), &engine.state().policy, p)?;
                Some(ProgrammeCostView {
                    id: p.id.0,
                    name_key: t.name_key.clone(),
                    status: crate::office::snake(&p.status),
                    est_annual_cost: mandate_sim::actions::estimate_cost(st, engine.content(), p),
                    funded_pct: st.finance.funded_share(p.id.0) * 100.0,
                })
            })
            .collect(),
    })
}

/// Internal polls: the model's expected shares now, plus small noise from a
/// keyed RNG per week (views never advance a stream; DEC-041).
fn internal_poll(engine: &Engine) -> Option<InternalPollView> {
    const SD: f64 = 0.015;
    let st = engine.state();
    let content = engine.content();
    let pl = st.player.as_ref()?;
    let week = (st.calendar.date.0 / 7) as u64;
    let mut rng = st.rng.keyed(Stream::Polling, &[0x1a7e_9011, week]);
    let reads = mandate_sim::scheduler::Reads::of_state(st, content);
    let el = &st.politics.elections;
    let race = el
        .races
        .iter()
        .enumerate()
        .find(|(_, r)| r.candidates.iter().any(|c| c.politician == pl.politician));
    let race = race.map(|(ri, r)| {
        let mut rr = st
            .rng
            .keyed(Stream::Polling, &[0x1a7e_9011, week, ri as u64]);
        let truth = mandate_sim::politics::elections::expected_shares(&st.politics, &reads, r);
        let mut s: Vec<f64> = truth
            .iter()
            .map(|p| (p + rr.normal(0.0, SD)).max(0.005))
            .collect();
        let sum: f64 = s.iter().sum::<f64>().max(1e-9);
        for v in s.iter_mut() {
            *v /= sum;
        }
        InternalRaceView {
            post: crate::post::post_dto(engine, r.seat),
            candidates: r
                .candidates
                .iter()
                .zip(s)
                .map(|(c, share)| InternalShareView {
                    name: st
                        .politics
                        .get(c.politician)
                        .map(|p| p.full_name())
                        .unwrap_or_default(),
                    party: c.party.map(|p| p.0),
                    share,
                    is_player: c.politician == pl.politician,
                })
                .collect(),
        }
    });
    let ward = mandate_sim::policy::ward_hoods(&st.world, pl.ward);
    let aw = weighted(st, &ward, |c| c.player_awareness as f64);
    let ap = weighted(st, &ward, |c| c.player_approval as f64);
    Some(InternalPollView {
        race,
        ward_awareness: (aw + rng.normal(0.0, SD)).clamp(0.0, 1.0),
        ward_approval: (ap + rng.normal(0.0, SD)).clamp(-1.0, 1.0),
        margin: SD * 2.0,
    })
}

fn overheard(engine: &Engine, loc: Location) -> Option<OverheardView> {
    let st = engine.state();
    st.player.as_ref()?;
    let h = places::hood_of(st, loc)?;
    let n = st.world.neighborhoods.get(h.index())?;
    let content = engine.content();
    let lines = mandate_sim::overheard::lines(st, content, loc)
        .into_iter()
        .map(|l| {
            let mut args = l.args;
            // A retold story carries its headline (and the outlet's name).
            if let Some(a) = l
                .story
                .and_then(|id| st.media.stories.iter().find(|s| s.id == id))
                .and_then(|s| crate::media::article(engine, s))
            {
                for (k, v) in a.args {
                    args.entry(k).or_insert(v);
                }
                args.insert("headline_key".into(), a.headline_key);
                args.insert("outlet_key".into(), a.outlet.name_key);
                args.insert("outlet_place".into(), a.outlet.place);
            }
            OverheardLineView {
                id: l.id,
                key: l.key,
                args,
            }
        })
        .collect();
    Some(OverheardView {
        neighborhood: h.0,
        neighborhood_name: n.name.clone(),
        lines,
    })
}

/// How each person present is here (`PlacePanelView.here_as`): presentation
/// for the interior's spots. It follows `ambient::stored_at`, which places
/// officials by the posts whose body or office works at a place kind
/// (DEC-098): an office's holder there is `presiding`, a body's member there
/// is a `member`; no office or body is named.
fn here_as(st: &State, loc: Location, people: &[crate::characters::PersonRefView]) -> Vec<String> {
    use mandate_sim::world::Jurisdiction;
    let place = match loc {
        Location::Place { place } => st.world.places.get(place.index()).map(|p| (place, p)),
        _ => None,
    };
    let pol = &st.politics;
    people
        .iter()
        .map(|p| {
            let c =
                p.id.and_then(|id| pol.characters.get(mandate_sim::characters::CharacterId(id)));
            let (Some(c), Some((pid, pl))) = (c, place) else {
                return "visitor";
            };
            let at = Jurisdiction::town(pl.municipality);
            if let Some(who) = c.politician {
                if pol
                    .offices
                    .iter()
                    .any(|o| o.at == at && o.works_at == Some(pl.kind) && o.holder == Some(who))
                {
                    return "presiding";
                }
                if pol
                    .bodies
                    .iter()
                    .any(|b| b.at == at && b.works_at == Some(pl.kind) && b.members.contains(&who))
                {
                    return "member";
                }
            }
            if c.named.as_deref().is_some_and(|n| n.place == Some(pid)) {
                "works"
            } else {
                "visitor"
            }
        })
        .map(String::from)
        .collect()
}

pub fn place_panel_view(engine: &Engine, loc: LocationDto) -> Option<PlacePanelView> {
    let st = engine.state();
    let content = engine.content();
    let pl = st.player.as_ref()?;
    let loc = loc.to_sim();
    places::validate_move(st, loc).ok()?;
    let here = pl.location == loc;
    let services: Vec<String> = places::services_at(st, content, loc)
        .into_iter()
        .map(String::from)
        .collect();
    let has = |s: &str| here && services.iter().any(|x| x == s);
    let people = crate::characters::people_at(engine, loc);
    Some(PlacePanelView {
        location: location_view(st, loc),
        here,
        plant: match loc {
            Location::Place { place } => mandate_sim::economy::plants::at_place(&st.economy, place)
                .and_then(|p| plant_view(engine, p.id)),
            _ => None,
        },
        actions: places::actions_at(st, content, loc)
            .into_iter()
            .map(|(k, late)| offer_view(engine, loc, k, late))
            .collect(),
        verbs: verb_offers(engine, loc),
        town_data: if has("town_data_detailed") {
            town_data(engine)
        } else {
            None
        },
        budget: if has("budget_papers") {
            budget_papers(engine)
        } else {
            None
        },
        internal_poll: if has("internal_polls") {
            internal_poll(engine)
        } else {
            None
        },
        overheard: if has("overheard") {
            overheard(engine, loc)
        } else {
            None
        },
        newspaper: if has("newspaper") {
            crate::media::newspaper_view(engine, 24)
        } else {
            None
        },
        here_as: here_as(st, loc, &people),
        interior: content
            .place_kind(places::kind_key(st, loc))
            .and_then(|k| k.interior.as_deref())
            .and_then(|id| {
                content
                    .interiors
                    .iter()
                    .filter(|r| r.id == id)
                    .max_by_key(|r| r.version)
            })
            .and_then(|r| serde_json::to_value(r).ok()),
        people,
        faults: match loc {
            Location::Street { neighborhood } => crate::upkeep::faults_in(engine, neighborhood),
            _ => Vec::new(),
        },
        decisions_here: crate::narrative::pending_decisions(engine)
            .into_iter()
            .filter(|d| {
                d.scene
                    .place
                    .as_ref()
                    .is_some_and(|p| p.location == LocationDto::of(loc))
            })
            .collect(),
        services,
    })
}
