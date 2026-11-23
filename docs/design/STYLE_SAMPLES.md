# Style samples for approval

Twenty passages written to `docs/design/STYLE_BIBLE.md`, for the owner to approve, reject or mark up. Once approved they are the standard a new line is read against. Each shows the text as a player meets it, with plausible fills; the template follows where the wording depends on one. Five are rewrites of lines now in the game (marked **Replaces**), so old and new can be compared. None of them is in the pack yet.

Fills used: the town of Mireenwick, its neighbourhoods Quarryrow and Castlelands, the *Corriton Ledger*; Sofi Brook (union steward), Rafe Garrowby (plant manager), Halle Talliswell (community organiser), Aurel Garrowby (shopkeeper), Ada Roe (retiree), Celia Janssenfield (councillor).

---

### 1. Street dialogue: a steward asked about the union hall (warm; needs the plant-closure crisis)

> Sofi Brook makes room for you on the bench.
> "The hall? Thursday nights it's full, and it's never full for a good reason. Forty-one of ours are on short time. They come in out of the rain and ask me what management's planning."

### 2. Street dialogue: small talk with a shopkeeper (cool, wry)

> Aurel Garrowby doesn't stop pricing tins.
> "Small talk. Right. There's a customer who comes in every morning, reads the paper and puts it back. That's as much talking as I do before ten."

### 3. Street dialogue: a retiree on what Castlelands worries about (folksy; needs a services `concern`)

> "Round Castlelands it's the state of the services. I waited forty minutes for the bus to the surgery on Tuesday, and the driver apologised when it came. Lovely man. Not his fault."

### 4. Street dialogue: asked about an ambitious person

> "Celia Janssenfield wants the next job before they've done this one. Everybody can see it but them."

`talk-about-person-ambitious = "{ $person } wants the next job before they've done this one. Everybody can see it but them."`
**Replaces** `talk-about-person-ambitious`: "{ $person } wants more than they've got. Everybody can see it, except possibly { $person }." The old line names the person twice, and "wants more than they've got" is a stock phrase. The new one says what the wanting looks like.

### 5. An ask: title, situation and what keeps it

> **A benefits form**
> Ada Roe (Retiree) has been chasing a benefits claim for months. The town hall has lost the form twice, and Ada Roe can't get to the counter before it shuts at four.
> *Next:* Sort it out at the town hall

### 6. An ask kept

> You sat through the whole of the traders' meeting at the chamber of commerce. Rafe Garrowby thanked you in front of the room.

`ask-attend-meeting-kept = You sat through the whole of the traders' meeting at { $place }. { $asker } thanked you in front of the room.`
**Replaces** `ask-attend-meeting-kept`: "You came to the traders' meeting. { $asker } will not forget it." The old closer is one of the pack's reflexive endings (a dozen aftermaths end on someone remembering). The new one shows the thanks.

### 7. A crisis beat: the rebuilding, from the organiser running the relief hall

> "You kept your word to me once. The hall's full of donations, and there are four of us to sort them. Pick up a box."

`scene-disaster-helper = "{ $memory } The hall's full of donations, and there are four of us to sort them. Pick up a box."`
**Replaces** `scene-disaster-helper`: "{ $memory } We could use your hands, not just your sympathy." The old line is a contrast frame. The new one gives the room a physical fact and ends on what Halle Talliswell wants you to do.

### 8. A crisis beat: the plant manager as the lay-offs begin

> "Head office has sent me a number, and the number is the night shift. I can argue for a fortnight. After that I need something to show them."

### 9. A decision: situation and options

> **The pollution row**
> Washing comes in grey off the lines in Castlelands. Halle Talliswell (Community organiser) wants an inspector at Rafe Garrowby's works this month. Rafe Garrowby (Plant manager) says an inspection stops the line for two days, and the order book can't take it.
> - Demand an inspection this month
> - Put the jobs first, and leave the works alone
> - Ask for a month of readings before anyone decides

### 10. A decision's aftermath: brokering talks at the plant

> You kept Rafe Garrowby and Sofi Brook at one table for most of a day. Nothing was signed, and neither of them blamed you for that.

`dilemma-plant-layoffs-broker-talks-after = You kept { $owner } and { $steward } at one table for most of a day. Nothing was signed, and neither of them blamed you for that.`
**Replaces** `dilemma-plant-layoffs-broker-talks-after`: "You spent a long day brokering talks. Neither side got what it wanted, but both saw you try." The old line tells the player how to read the result. The new one names the two people and lets the result stand.

### 11. A headline and its lead: a plant closure

> **Manufacturing layoffs in Mireenwick: 140 jobs lost**
> A plant in Quarryrow has closed, and the 140 people who worked there are out of a job.

### 12. A headline and its lead: a charge

> **Rafe Garrowby charged over a cash bribe**
> Prosecutors in Mireenwick have charged Rafe Garrowby over a cash bribe. The case now goes to trial.

### 13. Overheard: housing (needs a housing `concern`)

> "The letting agent on the high street has stopped putting prices in the window. You have to go in and ask, and then you wish you hadn't."

### 14. Overheard: jobs (needs a jobs `concern`)

> "My brother's on his third agency this year. They ring at five in the morning, and he goes."

### 15. A memory quoted to the player's face

> "When it came to the layoffs at the plant, you were on my side. I've told people."

```ftl
talk-quote-stood-with-me = { $subject_kind ->
    [decision] "When it came to { $subject }, you were on my side. I've told people."
    [bill] "When it came to the { $subject } bill, you were on my side. I've told people."
   *[other] "You were on my side in { $year }. I've told people."
}
```
**Replaces** `talk-quote-stood-with-me`: "When it mattered, you were on my side. People notice that sort of thing." "When it mattered" is a stock phrase, and "people notice that sort of thing" is a summarising closer. The no-subject variant now carries the year instead.

### 16. A callback from someone you voted against (wry)

> "You voted against my bill, Small business grants, in 2030. I'm asking again anyway. I've had worse answers from better people."

### 17. While you were away

> **While you were away** · 3 March to 14 April · 6 turns
> A crisis began at home: Jobs disappearing.
> A plant closed: 140 jobs lost.
> You kept your word: The traders' meeting.
> Decided for you: the pollution row (Ask for a month of readings before anyone decides).

### 18. An ask broken

> You promised Castlelands a meeting and never held it. Halle Talliswell has stopped asking.

### 19. The epilogue's record

> Won the election for Councillor, ward 3 (54%).
> Passed a bill: Small business grants.
> Kept a promise to Sofi Brook.
> Broke a promise to Rafe Garrowby.

(The second line proposes `legacy-law_passed = Passed a bill: { $policy }.` in place of "Passed { $policy }.", so the policy title sits after a colon as EXTENDING asks.)

### 20. UI labels: the end of a skip

> **What changed** · **What happened**
> Nothing of note.
> [ See the decision ]  [ Back to work ]
