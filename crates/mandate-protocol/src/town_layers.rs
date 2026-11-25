//! The town map's layers (DEC-086, DEC-089's pattern; M6 note 11): what the
//! town map overview shows over the player's home town, one view per entry
//! of the registry (`mandate_content::town_map_layers`), all computed here.
//! The client paints what it is given and lists the same data as text
//! (DEC-051); it decides nothing.
//!
//! - **Boundaries** (source `districts`): the districts of the home
//!   jurisdiction's legislature (the constitution's body by procedure,
//!   DEC-098), from the neighbourhood-to-district mapping its elections use,
//!   each named as the "where you are" chip names it, with its seat's holder.
//! - **Markers** (source `people`): the town's people the player has at
//!   least heard of (DEC-091), at their workplace or where the player saw
//!   them, with the filters the registry lists.
//! - **Shaded** (source `worries`): each neighbourhood's class for the worry
//!   chosen, or its top worry, from `concerns::ranked` as felt, cut to what
//!   the player knows of that neighbourhood (`gate_concerns`). A
//!   neighbourhood the cut says nothing about has no class: it is drawn as
//!   unknown, never as low.
//!
//! Nothing here writes: a query never touches the knowledge ledger.
//!
//! A later shaded source (a bill's impact by neighbourhood, with the bill as
//! its parameter) adds a variant to `TownLayerSource` and one function
//! returning `TownShadeOptionView`s (one per bill); the registry's shape,
//! these views and the client stay as they are.

use crate::characters::{PersonRefView, person_ref_known};
use crate::knowledge::{Gate, KnowledgeView, SeenView, gate_concerns};
use crate::office::{ConcernView, concerns_for};
use crate::places::{LocationView, location_view};
use crate::post::{JurisdictionDto, PostDto, jurisdiction_dto, post_dto};
use crate::views::DateView;
use mandate_content::town_map_layers::{
    TownLayerFilter, TownLayerScale, TownLayerSource, TownMapLayerDef,
};
use mandate_content::{Constituency, Selection};
use mandate_sim::characters::{self, CharacterId, routines};
use mandate_sim::places::Location;
use mandate_sim::player::knowledge::{Level, Subject};
use mandate_sim::politics::{self, Post};
use mandate_sim::world::{Jurisdiction, MunicipalityId, NeighborhoodId};
use mandate_sim::{Engine, State};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

/// The parameter value of "each neighbourhood's top worry".
pub const TOP: &str = "top";

/// A neighbourhood of the town shown: its id is the layout's district id.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TownLayerHoodView {
    pub id: u32,
    pub name: String,
    /// For the centre (it has its town's name): the Fluent key that names
    /// it in place of `name`, with `name_args` (M6 note 19).
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_key: Option<String>,
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_args: Option<BTreeMap<String, String>>,
}

/// A party as the layers colour by it (the app's party colour slots).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TownLayerPartyView {
    pub id: u32,
    pub name: String,
    pub short: String,
    /// Stable categorical colour slot (0-based), as `PartyView.colour`.
    pub colour: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum TownLayerScaleView {
    /// The app's party colours.
    Party,
    /// One hue, light to dark, lowest class first.
    Sequential,
    /// A few hues for the named categories, then "Other".
    Categorical,
}

/// Someone on the map: who they are and where, as far as the player knows
/// (DEC-091). The name is empty below `heard`; role and seat are public.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TownMarkerView {
    pub person: PersonRefView,
    pub is_player: bool,
    pub party: Option<u32>,
    /// The seat they hold in the home legislature, if any.
    pub seat: Option<PostDto>,
    /// Where the marker goes: a place, a neighbourhood's streets or the
    /// player's home, in this town. `null`: listed, not placed.
    pub at: Option<LocationView>,
    /// Why there: `works` (their post's workplace or their own place, as
    /// their card gives it), `usually` or `last_seen` (the player's own
    /// sightings, `seen`), `self` (the player) or `unknown`.
    pub basis: String,
    /// The sighting behind `usually` or `last_seen`: its line and date.
    pub seen: Option<SeenView>,
    /// The ids of the layer's filters that keep this marker.
    pub filters: Vec<String>,
}

/// One entry of a legend. Named by `label_key`, or by its party.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TownLegendBinView {
    /// Fluent key of the entry's name; `null` where its party names it.
    pub label_key: Option<String>,
    /// Party scale: the party (`null`: none).
    pub party: Option<u32>,
    /// Areas, markers or neighbourhoods in it.
    pub count: u32,
}

