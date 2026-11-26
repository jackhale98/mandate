//! Doors for the client (DEC-114 §2, `BEING_THERE.md` §3): a street's door
//! slots as rows (quick mode) and as hotspots on its houses (the street maps
//! slot *k* to the *k*-th house by the plan's fill rank), and a doorstep:
//! who answers, what they say first, and the three things the player can
//! say. Facts and Fluent keys only.
//!
//! What is behind a door is generated from live state when asked
//! (`characters::households`), never stored; the name on the door is for
//! anyone to read. What the household remembers of the player is their own
//! record (`DoorstepRecord`). Nothing here counts doors (DEC-113).

use crate::pocket::TextView;
use crate::talk::CanvassWorryView;
use crate::views::DateView;
use mandate_sim::Engine;
use mandate_sim::characters::households::{self, Household, Work};
use mandate_sim::places::Location;
use mandate_sim::world::NeighborhoodId;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// A household's last call, as they remember it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DoorCallView {
    pub on: DateView,
    /// `warm`, `cool` or `not_in`.
    pub went: String,
    /// `listened`, `made_the_case`, `asked` or `knocked`.
    pub said: String,
    /// The worry heard or argued then (its talk key).
    pub worry_key: Option<String>,
    /// A promise they made: `vote` or `poster`.
    pub promise: Option<String>,
    /// The member the player has met (a stored person), if one.
    pub person: Option<u32>,
}

/// A door of a street.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DoorRowView {
    /// The slot (`ActionDto::Doorstep { door }`).
    pub door: u32,
    /// The house number.
    pub number: u32,
    /// The name on the door.
    pub family: String,
    /// Nobody lives there now.
    pub vacant: bool,
    /// Knocked on in the canvass under way.
    pub knocked: bool,
    /// A poster in the window (a promise kept on their record).
    pub poster: bool,
    /// Their last call, while they remember it.
    pub last: Option<DoorCallView>,
}

/// The canvass under way on this street.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct StreetCanvassView {
    /// Doors knocked so far, and how many more the street takes.
    pub knocked: u32,
    pub left: u32,
}

/// A neighbourhood's doors (`PlacePanelView.doors` on its streets).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct StreetDoorsView {
    pub neighborhood: u32,
    /// Every door slot, by slot.
    pub doors: Vec<DoorRowView>,
    /// The doors the engine would knock for "Canvass the street", which
    /// quick mode lists first: households not called on lately first.
    pub suggested: Vec<u32>,
    /// The canvass under way here, if one.
    pub canvass: Option<StreetCanvassView>,
    /// Doors a canvass knocks at most, and the slots its first knock spends.
    pub street_doors: u32,
    pub slots: u32,
    /// Whether a door can be knocked now (here, with the time, or a canvass
    /// under way), and why not (`ui-doors-blocked-…`).
    pub can_knock: bool,
    pub blocked_key: Option<String>,
    /// What "Ask" asks for: `vote` in a campaign, else `poster`.
    pub ask: String,
    /// The worries the player can make the case on (the canvass preview's).
    pub worries: Vec<CanvassWorryView>,
}

/// One of the three things to say at a door.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DoorChoiceView {
    /// `listen`, `case` or `ask` (`DoorChoiceDto`).
    pub kind: String,
    pub label_key: String,
}

/// A doorstep (`QueryDto::Door`): who answers and what they say first, as
/// lines in order, and the choices.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DoorView {
    pub neighborhood: u32,
    pub door: u32,
    pub number: u32,
    pub family: String,
    /// Someone came to the door.
    pub answers: bool,
    /// Who answered, by name (empty when nobody did).
    pub given: String,
    pub opener: Vec<TextView>,
    /// The three choices (one, "Move on", when nobody answers).
    pub choices: Vec<DoorChoiceView>,
    /// Knocked on already in the canvass under way.
    pub knocked: bool,
    /// The member the player has met (a stored person), if one.
    pub person: Option<u32>,
    /// The street around it.
    pub street: StreetDoorsView,
}

/// Wire form of `actions::DoorChoice`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DoorChoiceDto {
    Listen,
    Case { issue: String },
    Ask,
}

