//! Media and information (DEC-034): outlets, knowledge, coverage, and
//! perception; scandals that grow from hidden facts; rumours.
//!
//! **Four layers.** *Facts* are the narrative's facts log and its hidden
//! facts (`NarrativeState::hidden`). *Knowledge* is who uncovered which
//! hidden fact, with what evidence (`MediaState::knowledge`). *Coverage* is
//! the stories outlets run (`MediaState::stories`): chosen by newsworthiness
//! × editorial interest × access, framed by each outlet's lean. *Perception*
//! is what voters and people believe: it changes only through coverage (and
//! the player's own communication), never from the truth directly. A story
//! about the player moves cohort approval and awareness (population applies
//! `Effect::Perceive`); a story about anyone moves their scandal valence in
//! elections (`Effect::Valence`, the same for NPCs and the player, DEC-030);
//! colleagues remember (`Effect::Remember`).
//!
//! **Whether and when (DEC-033 B).** Whether a hidden fact will come out,
//! and to whom, is settled when it happens (`hide`: evidence × interest ×
//! access, a keyed draw). The director chooses when within its evidence
//! window (`narrative::director::discovery`). Discovery here only reads both.
//!
//! **Determinism.** Every draw is keyed (seed, `Stream::Media`, a tag, ids),
//! so nothing here shifts a sequential stream. The media systems run only
//! while play is on (outlets exist), never in the history warm-up.

use crate::characters::{self, CharacterId, MemoryKind};
use crate::events::{EventId, EventKind};
use crate::narrative::{Actor, Fate, HiddenFact};
use crate::politics::PoliticianId;
use crate::scheduler::{Effect, Outs, Reads};
use crate::spine::{Cause, Causes, Link, PlayerAct, Process, Thread};
use crate::state::State;
use crate::world::{MunicipalityId, PlaceId};
use mandate_content::{
    ContentBundle, ContentKey, Discoverer, Gravity, HiddenSpec, OutletScope, ScandalResponseKind,
};
use mandate_core::math::clamp;
use mandate_core::{SimDate, Stream};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Keyed-RNG tags (DEC-041).
const KEY_FATE: u64 = 0x3ED1;
const KEY_WHO: u64 = 0x3ED2;
const KEY_OUTLET: u64 = 0x3ED3;
const KEY_CORRECT: u64 = 0x3ED4;

/// Where an outlet publishes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Scope {
    Town { municipality: u32 },
    Region { region: u32 },
    Nation,
}

impl Scope {
    /// The jurisdiction the outlet covers (DEC-098; the saved shape stays).
    pub fn at(self) -> crate::world::Jurisdiction {
        use crate::world::{Jurisdiction, RegionId};
        match self {
            Scope::Town { municipality } => Jurisdiction::town(MunicipalityId(municipality)),
            Scope::Region { region } => Jurisdiction::Region {
                id: RegionId(region),
            },
            Scope::Nation => Jurisdiction::Nation,
        }
    }
}

/// An outlet (DEC-034), generated from an `outlets` template.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Outlet {
    pub id: u32,
    pub template: ContentKey,
    pub scope: Scope,
    /// Its home place (a town outlet's newspaper office, DEC-079).
    pub place: Option<PlaceId>,
    /// A proper name (a town paper: its office's name), else empty.
    pub name: String,
    /// The `$place` argument of its name key (its region), else empty.
    pub name_arg: String,
    pub social: bool,
    pub reach: f32,
    /// Audience weights: [low, middle, high] and four age bands.
    pub class_weight: [f32; 3],
    pub age_weight: [f32; 4],
    /// Editorial lean, one value per attitude (registry order).
    pub lean: Vec<f32>,
    pub credibility: f32,
    pub interests: Vec<ContentKey>,
    /// Yearly budget (currency units).
    pub budget: f64,
    pub stories_per_week: u8,
    pub journalists: Vec<CharacterId>,
    /// Extra hostility toward a politician (after a counter-attack).
    pub grudges: Vec<(PoliticianId, f32)>,
}

impl Outlet {
    /// Covers this town.
    pub fn covers(&self, world: &crate::world::World, m: u32) -> bool {
        match self.scope {
            Scope::Town { municipality } => municipality == m,
            Scope::Region { region } => world
                .municipalities
                .get(m as usize)
                .is_some_and(|x| x.region.0 == region),
            Scope::Nation => true,
        }
    }

    /// Relative audience weight of a cohort (age band, class).
    pub fn weight(&self, age: usize, class: usize) -> f64 {
        self.class_weight.get(class).copied().unwrap_or(1.0) as f64
            * self.age_weight.get(age).copied().unwrap_or(1.0) as f64
    }
}

/// Who knows a hidden fact.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Knower {
    Person {
        id: CharacterId,
    },
    Outlet {
        id: u32,
    },
    /// Auditors or investigators (an office, not a person).
    Office {
        by: Discoverer,
    },
}

/// Knowledge (DEC-034): a knower, a hidden fact, the evidence they hold.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Knowledge {
    pub hidden: u32,
    pub knower: Knower,
    pub evidence: f32,
    pub since: SimDate,
}

/// A hidden fact that came out: a public affair with its heat.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Scandal {
    pub id: u32,
    pub hidden: u32,
    pub subject: PoliticianId,
    /// The fact kind (Fluent `fact-<kind>`).
    pub kind: String,
    pub severity: Gravity,
    /// Evidence in the open (an admission adds to it).
    pub evidence: f32,
    pub town: u32,
    pub broke: SimDate,
    pub by: Discoverer,
    pub who: Option<CharacterId>,
    /// The outlet that broke it.
    pub outlet: Option<u32>,
    /// 1 when it breaks; fades weekly.
    pub heat: f32,
    pub stories: u32,
    pub response: Option<ScandalResponseKind>,
    pub responded: Option<SimDate>,
    /// A denial against strong evidence unravels on this date.
    pub backfire_on: Option<SimDate>,
    pub ended: Option<SimDate>,
    /// A leak brought it out (the player's or an NPC's act).
    #[serde(default)]
    pub leaked: bool,
}

impl Scandal {
    pub fn running(&self) -> bool {
        self.ended.is_none()
    }
}

/// An unverified claim (DEC-034 rumours, kept small): it spreads on social
/// platforms and a credible outlet may correct it. It traces to the leak
/// that started it (a hidden fact of the leaker).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rumour {
    pub id: u32,
    pub about: PoliticianId,
    /// The leak (a hidden fact of whoever started it).
    pub leak: u32,
    pub town: u32,
    pub started: SimDate,
    pub weeks: u8,
    /// Scandal valence it has cost so far (a correction gives part back).
    pub harm: f32,
    pub corrected: Option<SimDate>,
}

/// A statement to the press (the player's press action, DEC-034 player
/// communication): outlets decide whether and how to cover it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Statement {
    pub politician: PoliticianId,
    pub date: SimDate,
    pub policy: Option<u32>,
}

/// What a story reports: every story traces to a fact (REQ-003).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StorySource {
    Scandal {
        id: u32,
    },
    /// The subject's answer to a scandal.
    Response {
        scandal: u32,
        response: ScandalResponseKind,
    },
    /// A denial that unravelled.
    Backfire {
        scandal: u32,
    },
    /// A legal development: `investigation`, `charged`, `dropped`,
    /// `acquitted`, `convicted`.
    Case {
        id: u32,
        stage: String,
    },
    Rumour {
        id: u32,
    },
    Correction {
        rumour: u32,
    },
    Statement {
        politician: PoliticianId,
        policy: Option<u32>,
    },
    /// Town news from the event log (no perception effect).
    Town {
        event: EventId,
    },
    /// An exchange scene's closing event (DEC-112): the paper's account of
    /// a debate, a hearing, a meeting. About the player; those who were
    /// not in the room hear of it through the media's own perception rule.
    Exchange {
        event: EventId,
    },
}

