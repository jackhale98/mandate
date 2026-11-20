# Mandate — UI Specification (draft, D12)

**Status:** Proposed. IDs refer to `DECISIONS.md` (DEC-047 to DEC-051).

## 1. Screen map

```text
Title ── Runs (concurrent run slots, DEC-008) ── Hall of Legacies (DEC-039) ── Settings
  │
  └─ New Game
       ├─ World setup: seed, size, constitution (or random), storyteller, difficulty (DEC-038),
       │    "Start with local knowledge" (DEC-091: off by default; "You grew up here: start knowing
       │    the town's people and worries, and who really runs things")
       ├─ Generation & burn-in progress: world visibly forming; history ticker (DEC-022)
       ├─ Constitution card (DEC-007)
       └─ Candidate draft: 3 cards, rerolls (DEC-036)
              │
              ▼
        HUB (primary screen, Canvas 2D scene, DEC-009, DEC-080, DEC-088 "Main screen"): the walkable
          street of the player's town, full screen (phone and desktop), opened when a run starts and
          when returning to the town. Tap a building to walk there; arriving opens its place panel
          Bar (one slim row): Map (a full-screen overlay with Town | Country at its top, the last
               choice remembered per device: the town map, DEC-086, where tapping a place closes it
               and walks there in the street; or the national map, DEC-021, with its layers, "You"
               and a town's panel on tap; the pocket's Map link opens it on Country) · date, turn, slots ·
               Pocket (badges: decisions, alerts, new notebook items) · End Turn
          Where you are: a translucent label at the top of the street (neighbourhood · place),
               following the neighbourhood underfoot and naming the place on arrival; it never
               takes taps and announces only a change of neighbourhood
          Pocket (bottom sheet on phones with its tabs at the top; a side panel on desktop that can
               stay docked open beside the town, the street resizing to fit). Tabs, remembered per device:
            ├─ Today → slots, money, Mandate, "You learned N new things · Open the notebook" (when
            │          something is new), alerts, decisions waiting, legal, Agenda (auto-routed
            │          actions), your bills and votes, next election, tracks
            ├─ Town → the town card (industry, crisis, power broker, rival and their plan, priorities),
            │          headline numbers, neighbourhoods and their worries, news; gated by knowledge
            │          (DEC-091: placeholders and leads where something is not known yet)
            ├─ Places → every place in town as a button, by neighbourhood, reachable without walking
            │          (the accessible path, DEC-051, and quick mode's future home)
            ├─ Notebook → what you know about the town (DEC-091, K5), organised for finding things,
            │          not as a log. Its own tabs, styled like the pocket's (icon over label):
            │          People (who runs things, the rival's plans, people with their role, what they
            │          care about and, in detail, whom they owe; gossip), Places (hidden places found),
            │          Worries (the whole town, then by neighbourhood), Policies (support, proposals),
            │          Leads (those picked up, newest first, then ways to find out what is not known).
            │          Each entry: title, level as dots and in words (heard / known / detailed; never
            │          colour alone), where and when it was learned ("Canvass a neighbourhood, 1 March"),
            │          a reading's date and "old figures", "How you learned this" when the steps came
            │          from different places, a lead to learn more, and for people a link to their
            │          sheet. Placeholders (dashed) for public things not known yet (the town's and
            │          each neighbourhood's worries, who runs things, the rival's plans, support for
            │          the standard menu; never people or hidden places) with the lead that would
            │          reveal them and "Go there" (auto-routed). New entries come first, marked "New",
            │          with counts on the notebook's tabs; it opens on the first tab with news.
            │          Opening it is seeing it: the engine is told (`notebook_seen`), the badge
            │          clears and stays clear after a reload; the marks stay while you look. A search
            │          box filters the open tab and names the other tabs with matches
            └─ People → who matters in town (council and mayor, broker, rival) and how they stand
                       with you
            plus links: national map, overview, the story so far, save & quit
          Accessibility: a "Skip to Places" link (visible on focus), landmarks (main: the town;
               complementary: the pocket; navigation: the bar), the canvas labelled with a pointer to
               Places; every action works by keyboard and screen reader without the canvas
          Keyboard walking (desktop): the street is a Tab stop (after the skip link, before the bar)
               with a focus ring; while focused, arrows or WASD walk (diagonals too, Shift hurries,
               sliding along walls and round corners), the camera follows, a key cancels a tap's walk;
               at a place's door a hint "Enter: <place>" shows and is announced, and Enter or Space
               goes in (the same command as a tap). Keys do nothing in the pocket, dialogs or inputs
          Battery (DEC-080 note): townsfolk-only frames at 15 fps (the player, camera and gestures at
               full rate); after 20 s without input townsfolk stop and nothing is drawn until the next
               pointer, wheel, key or touch; paused behind the map overlay, election night, a hidden
               page, and a sheet (pocket, panel) covering over 60 % of the street (phones' bottom
               sheets; beside the street on wide screens it stays live). Budget: e2e/street-battery
          Not yet: pressure-track mini-gauges (DEC-006) and staff in the bar
          In-world portals (each also in a global menu):
            ├─ Office desk → Bills & policies · Budget · Agencies & appointments (DEC-024/026)
            ├─ News (paper / TV / feed) → coverage, polls (DEC-029/034)
            ├─ Map room → national / regional map with overlays (DEC-021): a layer picker (terrain, unemployment; more layers proposed in DEC-089)
            ├─ Phone / contacts → relationships, favors, requests (DEC-031/032)
            ├─ Campaign HQ (campaign phase) → strategy, spending, canvassing
            └─ Places / people → conversations & located actions (DEC-035)
          Overlays: Conversation · Staged scene (DEC-009) · "Why?" breakdown (DEC-051)
              │
              ▼
        Turn flow: Briefing (start of turn) → actions → End Turn → Developments summary
        Chapter flow: chapter resolution (election night, session record, budget passed)
        Act flow: act epilogue → Term in review (directives, time skip, interrupts, DEC-012)
                  → Homecoming / new hub (DEC-009 addendum)
        Run end: terminal or summit scene → Legacy epilogue → unlocks (DEC-006/039)
```

