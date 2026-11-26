# Doors: households, the doorstep and a canvass as a street (B3)

**Date:** 2026-11-26
**Status:** built (DEC-114 §2, `BEING_THERE.md` §3 and §10.2's B3 row)

**Door slots.** Each neighbourhood has door slots from its residents (`characters.jsonc` `households`: one per 150, within 16–60; `characters::households::doors`). The street maps slot *k* to the *k*-th standing house of the neighbourhood by the plan's fill rank (`hub/townScene.ts` `doorPlots`, reading the plan's ranks; the player's own house has none; a boarded or derelict house keeps its door), so a door is the same house every year. In quick mode the same slots are rows (`PlacePanelView.doors`, `StreetDoorsView`): the doors the engine would knock next, then the households who remember a call, and "Every door on these streets".

**Households on demand** (`mandate-sim/src/characters/households.rs`). Never stored: a pure function of (seed, neighbourhood, door, generation) and the live state, each part its own keyed draw (`KEY_HOUSEHOLD` and a part tag), so a family keeps its name whatever else moves. Who answers and their age (the neighbourhood's cohorts by adults), adults and children, owner or tenant (by class), where the earner works (retired past working age; between jobs by the town's class employment; else a running plant of the town by its jobs, or a sector by its jobs), what worries them (the neighbourhood's felt worries weighted by their cohort's lean), what they think of the player (aware by the cohort's awareness; the cohort's approval plus a keyed spread), and an empty home by the town's vacancy. `generation` is 0 until households move (M7). Whether anyone is in is a keyed draw on the door and the day.

**The doorstep** (`Action::Doorstep { neighborhood, door, choice }`, `actions/ground.rs`; `QueryDto::Door` → `DoorView` in `mandate-protocol/src/doors.rs`). Who comes to the door and their opener (work, home, their view of the player, or "You came round in March about the rent" once met), then one of three: *Listen* (their worry; two doors of a street naming one worry teach the neighbourhood's worries, `known`), *Make the case* on a worry the player knows of (their own worry lands strong, else the neighbourhood's fit as the formula reads it), *Ask* for their vote in a campaign or a poster otherwise (`ask_base + ask_view × view`, keyed). The reply is the action's outcome (`door-reply-*`). The web: a house's door on the street (walked to, or Enter at it) and the street's rows open one door sheet (`components/doors/`).

**A canvass is a street.** The first knock spends the canvass's two slots and opens the street (`Households.street`); up to `street_doors` (4) doors; the fourth finishes it, as `Canvass` on that neighbourhood does sooner, and as anything else the player does or the turn's end does (`actions::close_street`, in `actions::apply` and `Engine::end_one_turn`). Finishing applies today's formula once (`canvass_effect`, the old body unchanged), with the worry argued at most doors as the issue (ties: the first). The doors add no approval: knowledge, memory and the ask. "Canvass the street" (`Canvass` with no street open) is the whole street in one tap: the engine chooses the doors (not called on lately first, from a keyed start), makes the case on the issue at each (or listens) and finishes with the player's issue. Bots and stand-ins canvass that way, unchanged.

**What is remembered.** A `DoorstepRecord` per household met, in the characters domain, naming the player's character: the day, how it went (warm, cool, not in), what was said, the worry, a promise (kept through later calls), the speaker. Sixteen a neighbourhood (the oldest forgotten), forgotten after a year. The doorstep ask is still one roll a canvass (`asks::offers::doorstep_roll`): when it comes up (`doorstep_due`), the street promotes the answerer of one household it knocked on to a Named resident (`households::promote`, origin `(2<<32 | hood, door, generation)`), names them on the record, and `from_deeds` casts them first, then the neighbourhood's residents.

**Save:** v35 (`doorsteps`: nothing to rewrite; no records and no street in an older save). The branch steps v33 → v34 the same way; renumber on merge.

**Bots** (`bots --seeds 16`): every enforced gate holds. ladder_rote mixed share 0.720 → 0.726; ladder_learned explorer − mixed 0.030 → 0.024 (margin 0.01); proxy_not_optimal directed 0.647 → 0.647, informed median 0.750 → 0.751; corrupt −0.0625 unchanged. Report-only `knowledge_matters` and `skill_gap` fail as before.

**Departures from §3, the smaller sound option each time:**
1. A street ends with any other command (a talk included) or the turn's end, not only "Finish the street": the slots are spent at the first knock, so the formula must apply.
2. The one tap knocks the engine's doors with records and the doorstep ask, so quick mode loses only the choosing (the agree rule can teach a neighbourhood's worries a canvass sooner).
3. A boarded house on the street and a vacant household need not coincide: vacancy is the engine's keyed draw on the town's vacancy; the dress pass boards by decline rank (B4 can align the cue).
4. The poster is kept on the record and shown on the row; the street does not draw it yet (B4).
5. Owner or tenant is drawn by class (the housing model has no tenure).
6. Notebook neighbourhood rows naming known households, and doorstep records read by later lines (T5), wait for B6.

Left: the poster and boarded-door cues (B4), lines (B6), households moving (M7), Act 2's campaign team and phone bank (§3.5).