/// A story an outlet ran (coverage).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Story {
    pub id: u32,
    pub outlet: u32,
    pub date: SimDate,
    pub source: StorySource,
    pub subject: Option<PoliticianId>,
    pub town: u32,
    /// −1 hostile … 0 straight … 1 sympathetic.
    pub framing: f32,
    /// Approval of the subject per unit of exposure (signed), before room.
    pub impact: f32,
    /// Name recognition per unit of exposure.
    pub aware: f32,
    pub score: f32,
}

/// The media domain (DEC-034).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MediaState {
    /// Outlets exist (play has begun).
    pub active: bool,
    pub outlets: Vec<Outlet>,
    pub knowledge: Vec<Knowledge>,
    pub scandals: Vec<Scandal>,
    pub next_scandal: u32,
    pub stories: Vec<Story>,
    pub next_story: u32,
    pub rumours: Vec<Rumour>,
    pub next_rumour: u32,
    /// Statements awaiting coverage this week.
    pub statements: Vec<Statement>,
    /// Hidden facts already turned into scandals.
    pub discovered: BTreeSet<u32>,
    /// The last event-log entry the town news looked at.
    pub last_event: u32,
}

const KNOWLEDGE_CAP: usize = 600;
const SCANDALS_CAP: usize = 300;
const RUMOURS_CAP: usize = 60;

impl MediaState {
    pub fn scandal(&self, id: u32) -> Option<&Scandal> {
        self.scandals.iter().find(|s| s.id == id)
    }

    pub fn scandal_mut(&mut self, id: u32) -> Option<&mut Scandal> {
        self.scandals.iter_mut().find(|s| s.id == id)
    }

    pub fn story(&self, id: u32) -> Option<&Story> {
        self.stories
            .binary_search_by_key(&id, |s| s.id)
            .ok()
            .map(|i| &self.stories[i])
    }

    pub fn outlet(&self, id: u32) -> Option<&Outlet> {
        self.outlets.get(id as usize)
    }

    /// Running scandals about a politician.
    pub fn running_about(&self, p: PoliticianId) -> impl Iterator<Item = &Scandal> {
        self.scandals
            .iter()
            .filter(move |s| s.subject == p && s.running())
    }

    /// The town outlet of a municipality (its newspaper), if it has one.
    pub fn town_outlet(&self, m: u32) -> Option<&Outlet> {
        self.outlets
            .iter()
            .find(|o| o.scope == Scope::Town { municipality: m })
    }
}

fn keyed(seed: u64, parts: &[u64]) -> mandate_core::SimRng {
    mandate_core::rng::keyed(seed, Stream::Media, parts)
}

// --- Outlets ---------------------------------------------------------------------------

/// Where an outlet is generated: its scope, home place, proper name, name
/// argument, and the neighbourhoods of its audience.
type OutletSite = (Scope, Option<PlaceId>, String, String, Vec<usize>);

/// Generate the outlets (when play begins, and on loading a save from before
/// media): a town outlet per town with its home place, one per region, one
/// for the nation. Keyed by template id and scope, so the result does not
/// depend on order or on any sequential stream.
pub fn generate(state: &mut State, content: &ContentBundle) {
    if state.media.active {
        return;
    }
    let seed = state.meta.seed;
    let mut outlets = Vec::new();
    for def in &content.outlets {
        let tk = characters::key_of(&def.id);
        let scopes: Vec<OutletSite> = match def.scope {
            OutletScope::Town => state
                .world
                .municipalities
                .iter()
                .filter_map(|m| {
                    let place = m.places.iter().copied().find(|p| {
                        state
                            .world
                            .places
                            .get(p.index())
                            .is_some_and(|x| Some(x.kind.key()) == def.place.as_deref())
                    })?;
                    let name = state.world.places[place.index()].name.clone();
                    Some((
                        Scope::Town {
                            municipality: m.id.0,
                        },
                        Some(place),
                        name,
                        m.name.clone(),
                        state.world.hoods_of(m.id).collect(),
                    ))
                })
                .collect(),
            OutletScope::Region => state
                .world
                .regions
                .iter()
                .map(|r| {
                    (
                        Scope::Region { region: r.id.0 },
                        None,
                        String::new(),
                        r.name.clone(),
                        state.world.hoods_of_region(r.id),
                    )
                })
                .collect(),
            OutletScope::Nation => vec![(
                Scope::Nation,
                None,
                String::new(),
                String::new(),
                (0..state.world.neighborhoods.len()).collect(),
            )],
        };
        for (scope, place, name, arg, hoods) in scopes {
            let sk = match scope {
                Scope::Town { municipality } => 1 + municipality as u64,
                Scope::Region { region } => (1 << 32) + region as u64,
                Scope::Nation => 1 << 40,
            };
            let mut rng = keyed(seed, &[KEY_OUTLET, tk, sk]);
            // Lean: the audience's views (weighted by who follows it), plus noise.
            let mut acc = vec![0.0f64; state.population.attitude_count()];
            let mut w = 0.0;
            for &h in &hoods {
                let Some(hp) = state.population.hoods.get(h) else {
                    continue;
                };
                for (ci, c) in hp.cohorts.iter().enumerate() {
                    let a = crate::population::age_of(ci);
                    let k = crate::population::class_of(ci);
                    let ww = c.count as f64 * def.audience.class[k] * def.audience.age[a];
                    for (x, v) in acc.iter_mut().zip(&c.attitudes) {
                        *x += *v as f64 * ww;
                    }
                    w += ww;
                }
            }
            let lean: Vec<f32> = acc
                .iter()
                .map(|x| clamp(x / w.max(1.0) + rng.normal(0.0, def.lean_sd), -1.0, 1.0) as f32)
                .collect();
            let credibility = rng.range_f64(def.credibility[0], def.credibility[1]) as f32;
            let budget = rng.range_f64(def.finances[0], def.finances[1]);
            let journalists: Vec<CharacterId> = match place {
                Some(p) => state
                    .politics
                    .characters
                    .people
                    .iter()
                    .filter(|c| {
                        c.named.as_deref().is_some_and(|n| {
                            n.place == Some(p) && def.journalists.contains(&n.occupation)
                        })
                    })
                    .map(|c| c.id)
                    .collect(),
                None => Vec::new(),
            };
            let id = outlets.len() as u32;
            outlets.push(Outlet {
                id,
                template: def.id.clone(),
                scope,
                place,
                name,
                name_arg: arg,
                social: def.social,
                reach: def.reach as f32,
                class_weight: def.audience.class.map(|x| x as f32),
                age_weight: def.audience.age.map(|x| x as f32),
                lean,
                credibility,
                interests: def.interests.clone(),
                budget,
                stories_per_week: def.stories_per_week,
                journalists,
                grudges: Vec::new(),
            });
        }
    }
    let m = &mut state.media;
    m.outlets = outlets;
    m.active = true;
    m.last_event = state.events.iter().last().map(|e| e.id.0).unwrap_or(0);
}

/// Load: a save from before media gets its outlets (the rest starts empty).
pub fn on_load(state: &mut State, content: &ContentBundle) {
    if !state.media.active && state.player.is_some() && state.politics.characters.named_seeded {
        generate(state, content);
    }
}

// --- Hidden facts: exposure ------------------------------------------------------------

