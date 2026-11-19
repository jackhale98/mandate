# M6 engine contracts: the knowledge ledger, policy families, and conversations

**Status:** Interface-first design brief for M6 (2026-11-19). It turns DEC-091 (knowledge is earned), DEC-092 (policy families grounded in the town) and DEC-093 (conversations) into exact contracts for implementation agents. Nothing here changes an accepted decision. Choices these decisions leave open are marked **Proposed** and are listed again in §5 for the owner.
**Built on:** `main` at save format v13, plus the M5d part 2 branch (situational policies, `SituationTrigger`, the policy preview query, the overheard registry), which must merge first.
**Read with:** DEC-031, DEC-034, DEC-035, DEC-041, DEC-053, DEC-054, DEC-073, DEC-077, DEC-078, DEC-079, DEC-088, DEC-089, DEC-090, DEC-091, DEC-092, DEC-093; OVERVIEW §3 and §4.3; `docs/design/research/TOWN_GENERATION.md` §4, §10, §11 and §15.

---

## 0. In short

- **Knowledge ledger.** The player has a notebook in the engine: for each *subject* (a neighbourhood's worries, the power broker, a person, a proposal, a hidden place) it records a *level* (unknown, heard, known, detailed), when and how each level was reached, and, for numbers, the reading and its date. Protocol views read the ledger and hide or coarsen what the player does not know. The UI never decides.
- **Learning** happens through commands the player makes anyway: talking to people (the main source), arriving at a place, canvassing, lobbying, council work, decisions. Other systems (the press, the director, the policy system) post learning as effect requests, which the player domain applies.
- **Policy families.** A family is a content template with *slots* that bind to the real town: "Rescue deal for Halton Works", "Flood wall for Riverside", "Licences for the Corn Market". Each run draws its own deck of families. An instance opens when its trigger holds, offers two or three *choice axes* (where, how paid, who delivers), and may come with *sponsors* (the union, the traders) who bring a ready version and a price. Meeting the sponsor is how you learn of it.
- **Conversations** (§3). The player talks to someone by topic (their worries, who really runs things, a subject, a pitch, support, a favour, small talk). The engine composes the reply from content lines: an opener set by the relationship, the substance set by the facts and by what the speaker will tell, a closer in the speaker's voice, sometimes a quote from their memories. Replies teach the ledger, give leads, reveal sponsors' proposals and move relationships. Each conversation costs time, and people don't repeat themselves.
- **Everything new is content** except a short list of typed extension points (§2.15, §3.14).

---

## 1. Knowledge ledger (DEC-091)

### 1.1 Levels and subjects

Levels are ordered and only ever rise (identities never fade, DEC-091). Numbers carry their date instead of fading.

| Level | Meaning | Typical source |
|---|---|---|
| `unknown` | Nothing | — |
| `heard` | A rumour or a name | Overheard talk, a story in the paper, a decision naming someone, a party primer |
| `known` | Met or seen first-hand | Meeting someone, canvassing a neighbourhood, visiting a place, a town hall meeting |
| `detailed` | From the right place or person | The statistics desk, internal polls, a second conversation, a sponsor's own briefing |

| Subject | `heard` shows | `known` adds | `detailed` adds |
|---|---|---|---|
| `town_concerns { town }` | The town's top worry | The top three, ranked | Each concern's level, dated |
| `hood_concerns { hood }` | The neighbourhood's top worry | Its top three, ranked | Levels, dated |
| `broker { town }` | A name, marked as rumour | The broker, confirmed | Their stake (the industry or crisis behind them) |
| `person { person }` | Name and role | Personality, traits, expertise, where to find them, convictions | Goals (DEC-032), favours they hold with others, pragmatism |
| `rival_plans { town }` | "The rival is planning something" | Which seat they mean to contest | Their focus (the neighbourhoods and issue they target) |
| `policy_support { town, policy }` | A word: popular, divisive or unpopular | Support by class, to 0.1 | Exact support by class and by neighbourhood, dated |
| `proposal { proposal }` | It exists: title, the sponsor's role, a lead | The full menu: axes, options, costs, the sponsors by name | The sponsors' own versions and prices, who else backs it |
| `situational { town, template }` | **Transitional** until F4 converts the four M5d templates (§2.20): as `proposal` | | |
| `place { place }` | Hidden places only (DEC-090 D7): a hint | Found: listed, routable | (Its depth service, if any) |
| `layer { layer, town }` | — | A gated map layer (DEC-089, M7) | Dated |
| `secret { hidden }` | Gossip about someone else's hidden fact (DEC-034) | You hold evidence | — |

**Never gated** (DEC-091): the rules; the player's own state (tracks, money, slots, Mandate, record, filing, favours owed to or by the player, memories others hold of the player, opinion and trust bands toward the player); what an action does, its cost, its odds and risks (DEC-073: lobbying odds, gaffe and hostile-crowd risk, shady risk lines); deadlines; anything aimed at the player (decisions, stories about them, cases); the standard policy menu with its costs and side effects; public statistics (unemployment, rents, the defining industry, the live crisis); election calendars, candidates, polls and results; the rival's identity.

**Ordinary places are always known.** Only hidden places (DEC-090 D7) use `place`. This replaces the `PlayerState.known_places` set proposed in TOWN_GENERATION §11: discovery is a ledger entry.

### 1.2 Rust types (`crates/mandate-sim/src/player/knowledge.rs`, new)

```rust
use crate::characters::CharacterId;
use crate::narrative::DecisionId;
use crate::policy::{FamilyInstanceId, PolicyInstanceId};
use crate::politics::PartyId;
use crate::world::{MunicipalityId, NeighborhoodId, PlaceId};
use mandate_content::ContentKey;
use mandate_core::SimDate;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Level { #[default] Unknown, Heard, Known, Detailed }

/// Something the player can know. Ordered (BTreeMap key, DEC-041); typed ids.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Subject {
    TownConcerns { town: MunicipalityId },
    HoodConcerns { hood: NeighborhoodId },
    Broker { town: MunicipalityId },
    Person { person: CharacterId },
    RivalPlans { town: MunicipalityId },
    PolicySupport { town: MunicipalityId, policy: PolicyRef },
    Proposal { proposal: FamilyInstanceId },
    /// Transitional (until the M5d situational templates become families).
    Situational { town: MunicipalityId, template: ContentKey },
    Place { place: PlaceId },
    Layer { layer: ContentKey, town: MunicipalityId },
    /// A hidden fact (DEC-034). Raw u32 until hidden facts get a typed id (§6).
    Secret { hidden: u32 },
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PolicyRef {
    Template { template: ContentKey },
    Proposal { proposal: FamilyInstanceId },
}

/// How something was learned. Facts only: the UI words it (`learned-<type>`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Source {
    Start { profile: ContentKey },
    LocalOption,
    Primer { party: Option<PartyId> },
    Migrated,
    Visit { place: PlaceId },
    Overheard { place: PlaceId, line: ContentKey },
    /// A player action (`canvass`, `lobby`, `town_hall`, …), where it was done.
    Action { action: String, place: Option<PlaceId> },
    Met { person: CharacterId },
    Service { place: PlaceId, service: String },
    Story { story: u32 },
    Decision { decision: DecisionId },
    Council { policy: PolicyInstanceId },
    Sponsor { proposal: FamilyInstanceId, person: CharacterId },
    /// A conversation (DEC-093, §3).
    Talk { person: CharacterId, topic: ContentKey },
    Lab,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Learned { pub level: Level, pub on: SimDate, pub source: Source }

/// A dated reading of a numeric subject: values quantised to per mille, by
/// key (a concern id, `low`/`middle`/`high`, a neighbourhood id as a string).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Reading { pub on: SimDate, pub source: Source, pub values: Vec<(String, i32)> }

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub level: Level,
    /// The first time each level was reached, lowest first (at most three).
    pub learned: Vec<Learned>,
    /// The latest reading, for numeric subjects at `detailed`.
    pub reading: Option<Reading>,
}

/// Where a lead points. A lead is guidance, never a requirement.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LeadTarget {
    Place { place: PlaceId },
    PlaceKind { town: MunicipalityId, kind: String },
    Person { person: CharacterId },
    Streets { hood: NeighborhoodId },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Lead {
    pub subject: Subject,
    /// The level following the lead gives.
    pub level: Level,
    /// Fluent key (`lead-…`); its arguments come from the subject and target.
    pub key: String,
    pub to: LeadTarget,
    pub since: SimDate,
    pub source: Source,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Zone { Town { town: MunicipalityId } } // Region and Nation from M7–M8

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LedgerOrigin {
    /// Not seeded yet (the serde default: older saves are seeded on load).
    #[default]
    Unseeded,
    Profile { profile: ContentKey },
    /// The "Start with local knowledge" option (DEC-091, off by default).
    LocalOption { profile: ContentKey },
    Migrated,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeLedger {
    pub origin: LedgerOrigin,
    pub zone: Option<Zone>,
    #[serde(with = "mandate_core::serde_util::map_as_pairs")]
    pub entries: BTreeMap<Subject, Entry>,
    /// Newest last; capped (`knowledge.leads_cap`), oldest dropped first.
    pub leads: Vec<Lead>,
    /// Places whose first-visit introduction has been given.
    pub visited: BTreeSet<PlaceId>,
    /// Lab only: every subject reads as `detailed` (baselines). Never set by the UI.
    #[serde(default)]
    pub lab_omniscient: bool,
}
```

It lives in the player domain: `PlayerState.knowledge: KnowledgeLedger` (`#[serde(default)]`). There is one ledger per player; bots in the lab are the player, so they have one too.

**API** (the only writers):

```rust
impl KnowledgeLedger {
    pub fn level(&self, s: &Subject) -> Level;
    /// Raise to `level` if higher (never lowers). Records `Learned` for each
    /// newly reached level, drops leads the new level satisfies. Returns
    /// whether anything changed.
    pub fn learn(&mut self, s: Subject, level: Level, on: SimDate, src: Source) -> bool;
    /// Store a dated reading; requires the subject at `known` or above and
    /// raises it to `detailed`.
    pub fn read(&mut self, s: Subject, r: Reading);
    /// Add a lead unless the subject is already at its level or the same
    /// (subject, level) lead exists; enforce the cap.
    pub fn lead(&mut self, l: Lead, cap: usize);
    pub fn enter_zone(&mut self, z: Zone, grants: &[Grant], on: SimDate, src: Source);
}

/// What the player knows of a subject; `Detailed` when there is no player
/// (lab runs) or the lab switch is on. Views and bots use this, never the ledger directly.
pub fn knows(state: &State, s: &Subject) -> Level;
```

### 1.3 How knowledge is produced

**Direct writes** (the player's own commands, Commands phase): the command handler calls `learn` after the command succeeds. This is the player domain writing its own state.

**Effect requests** (any other system, OVERVIEW §4.3). New `Effect` variants, applied by `player::effects` (Characters phase, daily; effects posted later in a day apply the next day):

```rust
Effect::Learn { subject: Subject, level: Level, source: Source },
Effect::Read { subject: Subject, reading: Reading },
Effect::Lead { lead: Lead },
```

| Trigger | What is learned | Writer |
|---|---|---|
| Arriving at a place (`MoveTo` or `Go` landing there), first time in this zone | The place's introduction: content `knowledge.visit_intro[<kind>]` (for example the council chamber: every councillor `known`; the union hall: its steward `known`, the broker `heard` if the broker is a union leader); the people who work there `heard`; a hidden place in that neighbourhood whose discovery rule holds `known` (DEC-090 D7) | Player command |
| Arriving at a place with `overheard` | Each line drawn this week teaches its subjects at `heard` (a line naming the broker: `broker heard`; about a neighbourhood's worry: `hood_concerns heard`; about a proposal: `proposal heard`; about the rival: `rival_plans heard`). Open proposals whose `heard` hook names this place kind are forced into the week's draw if not yet heard (§2.7) | Player command |
| Arriving at a place with `town_data_detailed` | A `read` of `hood_concerns` and `town_concerns` for every subject already at `known` (numbers need the identity first); public statistics are shown live and need no ledger | Player command |
| Arriving at a place with `internal_polls` | `policy_support detailed` (a reading) for every bill before the council and every standard template; `rival_plans known` | Player command |
| `Canvass { neighborhood }` | `hood_concerns known` (the second canvass reads it: `detailed`); one resident `known` | Player command |
| `TownHall { neighborhood, policy }` | `hood_concerns known`; with a bill, `policy_support known` | Player command |
| `Lobby` or `TradeFavor` with someone | `person known`; a second lobby of the same person, or one at the diner late at night, `detailed` | Player command |
| `Talk { to, topic, about }` (§3) | What the reply's lines teach, capped by the speaker's discretion; `person known` for the speaker | Player command (requests applied at once, §3.6) |
| `Meet { slot }` | `person known` | Player command |
| `Fundraise` (party office) | The party chair `known`; `rival_plans heard` | Player command |
| `Answer` a decision | Every cast person `known`; the scene's `teaches` hooks (content) | Player command |
| A story runs in an outlet covering the player's town | Everyone it names `heard` (the player reads the paper) | Media, `Effect::Learn` |
| A decision opens (it names its cast) | Its cast `heard` | Narrative, `Effect::Learn` |
| A narrative effect `learn` or `reveal_place` (dilemma options, storyline steps, scenes) | As stated in content | Narrative, `Effect::Learn` |
| A proposal opens | Its lead (`Effect::Lead`); `heard` for all if the family is `public` | Policy, `Effect::Learn` / `Effect::Lead` |
| A sponsor approaches the player (a request) | `proposal known`, `person known` | Narrative, `Effect::Learn` |

**Proposed:** arriving counts as an action for learning. DEC-091 says "you learn through actions, not movement, so the List view and quick mode learn the same things". Arrival is the `MoveTo`/`Go` command, which the map, the List view and quick mode's auto-routing all send; "movement" here means the client-side walk, which teaches nothing. Arrival teaches only introductions and `heard`; deeper levels need actions (§5, question 1).

**Narrative content gains** (in `mandate-content/src/narrative.rs`): `EffectDef::Learn { subject: SubjectDef, level }`, `EffectDef::RevealPlace { role }`, and `Condition::Knows { subject: SubjectDef, min: Level }` (this is the `knows_place` condition of TOWN_GENERATION §11, generalised). `SubjectDef` names subjects by role (`{ "type": "person", "role": "steward" }`, `{ "type": "broker" }`, `{ "type": "hood_concerns", "role": "hood" }`).

### 1.4 Starting knowledge, the option, and new zones

Content `content/base/knowledge.jsonc` (`kind: "knowledge"`, one file across packs) holds start profiles. A profile is a list of grants:

```jsonc
// Grant: { "subject": <kind>, "which": <selector>, "level": <level> }
// subject: town_concerns | hood_concerns | broker | council | people_at | rival_plans | policy_support | proposals
// which:   home | ward | town | <place kind> (for people_at)
"starts": [
  { "id": "start.grew_up_here", "grants": [
      { "subject": "town_concerns", "which": "town", "level": "known" },
      { "subject": "hood_concerns", "which": "home", "level": "known" },
      { "subject": "hood_concerns", "which": "town", "level": "heard" },
      { "subject": "council", "which": "town", "level": "heard" },
      { "subject": "policy_support", "which": "town", "level": "heard" } ] },
  { "id": "start.newcomer", "grants": [
      { "subject": "council", "which": "town", "level": "heard" } ] },
  // Today's start (a sitting councillor in a marginal ward) until the M6 draft of three.
  { "id": "start.sitting_councillor", "grants": [
      { "subject": "town_concerns", "which": "town", "level": "known" },
      { "subject": "hood_concerns", "which": "ward", "level": "known" },
      { "subject": "council", "which": "town", "level": "known" },
      { "subject": "policy_support", "which": "town", "level": "known" } ] },
],
"default_start": "start.sitting_councillor",
// "Start with local knowledge" (off by default): applied on top of the profile.
"local_option": [
  { "subject": "town_concerns", "which": "town", "level": "known" },
  { "subject": "hood_concerns", "which": "town", "level": "known" },
  { "subject": "broker", "which": "town", "level": "known" },
  { "subject": "council", "which": "town", "level": "known" },
  { "subject": "people_at", "which": "town", "level": "heard" },
  { "subject": "proposals", "which": "town", "level": "heard" } ],
// A party contact's primer on entering a new zone (the region, the capital).
"primer": [
  { "subject": "town_concerns", "which": "town", "level": "heard" },
  { "subject": "broker", "which": "town", "level": "heard" },
  { "subject": "council", "which": "town", "level": "heard" } ],
```

- The broker is never in a profile: even a native "doesn't know who really runs things" (DEC-091). Only the option gives it.
- The local option never gives `detailed` or hidden places: depth still comes from exploring.
- **Seeding:** `knowledge::seed(state, content, profile, local_option)` runs once when play begins (`Engine::mark_started`). The draft of three (DEC-036, M6) names a profile per background; until then `default_start`. `NewGameParams.local_knowledge: bool` (`#[serde(default)]`, false) carries the option. It is engine state (it changes outcomes), set only at new game, and shown in the run's metadata.
- **Zone change:** `enter_zone` sets the zone and applies the primer. Nothing is deleted: subjects are keyed by town, neighbourhood and person, so a new town's subjects simply start unknown, and the old town's entries stay (identities never fade). M6 has no zone change in play (Act 1 is local); it ships the API, a lab command and a test, and M7 (the region) first uses it.

### 1.5 Numbers and staleness

- Identities never fade. A number shown from the ledger is the reading as it was, with its date and source ("as of March, from the statistics desk").
- A reading older than `knowledge.fresh_days` (content; proposed 90) is flagged `stale` in views. Nothing is removed.
- At the source (the statistics desk, internal polls, the chamber's whip), figures are live, as DEC-079 already provides. Away from it, the dated reading, or the coarse public figure, whichever the view had before.

### 1.6 Ignorance costs effectiveness, never fairness

Content parameters in `knowledge.jsonc` (`effects` block), applied symmetrically by the shared formulas:

| Where | Rule | Proposed values |
|---|---|---|
| Lobbying odds (`npc::lobby_odds_between`, shared by the player and NPCs) | A logit term by the actor's knowledge of the target's `person` level | unknown −0.5, heard −0.3, known 0, detailed +0.25 |
| Canvassing | `Canvass` gains `issue: Option<ContentKey>` (a concern, `#[serde(default)]`). The approval gain is multiplied by `1 + match_bonus × (salience − 1)` from the neighbourhood's true salience of that issue, clamped | `match_bonus` 0.5, floor 0.6 |
| Town hall about a bill | Unchanged formula; the player sees the neighbourhood's support only as known | — |

- The odds shown before the choice include the term (DEC-073: the cost is visible).
- NPCs have no ledgers (§1.10): they use the `known` term.
- The truth decides outcomes. Knowledge changes what the player can see and aim at, plus these small, visible terms.

### 1.7 Protocol redaction

A helper in `mandate-protocol/src/knowledge.rs` (new): `Gate::of(engine)` with `level(&Subject) -> Level`. Every gated field below goes through it. Views never write. A view-only `KnowledgeLevelDto` (`"unknown" | "heard" | "known" | "detailed"`) and a `level` field travel with every gated value, so the UI can show a placeholder and the lead.

| View · field | Subject | `unknown` | `heard` | `known` | `detailed` |
|---|---|---|---|---|---|
| `PocketView.headlines.top_concern_keys` | `town_concerns` | `[]` | first only | two | two |
| `TownPlacesView.character.priorities` | `town_concerns` | `[]` | one, weight 0 | three, weight to 0.1 | exact, with `as_of` |
| `TownPlacesView.character.broker` | `broker` | `None` | `PersonRefView` with `rumoured: true` | full | full, plus `stake_key` (new) |
| `TownCharacterView.rival_plan` (new) | `rival_plans` | `None` | `rival-plan-something` | the seat | the seat and the focus |
| `TownPlacesView.neighborhoods[].spots` (hidden places) | `place` | omitted | omitted (lead in the notebook) | shown | shown |
| `TownHoodView.top_concern_key` (new, for the map's concern overlay) | `hood_concerns` | `None` | key | key | key, dated |
| `PlacePanelView.people[]` (`PersonRefView.name`) | `person` | `""` (the role key only: "a union steward") | name | name | name |
| `PlacePanelView.town_data.concerns` | `town_concerns` | `[]` | live levels for subjects at `heard` or above | live | live |
| `PlacePanelView.town_data.neighborhoods[].top_concern_key` | `hood_concerns` | `None` | key | key | key |
| `OfficeView.town.concerns` (`TownView`) | `town_concerns` | as the pocket | | | |
| `OfficeView.council[]` and `.mayor` (`MemberView.name`) | `person` | `""` plus role and ward | name | name | name |
| `MemberView.convictions` | `person` | `[]` | `[]` | two | two |
| `MemberView.pragmatism` (becomes `Option<f64>`) | `person` | `None` | `None` | `None` | value |
| `BillView.whip[].name`, `BillView.sponsor_name` | `person` | as `MemberView` | | | |
| `BillView.support`, `TemplateView.support`, `PolicyPreviewView.support` (`SupportView` gains `level` and `band`) | `policy_support` | `band` only (`popular`, `divisive`, `unpopular` from the overall figure), classes `None` | `band` only | classes to 0.1 | exact |
| `BillView.ward_support` | `policy_support` | to 0.25 | to 0.25 | to 0.1 | exact |
| `OfficeView.catalog` (situational templates, until F4) | `situational` | omitted (lead) | a stub: name, reason, lead | full | full |
| `CharacterView.name` | `person` | `""` | name | name | name |
| `CharacterView.personality`, `.traits`, `.expertise_key` | `person` | `[]`/`None` | `[]`/`None` | full | full |
| `CharacterView.place` | `person` | `None` | shown | shown | shown |
| `CharacterView.is_broker` | `broker` | false | false | true | true |
| `CharacterView.goals` (new: goal name keys, DEC-032) | `person` | `None` | `None` | `None` | list |
| `CharacterView.favours_with_others` (new: counts by counterpart) | `person` | `None` | `None` | `None` | list |

Unchanged on purpose: `opinion_band`, `trust_band`, `bonds`, `owes_you`, `you_owe`, `favours`, `memories` (all about the player); `whip_exact` at the chamber; odds and risks; decisions, alerts, stories, the legal view; campaigns, polls and results; the map's public layers (gated layers wait for DEC-089 in M7).

**Ids still travel.** A `PersonRefView` keeps its `id` when the name is hidden: commands need it, and an id carries no information the UI shows.

### 1.8 The notebook

```rust
QueryDto::Notebook  →  ResponseBody::Notebook(NotebookView)

pub struct NotebookView {
    pub zone: Option<String>,                // the town's name
    pub origin: String,                      // `profile` | `local_option` | `migrated`
    pub sections: Vec<NotebookSectionView>,  // people, neighbourhoods, power, proposals, places, rival, support
    pub leads: Vec<LeadView>,
}
pub struct NotebookSectionView { pub key: String, pub entries: Vec<NotebookEntryView> }
pub struct NotebookEntryView {
    pub subject: SubjectDto,                 // mirrors `Subject` with plain u32 ids
    pub level: String,
    pub title_key: String,                   // `notebook-<subject type>`
    pub args: BTreeMap<String, String>,
    pub history: Vec<LearnedView>,           // level, date, `learned-<source type>`, args
    pub as_of: Option<DateView>,
    pub stale: bool,
    pub new_this_turn: bool,
}
pub struct LeadView {
    pub key: String,
    pub args: BTreeMap<String, String>,
    pub level: String,                       // what following it gives
    pub to: Option<LocationView>,
    pub route: Option<RouteView>,            // auto-routing (DEC-079 rule 2)
    pub since: DateView,
}
```

- `PocketView` gains `notebook_new: u32` (entries learned this turn), for the badge.
- The notebook lists only what the player knows; it never shows a total that would reveal how much exists.

### 1.9 Save

- `PlayerState.knowledge` is new, so the save format bumps. The step `v<N> → v<N+1>` removes any `/state/player/knowledge` key (so the serde default, `Unseeded`, applies), as `v12_media` does. The lead assigns the version number at merge: town layout v2's baselines (E1) may take the next number first.
- `Engine::from_state` calls `player::knowledge::on_load`, which seeds an `Unseeded` ledger as `Migrated`: the default profile plus the local option, plus `person known` for everyone with a relationship edge to the player or a memory of them. A run already in progress never loses what its player plainly knew.
- The shape fingerprint changes. The state hash covers the ledger.
- Size: about 200–500 entries in a town (neighbourhoods, people, proposals), tens of kilobytes before compression. Leads are capped (proposed 24).

### 1.10 Bots, NPCs, and the lab gate

**Bots** (`mandate-cli bots`) are the player and learn through the same commands.
- Strategy code reads gated information only through `mandate_sim::player::knowledge::Knowing<'_>`: `concerns_of(hood)`, `broker()`, `goals_of(person)`, `support_of(policy)`, `proposals()`. Each returns `None` below the level the view would need.
- Example: `legislate` today ranks templates by `public_support` for the whole town, which is omniscient. It must use `support_of`.
- A **blind-invariance test** enforces the rule. Perturb truths the bot does not know (shuffle the goals of NPCs it hasn't met, swap two neighbourhoods' concern levels it hasn't learned), then check that its commands for the next N turns are identical.

**NPCs** have no ledgers. NPC politicians are locals, and their AI already uses "polls and public knowledge only" (`npc::inputs`). Symmetry (DEC-030) holds through the shared formulas: NPC lobbying uses the `known` term. **Proposed** (§5, question 9).

**New bot strategies:** `explorer` (`mixed`, but its first turns go on visiting every place, meeting the people there, and canvassing each ward neighbourhood once) and `explore_only` (only knowledge-giving actions, all run). New flags: `--local-knowledge` (the option on) and `--omniscient` (the lab switch, for baselines).

**Gates** (CI at 16 seeds; nightly at 64). The thresholds are proposed and get tuned in BALANCE_LOG:

| Gate | Rule |
|---|---|
| Exploring early pays | `explorer`'s mean first-election vote share ≥ `mixed`'s + 1.0 point, and its win share ≥ `mixed`'s |
| Exploring alone never wins | `explore_only`'s win share ≤ `idle`'s + 5 points, and below every other non-idle strategy |
| The option is a convenience | `mixed --local-knowledge` beats `mixed` by at most 3 points of vote share |
| Ignorance is not unfair | `bots --careers`: the share of brutal early ends with every ledger forced to `unknown` is within 2 points of the default |
| Variety still holds | The DEC-077 §5 variety gate passes with knowledge on |

### 1.11 Determinism

- The ledger draws no randomness. Every write is a function of commands and state.
- Knowledge terms change effect sizes, never the number of RNG draws, so a stream's position never depends on what the player knows.
- Overheard draws stay keyed on the dialogue stream (existing), with the forced proposal line chosen by rule, not by a draw.
- Views never write (test: query every view twice; the state hash is unchanged).
- Tests: same seed and commands give the same ledger; save, load and continue matches; native equals WASM.

---

## 2. Policy families (DEC-092)

### 2.1 Lifecycle

```text
run start ──► deck: which families this run can use in this town (keyed draw)
monthly ──► for each family in the deck: slots bind? trigger holds? room? ──► instance opens (status Open)
                 │                                                            │ lead to the player (knowledge hook)
                 ▼                                                            ▼
           not yet                                   player (Known) or NPC drafts it: choices + params (+ sponsor)
                                                                              │
                                                     an ordinary bill: council, law, implementation (DEC-024)
open instance, no bill: trigger gone for grace_months, max_months reached, or expiry.when ──► Lapsed (kept; cooldown)
```

- A drafted bill or a passed law stays when the situation eases (as for situational policies).
- Instances are stored, so bindings never change under the player. Ids are never reused.

### 2.2 The format, field by field (for the owner)

A family is one entry in `content/<pack>/policy_families/*.jsonc` (`"kind": "policy_families"`). Read the plant rescue in §2.16 alongside this table.

| Field | What it means | Example |
|---|---|---|
| `id` | The family's permanent name (never shown) | `family.plant_rescue` |
| `name_key`, `desc_key` | Fluent keys for its title and summary. Slot names are available as `{ $slot }` | "Rescue deal for { $plant }" |
| `tier`, `category`, `requires` | Which level of government, which budget line, and the power needed to propose it | `local`, `economic_development`, `propose_ordinance` |
| `needs` | Optional. A power or competence the town may lack (DEC-092 §5; M7) | — |
| `draw` | How often it turns up in a run: `rarity` (`always`, `common`, `uncommon`, `rare`) and `weight` (priority when several are ready at once) | `{ "rarity": "common", "weight": 3 }` |
| `max_instances` | How many can be open at once in a town (default 1) | `2` for flood walls |
| `slots` | The real things it is about. Each has an `id`, a `bind` (what kind of thing and how to choose it, §2.3), and `optional` | the plant, its owner, the union |
| `trigger` | When it comes onto the table (§2.4). Without one, it opens as soon as its slots bind | the plant is losing jobs |
| `expiry` | When an unused one goes away: `when` (a condition), `grace_months` after the trigger stops, `max_months`, `cooldown_months` before it can return | the plant has closed |
| `base` | What every version has: sliders (`params`), `cost`, `mechanisms`, `side_effects`, `signature`, `implementation_months`, `secondary_keys`, exactly as in today's policy templates | a wage subsidy for the plant's sector |
| `axes` | The design choices (two or three). Each axis has `id`, `name_key`, `default`, and `options` (§2.5) | terms; how it is paid for |
| `sponsors` | People who bring a ready version: which slot, their `preset` (an option per axis), their slider values, and their `price` (§2.6) | the union's version |
| `knowledge` | How the player finds out about it (§2.7): `heard`, `known`, `detailed` hooks, a `lead`, and `public` | overheard at the union hall |

Words starting with `@` refer to a slot: `"sector": "@sector"` means "the sector bound to the `sector` slot".

### 2.3 Slots: binding kinds and selectors

Slots bind in order, so a later slot can refer to an earlier one. A required slot that cannot bind means no instance. Candidates are taken in id order, ranked by `pick`, and ties go to the lowest id.

| `bind.type` | Binds | Selector fields | Fluent argument |
|---|---|---|---|
| `place` | A place in the town | `kinds` (place kinds); `drivers` (sector drivers of the place, via the new place-kind field `drivers`); `in_slot` (inside a bound neighbourhood); `open_only` (default true, needs DEC-090 D3 place status) | `$slot`: the place's name |
| `neighborhood` | A neighbourhood | `kinds` (neighbourhood kinds); `of_slot` (the neighbourhood of a bound place); `hazard` + `min_exposure` (needs D8); `concern` + `top` (its top concerns); `lacks` (place kinds it has none of) | `$slot`: its name |
| `person` | A Named or Principal character | `occupations`; `at_slot` (works at a bound place); `in_slot` (lives or works in a bound neighbourhood); `fallback`: `broker`, `mayor` or `none` | `$slot`: their name |
| `broker` | The town's power broker | — | `$slot` |
| `sector` | A sector | `of_slot` (the sector of a bound place, by its kind's drivers and the town's largest matching sector); `drivers`; `defining` (the defining industry) | `$slot_key`: the sector's name key |
| `crisis` | The live crisis | `kinds` | `$slot_key` |
| `hazard` | A hazard | `hazards`; `in_slot` (exposure of a bound neighbourhood, D8, else the town's `GeoProfile`); `min_exposure` | `$slot_key`: `hazard-text-<kind>` |

| `pick` | Chooses |
|---|---|
| `first` (default) | Lowest id: stable and plain |
| `most_at_risk` | Places: the largest job decline in their sector (E1 baselines) |
| `most_exposed` | Neighbourhoods and hazards: the highest exposure |
| `most_concerned` | Neighbourhoods: the highest salience of `concern` |
| `largest` | The most residents or jobs |
| `keyed` | A keyed draw among candidates (variety where nothing ranks them) |

**Organisations.** There are no organisations as actors yet (DEC-020, DEC-034 planned). "The union" binds its leader (`person` with `occupations: ["occupation.union_steward"]`) and, if needed, the union hall (`place`). An `organisation` binding kind is an extension point for when organisations exist.

### 2.4 Triggers and expiry, on the situation condition type

Families reuse `SituationTrigger` and `SituationCondition` from M5d part 2 (`mandate-content/src/types.rs`), extended. The extension is backward compatible: today's situational templates keep working.

```rust
pub struct SituationTrigger {
    /// Every one of these holds (new; default empty).
    #[serde(default)] pub all: Vec<SituationCondition>,
    /// At least one of these holds, when any are given (now `#[serde(default)]`).
    #[serde(default)] pub any: Vec<SituationCondition>,
    pub reason_key: String,
}

pub enum SituationCondition {
    // Existing (M5d part 2):
    Crisis { kind: ContentKey, min_severity: f64 },
    Indicator { indicator: Indicator, min: Option<f64>, max: Option<f64> },
    PlantClosure { within_months: u16 },
    Disaster { within_months: u16 },
    // New, town-level (mirroring the narrative `Condition` vocabulary):
    Concern { concern: ContentKey, top: u8 },
    Industry { drivers: Vec<SectorDriver> },
    LawInForce { templates: Vec<ContentKey>, absent: bool },   // templates or family ids
    // New, slot-relative (families only; rejected on templates):
    HoodConcern { slot: String, concern: ContentKey, top: u8 },
    HoodHazard { slot: String, hazard: HazardKey, min: f64 },  // needs D8
    SectorDecline { slot: String, min: f64 },                  // needs E1 baselines
    PlaceOpen { slot: String },                                // needs D3 place status
    PlaceClosed { slot: String },
}
```

- The evaluator becomes `policy::situation_holds(state, content, c, town, bindings: Option<&[SlotBinding]>)`. `triggered` is true when every `all` condition holds and either `any` is empty or one of its conditions holds.
- Validation needs at least one condition in total.

**Expiry** (`ExpiryDef`): an open instance with no bill lapses when the first of these happens:
- `when` holds;
- the trigger has not held for `grace_months` (default 3);
- `max_months` have passed since it opened.

It can open again in the town after `cooldown_months` (default 12). A lapsed instance is kept (status `Lapsed { on, reason }`), so ids and history stay.

### 2.5 Choice axes and options

Each axis has two or three options, and every draft picks exactly one option per axis. An option may carry:

| Field | Effect when chosen |
|---|---|
| `name_key`, `desc_key` | What it is and its trade-off in words |
| `area` | Where it applies: `{ "type": "town" }`, `{ "type": "slot", "slot": "hood" }`, or `{ "type": "slot_and_adjacent", "slot": "hood" }`. At most one axis per family sets `area` |
| `params` | Extra sliders |
| `fixed` | Hidden parameter values (`{ "evening": 1 }`) that its mechanisms use |
| `cost` | Added to the base cost |
| `cost_factor` | Multiplies the whole cost (a contractor's margin: 1.25) |
| `effect_factor` | Multiplies every mechanism's effect (a patchy job: 0.7) |
| `months` | Added to the implementation time (minimum 1 overall) |
| `mechanisms`, `side_effects` | Added to the base |
| `signature` | Added to the base signature, per attitude, clamped to −1..1 |
| `secondary_keys` | Added |
| `excludes` | Options it cannot be combined with (`"pay.levy"`) |
| `needs` | A tier or competence (M7, DEC-092 §5): in M6 the option shows as unavailable with the reason |

**Composition** (`policy::resolve`, pure): base plus the chosen options gives a `Resolved` policy, which derefs to `PolicyTemplate` (id and name key from the family, slot references replaced by bound keys, `signature_weights` computed) and also holds:
- the area (neighbourhood ids);
- the effect factor;
- the Fluent arguments;
- the hidden parameters (`ParamDef.hidden`, new, `#[serde(default)]`).

On drafting, the area becomes `Clause::Area { neighborhoods }` on the `PolicyInstance`. Per-capita costs use the area's residents. Area-aware mechanisms and side effects (`hazard_protection`, `housing_supply`, `disruption`, `grievance`) apply to those neighbourhoods through the per-neighbourhood modifiers of D8. Town-wide mechanisms (tax rates, sector subsidies) ignore the area. `Clause::Pilot` stays as it is.

**Sliders remain** where a quantity is the real choice (DEC-092 §3): wall height, stall numbers, aid level.

### 2.6 Sponsors and prices

```jsonc
{ "id": "union", "slot": "union", "name_key": "sponsor-union-version",
  "preset": { "terms": "job_guarantee", "pay": "bond" },   // one option per axis (missing: the axis default)
  "params": { "aid": 7 },
  "price": { "type": "endorsement", "classes": ["low"], "approval": 0.03, "backlash": 0.02 } }
```

| Price `type` | What the player pays | Mechanics (existing systems) |
|---|---|---|
| `favour` | `count` favours owed to the sponsor, with `context_key` (a "vote" is a favour they call in on a later bill) | The favour ledger (DEC-031); the sponsor calls it in like any creditor |
| `endorsement` | A public endorsement: approval rises among the sponsor's people (`classes`, or residents of `area`) and falls a little among everyone else | The narrative `Approval` effect machinery; a `stood_with_me` memory for the sponsor |

- **Adopting** a sponsor's version (drafting with `sponsor` and the preset unchanged) gives the sponsor `adopt_opinion` (proposed +8) and charges the price at drafting.
- **Amending** away from the preset costs `amend_opinion` (proposed −4) per changed axis, remembered as `pressed_me`. Slider changes within ±1 step are free.
- These values live in `policy_families/params.jsonc`.
- **Proposed:** no cash prices (they belong to the shady actions, DEC-034).

### 2.7 Knowledge hooks

```jsonc
"knowledge": {
  "public": false,                     // true: in the paper when it opens (heard by all)
  "heard":    [ { "type": "overheard", "at": ["union_hall", "diner"] }, { "type": "visit", "slot": "plant" } ],
  "known":    [ { "type": "meet", "slot": "union" }, { "type": "meet", "slot": "owner" } ],
  "detailed": [ { "type": "service", "service": "budget_papers" } ],
  "lead": { "key": "lead-plant-rescue", "to": { "type": "slot", "slot": "union" },
            "else": { "key": "lead-plant-rescue-hall", "to": { "type": "place_kind", "kind": "union_hall" } } }
}
```

| Hook `type` | Fires when the player… |
|---|---|
| `overheard { at }` | arrives at a place of one of those kinds in the town (a proposal line is forced into that week's talk there) |
| `visit { slot }` | arrives at the bound place, or the streets of the bound neighbourhood |
| `meet { slot }` | meets, lobbies or trades a favour with the bound person, or answers a decision they are cast in |
| `action { action, slot }` | does that action (`canvass`, `town_hall`) in or with the bound thing |
| `service { service }` | arrives where that service is offered |
| `talk { slot, topic }` | talks to the bound person on that topic (§3.6) |
| `approach { slot, after_months }` | (optional) the sponsor asks for a meeting: a request decision raised through the existing goals and requests path |

- `meet` hooks also fire on any conversation with the bound person.
- **Drafting needs `known`.** A `heard` proposal appears as a stub with its lead (§5, question 3).
- The lead is posted when the instance opens. `else` applies when the target slot is unbound.

### 2.8 Draw weights and the run deck

- **Deck.** When play begins in a town (and on entering a zone), each family whose tier matches is in the deck if a keyed draw falls under its rarity chance. The draw is `rng::keyed(seed, Stream::Policy, [TAG_DECK, town, hash(family id)])` with BLAKE3 `StableHash`. Rarity chances are content (`params.jsonc`, proposed: common 0.85, uncommon 0.5, rare 0.2, always 1). Because the draw is keyed, a save loaded later draws the same deck.
- **Opening.** Monthly, among deck families whose slots bind and trigger holds, instances open in order of `weight` (descending), then registry order. Two caps apply: `max_open_per_town` (proposed 4) and each family's `max_instances`.
- **Binding draws** (`pick: keyed`) use `[TAG_BIND, town, hash(id), slot index, instance ordinal]`.
- **Stream.** `Stream::Policy` is new and keyed-only, like `Stream::Media`.

### 2.9 Rust types (content: `crates/mandate-content/src/policy_family.rs`, new)

```rust
pub struct PolicyFamilyDef {
    pub id: ContentKey,
    pub name_key: String,
    pub desc_key: String,
    pub tier: TierKey,
    pub category: BudgetCategory,
    pub requires: Power,
    #[serde(default)] pub needs: Option<NeedsDef>,
    #[serde(default)] pub draw: DrawDef,
    #[serde(default = "one_u8")] pub max_instances: u8,
    pub slots: Vec<SlotDef>,
    #[serde(default)] pub trigger: Option<SituationTrigger>,
    #[serde(default)] pub expiry: ExpiryDef,
    pub base: FamilyBaseDef,
    pub axes: Vec<AxisDef>,
    #[serde(default)] pub sponsors: Vec<SponsorDef>,
    #[serde(default)] pub knowledge: FamilyKnowledgeDef,
}
pub struct DrawDef { pub rarity: Rarity, pub weight: f64 }            // default: common, 1
pub enum Rarity { Always, Common, Uncommon, Rare }
pub struct SlotDef { pub id: String, pub bind: BindDef, #[serde(default)] pub optional: bool }
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum BindDef {
    Place { kinds: Vec<String>, #[serde(default)] drivers: Vec<SectorDriver>,
            #[serde(default)] in_slot: Option<String>, #[serde(default = "yes")] open_only: bool,
            #[serde(default)] pick: Pick },
    Neighborhood { #[serde(default)] kinds: Vec<String>, #[serde(default)] of_slot: Option<String>,
                   #[serde(default)] hazard: Option<HazardKey>, #[serde(default)] min_exposure: f64,
                   #[serde(default)] concern: Option<ContentKey>, #[serde(default = "three")] top: u8,
                   #[serde(default)] lacks: Vec<String>, #[serde(default)] pick: Pick },
    Person { #[serde(default)] occupations: Vec<ContentKey>, #[serde(default)] at_slot: Option<String>,
             #[serde(default)] in_slot: Option<String>, #[serde(default)] fallback: PersonFallback },
    Broker,
    Sector { #[serde(default)] of_slot: Option<String>, #[serde(default)] drivers: Vec<SectorDriver>,
             #[serde(default)] defining: bool },
    Crisis { kinds: Vec<ContentKey> },
    Hazard { hazards: Vec<HazardKey>, #[serde(default)] in_slot: Option<String>,
             #[serde(default)] min_exposure: f64, #[serde(default)] pick: Pick },
}
pub enum Pick { First, MostAtRisk, MostExposed, MostConcerned, Largest, Keyed }
pub enum PersonFallback { None, Broker, Mayor }
pub struct ExpiryDef { pub when: Option<SituationTrigger>, pub grace_months: u16,
                       pub max_months: Option<u16>, pub cooldown_months: u16 }
pub struct FamilyBaseDef {
    #[serde(default)] pub params: Vec<ParamDef>,
    pub cost: CostDef,
    pub mechanisms: Vec<MechanismDef>,
    pub implementation_months: u16,
    pub signature: BTreeMap<String, f64>,
    pub secondary_keys: Vec<String>,
    #[serde(default)] pub side_effects: Vec<SideEffectDef>,
}
pub struct AxisDef { pub id: String, pub name_key: String,
                     #[serde(default)] pub default: Option<String>, pub options: Vec<FamilyOptionDef> }
pub struct FamilyOptionDef {
    pub id: String, pub name_key: String, pub desc_key: String,
    #[serde(default)] pub area: Option<AreaDef>,
    #[serde(default)] pub params: Vec<ParamDef>,
    #[serde(default)] pub fixed: BTreeMap<String, f64>,
    #[serde(default)] pub cost: Option<CostDef>,
    #[serde(default = "one_f64")] pub cost_factor: f64,
    #[serde(default = "one_f64")] pub effect_factor: f64,
    #[serde(default)] pub months: i16,
    #[serde(default)] pub mechanisms: Vec<MechanismDef>,
    #[serde(default)] pub side_effects: Vec<SideEffectDef>,
    #[serde(default)] pub signature: BTreeMap<String, f64>,
    #[serde(default)] pub secondary_keys: Vec<String>,
    #[serde(default)] pub excludes: Vec<String>,          // "axis.option"
    #[serde(default)] pub needs: Option<NeedsDef>,
}
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AreaDef { Town, Slot { slot: String }, SlotAndAdjacent { slot: String } }
pub struct NeedsDef { pub tier: TierKey, #[serde(default)] pub competence: Option<ContentKey> }
pub struct SponsorDef { pub id: String, pub slot: String, pub name_key: String,
                        pub preset: BTreeMap<String, String>, #[serde(default)] pub params: BTreeMap<String, f64>,
                        pub price: PriceDef }
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum PriceDef {
    Favour { count: u8, context_key: String },
    Endorsement { #[serde(default)] classes: Vec<ClassKey>, #[serde(default)] area: Option<AreaDef>,
                  approval: f64, backlash: f64 },
}
pub struct FamilyKnowledgeDef { pub public: bool, pub heard: Vec<HookDef>, pub known: Vec<HookDef>,
                                pub detailed: Vec<HookDef>, pub lead: Option<LeadDef> }
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum HookDef { Overheard { at: Vec<String> }, Visit { slot: String }, Meet { slot: String },
                   Action { action: String, #[serde(default)] slot: Option<String> }, Service { service: String },
                   Talk { slot: String, topic: ContentKey }, Approach { slot: String, after_months: u16 } }
pub struct LeadDef { pub key: String, pub to: LeadToDef, #[serde(default)] pub r#else: Option<Box<LeadDef>> }
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LeadToDef { Slot { slot: String }, PlaceKind { kind: String }, Streets { slot: String } }
```

**Changes to existing content types** (in F1):
- `MechanismDef::HazardProtection.hazard` becomes `HazardRef` (`#[serde(untagged)] enum HazardRef { Key(HazardKey), Slot(String) }`), so `"flood"` still parses and `"@hazard"` is a slot.
- `MechanismDef::SectorSubsidy.sector` already takes a string, so `"@sector"` parses; validation checks it.
- New mechanism variants are declared here and simulated in F2b: `Borrow { share }` (that share of the annual cost is added to debt instead of spent: finance), `Revenue { per_capita, per_unit, param }` (income offsetting cost: finance), `SectorCapacity { sector, param, per_unit }` (structural jobs: economy).
- `ParamDef.hidden: bool` (default false).
- `PolicyTemplate.retired: bool` (default false): not offered and not draftable by anyone, but bills and laws continue and saves load.
- `PlaceKindDef.drivers: Vec<SectorDriver>`, so the factory and the mill are `industrial` and the mine is `extraction`.
- `ContentFile::PolicyFamilies { items }`, `ContentFile::PolicyFamilyParams { value }`, `ContentFile::PolicyFamilyPatches { items }`. `ContentBundle.policy_families: Vec<PolicyFamilyDef>` is in load order.
- **Patches** (DEC-092: "mods can … patch them"; today a duplicate id is an error, EXTENDING "Packs"). A targeted patch format, applied in pack order: `{ "family": "family.x", "op": "set_draw" | "add_option" | "remove_option" | "add_sponsor" | "remove_sponsor" | "disable", … }`. Families are the first registry with patches (§5, question 10).

### 2.10 Instances in state and save

`crates/mandate-sim/src/policy/families.rs` (new), inside the policy domain:

```rust
define_id!(/// A policy-family instance (a proposal in a town).
           FamilyInstanceId);

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FamilyState {
    pub ids: IdAllocator,
    /// Families in play this run, per town, in registry order (keyed draw).
    #[serde(with = "map_as_pairs")] pub decks: BTreeMap<MunicipalityId, Vec<ContentKey>>,
    /// Indexed by `FamilyInstanceId`.
    pub instances: Vec<FamilyInstance>,
    /// When each (town, family) last lapsed: cooldowns.
    #[serde(with = "map_as_pairs")] pub lapsed: BTreeMap<(MunicipalityId, ContentKey), SimDate>,
    /// Registry keys the decks were drawn with (redrawn for new families on load).
    pub family_keys: Vec<ContentKey>,
}
pub struct FamilyInstance {
    pub id: FamilyInstanceId,
    pub family: ContentKey,
    pub town: MunicipalityId,
    pub bindings: Vec<SlotBinding>,          // slot order
    pub sponsors: Vec<SponsorCast>,          // sponsor id → the person bound to its slot
    pub opened: SimDate,
    pub trigger_held: SimDate,               // last month the trigger held
    pub status: FamilyStatus,                // Open | Lapsed { on, reason }
    pub drafts: Vec<PolicyInstanceId>,
}
pub struct SlotBinding { pub slot: String, pub bound: Bound }
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Bound { Place { place: PlaceId }, Neighborhood { hood: NeighborhoodId },
                 Person { person: CharacterId }, Sector { sector: ContentKey },
                 Crisis { kind: ContentKey }, Hazard { hazard: HazardKey }, Unbound }
pub struct SponsorCast { pub sponsor: String, pub person: CharacterId }
```

- `PolicyState` gains `#[serde(default)] families: FamilyState`.
- `PolicyInstance` gains `#[serde(default)] family: Option<FamilyDraft>`, where `FamilyDraft { instance: FamilyInstanceId, choices: Vec<(String, String)> /* axis order */, sponsor: Option<String>, changed_axes: u8 }`. Its `template` holds the family id.
- **One lookup seam.** Today 28 call sites read `content.policies.get(&p.template)` (sim, protocol, CLI). They all move to `policy::resolve(content, &state.policy, p) -> Option<Resolved<'_>>`, which returns the plain template for a standard bill and composes on the fly for a family bill. Composition is microseconds and runs only where templates are read (monthly modifiers, views, NPC agendas), so no cache. Add one if profiling asks.
- **System.** `policy.families` (monthly) is registered in the Characters phase right after `characters.monthly`, so it reads the fresh town character and crises. It writes only `policy.families` and posts `Effect::Learn`/`Effect::Lead`. In M6 it runs for the player's zone only; other towns keep the standard menu. This is an addition to the phase table (OVERVIEW §4.2) and needs a determinism test update.
- **NPCs.** An NPC councillor may draft an open instance in their town using the preset of the sponsor they are closest to (highest opinion), scored through the same agenda as templates (`npc.crisis_agenda`, `npc.concern_agenda`).
- **Save.** A format bump with a migration step: `policy.families` defaults to empty; `on_load` draws decks for the player's town (keyed, so the same deck a fresh start would draw). No instances exist before the next monthly pass.
- **Loading under changed rules** (DEC-074). `missing_references` adds:
  - families referenced by a bill or law;
  - options referenced by a bill's choices.

  Open instances of a removed family lapse.

### 2.11 Commands

```rust
PlayerCommand::DraftFamily {
    proposal: u32,
    choices: BTreeMap<String, String>,   // axis → option; missing → the axis default
    params: BTreeMap<String, f64>,
    sponsor: Option<String>,             // adopt this sponsor's version (choices must equal its preset)
}
PlayerCommand::AmendPolicy { policy, params, clauses,
    #[serde(default)] choices: Option<BTreeMap<String, String>> }  // family bills only
```

`DraftFamily` is located at the town hall (like `DraftPolicy`) and is free. It is refused when:
- the proposal is not `Open` or not in the player's town;
- `knows(proposal) < Known` (DEC-091);
- the office lacks `requires`;
- an option is unknown, excluded or unavailable (`needs`);
- a parameter is out of range.

### 2.12 The preview query, extended, and the views

```rust
QueryDto::PolicyPreview {
    template: String,                                  // a template id, or the family id with `proposal`
    params: Vec<ParamValue>,
    #[serde(default)] proposal: Option<u32>,
    #[serde(default)] choices: Vec<ChoiceValue>,       // { axis, option }
}
pub struct PolicyPreviewView {
    /* existing: template, params, est_annual_cost, budget_share, side_effects, support (gated), available */
    #[serde(default)] pub axes: Vec<AxisPreviewView>,
    #[serde(default)] pub sponsors: Vec<SponsorTermsView>,
    #[serde(default)] pub months: u32,
}
pub struct AxisPreviewView { pub id: String, pub name_key: String, pub chosen: String,
                             pub options: Vec<OptionPreviewView> }
pub struct OptionPreviewView {
    pub id: String, pub name_key: String, pub desc_key: String,
    /// The whole package with this option swapped in, others as chosen:
    pub est_annual_cost: f64, pub months: u32,
    pub support: SupportView,                          // gated like every support
    pub side_effects: Vec<SideEffectView>,
    pub available: bool, pub unavailable_key: Option<String>,
}
pub struct SponsorTermsView { pub id: String, pub name_key: String, pub person: PersonRefView,
                              pub preset: Vec<ChoiceValue>, pub params: Vec<ParamValue>,
                              pub price: PriceView, pub matches_current: bool }
```

- The engine computes every option's cost and support (DEC-092 §3). With three axes of three options that is at most nine compositions per query.
- `OfficeView` gains `proposals: Vec<ProposalView>`, gated by `proposal`. A `ProposalView` has:
  - id, family, name key and arguments, description key;
  - level;
  - the reason key;
  - bindings (slot, kind, name, `LocationView`);
  - the default cost;
  - sponsors (from `known`);
  - status;
  - the lead (when `heard`).
- `PlacePanelView` gains `proposals_here` (proposals whose bound place, or whose sponsor's workplace, is here).

### 2.13 Validation (`mandate-cli content validate`)

In `mandate-content` (`validate_bundle`):
1. Ids look like `family.<name>` and are unique. Slot, axis, option and sponsor ids are unique within the family. Parameter ids are unique across the base and all options.
2. Binding kinds and selectors name real things: place kinds in the `places` registry, occupations, sectors and drivers, crises, concerns, hazards. `of_slot`, `at_slot` and `in_slot` refer to earlier slots of the right kind. A required slot never depends on an optional one.
3. Every `@slot` reference names a slot of the matching kind (a sector slot for `sector`, a hazard slot for `hazard`).
4. Triggers: at least one condition; slot-relative conditions name a slot of the right kind; slot conditions on templates are rejected. Ranges are sane (as `check_situation`).
5. Axes: 1–3 per family, 2–3 options each; `default` names an option; at most one axis sets `area`; `excludes` name real options and leave every axis at least one compatible choice.
6. **DEC-073 per option:** every option has at least one cost and one benefit against the other options of its axis. Static check: no option is identical to or dominated by another on the vector (cost added, `cost_factor`, `effect_factor`, `months`, side-effect harm, area size); financing options count as a cost through their mechanism (tax, borrowing) or, for `budget`, through money. The whole family, like every template, has a simulated downside besides money.
7. Sponsors: `slot` is a person slot; the preset names one real option per axis it lists and respects `excludes`; parameter values are in range; prices are valid (`count` 1–3; approval and backlash in 0..0.1; `context_key` exists).
8. Knowledge hooks name real slots, place kinds, services and actions. A family that is not `public` has at least one `heard` hook. `known` is reachable without a hidden place (DEC-079: nothing critical only at a hidden place).
9. Fluent keys exist in English with the right arguments: every `{ $x }` in a family's keys is a slot id (or `<slot>_key` for sectors, crises and hazards). New check: Fluent placeholders parsed from the `.ftl` source.
10. Draw: `weight` in (0, 10].

In `mandate-cli` (it can generate worlds; the content crate cannot):

11. **Every family binds in at least one canonical test world** (the five scenarios at two sizes, with a 36-month run). A family that never binds is an error.

### 2.14 Lab commands

- `mandate-cli policies --seed <s> [--size small|medium|large] [--scenario <id>] [--town <id>] [--months <n>]` generates the world, plays `n` months (default 24) with NPC turns, and prints JSON:
  - the town's deck;
  - every instance (opened, lapsed, why), its bindings by name, and each sponsor;
  - the default package's cost, budget share and support;
  - each sponsor's version with its cost and price.
- `mandate-cli policies --coverage --seeds <n> [--months <n>]` reports per family:
  - drawn share;
  - share of runs with an instance;
  - instances per run;
  - median months open;
  - median default cost and support;
  - which slot or condition failed most;
  - families that never bind (a CI gate at 16 seeds).
- `--set` (DEC-078) reaches families and their parameters: `policy_families[family.flood_wall].draw.weight=2`, `policy_family_params.max_open_per_town=3`.
- The bot variety check (DEC-077 §5) counts which families winning strategies drafted.

### 2.15 What remains code (extension points, for EXTENDING.md)

| Extension | Where | What a new one needs |
|---|---|---|
| A binding kind (`organisation`, `bridge`, `school district`) | `BindDef`, `policy::families::bind` | A variant, a selector over the entity, an argument rule, validation, a test |
| A `pick` rule | `Pick`, `families::rank` | A variant and a ranking function |
| A situation condition | `SituationCondition`, `policy::situation_holds` | A variant, its evaluation, validation |
| A mechanism type | `MechanismDef`, `policy::mods_for` and the reading domain | A variant, a domain that reads it, a test (DEC-024) |
| A side-effect type | `SideEffectDef` | As today |
| A price type | `PriceDef`, `policy::families::pay` | A variant, its effect, validation |
| A knowledge hook | `HookDef`, `player::knowledge` producers | A variant and the command or system that fires it |
| A ledger subject | `Subject`, protocol `Gate` | A variant, its levels table, view redaction |

Everything else is content: new families, options, sponsors, prices of existing types, retuning, rarity and weights.

### 2.16 Worked example 1: a rescue deal for the plant

```jsonc
// content/base/policy_families/economy.jsonc
{
  "kind": "policy_families",
  "items": [
    {
      "id": "family.plant_rescue",
      "name_key": "family-plant-rescue",
      "desc_key": "family-plant-rescue-desc",
      "tier": "local", "category": "economic_development", "requires": "propose_ordinance",
      "draw": { "rarity": "common", "weight": 3 },

      // What it is about: the plant losing jobs, its sector, whoever owns or
      // runs it (else the power broker), and the union, if the town has one.
      "slots": [
        { "id": "plant",  "bind": { "type": "place", "kinds": ["factory", "mill", "mine"], "pick": "most_at_risk" } },
        { "id": "sector", "bind": { "type": "sector", "of_slot": "plant" } },
        { "id": "owner",  "bind": { "type": "person", "occupations": ["occupation.plant_manager"], "at_slot": "plant", "fallback": "broker" } },
        { "id": "union",  "bind": { "type": "person", "occupations": ["occupation.union_steward"] }, "optional": true },
      ],

      // On the table while the plant is open and its sector is shedding jobs.
      "trigger": {
        "all": [ { "type": "place_open", "slot": "plant" } ],
        "any": [
          { "type": "sector_decline", "slot": "sector", "min": 0.04 },
          { "type": "crisis", "kind": "crisis.plant_closure" },
        ],
        "reason_key": "reason-plant-at-risk",
      },
      // Too late once it closes (the closure response takes over).
      "expiry": {
        "when": { "any": [ { "type": "place_closed", "slot": "plant" } ], "reason_key": "reason-plant-closed" },
        "grace_months": 3, "max_months": 24, "cooldown_months": 24,
      },

      "base": {
        "params": [ { "id": "aid", "name_key": "param-wage-aid", "min": 2, "max": 12, "default": 5, "step": 1, "unit_key": "unit-percent-wages" } ],
        "cost": { "per_capita_per_unit": 8, "param": "aid" },
        "mechanisms": [ { "type": "sector_subsidy", "sector": "@sector", "param": "aid", "elasticity": 1.4 } ],
        "implementation_months": 3,
        "signature": { "localism": 0.3, "redistribution": 0.2, "tradition": 0.1 },
        "secondary_keys": ["secondary-rescue-precedent"],
        "side_effects": [ { "type": "grievance", "class": "high", "per_unit": 0.006, "param": "aid" } ],
      },

      "axes": [
        { "id": "terms", "name_key": "axis-rescue-terms", "default": "no_strings",
          "options": [
            // Fast and full strength; workers resent a blank cheque.
            { "id": "no_strings", "name_key": "opt-rescue-no-strings", "desc_key": "opt-rescue-no-strings-desc",
              "signature": { "tax_tolerance": -0.1 },
              "side_effects": [ { "type": "grievance", "class": "low", "per_unit": 0.006, "param": "aid" } ] },
            // More jobs kept per unit of aid; longer talks; owners' circle grumbles.
            { "id": "job_guarantee", "name_key": "opt-rescue-guarantee", "desc_key": "opt-rescue-guarantee-desc",
              "effect_factor": 1.2, "months": 2,
              "signature": { "redistribution": 0.2 },
              "side_effects": [ { "type": "grievance", "class": "high", "per_unit": 0.006, "param": "aid" } ] },
            // Money back if it closes anyway: cheaper, but the firm is less keen.
            { "id": "clawback", "name_key": "opt-rescue-clawback", "desc_key": "opt-rescue-clawback-desc",
              "cost_factor": 0.75, "effect_factor": 0.85,
              "signature": { "tax_tolerance": 0.1 } },
          ] },
        { "id": "pay", "name_key": "axis-paid-by", "default": "budget",
          "options": [
            { "id": "budget", "name_key": "opt-pay-budget", "desc_key": "opt-pay-budget-desc" },
            { "id": "bond", "name_key": "opt-pay-bond", "desc_key": "opt-pay-bond-desc",
              "mechanisms": [ { "type": "borrow", "share": 1.0 } ],
              "signature": { "tax_tolerance": 0.1 } },
            { "id": "levy", "name_key": "opt-pay-levy", "desc_key": "opt-pay-levy-desc",
              "params": [ { "id": "levy", "name_key": "param-levy", "min": 0.05, "max": 0.3, "default": 0.1, "step": 0.05, "unit_key": "unit-percent-points" } ],
              "mechanisms": [ { "type": "local_tax_rate", "param": "levy" } ],
              "side_effects": [ { "type": "grievance", "min_age_band": 1, "per_unit": 0.4, "param": "levy" } ] },
          ] },
      ],

      "sponsors": [
        { "id": "union", "slot": "union", "name_key": "sponsor-union-version",
          "preset": { "terms": "job_guarantee", "pay": "bond" }, "params": { "aid": 7 },
          "price": { "type": "endorsement", "classes": ["low"], "approval": 0.03, "backlash": 0.02 } },
        { "id": "owner", "slot": "owner", "name_key": "sponsor-owner-version",
          "preset": { "terms": "no_strings", "pay": "budget" }, "params": { "aid": 6 },
          "price": { "type": "favour", "count": 1, "context_key": "favour-rescue" } },
      ],

      "knowledge": {
        "heard":    [ { "type": "overheard", "at": ["union_hall", "diner", "market"] }, { "type": "visit", "slot": "plant" } ],
        "known":    [ { "type": "meet", "slot": "owner" }, { "type": "meet", "slot": "union" } ],
        "detailed": [ { "type": "service", "service": "budget_papers" } ],
        "lead": { "key": "lead-plant-rescue", "to": { "type": "slot", "slot": "union" },
                  "else": { "key": "lead-plant-rescue-hall", "to": { "type": "place_kind", "kind": "union_hall" } } },
      },
    },
  ],
}
```

```ftl
# content/base/locale/en/families.ftl
family-plant-rescue = Rescue deal for { $plant }
family-plant-rescue-desc = Help { $plant } keep its jobs in town with a wage subsidy, on terms the council sets.
param-wage-aid = Wage aid
secondary-rescue-precedent = Other firms will ask for the same.
axis-rescue-terms = Terms
opt-rescue-no-strings = No strings
opt-rescue-no-strings-desc = The money arrives quickly and does the most. Workers ask why the owners get a cheque for nothing.
opt-rescue-guarantee = A job guarantee
opt-rescue-guarantee-desc = The firm must keep its jobs for five years. More jobs stay, but the talks take two months longer and the firm's backers grumble.
opt-rescue-clawback = Repay if they leave
opt-rescue-clawback-desc = The money comes back if the plant closes anyway. Cheaper for taxpayers, but the firm is less keen.
sponsor-union-version = The union's version
sponsor-owner-version = The owner's version
favour-rescue = the rescue deal
reason-plant-at-risk = while { $plant } is losing jobs
reason-plant-closed = { $plant } has closed
lead-plant-rescue = { $union } at the union hall would know what a rescue for { $plant } would take.
lead-plant-rescue-hall = The union hall would know who is behind a rescue for { $plant }.

# Shared by many families
axis-paid-by = Paid for by
opt-pay-budget = This year's budget
opt-pay-budget-desc = No new tax or debt, but other programmes get less.
opt-pay-bond = A bond
opt-pay-bond-desc = Borrow now and repay over the years: nothing to pay today, interest for years.
opt-pay-levy = A local levy
opt-pay-levy-desc = A small rise in the property tax pays for it. Homeowners notice.
param-levy = Levy
```

A typical game: the player overhears at the diner that Halton Works may close, which makes the proposal `heard` and leaves the lead "Dana Price at the union hall would know…". Meeting Dana makes it `known` and offers the union's version (a job guarantee on a bond, aid 7%, for a public endorsement that low-income voters like and others resent). The owner's version is cheaper up front and costs a favour.

### 2.17 Worked example 2: a flood wall for a flood-prone neighbourhood

```jsonc
// content/base/policy_families/hazards.jsonc
{
  "kind": "policy_families",
  "items": [
    {
      "id": "family.flood_wall",
      "name_key": "family-flood-wall",
      "desc_key": "family-flood-wall-desc",
      "tier": "local", "category": "environment", "requires": "propose_ordinance",
      "draw": { "rarity": "common", "weight": 2 },
      "max_instances": 2,

      // The most exposed neighbourhood (D8), the hazard it faces, its
      // residents' organiser, and the builders' voice in town.
      "slots": [
        { "id": "hood",      "bind": { "type": "neighborhood", "hazard": "flood", "min_exposure": 0.35, "pick": "most_exposed" } },
        { "id": "hazard",    "bind": { "type": "hazard", "hazards": ["flood", "storm"], "in_slot": "hood", "pick": "most_exposed" } },
        { "id": "residents", "bind": { "type": "person", "occupations": ["occupation.organiser", "occupation.clergy"], "in_slot": "hood" }, "optional": true },
        { "id": "builders",  "bind": { "type": "person", "occupations": ["occupation.business_leader"] }, "optional": true },
      ],

      "trigger": {
        "any": [
          { "type": "disaster", "within_months": 36 },
          { "type": "hood_concern", "slot": "hood", "concern": "concern.recovery", "top": 3 },
          { "type": "hood_hazard", "slot": "hood", "hazard": "flood", "min": 0.6 },
        ],
        "reason_key": "reason-flood-risk",
      },
      "expiry": { "grace_months": 6, "cooldown_months": 12 },

      "base": {
        "params": [ { "id": "height", "name_key": "param-wall-height", "min": 1, "max": 3, "default": 2, "step": 1, "unit_key": "unit-level" } ],
        // Per resident of the area protected (the area clause, §2.5).
        "cost": { "per_capita_per_unit": 110, "param": "height" },
        "mechanisms": [ { "type": "hazard_protection", "hazard": "@hazard", "param": "height", "per_unit": 0.25 } ],
        "implementation_months": 18,
        "signature": { "environment": 0.2, "law_and_order": 0.2, "localism": 0.2, "tax_tolerance": 0.1 },
        "secondary_keys": ["secondary-wall-river-view"],
        "side_effects": [ { "type": "disruption", "per_unit": 0.04, "param": "height" } ],
      },

      "axes": [
        { "id": "where", "name_key": "axis-where", "default": "hood",
          "options": [
            { "id": "hood", "name_key": "opt-wall-hood", "desc_key": "opt-wall-hood-desc",
              "area": { "type": "slot", "slot": "hood" } },
            { "id": "riverfront", "name_key": "opt-wall-riverfront", "desc_key": "opt-wall-riverfront-desc",
              "area": { "type": "slot_and_adjacent", "slot": "hood" }, "months": 6,
              "secondary_keys": ["secondary-wall-long-works"] },
          ] },
        { "id": "pay", "name_key": "axis-paid-by", "default": "budget",
          "options": [
            { "id": "budget", "name_key": "opt-pay-budget", "desc_key": "opt-pay-budget-desc" },
            { "id": "bond", "name_key": "opt-pay-bond", "desc_key": "opt-pay-bond-desc",
              "mechanisms": [ { "type": "borrow", "share": 1.0 } ],
              "signature": { "tax_tolerance": 0.1 } },
            // Builders by the river pay: revenue offsets the cost, fewer homes get built.
            { "id": "developer_levy", "name_key": "opt-pay-developer-levy", "desc_key": "opt-pay-developer-levy-desc",
              "fixed": { "levy_rate": 1 },
              "mechanisms": [
                { "type": "revenue", "per_capita": 40 },
                { "type": "zoning", "param": "levy_rate", "per_unit": -0.08 },
              ],
              "signature": { "redistribution": 0.1, "reform_appetite": -0.1 } },
          ] },
        { "id": "builder", "name_key": "axis-who-builds", "default": "council",
          "options": [
            { "id": "council", "name_key": "opt-build-council", "desc_key": "opt-build-council-desc" },
            { "id": "contractor", "name_key": "opt-build-contractor", "desc_key": "opt-build-contractor-desc",
              "cost_factor": 1.25, "months": -6,
              "side_effects": [ { "type": "grievance", "class": "low", "per_unit": 0.01, "param": "height" } ] },
            { "id": "landowners", "name_key": "opt-build-landowners", "desc_key": "opt-build-landowners-desc",
              "cost_factor": 0.4, "effect_factor": 0.7, "months": 6,
              "side_effects": [ { "type": "grievance", "class": "high", "min_age_band": 2, "per_unit": 0.03, "param": "height" } ] },
          ] },
      ],

      "sponsors": [
        { "id": "residents", "slot": "residents", "name_key": "sponsor-residents-version",
          "preset": { "where": "riverfront", "pay": "bond", "builder": "council" }, "params": { "height": 3 },
          "price": { "type": "endorsement", "area": { "type": "slot", "slot": "hood" }, "approval": 0.04, "backlash": 0.01 } },
        { "id": "builders", "slot": "builders", "name_key": "sponsor-builders-version",
          "preset": { "where": "hood", "pay": "budget", "builder": "contractor" }, "params": { "height": 2 },
          "price": { "type": "favour", "count": 1, "context_key": "favour-contract" } },
      ],

      "knowledge": {
        "heard":    [ { "type": "overheard", "at": ["diner", "market"] }, { "type": "action", "action": "canvass", "slot": "hood" } ],
        "known":    [ { "type": "meet", "slot": "residents" }, { "type": "action", "action": "town_hall", "slot": "hood" } ],
        "detailed": [ { "type": "service", "service": "town_data_detailed" } ],
        "lead": { "key": "lead-flood-wall", "to": { "type": "streets", "slot": "hood" } },
      },
    },
  ],
}
```

```ftl
family-flood-wall = Flood wall for { $hood }
family-flood-wall-desc = Build a wall and drains to protect { $hood } against { $hazard_key }.
hazard-text-flood = flooding
hazard-text-storm = storm surges
param-wall-height = Wall height
secondary-wall-river-view = The wall cuts { $hood } off from the water.
secondary-wall-long-works = Years of works along the whole riverfront.
axis-where = Where
opt-wall-hood = { $hood } only
opt-wall-hood-desc = Cheaper and quicker. The streets next door stay exposed.
opt-wall-riverfront = The whole riverfront
opt-wall-riverfront-desc = Protects { $hood } and the neighbourhoods beside it. Costs more and takes six months longer.
opt-pay-developer-levy = A levy on developers
opt-pay-developer-levy-desc = Builders by the water pay towards it. Fewer new homes get built.
axis-who-builds = Who builds it
opt-build-council = The council's works team
opt-build-council-desc = No contractor's margin; as quick and as good as the council's own staff.
opt-build-contractor = A contractor
opt-build-contractor-desc = Six months faster and a quarter dearer. Local workers who lose out complain.
opt-build-landowners = The landowners, under a council order
opt-build-landowners-desc = Owners by the water build their own banks: cheap for the council, patchier protection, and angry owners.
sponsor-residents-version = The residents' version
sponsor-builders-version = The builders' version
favour-contract = the flood wall contract
reason-flood-risk = while { $hood } is at risk of { $hazard_key }
lead-flood-wall = People in the streets of { $hood } still talk about the last flood.
```

### 2.18 Worked example 3: market licences

```jsonc
// content/base/policy_families/town_life.jsonc
{
  "kind": "policy_families",
  "items": [
    {
      "id": "family.market_licences",
      "name_key": "family-market-licences",
      "desc_key": "family-market-licences-desc",
      "tier": "local", "category": "economic_development", "requires": "propose_ordinance",
      "draw": { "rarity": "uncommon", "weight": 1 },

      "slots": [
        { "id": "market",    "bind": { "type": "place", "kinds": ["market"] } },
        { "id": "hood",      "bind": { "type": "neighborhood", "of_slot": "market" } },
        { "id": "traders",   "bind": { "type": "person", "occupations": ["occupation.shopkeeper"], "at_slot": "market" } },
        { "id": "residents", "bind": { "type": "person", "occupations": ["occupation.organiser", "occupation.clergy"], "in_slot": "hood" }, "optional": true },
      ],

      "trigger": {
        "any": [
          { "type": "indicator", "indicator": "unemployment", "min": 0.05 },
          { "type": "hood_concern", "slot": "hood", "concern": "concern.jobs", "top": 2 },
          { "type": "concern", "concern": "concern.taxes", "top": 2 },
        ],
        "reason_key": "reason-market-dispute",
      },
      "expiry": { "grace_months": 4, "cooldown_months": 18 },

      // The quantity is the real choice here, so it stays a slider (DEC-092 §3).
      "base": {
        "params": [ { "id": "stalls", "name_key": "param-stalls", "min": -40, "max": 120, "default": 30, "step": 10, "unit_key": "unit-stalls" } ],
        "cost": { "per_unit": 150, "param": "stalls" },
        "mechanisms": [ { "type": "sector_capacity", "sector": "sector.retail", "param": "stalls", "per_unit": 1.5 } ],
        "implementation_months": 2,
        "signature": { "localism": 0.3, "tax_tolerance": -0.1, "tradition": 0.1 },
        "secondary_keys": ["secondary-market-vans"],
        "side_effects": [ { "type": "grievance", "min_age_band": 2, "per_unit": 0.0008, "param": "stalls" } ],
      },

      "axes": [
        { "id": "hours", "name_key": "axis-market-hours", "default": "daytime",
          "options": [
            { "id": "daytime", "name_key": "opt-market-daytime", "desc_key": "opt-market-daytime-desc" },
            { "id": "evening", "name_key": "opt-market-evening", "desc_key": "opt-market-evening-desc",
              "area": { "type": "slot", "slot": "hood" },
              "fixed": { "evening": 20 },
              "mechanisms": [ { "type": "sector_capacity", "sector": "sector.tourism", "param": "evening", "per_unit": 1.0 } ],
              "side_effects": [ { "type": "grievance", "min_age_band": 2, "per_unit": 0.002, "param": "evening" } ],
              "signature": { "social_liberalism": 0.2, "tradition": -0.2 } },
          ] },
        { "id": "fee", "name_key": "axis-market-fee", "default": "low",
          "options": [
            { "id": "low", "name_key": "opt-fee-low", "desc_key": "opt-fee-low-desc", "effect_factor": 1.2,
              "cost": { "per_capita": 1 } },
            { "id": "high", "name_key": "opt-fee-high", "desc_key": "opt-fee-high-desc", "effect_factor": 0.8,
              "mechanisms": [ { "type": "revenue", "per_unit": 400, "param": "stalls" } ],
              "side_effects": [ { "type": "grievance", "class": "middle", "per_unit": 0.0005, "param": "stalls" } ] },
          ] },
        { "id": "order", "name_key": "axis-market-order", "default": "council",
          "options": [
            { "id": "council", "name_key": "opt-order-council", "desc_key": "opt-order-council-desc",
              "cost": { "per_capita": 1.5 } },
            { "id": "traders", "name_key": "opt-order-traders", "desc_key": "opt-order-traders-desc",
              "cost_factor": 0.5,
              "side_effects": [ { "type": "grievance", "min_age_band": 2, "per_unit": 0.0006, "param": "stalls" } ],
              "signature": { "law_and_order": -0.1 } },
          ] },
      ],

      "sponsors": [
        { "id": "traders", "slot": "traders", "name_key": "sponsor-traders-version",
          "preset": { "hours": "evening", "fee": "low", "order": "traders" }, "params": { "stalls": 80 },
          "price": { "type": "favour", "count": 1, "context_key": "favour-market" } },
        { "id": "residents", "slot": "residents", "name_key": "sponsor-neighbours-version",
          "preset": { "hours": "daytime", "fee": "high", "order": "council" }, "params": { "stalls": -20 },
          "price": { "type": "endorsement", "area": { "type": "slot", "slot": "hood" }, "approval": 0.03, "backlash": 0.01 } },
      ],

      "knowledge": {
        "heard":    [ { "type": "visit", "slot": "market" }, { "type": "overheard", "at": ["diner"] } ],
        "known":    [ { "type": "meet", "slot": "traders" }, { "type": "meet", "slot": "residents" } ],
        "detailed": [ { "type": "service", "service": "budget_papers" } ],
        "lead": { "key": "lead-market-licences", "to": { "type": "slot", "slot": "traders" } },
      },
    },
  ],
}
```

The evening option's `area` makes its side effects fall on the market's neighbourhood.

```ftl
family-market-licences = Licences for { $market }
family-market-licences-desc = Decide how many stalls { $market } may have, when it opens, what traders pay, and who keeps order.
param-stalls = Change in licensed stalls
unit-stalls = stalls
secondary-market-vans = Busier market days fill the side streets with vans.
axis-market-hours = Opening hours
opt-market-daytime = Daytime only
opt-market-daytime-desc = The market keeps its hours. Nothing changes for the neighbours.
opt-market-evening = An evening market
opt-market-evening-desc = More trade and visitors, and noise late into the evening for { $hood }.
axis-market-fee = Licence fee
opt-fee-low = A low fee
opt-fee-low-desc = More traders take a stall. The council collects little.
opt-fee-high = A high fee
opt-fee-high-desc = The fees pay for the market's upkeep. Fewer traders can afford a stall.
axis-market-order = Who keeps order
opt-order-council = Council inspectors
opt-order-council-desc = Rules are kept evenly, at a cost to the council.
opt-order-traders = The traders themselves
opt-order-traders-desc = Cheap, and the traders like it. Neighbours doubt they will police themselves.
sponsor-traders-version = The traders' version
sponsor-neighbours-version = The neighbours' version
favour-market = the market licences
reason-market-dispute = while jobs are scarce and { $market } wants to grow
lead-market-licences = { $traders } at { $market } has views on the stall licences.
```

### 2.19 The standard menu: trimming the 11 templates

DEC-092 §1: the standard menu holds the general levers and stays always known.

| Template | Becomes |
|---|---|
| `policy.transit_frequency` | Standard: transit |
| `policy.affordable_housing` | Standard: housing |
| `policy.upzoning` | Standard: zoning |
| `policy.small_business_grants` | Standard: grants |
| `policy.police_staffing` | Standard: policing |
| `policy.parks` | Standard: parks |
| `policy.hardship_fund` | Standard: the hardship fund |
| `policy.property_tax` | Standard: property tax |
| `policy.flood_defenses` | `retired: true` → `family.flood_wall` (§2.17) |
| `policy.tech_incubator` | `retired: true` → `family.campus_incubator` (binds a university or tech campus and the knowledge sector; sponsors: the campus head, the business association) |
| `policy.local_wage_floor` | `retired: true` → `family.living_wage_deal` (binds the union steward and the business association chair; axes: the rate, who is covered, phase-in) |

- Retired templates keep loading, so laws already in force continue and saves are not refused by `missing_references`.
- `crises.jsonc` `policies` lists may name family ids. NPC agendas then favour open instances of those families. Dilemma references (`Cast::Law`, `Condition::LawInForce`, `EffectDef::Draft`) accept family ids too.

### 2.20 The M5d part 2 situational policies become families

| Situational template (`situational.jsonc`) | Family | Slots | Axes | Sponsors |
|---|---|---|---|---|
| `policy.closure_response` (unlock: plant-closure crisis, a closure within 12 months, job loss ≥ 3%) | `family.closure_response`: "After { $plant }: a fresh start" | the closed plant (`place_closed`), its sector, the union steward | support (retraining stipends, wage subsidies to rehire, both); paid by | union (stipends, bond; endorsement); business association (rehire subsidies, budget; favour) |
| `policy.rebuild_fund` (unlock: disaster-recovery crisis, a disaster within 18 months) | `family.rebuild`: "Rebuild { $hood }" | the hardest-hit neighbourhood (D8 per-neighbourhood damage), the hazard | standard (like for like, higher standard); where (the neighbourhood, the town); who builds | residents' organiser; builders |
| `policy.emergency_rent` (unlock: housing squeeze, rent burden ≥ 0.2) | `family.rent_relief`: "Rent relief for { $hood }" | the neighbourhood most concerned about housing (`most_concerned`), the business association chair | form (relief payments, a freeze, both); where (the neighbourhood, the town); paid by | a tenants' organiser; landlords via the business association |
| `policy.fiscal_bridge` (unlock: fiscal hole, budget stress ≥ 0.3) | `family.fiscal_bridge`: "Close the gap" | the crisis, the union steward, the business association chair | how (a levy, cuts in one budget line, borrowing); protect (the poorest, services) | union (levy with rebate); business association (cuts) |

- Their `unlock` triggers move unchanged into `trigger` (now with `all`/`any`).
- Their parameters become the base sliders, and their mechanisms are split across the options.
- The four templates are kept with `retired: true`, so v13 saves with those bills load.
- `Subject::Situational` entries in ledgers become inert. Nothing removes them.

### 2.21 The first pack (15–20 families)

| Family | Binds | Needs |
|---|---|---|
| Rescue deal for {plant} | plant, sector, owner, union | E1, D3 |
| After {plant}: a fresh start | closed plant, sector, union | D3 |
| Flood wall for {hood} | neighbourhood, hazard | D8 |
| Rebuild {hood} | hit neighbourhood, hazard | D8 |
| Rent relief for {hood} | squeezed neighbourhood | — |
| Close the gap | the fiscal crisis | — |
| Licences for {market} | market, neighbourhood | — |
| Living wage deal with {union} | union steward, business chair | — |
| Incubator at {campus} | university or tech campus, knowledge sector | — |
| Clean air at {plant} | a polluting plant (pollution indicator), its downwind neighbourhood | wind in G4 |
| Safety at {mine} | mine, union | G3 mine status |
| Water for the farms | a farm, drought | — |
| Port expansion at {port} | port, waterfront neighbourhood | — |
| A community centre for {hood} | a neighbourhood that lacks one | — |
| Clinic hours at {clinic} | clinic, an older neighbourhood | — |
| A park for {hood} | a dense neighbourhood that lacks a park | — |
| Hotel bed tax at {hotel} | hotel, tourism | — |
| Night buses for {hood} | an outlying neighbourhood | — |

"Reopen {bridge}" (DEC-092's example) cannot bind in M6: bridges are layout connectors drawn by the client, not engine entities (TOWN_GENERATION §10: infrastructure state is M7 or later).

---

### 2.22 Need-aware reactions (DEC-092 addendum, owner 2026-11-19)

Today a bill's anticipated reaction is pure value alignment (`policy::public_support`, `population` law reactions): the same in a town that needs it and one that does not. Accepted change:

- **Content:** each template and family option names the concerns it addresses: `"addresses": [{ "concern": "concern.housing", "weight": 1.0 }]` (validated against the concern registry; at least one entry for every non-administrative policy).
- **Reaction:** a cohort's reaction = alignment × (`need.base` + `need.weight` × the cohort's neighbourhood level of the addressed concerns, weighted). A **waste term** from tax-averse attitudes, scaled by annual cost ÷ (1 + need), applies when need is low ("why spend on this?"). The parameters go in `opinion.jsonc` under `need`, and `--set` reaches them.
- **Outcomes, with diminishing returns where they are real:**
  - Housing: public homes lower rents less as vacancy rises above target. Empty new units cost upkeep (a budget line) and add blight to the neighbourhood.
  - Job subsidies: capped by spare labour and by the sector's spare capacity.
  - Services: an effect that saturates (each unit adds less).
  - Hazard protection: worth nothing where the hazard is absent (DEC-090 D8 exposure).
- **Views:** the policy preview (M5d part 2) and the "Why?" breakdown show the need part ("rents here are low; few see the point").
- **Lab gate, policy responsiveness:** for each policy, `mandate-cli` runs it in high-need and low-need towns of the same seed set. Approval and outcome effects must differ by at least a set margin in the expected direction. It runs in CI with the bots.
- **Task:** F7 in §4, alongside F3 (needs `addresses` in the family format) and before the F-series balance pass.

## 3. Conversations (DEC-093)

### 3.1 What a conversation is

- **Talking happens by topic.** The player walks up to someone (DEC-088), or chooses them in the List view, and picks a topic. The engine composes a reply from content lines and applies what it teaches. There are no dialogue trees. The decision scenes of DEC-035 stay as authored beats; conversations are the everyday layer around them.
- **One conversation is one topic with one person**, sent as one command. Quick mode and the List view send the same command (through `Go` when the person is elsewhere), so they reach the same conversations.
- **Who can be talked to:** anyone at the player's location. That means politicians at the chamber, their office or the diner, Named characters at their workplace (`PlacePanelView.people`), and passers-by (an Ambient passer-by is promoted to Named, as `Meet` does). NPCs have no schedules yet (DEC-030 symmetry for location is deferred), so someone with no workplace is reached where they live: the streets of their home neighbourhood.
- **It costs time** (DEC-073). **Proposed:** half a slot, so two conversations fill one slot. Late at night it is allowed where lobbying late is allowed, and it uses the late slot. `Meet` ("say hello") stays free, as it is in the code today.

```rust
PlayerCommand::Talk {
    to: TalkTarget,                    // Person { person } | Ambient { slot }
    topic: ContentKey,                 // `topic.who_runs`
    #[serde(default)] about: Option<AboutRef>,   // the subject, for topics that take one
    #[serde(default)] late_night: bool,
}
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AboutRef {
    Place { place: PlaceId }, Person { person: CharacterId }, Hood { hood: NeighborhoodId },
    Bill { policy: PolicyInstanceId }, Template { template: ContentKey },
    Proposal { proposal: FamilyInstanceId }, Rival,
}
```

The command's result has `created` set to the new exchange's id; the UI fetches it with `QueryDto::Exchange` (§3.9).

### 3.2 Topics

Topics live in a content registry. The engine offers a topic by the speaker's role, the place, the relationship and what the player already knows. Each topic has a **mechanic**, a small closed set kept in code:

| Base topic | `about` | Mechanic | What happens besides the reply |
|---|---|---|---|
| `topic.worries` (what people are worried about) | none, or a neighbourhood | `inform` | — |
| `topic.who_runs` (who really runs things) | none | `inform` | — |
| `topic.about` (about a subject: the plant, a bill, the rival) | place, person, bill, proposal, rival | `inform` | — |
| `topic.pitch` (pitch a policy) | bill, template, proposal | `pitch` | Sway, with the shared lobbying formula and odds shown first; with a non-politician, it moves their view of the bill and can make them back it in public |
| `topic.support` (ask for support or an endorsement) | bill, or the player's candidacy | `ask_support` | Odds shown; success is a public endorsement (approval among their people, as the endorsement price of §2.6), and they may ask a favour in return |
| `topic.favour` (call in or offer a favour) | bill | `favour` | The existing favour ledger: offer one (as `TradeFavor`) or call one in (as `CalledIn`) |
| `topic.small_talk` | none | `small_talk` | A small opinion gain (capped weekly) and a `met` memory |

### 3.3 The shared condition type

DEC-093 asks for the same reusable condition type as DEC-092's triggers. `SituationCondition` (§2.4) becomes the one vocabulary for town situations, policy triggers and conversation lines.

- **Slots.** In a conversation the slots are `speaker`, `about` (the subject), `here` (the place) and `player`.
- **Arguments.** Every variant documents the Fluent arguments it provides when it holds, as `HeardCondition` already does (for example `broker` gives `$broker` and `$broker_place`). Triggers ignore the arguments; lines use them.
- **Today's variants** (§2.4) keep working.

New variants, mostly the person-relative ones of the narrative `Condition` with `slot` in place of `role`:

```rust
// Added to SituationCondition (slot-relative; conversations and families):
Occupation { slot: String, occupations: Vec<ContentKey> },       // or "councillor" / "mayor"
Trait { slot: String, traits: Vec<ContentKey> },
Personality { slot: String, axis: String, min: Option<f64>, max: Option<f64> },
Goal { slot: String, goal: ContentKey },
Opinion { slot: String, min: Option<f64>, max: Option<f64> },     // of the player, −100..100
Trust { slot: String, min: Option<f64>, max: Option<f64> },
Remembers { slot: String, kinds: Vec<String> },                   // gives $memory_key, $subject_key, $year
Favour { slot: String, owed_to_player: bool },                    // gives $n
IsBroker { slot: String, #[serde(default = "yes")] is: bool },    // gives $broker, $broker_place, $stake_key (the sector's talk key, "the factories")
IsRival { slot: String, #[serde(default = "yes")] is: bool },
/// The speaker would tell the player at least `min` about `subject` (§3.4).
Tells { subject: SubjectRef, min: Level },
/// The player's ledger (DEC-091): what they already know.
PlayerKnows { subject: SubjectRef, min: Option<Level>, max: Option<Level> },
Any { of: Vec<SituationCondition> },
```

`SubjectRef` names a ledger subject from the context:
- `{ "type": "broker" }`;
- `{ "type": "hood_concerns", "of": "here" | "speaker_home" | "about" }`;
- `{ "type": "person", "slot": "about" }`;
- `{ "type": "rival_plans" }`;
- `{ "type": "proposal", "sponsored_by": "speaker" }`;
- `{ "type": "policy_support", "of": "about" }`.

### 3.4 What speakers know, and what they will tell

**NPCs have no ledgers** (§1.10). What a speaker knows is derived from rules in content (`talk_params.speaker_knows`). It is not stored, so it adds nothing to the save:

| `who` | Knows |
|---|---|
| `anyone_local` | The town's worries (`known`), their own neighbourhood's (`detailed`), the council (`known`) |
| `leader` (an occupation with `leader: true`) | The broker (`known`) |
| `stake_overlap` (their stake shares the broker's industry or crisis) | The broker (`detailed`) |
| `broker` | Themselves (`detailed`) |
| `politician` | The rival's plans (`known`), support for bills before the council (`known`) |
| `sponsor` | Proposals they sponsor (`detailed`) |
| `workplace` | Their workplace's proposals and the people there (`known`) |

**Discretion** caps what they tell (`talk_params.discretion`), by the speaker's opinion and trust bands toward the player (the bands of the protocol, `opinion_band` and `trust_band`):

| Opinion band | Tells at most |
|---|---|
| hostile, cool | `unknown` (they deflect) |
| neutral | `heard` |
| warm | `known` |
| friendly, with trust at least `trust_for_detailed` (0.5) | `detailed` |

- Traits shift the cap by whole levels: `trait.schemer` −1, `trait.cautious` −1, `trait.vain` +1 when flattered (after `topic.small_talk` this week).
- A favour owed to the player raises the cap by one for the topic it is called in on.
- `Tells { subject, min }` holds when min(what the speaker knows, the cap) ≥ `min`.

So a cool steward who knows everything still deflects, and a friendly one who knows little can only pass on a rumour.

### 3.5 Replies: lines, voices, composition

A reply has up to four parts, in this order:

| Part | Depends on | Chosen from |
|---|---|---|
| `opener` | The relationship | Lines with `part: "opener"` whose conditions hold (topic-specific ones first) |
| `quote` | Their memories of the player (DEC-031) | `quote` lines; drawn with probability `quote_share` when they hold |
| `substance` | The facts | Lines for the topic whose conditions hold; else the topic's fallback line for the speaker's role |
| `closer` | The speaker's voice | Lines of the voice; drawn with probability `closer_share` of the voice |

**Voices** come from traits and personality. The speaker's voice is the highest-`priority` voice whose conditions all hold. `voice.plain` (no conditions) is the fallback.

**Selection** (`mandate_sim::conversations::reply`, pure):
1. Build the context (speaker, player, topic, `about`, here) and compute the told levels (§3.4).
2. For each part, gather the lines whose `when` all hold.
3. **Repetition guard:** drop lines in the speaker's used-lines memory while any other candidate remains.
4. Pick by `weight` with `rng::keyed(seed, Stream::Dialogue, [TAG_TALK, speaker, hash(topic), week, part, n])`. `week` is the day number divided by 7. `n` counts this (speaker, topic) pair's earlier conversations this week, so asking again gives another line.
5. Take the arguments the holding conditions provide, plus `$speaker`, `$player`, `$town` and `$here`.
6. Collect the lines' requests (§3.6), capping every `teaches` at the told level.

The keyed draw never advances a sequential stream, and views never draw.

**Composition gives the variety.** For example: 3 openers × 6 substances × 4 voices × 3 closers × quote or none, for one topic and one role. A few hundred lines give thousands of distinct, grounded exchanges (DEC-093).

### 3.6 What a reply does

The resolver returns typed requests; the `Talk` handler applies each through the owning domain at once (the command phase), so the reply and its effects appear together. These are the effect requests of OVERVIEW §4.3, applied immediately rather than queued:

| Request | Applied by | Notes |
|---|---|---|
| `Effect::Learn { subject, level, source: Source::Talk { person, topic } }` | player (ledger) | Capped at the told level; the notebook shows "from Dana Price at the union hall" |
| `Effect::Lead { lead }` | player | A deflecting line usually points somewhere ("ask at the diner") |
| `Effect::Learn { subject: Proposal, level: Known }` | player | This is the DEC-092 sponsor meeting (family hook `talk`, §2.7) |
| `Effect::Remember { holder: speaker, actor: player, kind }` | characters | `met` for small talk; mechanics add their own (`lobbied_me`, `pressed_me`) |
| Opinion change (`effects: [{ "type": "opinion", "delta": … }]`) | characters | Scaled by personality, as every opinion change is |
| Mechanic outcomes (sway, endorsement, favour) | politics, population, characters | Through the existing lobbying, approval and favour paths |

- `Source` (§1.2) gains `Talk { person: CharacterId, topic: ContentKey }`.
- The policy-family hook list (§2.7) gains `{ "type": "talk", "slot": "union", "topic": "topic.about" }`: talking to the bound person on that topic fires it, and `meet` hooks also fire on any conversation with that person.

### 3.7 Rust types

Content (`crates/mandate-content/src/conversation.rs`, new):

```rust
pub struct TopicDef {
    pub id: ContentKey,                       // topic.who_runs
    pub name_key: String,                     // the button label
    pub mechanic: TalkMechanic,
    #[serde(default)] pub about: Vec<AboutKind>,   // empty: takes no subject
    #[serde(default)] pub offered: TopicOffer,
}
pub enum TalkMechanic { Inform, Pitch, AskSupport, Favour, SmallTalk }
pub enum AboutKind { Place, Person, Hood, Bill, Template, Proposal, Rival }
pub struct TopicOffer {
    #[serde(default)] pub roles: Vec<String>,              // occupation ids, `councillor`, `mayor`; empty = anyone
    #[serde(default)] pub places: Vec<String>,             // place kinds; empty = anywhere
    #[serde(default)] pub when: Vec<SituationCondition>,
}
pub struct TalkLineDef {
    pub id: ContentKey,                       // line.runs.named
    pub part: TalkPart,                       // opener | quote | substance | closer
    #[serde(default)] pub topic: Option<ContentKey>,   // required for substance
    #[serde(default)] pub voice: Option<ContentKey>,   // required for closers
    pub key: String,
    #[serde(default)] pub when: Vec<SituationCondition>,
    #[serde(default = "one_f64")] pub weight: f64,
    /// The topic's fallback for the roles in its `occupation` condition (or all roles).
    #[serde(default)] pub fallback: bool,
    #[serde(default)] pub teaches: Vec<TeachDef>,      // { subject: SubjectRef, level }
    #[serde(default)] pub lead: Option<TalkLeadDef>,   // { key, to: LeadToDef, subject, level }
    #[serde(default)] pub effects: Vec<TalkEffectDef>, // opinion { delta }, remember { kind }
}
pub enum TalkPart { Opener, Quote, Substance, Closer }
pub struct VoiceDef { pub id: ContentKey, pub priority: i32,
                      #[serde(default)] pub when: Vec<SituationCondition>,
                      #[serde(default = "half")] pub closer_share: f64 }
pub struct TalkParams { pub cost: TalkCost /* half_slot | slot */, pub discretion: DiscretionDef,
                        pub trait_discretion: BTreeMap<ContentKey, i8>, pub speaker_knows: Vec<KnowsDef>,
                        pub memory_lines: u8, pub quote_share: f64,
                        pub small_talk_opinion: f64, pub small_talk_weekly_cap: u8 }
```

The new `ContentFile` kinds are `talk_topics`, `talk_lines`, `talk_voices` and `talk_params`, under `content/<pack>/conversations/*.jsonc`. Registries are in load order.

State (the player domain, `crates/mandate-sim/src/player/talk.rs`, new):

```rust
define_id!(/// One conversation (an exchange) the player had.
           ExchangeId);

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TalkState {
    pub ids: IdAllocator,
    /// The repetition guard: lines each person has used with the player, newest last,
    /// at most `memory_lines` per person.
    #[serde(with = "map_as_pairs")] pub used: BTreeMap<CharacterId, Vec<UsedLine>>,
    /// The same guard for overheard talk, per place.
    #[serde(with = "map_as_pairs")] pub heard_at: BTreeMap<PlaceId, Vec<UsedLine>>,
    /// Recent exchanges (capped, proposed 40): the notebook's "where and when".
    pub log: Vec<Exchange>,
    /// Half a slot already spent on a conversation this turn.
    pub half_slot: bool,
}
pub struct UsedLine { pub line: ContentKey, pub on: SimDate }
pub struct Exchange {
    pub id: ExchangeId, pub on: SimDate, pub speaker: CharacterId, pub place: Option<PlaceId>,
    pub topic: ContentKey, pub about: Option<AboutRef>,
    /// Each part's line and the arguments it was spoken with (names and numbers
    /// captured then, so the exchange reads the same later; facts, not prose).
    pub parts: Vec<SpokenLine>,
    pub learned: Vec<(Subject, Level)>,
    pub outcome: Option<String>,             // a mechanic's outcome key
}
pub struct SpokenLine { pub part: TalkPart, pub line: ContentKey, pub key: String, pub args: Vec<(String, String)> }
```

`PlayerState` gains `#[serde(default)] talk: TalkState`.

**Overheard repetition (DEC-093: chatter follows the same rule).** Arriving at an overheard place records that week's lines in `heard_at`. The overheard draw (command and view alike) excludes lines recorded before the current week while alternatives exist. View and command therefore agree, and views still never write. This replaces today's stateless "avoid last week's draw".

### 3.8 Content format, field by field (for the owner)

| File (kind) | Field | Meaning |
|---|---|---|
| `topics` (`talk_topics`) | `id`, `name_key` | The topic and its button text |
| | `mechanic` | What it does besides talk: `inform`, `pitch`, `ask_support`, `favour`, `small_talk` |
| | `about` | Which subjects it can be about (empty: none) |
| | `offered.roles`, `.places`, `.when` | Who offers it, where, and under what conditions (for example "who runs things" only while the player doesn't yet know the broker in detail) |
| `lines` (`talk_lines`) | `id`, `part`, `key` | The line's name, which part of a reply it is, and its Fluent key |
| | `topic` | For substance lines: the topic it answers |
| | `voice` | For closers: the voice it belongs to |
| | `when` | Conditions that must all hold (the shared condition type, §3.3) |
| | `weight` | How often it is picked among those that hold (default 1) |
| | `fallback` | The safe answer for this topic for the roles named in its conditions; every topic needs one per role |
| | `teaches`, `lead`, `effects` | What the player learns (capped by discretion), where it points them, and any change in the relationship |
| `voices` (`talk_voices`) | `id`, `priority`, `when`, `closer_share` | A way of speaking, who has it, which voice wins, and how often a reply ends with a closer |
| `params` (`talk_params`) | `cost`, `discretion`, `trait_discretion`, `speaker_knows`, `memory_lines`, `quote_share`, small-talk caps | The rules of §3.4–3.6 as numbers |

### 3.9 Protocol views

```rust
QueryDto::Talk { person: u32 }  →  ResponseBody::Talk(TalkView)
pub struct TalkView {
    pub person: PersonRefView,               // gated name (§1.7)
    pub here: bool,
    pub route: Option<RouteView>,            // where they are, for auto-routing
    pub cost_halves: u32,                    // 1 = half a slot
    pub late_night: bool,
    pub topics: Vec<TopicView>,
}
pub struct TopicView {
    pub id: String, pub name_key: String, pub mechanic: String,
    /// Subjects the player can ask about (only things they know of: §1.7 gating).
    pub about: Vec<AboutChoiceView>,         // { about: AboutRefDto, label_key, args }
    pub odds: Option<f64>,                   // pitch and ask_support (DEC-073)
    pub asked_this_week: bool,
    pub available: bool, pub unavailable_key: Option<String>,
}
QueryDto::Exchange { id: u32 }  →  ResponseBody::Exchange(ExchangeView)
pub struct ExchangeView {
    pub id: u32, pub date: DateView, pub speaker: PersonRefView, pub topic_key: String,
    pub parts: Vec<SpokenLineView>,          // { part, key, args } in order
    pub learned: Vec<LearnedItemView>,       // { title_key, args, level }
    pub leads: Vec<LeadView>,
    pub relationship: Option<String>,        // `warmer` | `cooler`
    pub outcome_key: Option<String>, pub outcome_args: BTreeMap<String, String>,
}
```

- `PersonRefView` gains `can_talk: bool`.
- The notebook's `LearnedView` for `Source::Talk` names the speaker and links the exchange.
- Topics are buttons and replies are text (DEC-051). The walkable town shows the same exchange in a speech panel.

### 3.10 Save

- `PlayerState.talk` is new, so the format bumps. If C1 merges alongside K1, the two share one bump. The migration step removes any `talk` key, so it starts empty.
- Size: at most `memory_lines` (12) lines per person talked to (a few dozen people in a run's town), places' overheard lines, and 40 exchanges: tens of kilobytes before compression.
- The state hash covers it.

### 3.11 Validation

1. Ids are unique (`topic.*`, `line.*`, `voice.*`). Every substance line names a real topic; every closer names a real voice; every quote line has a `remembers` condition.
2. **Every topic has a fallback substance line for every role it is offered to** (occupations, `councillor`, `mayor`). A fallback line's only conditions are on the speaker's role.
3. Conditions reference real things (slots valid for the part and topic, real occupations, traits, goals, memory kinds, personality axes, concerns) and sane ranges.
4. `teaches` subjects are valid for the topic's `about`. A line that teaches `level` must carry a `Tells` condition at that level or higher, so discretion is never bypassed.
5. Fluent keys exist, and their placeholders are among the arguments the line's conditions and context provide (the placeholder check of §2.13 item 9).
6. At least one voice with no conditions exists. Every voice with `closer_share > 0` has at least two closers (so the guard has an alternative).
7. Weights are in (0, 10]; `quote_share` and `closer_share` in [0, 1].

### 3.12 Lab

`mandate-cli talk --seed <s> [--months <n>] [--occupation <id> | --person <id>] [--topic <id>] [--samples <n>]` plays `n` months, then samples conversations in the player's town (each topic with each reachable person, at the person's real relationship and at forced bands: hostile, neutral, friendly). It reports:
- distinct exchanges;
- lines never chosen;
- the share of replies that fell back, per topic and role;
- the mean told level per band;
- **repetition violations** (a line repeated to the same person while an alternative held), which must be 0. This is a CI gate.

`--coverage --seeds <n>` aggregates over worlds. Bots (K4) use `TalkView` and `Talk` like the UI. The `explorer` strategy spends its early conversations on `topic.worries` and `topic.who_runs`.

### 3.13 Worked example: "Who really runs things?" asked of the union steward

The steward, Dana Price, has boldness 0.7 and warmth 0.35 (so `voice.blunt`), knows the broker (`leader`), and has a stake that overlaps the broker's (so `detailed`). The broker is Ruth Okafor, chair of the business association, with a stake in manufacturing.

```jsonc
// content/base/conversations/topics.jsonc
{ "kind": "talk_topics", "items": [
  { "id": "topic.who_runs", "name_key": "talk-topic-who-runs", "mechanic": "inform",
    // Offered until the player knows the broker in detail.
    "offered": { "when": [ { "type": "player_knows", "subject": { "type": "broker" }, "max": "known" } ] } },
] }

// content/base/conversations/voices.jsonc
{ "kind": "talk_voices", "items": [
  { "id": "voice.smooth",  "priority": 3, "closer_share": 0.7,
    "when": [ { "type": "trait", "slot": "speaker", "traits": ["trait.charmer", "trait.schemer"] } ] },
  { "id": "voice.guarded", "priority": 2, "closer_share": 0.6,
    "when": [ { "type": "trait", "slot": "speaker", "traits": ["trait.cautious"] } ] },
  { "id": "voice.blunt",   "priority": 1, "closer_share": 0.6,
    "when": [ { "type": "personality", "slot": "speaker", "axis": "boldness", "min": 0.6 },
              { "type": "personality", "slot": "speaker", "axis": "warmth", "max": 0.45 } ] },
  { "id": "voice.plain",   "priority": 0, "closer_share": 0.3 },
] }

// content/base/conversations/lines.jsonc
{ "kind": "talk_lines", "items": [
  // Openers, by relationship (any topic).
  { "id": "line.open.cool",    "part": "opener", "key": "talk-open-cool",
    "when": [ { "type": "opinion", "slot": "speaker", "max": -15 } ] },
  { "id": "line.open.neutral", "part": "opener", "key": "talk-open-neutral",
    "when": [ { "type": "opinion", "slot": "speaker", "min": -15, "max": 50 } ] },
  { "id": "line.open.warm",    "part": "opener", "key": "talk-open-warm",
    "when": [ { "type": "opinion", "slot": "speaker", "min": 50 } ] },

  // Substance: who runs things. The fallback deflects and points to the diner.
  { "id": "line.runs.deflect", "part": "substance", "topic": "topic.who_runs", "key": "talk-runs-deflect",
    "fallback": true,
    "lead": { "key": "lead-ask-diner", "to": { "type": "place_kind", "kind": "diner" },
              "subject": { "type": "broker" }, "level": "heard" } },
  { "id": "line.runs.rumour", "part": "substance", "topic": "topic.who_runs", "key": "talk-runs-rumour",
    "when": [ { "type": "tells", "subject": { "type": "broker" }, "min": "heard" },
              { "type": "is_broker", "slot": "speaker", "is": false } ],
    "teaches": [ { "subject": { "type": "broker" }, "level": "heard" } ] },
  { "id": "line.runs.named", "part": "substance", "topic": "topic.who_runs", "key": "talk-runs-named",
    "when": [ { "type": "tells", "subject": { "type": "broker" }, "min": "known" },
              { "type": "is_broker", "slot": "speaker", "is": false } ],
    "teaches": [ { "subject": { "type": "broker" }, "level": "known" } ] },
  { "id": "line.runs.stake", "part": "substance", "topic": "topic.who_runs", "key": "talk-runs-stake", "weight": 2,
    "when": [ { "type": "tells", "subject": { "type": "broker" }, "min": "detailed" },
              { "type": "is_broker", "slot": "speaker", "is": false } ],
    "teaches": [ { "subject": { "type": "broker" }, "level": "detailed" } ],
    "effects": [ { "type": "opinion", "delta": 1 } ] },
  { "id": "line.runs.self", "part": "substance", "topic": "topic.who_runs", "key": "talk-runs-self", "weight": 5,
    "when": [ { "type": "is_broker", "slot": "speaker" },
              { "type": "tells", "subject": { "type": "broker" }, "min": "known" } ],
    "teaches": [ { "subject": { "type": "broker" }, "level": "detailed" } ] },

  // Closers, by voice.
  { "id": "line.close.blunt.1",   "part": "closer", "voice": "voice.blunt",   "key": "talk-close-blunt-1" },
  { "id": "line.close.blunt.2",   "part": "closer", "voice": "voice.blunt",   "key": "talk-close-blunt-2" },
  { "id": "line.close.smooth.1",  "part": "closer", "voice": "voice.smooth",  "key": "talk-close-smooth-1" },
  { "id": "line.close.smooth.2",  "part": "closer", "voice": "voice.smooth",  "key": "talk-close-smooth-2" },
  { "id": "line.close.guarded.1", "part": "closer", "voice": "voice.guarded", "key": "talk-close-guarded-1" },
  { "id": "line.close.guarded.2", "part": "closer", "voice": "voice.guarded", "key": "talk-close-guarded-2" },
  { "id": "line.close.plain.1",   "part": "closer", "voice": "voice.plain",   "key": "talk-close-plain-1" },
  { "id": "line.close.plain.2",   "part": "closer", "voice": "voice.plain",   "key": "talk-close-plain-2" },

  // A memory quote, layered on top.
  { "id": "line.quote.broke_promise", "part": "quote", "key": "talk-quote-broke-promise",
    "when": [ { "type": "remembers", "slot": "speaker", "kinds": ["broke_promise"] } ] },
  { "id": "line.quote.kept_promise", "part": "quote", "key": "talk-quote-kept-promise",
    "when": [ { "type": "remembers", "slot": "speaker", "kinds": ["kept_promise"] } ] },
] }
```

```ftl
# content/base/locale/en/talk.ftl
talk-topic-who-runs = Who really runs things here?

talk-open-cool = { $speaker } keeps working while you talk.
talk-open-neutral = { $speaker } puts the kettle on.
talk-open-warm = { $speaker } waves you into the back office and shuts the door.

talk-runs-deflect = "Runs things? Nobody runs anything round here. Ask at the diner if you want gossip."
talk-runs-rumour = "People say { $broker } has the mayor's ear. I couldn't tell you more than that."
talk-runs-named = "{ $broker }. Nothing gets decided in this town without someone checking with { $broker } first."
talk-runs-stake = "{ $broker }, at { $broker_place }. And { $broker } has money riding on { $stake_key }, so watch what happens there."
talk-runs-self = "You're sitting across from the answer."

talk-close-blunt-1 = That's the long and short of it.
talk-close-blunt-2 = Don't make me say it twice.
talk-close-smooth-1 = But you didn't hear that from me, of course.
talk-close-smooth-2 = Let's keep this between friends.
talk-close-guarded-1 = I've said more than I should.
talk-close-guarded-2 = Best not to repeat that.
talk-close-plain-1 = Anyway, that's how I see it.
talk-close-plain-2 = For what it's worth.

talk-quote-broke-promise = "You made me a promise once, over { $subject_key }, back in { $year }. I remember."
talk-quote-kept-promise = "You kept your word on { $subject_key }. That counts for something here."

lead-ask-diner = The diner hears everything. Someone there might say who really runs this town.

# Fluent select for numbers, used by favour topics:
talk-favour-owed = "You owe me { $n ->
    [one] a favour
   *[other] { $n } favours
} already."
```

The same question at three relationship levels:

| Dana's opinion of the player | Told level | Reply (opener · quote · substance · closer) | Ledger |
|---|---|---|---|
| −30, cool | `unknown` | "Dana Price keeps working while you talk. 'Runs things? Nobody runs anything round here. Ask at the diner if you want gossip.' Don't make me say it twice." | Nothing learned; lead to the diner |
| +10, neutral | `heard` | "Dana Price puts the kettle on. 'People say Ruth Okafor has the mayor's ear. I couldn't tell you more than that.' That's the long and short of it." | `broker heard` (rumoured), source Talk with Dana at the union hall |
| +65, friendly, trust 0.7 | `detailed` | "Dana Price waves you into the back office and shuts the door. 'You kept your word on the closure response. That counts for something here.' 'Ruth Okafor, at the business association. And Ruth Okafor has money riding on the factories, so watch what happens there.' That's the long and short of it." | `broker detailed` (with the stake); +1 opinion |

The same facts from a smooth business leader (`trait.charmer`) end with "But you didn't hear that from me, of course." Asked again the same week, Dana picks the other blunt closer, and the guard keeps the same substance line out while another holds. If none does, the same fact comes back in its only line, which the lab reports as a coverage gap.

**Placeholders without pronouns.** The lines avoid pronouns ("{ $broker } has…") because characters have no pronoun field yet (§6). Once one exists, `{ $speaker_pronoun -> … }` selects work as above.

### 3.14 What remains code

- Topic mechanics (`TalkMechanic`).
- `who` rules for speaker knowledge (`KnowsDef`).
- New condition variants (§3.3).
- New request types.

Topics, lines, voices, discretion and costs are content.

## 4. M6 task breakdown

**Order:** M5d part 2 merges first. Then the knowledge ledger with conversations right after it (DEC-093: conversations are the ledger's main source), and then policy families, whose sponsors are met in conversation. Town layout v2 "stage B" means its engine inputs (E1 baselines, E2 `TownGeographyView`, E3 `TownStateView` and place status: D3) and its content registries (C: D4), plus G1 (D8, per-neighbourhood hazards).

**Rules:**
- **Save bumps.** The lead numbers save versions at merge, one bump per task. Each task writes its migration against "the previous version" and renumbers on rebase.
- **Shared files.** The shared enums `ContentFile`, `ContentBundle`, `QueryDto`, `ResponseBody` and `Effect` take append-only edits from several tasks; the lead resolves conflicts at merge.
- **Build parallelism** is set per brief (`CARGO_BUILD_JOBS`, WORKFLOW §7).

| # | Task | After | OWNS | Runs beside |
|---|---|---|---|---|
| **B1** | Town v2 E1: baselines at generation (save bump) | M5d2 | town v2 brief | K0, B3 |
| **B3** | Town v2 C: layout registries (D4) | M5d2 | town v2 brief | B1, K1 |
| **K0** | Lead: accept this contract, DEC-091/092 status notes | — | `docs/design/**` | B1, B3 |
| **K1** | Knowledge ledger core: types, `learn`/`read`/`lead`/`enter_zone`, `knows`, `Knowing`, `Effect::Learn/Read/Lead` and their application, `knowledge.jsonc` (starts, option, primer, intros, effect params), seeding at play start and on load, save bump and migration, fingerprint, determinism tests, lab `--omniscient` switch | B1 | `crates/mandate-sim/src/player/**`; `scheduler.rs` (Effect variants); `engine.rs` (seed and `on_load` calls); `crates/mandate-content/src/knowledge.rs` (new) and its `ContentFile`/bundle lines; `content/base/knowledge.jsonc`; `content/schemas/**`; `crates/mandate-save/**` | B2, B3 |
| **B2** | Town v2 E2/E3: town views, place status (D3) | B1 | town v2 brief (`mandate-protocol/src/town.rs` new; sim `world.rs`, `places.rs` status) | K1 |
| **G1** | Town v2 D8: per-neighbourhood hazard exposure, damage and displacement (save bump) | B1 | `environment/**`, `economy/housing.rs`, `population/**` (displacement), worldgen exposure | K2–K5 |
| **K2** | Protocol redaction (§1.7), `Gate`, `NotebookView`, `NewGameParams.local_knowledge`, TS types | K1, B2 | `crates/mandate-protocol/src/**`; `web/src/engine/protocol/**` (generated); `crates/mandate-wasm/src/session.rs` (new-game option) | K3, G1, F1 |
| **K3** | Knowledge producers (§1.3): arrival intros, overheard teaching, actions, decisions and scenes, stories; narrative `Learn`, `RevealPlace`, `Knows`; overheard `teaches` | K1 | `crates/mandate-sim/src/{actions,places,overheard,media}.rs`; `narrative/**`; `crates/mandate-content/src/narrative.rs`; `content/base/{overheard,dilemmas,scenes}.jsonc` (hooks only) | K2, G1, F1 |
| **G2** | Town v2 D7: hidden places on the ledger (`Subject::Place`, no `known_places`) | K3 | town v2 brief | K4 |
| **C1** | Conversations engine (§3): content types (`conversation.rs`: topics, lines, voices, params), the shared condition extension of §3.3 with argument providers, speaker knowledge and discretion, `conversations::reply` with the keyed guard, `PlayerCommand::Talk` (half-slot cost, mechanics through the existing lobby and favour paths), `TalkState`, the overheard repetition guard, save bump (shared with K1 if they merge together), validation 1–7, determinism tests | K1, K3 | `crates/mandate-content/src/{conversation.rs (new), types.rs (situation section only), validate.rs (situation and conversation sections), bundle.rs}`; `content/schemas/**`; `crates/mandate-sim/src/{conversations.rs (new), player/talk.rs (new), overheard.rs}`; `actions.rs` (`Talk` only); `crates/mandate-save/**`; `content/base/conversations/params.jsonc` and one sample topic | K2, K5 |
| **C2** | Protocol: `TalkView`, `ExchangeView`, `PersonRefView.can_talk`, notebook links to exchanges, TS | C1, K2 | `crates/mandate-protocol/src/{talk.rs (new), knowledge.rs, characters.rs, lib.rs}`; `web/src/engine/protocol/**` | C3 |
| **C3** | Content: the seven base topics, openers, substances (about 150–250 lines across roles), voices and closers, quotes, fallbacks per topic and role, `talk.ftl` | C1 (drafting can start with the formats) | `content/base/conversations/**`; `content/base/locale/en/talk.ftl` (new) | C2, C4, K4 |
| **C4** | Web: the talk panel (topics as buttons, odds, the exchange, what was learned), from the place panel, the List view and the walkable town; Playwright | C2, K5 | `web/src/**` except `engine/protocol` | K4 |
| **C5** | Lab: `mandate-cli talk` and `--coverage` (§3.12), the repetition gate in CI | C1 | `crates/mandate-cli/src/{talk.rs (new), main.rs}`; `.github/workflows/ci.yml` | C2–C4 |
| **K4** | Effectiveness terms (lobby, canvass issue), bots through `Knowing` (and talking through `TalkView`), `explorer`/`explore_only`, the gates (§1.10), blind-invariance test, BALANCE_LOG | C1 | `crates/mandate-sim/src/npc.rs` (lobby term); `actions.rs` (canvass issue); `crates/mandate-cli/src/{bots,main}.rs`; `content/base/knowledge.jsonc` (effect values); `docs/development/BALANCE_LOG.md` | K5, F1 |
| **K5** | Web: notebook page, placeholders with leads, pocket badge, "Start with local knowledge" at setup, Playwright | K2 | `web/src/**` except `engine/protocol` | C1, F1 |
| **F1** | Family content types, `SituationTrigger.all` and the family conditions of §2.4 (evaluated as "never holds" until F2b), `HazardRef`, new mechanism variants declared (rejected in content until F2b), `ParamDef.hidden`, `PolicyTemplate.retired`, `PlaceKindDef.drivers`, patches, validation 1–10, schema | B3, C1 | `crates/mandate-content/src/{policy_family.rs (new), types.rs (policy, situation, place-kind sections), validate.rs (policy, family, situation sections), bundle.rs}`; `content/schemas/**`; `content/base/policy_families/params.jsonc`; `content/base/places.jsonc` (`drivers`) | K4, C2–C5 |
| **F2a** | The lookup seam: every `content.policies.get(&p.template)` → `policy::resolve`; no behaviour change (state hashes identical) | F1, K4 | every call site (sim, protocol, CLI); runs alone, as a short freeze on those files | — |
| **F2b** | Families in the simulation: decks, binding, triggers, expiry, `policy.families` system, `Clause::Area` and per-neighbourhood modifiers, `Borrow`/`Revenue`/`SectorCapacity`, `DraftFamily`, `AmendPolicy` choices, sponsor prices, NPC drafting, `Stream::Policy`, save bump and migration, `missing_references` | F2a, G1, B2 | `crates/mandate-sim/src/policy/**`; `systems.rs`; `actions.rs` (draft and amend); `npc.rs` (agenda); `finance/**` (borrow, revenue); `economy/mod.rs` (capacity); `crates/mandate-core/src/rng.rs` (stream); `crates/mandate-save/**`; `engine.rs` | — |
| **F3** | Protocol: `ProposalView`, `proposals_here`, the extended `PolicyPreview` (§2.12), proposal gating, TS | F2b, K2 | `crates/mandate-protocol/src/{office,places,knowledge,lib}.rs`; `web/src/engine/protocol/**` | F4, F5 |
| **F4** | Content: the first pack (§2.21) with Fluent, the trim (§2.19), the four conversions (§2.20), proposal overheard lines, crisis and dilemma references | F2b (drafting can start after F1 with validation only) | `content/base/policy_families/**`; `content/base/policies/**`; `content/base/locale/en/families.ftl` (new); `content/base/{overheard,crises,dilemmas}.jsonc` | F3, F5 |
| **F5** | Lab: `policies --seed`, `--coverage` (CI gate: no family never binds), binding check 11, `--set` paths into families, variety counts families | F2b | `crates/mandate-cli/src/{policies.rs (new), main.rs, bots.rs (variety only)}`; `crates/mandate-content/src/overrides.rs`; `.github/workflows/ci.yml` | F3, F4 |
| **F6** | Web: the proposal sheet (axes with per-option cost and support from the preview, sponsors and their prices, the lead), Desk and place panel entries, Playwright | F3 | `web/src/**` except `engine/protocol` | — |
| **F7** | Need-aware reactions (§2.22): `addresses` on templates and family options, need-scaled reactions and the waste term, diminishing returns in housing, subsidies, services and hazard protection, the need part in previews and "Why?", the policy responsiveness lab gate | F2b, F3 | `crates/mandate-sim/src/{policy,population,economy}/**`, `content/base/{opinion.jsonc,policies/**,policy_families/**}`, `crates/mandate-cli/src/responsiveness.rs` (new) | F4, F5 |
| **V1** | Credit and blame (DEC-094): a per-neighbourhood track record per politician from law attribution ledgers (sponsor full share, voters a smaller share), slow decay, read by the vote record term for all candidates (× awareness × media), need-aware credit (§2.22), named laws from the media, "Why?" and election-night factors, lab gate (sponsors of laws that did well gain votes) | F2b, F7 | `crates/mandate-sim/src/{politics/**,policy/**}`, `content/base/politics.jsonc`, protocol "Why?" views, `crates/mandate-cli/src/bots.rs` | — |
| **V2** | Endorsements (DEC-094): endorser roles with follower weights per community, class and occupation in content; `Endorse` effects from conversations (C-series "ask for support"), favours and sponsors; the persuasion and turnout shift with a cap; the cost to the endorser; NPCs seek and give endorsements; election-night factors; lab gate (they matter, never dominate) | C1, V1 | `crates/mandate-sim/src/{politics/**,characters/**,npc.rs}`, `content/base/{characters,politics}.jsonc`, protocol, web election night | — |
| **A1** | No decorative attributes (DEC-095): administration (implementation speed and cost), organising (reach and mobilisation), intrigue (leaks, digging, deals, hiding their own facts), expertise (persuasion on their field; feeds C-series replies), loyalty (party-line votes, lobbying resistance, cross-party endorsements), boldness (risk-taking); parameters in content; "Why?" lines; the attribute-effect lab gate in CI | K4 | `crates/mandate-sim/src/{npc.rs,politics/**,characters/**,policy/**,media.rs}`, `content/base/{npc,characters}.jsonc`, `crates/mandate-cli/src/attributes.rs` (new) | C-series, F-series |
| **L1** | Odds calibration gate (owner, 2026-11-19): the bots record every probability shown to the player before acting (town hall hostility, press gaffe, lobbying and late-night lobbying, shady-act exposure bands, charge and conviction bands, council yes chances) beside what then happened; `mandate-cli calibration --seeds N` groups by stated chance (deciles) and fails when a group's observed rate falls outside a binomial confidence interval (99%); bands are checked against their ranges. Runs in CI with the bots (small N) and nightly (large N); a report table for BALANCE_LOG | K4 (bots read through `Knowing`) | `crates/mandate-cli/src/calibration.rs` (new), `crates/mandate-cli/src/{main.rs,bots.rs}` (recording hooks), `.github/workflows/{ci,nightly}.yml` | any |
| **K6** | "The world does not wait" guards (DEC-091 note): validation rejects `knows` in dilemma, storyline and family triggers (allowed in scene variants, lines, option framing and leads); a lab test that an omniscient and a default player with the same commands produce identical worlds except the ledger (exactly for the idle bot; except the player's own action outcomes after K4); audit the director so knowledge never suppresses an event | K3 | `crates/mandate-content/src/{narrative.rs,validate.rs}`, `crates/mandate-sim/src/narrative/**` (audit only), `crates/mandate-save/tests/knowledge*.rs` or `crates/mandate-cli` (test) | any |
| **P1** | The power broker acts (DEC-096): broker lobbying of councillors for their stake (standard formula), endorsements with large follower weight and campaign money for aligned candidates, ally help with strings (favours, sponsor deals, shady-help scandal risk) and opposition (rumours as media stories, rival funding, council pressure), broker turnover; parameters in content; lab gates (matters, never dominates; variety holds) | V2, A1 | `crates/mandate-sim/src/{npc.rs,characters/**,politics/**,media.rs}`, `content/base/{characters,npc,politics}.jsonc`, `crates/mandate-cli/src/bots.rs` | F-series |
| **D1** | Docs: OVERVIEW §4.2 and §4.3, DATA_MODEL, EXTENDING (extension points §2.15, families, knowledge), TEST_STRATEGY (gates), PROGRESS, DEC implementation notes | each merge | `docs/**` | — |

Then the rest of M6 (enterable places DEC-088, map layers DEC-089, the draft of three with background profiles, pixel-art integration) builds on these contracts. The draft of three only needs to name a start profile per background.

---

## 5. Open questions for the owner

**Answered (owner, 2026-11-19): "Go with all recommendations."** Every recommendation below is accepted, and every **Proposed** item in §1–§3 that refers to these questions is now settled as recommended. Need-aware reactions (DEC-092 addendum) are specified in §2.22.

| # | Question | Recommendation |
|---|---|---|
| 1 | DEC-091 says you learn "through actions, not movement". Should arriving at a place (the same command whether you walk, tap the List view or use quick mode) count as learning? | **Yes, lightly.** Arrival gives the place's short introduction and lets you overhear (rumours, names). Anything deeper needs an action or a person. |
| 2 | Should everyone know the councillors' names from the start (they are public figures), with their stances and goals learned later? | **Yes.** Names and wards are on the noticeboard; stances need a meeting, goals a second conversation. |
| 3 | Can a proposal you have only heard of be drafted? | **No.** It shows as a stub with a lead ("the union hall would know…"). Drafting needs you to have seen it properly or met the sponsor. |
| 4 | Add a paid shortcut: "commission a survey" at the statistics desk (money and one slot) for every neighbourhood's worries in detail? | **Yes, in M6.** It is DEC-079 rule 5, and it gives players who dislike exploring a fair, costly alternative. |
| 5 | Retire flood defences, the tech incubator and the local wage floor into families, leaving eight standard levers? | **Yes.** Existing runs keep their laws; new runs meet them as town-grounded proposals. |
| 6 | How many proposals per run and at once? | **About two-thirds of the families that could apply in a town, at most four open at once, one per family** (two for flood walls). Tune with the coverage report. |
| 7 | Sponsor prices in M6: a favour (including "back my next bill") or a public endorsement; no cash. Adopting a sponsor's version pleases them (+8); each change you make costs a little (−4). | **Yes.** Cash prices belong to the shady actions. |
| 8 | When the town lacks the power a proposal needs (DEC-092 §5), it becomes something to lobby the region for. Build that in M6? | **Defer to M7**, when the region is playable. Keep the field in the format; in M6 such options show "needs the region" and can't be chosen. |
| 9 | Should non-player politicians have their own notebooks? | **No.** They are locals and already act on public knowledge; fairness comes from using the same formulas. Lab bots (which play as you) do learn. |
| 10 | Mods "patch" families: allow small, named changes (draw weight, add or remove an option or sponsor, disable a family) rather than replacing whole entries? | **Yes.** It is safer for saves and easier to review. Families become the first registry with patches. |
| 11 | When you don't know something, show a placeholder ("Power broker: not known yet — try the union hall") or leave it out? | **Show the placeholder with its lead**, so finding things is guided, not hunted (DEC-091). |
| 12 | "Reopen {bridge}" needs bridges in the simulation. | **Defer** to the infrastructure work (M7 or later). The first pack in §2.21 doesn't need it. |
| 13 | How much time does a conversation take? | **Half a slot** (two conversations a slot), with "say hello" staying free. A whole slot would make talking too dear beside lobbying; free talk would make it busywork. |
| 14 | How guarded should people be? | **As in §3.4:** cool or hostile people deflect (and point you elsewhere), neutral ones pass on rumours, warm ones tell you plainly, and only friends who trust you give the detail. Schemers and cautious people are one step more guarded. |
| 15 | Can you ask the same person the same thing twice in a week? | **Yes, at the usual cost**, and you get a different line. The talk panel marks the topic "asked this week", and nothing new is learned unless your relationship changed. |
| 16 | Pronouns: lines would read better with "she said" or "they'll tell you", but characters have no pronouns yet. | **Add a pronoun field to characters** with the appearance work (pixel-art P1). Until then lines use names, as in §3.13. |

---

## 6. Where today's code differs from DEC-091, DEC-092 and DEC-093

These findings come from the code on `main` and the M5d part 2 branch. Each is a gap this contract closes, or one the owner should know about.

1. **Bots are omniscient.** `mandate-cli/src/bots.rs` reads state directly. `legislate` ranks templates by true town support, which DEC-091 gates. Fixed by `Knowing` and the blind-invariance test (K4).
2. **Every view shows everything.** The broker (`TownCharacterView.broker`), people's names at places, concerns in the pocket and the town views, personality, convictions and pragmatism are all ungated. This is expected: DEC-091 is new. §1.7 lists the redactions.
3. **A second discovery store is planned.** TOWN_GENERATION §11 proposes `PlayerState.known_places` and a `knows_place` condition. DEC-091 asks for one ledger, so G2 must use `Subject::Place` and `Condition::Knows`.
4. **Policy effects are town-wide.** `PolicyMods` is per municipality. `Clause::Pilot` scales cost and effect by 0.2 everywhere rather than placing them. Hazard damage and displacement hit whole towns. "Where it applies" (DEC-092 §3) needs D8 and `Clause::Area` (G1, F2b).
5. **One template per bill is hard-wired.** 28 call sites look templates up by `content.policies.get(&p.template)`. Families need per-instance composition, hence the F2a seam.
6. **Removing a template refuses saves.** `missing_references` refuses any save whose templates no longer exist, so trimming the menu or converting the situational templates would break runs. Hence `retired: true`.
7. **Powers are too coarse for DEC-092 §5.** The constitution's `Power` enum has no policy-area competences, and every local tier can `propose_ordinance`, so "the town lacks that power" can never hold. It needs competences per tier (M7, question 8).
8. **No organisations, no place sectors.** "The union" or "the plant's owner" can bind only a leader character and a place. Places have no sector, so the new `PlaceKindDef.drivers` is needed. There is no per-sector job history, so "this plant is at risk" needs E1 baselines.
9. **Pack patches are not supported.** A duplicate id is an error (EXTENDING "Packs"). DEC-092 expects mods to patch families, hence the patch format (question 10).
10. **Two overlapping condition languages.** Dilemmas use narrative `Condition` (crisis, indicator, concern, industry, law in force); policies use the new `SituationCondition` (crisis, indicator, closure, disaster). This contract extends `SituationCondition` as the town-level vocabulary and avoids a third. Merging the town-level variants of `Condition` into it is a later clean-up.
11. **`SituationTrigger` can only "any".** Families need `all` and slot-relative conditions. The extension in §2.4 is backward compatible.
12. **Untyped ids.** Media, justice and narrative use raw `u32` ids (scandals, stories, rumours, cases, outlets, hidden facts), and `PolicyInstance.jurisdiction` and `Clause::Pilot.neighborhood` are raw too, against DEC-041's typed ids. `Subject::Secret` and `Source::Story` inherit this. Typed wrappers are recommended when those modules are next touched.
13. **Views can't teach.** Overheard talk and place panels are queries, so they can't write the ledger. Learning has to hang on commands (arrival and actions), which is why §1.3 treats arrival as the learning moment (question 1).
14. **The start is not a background.** Worldgen always makes the player a sitting councillor in the most marginal ward. Background-based knowledge needs the M6 draft of three; until then a default profile stands in.
15. **There is no way to say something.** Today talking means overheard chatter (passive), `Meet` and decision scenes. `Meet` is free in the code (`PlayerCommand::Meet`: "introduce yourself … (free)"), while DEC-093's context says it uses a slot. This contract keeps it free and prices the new `Talk` (question 13).
16. **Three condition languages.** Narrative `Condition` (dilemmas), `SituationCondition` (policies) and `HeardCondition` (overheard) overlap. DEC-093 requires conversation lines to use the same type as DEC-092 triggers, so C1 extends `SituationCondition` with person-relative variants and argument providers (§3.3). Moving dilemmas and overheard lines onto it is a later clean-up.
17. **No pronouns or gender on characters.** `NamedCore` and `Politician` have names only, so the Fluent select expressions DEC-093 mentions for gender have no fact to select on (question 16).
18. **Overheard repetition is stateless.** It avoids last week's draw by recomputing it. DEC-093's guard needs a per-place memory of lines heard, recorded on arrival (§3.7).
19. **Doc drift (minor).** OVERVIEW §3 lists the named RNG streams without `Media`, and OVERVIEW §6 and DATA_MODEL still say save format v12 (the code is v13). D1 fixes both.
