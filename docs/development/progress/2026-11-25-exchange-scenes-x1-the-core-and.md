# Exchange scenes X1: the core, and the debate as its first scene

**Date:** 2026-11-25
**Status:** built, engine side (DEC-112; design, what is built and every departure: `docs/design/EXCHANGE_SCENES.md` §15). Save v30. The screen (X2), the stand-ins, bots and grader (X3) and the other arcs (X4) are not built.

## What was done

- **The beat and its constants.** `mandate-content/src/exchange.rs`: `ArcExchangeDef` (place, time, sides, audience, rounds, prompts, moves, prep, temper, count, table, outcomes, text), the eleven kinds of line, the twenty prompt sources, the four outcomes, and `ExchangeParams` (content kind `exchange_params`, `content/base/exchange.jsonc`) with overrides by tier and by scene. `BeatKind::Exchange` is the eighth beat kind; `ArcEffect::Learn` lets a beat teach a cast person or area.
- **The core.** `mandate-sim/src/arcs/exchange/`: the audience bound from live state (areas, classes, communities, a workforce, members by a power, a person); the other side's attacks from the player's record, anchored on the spine; the deck by skill and character; the player's lines by requirement and strength; `landing::land`, the one function the view and the command both call; the command (`Begin`, `Say`, `Leave`); the empty chair and the forfeit; the end and its effect requests; the view with every size as far as the player can know it.
- **The debate.** `arc.rival`'s `showdown` is no longer a decision: it is a debate at a hall, three rounds before the seat the player is standing for, on a day two or three weeks after the stage opens. Three preparation beats (rehearse with an ally, ask a backer to speak, read the cuttings). Turning up is accepting; an empty chair is declining.
- **Requests and the spine.** `Effect::Awareness` (recognition, applied by the population), the community filter on `Effect::Approval`, `Effect::Surface` (a hidden fact aired; the media open the story), `Effect::Promise` (a promise made on stage; the asks make it), `StorySource::Exchange` (the paper's account, through the media's perception rule). `PlayerAct::Exchange`; `ArcExchange` for each round and `ArcExchangeEnded`, each with its causes.
- **Skips.** A skip stops when a scene opens (`skip.rs`, built in until the act flow has a data-driven interrupt), and the stand-in has no entry for one. Nothing asks the player yes or no any more.
- **Validation.** The eight static lints of the design's §9.4 with a failing fixture each. `arc-turn-single-decision` is printed by `content validate` as a warning: seven stages in four arcs.
- **Save v30.** Everything new has a serde default and is not written when empty. One thing in flight is dropped: a challenge waiting as a decision in an older save lapses, and that campaign has no debate.
- **Text.** `locale/en/exchange.ftl`: the shared set (labels, costs and risks, locked lines, the room, the strip) and the debate (27 prompts, 16 lines of the player's, 5 answers, results, headlines) and the preparation beats: 120 messages.

## Tests

`mandate-save/tests/exchange.rs` (22): one hash twice, through a save and load in round 2 and from a fork; a scene is four commands; the preview is the landing when everything is known; the change applied is inside the range previewed for every line on offer; the command applies what the view showed; a ledger that knows everything and the default give the same world; passive, engaged and contrarian players; forfeits; the four "not a checklist" properties; the skip stop in eight seeds of eight; secrets aired by either side and a truce; the paper's story and a promise made on stage; the spine's causes; nothing saved with no exchange in the content; the v29 step. `mandate-content/tests/exchange.rs` (7): the pack's debate and the lint fixtures. `landing.rs` holds the design's worked debate (§6.1) as arithmetic.

## What is left

- **X2:** `QueryDto::ArcExchange`, the DTO, the scene's mode in the conversation view, the strip, the result card. The engine's view is `arcs::exchange::view`.
- **X3:** stand-ins and bots that send the command; the report table; the dynamic lints; the calibration gate; the data-driven skip interrupt; the pacing test counting a scene as a turn put to the player.
- **X4:** the rival's `lead_up` stage and the rest of its preparation; the favour's table (`table` is parsed and not built); the flood's meeting (`temper` is built; its sources are not); the hearing; retiring `dilemma.debate_challenge`.
- Not built in the core, and listed in the design's §15.3: a promise outside a campaign, `offer` across a table, the media's record of a denial, a member's goals in their fit, the rival-competence lever.
