# Arcs X4: the rival, the favour debt, the plant and the flood's meeting to the standard

**Date:** 2026-11-25
**Status:** done for the content and the smallest engine changes it needed; the negotiation table, the stand-ins' play of a scene (X3) and the plant's second scene are not built. Figures: the balance entry of the same day.

Design: `docs/design/EXCHANGE_SCENES.md` §10 (DEC-112), `STAGED_ARCS.md` §7.1 (DEC-109), `INDUSTRIES.md` §3 and §5 (DEC-110). The owner's rule: an arc's turn is played; never a single pop-up. Knowledge is not a checklist (DEC-091).

## The four arcs

- **The rival** (`arcs/rival.jsonc`): *shadow* (the rival's statement; an ally on who backs them) → *offer* (the truce) → *pact* → **lead_up** (new, from 40 days out: the challenge in the paper, the rival's dare, walk the area least pleased with the player, read the cuttings, rehearse, ask a backer) → *showdown* (from a month out, as X1: the debate two or three weeks on; what was not prepared can still be) → *won*, *lost*, *cooled*. `dilemma.debate_challenge` is retired: turning up is accepting. A backer who is not yet a friend speaks for a favour later, which the favour arc can call in.
- **The favour debt** (`arcs/favour_debt.jsonc`): *debt* (ask what they will want) → *reminder* (a colleague's read on what they are after) → **called_in** only when the bind is live (`bill_costs`: the creditor's bill would cost the player in their seat, with their party or against a promise), with a deadline; ways out through existing verbs: their ask for a word in public (`ask.favour_statement`, in another coin: it repays), a colleague who gives their vote instead (the debt moves to them), asking for time, saying what it costs, and the vote itself (a yes repays, as before). Exits *repaid*, **deferred** (new), *burned*, *settled*. Two colleagues (one of each side) learn how it went. `dilemma.favour_called_in` is retired.
- **The plant** (`arcs/plant_closure.jsonc`): rebuilt on DEC-110's review. Raised by a plant on short time or under notice; cast from live state: the plant, its steward and manager (people whose workplace it is), its district, a hall near it. *short_time* (hidden: the hours, a tour, what it would take) → *notice* (the plant's own date: the gate, where the player stands, the steward carrying short weeks and the manager making the case, both relief on the plant's margin; a ballot that stops the line) → **hall** (the turn: a hostile meeting of the members, temper = heat) → *eve* → *reprieved*, *scare*, *answered*, *closed*, by the economy's rule. `dilemma.workers_campaign` is retired; `dilemma.plant_layoffs` stays as the notice's decision.
- **The flood's meeting** (`arcs/flood.jsonc`): the decision is replaced by a hostile meeting (the struck area by income class and the organiser; temper = heat; carrying boxes removes the cold first round) and a date given to the organiser in person. `dilemma.flood_meeting` and `scene.flood_meeting` are retired. An official who owes the player nothing sends a crew for a favour later.

`arc-turn-single-decision` now reports only the budget round's four turns.

## Engine changes (each the smallest the content needed)

- Conditions: `plant` (a plant's state, by a place role or anywhere in the jurisdiction; its date within N days; its name parts and jobs as arguments), `bill_costs` (a creditor's bill a yes would cost), `favour` with `is: false`.
- Casts: `plant` (the plant furthest on in its review), an occupation `at` a place role, an area that holds a place role.
- Arc effects: `plant_relief` (`Effect::PlantRelief`, held to the month like a displacement) and `repay` (a new request, `Effect::Repay`, applied by the characters domain).
- A run in a hidden stage leads no chapter (`narrative/director/arcs.rs::can_lead`): a plant's short weeks otherwise silenced the rival's debate.
- The `act` line leaves the mark `tabled`; a beat's text has the arc's facts; validation reads `<x>_key` as giving `$x`; a plant's short time or notice is the arc's root on the spine.
- Lab and grader: `Intervention::PlantShock` and `Intervention::Favour`, planted by `arcs grade`; the favour's coverage reason replaced. The doctor reads a plant's review and a debt paid as levers.

## Not done, and open

- The stand-ins do not play a scene (X3): every graded meeting and debate is an empty chair, so the grader measures the levers around the turns and not the turns.
- The negotiation table (`table`) is not built: the favour's turn is beats over existing verbs. A table would add conditional offers struck as one deal, the creditor's lean shown, and watchers found by who owes whom.
- The plant's second scene (the negotiation at the works) waits for the table; `plant_review` acts (extend, concede) wait for the economy to price them.
- The rival's ask of a resident (a street's matter raised at the hall) is not written.
- Left in files this task does not own: the retired decisions' keys in `locale/en/narrative.ftl`, and `scene.debate`, `scene.workers_campaign`, `scene.favour_called_in` in `scenes.jsonc` (the doctor reports one as never fired).
