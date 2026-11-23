# W1 play notes: web

**Date:** 2026-11-23
**Status:** done (web side of owner notes 2, 3, 4, 6, 7)

- Note 2: the person's opener is said before the player's question (`TalkSheet`); the question is kept in the store (`talkAsked`) so meeting a passer-by, which rebuilds the dialogue, does not lose it.
- Note 3: the map's Town | Country buttons no longer shrink below their text (`.map-scope .seg`).
- Note 4: the town card stacks label over text at every width; each neighbourhood shows its ward (`TownHoodView.ward_number`, label by `districtLabel` from the constitution's `ui-post-seat`).
- Note 6: arriving from the town map enters a place that has an interior (`walkIn`); places without one keep the panel; the Places tab is unchanged.
- Note 7: anyone standing in the street whose place changes walks off towards where the engine says they are (`whereIs`: the talk view's route), or the nearest street exit, before going; deferred while a conversation is open.