impl DoorChoiceDto {
    pub fn to_sim(&self) -> mandate_sim::actions::DoorChoice {
        use mandate_sim::actions::DoorChoice as C;
        match self {
            DoorChoiceDto::Listen => C::Listen,
            DoorChoiceDto::Case { issue } => C::Case {
                issue: issue.clone(),
            },
            DoorChoiceDto::Ask => C::Ask,
        }
    }
}

fn in_focus(engine: &Engine, hood: NeighborhoodId) -> bool {
    let st = engine.state();
    st.world
        .neighborhoods
        .get(hood.index())
        .is_some_and(|n| Some(n.municipality) == mandate_sim::player::home(st))
}

fn said_key(s: households::Said) -> &'static str {
    match s {
        households::Said::Listened => "listened",
        households::Said::MadeTheCase => "made_the_case",
        households::Said::Asked => "asked",
        households::Said::Knocked => "knocked",
    }
}

/// A worry's words in talk ("the rent"), by concern id.
fn worry_key(engine: &Engine, id: &str) -> Option<String> {
    engine
        .content()
        .concerns
        .iter()
        .find(|c| c.id == id)
        .map(|c| c.talk_key.clone().unwrap_or_else(|| c.name_key.clone()))
}

fn last_call(engine: &Engine, h: &Household) -> Option<DoorCallView> {
    let st = engine.state();
    let me = mandate_sim::characters::player_character(st)?;
    let p = &engine.content().characters.households;
    let r = st
        .politics
        .characters
        .households
        .record(me, h.hood, h.door, h.generation)?;
    if r.on.days_until(st.calendar.date) > p.forget_days {
        return None;
    }
    Some(DoorCallView {
        on: DateView::of(r.on),
        went: r.went.key().into(),
        said: said_key(r.said).into(),
        worry_key: r.worry.as_deref().and_then(|w| worry_key(engine, w)),
        promise: r.promise.map(|x| x.key().into()),
        person: r.speaker.map(|x| x.0),
    })
}

/// The doors of a neighbourhood's streets (in the player's home town).
pub fn street_doors(engine: &Engine, hood: NeighborhoodId) -> Option<StreetDoorsView> {
    let st = engine.state();
    let content = engine.content();
    let pl = st.player.as_ref()?;
    if !in_focus(engine, hood) {
        return None;
    }
    let p = &content.characters.households;
    let n = households::doors(st, content, hood);
    let street = st
        .politics
        .characters
        .households
        .street
        .as_ref()
        .filter(|s| s.hood == hood);
    let mut doors = Vec::new();
    for d in 0..n {
        let Some(h) = households::household(st, content, hood, d as u16) else {
            continue;
        };
        let last = last_call(engine, &h);
        doors.push(DoorRowView {
            door: d,
            number: d + 1,
            family: h.family.clone(),
            vacant: h.vacant,
            knocked: street.is_some_and(|s| s.doors.contains(&(d as u16))),
            poster: last.as_ref().is_some_and(|l| l.promise.as_deref() == Some("poster")),
            last,
        });
    }
    let here = pl.location
        == Location::Street {
            neighborhood: hood,
        };
    let slots = content.tuning.action_cost("canvass").slots;
    let (can_knock, blocked_key) = if street.is_some() {
        (true, None)
    } else if !here {
        (false, Some("ui-doors-blocked-away".to_string()))
    } else if pl.slots_left() < slots {
        (false, Some("ui-doors-blocked-time".to_string()))
    } else {
        (true, None)
    };
    let campaign = mandate_sim::characters::is_candidate(&st.politics, pl.politician);
    Some(StreetDoorsView {
        neighborhood: hood.0,
        doors,
        suggested: households::engine_doors(st, content, hood)
            .into_iter()
            .map(u32::from)
            .collect(),
        canvass: street.map(|s| StreetCanvassView {
            knocked: s.doors.len() as u32,
            left: (p.street_doors as u32).saturating_sub(s.doors.len() as u32),
        }),
        street_doors: p.street_doors as u32,
        slots: slots as u32,
        can_knock,
        blocked_key,
        ask: if campaign { "vote" } else { "poster" }.into(),
        worries: crate::talk::canvass_view(engine, hood.0)
            .map(|v| v.worries)
            .unwrap_or_default(),
    })
}

