# Play note 20: filing, nomination and the ballot

**Date:** 2026-11-24
**Status:** done.

- The rule: filing for a post the player does not hold asks the party's nomination. The party passes them over when its own sitting holder stands again (a hidden draw; the player is told the holder "usually stands again") or when their Standing is below the post's gate (`career.nomination`). Passed over, the player stands for the seat they hold exactly as if they had not filed (the party's renomination rule still applies); with nothing held they are on no ballot. Either way a `NominationRefused` event (causes: the parties' process, and the holder's choice or the Standing track) says why and what they stand for now: in the pocket's alerts for the whole campaign, on the filing card and the campaign hub, in the story log, and in a skip summary (`SummaryItem::NotNominated`).
- A player in office who files for a seat (not theirs) is now their party's nominee there when the gate allows (before, only a player out of office could be).
- The holder's draws now go through `elections::outlook` (term limit, retiring age, else a draw at the same chance), consumed exactly as before: worlds where the player does not file are unchanged.
- Filing view (`NominationView`): the holder, their party, whether they are of the player's party, their outlook in words, the Standing gate and the player's Standing, and a warning when filing will most likely leave the player defending their seat (or on no ballot).
- Election night (`CampaignView.seat_outcome`, web `ElectionNight`): shown whenever an election was held where the player lives. Before, the web opened it only when the player had a result (`player_ran`). Now it shows "You were not on the ballot." when they were on none, what became of the post they held, the offices' results and who won each seat.
- No save change (an event variant and a summary variant are additive).
- Tests: `mandate-worldgen/tests/ballot.rs` (holder stands: fallback with the same ballot as not filing; Standing below the gate; office held by another party or open; no seat; in office means on a ballot or told), `mandate-wasm/tests/ballot_view.rs`, `CampaignCards.test.tsx`, `e2e/ballot.spec.ts`.
- Left: a player holding an office who files for a seat and is refused cannot fall back to the office (offices are decided first; not offered as a path in M6 play). Challenging your own party's holder for the nomination is DEC-107's, later.
