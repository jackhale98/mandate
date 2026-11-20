# Text audit 1: does it read right?

The owner asked for this read-through (2026-11-20): "we should audit our conversation text and general stories to make sure they make sense when read. We don't want npcs to say things that don't make sense or read weird."

This first pass covers the existing text: 298 conversation lines in nine voices (eight plus plain), 31 dilemmas and requests, their scenes, overheard talk, the press, outcomes and the notebook. It was read from the rendered corpus (`mandate-cli content doctor --corpus <dir> --seeds 12 --years 4`). The corpus is 12 careers × 4 years across scenarios, storytellers and bot strategies, about 3,100 renderings, each read beside its situation and the truth about it. The audit was read by kind, then by template, with all the fills side by side. Its house style is "Writing for Mandate" in `docs/development/EXTENDING.md`, and `TEXT_SAMPLES.md` shows the result.

## Counts

| Category | Problems found | Fixed | Left (with owner) |
|---|---:|---:|---:|
| 1. Nonsense in context (contradicts the truth) | 17 | 15 | 2 (E3, U1) |
| 2. Awkward fills (labels, policy names, empty or raw arguments, names) | 14 classes, about 90 messages | 14 | 0 |
| 3. Voice slips and clashes | 5 | 4 | 1 (E13: opener and topic) |
| 4. Repetition and tics | 6 | 5 | 1 (E3: dilemma reasons) |
| 5. Tone (incl. real names) | 1 | 1 | 0 (no real people, parties or countries found) |
| 6. Clarity (options, outcomes) | 8 | 7 | 1 (E3) |

Text changed: 156 English messages rewritten and 36 added (`content/base/locale/en/*.ftl`); five conversation lines added and three lines' conditions changed (`content/base/conversations/lines.jsonc`). Two known bugs needed code (below). Corpus flags (unfilled `{$x}`, missing keys, doubled spaces): 3 before, 0 after. The new doctor text rules: 5 findings on the old corpus, 0 on the new one, and none in the pack's messages.

## The ten worst, before and after

1. **Retold headlines broke** (`overheard.news.headline`, `overheard.news.doubt`).
   - Before: "Did you see The Mireenwick Chronicle? 'Mireenwick passed {$policy} (1–0)..' It's all anyone's on about."
   - After: "Did you see The Mireenwick Chronicle? 'Local wage floor bill passes in Mireenwick.' It's all anyone's on about."
   - Fix: the web `Translator` renders a key argument with the other keys' text (two passes). The headline's `policy_key` now reaches it. A town story carries a newspaper headline (`headline-…`: third person, no full stop) instead of the event log's sentence.
2. **The paper addressed the player in the second person**: "You were re-elected in Vellenbridge." "You lost your race in Strathford. Rafe Garrowby won." These lines ran as headlines and were retold at the diner. "Re-elected" also appeared on a first win. After: "Celia Janssenfield wins in Vellenbridge", "Rafe Garrowby wins in Strathford". The log line now says "You won your race in Vellenbridge."
3. **A memory with no subject left a hole** (`scene.donor_offer`).
   - Before: "You pressed them on  and got nowhere (2020). I'd like to help your campaign."
   - After: "You leaned on me once, and it didn't work. I'd like to help your campaign."
   - Fix: every memory line, on the sheet and in a quote, selects on `$subject_kind` (`bill`, `decision`, none) and has a variant without the subject.
4. **Scene beats quoted the character sheet in the speaker's mouth.**
   - Before: "You stood with them over the smoke over town (2020). The children here can taste the air." The speaker is "them", the year is in brackets, and the decision is named from the player's side.
   - After: "You stood with me over the pollution row. The children here can taste the air."
   - Fix: first-person `scene-quote-…` keys for every memory kind.
5. **"That business"** for every decision a speaker remembered.
   - Before: "When it came to that business, you were on my side. People notice that sort of thing." All nine renderings were the same. `line.quote.stood_with_me` was 36% of all quotes.
   - After: "When it came to the jobless figures, you were on my side…" and a sibling, "I've not forgotten the building work. You turned up for me, and plenty didn't."
   - Fix: the engine names the decision (its title) and the subject's kind. The doctor no longer flags the line's share.
6. **Lines assumed the player held a seat.**
   - "Ivo Marshby wants your seat, and isn't shy about it. Everybody on the council knows", said to a player out of office.
   - "Did you sit through the whole planning item last week?" and "You get the same postbag I do."
   - After: "…is after the same thing you are, and makes no secret of it." The planning line needs the player to hold a post (`occupation` on slot `player`). "If I knew what worried people, I'd have a bigger majority. Ask them yourself."
