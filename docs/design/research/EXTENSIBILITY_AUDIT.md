# Extensibility audit: seams for Acts 2–4, and what to make data

**Status:** Research (2026-11-19). Read-only audit of `claude/bold-brown-shyeug` at save format v17 (K1–K4, K6, C1, C3, B1–B3, G1 merged; F-, E-, V-, P- and A-series not started). Nothing here changes an accepted decision. Proposals for DECISIONS.md are in §6; the lead decides.
**Owner questions:** "Do we have the hooks in place for adding the next stages later? Like governor senator and president? Or will it be a big build out once we're happy with act 1?" and "what parts of the game we should consider making data file driven so it's easier for us to add and expand later". **Owner requirement (2026-11-19, decided):** "The offices should actually be integrated as part of the constitution as well." See §7.
**Read with:** DEC-004, DEC-017, DEC-020, DEC-036, DEC-053, DEC-075, DEC-076, DEC-078, DEC-079, DEC-091–DEC-097; `ROADMAP.md` §2; `M6_ENGINE_CONTRACTS.md`; `ENGAGEMENT.md`.

Costs used throughout: **S** a local edit; **M** a system's interface; **L** cross-cutting, or the state shape or save format. `[save]` marks a type that is written into saves; `[protocol]` a type the web receives.

---

## 1. Summary for the owner

**Short answer: half the hooks are in place.** The *world* is already built for all three tiers. The *politics and the player* are built for one town.

| Layer | Ready for regional and national play? | Evidence |
|---|---|---|
| Constitution | **Yes, as data.** Local, regional and national tiers with bodies, offices, terms, calendars, powers and procedures | `content/base/constitutions/federal_presidential.jsonc:9-127` |
| World | **Yes.** Regions, regional and national districts, capitals with an assembly, a parliament and an executive residence | `mandate-worldgen/src/lib.rs:504-527`, `world.rs:259-261` |
| Population, economy, finance | **Yes.** Cohorts nationwide; local, regional and national treasuries; grants between tiers; national macro | `finance/mod.rs:170-183` |
| Media | **Yes.** Outlets scoped to a town, a region or the nation | `media.rs:49-53` |
| Characters | **Mostly.** Named people exist in every town; town character (broker, rival) is per town | `characters/mod.rs:306-308` |
| Offices and roles | **No.** A politician is a councillor, a mayor, or out of office, coded in Rust | `politics/mod.rs:25-38` |
| Elections | **No.** One local calendar, council and mayoral seats only | `politics/elections.rs:27-46`, `:156-166` |
| Law-making | **No.** Every vote is the town council's and every signature the mayor's, whatever the procedure says | `policy/mod.rs:572`, `:725-760` |
| The player's world | **No.** 109 places read "the player's town"; no travel; the director, goals, NPC detail and talk all centre on it | §2, S9 |
| Screens | **No.** The office, chamber and campaign screens are shaped as "council plus mayor" | `protocol/src/office.rs:392-400`, `web/src/components/CampaignCards.tsx:18-19` |

**How big are M7 and M8?**
- **M7 (regional) is a large milestone, not a rewrite.** About the size of M4 (elections, parties and NPC AI) plus M5a (places). Roughly half of its engine work is *generalising* what Act 1 hard-codes: roles, seats, elections per tier, law-making through the procedure's real bodies, "the player's town" becoming "where the player is and what they hold", travel between hubs, and detail levels. The other half is new: regional institutions and content, the DEC-078 registries, neighbour nations. **If the two items at the top of the list below are done in M6, most of that generalisation is already paid for**, and each M6 system built in the meantime (asks, secrets, families, endorsements, the broker) will not need re-keying.
- **M8 (national and presidency) is larger than M7 but mostly new systems:** a bicameral legislature with leadership, a cabinet and appointments (the constitution schema lacks appointed offices with portfolios, §7.6), macro levers (national finance already runs), the Mandate Test and foreign policy (a new domain, DEC-087). It rides on M7's seams; without them, M8 would repeat M7's generalisation for a third tier.

**Top 10 actions, ranked by "cheap now, expensive later":**

| # | Action | Cost now | Cost if left to M7 | Where |
|---|---|---|---|---|
| 1 | **Offices and seats from the constitution** (owner requirement): `Role::Seat { body, at, constituency }` / `Role::Office { office, at }` / `Role::Private`, elections and filing on the same `Post`, powers read from the constitution, a save migration. Do it while there are two offices and before the M6 systems add more matches | M (≈ 30 files, one save bump) | L (every M6 system adds `Role::Mayor` matches; regional roles force the same rewrite plus a bigger migration) | §7, task R1 + R2 |
| 2 | **A `Jurisdiction` key, and key the not-yet-built M6/DEC-097 state on it**: policy-family decks, instances and cooldowns; asks' askers and approval effects; NPC deeds; secrets. These are brief edits today | ≈ 0 (brief amendments) | L (each is saved state; each needs its own migration) | §3, §7.1 |
| 3 | **The draft of three describes a start as a constitution post** (a seat or an office, plus a constituency selector and a knowledge profile), not "councillor / challenger" in code | S | M (starts are written into saves and the title screen) | §3.9 |
| 4 | **A brief rule: no new reads of `player.municipality`.** New code asks `player::home(state)` or `state.focus()`; both added in R1 | S | Grows by roughly ten read sites per M6 system | §5.1 |
| 5 | **Law-making follows the procedure's own bodies and offices** (today the stage's `body` and `office` are ignored) | S (inside R1) | M (bicameral national procedure in M8) | §7.3 |
| 6 | **Election calendars per body and office** (today the term comes from the tier's first body, which would schedule the president on the House's 2-year cycle) | S fix now; per-tier cycles additive in M7 | M | §7.4 |
| 7 | **Endings and Legacy as a registry**, built data-first when Act 1 endings are built in M6 | S | M (endings referenced by saves and Legacy) | §4 |
| 8 | **Content refers to offices by constitution ids**, not by `"mayor"`/`"councillor"` enums (`Cast::Mayor`, `OfficeRole`, `POLITICIAN_ROLES`). E3 adds 22 dilemmas and E8 16 asks that would otherwise use the old names | S (inside R1, old spellings kept as aliases) | M (content volume) | §7.2 |
| 9 | **Interiors and character appearance as data** in the pixel-art P1 (room recipes; an appearance genome by occupation and tier), since M7/M8 need assembly, parliament and suits | S | M | §4 |
| 10 | **DEC-078 decision 5 registries before M7** (budget categories, revenue types, hazards, commodities), plus the worldgen place plan; move the hard-coded treasury shares to content now | S now for the shares; M for the registries either way | M | §4 |

**Not worth doing early:** a scripting language (§4.3), converting the `PlaceKind` enum, region-wide knowledge subjects, or per-tier cohort approval. Each can be added later with a serde default and no migration.

---

## 2. Seams table

Verdicts: **now** = cheap to generalise now and expensive later; **M6 brief** = generalise inside an M6 task that is about to start; **before M7** = needed for M7, cost does not grow much by waiting; **fine** = can be added later without a migration (usually an additive field or enum variant with `#[serde(default)]`).

### 2.1 Engine: politics, elections, law-making (save format first)

