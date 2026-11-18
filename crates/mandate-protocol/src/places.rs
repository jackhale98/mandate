//! Places as the interface (M5a, DEC-079): the player's location, the town's
//! places, the pocket (always available, coarse), and place panels (precise
//! information where a place offers it). The engine decides precision; the
//! client only renders.

use crate::campaign::campaign_view;
use crate::map::SectorJobsView;
use crate::office::{ConcernView, TracksView, bill_view, coarse, concerns_for};
use crate::views::DateView;
use mandate_content::{LATE_NIGHT_ACTIONS, PLACE_ACTIONS};
use mandate_core::Stream;
use mandate_sim::places::{self, Location, NEIGHBORHOOD_ACTIONS};
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
    /// The town's two strongest concerns (Fluent name keys, DEC-078).
    pub top_concern_keys: Vec<String>,
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
}

/// One location on the town schematic, with what it offers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SpotView {
    pub location: LocationView,
    pub actions: Vec<String>,
    pub late_night: Vec<String>,
    pub services: Vec<String>,
    pub here: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TownHoodView {
    pub id: u32,
    pub name: String,
    /// Neighbourhood kind (Fluent `hood-<kind>`).
    pub kind: String,
    pub is_home: bool,
    pub in_ward: bool,
    /// Home (in the home neighbourhood), the streets, then each place.
    pub spots: Vec<SpotView>,
}

/// The player's town as places, grouped by neighbourhood.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TownPlacesView {
    pub municipality: String,
    pub location: LocationDto,
    pub home_neighborhood: Option<u32>,
    pub neighborhoods: Vec<TownHoodView>,
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
    /// Fluent name key of the neighbourhood's strongest concern.
    pub top_concern_key: Option<String>,
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
    /// `mayor` | `ward`
    pub seat: String,
    pub ward_number: Option<u32>,
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
    /// Fluent keys, e.g. `overheard-jobs`.
    pub keys: Vec<String>,
}

/// A place panel: what can be done and learned there.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PlacePanelView {
    pub location: LocationView,
    pub here: bool,
    pub actions: Vec<OfferView>,
    pub services: Vec<String>,
    /// Service payloads: only filled while the player is here.
    pub town_data: Option<TownDataView>,
    pub budget: Option<BudgetPapersView>,
    pub internal_poll: Option<InternalPollView>,
    pub overheard: Option<OverheardView>,
}

pub fn location_view(st: &State, loc: Location) -> LocationView {
    let hood = places::hood_of(st, loc);
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
    if let Some(c) = &campaign
        && c.in_campaign
        && let Some(d) = c.days_to_election
    {
        alerts.push(alert("alert-campaign", &[("n", d.to_string())]));
    }
    let econ = st.economy.munis.get(m.index());
    let mut concerns = concerns_for(st, content, m, &hoods);
    concerns.truncate(2);
    let c5 = |v: f64| coarse(v, 0.05);
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
        headlines: HeadlinesView {
            unemployment: econ.map(|e| coarse(e.unemployment, 0.01)).unwrap_or(0.0),
            monthly_rent: econ.map(|e| coarse(e.housing.rent, 10.0)).unwrap_or(0.0),
            gov_approval: c5(weighted(st, &hoods, |c| c.gov_approval as f64)),
            approval_ward: c5(weighted(st, &ward_hoods, |c| c.player_approval as f64)),
            awareness_ward: c5(weighted(st, &ward_hoods, |c| c.player_awareness as f64)),
            approval_town: c5(weighted(st, &hoods, |c| c.player_approval as f64)),
            top_concern_keys: concerns.into_iter().map(|c| c.name_key).collect(),
        },
        location: location_view(st, pl.location),
        routes: routes(engine),
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
        here: st.player.as_ref().is_some_and(|p| p.location == loc),
    }
}

pub fn town_places_view(engine: &Engine) -> Option<TownPlacesView> {
    let st = engine.state();
    let pl = st.player.as_ref()?;
    let mu = st.world.municipalities.get(pl.municipality.index())?;
    let home = places::home_neighborhood(st);
    let ward: Vec<usize> = mandate_sim::policy::ward_hoods(&st.world, pl.ward);
    let neighborhoods = mu
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
                {
                    spots.push(spot(engine, Location::Place { place: *pid }));
                }
            }
            TownHoodView {
                id: n.id.0,
                name: n.name.clone(),
                kind: crate::office::snake(&n.kind),
                is_home: home == Some(n.id),
                in_ward: ward.contains(&n.id.index()),
                spots,
            }
        })
        .collect();
    Some(TownPlacesView {
        municipality: mu.name.clone(),
        location: LocationDto::of(pl.location),
        home_neighborhood: home.map(|h| h.0),
        neighborhoods,
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
        concerns: concerns_for(st, content, m, &hoods),
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
                    top_concern_key: concerns_for(st, content, m, &[h])
                        .first()
                        .map(|c| c.name_key.clone()),
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
                let t = engine.content().policies.get(&p.template)?;
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
            seat: if r.seat.is_executive() {
                "mayor"
            } else {
                "ward"
            }
            .into(),
            ward_number: match r.seat {
                mandate_sim::politics::elections::Seat::Council { ward, .. } => {
                    st.world.constituencies.get(ward.index()).map(|c| c.number)
                }
                _ => None,
            },
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
    let pl = st.player.as_ref()?;
    let h = places::hood_of(st, loc)?;
    let n = st.world.neighborhoods.get(h.index())?;
    let content = engine.content();
    let keys = concerns_for(st, content, pl.municipality, &[h.index()])
        .into_iter()
        .filter_map(|c| {
            content
                .concerns
                .iter()
                .find(|d| d.id == c.id)
                .and_then(|d| d.overheard_key.clone())
        })
        .take(2)
        .collect();
    Some(OverheardView {
        neighborhood: h.0,
        neighborhood_name: n.name.clone(),
        keys,
    })
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
    Some(PlacePanelView {
        location: location_view(st, loc),
        here,
        actions: places::actions_at(st, content, loc)
            .into_iter()
            .map(|(k, late)| offer_view(engine, loc, k, late))
            .collect(),
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
        services,
    })
}