/// How exposed an act is: each discoverer's chance, the total, and the
/// evidence window.
#[derive(Clone, Debug, PartialEq)]
pub struct Exposure {
    pub parts: Vec<(Discoverer, f64, Option<CharacterId>)>,
    pub chance: f64,
    /// Days after the act: [earliest, latest].
    pub window: [u32; 2],
}

/// The town of a politician.
fn town_of(state: &State, p: PoliticianId) -> u32 {
    state.politics.get(p).map(|x| x.municipality()).unwrap_or(0)
}

/// A politician's rival for discovery: for the player, their town's named
/// rival; for anyone else, the most ambitious officeholder of another party
/// in their town (never the player: the player learns only from coverage).
fn rival_of(state: &State, p: PoliticianId) -> Option<CharacterId> {
    let me = state.politics.get(p)?;
    if me.is_player {
        let m = me.municipality() as usize;
        return state.politics.characters.towns.get(m).and_then(|t| t.rival);
    }
    let at = crate::world::Jurisdiction::town(MunicipalityId(me.municipality()));
    crate::politics::officials_at(&state.politics, at)
        .into_iter()
        .filter_map(|id| state.politics.get(id))
        .filter(|x| x.id != p && !x.is_player && x.party != me.party)
        .max_by(|a, b| a.ambition.total_cmp(&b.ambition).then(b.id.cmp(&a.id)))
        .map(|x| x.character)
}

/// The exposure of an act by `subject` (DEC-034): each discoverer's chance
/// is evidence × interest (by severity) × access. Journalists' access grows
/// with the town's journalists; a rival needs to exist.
pub fn exposure(
    state: &State,
    content: &ContentBundle,
    spec: &HiddenSpec,
    subject: PoliticianId,
) -> Exposure {
    let d = &content.media.discovery;
    let m = town_of(state, subject);
    let interest = d.interest.get(spec.severity);
    let ev = clamp(spec.evidence, 0.0, 1.0);
    let allowed = |x: Discoverer| spec.discoverers.is_empty() || spec.discoverers.contains(&x);
    let access = |x: Discoverer| d.access.get(x.key()).copied().unwrap_or(0.0);
    let mut parts = Vec::new();
    for x in Discoverer::ALL {
        if !allowed(x) {
            continue;
        }
        let (a, who) = match x {
            Discoverer::Journalist => {
                let js: Vec<CharacterId> = state
                    .media
                    .town_outlet(m)
                    .map(|o| o.journalists.clone())
                    .unwrap_or_default();
                let n = js.len() as f64;
                let share = d.journalist_floor
                    + (1.0 - d.journalist_floor) * (n / d.journalists_full.max(0.1)).min(1.0);
                (access(x) * share, js.first().copied())
            }
            Discoverer::Rival => match rival_of(state, subject) {
                Some(r) => (access(x), Some(r)),
                None => (0.0, None),
            },
            Discoverer::Auditor => (access(x), None),
            Discoverer::Investigator => (
                access(x)
                    * if spec.severity == Gravity::Crime {
                        1.0
                    } else {
                        0.5
                    },
                None,
            ),
        };
        if a > 0.0 {
            parts.push((x, clamp(ev * interest * a, 0.0, 1.0), who));
        }
    }
    let miss: f64 = parts.iter().map(|(_, p, _)| 1.0 - p).product();
    let lerp = |a: u32, b: u32| (b as f64 + (a as f64 - b as f64) * ev).round() as u32;
    let lo = lerp(d.window_strong[0], d.window_weak[0]);
    let hi = lerp(d.window_strong[1], d.window_weak[1]).max(lo + 1);
    Exposure {
        parts,
        chance: 1.0 - miss,
        window: [lo, hi],
    }
}

/// The chance an exposure comes out within the risk line's horizon (a
/// uniform date in the window; the director only shifts it a little).
pub fn risk_within_horizon(content: &ContentBundle, e: &Exposure) -> f64 {
    let h = content.media.discovery.horizon_days as f64;
    let [lo, hi] = e.window;
    let share = clamp((h - lo as f64) / (hi as f64 - lo as f64).max(1.0), 0.0, 1.0);
    e.chance * share
}

/// The risk band's key: `low`, `real`, or `high` (DEC-073: a coarse band).
pub fn band(content: &ContentBundle, p: f64) -> &'static str {
    let b = content.media.discovery.bands;
    if p < b[0] {
        "low"
    } else if p < b[1] {
        "real"
    } else {
        "high"
    }
}

/// Record a borderline or illegal act (the Commands phase or an NPC's turn):
/// its fate is settled now (keyed by the fact's id), its window opens after
/// the evidence-dependent delay, and the director times it from there.
pub fn hide(
    state: &mut State,
    content: &ContentBundle,
    spec: &HiddenSpec,
    subject: PoliticianId,
    actors: Vec<Actor>,
    cause: Cause,
) -> u32 {
    let date = state.calendar.date;
    let e = exposure(state, content, spec, subject);
    let id = state.narrative.next_hidden;
    let seed = state.meta.seed;
    let discovered = keyed(seed, &[KEY_FATE, id as u64]).next_f64() < e.chance;
    let total: f64 = e.parts.iter().map(|p| p.1).sum();
    let mut u = keyed(seed, &[KEY_WHO, id as u64]).next_f64() * total;
    let mut by = e
        .parts
        .first()
        .map(|p| (p.0, p.2))
        .unwrap_or((Discoverer::Journalist, None));
    for (x, p, who) in &e.parts {
        if u < *p {
            by = (*x, *who);
            break;
        }
        u -= p;
    }
    let town = Some(town_of(state, subject));
    state.narrative.hide(HiddenFact {
        id,
        kind: spec.kind.clone(),
        date,
        actors,
        window: [date.add_days(e.window[0]), date.add_days(e.window[1])],
        discoverable_on: None,
        surfaced: false,
        subject: Some(subject),
        severity: spec.severity,
        evidence: spec.evidence as f32,
        town,
        fate: Some(Fate {
            chance: e.chance as f32,
            discovered,
            by: by.0,
            who: by.1,
        }),
        cause: Some(cause),
    })
}

// --- The weekly media system -----------------------------------------------------------

/// Hostility of an outlet toward a politician (0..1): how far their views are
/// from its lean, plus any grudge.
pub fn hostility(o: &Outlet, p: &crate::politics::Politician) -> f64 {
    let n = o.lean.len().min(p.attitudes.len()).max(1);
    let d: f64 = o
        .lean
        .iter()
        .zip(&p.attitudes)
        .map(|(a, b)| (*a as f64 - *b as f64).abs())
        .sum::<f64>()
        / n as f64;
    let grudge = o
        .grudges
        .iter()
        .find(|g| g.0 == p.id)
        .map(|g| g.1 as f64)
        .unwrap_or(0.0);
    clamp((d - 0.2) * 2.0 + grudge, 0.0, 1.0)
}

/// An outlet's access to a town affair of a severity.
fn access(o: &Outlet, content: &ContentBundle, sev: Gravity) -> f64 {
    let c = &content.media.coverage;
    let w = content.media.perception.severity_weight.get(sev);
    match o.scope {
        Scope::Town { .. } => 1.0,
        _ if o.social => 0.8,
        Scope::Region { .. } => c.regional_access * w,
        Scope::Nation => c.national_access * w,
    }
}

/// How many stories an outlet runs this week.
fn capacity(o: &Outlet, content: &ContentBundle) -> usize {
    let c = &content.media.coverage;
    let k = if o.social || c.typical_budget <= 0.0 {
        1.0
    } else {
        clamp(o.budget / c.typical_budget, 0.34, 2.0)
    };
    ((o.stories_per_week as f64 * k).round() as usize).max(1)
}