/// An outlined area of a boundaries layer: a district.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TownAreaView {
    /// The district's number among its jurisdiction's.
    pub number: u32,
    /// Its name as the "where you are" chip gives it: a Fluent key with
    /// arguments (`ui-post-seat`: `n`, `tier`).
    pub label_key: String,
    pub label_args: BTreeMap<String, String>,
    /// Its neighbourhoods (ids).
    pub hoods: Vec<u32>,
    /// The player holds its seat.
    pub yours: bool,
    /// The player lives in it.
    pub home: bool,
    /// Its seat (DEC-098).
    pub seat: PostDto,
    /// Who holds the seat (`null`: vacant), and their class in the legend.
    pub holder: Option<TownMarkerView>,
}

/// A way to narrow a markers layer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TownLayerFilterView {
    /// `all`, `body`, `met` or `party:<id>`: what markers list in `filters`.
    pub id: String,
    /// Fluent key of its name (the body's is the constitution's); `null`
    /// for a party's filter, which its party names.
    pub label_key: Option<String>,
    pub party: Option<u32>,
    /// Markers it keeps.
    pub count: u32,
}

/// A neighbourhood under one choice of a shaded layer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TownShadeCellView {
    /// The index of its class in the choice's `bins`; `null`: not known.
    pub class: Option<u8>,
    /// What is written on it and in the list (a Fluent key); `null`: not
    /// known.
    pub label_key: Option<String>,
}

/// One choice of a shaded layer's parameter, with its legend and every
/// neighbourhood's class.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TownShadeOptionView {
    /// The parameter's value: `top`, or a concern's content id. Empty for a
    /// layer with no parameter.
    pub id: String,
    /// Fluent key of the choice's name.
    pub label_key: String,
    pub scale: TownLayerScaleView,
    /// The legend, lowest class first (categorical: the named categories in
    /// registry order, then Other).
    pub bins: Vec<TownLegendBinView>,
    /// Per neighbourhood, in `TownMapLayersView.hoods` order.
    pub cells: Vec<TownShadeCellView>,
    /// Neighbourhoods with no class (not known).
    pub unknown: u32,
}

/// What a layer shows, by its kind.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TownLayerDataView {
    Boundaries {
        /// The districts, in number order. Every neighbourhood is in one.
        areas: Vec<TownAreaView>,
        /// The holders' parties (then no party, then vacant seats).
        legend: Vec<TownLegendBinView>,
    },
    Markers {
        /// In id order.
        markers: Vec<TownMarkerView>,
        /// `all` first, then the registry's filters.
        filters: Vec<TownLayerFilterView>,
        legend: Vec<TownLegendBinView>,
    },
    Shaded {
        /// The parameter's name (`worry`), or `null` for a layer with one
        /// choice.
        param: Option<String>,
        /// Fluent key of the parameter's name (the picker's label).
        param_label_key: Option<String>,
        /// The choices, the default first.
        options: Vec<TownShadeOptionView>,
        /// Per neighbourhood, in `hoods` order: what the player knows of
        /// what the layer shows there, with the lead toward more.
        knowledge: Vec<KnowledgeView>,
        /// Per neighbourhood: Fluent keys of the detail to read out (the
        /// worries known there, strongest first; at most three).
        details: Vec<Vec<String>>,
    },
}

/// A town map layer from the registry, with what it shows.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TownMapLayerView {
    /// Content id, e.g. `town_map_layer.districts`.
    pub id: String,
    pub label_key: String,
    /// The legend's line: what the marks are.
    pub unit_key: String,
    pub note_key: Option<String>,
    /// Arguments of the three keys: `tier` (the home jurisdiction's).
    pub args: BTreeMap<String, String>,
    /// The registry's source name.
    pub source: String,
    pub scale: TownLayerScaleView,
    pub data: TownLayerDataView,
}

/// The town map's layers for the player's home town, in registry order.
/// Small; fetched when the map opens and after each command.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TownMapLayersView {
    /// The town shown: the player's home jurisdiction.
    pub at: JurisdictionDto,
    pub date: DateView,
    /// Its neighbourhoods, in the town's order.
    pub hoods: Vec<TownLayerHoodView>,
    /// The parties the layers colour by.
    pub parties: Vec<TownLayerPartyView>,
    pub layers: Vec<TownMapLayerView>,
}

/// What every layer reads.
struct Ctx<'a> {
    engine: &'a Engine,
    st: &'a State,
    gate: Gate<'a>,
    town: MunicipalityId,
    at: Jurisdiction,
    hoods: Vec<NeighborhoodId>,
    me: Option<CharacterId>,
    /// The home legislature's members, as characters.
    members: Vec<CharacterId>,
}