/// What the earner's work is, as their opener names it.
fn work_line(engine: &Engine, h: &Household) -> TextView {
    let st = engine.state();
    let content = engine.content();
    match &h.work {
        Work::Retired => TextView::new("door-work-retired"),
        Work::Looking => TextView::new("door-work-looking"),
        Work::Sector { sector } => {
            let key = content
                .sector(sector)
                .map(|d| d.talk_key.clone().unwrap_or_else(|| d.name_key.clone()))
                .unwrap_or_default();
            TextView::new("door-work-sector").with("sector_key", key)
        }
        Work::Plant { plant } => {
            use mandate_sim::economy::plants::PlantStatus;
            let pl = mandate_sim::economy::plants::PlantId(*plant);
            match crate::places::plant_view(engine, pl) {
                Some(v) => {
                    let status = st
                        .economy
                        .plants
                        .iter()
                        .find(|x| x.id == pl)
                        .map(|x| match x.status {
                            PlantStatus::Running => "running",
                            PlantStatus::ShortTime { .. } => "short",
                            PlantStatus::Threatened { .. } => "notice",
                            PlantStatus::Closed { .. } => "closed",
                        })
                        .unwrap_or("running");
                    TextView::new(format!("door-work-plant-{status}"))
                        .with("plant_key", v.name_key)
                        .with("stem", v.stem)
                        .with("works_key", v.works_key)
                }
                None => TextView::new("door-work-looking"),
            }
        }
    }
}

/// A doorstep: who answers and what they say first, as the knock finds it.
pub fn door_view(engine: &Engine, hood: NeighborhoodId, door: u16) -> Option<DoorView> {
    let st = engine.state();
    let content = engine.content();
    let street = street_doors(engine, hood)?;
    let h = households::household(st, content, hood, door)?;
    let answers = households::answers(st, content, &h);
    let last = last_call(engine, &h);
    let number = door as u32 + 1;
    let mut opener = Vec::new();
    let mut choices = Vec::new();
    if !answers {
        opener.push(
            TextView::new(if h.vacant {
                "door-open-vacant"
            } else {
                "door-open-not-in"
            })
            .with("family", h.family.clone())
            .with("number", number.to_string()),
        );
        choices.push(DoorChoiceView {
            kind: "listen".into(),
            label_key: "ui-door-move-on".into(),
        });
    } else {
        // Who answers: a stranger, or someone who remembers the last call.
        let who = TextView::new("door-open-answers")
            .with("given", h.given.clone())
            .with("family", h.family.clone())
            .with("age", h.age.to_string());
        opener.push(who);
        match last.as_ref().filter(|l| l.went != "not_in") {
            Some(l) => {
                let mut t = TextView::new(match (&l.promise, &l.worry_key) {
                    (Some(pr), _) if pr == "poster" => "door-again-poster",
                    (Some(_), _) => "door-again-vote",
                    (None, Some(_)) => "door-again-worry",
                    (None, None) => "door-again",
                })
                .with("month_key", format!("month-{}", l.on.month));
                if let Some(w) = &l.worry_key {
                    t = t.with("worry_key", w.clone());
                }
                opener.push(t);
            }
            None => opener.push(work_line(engine, &h)),
        }
        opener.push(
            TextView::new(format!(
                "door-home-{}",
                if h.owner { "owner" } else { "tenant" }
            ))
            .with("n", h.children.to_string()),
        );
        opener.push(TextView::new(format!("door-mood-{}", h.mood(content))));
        choices.push(DoorChoiceView {
            kind: "listen".into(),
            label_key: "ui-door-listen".into(),
        });
        choices.push(DoorChoiceView {
            kind: "case".into(),
            label_key: "ui-door-case".into(),
        });
        choices.push(DoorChoiceView {
            kind: "ask".into(),
            label_key: format!("ui-door-ask-{}", street.ask),
        });
    }
    Some(DoorView {
        neighborhood: hood.0,
        door: door as u32,
        number,
        family: h.family.clone(),
        answers,
        given: if answers { h.given.clone() } else { String::new() },
        opener,
        choices,
        knocked: street.doors.iter().any(|d| d.door == door as u32 && d.knocked),
        person: h.speaker.map(|x| x.0),
        street,
    })
}
