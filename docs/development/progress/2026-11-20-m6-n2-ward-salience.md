# M6 N2 Ward salience (DEC-092 note, DEC-078, DEC-098)

**Date:** 2026-11-20
**Status:**

**Done**, 2026-11-20: issue salience per constituency of any tier (`concerns::political_levels`, `salience_in`): each town's measured levels shifted by how much more or less the constituency's neighbourhoods feel each worry than the town, towns weighted by residents, then the town formula (`salience_from`, mean 1); a whole town gives the town's own salience bit for bit. The vote model weighs a race's voters by its constituency's salience (ward races their ward's; mayoral races the town's); a ward member's agenda reads its ward's levels (`npc::agenda_bonus_in`; the memo is keyed by constituency), town agendas unchanged. Derived on demand: no save change. Tests: an industrial ward weighs jobs above a suburban ward of the same town; a town-wide race and agenda reproduce the town's figures; the vote model reads the ward's worries; regional and national constituencies weigh whole towns by residents; determinism and a state round trip. Bots 16 and 32 and the CI sweep pass every gate (BALANCE_LOG); explorer − mixed at 32 seeds 1.02 points (gate 1)