7. **Policy names used bare** in about 40 messages.
   - "Small business grants was defeated in Mireenwick." "Pia Reed is the vote Local wage floor is waiting on." "I've counted heads on Local wage floor."
   - After: "Mireenwick voted down the Small business grants bill." "Your bill, Local wage floor, is one vote short, and Pia Reed could be that vote." "Local wage floor? On the doorstep, most people are for it."
8. **Worry labels mid-sentence** in 13 lines.
   - "If you want one thing, it's this: Environment." "Environment, from what I hear." "In Abbey Green? Environment, plain and simple."
   - After: "If you want one thing, it's the pollution." "From what I hear, it's the rent." "In Abbey Green? It's the pollution, plain and simple." These use `$concern_talk`.
9. **Decision titles told from the player's side**, then quoted by the people in them. The rival said: "You took the other side over the rival's offer." Twelve titles are now neutral: "the truce offer", "the donation", "the pollution row", "the party's deal", "the household bills", "the newspaper story", and so on.
10. **A claim stronger than its condition, and a misleading tally.**
    - "Jobs are the first thing anyone in Mireenwick talks about" appeared when jobs were the town's second worry (the trigger is top 2). It now reads "Jobs come up in every conversation in Mireenwick."
    - Every enacted bill read "(1–0)". That is the last stage's tally, a signature, not the vote. The tally is dropped from the enacted line and headline. Defeats and overrides, which are votes, keep theirs.

## What was fixed, by category

### 1. Nonsense in context

- The items above (1, 2, 4, 5, 6, 9, 10).
- **Met before?** Two openers claimed an acquaintance that warmth does not prove. "{player}, isn't it? I've seen you about" and "You again" are now "looks up from what they were doing. 'Yes? What can I do for you?'" and "Well, well."
- **Heard in a new career:** "{player}? All talk. Haven't seen them since the last election". A player may not have stood yet. Now: "You never see them round {hood} unless they want something."
- **The scene vocatives:** "We've never really spoken, councillor" and "someone on the council" assumed the player's post. Both are now post-neutral.
- **Number and sense:** "This town lives on the factories. When that sneezes…" mixed a plural sector, a singular verb, and the town with the neighbourhood. Now: "Everything round here comes back to the factories. One bad year there and the whole of Quarryrow feels it."
- **Role words:** the clergy role was "Minister", which in a political game reads as a government minister. It is now "Vicar".
- **Checked and true:** every line that names the town's or a neighbourhood's top worry through `$concern` matched the truth in all fills. So did every "my end of town" line (the speaker's own home), every rival line's conditions, and every expertise line's indicator.

### 2. Awkward fills

- Policy names framed as names ("the { $bill } bill", "your bill, { $bill },", after a colon) in the event log, dilemma situations, options, aftermaths and effect lines.
- `$concern_talk` mid-sentence.
- Labels after a colon:
  - "At stake: Health", not "Your Health is at stake";
  - "What Sofi Brook is after: Rallying the traders", not "Sofi Brook is Rallying the traders";
  - "Part of a storyline: The rival";
  - "Mireenwick faces a crisis: Jobs disappearing", not "now faces Jobs disappearing".
- Names said three times in one line ("{person} wants more than {person} has got… except possibly {person}") now use singular "they": ten lines in all.
- Full-name vocatives: "Always good to see you, Aurel Garrowby." "Halle Talliswell! I was hoping you'd come by." The player's full name in direct address reads like a form. Seven conversation lines and a scene line were reworded to address the player without the name.
- Preview tense: the gains list said "You drafted Small business grants" before you chose. Now: "A draft bill: Small business grants".
- "a job opening at the town" is now "at the town hall". "Tomorrow's town hall" and "The town hall in X drew a crowd" are now "town hall meeting" (the event, not the building).
- Sheet memories with no subject ("You pressed them on  and…").

### 3. Voice

- Folksy closers for cool speakers: "Mind how you go, love." "And eat something." These now need the speaker not to be cool. Each voice still has two closers that fit anyone.
- The pompous closer "at the last committee" was said by residents. Now: "at the last public meeting".
- A cool opener said someone "carries on as if you weren't there", and then they answered. Now: "barely looks up".
- Stilted scene lines without contractions are now contracted: "It's your campaign. Who's going to look?", "We haven't worked together yet."

### 4. Repetition

| Line | Doctor share before | After |
|---|---|---|
| `line.quote.stood_with_me` | 36% of quote picks | below the too-often threshold (shares picks with `.2`, and has real subjects) |
| `line.small.bus` | 29% | below (two new lines for anyone: `small.ducks`, `small.match`) |
| `line.worries.rumour` | 26% | below (`rumour.3` added; all three reworded) |

- "I've counted heads" appeared in two lines, and the reply that used it was about public support, not votes. Both are reworded.
- `stood_against_me` has a sibling too.

### 5. Tone

- No real people, parties, countries, cities or brands appear in the text.
- The hardship lines (lay-offs, damp flats, food banks) are played straight. The humour lives in small talk and in the wry and folksy voices, and none of it is at a struggling person's expense.
- Fixed: "Minister" (above).

### 6. Clarity

- "Your party wants your vote one way on X" never said which way. Now: "Your party has told you how to vote on the X bill."
- Options now say what you get:
  - "Haggle for a promise" is now "Haggle: less money, but the whip owes you one";
  - "Offer a sunset instead" is now "Offer a four-year sunset clause instead";
  - "Let the offer lapse" is now "Don't answer, and let the offer lapse";
  - "Leave it to the clerk" is now "Leave the tender to the clerk".
- "They would like to be able to call you" is now "In return, they would like to be able to call you."
- "You asked for more data. {voice} heard a delay" now adds "{owner} heard a reprieve", so both sides of the trade-off are told.
- "Why this came up" said "{name} remembers what you did" on most decisions. It now reads "You and {name} have history". It is still the commonest reason (see E3).

## Code for the two known bugs (and where)

- `web/src/i18n/fluent.ts`: key arguments are rendered in two passes, so a key's own key arguments reach it (a retold headline's `policy_key`). Tested in `fluent.test.ts`.
- `crates/mandate-protocol/src/media.rs`: a town story uses `headline-<event>` instead of the event's log key. The 12 headlines are in `media.ftl`.
- `crates/mandate-sim/src/talk/context.rs`: a `remembers` condition gives a decision's title as `$subject` (it gave "that business") and `$subject_kind`. `crates/mandate-content/src/conversation.rs` lists `subject_kind` among the arguments the condition provides. The schema's doc string for `remembers` still names only `$memory_key`, `$subject_key` and `$year`, because changing it would regenerate the schema, which is outside this task.
- `web/src/components/CharacterSheet.tsx`: `subjectKind`, `memoryText` and `quoteKey`. The sheet, the decision sheet's scene beats and the corpus renderer pass `$subject_kind`, and the beats use the first-person `scene-quote-…` keys. Tested in `memoryText.test.ts`.