struct Cand {
    source: StorySource,
    subject: Option<PoliticianId>,
    town: u32,
    score: f64,
    /// Severity weight × heat (scandal-like stories), or 0.
    weight: f64,
    /// Kind of perception: −1 harm, +1 relief, 0 statement or neutral.
    sign: f64,
    statement: bool,
}

/// Media domain, Information phase, weekly: discovery (hidden facts whose
/// date has come and whose fate is to come out), scandal heat, coverage
/// (each outlet's stories), rumours and corrections, and the perception
/// effects of what ran.
pub fn weekly(m: &mut MediaState, r: &Reads<'_>, out: &mut Outs<'_>) {
    if !m.active || r.player.is_none() {
        return;
    }
    // Hidden facts aired in public since last week (`Effect::Surface`):
    // a story now, by the rules a leak follows.
    for q in out.take(|e| matches!(e, Effect::Surface { .. })) {
        let Effect::Surface { hidden, by } = q.effect else {
            continue;
        };
        let fact = r
            .narrative()
            .hidden
            .iter()
            .find(|h| h.id == hidden && !m.discovered.contains(&h.id))
            .cloned();
        let Some(h) = fact else { continue };
        let Some(id) = open_scandal(m, r.politics(), r.world, &h, Discoverer::Rival, by, r.date, true)
        else {
            continue;
        };
        let Some(subject) = h.subject else { continue };
        let causes = h
            .cause
            .into_iter()
            .fold(Causes::from(q.cause), Causes::and)
            .and_event(out.events.head(Thread::Hidden(h.id)), Link::Revealed);
        out.events.record(
            r.date,
            EventKind::ScandalBroke {
                scandal: id,
                politician: subject.0,
                municipality: h.town.unwrap_or(0),
                severity: h.severity.key().into(),
                kind: h.kind.clone(),
            },
            causes,
        );
    }
    discover(m, r, out);
    heat(m, r, out);
    cover(m, r, out);
    // Statements are news for one week.
    m.statements.retain(|s| s.date.add_days(7) > r.date);
    if m.knowledge.len() > KNOWLEDGE_CAP {
        let drop = m.knowledge.len() - KNOWLEDGE_CAP;
        m.knowledge.drain(..drop);
    }
    let ended = m.scandals.iter().filter(|s| !s.running()).count();
    if m.scandals.len() > SCANDALS_CAP && ended > 0 {
        let mut drop = (m.scandals.len() - SCANDALS_CAP).min(ended);
        m.scandals.retain(|s| {
            if drop > 0 && !s.running() {
                drop -= 1;
                false
            } else {
                true
            }
        });
    }
    if m.rumours.len() > RUMOURS_CAP {
        let drop = m.rumours.len() - RUMOURS_CAP;
        m.rumours.drain(..drop);
    }
    let keep = r.content.media.coverage.stories_kept as usize;
    if m.stories.len() > keep {
        let drop = m.stories.len() - keep;
        m.stories.drain(..drop);
    }
    // Hidden facts trimmed from the narrative no longer need remembering.
    let live: BTreeSet<u32> = r.narrative().hidden.iter().map(|h| h.id).collect();
    m.discovered.retain(|id| live.contains(id));
}

/// Open a scandal from a discovered hidden fact.
#[allow(clippy::too_many_arguments)]
pub(crate) fn open_scandal(
    m: &mut MediaState,
    pol: &crate::politics::PoliticsState,
    world: &crate::world::World,
    h: &HiddenFact,
    by: Discoverer,
    who: Option<CharacterId>,
    date: SimDate,
    leaked: bool,
) -> Option<u32> {
    let subject = h.subject?;
    let town = h
        .town
        .unwrap_or_else(|| pol.get(subject).map(|p| p.municipality()).unwrap_or(0));
    let id = m.next_scandal;
    m.next_scandal += 1;
    m.discovered.insert(h.id);
    // Who broke it: the discoverer's outlet (a journalist), else the town
    // paper, else whoever covers the town.
    let outlet = who
        .and_then(|w| m.outlets.iter().find(|o| o.journalists.contains(&w)))
        .or_else(|| m.town_outlet(town))
        .or_else(|| {
            m.outlets
                .iter()
                .find(|o| !o.social && o.covers(world, town))
        })
        .map(|o| o.id);
    let knower = match (by, who) {
        (_, Some(p)) => Knower::Person { id: p },
        (Discoverer::Auditor | Discoverer::Investigator, None) => Knower::Office { by },
        _ => Knower::Outlet {
            id: outlet.unwrap_or(0),
        },
    };
    m.knowledge.push(Knowledge {
        hidden: h.id,
        knower,
        evidence: h.evidence,
        since: date,
    });
    if let Some(o) = outlet {
        m.knowledge.push(Knowledge {
            hidden: h.id,
            knower: Knower::Outlet { id: o },
            evidence: h.evidence,
            since: date,
        });
    }
    m.scandals.push(Scandal {
        id,
        hidden: h.id,
        subject,
        kind: h.kind.clone(),
        severity: h.severity,
        evidence: h.evidence,
        town,
        broke: date,
        by,
        who,
        outlet,
        heat: 1.0,
        stories: 0,
        response: None,
        responded: None,
        backfire_on: None,
        ended: None,
        leaked,
    });
    Some(id)
}

/// Effects of a scandal breaking: the subject's colleagues remember it, the
/// discoverer remembers it, the party's patience with the player shrinks.
fn broke_effects(r: &Reads<'_>, out: &mut Outs<'_>, s: &Scandal) {
    let pol = r.politics();
    let Some(subj) = pol.get(s.subject) else {
        return;
    };
    let actor = subj.character;
    let mut holders: Vec<CharacterId> = Vec::new();
    let at = crate::world::Jurisdiction::town(MunicipalityId(s.town));
    for id in crate::politics::officials_at(pol, at) {
        if let Some(p) = pol.get(id)
            && p.id != s.subject
            && p.party.is_some()
            && p.party == subj.party
        {
            holders.push(p.character);
        }
    }
    holders.extend(s.who);
    // The story makes the hidden fact known: it names the act that left the
    // evidence (the player's or someone's own) and, once the director has
    // logged that it can come out, links to that. The event is recorded
    // first, so that its effects name it; nothing reads the log in between.
    let made = r
        .narrative()
        .hidden
        .iter()
        .find(|h| h.id == s.hidden)
        .and_then(|h| h.cause);
    let ev = out.events.record(
        r.date,
        EventKind::ScandalBroke {
            scandal: s.id,
            politician: s.subject.0,
            municipality: s.town,
            severity: s.severity.key().into(),
            kind: s.kind.clone(),
        },
        made.into_iter()
            .fold(Causes::from(Cause::Process(Process::Press)), Causes::and)
            .and_event(out.events.head(Thread::Hidden(s.hidden)), Link::Revealed),
    );
    if subj.is_player {
        out.events.pin(ev);
    }
    for h in holders {
        out.effects.push(
            Effect::Remember {
                holder: h,
                actor,
                kind: MemoryKind::Scandal,
            },
            Cause::caused_by(ev),
        );
    }
    if subj.is_player {
        let w = r.content.media.perception.severity_weight.get(s.severity);
        out.effects.push(
            Effect::PlayerTrack {
                standing: -(r.content.media.perception.standing * w) as f32,
                viability: 0.0,
                money: 0.0,
                legal_floor: None,
            },
            Cause::caused_by(ev),
        );
    }
}