| # | Where | What it assumes | Blocks | Cost later | Verdict |
|---|---|---|---|---|---|
| S1 | `mandate-sim/src/politics/mod.rs:25-38` `Role { Councillor { municipality: u32, ward }, Mayor { municipality }, Private { municipality } }` `[save]` | Two local offices, named in Rust; raw `u32` town | Governor, assembly member, House, Senate, President, cabinet | L | **now** (§7, R1) |
| S2 | `politics/elections.rs:27-46` `Seat { Council, Mayor }`; stored in `Race.seat` (`:67`), `RaceResult.seat` (`:99`) and memories `characters/mod.rs:196` `Subject::Seat` `[save]` | The same two offices | Any non-local race | L | **now** (R1) |
| S3 | `politics/elections.rs:1512-1517` `Filing { Council, Mayor, Retire }` in `PlayerState.filing` `[save]`; `protocol/src/lib.rs:212-216` `FilingDto` `[protocol]` | Running means "my ward" or "mayor" | Running for governor or Congress | M | **now** (R1, R2) |
| S4 | `politics/mod.rs:95-108` `Council { municipality, members, mayor }`, `PoliticsState.councils` indexed by municipality `[save]`; read in 30 places (`elections.rs` 9, `policy/mod.rs` 3, `bots.rs` 3, …) | Every jurisdiction has one council and one mayor | Regional assembly, House and Senate storage | L | **now** (R1) |
| S5 | `actions.rs:176-215` `powers_for` reads only `TierKey::Local`, matches on `Role`, filters bodies by `Constituency::Ward` and offices by `AtLarge`; `npc.rs:115-130` `holds_power` the same | Powers depend on the role's name and the local tier | Any regional or national power | M | **now** (R1) |
| S6 | `policy/mod.rs:572` and `:647` `council_leans` / `mayor_lean` read `councils[inst.jurisdiction]`; `:725-760` the procedure's `StageDef::Vote { body, .. }` and `Sign { office, .. }` are matched but `body` and `office` are ignored; `:805-812` the signer is always the mayor | Every stage is the council or the mayor | Governor's veto, bicameral House + Senate, the presidential signature | M | **now** (R1, §7.3) |
| S7 | `politics/elections.rs:156-166` `local_calendar` takes the term from `bodies.first()`; `:288-293` the term limit from `offices.first()`; `:186-190` `daily` runs the local tier only; `:128-138` `ElectionState` holds one cycle (`next_day`, `campaign_open`, `races`, `national_swing`) `[save]` | One election calendar; every post in a tier shares one term | Regional mid-terms (offset 2), House 2 y / President 4 y / Senate 6 y | M | **now** for the term bug (R1); **fine** for per-tier cycles (additive `cycles` in M7) |
| S8 | `politics/elections.rs:319-320` campaigns open for municipalities only; `:357-359` and `protocol/src/campaign.rs:335` "Standing ≥ 40 to run for mayor" in code | The career ladder's one step is coded | Every later step (governor, Congress, President) | S each, but repeated per office | **now** to data (R1, `career.nomination`) |
| S9 | `politics/elections.rs:1318-1356` seating winners by `Seat::Council` / `Seat::Mayor`; `:1523-1560` `appoint_replacement` by role | As S1 | As S1 | M | **now** (R1) |
| S10 | `policy/mod.rs:65-71` `PolicyInstance { tier: TierKey, jurisdiction: u32 }` `[save]`; `:131-135` `in_municipality` filters `TierKey::Local` | Already (tier, id): a jurisdiction in disguise | Nothing, once read through an accessor | S | **fine**; add `PolicyInstance::at() -> Jurisdiction` in R1 |
| S11 | `policy/mod.rs:44-51` `Clause::Earmark { ward }`, `Pilot { neighborhood: u32 }` `[save]` | A ward project | Nothing: `ConstituencyId` exists at every tier | — | **fine** |
| S12 | `policy/mod.rs:117-121` one `Agency` per municipality `[save]` | Only towns implement laws | Regional and national agencies | S (additive) | **fine** |
| S13 | `policy/mod.rs:728`, `actions.rs:634`, `npc.rs:807`, `protocol/src/office.rs:571` read the procedure `"ordinance"` by id; `mandate-content/src/validate.rs:376` requires `ordinance` and `budget` | Procedure ids are a contract with content | Nothing | — | **fine** (documented contract) |
| S14 | `mandate-content/src/types.rs:1929-1960` `CouncilParams.mayor`, `mayor_own`, `mayor_public`, `sign_bias` | Parameters named for the mayor | Nothing: they apply to any signing office | S | **fine**; document as "the signing office" |

### 2.2 Engine: the player, focus and detail level

| # | Where | What it assumes | Blocks | Cost later | Verdict |
|---|---|---|---|---|---|
| S15 | `player/mod.rs:42-45` `PlayerState { municipality, ward }` `[save]`; **109 read sites** (`bots.rs` 15, `actions.rs` 11, `player/knowledge.rs` 10, `elections.rs` 9, `characters/mod.rs` 7, `places.rs` 6, `protocol/src/places.rs` 6, …) | One field means three things: where the player lives, the constituency they hold, and where the simulation is detailed | Travel between hubs, a regional constituency, detail levels (M7) | L, and grows ≈ 10 sites per new system | **now**: add `player::home(state)`, `player::posts(state)` and `State::focus()`; brief rule §5.1; convert the old sites before M7 |
| S16 | `engine.rs:502-515` `npc_turns`: the player's town every turn, the rest every fourth | Detail follows the home town | Detail level promotion and demotion (M7) | S once S15's `focus()` exists | **fine** |
| S17 | `places.rs:107-117` `hood_in_town`, `:119-160` `spots`: movement and routing only inside the home town | One hub | Travel (M7) | S: `Location` (`places.rs:26-34`) already uses world-wide ids | **fine** |
| S18 | `narrative/cast.rs:44-50` `Ctx::town()`; `characters/goals.rs:2-10`, `:99` goals for the player's town only; `narrative/director.rs:301` scenes in the player's town; `environment/mod.rs:269` director levers on the player's town | The director's world is one town | Regional stories | S each once S15's `focus()` exists | **fine** (convert with S15) |
| S19 | `politics/elections.rs:669`, `:1126-1131`, `:1387-1388` campaign weeks, polls and events detailed for the player's town only | As S16 | As S16 | S | **fine** (convert with S15) |
| S20 | `population/mod.rs:77-82` `Cohort.gov_approval` is approval of the *local* government; `player_awareness`, `player_approval` `[save]` | Voters judge one government | Approval of the governor or the President | M (cohort size × 3) | **fine**: add per-tier approval with a default when a tier becomes playable |

### 2.3 Engine: knowledge, characters, narrative, media, finance

| # | Where | What it assumes | Blocks | Cost later | Verdict |
|---|---|---|---|---|---|
| S21 | `player/knowledge.rs:233-235` `Zone { Town { town } }` `[save]` | Local knowledge zones are towns | Nothing: a regional capital is a town. A region-wide primer can add `Zone::Region` later | S (additive) | **fine**. `Zone::Capital` is not needed: the capital is a `Town` |
| S22 | `player/knowledge.rs:41-80` `Subject::{TownConcerns, Broker, RivalPlans, PolicySupport, Situational, Layer} { town: MunicipalityId }` `[save]` | Knowledge is about towns | Region-wide worries and a regional broker | S (additive variants such as `RegionConcerns { region }`) | **fine**. Do not rename existing variants (ledger keys are saved) |
| S23 | `player/knowledge.rs:395-400` `Scope::of_player` (town, home, ward); `mandate-content/src/knowledge.rs:82-83` `GrantSubject::Council` "every councillor and the mayor" | Grants name the local council | Primer on entering a region | S | **R1**: `council` means "the members and holders of the zone town's local-tier posts" |
| S24 | `characters/mod.rs:306-308` `towns: Vec<TownCharacter>` per municipality; `:314-316` goals for the player's town `[save]` | Broker, rival and crises are per town | A regional broker | S (additive `regions`) | **fine** |
| S25 | `characters/town.rs:231-233` the broker's seniority: "the mayor counts double" | An office name in a formula | A governor or senator as broker | S | **R1** (the executive of the jurisdiction) |
| S26 | `characters/knows.rs:30-32`, `characters/mod.rs:1430-1432` role keys `"councillor"`, `"mayor"`, `"private"` | Content and Fluent keys use these words | New offices in conversation topics and lines | S now, M once more content uses them | **R1** (§7.2) |
| S27 | `talk/context.rs:208-209` "a mayor is found at the town hall, anyone else at the chamber" | Where an office works is code | Governor at the capitol, senator at parliament | S | **R1** (`works_at` on bodies and offices, §7.6) |
| S28 | `narrative/mod.rs:459-461` `HiddenFact.town: Option<u32>` (raw) `[save]` | Scandals belong to a town | Regional or national scandals | S now inside E9's planned bump; M later | **M6 brief** (E9, §3.6) |
| S29 | `media.rs:49-53` `Scope { Town { municipality: u32 }, Region { region: u32 }, Nation }` `[save]` | — | — | — | **fine**: this is already a jurisdiction. `Jurisdiction` (§7.1) copies its shape with typed ids |
| S30 | `events.rs:35-80` policy, closure and disaster events carry `municipality: u32`; `:89-96` `RaceDecided { municipality, mayor: bool }` `[save]` | Events happen in towns; races are mayoral or not | Regional bills and races in the log | S (additive `#[serde(default)]` fields) | **fine**; R1 adds `post` to `RaceDecided` |
| S31 | `finance/mod.rs:160-163` `Appropriation.municipality: u32` `[save]` | Only towns fund programmes | Regional and national programmes | M | **fine**: add `#[serde(default)] tier` with the first regional bill (M7) |
| S32 | `finance/mod.rs:15-67` `Category`, `Revenue`, `TierKind`; `:96-99` budget shares per tier hard-coded; `:148-157` `TaxRates` fields per tier; `mandate-content/src/types.rs:339` a second `BudgetCategory` enum | Fixed categories and taxes; three separate tier enums (`world::Tier` `world.rs:200`, `TierKey` `types.rs:136`, `finance::TierKind`) | Defence and foreign affairs categories (DEC-078 §5), national tax design | M | **before M7** (registries, DEC-078 §5); move the shares to `economy.jsonc` now (S); stop adding tier enums (R1 adds `From<Tier> for TierKey`) |
| S33 | `environment/mod.rs:43-49` `HazardKind` (5) and `mandate-content/src/types.rs:365-369` `HazardKey` (3) | Two closed hazard enums | Heatwave, epidemic (DEC-078 §5) | M | **before M7**. Content spellings (`"flood"`) survive a registry, so F1's `HazardRef` is not at risk |
| S34 | `events.rs:30-33` `CommodityShock { commodity: u8 }` "0 agriculture, 1 minerals, 2 energy" `[save]` | Three commodities by index | Commodity registry (DEC-078 §5), trade (DEC-087) | M | **before M7** |