## New doctor rules (`crates/mandate-cli/src/doctor.rs`)

- `text_problems` finds:
  - `text-raw`: a placeable or missing key in the text;
  - `text-empty-arg`: two spaces, a space before punctuation, empty brackets or quotes;
  - `text-doubled-punct`: two stops; an ellipsis and "?!" are allowed.
- It runs on every English message (rendered with long sample arguments) in the report. It also runs on the rendered corpus after `--corpus`, and on its own with `--check-corpus <dir>`.
- `worry_checks` (`worry-unconditioned`) covers talk lines, overheard lines and dilemma situations. A line that names a particular worry (rent, lay-offs, the air, the rates, the bins, the rebuilding…) is flagged when nothing in its conditions, `concern` or triggers makes that worry real.
- Each rule has a test: planted lines, the old headline and memory renderings, and a small corpus. The base pack is clean on all four rules. The old corpus gives five findings: both broken headlines, twice each, and the empty memory.

## Left for later, with reasons

**U1 (condition-language unification)**: what the conditions cannot express today.

- **A disaster's hazard.** `dilemma.disaster_relief` follows any disaster (an earthquake in the corpus), but its second option drafts flood defences. Options have no conditions, and no condition reads the hazard. The label now says "for the long term" and promises nothing about the next one. It still says "flood".
- **"Has met the player."** Openers that presume acquaintance were reworded. The language cannot tell an acquaintance from a warm stranger: `remembers met` is the speaker's memory of a meeting, and it is not known to exist before the first conversation.
- **Equality between arguments.** The neighbourhood can share the town's name ("the whole of Mireenwick" in Mireenwick). The about-subject being the speaker's own home is guaranteed only by `tells` semantics.
- **Worry evidence is spelled three ways**: a talk `concern` (top one, of the town or the speaker's home), a dilemma `concern` (top N of the town) and overheard `concern` (top three of the neighbourhood). The doctor's worry rule needs an adapter per language. One condition would let it, and writers, say "top worry" and "among the worries" the same way.
- **The player's post**: `occupation` on slot `player` in talk and `office` in narrative conditions.

**E3 (dilemmas)**

- Office and body names are still hard-coded in some dilemmas ("Mayor { $mayor }", "the mayor's cuts", "the council"). DEC-098 wants them from the constitution, as `$x_role`. The text audit did not restructure casts.
- "Why this came up" is dominated by `why-memory`. Better reasons would name the memory ("You stood with them over the building work").
- Effect lines with a zero delta ("Name recognition (your ward): 0") should not be shown. That is presentation of the engine's numbers.
- The enacted tally should be the deciding vote, not the last stage's. That needs an engine field. Until then the text omits it.
- Eight dilemmas, their scenes and the rival storyline fire in over 60% of careers (the director, E2).

**E8 (asks)**

- The support and favour topics are thin, and their memory lines (`support.kept`, `support.owes`, `favour.owed`, `favour.owes`) never held in bot careers.
- "Let me see what I can do" and "That depends on what's in it for us" are the generic answers to every ask. Asks should hear about the thing asked for.

**E13 (conversation memory)**

- Openers are chosen without the topic. "Draws you a little way off, out of earshot" can precede "Our dog's learned to open the back gate."
- Quotes pick the most salient memory whatever the topic.
- The small-talk fallbacks (`small.weather`, `small.family`) can never be chosen (below).
- Characters need a pronoun field. Until then lines use names or "they".

## The 33 lines that never held (12 × 4 corpus, before and after)

The same 33 lines never held after the fixes (`coverage.json`). The five new lines all held. "Reachable" means the content and the engine allow it. "What would reach it" is what has to happen in play.

| Line | Reachable? | What would reach it |
|---|---|---|
| `line.open.folksy.trusted` | yes | a warm-natured speaker (warmth ≥ 0.68, no voice trait) at opinion ≥ 50 and trust ≥ 0.5; the corpus had friendly speakers, none folksy |
| `line.open.smooth.trusted` | yes | the same with a charmer or schemer |
| `line.quote.broke_promise`, `.2` | yes | owe someone a favour, then vote against their bill, or refuse or dodge the "favour called in" dilemma; the memory must then be their most salient one |
| `line.quote.kept_promise`, `.2`, `line.support.kept` | yes | owe a sponsor a favour, then vote for their bill (it repays the favour), or pay a called-in favour |
| `line.quote.vetoed_my_bill`, `line.quote.signed_my_bill` | yes | the player holds the signing office (mayor) and signs or vetoes a councillor's bill; no bot career reached the mayoralty |
| `line.quote.broke_ranks` | yes | vote against your party's line on a bill; party colleagues remember it |
| `line.worries.fb.farm`, `line.worries.farm`, `line.worries.farm.air`, `line.worries.farm.rent`, `line.runs.fb.farm`, `line.runs.farm`, `line.about.fb.farm`, `line.pitch.fb.farm`, `line.support.fb.farm`, `line.small.farm` | **no, today** | no generated town has a farmer (`occupation.farmer`; the doctor's `no-town-has`); it needs agricultural towns to place one (S1, places and characters) |
| `line.worries.workers.closure` | yes | a plant closure (the event, not only the "Jobs disappearing" crisis) within 12 months, and a steward or organiser not cool on the player |
| `line.worries.clergy.disaster` | yes | a vicar in a town hit by a disaster within about a year (`recent_disaster` ≥ 0.5); the corpus's disaster town had a vicar, but later |
| `line.runs.stake`, `.2`, `.3` | yes | someone who knows the broker in detail and trusts the player enough to tell it (`tells broker detailed`) |
| `line.runs.self`, `.2` | yes | the broker themselves, trusting the player that far |
| `line.about.person.maverick` | yes | ask about someone with `trait.maverick` (8% of characters) whom the speaker knows; no such pair was sampled |
| `line.support.owes` | yes | a speaker who owes the player a favour, asked for support |
| `line.favour.owed`, `line.favour.owes` | yes | the favour topic with a politician who is owed, or owes, a favour |
| `line.small.weather`, `line.small.family` | **no** | they are small talk's fallbacks, used only when no other small-talk line holds, and the lines for anyone (≥ −15) and for the cool (≤ −15) cover every opinion. Reaching them needs a speaker for whom nothing else holds. A `mandate-content` test names both by their exact text, so they stay. E13 should make them ordinary lines or narrow the others |

## How to repeat this

```text
cargo run --release -p mandate-cli -- content doctor --corpus /tmp/corpus --seeds 12 --years 4
cargo run --release -p mandate-cli -- content doctor --check-corpus /tmp/corpus
cargo run --release -p mandate-cli -- content doctor --seeds 16 --years 4
```

Then read by kind and template with `jq` (see "Checking your writing" in EXTENDING.md). The next pass should start from the E3, E8 and E13 content as it lands.
