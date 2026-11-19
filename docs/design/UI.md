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
        HUB (primary screen, Canvas 2D scene, DEC-009, DEC-080). Until the walkable town lands in M6 the town screen opens on the town map (DEC-086): an illustrated map of neighbourhoods, routes, and landmarks; tap a building to go there. The "List" tab beside it is the accessible text view of the same places
          HUD: turn span ("Week of 3 March"), day-part slots, Mandate, money, staff,
               pressure-track mini-gauges (DEC-006), notifications, Agenda, End Turn
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