fn discover(m: &mut MediaState, r: &Reads<'_>, out: &mut Outs<'_>) {
    let date = r.date;
    let pol = r.politics();
    let due: Vec<HiddenFact> = r
        .narrative()
        .hidden
        .iter()
        .filter(|h| {
            h.fate.as_ref().is_some_and(|f| f.discovered)
                && h.discoverable_on.is_some_and(|d| d <= date)
                && !m.discovered.contains(&h.id)
        })
        .cloned()
        .collect();
    for h in due {
        let Some(f) = h.fate.clone() else { continue };
        if let Some(id) = open_scandal(m, pol, r.world, &h, f.by, f.who, date, false)
            && let Some(s) = m.scandal(id).cloned()
        {
            broke_effects(r, out, &s);
        }
    }
}

/// Heat fades weekly; a denial against strong evidence unravels.
fn heat(m: &mut MediaState, r: &Reads<'_>, out: &mut Outs<'_>) {
    let c = &r.content.media;
    let date = r.date;
    let player = r.player.map(|p| p.politician);
    let mut backfires = Vec::new();
    for s in m.scandals.iter_mut().filter(|s| s.running()) {
        if s.broke < date {
            s.heat *= c.coverage.heat_keep as f32;
        }
        if s.backfire_on.is_some_and(|d| d <= date) {
            s.backfire_on = None;
            s.heat = (s.heat * c.responses.deny_backfire_heat as f32).clamp(0.3, 1.5);
            backfires.push(s.id);
            if Some(s.subject) == player {
                // The denial unravelled: the scandal's own consequence.
                out.effects.push(
                    Effect::PlayerTrack {
                        standing: -c.responses.deny_backfire_standing as f32,
                        viability: 0.0,
                        money: 0.0,
                        legal_floor: None,
                    },
                    Cause::event_or(
                        out.events.head(Thread::Scandal(s.id)),
                        Link::Caused,
                        Cause::Process(Process::Press),
                    ),
                );
            }
        }
        if (s.heat as f64) < c.coverage.heat_end && s.backfire_on.is_none() {
            s.ended = Some(date);
        }
    }
    let _ = backfires;
}

/// The stories one outlet could run this week.
fn candidates(
    m: &MediaState,
    o: &Outlet,
    r: &Reads<'_>,
    week_start: SimDate,
    news: &[(EventId, u32)],
    scenes: &[(EventId, u32, i32)],
) -> Vec<Cand> {
    let content = r.content;
    let cv = &content.media.coverage;
    let pw = &content.media.perception;
    let pol = r.politics();
    let mut out = Vec::new();
    let interest = |subject: PoliticianId| -> f64 {
        1.0 + cv.hostility_interest * pol.get(subject).map(|p| hostility(o, p)).unwrap_or(0.0)
    };
    for s in m.scandals.iter().filter(|s| s.running()) {
        if !o.covers(r.world, s.town) {
            continue;
        }
        let a = access(o, content, s.severity);
        // The outlet that broke it always runs the first story.
        let breaking = s.stories == 0 && s.outlet == Some(o.id);
        let score = cv.newsworthiness.get(s.severity)
            * s.heat as f64
            * interest(s.subject)
            * a
            * if breaking { 10.0 } else { 1.0 };
        let fresh_response = s.responded.is_some_and(|d| d >= week_start);
        let backfire = s.backfire_on.is_none()
            && s.response == Some(ScandalResponseKind::Deny)
            && s.evidence as f64 >= content.media.responses.deny_strong
            && s.responded.is_some_and(|d| d < week_start)
            && s.heat > 0.29
            && s.stories > 0
            && fresh_backfire(m, s.id, o.id);
        let source = if backfire {
            StorySource::Backfire { scandal: s.id }
        } else if fresh_response && let Some(resp) = s.response {
            StorySource::Response {
                scandal: s.id,
                response: resp,
            }
        } else {
            StorySource::Scandal { id: s.id }
        };
        out.push(Cand {
            source,
            subject: Some(s.subject),
            town: s.town,
            score,
            weight: pw.severity_weight.get(s.severity) * s.heat as f64,
            sign: -1.0,
            statement: false,
        });
    }
    // Legal developments this week (cases change stage in the Institutions phase).
    for c in r.justice().cases.iter() {
        let Some(&(stage, on)) = c.history.last() else {
            continue;
        };
        if on < week_start || !o.covers(r.world, c.town) || o.social && stage.key() == "dropped" {
            continue;
        }
        let a = access(o, content, c.severity);
        let w = pw.severity_weight.get(c.severity) * pw.legal_story;
        let (sign, mult) = match stage {
            crate::justice::Stage::Investigation => (-1.0, 0.6),
            crate::justice::Stage::Charged => (-1.0, 1.0),
            crate::justice::Stage::Dropped => (1.0, 0.3),
            crate::justice::Stage::Acquitted => (1.0, pw.acquittal),
            crate::justice::Stage::Convicted => (-1.0, 1.2),
            crate::justice::Stage::Adjourned => continue,
        };
        out.push(Cand {
            source: StorySource::Case {
                id: c.id,
                stage: stage.key().into(),
            },
            subject: Some(c.subject),
            town: c.town,
            score: cv.newsworthiness.get(c.severity) * 1.2 * a * interest(c.subject),
            weight: w * mult,
            sign,
            statement: false,
        });
    }
    // Rumours spread on social platforms; credible outlets may correct them.
    for u in &m.rumours {
        if !o.covers(r.world, u.town) {
            continue;
        }
        if u.corrected.is_none() && o.social && u.weeks < content.media.rumours.spread_weeks {
            out.push(Cand {
                source: StorySource::Rumour { id: u.id },
                subject: Some(u.about),
                town: u.town,
                score: cv.newsworthiness.misconduct * 0.8 * interest(u.about),
                weight: pw.severity_weight.misconduct * content.media.rumours.weight,
                sign: -1.0,
                statement: false,
            });
        }
        if u.corrected.is_none() && !o.social && o.credibility > 0.6 && u.weeks > 0 {
            let p = content.media.rumours.correction_chance * o.credibility as f64;
            let draw = keyed(
                r.meta.seed,
                &[KEY_CORRECT, u.id as u64, o.id as u64, r.date.0 as u64],
            )
            .next_f64();
            if draw < p {
                out.push(Cand {
                    source: StorySource::Correction { rumour: u.id },
                    subject: Some(u.about),
                    town: u.town,
                    score: 1.0,
                    weight: 0.0,
                    sign: 1.0,
                    statement: false,
                });
            }
        }
    }
    // Statements: an outlet covers one if it is news to it (DEC-034: it does
    // not just repeat the politician).
    for st in &m.statements {
        let Some(p) = pol.get(st.politician) else {
            continue;
        };
        let town = p.municipality();
        if !o.covers(r.world, town) || o.social {
            continue;
        }
        let scandal = m.running_about(st.politician).next().is_some();
        let score = (0.25 + if scandal { 0.5 } else { 0.0 })
            * if matches!(o.scope, Scope::Town { .. }) {
                1.0
            } else {
                0.3
            };
        out.push(Cand {
            source: StorySource::Statement {
                politician: st.politician,
                policy: st.policy,
            },
            subject: Some(st.politician),
            town,
            score,
            weight: 0.0,
            sign: 0.0,
            statement: true,
        });
    }
    // Town news: the local paper reports what happened in town (no perception effect).
    if let Scope::Town { municipality } = o.scope
        && cv.town_news
    {
        for (e, x) in news {
            if *x == municipality {
                out.push(Cand {
                    source: StorySource::Town { event: *e },
                    subject: None,
                    town: municipality,
                    score: 0.4,
                    weight: 0.0,
                    sign: 0.0,
                    statement: false,
                });
            }
        }
        // A scene (DEC-112): the paper was in the room. Its weight is the
        // result against the scene's own thresholds; the framing is the
        // outlet's, as for any story about the player.
        if let Some(pl) = r.player {
            let x = &content.exchange;
            for (e, town, result_pm) in scenes {
                if *town != municipality {
                    continue;
                }
                let d = f64::from(*result_pm) / 1000.0;
                let line = if d >= 0.0 { x.win } else { x.lose }.max(1e-6);
                out.push(Cand {
                    source: StorySource::Exchange { event: *e },
                    subject: Some(pl.politician),
                    town: municipality,
                    score: 0.6,
                    weight: x.world.story_weight * (d.abs() / line).min(1.0),
                    sign: if d > 0.0 {
                        1.0
                    } else if d < 0.0 {
                        -1.0
                    } else {
                        0.0
                    },
                    statement: false,
                });
            }
        }
    }
    out
}