### 2.4 World, worldgen, content schema

| # | Where | What it assumes | Blocks | Cost later | Verdict |
|---|---|---|---|---|---|
| S35 | `world.rs:200-215` `Tier` and `Constituency { tier, jurisdiction: u32 }` `[save]` | — | — | — | **fine**: already tiered. Add `Constituency::at() -> Jurisdiction` (R1) |
| S36 | `world.rs:237-262` `PlaceKind` enum (23, saved as variant names) | New kinds need a Rust variant | Nothing for M7/M8: `RegionalAssembly`, `Parliament`, `ExecutiveResidence` exist | S per kind | **fine** |
| S37 | `mandate-worldgen/src/lib.rs:942-1010` `place_player`: always a sitting ward councillor in the largest town | The start is code | The draft of three, varied openings (M6, DEC-036, DEC-077 §2) | S now, M later | **M6 brief** (§3.9) |
| S38 | `mandate-worldgen/src/places.rs:34-` the place plan per town is code (which places, which neighbourhood, names) | Town recipes in Rust | Regional and national kits (P2), mods | M | **before M7** (§4) |
| S39 | `mandate-content/src/types.rs:176-183` `Constituency { Ward, AtLarge }`; `:200-232` `BodyDef`/`OfficeDef` have no seat count, no own calendar, no workplace; `:146-160` `Power` has no parameters; `Selection::AppointedBy { office }` has no confirmation | The schema fits the local tier | A senate elected per region, staggered terms, cabinets, confirmations | M (schema plus validation) | **R1** for the small fields; **before M8** for appointments (§7.6) |
| S40 | `mandate-content/src/narrative.rs:202-203` `Cast::Mayor`; `:258-265` `OfficeRole { Councillor, Mayor, Any, None }`; `:404` `Condition::Office { role }`; `content/base/dilemmas.jsonc:164` `{ "type": "office", "role": "councillor" }` | Content names offices through Rust enums | New offices in dilemmas, asks and storylines | S now, M as content grows (E3, E8) | **now** (R1, §7.2) |
| S41 | `mandate-content/src/conversation.rs:22` `POLITICIAN_ROLES = ["councillor", "mayor"]`; `content/base/conversations/topics.jsonc:25` `"roles": ["councillor", "mayor"]` | As S40 | As S40 | S | **now** (R1) |
| S42 | Voting rules: `VotingRule::{TwoRound, RankedChoice, ListPr}` (`types.rs:167-173`) are accepted by the schema but the tabulator only implements plurality | The schema promises more than the engine does | M9 archetypes | S | **fine**; validation should refuse unimplemented rules (§6) |

### 2.5 Protocol, bots and web

| # | Where | What it assumes | Blocks | Cost later | Verdict |
|---|---|---|---|---|---|
| S43 | `protocol/src/lib.rs:270-330` `QueryDto::{Office, Campaign, Pocket, Town, Newspaper}` take no jurisdiction `[protocol]` | "The player's town" | Viewing a region or the capital | S (additive `#[serde(default)] at`) | **fine** |
| S44 | `protocol/src/office.rs:115-124` `PlayerView { role_key, municipality, municipality_id, ward_number, ward_id }`; `:392-400` `OfficeView { council, mayor }`; `:305` `BillView.mayor`; `:475-477`, `:546-547`, `:869-871` role keys by match `[protocol]` | The office screen is a council and a mayor | Every non-local office screen | M (generated TS plus several screens) | **now** (R2) |
| S45 | `protocol/src/campaign.rs:50-61` `RaceView.seat: "mayor" \| "ward"`; `:105-108` `filing`, `mayor_nomination_open`; `:128-142` `ward_number`, `seat_key`; `:329-335` `[protocol]` | Two kinds of race | Any other race | M | **now** (R2) |
| S46 | `protocol/src/knowledge.rs:576-577` the rival's seat; `protocol/src/characters.rs:229-237`, `:304-308`; `protocol/src/places.rs:835-841` `[protocol]` | As S44 | As S44 | S | **now** (R2) |
| S47 | `mandate-wasm/src/session.rs:375-377` maps `FilingDto` | As S3 | As S3 | S | **now** (R2) |
| S48 | `mandate-cli/src/bots.rs:191`, `:575-580`, `:619-626`, `:688`, `:1427`, `:1541-1574`, `:1633`, `:1663` the player's town and `councils[...]` | Bots play one town | Bots in regional races (M7 gates) | S (mechanical) | **R1** (mechanical) |
| S49 | `web/src/components/CampaignCards.tsx:10`, `:18-22`, `:58`, `:193`; `TownCard.tsx:36`; `BillCard.tsx:92`; `Chamber.tsx:118`, `:175`; `ActionForm.tsx:39`; `PeopleList.tsx:32` | Labels and lists for "council" and "mayor" | Every other office in the UI | M | **now** (R2) |
| S50 | `web/src/state/gameStore.ts:301` loads office, campaign, pocket and town for the one town; `web/src/hub/gameTown.ts:70` `planPlayerTown` | One hub | Travel, the regional capital | S | **fine**: the plan cache is already keyed by town (`hub/townPlanCache.ts:59`), and `TownGeography`/`TownState` take an id |
| S51 | Layout v2 is tuned on towns: plan time 256 ms cold for Fenoport against a 60 ms budget (`PROGRESS.md`, "Town layout v2, stage A"); regional capitals are `City` or `Metro` (`mandate-worldgen/src/lib.rs:313-318`) and the player has never been placed in one (`lib.rs:942-956`) | Hubs are towns | The M7 hub in a regional capital | L (performance, possibly an algorithm change) | **measure now** (a lab check, §5.3); fix in M7 |

---

## 3. In-flight M6 and DEC-097 work: amendments to the briefs

None of the F-, E-, V-, P- or A-series state exists yet, so keying it on a jurisdiction costs nothing now. Each amendment below is phrased as an edit to the task's brief. They all assume R1's `Jurisdiction`, `Post`, `player::home` and `State::focus()` (§7, §5). Where a task runs before R1, it uses `Jurisdiction::Municipality { id }` from R1's first commit, or a local stand-in with the same serde shape.

### 3.1 Policy families (F1, F2b, F3, F5; `M6_ENGINE_CONTRACTS.md` §2)

| Contract (§ of M6_ENGINE_CONTRACTS) | Change |
|---|---|
| §2.10 `FamilyState.decks: BTreeMap<MunicipalityId, Vec<ContentKey>>` | `BTreeMap<Jurisdiction, Vec<ContentKey>>` |
| §2.10 `FamilyState.lapsed: BTreeMap<(MunicipalityId, ContentKey), SimDate>` | `BTreeMap<(Jurisdiction, ContentKey), SimDate>` |
| §2.10 `FamilyInstance.town: MunicipalityId` | `at: Jurisdiction`. A family's `tier` must equal `at.tier()` (validated when an instance opens) |
| §2.8 deck draw `[TAG_DECK, town, hash(id)]`; binding `[TAG_BIND, town, …]` | `[TAG_DECK, at.tier_index(), at.raw(), hash(id)]` (and likewise for binding), so a region's deck never collides with a town's |
| §2.8, §2.21 `max_open_per_town` | `max_open_per_jurisdiction` (content key; the value is unchanged) |
| §2.4 `policy::situation_holds(state, content, c, town, bindings)` | `at: Jurisdiction`. Town-level conditions evaluate for `Municipality`; for `Region`/`Nation` they read the aggregate or, until M7, never hold (validated: a regional family may only use conditions marked tier-generic) |
| §2.3 binding kinds "in the town" (`place`, `neighborhood`, `person`, `broker`) | "in the jurisdiction": iterate `at.hoods(world)` and the jurisdiction's places; `broker` reads the town's broker for `Municipality` and is unbound elsewhere until M7 |
| §2.10 "In M6 it runs for the player's zone only" | "for `state.focus()` jurisdictions only" |
| §2.11 `DraftFamily` refused when "not in the player's town" | refused when the player holds no post with the family's `requires` power at `at` (R1's `holds_power_at`) |
| §1.1 `Subject::PolicySupport { town, policy }` | unchanged (towns are where support is measured); a regional proposal adds a variant in M7 |
| F3 `ProposalView` | add `at: JurisdictionDto` |