impl Ctx<'_> {
    /// The seat someone holds in the home legislature.
    fn seat_of(&self, id: CharacterId) -> Option<Post> {
        let pol = &self.st.politics;
        let p = pol.get(pol.characters.get(id)?.politician?)?;
        let post = p.role.post()?;
        let body = politics::legislature(crate::post::constitution(self.engine)?, self.at.tier())?;
        (post.at() == self.at && !post.is_office() && post.id() == body.id).then_some(post)
    }

    fn party_of(&self, id: CharacterId) -> Option<u32> {
        let pol = &self.st.politics;
        pol.get(pol.characters.get(id)?.politician?)?
            .party
            .map(|p| p.0)
    }

    /// Whether a location is one the map of this town can show.
    fn in_town(&self, loc: Location) -> bool {
        match loc {
            Location::Home => true,
            Location::Place { place } => {
                routines::town_of(self.st, loc) == Some(self.town)
                    && crate::knowledge::place_listed(&self.gate, self.st, place)
            }
            Location::Street { .. } => routines::town_of(self.st, loc) == Some(self.town),
        }
    }

    /// Where the player knows to find someone: their workplace once heard
    /// of (a post's is public; someone's own place is on their card from
    /// `heard`), else where the player usually or last saw them.
    fn known_place(
        &self,
        id: CharacterId,
    ) -> (Option<LocationView>, &'static str, Option<SeenView>) {
        if Some(id) == self.me {
            return (None, "self", None);
        }
        if self.gate.person(id) >= Level::Heard
            && let Some(loc) = routines::workplace(self.st, self.engine.content(), id)
            && self.in_town(loc)
        {
            return (Some(location_view(self.st, loc)), "works", None);
        }
        let ledger = self.st.player.as_ref().map(|p| &p.knowledge);
        if let Some(s) = ledger.and_then(|k| k.usually(id))
            && self.in_town(s.at)
            && let Some(seen) = crate::knowledge::usually_seen(self.engine, id)
        {
            return (Some(seen.at.clone()), "usually", Some(seen));
        }
        if let Some(s) = ledger.and_then(|k| k.last_seen(id))
            && self.in_town(s.at)
            && let Some(seen) = crate::knowledge::last_seen(self.engine, id)
        {
            return (Some(seen.at.clone()), "last_seen", Some(seen));
        }
        (None, "unknown", None)
    }

    fn marker(&self, id: CharacterId) -> Option<TownMarkerView> {
        let person = person_ref_known(self.engine, &self.gate, id)?;
        let (at, basis, seen) = self.known_place(id);
        Some(TownMarkerView {
            person,
            is_player: Some(id) == self.me,
            party: self.party_of(id),
            seat: self.seat_of(id).map(|p| post_dto(self.engine, p)),
            at,
            basis: basis.into(),
            seen,
            filters: Vec::new(),
        })
    }
}

/// A party-scale legend: each party with someone in it (party order), then
/// those with no party under `none_key`.
fn party_legend(
    st: &State,
    parties: impl Iterator<Item = Option<u32>>,
    none_key: &str,
) -> Vec<TownLegendBinView> {
    let mut by: BTreeMap<Option<u32>, u32> = BTreeMap::new();
    for p in parties {
        *by.entry(p).or_default() += 1;
    }
    let mut out: Vec<TownLegendBinView> = st
        .politics
        .parties
        .iter()
        .filter_map(|p| {
            by.get(&Some(p.id.0)).map(|n| TownLegendBinView {
                label_key: None,
                party: Some(p.id.0),
                count: *n,
            })
        })
        .collect();
    if let Some(n) = by.get(&None) {
        out.push(TownLegendBinView {
            label_key: Some(none_key.into()),
            party: None,
            count: *n,
        });
    }
    out
}

