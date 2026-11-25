# Industries and named plants (I0, I1, minimal P1)

**Date:** 2026-11-24
**Status:** built on `claude/bold-brown-shyeug` (DEC-110; design and every correction: `docs/design/INDUSTRIES.md`, "What I0, I1 and P1 built" and "The seam for foreign trade"). Save v28.

## What was done

- **The registry (I0).** `content/base/industries.jsonc`: sixteen industries under manufacturing, resources and agriculture, with types, schema, validation and English names. `markets.jsonc` (one market) and `goods.jsonc` (fourteen goods) beside it; a place kind can be a gateway (`place.port`).
- **The economy (I1).** `economy/industries.rs`: industry state per town, demand as one product of named factors with exponents in content, external demand summed over markets (each industry's index in each market on its own keyed draw), prices read through the goods registry, local supply links, sector totals as sums. `economy/plants.rs`: named plants at the works a town has, the review (short time, a dated notice, a reprieve or the closure), openings from idle labour and from booms, `facts` for arcs and views. The closure roll left the sequential economy stream.
- **Spine.** `PlantShortTime`, `PlantThreatened`, `PlantReprieved`, `PlantOpened`, `IndustryShock`; `PlantClosure` names its industry, plant, factor and upstream; `Thread::Plant`; pressures `ExternalDemand { market, good }`, `HomeDemand`, `InputCosts`, `Supply`, `Water`, `SlackLabour`.
- **Scheduler.** `Effect::PlaceStatus` (a place closes or reopens where it stands), applied to the world as the day ends; `Effect::PlantRelief`, applied by the economy.
- **Worldgen.** A town's split sectors are shared among industries by geography, gateways, class, the region's tradition and a town draw (`industries::scores`, in the sim so a save can use it); one plant a works place, named in parts.
- **Save v28.** An older save gets industries, markets and plants as it loads, by the routine a new world uses; sector totals are unchanged. Fixtures: `v27_town.mndt` (written by 12bfe17), and v25 and v26 load through.
- **Lab and audit.** The planted closure names a plant; `--industry-shock <town|all>:<industry>:<factor>:<months>`; `audit stuck` has an `industries` block; `audit policy` has per-industry job rows.
- **P1, minimal.** `PlantView` on the place panel and in the statistics desk's town data; plant events have text keys with the plant's name parts; a named closure's headline names the plant.

## What is left

I2 (several works per town, layout), I3 (new works on a plot; people at a reopened works), I4 (industry and plant targets for laws; `plant_support`), I5 (town character, map layers, knowledge gating of exact jobs and the why line), I7 (`audit industries`; what awaits the wave's tuning pass is listed in `INDUSTRIES.md` and the balance entry), I8 (stocks drawn by the industries that name them), I9, C1's talk and overheard lines, C2. Only closures make the paper; the notice, reprieve and opening headlines are written but not yet printed. Every industry's `export_share` is 1.0: the domestic part of demand is built and carries no weight until a balance pass.

`mandate-wasm/tests/town.rs` (`closed_places_are_kept_and_offer_nothing`) assumed a new world has no closed place; it now reads the open places, because history shuts works before play begins.