### 3.2 Asks and promises (E6, E7, E8; `ENGAGEMENT.md` §3)

| Contract | Change |
|---|---|
| §3.6 `Asker::Ward { id: ConstituencyId }` | `Asker::Constituency { id: ConstituencyId }` (a constituency is tiered already; a regional district's voters can then ask) |
| §3.6 `Asker::Outlet { id: u32 }` | unchanged (typed `OutletId` recommended with M6 §6 item 12) |
| §3.6 new `Effect::Approval { scope, delta }`, "a neighbourhood, the ward or the town" | `scope: Area` with `Area::{ Hood { id }, Constituency { id }, In { at: Jurisdiction } }` (R1, `world.rs`) |
| §3.2 "Generation (weekly, the asks system, player's town only)" | "for `state.focus()`". The asker must live in, or hold a post in, a focus jurisdiction |
| §3.5 promise deadline "the next budget year's end or a term" | "the end of the budget year or term **of the post the player is running for**" (read from the post's jurisdiction and term, §7.4) |
| §3.5 `promises_this_campaign` | keyed by the race's `Post` (`Option<Post>` beside the count), so a promise belongs to one race |
| §3.8 `ask.vote_with_me` "Councillor with `seek_backing`"; `ask.second_my_bill` | cast by "a colleague: a holder of a post in the same body" (R1's `Cast::Colleague` reading the body), not by the word councillor |
| E7 `AskerView` "the ward" | `AskerView::Constituency { name_key, number }` |

### 3.3 Director, storylines and the press (E2, E4, E5; `ENGAGEMENT.md` §4)

- **E2:** no key changes. Chapters stay `session` and `campaign`; the session's budget year is the budget year of the jurisdiction of the player's main post (`player::posts(state).first()`), not a hard-coded local one. `crisis_cap` stays per town (crises are town state).
- **E4:** `npc_deeds: Vec<NpcDeed>` "the player's town only": add `at: Jurisdiction` to `NpcDeed` and filter by `state.focus()`, so M7 widens the filter without a migration. The `npc_did` condition gains an optional `at` selector (`same`, the default).
- **E5:** `StorySource::Work { politician, … }` is keyed by politician: no change.

### 3.4 Credit, endorsements, attributes and the broker (V1, V2, A1, P1; `M6_ENGINE_CONTRACTS.md` §4)

- **V1:** the per-neighbourhood track record is tier-free (neighbourhoods aggregate to any constituency). No change. State that the record term reads the race's own neighbourhoods (`Race.hoods`), so it works for a regional race unchanged.
- **V2:** endorser roles come from occupations and from posts; any role that names an office uses R1's post keys (`"office": "mayor"`), not `Role` matches.
- **A1:** "administration: the sponsor, or the mayor for town programmes" becomes "the sponsor, or the holder of the executive office of the law's jurisdiction" (R1's `executive_of(at)`).
- **P1:** "the mayor counts double" in broker scoring (`characters/town.rs:231`) is replaced by R1; the broker's lobbying targets "members of the body that votes on the bill" (the procedure's `Vote` stage), not `councils[m]`.

### 3.5 Conversations (C2, C4)

- C2's views (`TalkView`, `PersonRefView.can_talk`) carry no office names: no change. C2 owns `protocol/src/{characters,knowledge}.rs`, which R1 must also touch mechanically, so **R1 starts after C2 merges** (§7.8).
- "A colleague: a politician in office in the player's town" (`talk/mod.rs:266`) becomes "a holder of a post whose jurisdiction is a focus jurisdiction" (R1).

### 3.6 Secrets (E9; `ENGAGEMENT.md` §5)

- E9 already bumps the save for `HiddenFact`. In the same bump, replace `HiddenFact.town: Option<u32>` (`narrative/mod.rs:459-461`) with `at: Option<Jurisdiction>` (migration: `town: m` → `{ "type": "municipality", "id": m }`), and adopt the typed `HiddenId` the brief already recommends.
- `secrets.max_live_per_town` → `max_live_per_jurisdiction`; seeding keyed by `[month, kind, at.tier_index(), at.raw(), cast]`.
- `secret.party_challenge` "deselect the mayor or the player" → "a post holder of the faction's party in the jurisdiction".

### 3.7 Walking rewards (E10, E11)

- **Keep `Curiosity.town: MunicipalityId`.** Curiosities are physical things in a town; the town is the right key. Not everything should be generalised.
- E10 already plans seeding "when the player enters a new zone (M7)". Make the seed function take a `MunicipalityId` argument rather than reading `player.municipality`.

### 3.8 Knowledge (K-series, done)

- No change needed now. `Zone::Town` covers a regional capital (it is a town). M7 adds `Zone::Region` and region subjects as new variants (additive).
- K1's `GrantSubject::Council` keeps its spelling; R1 redefines it as "the members and holders of the zone town's local-tier posts" (§7.2).

### 3.9 Not yet briefed: the draft of three, detours, endings

- **Draft of three (DEC-036, DEC-077 §2).** A background's starting situation names a constitution post and a selector, not a code path:
  ```jsonc
  { "id": "start.incumbent_marginal",
    "post": { "body": "council" },                  // or { "office": "mayor" }, or null for a challenger
    "constituency": "marginal",                      // marginal | safe | any (selectors, code)
    "running_for": { "office": "mayor" },            // optional: a challenger's target
    "knowledge": "start.sitting_councillor" }        // a K1 profile
  ```
  `place_player` (`mandate-worldgen/src/lib.rs:942-1010`) becomes `seat_player(world, politics, start)`. This is the item that makes "start as a challenger for mayor" content.
- **Detours (DEC-020, 1–2 in M6).** Do **not** add `Role::UnionLeader`. `Role` is for constitution posts only. An organisation post is a separate field (`Politician.affiliation: Option<OrgPost>`, added when organisations become actors), so a union leader can also be out of office, or (later) hold a seat.
- **Act 1 endings and Legacy.** Build as a registry (§4, row 11), with conditions that read posts and tiers ("reached the regional tier", "held the executive office of a town for two terms"), so "the next rung" teaser and M7/M8 endings are content.

---

## 4. What is data, what is code, and what to change

### 4.1 Already data-driven (registries in `content/base/**`, schemas generated from Rust types, DEC-053)

| Registry | File | Decision |
|---|---|---|
| Constitution: tiers, bodies, offices, terms, calendars, powers, procedures | `constitutions/federal_presidential.jsonc` | DEC-017 |
| Sectors (drivers are code) | `sectors.jsonc` | DEC-075 stage 1 |
| Attitudes, communities | `attitudes.jsonc`, `communities.jsonc` | DEC-075 stages 2–3 |
| Concerns, scenarios | `concerns.jsonc`, `scenarios.jsonc` | DEC-078 §3–4 |
| Balance numbers: politics, opinion, actions, council, career, NPC, economy, tuning | `politics`, `opinion`, `actions`, `council`, `career`, `npc`, `economy`, `tuning.jsonc` | DEC-078 §1 |
| Place kinds: actions, services, fallbacks, layout blocks | `places.jsonc` | DEC-079, DEC-090 D4 |
| Town layout v2: district types, terrain features, connectors | `layout/*.jsonc` | DEC-090 D4 |
| Policy templates (mechanisms are code) | `policies/*.jsonc` | DEC-024 |
| Traits, occupations, crises, characters | `traits`, `occupations`, `crises`, `characters.jsonc` | DEC-031, DEC-077 |
| Director pacing, storyteller profiles | `director.jsonc`, `storytellers.jsonc` | DEC-033 |
| Dilemmas, goals, storylines, scenes, overheard talk | `dilemmas`, `goals`, `storylines`, `scenes`, `overheard.jsonc` | DEC-032, DEC-077 §4 |
| Outlets, media, justice, shady acts | `outlets`, `media`, `justice.jsonc` | DEC-034 |
| Knowledge profiles, conversations | `knowledge.jsonc`, `conversations/*.jsonc` | DEC-091, DEC-093 |
| All text | `locale/en/*.ftl` | DEC-054 |

Planned as data by accepted designs: policy families (DEC-092), asks and promises, secret kinds, curiosity kinds (DEC-097), map layers (DEC-089).

### 4.2 Candidates

| # | Candidate | Current state | Value of data | Cost | Recommendation |
|---|---|---|---|---|---|
| 1 | **Actions and their effects** | Kinds are code (`actions.rs:21-59` `Action`, 11 kinds; handlers through `:1649`); numbers in `actions.jsonc`, `tuning.jsonc` `action_costs`, `career.jsonc`; places in `places.jsonc`. Who may act is code (`actions.rs:1348-1351` "approve or veto to vote", `:1523` `requires`) | Moderate. New verbs need code anyway (DEC-078 §7). M7/M8 add verbs (appoint, confirm, travel, debate) whose *availability* depends on posts and powers | M | **Before M7:** an action availability table in content (`requires_power`, `phases`, `tiers`), read by the action gate. Verbs and effects stay code. Do the power part in R1 |
| 2 | **Office powers and what each enables** | Powers per body/office are data; `Power` (`types.rs:146-160`, 8 kinds) is a closed enum; what a power enables is code; gating matches on `Role` (S5) | High for M7/M8 (governor, cabinet, President), and DEC-017 asks for it | M | **Now (R1):** gate by reading the post's powers. **Before M8:** parameterised powers (`appoint { office }`, `confirm { office }`, `decree`, `dissolve`), each a code extension point |
| 3 | **Election rules and campaign actions** | Calendar per tier, voting rule and constituency in the constitution; only plurality is implemented; campaign numbers in `politics.jsonc`; campaign verbs are code | High: each tier has its own calendar and terms | S–M | **Now:** term per post (R1). **M7:** per-tier cycles. **M9:** tabulators as code extension points; refuse unimplemented rules in validation now |
| 4 | **NPC utility weights and goals** | Data (`politics.ai`, `npc.jsonc`, `goals.jsonc`) | DEC-076 "harder as you climb": NPC competence and budgets per tier | S | **Before M7:** a `by_tier` override block in `npc.jsonc` and `politics.ai` |
| 5 | **Storyteller profiles** | Data (`storytellers.jsonc`) | — | — | **Done** |
| 6 | **Director pacing (ENGAGEMENT cycle)** | Data (`director.jsonc`; E2 adds fields); chapter kinds are code (`narrative/mod.rs:258-263`) | Chapter kinds are mechanisms | — | **Done**; keep chapter kinds in code (**never**) |
| 7 | **Media outlets** | Data (`outlets.jsonc`, scope town/region/nation) | M7/M8 add regional and national outlets as entries | — | **Done** |
| 8 | **Scandal and fact kinds** | Kinds are strings inside `media.jsonc` (`shady_actions`, `npc_acts`, `late_night_*`, `:68-98`) plus Fluent; severity is an enum (right) | Moderate: E9 adds secret kinds; mods add misdeeds | S | **M6, with E9:** one `fact_kinds` registry (kind, severity, evidence, newsworthiness override, Fluent) that shady acts and secret kinds reference |
| 9 | **Hazards and disasters** | Two enums (S33) | M7 needs a registry (DEC-078 §5): heatwave, epidemic | M | **Before M7** (as planned). Content spellings survive, so F1 and G1 work is not wasted |
| 10 | **Economy: sectors, commodities, taxes, budget categories** | Sectors data; commodities by index (S34); `Revenue` and `Category` enums (S32), duplicated in content; treasury shares per tier hard-coded (`finance/mod.rs:96-99`) | M7/M8 need national categories (defence, foreign affairs) and trade (DEC-087) | M (save remap like `sector_keys`) | **Now:** treasury shares into `economy.jsonc` (S). **Before M7:** the registries (DEC-078 §5), with `category_keys` and `revenue_keys` saved for remapping |
| 11 | **Endings and Legacy** | Not built. Today only `RunStatus::Ended { reason: Prison \| Disqualified }` (`player/mod.rs:84-105`) | High: endings span all acts (DEC-006); "next rung" teaser in M6; M7/M8 add endings; mods | S now | **Now (M6):** an `endings.jsonc` registry: conditions on posts, tiers, tracks, age and events; epilogue Fluent keys; Legacy weights. Run-status reasons stay code |
| 12 | **Tutorial** | Not built (M6: "first chapter as a guided chapter") | Low–moderate; it is UI guidance | S | **M6, in the web:** a JSON step list in `web/` (hint, trigger on a view field or command). **Never** engine content: the engine must not know about tutorials |
| 13 | **UI layouts and tabs** | React code | Low. Screens are clearer as code | — | **Never.** But screens render bodies and offices from the view's lists (R2), not from fixed "council" and "mayor" slots |
| 14 | **Town-generation recipes** | Layout v2 registries are data (`layout/*.jsonc`); which places a town has is code (`mandate-worldgen/src/places.rs:34-`) | High for P2 kits (regional capitals, parliament), scenarios and mods | M (keep the same draws, or refresh pinned worlds) | **Before M7:** a place-plan registry by settlement class and capital flag (required, chance, neighbourhood kind, naming pool) |
| 15 | **Interiors** | Pixelkit TS (`tools/pixelkit/art/mandate/*.ts`); interiors planned in P1-7 (`PIXEL_ART_TOOL.md:196`, `:1075`) | High for M7/M8 (assembly, parliament, executive residence) and for DEC-088 enterable places (where people stand) | S if done first | **Now, in P1:** room recipes as JSON (a grid of part ids, seats, standing spots, interaction points); parts stay drawn code |
| 16 | **Character appearance** | Not in the engine yet (P1: "appearance genome in the engine") | High: dress by occupation, class and tier (suits at the capitol) | S if done first | **Now, in P1:** genome traits and per-occupation/tier palettes as a content registry; the engine draws from it with keyed RNG |
| 17 | **Difficulty, settings, quick mode** | Storyteller profiles are the difficulty lever (data); quick mode and settings are web preferences (`web/src/state/devicePrefs.ts`) | Moderate (DEC-076 gates in M6) | S | **M6:** difficulty presets as data when DEC-076 targets become gates (a preset = storyteller + NPC competence scale + poll noise). Settings stay code (**never**) |
| 18 | **Bot strategies** | Code (`mandate-cli/src/bots.rs:55-66`, `Taste` table `:305-360`) | Low: strategies are logic; the weights are lab-only | — | **Never** for logic. Weights may move to a lab file if tuning needs `--set` on them |
| 19 | **Lab gates and thresholds** | Rust constants (`bots.rs:1012-1014`, `:1236-1257`); ENGAGEMENT §7.2 adds more | Moderate: per-tier gates multiply in M7 | S | **Before M7, optional:** a lab config (`crates/mandate-cli/gates.jsonc`, not in the game pack) so BALANCE_LOG changes are data diffs |
| 20 | **Constitution gaps** | §7.6 | High (M8) | M | **R1** small fields; **before M8** appointments, cabinets, staggered terms |
| 21 | **Career ladder and nomination gates** | "Standing ≥ 40 for mayor" in code twice (S8) | High: every rung | S | **Now (R1):** `career.nomination` per post and the constitution's `career` block (§7.5) |

### 4.3 No scripting language

The authored logic in this game is "when these facts hold, offer this, with these effects". That is what the condition and effect vocabularies already do (narrative `Condition`, `SituationCondition`, `HeardCondition`, `EffectDef`). A script engine would cost determinism review (DEC-041), WASM size, schema validation and lab coverage, and it would let content do things the "Why?" view cannot explain (DEC-051). The real problem is **three overlapping condition languages** (M6_ENGINE_CONTRACTS §6 items 10 and 16). Unify them before the content volume of E3 (22 dilemmas) and E8 (16 asks) lands on the narrative one: that is the cheap-now step, not scripting.

---

## 5. Brief rule and proposed tasks

### 5.1 A rule for every future brief (add to WORKFLOW §2's template under DECISIONS, or to AGENTS.md "Hard rules")

```text
TIER-GENERIC: State that belongs to "a town, a region or the nation" is keyed by
  `Jurisdiction`, never by `MunicipalityId` or a raw u32 (keep `MunicipalityId`
  for physical things in a town: places, neighbourhoods, curiosities).
  Offices, seats, powers, terms and calendars are read from the constitution;
  no Rust code or content names an office or body ("mayor", "council") except
  through constitution ids. New code does not read `player.municipality` or
  `player.ward`: it asks `player::home(state)`, `player::posts(state)` or
  `state.focus()`. A brief that cannot follow this says why in DECISIONS.
```

### 5.2 Tasks for M6

**R1 and R2** generalise offices now (§7.8 has the full briefs). Everything else in this audit goes into existing briefs (§3) or before M7 (§4).

| # | Task | After | OWNS | Runs beside |
|---|---|---|---|---|
| **R1** | Constitution roles, engine and save (§7) | C2 and N1 merged | §7.8 | E1 (if it keeps to the `Canvass` arm), C3-style content tasks, K5, web tasks |
| **R2** | Constitution roles, protocol and web (§7) | R1, C4 | §7.8 | F1, E6 |
| **X1** | Lab check: plan a regional capital's layout v2 and record the time and any `check.ts` problems; no fix | — | `web/src/hub/layoutV2/*.test.ts` (a new test only), `docs/development/BALANCE_LOG.md` or `TOWN_GENERATION.md` (one note) | anything |

Small items to fold into running briefs instead of new tasks: treasury shares into `economy.jsonc` (with any finance task, e.g. F2b); validation refuses voting rules other than plurality (with R1); the `fact_kinds` registry (with E9).

### 5.3 Before M7 (not M6)

The DEC-078 §5 registries (budget categories, revenue types, hazards, commodities); the place-plan registry; per-tier NPC overrides; per-tier election cycles; converting the remaining `player.municipality` reads to `focus()`; an action availability table; layout v2 performance for cities (X1's result decides the size).

---

## 6. Proposed DECISIONS.md notes (for the lead; DECISIONS.md is not edited here)

1. **New DEC (number by the lead): offices come from the constitution.** Owner, 2026-11-19: "The offices should actually be integrated as part of the constitution as well." Decided. Politicians hold posts (`Role::Seat { body, at, constituency }`, `Role::Office { office, at }`, `Role::Private { home }`) whose ids, powers, terms, limits, selection and constituency are read from the constitution; elections, filing, power checks, law-making stages, career steps and labels follow. Implements DEC-017's consequence "systems must query powers and procedures rather than branch on archetype names". Tasks R1 and R2 (§7.8). Schema additions in §7.6.
2. **New DEC (or a DEC-075 addendum): tier-generic keys.** `Jurisdiction` and `Area` (§7.1) key all state that belongs to a government's territory; the §5.1 brief rule. Records the amendments in §3 as accepted changes to the F-, E-, V- and P-series contracts.
3. **DEC-017 implementation note.** The tabulator implements plurality only; validation refuses other voting rules until their tabulators exist. Body and office ids are unique across a constitution's tiers. Procedure ids `ordinance` and `budget` are a contract with the engine.
4. **DEC-078 decision 5 addendum.** Add to "before national play": the worldgen place plan as a registry; treasury budget shares in content (now); per-tier NPC competence overrides (DEC-076 "harder as you climb"). Note that `Category`/`BudgetCategory` and `HazardKind`/`HazardKey` are duplicated today and become one registry each.
5. **DEC-036 note.** A background's start names a constitution post and selectors (§3.9), so varied openings (DEC-077 §2) are content.
6. **DEC-020 note.** Detour roles are organisation posts, separate from `Role` (constitution posts only).
7. **DEC-006 / DEC-039 note.** Endings and Legacy are a content registry from their first build in M6.
8. **Record: no scripting language** (§4.3); unify the three condition languages instead, before E3 and E8 add content.

---

## 7. Offices from the constitution (owner requirement, 2026-11-19)

### 7.1 Shared keys (`crates/mandate-sim/src/world.rs`)

```rust
/// A government's territory at any tier (DEC-017). The key for anything that
/// belongs to "a town, a region or the nation". Physical things in a town
/// (places, neighbourhoods, curiosities) keep `MunicipalityId`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Jurisdiction {
    Municipality { id: MunicipalityId },
    Region { id: RegionId },
    Nation,
}

impl Jurisdiction {
    pub fn tier(self) -> mandate_content::TierKey;
    /// From the (tier, raw id) pairs stored today (`Constituency`, `PolicyInstance`).
    pub fn of(tier: TierKey, raw: u32) -> Self;
    pub fn raw(self) -> u32;              // 0 for Nation, as `Constituency.jurisdiction`
    pub fn tier_index(self) -> u64;       // for keyed RNG
    pub fn hoods(self, w: &World) -> Vec<usize>;
    pub fn contains_hood(self, w: &World, h: NeighborhoodId) -> bool;
    pub fn parent(self, w: &World) -> Option<Jurisdiction>;
}

/// Where an effect or a scope applies.
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Area {
    Hood { id: NeighborhoodId },
    Constituency { id: ConstituencyId },
    In { at: Jurisdiction },
}
```

Also: `impl From<world::Tier> for TierKey` (no fourth tier enum; `finance::TierKind` maps the same way), `Constituency::at()`, `PolicyInstance::at()`. `media::Scope` keeps its saved shape and gains `Scope::at()`.

### 7.2 (a) Every place that hard-codes an office or a body

| Where | What | Cost |
|---|---|---|
| `mandate-sim/src/politics/mod.rs:25-38` | `Role` enum `[save]` | L (the change itself) |
| `politics/mod.rs:86-93` | `Politician::municipality()` from the role | S |
| `politics/mod.rs:95-108`, `:286-325` | `Council { members, mayor }`, `councils` by municipality; `init` builds wards and a mayor for each town `[save]` | M |
| `politics/mod.rs:171-181` | removal from office sets `Role::Private` | S |
| `politics/elections.rs:27-46` | `Seat` `[save]` | M |
| `politics/elections.rs:141-151` | `races_in`, `latest_result` by seat | S |
| `politics/elections.rs:156-166`, `:288-293` | calendar and term from the first body; term limit from the first office | S |
| `politics/elections.rs:310-500` | candidacy: mayor first, then wards; `Filing::Mayor`/`Council`; Standing ≥ 40 (`:357-359`); recruits `Role::Private` (`:465`, `:540`) | M |
| `politics/elections.rs:593-594` | `in_office_here` by municipality | S |
| `politics/elections.rs:1318-1381` | seating winners; leaving office | M |
| `politics/elections.rs:1388-1400` | events: `RaceDecided { mayor }` | S |
| `politics/elections.rs:1512-1517` | `Filing` `[save]` | S |
| `politics/elections.rs:1523-1560` | `appoint_replacement` by role | S |
| `actions.rs:166-215` | `player_powers`, `powers_for` by role, local tier | M |
| `actions.rs:578-580` | outcome keys `outcome-filed-council` / `-mayor` / `-retire` | S |
| `actions.rs:1348-1351` | voting needs `ApproveOrdinance` or `Veto` | S |
| `actions.rs:1644-1649` | `ward_of` from `Role::Councillor` | S |
| `npc.rs:115-130` | `holds_power` by role | S |
| `npc.rs:296` | ward support by `Role::Councillor` | S |
| `policy/mod.rs:488`, `:511` | earmarks match `Role::Councillor { ward }` | S |
| `policy/mod.rs:572-605`, `:647-660` | council leans and the mayor's lean from `councils` | M |
| `policy/mod.rs:725-760`, `:805-812` | stages ignore `body`/`office`; the signer is the mayor | M |
| `characters/town.rs:231-233` | broker: "the mayor counts double" | S |
| `characters/town.rs:252-264`, `:311-315` | the rival's seat from `Filing` and `Role` | S |
| `characters/knows.rs:30-32`; `characters/mod.rs:1430-1432` | role keys `councillor`, `mayor`, `private` | S |
| `characters/mod.rs:196` | memory `Subject::Seat` `[save]` | S |
| `narrative/cast.rs:197-207`, `:301-305`, `:324`, `:647-653` | `Cast::Mayor`, colleagues, `OfficeRole` matching | S |
| `narrative/resolve.rs:388` | a rider's ward from `Role::Councillor` | S |
| `talk/context.rs:208-209` | the mayor is at the town hall | S |
| `talk/mod.rs:266` | colleagues: in office in the player's town | S |
| `player/knowledge.rs:827` (and `:517`, `:531`) | "every councillor and the mayor" | S |
| `mandate-content/src/narrative.rs:202-203`, `:258-265`, `:404` | `Cast::Mayor`, `OfficeRole`, `Condition::Office` (schema) | S |
| `mandate-content/src/conversation.rs:22`, `:121`, `:417`, `:563-564` | `POLITICIAN_ROLES` (schema docs and validation) | S |
| `mandate-content/src/types.rs:501` | `SituationCondition::Occupation` accepts `councillor`, `mayor` | S |
| `mandate-content/src/knowledge.rs:82-83` | `GrantSubject::Council` | S |
| `mandate-content/src/types.rs:1929-1960` | `CouncilParams.mayor*` | S (rename optional) |
| `content/base/dilemmas.jsonc:164-212` | `{ "type": "office", "role": "councillor" }`; cast role `mayor` | S |
| `content/base/conversations/topics.jsonc:25`; `params.jsonc:35`; `knowledge.jsonc:22-63` | role and grant words | S (kept as aliases) |
| `content/base/locale/en/base.ftl:316-318`, `:469-473`, `:541-550`; `knowledge.ftl:36-37` | `role-*`, `mayor-*`, `ui-file-*`, `ui-race-*`, `rival-seat-*` | S |
| `mandate-worldgen/src/lib.rs:942-1010` | the player starts as `Role::Councillor` | S |
| `mandate-protocol/src/lib.rs:212-216` | `FilingDto` `[protocol]` | S |
| `mandate-protocol/src/office.rs:115-124`, `:305`, `:392-400`, `:475-477`, `:546-547`, `:582-699`, `:869-871`, `:934-935` | player role, council and mayor lists, the mayor's lean `[protocol]` | M |
| `mandate-protocol/src/campaign.rs:50-61`, `:105-108`, `:128-142`, `:329-335` | seats as `mayor`/`ward`, filing, mayoral nomination `[protocol]` | M |
| `mandate-protocol/src/characters.rs:229-237`, `:304-308`; `knowledge.rs:576-577`; `places.rs:835-841` | role labels, seat subjects, rival seat `[protocol]` | S |
| `mandate-wasm/src/session.rs:375-377` | `FilingDto` → `Filing` | S |
| `mandate-cli/src/bots.rs:191`, `:1633`, `:1663` | `councils[...]` | S |
| `web/src/components/CampaignCards.tsx:10`, `:18-22`, `:58`, `:193` | filing buttons and race labels | S |
| `web/src/components/{TownCard.tsx:36, BillCard.tsx:92, Chamber.tsx:118,175, ActionForm.tsx:39, PeopleList.tsx:32}` | seat labels, the mayor's lean, member lists | S each |

### 7.3 (b) The target shape

**Role and Post** (`mandate-sim/src/politics/mod.rs`):

```rust
/// Something a politician can hold or run for (constitution ids, DEC-017).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Post {
    /// A seat in a body. `constituency` is the district for `Constituency::Ward`
    /// bodies, `None` for at-large bodies.
    Seat { body: ContentKey, at: Jurisdiction, constituency: Option<ConstituencyId> },
    /// A single-holder office.
    Office { office: ContentKey, at: Jurisdiction },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Role {
    Seat { body: ContentKey, at: Jurisdiction, constituency: Option<ConstituencyId> },
    Office { office: ContentKey, at: Jurisdiction },
    /// Out of office; `home` is the town they live in.
    Private { home: MunicipalityId },
}
impl Role { pub fn post(&self) -> Option<Post>; pub fn in_office(&self) -> bool; }
impl From<Post> for Role { … }
```

- `elections::Seat` is replaced by `Post` (`Race.seat`, `RaceResult.seat`, memory `Subject::Seat { seat: Post }`). The saved JSON of a race's seat and of a role are the same shape.
- `Filing` becomes `enum Filing { Stand { post: Post }, Retire }`; `None` still means "defend what I hold".
- `Role`, `Post` and memory `Subject` lose `Copy` (a `ContentKey` is a `String`). About 50 `.role` reads take `&` or `.clone()`. If that churn matters, R1 may use a small `Copy` key type that serialises as a string; the saved shape is the same either way.

**Storage** replaces `councils`:

```rust
pub struct BodySeats { pub body: ContentKey, pub at: Jurisdiction, pub members: Vec<PoliticianId> } // constituency order
pub struct OfficeHolder { pub office: ContentKey, pub at: Jurisdiction, pub holder: Option<PoliticianId> }
// PoliticsState: bodies: Vec<BodySeats>, offices: Vec<OfficeHolder>  (sorted by (at, id))
pub fn members_of(st: &PoliticsState, body: &str, at: Jurisdiction) -> &[PoliticianId];
pub fn holder_of(st: &PoliticsState, office: &str, at: Jurisdiction) -> Option<PoliticianId>;
/// The body of the `ordinance` procedure's first `Vote` stage, and the office
/// of its `Sign` stage: "the legislature" and "the executive", by procedure, never by name.
pub fn legislature(c: &ConstitutionDef, tier: TierKey) -> Option<&BodyDef>;
pub fn executive(c: &ConstitutionDef, tier: TierKey) -> Option<&OfficeDef>;
```

In M6 only local-tier posts are instantiated (as today). M7 instantiates regional ones with no shape change.

**Powers gate actions by reading the constitution.** `powers_of(content, constitution, &Post) -> &[Power]` returns `tier(post.at.tier()).body(id).powers` or `.office(id).powers`. `holds_power_at(state, content, who, power, at) -> bool` is true when one of `who`'s posts is at `at` (or, for a power that reaches down, at an ancestor, M7) and lists `power`. `actions::player_powers`, `npc::holds_power`, the voting check (`actions.rs:1348-1351`), drafting (`:1523`) and DEC-092's `requires` all call it with the bill's `at()`. No `match` on `Role` remains outside `politics`.

**Law-making follows the procedure.** For each stage: `Vote { body, rule }` tallies `members_of(body, inst.at())`; `Sign { office, override_body, override_rule }` asks `holder_of(office, inst.at())` with the signing lean (today's `mayor_lean`, renamed `signer_lean`), and the override tallies `override_body`. A bicameral national procedure then works without new code. The memory "signed/vetoed my bill" names the holder, not "the mayor".

**Labels and content references.** `BodyDef` gains `member_key` (`"councillor"`: Fluent `role-councillor`, and the word content uses); an office's own id is its word (`"mayor"`). So `topics.jsonc`'s `"roles": ["councillor", "mayor"]`, `dilemmas.jsonc`'s `"role": "councillor"` and K1's `council` grant keep their spelling, now resolved through the constitution. `Cast::Mayor` becomes `Cast::Office { office: Option<String> }` (default: the executive of the town), `OfficeRole` becomes `{ "type": "office", "posts": ["councillor"] }` with `any` and `none`; the old forms parse as aliases. Validation: every role word in content is a `member_key`, an office id, or an occupation.

### 7.4 Elections and terms per body and office

- A post's term is its own `term_years`; its term limit its own `term_limit`. `local_calendar` (`elections.rs:156-166`) becomes `calendar_for(content, constitution, &Post) -> (ElectionCalendarDef, term)`: the post's own calendar if it has one (new optional field, §7.6), else its tier's, with the post's own term. This fixes the latent bug where the national tier would schedule the President on the House's 2-year term.
- `open_campaign` iterates the tier's elected posts (`Selection::Election`) for every jurisdiction of that tier: one race per constituency for a `ward` body, one at-large race for an office (and, from §7.6, `seats` at-large races for an at-large body).
- `ElectionState` keeps its single cycle in M6 (only the local tier runs). M7 adds `#[serde(default)] cycles: Vec<Cycle { tier, next_day, campaign_open, races }>` beside it; additive, no migration.
- **Nomination gates are data.** `career.jsonc` gains `"nomination": { "mayor": { "min_standing": 40 } }` keyed by office id or `member_key`, default none. `elections.rs:357-359` and `protocol/src/campaign.rs:335` read it.
- **Which posts can the player file for** is derived: elected posts whose constituency contains the player's home neighbourhood, in a tier whose election is the next one. In M6 that yields their ward seat and the mayoralty, exactly as today.

### 7.5 The career ladder

- **Derived by default:** posts ordered by tier (local, regional, national), then seats before offices within a tier. "The next rung" is the lowest post above the player's highest.
- **An explicit `career` block is needed** in the constitution, because acts are not tiers (Act 4, the presidency, is one national office; DEC-004) and because DEC-017's validation needs a summit:
  ```jsonc
  "career": {
    "summit": "president",
    "acts": [
      { "id": "act.local",    "tier": "local" },
      { "id": "act.regional", "tier": "regional" },
      { "id": "act.national", "tier": "national", "except": ["president"] },
      { "id": "act.summit",   "posts": ["president"] } ] }
  ```
  Validation (DEC-017 "the player has a viable path to the summit"): a chain of elected (or body-elected) posts from a local post to `summit`, where each step's constituency contains the previous one's. Endings (§4 row 11) and the director's act transitions read `acts`.

### 7.6 What the constitution schema still lacks

| Gap | Needed for | Proposed field | When |
|---|---|---|---|
| A seat count for at-large bodies | Senate | `BodyDef.seats: Option<u16>` (required when `constituency: at_large`) | R1 (schema only) |
| Seats per sub-unit | A senate elected per region | `Constituency::PerUnit { tier, seats }` | before M8 |
| Own calendar for a body or office | House 2 y, President 4 y, Senate 6 y | `calendar: Option<ElectionCalendarDef>` on `BodyDef`/`OfficeDef` | R1 (schema) |
| Staggered terms | Senate thirds | `BodyDef.classes: Option<u8>` | before M8 |
| Where the post works | "Find them at", scenes, travel | `works_at: String` (a place kind) on `BodyDef`/`OfficeDef` | R1 |
| A member's word | Labels and content roles | `BodyDef.member_key: String` | R1 |
| Appointed offices with portfolios, confirmation | Cabinet (M8) | `Selection::Appointment { by, confirmation: Option<String> }` (DEC-017 already names it); `OfficeDef.portfolio: Option<BudgetCategory>` and `count` | before M8 |
| Offices with no election, ex officio | Speaker, majority leader, a lieutenant governor | `Selection::ExOfficio { of }`, `Selection::ElectedByBody` already exists | before M8 |
| Parameterised powers | Appoint, confirm, decree, dissolve | `Power::Appoint { office }` and new variants (code extension points) | before M8 |
| Removal and triggers | Recall, impeachment, no-confidence | DEC-017's `Trigger` | M8–M9 |
| Vacancy rule | A by-election or a party appointment | `BodyDef.vacancy: by_election \| party_appoints` (today code: `appoint_replacement`) | before M7 |
| Competences per tier (policy areas) | DEC-092 §5, M6 contracts §6 item 7 | `TierDef.competences: Vec<BudgetCategory or concern>` | before M7 |
| Leadership inside bodies (whips, speaker) | M8 legislature | `BodyDef.leadership: Vec<OfficeDef>` | before M8 |

### 7.7 Save migration (v17, or v18 if N1 takes it first)

One JSON step, numbered by the lead at merge (M6 §4 rule). The step describes the old format, where `Councillor` and `Mayor` existed only for the base constitution's local tier, so it may name `council` and `mayor`; `Engine::from_state` then checks every post's body and office id against the run's constitution (`missing_references`, refusing the save if one is missing, as for templates).

| Path | Old | New |
|---|---|---|
| `/state/politics/politicians/*/role` | `{"Councillor":{"municipality":m,"ward":w}}` | `{"type":"seat","body":"council","at":{"type":"municipality","id":m},"constituency":w}` |
| | `{"Mayor":{"municipality":m}}` | `{"type":"office","office":"mayor","at":{"type":"municipality","id":m}}` |
| | `{"Private":{"municipality":m}}` | `{"type":"private","home":m}` |
| `/state/politics/councils[m]` | `{ municipality, members, mayor }` | `bodies[]: {"body":"council","at":…m,"members":…}` and `offices[]: {"office":"mayor","at":…m,"holder":…}`; `councils` removed |
| `/state/politics/elections/races/*/seat`, `/results/*/seat` | `{"Council":{…}}`, `{"Mayor":{…}}` | the `Post` JSON as above |
| memories and favours, `subject: {"type":"seat","seat":…}` | old seat | `Post` JSON |
| `/state/player/filing` | `"Council"` / `"Mayor"` / `"Retire"` | `{"type":"stand","post":<the seat of /state/player/ward in /state/player/municipality>}` / `{"type":"stand","post":<mayor of that town>}` / `{"type":"retire"}` |
| events `RaceDecided` | `{ municipality, mayor }` | unchanged; new `#[serde(default)] post: Option<Post>` written from now on |

Acceptance follows DEC-075's pattern: results unchanged. Same seed and commands give the same outcomes (sweep metrics, bot win rates, election results); decoded states differ only in shape and the content hash; v16/v17 fixtures load and continue identically to before.

### 7.8 (c) The M6 tasks

**R1: Constitution roles, engine and save**

```text
OBJECTIVE:   Politicians hold constitution posts; powers, elections, filing,
             law-making stages, nomination gates and role words are read from
             the constitution. No office or body name in Rust outside the save
             migration. Behaviour identical on the base content.
DECISIONS:   DEC-017, DEC-041, DEC-074, DEC-075 (keys in saves), the new DEC of §6.1
CONTRACT:    §7.1 Jurisdiction, Area; §7.3 Post, Role, BodySeats, OfficeHolder,
             members_of, holder_of, legislature, executive, powers_of,
             holds_power_at; §7.4 calendar_for, Filing; player::home(state),
             player::posts(state), State::focus() (returns the home town in M6)
OWNS:        crates/mandate-sim/src/{world.rs, politics/**, actions.rs (power
             checks, File arm, ward_of), npc.rs (holds_power, role matches),
             policy/mod.rs (leans, stages, earmarks, PolicyInstance::at),
             characters/{mod.rs, town.rs, knows.rs} (role matches),
             narrative/{cast.rs, resolve.rs} (role matches only), talk/{context.rs,
             mod.rs} (role matches only), player/{mod.rs, knowledge.rs} (accessors,
             council grant), state.rs (focus), engine.rs (from_state check),
             events.rs (RaceDecided.post)};
             crates/mandate-content/src/{types.rs (constitution section, Occupation
             doc), narrative.rs (Cast::Office, Condition::Office alias),
             conversation.rs (role words), knowledge.rs (council grant doc),
             validate.rs (constitution and role-word sections)};
             content/base/constitutions/**, content/base/career.jsonc (nomination);
             content/schemas/**; crates/mandate-save/** (migration, fixtures);
             crates/mandate-worldgen/src/lib.rs (seat_player);
             crates/mandate-cli/src/bots.rs (mechanical);
             crates/mandate-protocol/src/** (mechanical compile fixes only: DTO
             shapes unchanged); crates/mandate-wasm/src/session.rs (mechanical)
OUT OF SCOPE: DTO shapes and web (R2); per-tier election cycles and regional
             instantiation (M7); appointments (M8)
ACCEPTANCE:  grep finds no `Councillor`/`Mayor` variant, and no "mayor"/"council"
             literal outside the migration and tests; v16/v17 fixtures load and
             continue identically (state compared after normalising shape);
             sweep and bots metrics identical at 16 seeds; determinism (2026 × 80)
             twice and through save/load; native = WASM; a test constitution
             fixture with renamed bodies and offices plays a local election;
             validation refuses an unknown role word, an at-large body without
             seats, and a voting rule other than plurality
TESTS:       unit (powers_of, calendar_for, Post serde), migration, integration
             (an election, a bill with a veto and an override), determinism
DELIVERABLES: code, migration, EXTENDING "Adding an office or body", DATA_MODEL rows
INTEGRATION: runs after C2 and N1 merge; a short freeze on politics/**, policy/mod.rs,
             npc.rs, actions.rs power checks, narrative/cast.rs; before E2, E4,
             E6, F1, V2, P1 start (they would add Role matches). E1 may run beside
             it if E1 stays inside the Canvass arm.
```

**R2: Constitution roles, protocol and web**

```text
OBJECTIVE:   Views describe posts generically; the web renders bodies and offices
             from lists, with labels from the constitution's Fluent keys.
CONTRACT:    PostDto { kind: "seat"|"office", id, at: JurisdictionDto,
             constituency_number, label_key }; PlayerView.posts: Vec<PostDto>
             (role_key kept, derived); OfficeView.bodies: Vec<BodyView { id,
             label_key, members }>, OfficeView.executive: Option<MemberView>
             (council and mayor kept one release as aliases, then removed);
             BillView.signer { office, lean }; RaceView.post: PostDto;
             CampaignView.filing: Option<PostDto> | "retire",
             CampaignView.nominations: Vec<NominationView { post, open, reason_key }>;
             CommandDto File { post: Option<PostDto>, retire: bool } (FilingDto removed)
OWNS:        crates/mandate-protocol/src/{lib.rs, office.rs, campaign.rs,
             characters.rs, knowledge.rs (rival seat), places.rs (seat labels)};
             crates/mandate-wasm/src/session.rs; web/src/engine/protocol/** (generated);
             web/src/components/{CampaignCards, TownCard, BillCard, Chamber,
             ActionForm, PeopleList}.tsx and their tests; content/base/locale/en/{base,
             knowledge}.ftl (role, race and filing keys)
ACCEPTANCE:  Rust ⇄ TS round-trip; Playwright: file for mayor, see the race,
             vote, see the signer's lean, on phone and desktop; no "mayor" or
             "council" literal in web/src outside Fluent keys
INTEGRATION: after R1 and after C4 (both edit web/src); beside F1 and E6
```

**Why now:** today there are two offices, 75 `Role::`/`Seat::` matches and 30 `councils` reads. Every M6 task still to come adds more: E6–E8 cast councillors as askers, V2 and P1 read the mayor, the draft of three seats the player, Act 1 endings read offices. Doing R1 before them costs one migration of three small JSON shapes. Doing it in M7 costs the same rewrite over a larger code base, plus a migration that also has to touch asks, endorsements, the broker's history and endings.