/// The districts of the home legislature and each seat's holder.
fn districts(cx: &Ctx<'_>) -> TownLayerDataView {
    let st = cx.st;
    let tier = jurisdiction_dto(&st.world, cx.at).tier;
    let body = crate::post::constitution(cx.engine)
        .and_then(|c| politics::legislature(c, cx.at.tier()).map(|b| (c, b)))
        .filter(|(_, b)| {
            matches!(
                b.selection,
                Selection::Election {
                    constituency: Constituency::Ward,
                    ..
                }
            )
        });
    let Some((constitution, _)) = body else {
        // An at-large body has no districts to draw.
        return TownLayerDataView::Boundaries {
            areas: Vec::new(),
            legend: Vec::new(),
        };
    };
    let mine = mandate_sim::player::posts(st);
    let home = mandate_sim::places::home_neighborhood(st);
    let mut areas: Vec<TownAreaView> = cx
        .at
        .constituencies(&st.world)
        .filter_map(|d| {
            let post = politics::legislature_seat(constitution, cx.at, d.id)?;
            let holder = politics::holder_of_post(&st.politics, &st.world, post)
                .and_then(|p| st.politics.get(p))
                .and_then(|p| cx.marker(p.character));
            Some(TownAreaView {
                number: d.number,
                label_key: "ui-post-seat".into(),
                label_args: BTreeMap::from([
                    ("n".to_string(), d.number.to_string()),
                    ("tier".to_string(), tier.clone()),
                ]),
                hoods: d
                    .neighborhoods
                    .iter()
                    .filter(|h| cx.hoods.contains(h))
                    .map(|h| h.0)
                    .collect(),
                yours: mine.contains(&post),
                home: home.is_some_and(|h| d.neighborhoods.contains(&h)),
                seat: post_dto(cx.engine, post),
                holder,
            })
        })
        .collect();
    areas.sort_by_key(|a| a.number);
    let mut legend = party_legend(
        st,
        areas
            .iter()
            .filter_map(|a| a.holder.as_ref().map(|h| h.party)),
        "ui-independent",
    );
    let vacant = areas.iter().filter(|a| a.holder.is_none()).count() as u32;
    if vacant > 0 {
        legend.push(TownLegendBinView {
            label_key: Some("town-map-legend-vacant".into()),
            party: None,
            count: vacant,
        });
    }
    TownLayerDataView::Boundaries { areas, legend }
}

/// The town's people the player has at least heard of, as markers.
fn people(cx: &Ctx<'_>, def: &TownMapLayerDef) -> TownLayerDataView {
    let st = cx.st;
    let mut markers: Vec<TownMarkerView> = routines::townsfolk(st, cx.town)
        .into_iter()
        .filter(|id| cx.gate.person(*id) >= Level::Heard)
        .filter_map(|id| cx.marker(id))
        .collect();
    let body_key = crate::post::constitution(cx.engine)
        .and_then(|c| politics::legislature(c, cx.at.tier()))
        .map(|b| b.name_key.clone());
    let mut filters = vec![TownLayerFilterView {
        id: "all".into(),
        label_key: Some("town-map-filter-all".into()),
        party: None,
        count: markers.len() as u32,
    }];
    for m in &mut markers {
        m.filters.push("all".into());
    }
    for f in &def.filters {
        match f {
            TownLayerFilter::Body => {
                let Some(key) = &body_key else { continue };
                let mut n = 0;
                for m in &mut markers {
                    if m.person
                        .id
                        .is_some_and(|id| cx.members.contains(&CharacterId(id)))
                    {
                        m.filters.push("body".into());
                        n += 1;
                    }
                }
                filters.push(TownLayerFilterView {
                    id: "body".into(),
                    label_key: Some(key.clone()),
                    party: None,
                    count: n,
                });
            }
            TownLayerFilter::Met => {
                let mut n = 0;
                for m in &mut markers {
                    if m.person.level >= crate::knowledge::KnowledgeLevelDto::Known {
                        m.filters.push("met".into());
                        n += 1;
                    }
                }
                filters.push(TownLayerFilterView {
                    id: "met".into(),
                    label_key: Some("town-map-filter-met".into()),
                    party: None,
                    count: n,
                });
            }
            TownLayerFilter::Party => {
                for p in &st.politics.parties {
                    let id = format!("party:{}", p.id.0);
                    let mut n = 0;
                    for m in &mut markers {
                        if m.party == Some(p.id.0) {
                            m.filters.push(id.clone());
                            n += 1;
                        }
                    }
                    if n > 0 {
                        filters.push(TownLayerFilterView {
                            id,
                            label_key: None,
                            party: Some(p.id.0),
                            count: n,
                        });
                    }
                }
            }
        }
    }
    let legend = party_legend(
        st,
        markers.iter().map(|m| m.party),
        "town-map-legend-no-party",
    );
    TownLayerDataView::Markers {
        markers,
        filters,
        legend,
    }
}