## 2. Layout principles
- **One primary screen (the hub).** Everything else is a panel, overlay, or flow screen over it. The player never "leaves the world" into a separate dashboard application.
- **Progressive disclosure.** Each panel has a summary level (Casual/Standard). Advanced detail expands in place (DEC-038).
- **Every important number has a "Why?"** that opens its causal breakdown.
- **Decisions before data.** Panels lead with what needs a decision, then supporting information.

## 3. Talking to people (DEC-093)
- **The talk panel** is a sheet over the place panel and the character sheet (bottom sheet on phones, side panel from 900 px). It opens from:
  - the place panel's **People here** (a Talk button beside each person; a passer-by is met as the talk starts);
  - the pocket's **People** tab (the council, the mayor, the power broker, the rival);
  - the **walkable street**: the engine's people on the street the player is on stand about it with a name tag; a tap on them (or their tag) walks up to them and opens the panel; with the keyboard, standing beside them shows "Enter: talk to …" (the door-hint pattern) and Enter opens it. At a place, its people are in the place panel.
- **Topics are buttons.** A topic about something has a subject picker (grouped by kind) and an Ask button. Contested topics (a pitch, an ask for support) show the engine's odds, the same figure the roll uses (DEC-073). An unavailable topic is disabled with the engine's reason (`talk-why-*`). Topics a person is not asked (their role, the place, conditions) are left out.
- **The cost is stated:** half a slot; the next conversation uses the half left over; late at night, the late slot. Out of time, a warning says so and every topic waits.
- **Someone elsewhere:** the panel says where they are, with a button to go there (free); topics show no odds until the player is there.
- **The reply** shows as a short dialogue: the question, an opener as a stage direction, then the speaker's lines in their voice, the outcome and any change in how warm they are. Then **What you learned** (the notebook's entries, with their level) and **Leads** (with a button to go there). Everything is Fluent keys and facts from `ExchangeView`; the UI composes nothing.
- **Canvassing on a worry (DEC-091):** the canvass form has a **Talk about** picker, "No particular issue" first, then only the worries the player knows of in that neighbourhood (its own, then the town's). The match comes from the engine's preview (`CanvassView`): a word (strong, fair, weak fit) once the neighbourhood's worries are known, the multiplier once known in detail, "isn't known yet" before. After the canvass, a line says how the worry went down (the engine's outcome).

## 4. Interiors (DEC-088)
- **Which places.** A place kind whose content names an interior (`places.jsonc` `interior`; M6: the council chamber, the town hall, the diner) is entered from the street: a tap on the building walks to its door and in, as does Enter at its door with the keyboard. Every other place keeps its panel. The **Places tab** and the **town map** always open the panel (DEC-051: nothing is reachable only by walking); so will quick mode.
- **The transition.** A short fade from dark into the room (220 ms) and out again at the door; none with reduced motion.
- **The room.** One screen, the whole room in view at a whole-pixel zoom, in the street's scale and view. Composed on the client from the recipe the engine's place view sends (`PlacePanelView.interior`, the content's JSON) and a seed keyed by the town and the place (DEC-085), cached by place, recipe version and seed. The light follows the turn's slots, as on the street: daylight under the windows by day, the lamps once they come on, the street's colour through the glass.
- **People.** Who is here is the engine's (the place view's `people`); how they are here is the engine's too (`here_as`: presiding, member, works, visitor). The room only decides where they stand: the recipe's `cast` maps each to spot roles in order of preference, taken in the recipe's order; seated people sit on their seat, cut at the hips; more people than spots stand on free floor near the door. Each has a name tag (a stranger: their role). Staff nobody present takes (the clerk, the usher, the cook, the waiter) are scenery, like the street's townsfolk: no tag, no conversation; fewer in the evening.
- **Talking.** A tap on someone (or their tag) walks up to them and starts a conversation through the store's `openTalk`; with the keyboard, standing beside them shows "Enter: talk to …" and Enter starts it. The room turns them and the player to each other while the conversation lasts. The dialogue mode attaches through `web/src/interior/stage.ts` (`interiorStage()`: `anchorOf(key)` for bubbles, `frame(key)` for framing, `onInteriorStage` to hear when a room is shown).
- **Hotspots.** Each offered hotspot has a small label (the clerk's windows, the notice board, the lectern, a booth). A tap walks to the tile in front of it and opens its sheet: the place panel's forms for what the recipe puts there, filtered by what the place view offers now (the back booth's envelope only late at night); services open the place panel where their figures are. With the keyboard, Enter at the hotspot's tile opens it.
- **Leaving.** The door (a tap, its "Out" label, Down or Enter on the door tile, or Escape) or the Leave button: back in the street at the place's door. Going somewhere else (an action routed elsewhere, the Places tab) leaves the room too.
- **Battery (DEC-080).** Nothing in a room moves by itself: it draws only while the player walks or the light changes, and not at all behind a sheet or while the page is hidden.
