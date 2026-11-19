# Mandate — UI Specification (draft, D12)

**Status:** Proposed. IDs refer to `DECISIONS.md` (DEC-047 to DEC-051).

## 1. Screen map

```text
Title ── Runs (concurrent run slots, DEC-008) ── Hall of Legacies (DEC-039) ── Settings
  │
  └─ New Game
       ├─ World setup: seed, size, constitution (or random), storyteller, difficulty (DEC-038)
       ├─ Generation & burn-in progress: world visibly forming; history ticker (DEC-022)
       ├─ Constitution card (DEC-007)
       └─ Candidate draft: 3 cards, rerolls (DEC-036)
              │
              ▼
        HUB (primary screen, Canvas 2D scene, DEC-009, DEC-080, DEC-088 "Main screen"): the walkable
          street of the player's town, full screen (phone and desktop), opened when a run starts and
          when returning to the town. Tap a building to walk there; arriving opens its place panel
          Bar (one slim row): Map (the town map, DEC-086, as a full-screen overlay; tapping a place
               closes it and walks there in the street) · date, turn, slots, where you are ·
               Pocket (badges: decisions, alerts, new notebook items) · End Turn
          Pocket (bottom sheet on phones with its tabs at the top; a side panel on desktop that can
               stay docked open beside the town, the street resizing to fit). Tabs, remembered per device:
            ├─ Today → slots, money, Mandate, alerts, decisions waiting, legal, Agenda (auto-routed
            │          actions), your bills and votes, next election, tracks
            ├─ Town → the town card (industry, crisis, power broker, rival and their plan, priorities),
            │          headline numbers, neighbourhoods and their worries, news; gated by knowledge
            │          (DEC-091: placeholders and leads where something is not known yet)
            ├─ Places → every place in town as a button, by neighbourhood, reachable without walking
            │          (the accessible path, DEC-051, and quick mode's future home)
            └─ People → who matters in town (council and mayor, broker, rival) and how they stand
                       with you; the notebook (K5) joins this tab
            plus links: national map, overview, the story so far, save & quit
          Accessibility: a "Skip to Places" link (visible on focus), landmarks (main: the town;
               complementary: the pocket; navigation: the bar), the canvas labelled with a pointer to
               Places; every action works by keyboard and screen reader without the canvas
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