/// The classes of a chosen worry, lowest first: how high it stands among a
/// neighbourhood's worries, as far as the player's cut of the ranking says.
const RANK_KEYS: [&str; 3] = [
    "town-map-worry-rank-low",
    "town-map-worry-rank-high",
    "town-map-worry-rank-top",
];

/// A chosen worry's class in a neighbourhood from the player's cut of its
/// ranking (`gate_concerns`: the first at `heard`, three at `known`, all in
/// detail). `None` where the cut cannot say: nothing known, or only the top
/// worry known and this is not it.
fn rank_class(level: Level, cut: &[ConcernView], concern: &str) -> Option<u8> {
    match (level, cut.iter().position(|c| c.id == concern)) {
        (Level::Unknown, _) => None,
        (_, Some(0)) => Some(2),
        (Level::Heard, _) => None,
        (_, Some(1 | 2)) => Some(1),
        // Known not to be among the top three (or, in detail, lower still).
        _ => Some(0),
    }
}

fn count_bins(bins: &mut [TownLegendBinView], cells: &[TownShadeCellView]) -> u32 {
    for (i, b) in bins.iter_mut().enumerate() {
        b.count = cells.iter().filter(|c| c.class == Some(i as u8)).count() as u32;
    }
    cells.iter().filter(|c| c.class.is_none()).count() as u32
}

/// What each neighbourhood worries about, as far as the player knows it.
fn worries(cx: &Ctx<'_>, def: &TownMapLayerDef, top: bool) -> TownLayerDataView {
    let st = cx.st;
    let content = cx.engine.content();
    // The player's cut of each neighbourhood's one ranking (felt).
    let cuts: Vec<(Level, Vec<ConcernView>)> = cx
        .hoods
        .iter()
        .map(|h| {
            let level = cx.gate.hood_concerns(*h);
            let all = concerns_for(st, content, mandate_sim::concerns::Scope::Neighbourhood(*h));
            (level, gate_concerns(level, all, false, usize::MAX))
        })
        .collect();
    let bin = |key: &str| TownLegendBinView {
        label_key: Some(key.into()),
        party: None,
        count: 0,
    };
    let mut options = Vec::new();
    if top {
        // Each one's top worry: the most common get a class of their own
        // (ties: registry order), shown in registry order; the rest are Other.
        let index = |c: &ConcernView| content.concerns.iter().position(|d| d.id == c.id);
        let tops: Vec<Option<usize>> = cuts
            .iter()
            .map(|(_, cut)| cut.first().and_then(index))
            .collect();
        let mut counts = vec![0u32; content.concerns.len()];
        for i in tops.iter().flatten() {
            counts[*i] += 1;
        }
        let mut by_count: Vec<usize> = (0..counts.len()).filter(|i| counts[*i] > 0).collect();
        by_count.sort_by(|a, b| counts[*b].cmp(&counts[*a]).then(a.cmp(b)));
        let mut named: Vec<usize> = by_count.iter().take(def.categories()).copied().collect();
        named.sort_unstable();
        let other = named.len() as u8;
        let cells: Vec<TownShadeCellView> = tops
            .iter()
            .map(|t| TownShadeCellView {
                class: t.map(|i| {
                    named
                        .iter()
                        .position(|n| *n == i)
                        .map_or(other, |p| p as u8)
                }),
                label_key: t.map(|i| content.concerns[i].name_key.clone()),
            })
            .collect();
        let mut bins: Vec<TownLegendBinView> = named
            .iter()
            .map(|i| bin(&content.concerns[*i].name_key))
            .collect();
        if cells.iter().any(|c| c.class == Some(other)) {
            bins.push(bin("ui-map-legend-other"));
        }
        let unknown = count_bins(&mut bins, &cells);
        options.push(TownShadeOptionView {
            id: TOP.into(),
            label_key: "town-map-worry-top".into(),
            scale: TownLayerScaleView::Categorical,
            bins,
            cells,
            unknown,
        });
    }
    for c in &content.concerns {
        let cells: Vec<TownShadeCellView> = cuts
            .iter()
            .map(|(level, cut)| {
                let class = rank_class(*level, cut, &c.id);
                TownShadeCellView {
                    class,
                    label_key: class.map(|k| RANK_KEYS[k as usize].to_string()),
                }
            })
            .collect();
        let mut bins: Vec<TownLegendBinView> = RANK_KEYS.iter().map(|k| bin(k)).collect();
        let unknown = count_bins(&mut bins, &cells);
        options.push(TownShadeOptionView {
            id: c.id.clone(),
            label_key: c.name_key.clone(),
            scale: TownLayerScaleView::Sequential,
            bins,
            cells,
            unknown,
        });
    }
    TownLayerDataView::Shaded {
        param: def.source.param().map(|p| p.key().to_string()),
        param_label_key: def
            .source
            .param()
            .map(|p| format!("town-map-param-{}", p.key())),
        options,
        knowledge: cx
            .hoods
            .iter()
            .map(|h| cx.gate.view(&Subject::HoodConcerns { hood: *h }))
            .collect(),
        details: cuts
            .into_iter()
            .map(|(_, cut)| cut.into_iter().take(3).map(|c| c.name_key).collect())
            .collect(),
    }
}

