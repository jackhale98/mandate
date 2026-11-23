# Mandate style bible

The standard for every word a player reads: conversation, overheard talk, asks, dilemmas, scenes, the press, summaries and the epilogue. It extends "Writing for Mandate" in `docs/development/EXTENDING.md`, which holds the truth rules (a line is true whenever it can be said) and the grammar of fills. Where the two disagree, EXTENDING's truth rules win. The approved passages in `docs/design/STYLE_SAMPLES.md` show the voice at its best; when this page and a sample disagree, follow the sample and fix this page.

Status: draft for the owner's approval (M6 checklist, item 13).

## 1. The voice

Mandate sounds like a small town talking to someone who might be able to help. From *Yes Minister* it takes dry civic realism: institutions run on procedure and self-interest, and the comedy comes from how reasonable everyone sounds. It leaves behind the farce and the knowing wink at the audience. From Crusader Kings it takes the event text: a named person, a concrete thing, a choice, in three sentences or fewer. It leaves the chronicle register and the cruelty played for laughs. From *Suzerain* it takes character-driven choice: every decision arrives through a person who wants something, and an option reads as what you would do to them or for them. It leaves the long briefings and the national melodrama. From *Citizen Sleeper* it takes the spare, humane register: tired people and money that runs out on a Thursday. It leaves the second-person interior voice; Mandate never tells the player what they feel.

In one line: name the thing, say what it costs and who pays, then stop.

## 2. Rules of the sentence

**Concrete nouns and numbers.** A thing a person could point at beats a category.
- Bad: "The community is experiencing significant housing pressure."
- Good: "Forty people turned up to view one flat in { $hood }. Forty." (`heard-housing-viewing`)

**One idea per sentence.** If a sentence needs "and" twice, it is two sentences.
- Bad: "The plant is cutting a shift and the steward is angry and the paper wants a quote."
- Good: "The plant is cutting the night shift. { $steward } wants you at the gate by six."

**Verbs that do work.** Prefer a verb to a noun built from one. Watch "is facing", "there is", "impact", "experience", "take action".
- Bad: "The closure had a significant impact on employment in the area."
- Good: "The closure took 140 jobs out of Dunmoor."

**Understatement.** State the fact and let the reader feel it. Hardship is played straight and small.
- Bad: "Heartbroken families watched helplessly as the waters destroyed everything."
- Good: "The water came up to here. You can still see the line on the walls in { $hood }." (`heard-recovery-waterline`)

**Vary the length.** A long sentence, then a short one. Three sentences of the same length read as a form.
- Good: "Six weeks for an appointment at the clinic. Six weeks." (`heard-services-clinic`)

**British English.** organise, neighbour, programme, licence (noun), the rates, a queue, a council estate, the high street, ward, postbag. Contractions in speech ("It's your campaign. Who's going to look?"); fewer in narration, none in labels.

**What a line may assume.** Everything a line asserts must be true whenever its conditions hold (EXTENDING, "The one rule"). Two consequences for craft:
- *Narration states only what the simulation established.* An aftermath, a summary or a headline reports events in the log and arguments it was given. It may say in a few words how the cast took it ("{ $owner } was relieved"). It does not add a crowd, a quote, a number or an event the engine never produced.
  - Bad (aftermath): "Cheering crowds lined the street as you arrived."
  - Good: "You marched at the front. Working families saw you there, and so did the party." (`dilemma-workers-campaign-march-after`)
- *Speech may carry the speaker's own colour,* never a claim about a simulated quantity. A shopkeeper can grumble about a customer and half a melon. A shopkeeper cannot say "unemployment's doubled" unless a condition makes it so, or "the rates went up twelve per cent" unless `$n` says twelve.
- A worry needs its condition; a strong claim needs a strong condition; the player may hold no post; familiarity is earned. These are in EXTENDING and the doctor checks some of them (`worry-unconditioned`, `subject-unread`).

**Never invent a named person or a real place in fixed text.** People come from the cast (`$steward`, `$asker`); a neighbour without a name is "the people at number nine". Towns, outlets and people are generated.

## 3. Register by text kind

Lengths are targets on a phone with ordinary names filled in; the doctor's `prose-habit` check reports lines over them (§8).

**Street dialogue** (`talk-*`). Up to 240 characters; openers and closers under 130. Present tense, first person, spoken to the player in double quotes; a stage direction is third person and names the speaker. Must answer the question asked and sound like this speaker (§4).
- Good: "Half my week at { $about } is people's rotas and the other half is their overtime." (`talk-about-place-own-steward-2`)
- Bad: "The union hall is an important hub for workers in our community." No speaker could have said it; it describes instead of answering.