/// A denial's unravelling is reported once (by the outlet that broke it).
fn fresh_backfire(m: &MediaState, scandal: u32, outlet: u32) -> bool {
    m.scandal(scandal).is_some_and(|s| s.outlet == Some(outlet))
        && !m
            .stories
            .iter()
            .rev()
            .take(200)
            .any(|x| matches!(x.source, StorySource::Backfire { scandal: y } if y == scandal))
}

fn source_order(s: &StorySource) -> (u8, u32) {
    match s {
        StorySource::Scandal { id } => (0, *id),
        StorySource::Response { scandal, .. } => (1, *scandal),
        StorySource::Backfire { scandal } => (2, *scandal),
        StorySource::Case { id, .. } => (3, *id),
        StorySource::Rumour { id } => (4, *id),
        StorySource::Correction { rumour } => (5, *rumour),
        StorySource::Statement { politician, .. } => (6, politician.0),
        StorySource::Town { event } => (7, event.0),
        StorySource::Exchange { event } => (8, event.0),
    }
}

fn cover(m: &mut MediaState, r: &Reads<'_>, out: &mut Outs<'_>) {
    let content = r.content;
    let pw = &content.media.perception;
    let date = r.date;
    let week_start = SimDate(date.0.saturating_sub(6));
    let pol = r.politics();
    // Town news: what the event log recorded in a town since last week.
    let mut news: Vec<(EventId, u32)> = Vec::new();
    let mut scenes: Vec<(EventId, u32, i32)> = Vec::new();
    let mut last = m.last_event;
    for e in out.events.iter().filter(|e| e.id.0 > m.last_event) {
        last = last.max(e.id.0);
        match &e.kind {
            EventKind::PlantClosure {
                municipality: x, ..
            }
            | EventKind::Disaster {
                municipality: x, ..
            }
            | EventKind::PolicyEnacted {
                municipality: x, ..
            }
            | EventKind::PolicyFailed {
                municipality: x, ..
            }
            | EventKind::RaceDecided {
                municipality: x, ..
            } => news.push((e.id, *x)),
            // An arc's story (DEC-109): a press beat, or an exit with a
            // headline, in the town it is about.
            EventKind::ArcBeat { at, kind, .. } if kind == "press" => {
                if let Some(m) = at.municipality() {
                    news.push((e.id, m.0));
                }
            }
            // A scene's result (DEC-112), in the town it was held in.
            EventKind::ArcExchangeEnded { at, result_pm, .. } => {
                if let Some(m) = at.municipality() {
                    scenes.push((e.id, m.0, *result_pm));
                }
            }
            EventKind::ArcEnded {
                at, template, exit, ..
            } => {
                let headline = content
                    .arc(template)
                    .and_then(|a| a.stage(exit))
                    .and_then(|s| s.exit.as_ref())
                    .is_some_and(|x| x.headline_key.is_some());
                if let (true, Some(m)) = (headline, at.municipality()) {
                    news.push((e.id, m.0));
                }
            }
            _ => {}
        }
    }
    let mut ran: Vec<Story> = Vec::new();
    for oi in 0..m.outlets.len() {
        let o = &m.outlets[oi];
        let mut cands = candidates(m, o, r, week_start, &news, &scenes);
        cands.retain(|c| c.score >= content.media.coverage.min_score);
        cands.sort_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then(source_order(&a.source).cmp(&source_order(&b.source)))
        });
        cands.truncate(capacity(o, content));
        for c in cands {
            let hostile = c
                .subject
                .and_then(|s| pol.get(s))
                .map(|p| hostility(o, p))
                .unwrap_or(0.0);
            let cred = o.credibility as f64;
            let (framing, impact, aware) = if c.statement {
                // Sympathetic where the outlet agrees with them, hostile where not.
                let f = clamp(0.6 - 1.2 * hostile, -1.0, 1.0);
                (f, pw.statement * f * cred, pw.awareness * 0.5)
            } else if c.sign < 0.0 {
                let f = -hostile;
                (
                    f,
                    -pw.approval * c.weight * cred * (1.0 + pw.framing * hostile),
                    pw.awareness,
                )
            } else if c.sign > 0.0 {
                // Good news for the subject: a correction, a case dropped, an acquittal.
                let w = match &c.source {
                    StorySource::Correction { .. } => {
                        pw.severity_weight.misconduct
                            * content.media.rumours.weight
                            * content.media.rumours.correction_share
                    }
                    _ => c.weight,
                };
                (0.3, pw.approval * w * cred, 0.0)
            } else {
                (0.0, 0.0, 0.0)
            };
            let id = m.next_story;
            m.next_story += 1;
            ran.push(Story {
                id,
                outlet: o.id,
                date,
                source: c.source,
                subject: c.subject,
                town: c.town,
                framing: framing as f32,
                impact: impact as f32,
                aware: aware as f32,
                score: c.score as f32,
            });
        }
    }
    // Apply what ran: counts, rumour spread, perception (DEC-034).
    let player = r.player.map(|p| p.politician);
    for s in &ran {
        let o = &m.outlets[s.outlet as usize];
        let cred = o.credibility as f64;
        // What the story makes known (the spine): its effects name that.
        let cause = story_cause(out.events, &s.source);
        match &s.source {
            StorySource::Scandal { id }
            | StorySource::Response { scandal: id, .. }
            | StorySource::Backfire { scandal: id } => {
                if let Some(x) = m.scandals.iter_mut().find(|x| x.id == *id) {
                    x.stories += 1;
                }
            }
            StorySource::Rumour { id } => {
                if let Some(u) = m.rumours.iter_mut().find(|u| u.id == *id) {
                    u.weeks += 1;
                }
            }
            StorySource::Correction { rumour } => {
                if let Some(u) = m.rumours.iter_mut().find(|u| u.id == *rumour) {
                    u.corrected = Some(date);
                    let back = u.harm as f64 * content.media.rumours.correction_share;
                    let ev = out.events.record(
                        date,
                        EventKind::RumourCorrected {
                            rumour: u.id,
                            politician: u.about.0,
                        },
                        Cause::Process(Process::Press),
                    );
                    out.effects.push(
                        Effect::Valence {
                            politician: u.about,
                            delta: -back as f32,
                        },
                        Cause::caused_by(ev),
                    );
                }
            }
            _ => {}
        }
        let Some(subject) = s.subject else { continue };
        // Elections read a politician's scandal valence: the same for anyone.
        if s.impact < 0.0 {
            let v = -(s.impact as f64) / pw.approval.max(1e-6) * pw.valence * o.reach as f64;
            out.effects.push(
                Effect::Valence {
                    politician: subject,
                    delta: v as f32,
                },
                cause,
            );
            if let StorySource::Rumour { id } = s.source
                && let Some(u) = m.rumours.iter_mut().find(|u| u.id == id)
            {
                u.harm += v as f32;
            }
        } else if s.impact > 0.0
            && !matches!(
                s.source,
                StorySource::Statement { .. } | StorySource::Correction { .. }
            )
        {
            let v = s.impact as f64 / pw.approval.max(1e-6) * pw.valence * o.reach as f64 * 0.5;
            out.effects.push(
                Effect::Valence {
                    politician: subject,
                    delta: -v as f32,
                },
                cause,
            );
        }
        if Some(subject) == player && (s.impact != 0.0 || s.aware != 0.0) {
            out.effects.push(Effect::Perceive { story: s.id }, cause);
        }
        let _ = cred;
    }
    // The player reads the paper (DEC-091, §1.3): everyone a story in an
    // outlet covering the player's town names is heard of. An effect
    // request to the player domain, posted only when there is a player.
    if let Some(pl) = r.player {
        let town = pl.municipality.0;
        for s in &ran {
            let covers = m
                .outlets
                .get(s.outlet as usize)
                .is_some_and(|o| o.covers(r.world, town));
            let named = s
                .subject
                .filter(|p| *p != pl.politician)
                .and_then(|p| pol.get(p))
                .map(|p| p.character);
            if let (true, Some(person)) = (covers, named) {
                out.effects.push(
                    Effect::Learn {
                        subject: crate::player::knowledge::Subject::Person { person },
                        level: crate::player::knowledge::Level::Heard,
                        source: crate::player::knowledge::Source::Story { story: s.id },
                    },
                    story_cause(out.events, &s.source),
                );
            }
        }
    }
    m.stories.extend(ran);
    m.last_event = last;
}

