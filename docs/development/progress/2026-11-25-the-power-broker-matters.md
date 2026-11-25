# The power broker matters (M6 note 27)

**Date:** 2026-11-25
**Status:** done (part 1 built; part 2 a design note, not built). Not done is listed at the end.

The owner asked whether the power broker does anything yet. Before this, the broker was chosen sensibly and then only offered bribes, filled a role in some decisions and was talked about. Now they move votes and races, and knowing who they are opens four choices, each with its price. Decisions: DEC-091 and its principle (2026-11-25), DEC-030, DEC-073, DEC-098.

## What the broker does now

- **Votes.** A bill touches the broker's interest when it subsidises or adds capacity to an industry their organisation speaks for, or answers a crisis it is drawn into while the town lives through it (`occupations.jsonc` `stake`; an officeholder broker speaks for the town's own industry and crisis). On such a bill the vote model has a named term, `broker` = `characters.broker.vote_pull` × the organisation's `influence` × how far each member goes along with the broker (½ with no view of them, 0 if they loathe them, 1 if they admire them). On any other bill it is zero. The whip's odds include it; `BillView.broker_pull` (`for` | `against`) says so once the player knows who the broker is; the bill's outcome event names the broker among its causes (`Cause::Actor`), so "Why?" shows them.
- **Elections.** Each campaign week the broker of a race's jurisdiction backs the candidate they regard most (their opinion plus `back_values` × shared views, at least `back_min_opinion`; nobody when they stand themselves): `back_money` to the candidate's funds (the player's through an effect request, caused by the broker) and `back_contacts` canvass-equivalents by the existing contact rule. Read from live state, nothing stored. `CandidateView.backed_by_broker` shows it once the broker is known.
- **Tier-generic.** `characters::town::broker_of(jurisdiction)` is the broker of the jurisdiction's leading town (itself, or its most populous town); no office is named.

## What knowing them opens

Talk hooks (owner `broker`, `actions/broker.rs`), offered only when the player knows the broker of a jurisdiction of their focus at `known` or better. The label carries the price; `broker::preview` is what the label shows and exactly what the choice applies (opinions before personality scales them).

| Choice | With | Gains | Costs |
|---|---|---|---|
| Lobby the broker on a bill before the body (one per bill not against their stake) | the broker | `lobby_sway` × compliance on every member: one conversation for several votes (the label gives the members and the expected votes) | a favour owed to the broker, or one they owe called in; refused below `lobby_min_opinion` |
| Do them a favour | the broker | they owe the player one; their opinion +`favour_opinion` | half a slot; approval `favour_resent_approval` among the classes who resent them |
| Oppose them openly | the broker | approval `oppose_resent_approval` among those who resent them | approval `oppose_depend_approval` among those who depend on them; their opinion `oppose_opinion`; members who like them cool (`oppose_ally_opinion` × liking) |
| Expose them through the press | an editor or journalist | members' opinion of the broker `expose_member_opinion` (so they go along less), approval among those who resent them, a statement the outlets cover | the press's gaffe risk, shown and rolled (keyed draw): on a misfire approval `expose_gaffe_approval` everywhere; the broker's opinion `expose_opinion` either way |

Who depends and who resents: the income classes the broker's organisation comes from (its `class` weights above or below a third). A stand against them is not offered again within `cooldown_days`.

## Built

- Content: `characters.jsonc` `broker` (19 parameters with ranges; `press_occupations` validated); `BrokerParams` in `mandate-content`; the schema regenerated. Text: `talk.ftl`, 20 lines.
- Engine: `characters/town.rs` (`leading_town`, `broker_of`, `Stake`, `stake_of`, `interest`, `BrokerPull`, `broker_pull`, `compliance`, `depend_and_resent`); `policy/mod.rs` (`VoteInputs.broker`, `member_leans` and `legislature_leans` take the pull, the outcome's causes); `politics/elections.rs` (`backed`, the weekly backing); `npc.rs` (the NPC whip reads the pull); `actions/broker.rs` (the openings); one line in `talk/hooks.rs` registers the hook.
- Protocol: `BillView.broker_pull`, `CandidateView.backed_by_broker` (both optional); the TypeScript types regenerated. The web does not show them yet.
- No new saved state; save format stays v31.

## Tests

`crates/mandate-worldgen/tests/broker.rs`: the pull on a bill in the broker's interest and none outside it, each member's share exact, a member who loathes the broker unmoved; the broker among the vote's causes; backing raises a race's funds and contacts against the same world with backing off; the openings absent until the broker is known (heard is not enough) and the exposé only with the press; each opening's effect equal to its preview (sway to the member, favours, approval by class, opinions, the cooldown, both sides of the exposé's roll); the whole path through a conversation, deterministic and through save → load.

Moved on purpose: every pin re-recorded with `scripts/repin.sh` (the spine proof's 16 games, the state-shape fingerprint with v31 kept, the town geography, the v18 and asks fixtures, three seeds). The flood arc's scripts (`tests/arcs.rs`) and the budget round's flood test move from seed 99 to 106: in seed 99's town the broker's votes now put flood defences in force before the flood, so the arc goes straight to `defended` and has no meeting.

Checks: fmt, clippy, content validate, seeds, generated files; every test of the eight crates (`--no-fail-fast`); web typecheck (after `scripts/build-wasm.sh`), lint, tests and the WASM pin test; `determinism --seed 2026 --turns 80` (one hash). Bots: see the balance entry; `idle_cap` is report-only.

## Not done

- NPCs do not use the openings (DEC-030 symmetry): NPC politicians do not lobby or oppose the broker. A follow-up.
- The web shows the two new fields and the choices' prices only through the talk panel's labels.
- Part 2 is a design note: `docs/design/FALLIBLE_ANSWERS.md`.
