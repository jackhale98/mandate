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
- **Dialogue mode.** A conversation is a classic RPG dialogue box at the bottom of the screen (about a third of a phone's height; a compact 680 px box on desktop, with the topics in two columns) and pixel speech bubbles over the speakers. The box and the bubbles have the game's pixel frames (CSS: whole 3 px borders, notched corners, a hard drop shadow), with light and dark sets. It opens from:
  - the place panel's **People here** (a Talk button beside each person; a passer-by is met as the talk starts);
  - the pocket's **People** tab (the council, the mayor, the power broker, the rival);
  - the **walkable street**: the engine's people on the street the player is on stand about it with a name tag. A tap on them (or their tag) walks the player up beside them (left or right where possible) and starts the conversation. With the keyboard, standing beside them shows "Enter: talk to …" (the door-hint pattern) and Enter starts it. At a place, its people are in the place panel.
- **In the street** (the person stands there by the player, and the street is in sight):
  - the camera eases to frame the two of them facing each other above the box, zoomed in a little (it snaps with reduced motion);
  - the rest of the street dims (a pool of light stays on the speakers);
  - walking, street input, labels, the "you are here" marker and the townsfolk pause until the conversation ends;
  - only the speaker moves (a one-pixel bob while their words come out), so the street draws nothing between replies. The street is not paused as it is behind a sheet.
- **Anywhere else** (the People tab, a place's people, the List view, anyone not on screen): the same box over a large pixel **portrait** of the person, with the bubbles beside it. The portrait is the bust of their street sprite: the same look, composed from the same character parts, at 8 CSS px per art pixel. The street behind is dimmed, blurred and paused.
- **The box:**
  - A name plate shows the name (the role, for a stranger). The header gives their role, how warm they are, the cost ("½ slot · 2½ left"; late at night, "The late slot") and a close button.
  - **Choices** are one line each, one per topic the engine offers (`TalkView`). Topics a person is not asked (their role, the place, conditions) are left out.
  - Contested topics show the engine's odds as a tag ("62%"), the same figure the roll uses (DEC-073). "Asked this week" is tagged too.
  - An unavailable topic is muted, with the engine's reason (`talk-why-*`). A reason every topic shares (out of time, not here) is given once, above the choices.
  - **Goodbye** ends the conversation.
- **A topic about something** shows "›" and opens a **second step in the same box**: Back, the topic's name, a search box when the list is long, then its subjects grouped by kind (only those the player knows of), each with its own odds and availability. Where the topic may be about nothing, "Nothing in particular" comes first.
- **Out of time:** the header says so, a warning above the choices says why, and every topic waits.
- **Someone elsewhere:** the box says where they are and offers "Go to …" first (the free move). The topics wait, without odds, until the player is there.
- **The reply:**
  - The player's question shows briefly in a bubble over the player; the person thinks ("…"); then the reply comes in a bubble over them (or beside the portrait).
  - Openers and unquoted closers are stage directions, in italics. Then come the lines in their voice, any leads in their voice, and any change in how warm they are.
  - Long replies are paged (about five lines on a 360 px phone). A ▼ and a **Continue (1 of 3)** choice turn the page, as do a tap on the bubble or the street, Enter and Space.
  - A typewriter reveals each page (a tap shows it all; it is off with reduced motion). The bubble keeps its size as it types.
  - Street bubbles follow the camera and stay on screen: clamped at the sides, flipped below the speaker near the top, never under the box.
- **After the reply:**
  - The outcome is a line above the choices.
  - What was learned is a **notebook toast** at the top ("✎ Noted: …, and 1 more"). It opens the notebook, which lands on the tab with the new entries.
  - A lead with somewhere to go is a choice ("Go to the market").
  - Everything is Fluent keys and facts from `ExchangeView`; the UI composes nothing.
- **Accessibility (DEC-051):**
  - The box is DOM. The choices are buttons in a list with one Tab stop: arrow keys, Home and End move; Enter chooses. Escape steps back from the subject step, else leaves. Enter or Space off any control turns the page.
  - The reply is read by a polite live region ("Dana Price: …"); the bubbles hold the same words in the DOM.
  - Focus goes into the box when it opens, and back to whatever opened it when it closes (if that is still there), else to the street.
- **Canvassing on a worry (DEC-091):** the canvass form has a **Talk about** picker, "No particular issue" first, then only the worries the player knows of in that neighbourhood (its own, then the town's). The match comes from the engine's preview (`CanvassView`): a word (strong, fair, weak fit) once the neighbourhood's worries are known, the multiplier once known in detail, "isn't known yet" before. After the canvass, a line says how the worry went down (the engine's outcome).

## 4. Interiors (DEC-088)
- **Which places.** A place kind whose content names an interior (`places.jsonc` `interior`; M6: the council chamber, the town hall, the diner) is entered from the street: a tap on the building walks to its door and in, as does Enter at its door with the keyboard. Every other place keeps its panel. The **Places tab** and the **town map** always open the panel (DEC-051: nothing is reachable only by walking); so will quick mode.
- **The transition.** A short fade from dark into the room (220 ms) and out again at the door; none with reduced motion.
- **The room.** One screen, the whole room in view at a whole-pixel zoom, in the street's scale and view. Composed on the client from the recipe the engine's place view sends (`PlacePanelView.interior`, the content's JSON) and a seed keyed by the town and the place (DEC-085), cached by place, recipe version and seed. The light follows the turn's slots, as on the street: daylight under the windows by day, the lamps once they come on, the street's colour through the glass.
- **People.** Who is here is the engine's (the place view's `people`); how they are here is the engine's too (`here_as`: presiding, member, works, visitor). The room only decides where they stand: the recipe's `cast` maps each to spot roles in order of preference, taken in the recipe's order; seated people sit on their seat, cut at the hips; more people than spots stand on free floor near the door. Each has a name tag (a stranger: their role). Staff nobody present takes (the clerk, the usher, the cook, the waiter) are scenery, like the street's townsfolk: no tag, no conversation; fewer in the evening.
- **Talking.** A tap on someone (or their tag) walks up to them and starts a conversation through the store's `openTalk`; with the keyboard, standing beside them shows "Enter: talk to …" and Enter starts it. Inside, the room is the dialogue mode's talk stage (`components/talk/stage.ts`, registered by the interior as the street registers itself outside): the player and the person turn to each other, the room moves up clear of the dialogue box, the rest dims and the speech bubbles sit over the two; the street's stage comes back on the way out.
- **Hotspots.** Each offered hotspot has a small label (the clerk's windows, the notice board, the lectern, a booth). A tap walks to the tile in front of it and opens its sheet: the place panel's forms for what the recipe puts there, filtered by what the place view offers now (the back booth's envelope only late at night); services open the place panel where their figures are. With the keyboard, Enter at the hotspot's tile opens it.
- **Leaving.** The door (a tap, its "Out" label, Down or Enter on the door tile, or Escape) or the Leave button: back in the street at the place's door. Going somewhere else (an action routed elsewhere, the Places tab) leaves the room too.
- **Battery (DEC-080).** Nothing in a room moves by itself: it draws only while the player walks or the light changes, and not at all behind a sheet or while the page is hidden.