**An ask** (`ask-*`: title, `-detail`, `-need`, `-kept`, `-broken`). Title: a noun phrase under 40 characters. Detail: up to two sentences, third person, present tense: who asks, what they want, why now. Need: an imperative the player can do, under 90 characters. Kept and broken: past tense, one or two sentences, what happened and who noticed.
- Good: "{ $asker } ({ $asker_role }) is counting the party's votes on the { $bill } bill. With money tight, they need to know you will vote with the party." (`ask-party-budget-detail`)
- Bad: "{ $asker } would really appreciate your support on an important matter." No bill, no reason, nothing to do.

**A crisis beat** (`scene-*`, a scene's line in a crisis). One or two spoken sentences under 200 characters, present tense. Must name the stake in physical terms and end on what the speaker wants from the player.
- Good: "We're short of everything. Whatever you can bring, bring it." (`scene-disaster-helper-fresh`)
- Bad: "This is a difficult time for all of us, and we must come together." Generic, and asks for nothing.

**A decision and its options** (`dilemma-*`). Title: a lower-case noun phrase that the cast can quote ("the pollution row"). Situation: three sentences at most, present tense, third person, up to 460 characters: what is happening, who wants what, what it costs you. Options: imperative, under 90 characters, differing in kind; the catch goes after a colon when it is not obvious. Aftermath: past tense, second person, what you did, then who noticed, in that order.
- Good option: "Haggle: less money, but the whip owes you one."
- Bad option: "Take a balanced approach." Nobody can weigh it, and every option is someone's balance.

**A headline and article lead** (`headline-*`). Headline: present tense, third person, no full stop, under 80 characters, never "you". A lead, when the game has one, is one sentence of 25 words or fewer: who, what, where, in that order, with the number if there is one.
- Good: "{ $policy } bill voted down in { $town }, { $yes }–{ $no }" (`headline-policy-defeated`)
- Bad: "Shock as town rejects bill!" Exclaims, and tells less.

**An overheard line** (`heard-*`). One or two sentences under 170 characters, in double quotes, a resident to another resident. The player is never addressed. Must carry one concrete detail of the worry its condition names.
- Good: "You can't hang washing out in { $hood } any more. It comes back grey." (`heard-environment-washing`)
- Bad: "Pollution is a growing concern for many residents." A survey result, not a person.

**A memory or callback** (`scene-quote-*`, `talk-quote-*`, `memory-*`). One sentence. On the sheet: third person past, the year in brackets. Quoted to the player's face: first person, the year spoken. Selects on `$subject_kind`, with a variant that leaves the subject out.
- Good: "You kept your word on the { $subject } bill."
- Bad: "I'll never forget what you did for me back then." Says nothing that the memory holds.

**A summary line** ("While you were away", `skip-item-*`). One short sentence, past tense, under 110 characters. Labels and policy names after a colon. No commentary.
- Good: "Your bill became law: { $policy }." (`skip-item-law-mine`)
- Bad: "Great news: your hard work paid off and the bill passed!" Cheerleads, exclaims, and puts the name in the wrong place.

**An epilogue line** (`legacy-*`, `epilogue-*`). The record, in the past tense with the subject left out ("Won the election for { $post } ({ $share })."). One fact per line, under 110 characters. The epilogue does not judge the career; the numbers and the list already do.
- Good: "Broke a promise to { $name }." (`legacy-promise_broken`)
- Bad: "A career that will be remembered for its ups and downs." Judges, and could close any career.

**UI labels** (`ui-*`). Sentence case, no full stop on buttons or headings, a verb on anything you press. Under 30 characters for a button.
- Good: "Back to work" (`ui-skip-continue`)
- Bad: "Proceed To Next Phase" Title case, and says nothing about the game.

## 4. Voice sheets

Each speaker type is a role word from the occupations (or a post from the constitution). Voices (`voice.blunt`, `voice.wry`…) colour a role; they never replace it. Three lines each show the register. None may be pasted into the pack unchanged.

**Union steward.** Cares about members, shifts, the rota, overtime and what management will do next. Words: members, the gate, the ballot, a grievance, short time. Rhythm: short declaratives, "we" more than "I". Never says "workforce", "stakeholders" or anything that sounds like a press release; never runs down a member to an outsider.
- "Forty-one members on short time, and management sends us a newsletter."
- "We balloted on Tuesday. I'll tell you the number when it's been counted twice."
- "Come to the gate at six, when the shifts change. Don't bring leaflets."

**Plant manager.** Cares about the order book, head office, downtime and the safety record. Words: orders, the line, the quarter, head office, margins. Rhythm: measured, a number in most sentences. Never calls the staff "family"; never threatens outright, only notes what head office will see.
- "Orders are down a fifth since spring. Head office reads that number before it reads my name."
- "I can keep the night shift until the new year. After that it's a phone call I don't make."
- "Every inspection costs me two days on the line. Two in one month and I'm explaining it upstairs."

**Newspaper editor.** Cares about what stands up, circulation, the lawyer and the advertisers. Words: copy, the front, the letters page, stand it up. Rhythm: clipped and sceptical, a joke at the paper's own expense. Never promises coverage; never says "breaking".
- "I'll run it when two people will say it with their names on."
- "We sell four thousand copies. Every one of them reads the letters page first."
- "If you want the front, do something on a Tuesday. Nothing happens on a Tuesday."

**Journalist.** Cares about the deadline, a source and a quote. Words: on the record, the desk, a tip, a line. Rhythm: quick, asks back. Never says what they are working on in detail; never claims to know what the conditions do not give them.
- "On the record or off? Decide now, because I can't unhear it."
- "I've three hundred words to fill by six and a set of minutes I can't make sense of."
- "Somebody rang the desk this morning and wouldn't leave a name. Those are usually the good ones."

**Shopkeeper.** Cares about takings, the rates, the pitch fees and regulars on credit. Words: the till, takings, on tick, a quiet week. Rhythm: chatty, an anecdote, then the point. Never uses economic jargon; never names a customer's debt to a stranger.
- "Takings were down again on Saturday. People still come in. They just put things back."
- "I've three on tick this month. I know their mothers, so I'll not chase it."
- "Another letter about the rates. I've stopped opening them before lunch."

**Diner owner.** Cares about the regulars, the early shift and who meets whom. Words: the usual, the early lot, table four, the pot. Rhythm: observational, unhurried, a little proud. Never repeats a secret with a name on it; never takes a side out loud.
- "The early lot have stopped ordering the full breakfast. That tells you more than the paper does."
- "I don't ask who's meeting who. I notice who pays."
- "Sit anywhere but the window. That's the regulars' table, and they'll tell you so."

**Community organiser.** Cares about the food bank, the hall, the volunteers and who has stopped coming. Words: parcels, the rota, the residents' meeting, forms, the hall. Rhythm: urgent and practical, numbers from this week. Never says "the less fortunate" or "vulnerable communities"; people are households and streets.
- "We did ninety parcels on Thursday. In the spring it was forty."
- "Number nine's boiler has been out a fortnight, and they won't ring anyone. So I'm ringing you."
- "Don't bring a camera. Bring a van."

**Vicar** (the clergy role; the faith is fictional). Cares about the lonely, funerals, the hall and whoever comes in for the soup. Words: the hall, the congregation, a visit, the soup run. Rhythm: gentle, plain, sometimes dry. Never quotes scripture, names a real denomination or festival, or condemns anyone; never tells the player how to vote.
- "I buried two people this month who had no one to ring. I'd like that number to be nought."
- "The hall's free on Wednesdays. I've stopped asking what people use it for."
- "I won't tell you how to vote. I'll tell you who's been coming in for the soup."

**Party branch chair.** Cares about the branch, selection, canvass returns and discipline. Words: the branch, the list, returns, selection, the line. Rhythm: brisk, counting. Never speaks of ideals; never says "the people".
- "Canvass returns from the east end are soft. Soft means they like you and won't come out."
- "You'll keep the nomination if you stop surprising me in public."
- "Forty at the last branch meeting. Twelve of them under seventy. Write that down."

**Business association chair.** Cares about the rates, red tape, footfall, parking and the high street. Words: members, confidence, investment, footfall, frankly. Rhythm: smooth and sociable, a favour wrapped as advice. Never admits self-interest outright; never sneers at workers.
- "My members don't want favours. They want Monday's rules to still be the rules on Friday."
- "Frankly, half the empty units on the high street are the parking."
- "Come to the breakfast. Shake every hand, eat nothing, leave by nine."

**Elected member** (a councillor, or whatever the constitution names the post; say it through the role word, never "the council"). Cares about casework, the agenda, the group and re-selection. Words: casework, an item, officers, the group, the ward. Rhythm: tired, procedural, wry. Never assumes the player sits in the same chamber (EXTENDING); never names the body except through `$x_role`.
- "Forty pieces of casework this month, and thirty-nine of them are drains."
- "Officers say it can't be done. Officers said that about the bus lane."
- "Vote how you like. Just tell me the night before, and not the morning after."

**Retiree.** Cares about the pension, the bus, the clinic and the allotment. Words: the bus, the surgery, the allotment, my pension. Rhythm: unhurried and dry, a story with a sting at the end. Never the butt of the joke for being old; no confusion about phones.
- "The last bus used to go at eleven. Now it's half seven, so I'm home by dark whether I like it or not."
- "I've voted in every election since I was twenty-one. Nobody has asked me what I think once."
- "My pension went up. So did everything else, only faster."

**Student.** Cares about rent, shifts, the university and whether to stay. Words: the lease, a shift, halls, lectures, home. Rhythm: quick and deadpan. Never slang that dates; never the apathetic stereotype.
- "Four of us in a two-bed. The landlord calls it a house share."
- "I work Fridays and Saturdays at the hotel bar, and I still owe the halls."
- "Half my year will be gone by the summer. Nobody leaves for the weather."

## 5. Writing over variables

A template is read with every value the engine can put in it. Write it for the worst fill, then read it with three (EXTENDING, "Writing for fills", has the shape of each argument).

**Put names where any length reads.** A name early in the sentence, or right after a verb, survives "Ada Roe" and "Bartholomew Okonkwo-Hale". A name at the end of a long clause pushes the point off the screen.
- Bad: "Asked about it last week at the market, the person who said nothing at all was { $person }."
- Good: "{ $person } said nothing at the market last week. That's not like them."

**One name, then "they".** Say a name once, at most twice, then the singular "they".
- Bad: "{ $person } wants more than they've got. Everybody can see it, except possibly { $person }." (`talk-about-person-ambitious`)
- Good: "{ $person } wants the next job before they've done this one. Everybody can see it but them."

**Plurals through selectors, only on `$n`.** Other counts arrive as strings; write around them.
```ftl
promise-count = { $n ->
    [one] one promise
   *[other] { $n } promises
}
```
- Bad: "You owe { $n } favours." It reads "You owe 1 favours."
- Around a string count: "{ $jobs } jobs gone" works for every value of `$jobs` above one, and the plant closure never fires for one job.

**No agreement on a phrase whose number you do not know.** `$industry` can be "the factories" or "the tourist trade".
- Bad: "{ $industry } is struggling." It reads "the factories is struggling."
- Good: "Everything round here comes back to { $industry }." (`heard-jobs-industry`)

**Worries mid-sentence use `$concern_talk`,** a lower-case noun phrase, as the object, never the subject.
- Bad: "{ $concern_talk } is getting worse." It reads "jobs is getting worse."
- Good: "From what I hear, it's { $concern_talk }."

**A line that only works for one value is a different line.** "{ $hazard } strikes { $town }" works for a flood and an earthquake; "The water's still in the cellars" works only for a flood and needs a condition that reads the hazard. If no condition can say it, write the line that is true for all.

**Selectors keep every branch a whole sentence.** Each variant must read on its own; never split a sentence across a selector and the text after it.

## 6. Sensitive ground

Faith, migration, crime and real politics are where a careless template can hurt someone. The rules:

- **No group is a villain or a punchline.** The town's problems are things (the rent, the waits, the smell from the weir), and its people are individuals with names and reasons. A newcomer or a believer is never a category the line sneers at.
- **A scapegoating claim is never stated as fact.** A character may hold a hard view only when a trait or conviction condition gives it to them, the line marks it as theirs ("I'll say it, if nobody else will"), and the game never confirms it in narration. Prefer to write the worry under the view: the waiting list, the shut factory.
  - Bad (overheard): "Since the newcomers came, you can't get a doctor's appointment."
  - Good (overheard): "Six weeks for an appointment at the clinic. Six weeks."
- **Crime is about one person and one act.** Headlines say "accused", "charged" and "convicted" in that order, and never before the case reaches the stage (`headline-case-*`). No line ties a crime to a faith, an origin or a neighbourhood's people.
- **Faiths are fictional, and so are parties.** No real denomination, scripture, festival, party, politician, slogan or culture-war phrase. The vicar is a role in a town, with no doctrine. A policy is described by what it does ("cuts the late bus"), never by a real-world label.
- **Even-handed.** Each option costs someone and the text says who. No line implies that one side of a vote is decent and the other is not.

## 7. Machine-writing habits to avoid

Each habit below reads as text nobody meant. The doctor's `prose-habit` check finds those marked **(lint)**; the rest are the reviewer's job.

**Stock words and phrases (lint).** Words that sound written and say nothing: vibrant, bustling, tapestry, testament, navigate the complexities, in the heart of, a stark reminder, hangs in the air, going forward. The list lives in `STOCK_PHRASES` and grows.
- Before: "In the heart of { $town }, a vibrant community faces an uncertain future."
- After: "{ $town } has a market on Thursdays and a bus that stops at six."

**"Not X, but Y" and "it's not just X, it's Y" (lint).** The contrast frame fakes an insight by denying something nobody said.
- Before: "It's not just about the rent. It's about respect."
- After: "The rent's gone up twice since spring. The second time, they didn't even send a letter."

**Reflexive lists of three (lint when two share a message).** Three adjectives or three nouns, chosen for rhythm. Keep the one that is true here.
- Before: "Families are tired, frightened and angry, and they want answers, action and accountability."
- After: "Families have been queueing at the clinic since seven."

**Em-dash asides (lint over one per message).** A dash pair hides a second sentence inside the first. Use a full stop, a comma or a colon; the anxious voice may trail off with one dash.
- Before: "The mayor — never one to miss a photograph — arrived early."
- After: "The mayor arrived early, ahead of the photographer."

**Moralising or summarising closers.** A last sentence that tells the reader what to think about what they just read. The pack's own version is the memory closer: about a dozen aftermaths end "{ $x } will not forget it" or "will remember". The lint lists those phrases; the general habit is the reviewer's to catch.
- Before: "You stayed out of it. Sometimes the hardest choice is to do nothing."
- After: "You said nothing. { $steward } took it personally, and { $hood } noticed the silence." (`dilemma-plant-layoffs-stay-out-after`)
- Before: "You came to the traders' meeting. { $asker } will not forget it." (`ask-attend-meeting-kept`)
- After: "You sat through the whole of the traders' meeting at { $place }. { $asker } thanked you in front of the room."

**Hedging stacks.** One hedge is a voice (`voice.guarded`). Three are a writer covering themselves.
- Before: "It may perhaps be the case that some residents could be somewhat unhappy."
- After: "Some people in { $hood } are angry, and they've stopped saying so quietly."

**Uniform sentence length.** Four ten-word sentences in a row read like a form.
- Before: "The plant has announced new job cuts this week. The union is planning a march on Saturday. The owner says costs have risen too far. The town is waiting to hear your view."
- After: "The plant is cutting forty jobs. { $steward } has called a march for Saturday, and { $owner } says the order book leaves no choice. Everyone wants to know where you stand."

**Adjective pairs.** "A long, hard year", "a cold, grey morning". Choose one, or replace both with a fact.
- Before: "It has been a long, difficult winter for the town."
- After: "The gritters ran out in January."

**Rhetorical questions as openers.** A line that opens by asking the reader something sounds like an essay. A real question asked of the player is fine ("Will you sign the petition?").
- Before: "What does it mean to stand up for a town?"
- After: "{ $asker } wants you on the picket line by Friday."

**Sentences that restate the one before.**
- Before: "The bins weren't collected. Rubbish has been piling up uncollected."
- After: "The bins weren't collected for two weeks. Rats everywhere." (`heard-services-bins`)

**Exclamation marks outside speech (lint).** Narration, headlines and summaries state; only a speaker exclaims.
- Before: "Your bill passed!"
- After: "Your bill became law: { $policy }."

**Telling the player what they feel.** Describe what anyone could see; leave the feeling to the player.
- Before: "You feel the weight of the town's hopes on your shoulders."
- After: "Three people are waiting outside your door, and one of them has brought a folder."

**Over-long lines (lint, by kind).** The prefix of a key names its kind (§3); a line over its target is reported. Cut the second idea, not the concrete detail.

## 8. The process

1. **Draft against this page.** Read the voice sheet and the register for what you are writing, then read the neighbouring lines of the same topic side by side so yours does not repeat them.
2. **Run the doctor.** `cargo run -p mandate-cli -- content doctor --seeds 0` reports `prose-habit` findings as info; `--strict-prose` makes them warnings. Fix the line, or keep it and say why in a comment above it (a contrast that the speaker would really make, a long situation that needs its third sentence).
3. **A reviewer who did not write it.** They read the rendered corpus (`content doctor --corpus`), not the `.ftl`, and ask of each line: is it true here, would this speaker say it, have I read it five times already, does any habit in §7 show? Findings go back to the writer as line ids.
4. **The owner's blind sample.** Twenty lines drawn at random from the new batch and twenty from approved text, mixed and unlabelled. The owner marks any line that reads machine-written. A batch merges when its marks are no higher than the approved text's. The approved samples (`STYLE_SAMPLES.md`) grow from the lines that pass.

The lint is a smoke alarm. A clean report proves nothing about voice, and a flagged line may be right; the reviewer and the owner decide.
