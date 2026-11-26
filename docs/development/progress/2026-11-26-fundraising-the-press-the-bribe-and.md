# Fundraising, the press, the bribe and the leak are conversations (B5)

**Date:** 2026-11-26
**Status:** done. Departures and open points at the end.

DEC-114 decision 1 and the owner's answer 6; BEING_THERE §2 and §10.2's B5 row. Lobbying's pattern (note 29): who you talk to, where, the topic and the replies, the odds and the costs shown before choosing, the result as an exchange, here with a Why.

## What the player does now

| Verb | With whom (`actions.talks.<verb>.with`) | Where (places, as before) | The choices | Shown before choosing |
|---|---|---|---|---|
| The press | the paper's editor or a journalist | the news office (the town hall's press room without one) | "Give them an interview"; "… about <bill>" for each bill before the council or in force (behind "Talk about a bill…") | its chance of going well (1 − `press_gaffe_risk`), one slot, the gaffe chance |
| Fundraising | the party chair, a business leader, a plant manager, the hotel's manager, a tech founder | the party office, the business association, the hotel, the tech campus | "Ask them to give to the campaign"; "Ask them over a late dinner" where money is raised late | one slot and health (late: the late slot and its health), the range raised at the player's oratory, donor ties; late: legal risk and the late money's exposure line |
| The bribe | whoever is offering (`actions::briber`: the broker, else a business leader or plant manager) | the diner, late (the back booth) | "Take the envelope" | the late slot, what it pays, legal risk, the exposure line (a crime) |
| The leak | a journalist or the editor | the news office | "Slip them <rival>'s file" | one slot, legal risk, the exposure line (misconduct) |

- The agenda card, a place's action card and the diner's booth now name whom the verb is had with and where ("With Mara Yatesson, an editor, at The Mireenwick Chronicle") and offer **Talk to them**: the player goes there (free) and the talk panel opens at the verb's choice. No toast: the result is the exchange, its outcome line, and **Why?** under it (`ExchangeView.why_key`, the outcome's arguments).
- They see the player where the verb is done (`verbs::sees_player`; `talk::can_talk` holds for them there, so it is a whole conversation, greeting and other topics too, whatever their day says): someone whose day has them there is chosen first, else the first of them in town by id. In all 24 small worlds probed (seeds 1 to 24) every verb has someone and somewhere.
- Replies are content (`actions.talks.<verb>.replies`, the first whose conditions hold): by role (an editor, the party chair) and by how the speaker sees the player.

## One effect

`actions::verbs::one_action` is the one action as it was (`publicity::press`, `publicity::fundraise`, `misconduct::shady`, the leak's deed, and what the act teaches of open proposals, which `learn_from` did for the `Act`). The choice calls it; its cost is `ChoiceCost::Verb` (the verb's own slots and health, paid by the effect, and checked where it is done). `Act { Press | Fundraise | TakeBribe | LeakFile }` is refused for the player (the variants stay so old run logs read) and gone from `ActionDto`; routing (`places::routed` → `verbs::as_talk`) makes such an `Act` the conversation with `partner_at`, so the bots, the stand-ins, the time-skip proxy and the tests that route still do all four.

## Built

- Content: `actions.jsonc` `talks`; `mandate-content/src/verb_talks.rs` (types, validation: occupations, someone to talk to, reply keys and their variables, a last reply with no conditions); the schema. Text: `talk.ftl` (choices, lines, replies, `verb-cost-*`, `why-*`, the panel's words).
- Engine: `actions/verbs.rs` (the four hooks, `one_action`, `as_talk`, `partners`, `partner_at`, `why_key`); `talk/hooks.rs` (`ChoiceCost::Verb`, the registrations); `talk/mod.rs` (its check); `actions/act.rs` (the refusal); `places.rs` (routing).
- Protocol: `PocketView.verb_talks` (`VerbTalkView`), `OfferView.with`/`topic`, `ExchangeView.why_key`, `talk-why-verb-place`; `ActionDto` without the four. No save change: the state's shape is the same.
- Web: `talk/VerbTalk.tsx` (the card); the pocket's agenda, the place panel and the hotspot sheet use it; `TalkSheet` shows a choice's costs beside its odds for every system choice, folds the press's bills behind one choice, and shows the Why; `ActionForm` keeps the one-tap forms only.
- Tests: `mandate-save/tests/verbs.rs` (6: the twin for each of six plans over three seeds; the press rolls the odds it shows over twelve seeds; the risk lines; every key named exists; the `Act` refused and routing made the conversation that does it; determinism and a save); a TalkSheet test; the routing unit test; `scandal.spec.ts` takes the envelope in the booth's conversation; `walk-then-result.spec.ts` uses the town hall meeting (fundraising has no toast now).

## The twin

Same world, at the place: the one action against the conversation. The states are equal (`State::hash`) once the conversation's own record is set aside: the talk log and the action log, what the player knows of the person and whom they saw where, one `PlayerDeed { Talk }` event and its line in the asks' deed log, and familiarity (only that field of any relationship may differ). The outcome key and the amount are the same; the leak's outcome is said as the reporter's (`outcome-leak-talk-*`).

## Departures from BEING_THERE §2, each for the acceptance ("each verb's effect equals today's")

1. **Fundraising's sum is today's formula** (`raised` × (0.5 + oratory)), not who the donor is and what they think of the player; no gift leaves a string (an expectation turned ask); no dinner of several donors. Each changes the effect, so each is for the tuning pass or M7.
2. **The press is one exchange,** opened on nothing in particular or a bill; the journalist's two questions drawn from the outlet's interests and the player's record wait for B6 (lines) and a mechanism of their own.
3. **The leak is at the news office only** (there was no late leak); the diner variant waits.

## Gates and pins

- `mandate-cli bots --seeds 16`, before and after (release): every enforced gate holds, none made report-only. ladder_rote mixed share 0.7222 → 0.7225 (win 1.0); ladder_learned explorer − mixed 0.0244 → 0.0238 (margin 0.01); corrupt − best honest −0.0625 → −0.0625; proxy_not_optimal directed 0.6468 → 0.6468 against informed median 0.7447 → 0.7441; effort_matters and exploring_alone_never_wins unchanged. The corrupt bot's mix is unchanged (it takes the envelope and leaks as often).
- The spine proof's pins moved (repin): each verb now adds a talk deed (one `PlayerDeed` event), and the people a verb is had with can be talked to where it is done, so the bots that ask people for support find more of them (seed 31's explorer legislator: 3373 → 3932 events). Seeds, the state's shape, the geography and the fixtures did not move.

## Not done, and risks

- The panel's header still states a conversation's cost (½ slot); a verb's own time is on its choice ("one slot").
- The panel shows money as units (`3543`), as the old toast did.