/// What a story makes known, as a cause: the latest event of the scandal,
/// case or rumour it covers, or the town event itself; a statement (or a
/// matter whose thread has closed) is the press's own doing.
fn story_cause(log: &crate::events::EventLog, source: &StorySource) -> Cause {
    let about = match source {
        StorySource::Scandal { id }
        | StorySource::Response { scandal: id, .. }
        | StorySource::Backfire { scandal: id } => log.head(Thread::Scandal(*id)),
        StorySource::Case { id, .. } => log.head(Thread::Case(*id)),
        StorySource::Rumour { id } | StorySource::Correction { rumour: id } => {
            log.head(Thread::Rumour(*id))
        }
        StorySource::Town { event } | StorySource::Exchange { event } => Some(*event),
        StorySource::Statement { .. } => None,
    };
    Cause::event_or(about, Link::Revealed, Cause::Process(Process::Press))
}

// --- The player's communication and acts (the Commands phase) -------------------------

/// The player (or anyone) answers a scandal: the heat changes at once;
/// a denial against strong evidence will unravel; an apology is an
/// admission; a counter-attack makes the outlet that broke it hostile.
pub fn respond(
    state: &mut State,
    content: &ContentBundle,
    scandal: u32,
    kind: ScandalResponseKind,
) {
    let x = &content.media.responses;
    let date = state.calendar.date;
    let Some(s) = state.media.scandal_mut(scandal) else {
        return;
    };
    s.response = Some(kind);
    s.responded = Some(date);
    match kind {
        ScandalResponseKind::None => {}
        ScandalResponseKind::Deny => {
            if (s.evidence as f64) < x.deny_strong {
                s.heat *= x.deny_heat as f32;
            } else {
                s.backfire_on = Some(date.add_days(14));
            }
        }
        ScandalResponseKind::Explain => s.heat *= x.explain_heat as f32,
        ScandalResponseKind::Apologise => {
            s.heat *= x.apologise_heat as f32;
            s.evidence = (s.evidence + x.apologise_evidence as f32).min(1.0);
        }
        ScandalResponseKind::CounterAttack => {
            s.heat *= x.counter_heat as f32;
            let (subject, outlet) = (s.subject, s.outlet);
            if let Some(o) = outlet.and_then(|o| state.media.outlets.get_mut(o as usize)) {
                match o.grudges.iter_mut().find(|g| g.0 == subject) {
                    Some(g) => g.1 = (g.1 + x.counter_hostility as f32).min(1.0),
                    None => o.grudges.push((subject, x.counter_hostility as f32)),
                }
            }
        }
    }
}

/// A statement to the press: outlets decide next week whether and how to
/// cover it; a scandal of theirs cools a little.
pub fn statement(state: &mut State, content: &ContentBundle, p: PoliticianId, policy: Option<u32>) {
    let date = state.calendar.date;
    let keep = content.media.responses.statement_heat as f32;
    for s in state
        .media
        .scandals
        .iter_mut()
        .filter(|s| s.subject == p && s.running())
    {
        s.heat *= keep;
    }
    state.media.statements.push(Statement {
        politician: p,
        date,
        policy,
    });
}

/// An ad buy changes the subject: running scandals cool a little.
pub fn advertised(state: &mut State, content: &ContentBundle, p: PoliticianId) {
    let keep = content.media.responses.ad_heat as f32;
    for s in state
        .media
        .scandals
        .iter_mut()
        .filter(|s| s.subject == p && s.running())
    {
        s.heat *= keep;
    }
}

/// A hidden fact aired in public (DEC-112, `Effect::Surface`): it comes
/// out now as a story, as a leaked fact does. `by`: who said it. Nothing
/// when the fact is already out or was never whose it was said to be.
pub fn surface(state: &mut State, hidden: u32, by: Option<CharacterId>, cause: Cause) -> Option<u32> {
    let date = state.calendar.date;
    let h = state
        .narrative
        .hidden
        .iter()
        .find(|h| h.id == hidden && !state.media.discovered.contains(&h.id))
        .cloned()?;
    let subject = h.subject?;
    let id = open_scandal(
        &mut state.media,
        &state.politics,
        &state.world,
        &h,
        Discoverer::Rival,
        by,
        date,
        true,
    )?;
    // Whoever aired it brought it out (and the subject's own act, which
    // the airing only exposed).
    let causes = h
        .cause
        .into_iter()
        .fold(Causes::from(cause), Causes::and)
        .and_event(state.events.head(Thread::Hidden(h.id)), Link::Revealed);
    state.events.record(
        date,
        EventKind::ScandalBroke {
            scandal: id,
            politician: subject.0,
            municipality: h.town.unwrap_or(0),
            severity: h.severity.key().into(),
            kind: h.kind.clone(),
        },
        causes,
    );
    Some(id)
}

/// What a leak did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Leak {
    /// It exposed one of the rival's own hidden facts: a scandal now.
    Exposed { scandal: u32 },
    /// There was nothing behind it: an unverified claim spreads.
    Rumour { id: u32 },
}

/// Leak a rival's file to a reporter (the player's shady act at the
/// newspaper): if the rival has a hidden fact still unknown, it comes out
/// now; otherwise a rumour starts. Either way the leak is itself a hidden
/// fact of the leaker (recorded by the caller first, as `leak`).
pub fn leak(state: &mut State, rival: PoliticianId, leak: u32) -> Option<Leak> {
    let date = state.calendar.date;
    let secret = state
        .narrative
        .hidden
        .iter()
        .find(|h| h.subject == Some(rival) && !state.media.discovered.contains(&h.id))
        .cloned();
    if let Some(h) = secret {
        let who = state
            .player
            .as_ref()
            .and_then(|p| state.politics.get(p.politician))
            .map(|p| p.character);
        let id = open_scandal(
            &mut state.media,
            &state.politics,
            &state.world,
            &h,
            Discoverer::Rival,
            None,
            date,
            true,
        )?;
        let _ = who;
        // The player's leak brought the rival's hidden fact out.
        // (And the rival's own act, which the leak only exposed.)
        let cause = h
            .cause
            .into_iter()
            .fold(
                Causes::from(Cause::Player(PlayerAct::LeakFile)),
                Causes::and,
            )
            .and_event(state.events.head(Thread::Hidden(h.id)), Link::Revealed);
        state.events.record(
            date,
            EventKind::ScandalBroke {
                scandal: id,
                politician: rival.0,
                municipality: h.town.unwrap_or(0),
                severity: h.severity.key().into(),
                kind: h.kind.clone(),
            },
            cause,
        );
        return Some(Leak::Exposed { scandal: id });
    }
    let town = state.politics.get(rival).map(|p| p.municipality())?;
    let id = state.media.next_rumour;
    state.media.next_rumour += 1;
    state.media.rumours.push(Rumour {
        id,
        about: rival,
        leak,
        town,
        started: date,
        weeks: 0,
        harm: 0.0,
        corrected: None,
    });
    let cause = Cause::Player(PlayerAct::LeakFile);
    state.events.record(
        date,
        EventKind::RumourSpread {
            rumour: id,
            politician: rival.0,
        },
        cause,
    );
    Some(Leak::Rumour { id })
}