fn layer_view(cx: &Ctx<'_>, def: &TownMapLayerDef) -> TownMapLayerView {
    let data = match &def.source {
        TownLayerSource::Districts {} => districts(cx),
        TownLayerSource::People {} => people(cx, def),
        TownLayerSource::Worries { top } => worries(cx, def, *top),
    };
    TownMapLayerView {
        id: def.id.clone(),
        label_key: def.label_key.clone(),
        unit_key: def.unit_key.clone(),
        note_key: def.note_key.clone(),
        args: BTreeMap::from([(
            "tier".to_string(),
            jurisdiction_dto(&cx.st.world, cx.at).tier,
        )]),
        source: def.source.key().into(),
        scale: match def.scale {
            TownLayerScale::Party => TownLayerScaleView::Party,
            TownLayerScale::Sequential => TownLayerScaleView::Sequential,
        },
        data,
    }
}

/// The town map's layers for the player's home town (DEC-098: their home,
/// never a field of theirs), every registry layer computed here. `None`
/// without a player.
pub fn town_map_layers_view(engine: &Engine) -> Option<TownMapLayersView> {
    let st = engine.state();
    let town = mandate_sim::player::home(st)?;
    let mu = st.world.municipalities.get(town.index())?;
    let at = Jurisdiction::town(town);
    let members = crate::post::constitution(engine)
        .map(|c| politics::legislature_of(&st.politics, c, at))
        .unwrap_or(&[])
        .iter()
        .filter_map(|p| st.politics.get(*p))
        .map(|p| p.character)
        .collect();
    let cx = Ctx {
        engine,
        st,
        gate: Gate::of(engine),
        town,
        at,
        hoods: mu.neighborhoods.clone(),
        me: characters::player_character(st),
        members,
    };
    let hoods = cx
        .hoods
        .iter()
        .filter_map(|h| st.world.neighborhoods.get(h.index()))
        .map(|n| {
            let centre = crate::talk::hood_centre_label(st, n.id);
            TownLayerHoodView {
                id: n.id.0,
                name: n.name.clone(),
                name_key: centre.as_ref().map(|(k, _)| k.clone()),
                name_args: centre.map(|(_, a)| a),
            }
        })
        .collect();
    Some(TownMapLayersView {
        at: jurisdiction_dto(&st.world, at),
        date: DateView::of(st.calendar.date),
        hoods,
        parties: st
            .politics
            .parties
            .iter()
            .map(|p| TownLayerPartyView {
                id: p.id.0,
                name: p.name.clone(),
                short: p.short.clone(),
                colour: p.colour as u32,
            })
            .collect(),
        layers: engine
            .content()
            .town_map_layers
            .iter()
            .map(|d| layer_view(&cx, d))
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cut(ids: &[&str]) -> Vec<ConcernView> {
        ids.iter()
            .map(|id| ConcernView {
                id: (*id).into(),
                name_key: String::new(),
                weight: 0.0,
            })
            .collect()
    }

    #[test]
    fn a_worry_the_cut_cannot_place_has_no_class() {
        let three = cut(&["a", "b", "c"]);
        // Nothing known: never a class, whatever the list holds.
        assert_eq!(rank_class(Level::Unknown, &three, "a"), None);
        // Heard: only the top worry is known.
        assert_eq!(rank_class(Level::Heard, &cut(&["a"]), "a"), Some(2));
        assert_eq!(rank_class(Level::Heard, &cut(&["a"]), "b"), None);
        // Known: the top three are placed; the rest are known to be lesser.
        assert_eq!(rank_class(Level::Known, &three, "a"), Some(2));
        assert_eq!(rank_class(Level::Known, &three, "c"), Some(1));
        assert_eq!(rank_class(Level::Known, &three, "d"), Some(0));
        assert_eq!(
            rank_class(Level::Detailed, &cut(&["a", "b", "c", "d"]), "d"),
            Some(0)
        );
    }
}
