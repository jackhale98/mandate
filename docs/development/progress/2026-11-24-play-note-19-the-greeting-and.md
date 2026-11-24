# Play note 19: the greeting, and answers that make sense

**Date:** 2026-11-24
**Status:** Done (M6 play note 19, all but "people walk in", which is not this task's).

- **The greeting.** `TalkView.greeting` (with `voice`) is the opener the first reply would have started with (`talk::greeting`, `reply::opener`: the same candidates, guard and keyed draw, keyed on `GREETING` with `n` = conversations with the person this week). The engine knows it was given from the exchange log: `talk::opens_now` holds while the player has not talked with the person today, and the first reply of the day (a hook's choice too) says no opener and records the greeting as a line used. No new saved state. The dialogue shows the greeting at once with the choices; the player's line, then the reply follow; the opener/question/rest split is gone.
- **Ties to a neighbourhood.** The `about` condition takes `office`, `works`, `here` and `tied` (any of home, office, works, here). The "not my end of town / never over that way / only know my own street" lines need `tied: false`. Doctor rule `subject-stranger`.
- **Knowledge.** New `speaker_knows` rules: `officeholder` knows every neighbourhood of their post's jurisdiction at `known` (by report: letters and casework) and the town as a whole at `detailed`; `works_there` knows the neighbourhood they work in at `heard`. Lines for each.
- **The town centre.** A neighbourhood with its town's name is named `ui-hood-centre` ("Larkowick town centre") in talk arguments (`<name>_key`), the about choices, exchange labels, notebook subject and lead arguments, `LocationView.neighborhood_key`, `TownHoodView.name_key`.
- **The whole town.** About kind `jurisdiction` (`AboutRef::Jurisdiction`): the focus jurisdictions containing where the player stands; answered from the town's worries (`line.worries.whole.*`).
- The talk view's choices read the player's focus, not `player.municipality`.