/// The name arguments of an outlet: (Fluent key, `$place`).
pub fn outlet_name(content: &ContentBundle, o: &Outlet) -> (String, String) {
    let key = content
        .outlet(&o.template)
        .map(|d| d.name_key.clone())
        .unwrap_or_default();
    let arg = if o.name.is_empty() {
        o.name_arg.clone()
    } else {
        o.name.clone()
    };
    (key, arg)
}

/// The municipality id type, for callers.
pub fn town(m: u32) -> MunicipalityId {
    MunicipalityId(m)
}

/// Population domain, Opinion phase, daily: what the stories about the
/// player did to the voters each outlet reaches (DEC-034 perception; DEC-025
/// perceived impact as shaped by media). Exposure is the outlet's reach ×
/// the cohort's audience weight (normalised over the town's adults); the
/// change in approval is room-scaled like every approval change.
pub fn perceive(pop: &mut crate::population::PopulationState, r: &Reads<'_>, out: &mut Outs<'_>) {
    let media = r.media();
    for e in out.take(|e| matches!(e, Effect::Perceive { .. })) {
        let Effect::Perceive { story } = e.effect else {
            continue;
        };
        let Some(s) = media.story(story) else {
            continue;
        };
        let Some(o) = media.outlet(s.outlet) else {
            continue;
        };
        let hoods: Vec<usize> = r.world.hoods_of(MunicipalityId(s.town)).collect();
        let (mut ww, mut n) = (0.0, 0.0);
        for &h in &hoods {
            for (ci, c) in pop.hoods[h].cohorts.iter().enumerate() {
                let w = o.weight(
                    crate::population::age_of(ci),
                    crate::population::class_of(ci),
                );
                ww += w * c.count as f64;
                n += c.count as f64;
            }
        }
        let mean = (ww / n.max(1.0)).max(1e-6);
        for &h in &hoods {
            for (ci, c) in pop.hoods[h].cohorts.iter_mut().enumerate() {
                let w = o.weight(
                    crate::population::age_of(ci),
                    crate::population::class_of(ci),
                );
                let x = clamp(o.reach as f64 * w / mean, 0.0, 1.0);
                let aw = c.player_awareness as f64;
                c.player_awareness = clamp(aw + s.aware as f64 * x * (1.0 - aw), 0.0, 1.0) as f32;
                let room = 1.0 - (c.player_approval as f64).abs();
                c.player_approval = clamp(
                    c.player_approval as f64 + s.impact as f64 * x * room,
                    -1.0,
                    1.0,
                ) as f32;
            }
        }
    }
}

/// The grounding audit (REQ-003, DEC-034): every story traces to a fact.
/// Problems with a story's source now (empty = grounded). Used by tests on
/// the day a story runs; read-only.
pub fn audit_story(state: &State, s: &Story) -> Vec<String> {
    let mut out = Vec::new();
    let mut bad = |m: String| out.push(format!("story {}: {m}", s.id));
    let hidden = |id: u32| state.narrative.hidden.iter().find(|h| h.id == id);
    match &s.source {
        StorySource::Scandal { id }
        | StorySource::Response { scandal: id, .. }
        | StorySource::Backfire { scandal: id } => match state.media.scandal(*id) {
            None => bad(format!("no scandal {id}")),
            Some(x) => {
                if Some(x.subject) != s.subject {
                    bad("the subject is not the scandal's".into());
                }
                match hidden(x.hidden) {
                    None => bad(format!("no hidden fact {}", x.hidden)),
                    Some(h) => {
                        if h.subject != Some(x.subject) {
                            bad("the hidden fact is someone else's".into());
                        }
                        if !x.leaked {
                            let fated = h.fate.as_ref().is_some_and(|f| f.discovered);
                            let timed = h.discoverable_on.is_some_and(|d| {
                                d >= h.window[0] && d <= h.window[1] && d <= x.broke
                            });
                            if !fated || !timed {
                                bad("it came out outside its fate or its window".into());
                            }
                        }
                    }
                }
                if matches!(s.source, StorySource::Response { .. }) && x.response.is_none() {
                    bad("no response was given".into());
                }
            }
        },
        StorySource::Case { id, stage } => match state.justice.case(*id) {
            None => bad(format!("no case {id}")),
            Some(c) => {
                if !c.history.iter().any(|(st, _)| st.key() == stage) {
                    bad(format!("the case never reached `{stage}`"));
                }
                if Some(c.subject) != s.subject {
                    bad("the subject is not the case's".into());
                }
            }
        },
        StorySource::Rumour { id } | StorySource::Correction { rumour: id } => {
            match state.media.rumours.iter().find(|u| u.id == *id) {
                None => bad(format!("no rumour {id}")),
                Some(u) => {
                    if hidden(u.leak).is_none_or(|h| h.subject == Some(u.about)) {
                        bad("the rumour does not trace to someone else's leak".into());
                    }
                    if matches!(s.source, StorySource::Correction { .. }) && u.corrected.is_none() {
                        bad("not corrected".into());
                    }
                }
            }
        }
        StorySource::Statement { politician, .. } => {
            if state.politics.get(*politician).is_none() {
                bad("no such politician".into());
            }
        }
        StorySource::Town { event } => match state.events.get(*event).map(|e| &e.kind) {
            Some(EventKind::PlantClosure { municipality, .. })
            | Some(EventKind::Disaster { municipality, .. })
            | Some(EventKind::PolicyEnacted { municipality, .. })
            | Some(EventKind::PolicyFailed { municipality, .. })
            | Some(EventKind::RaceDecided { municipality, .. }) => {
                if *municipality != s.town {
                    bad("the event is in another town".into());
                }
            }
            // An arc's story (DEC-109): a press beat, or an exit (those
            // with a headline are news), in the town it is about.
            Some(EventKind::ArcBeat { at, kind, .. }) if kind == "press" => {
                if at.municipality().map(|m| m.0) != Some(s.town) {
                    bad("the arc is in another town".into());
                }
            }
            Some(EventKind::ArcEnded { at, .. }) => {
                if at.municipality().map(|m| m.0) != Some(s.town) {
                    bad("the arc is in another town".into());
                }
            }
            _ => bad("no such event".into()),
        },
        StorySource::Exchange { event } => match state.events.get(*event).map(|e| &e.kind) {
            Some(EventKind::ArcExchangeEnded { at, .. }) => {
                if at.municipality().map(|m| m.0) != Some(s.town) {
                    bad("the scene was in another town".into());
                }
                if s.subject != state.player.as_ref().map(|p| p.politician) {
                    bad("the subject is not the player".into());
                }
            }
            _ => bad("no such scene".into()),
        },
    }
    out
}
