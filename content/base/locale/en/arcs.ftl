## Staged arcs (DEC-109; STAGED_ARCS §4.1). Keys by kind: `arc-*` (the story
## so far: one or two sentences, present tense, a fact and a date), `visit-arc-*`
## (a button: no arguments), `outcome-arc-*`, `talk-arc-*`, `headline-arc-*`,
## `skip-item-arc-*` (a cast person's move), `legacy-arc-*`, and the arc's own
## `dilemma-*`, `scene-*`, `ask-*` and `heard-*`.
## Arguments: the cast by role (`$hood`, `$organiser`, `$hall`, `$official`,
## `$above`; a person's role word as `$organiser_role` in what the view renders),
## `$place` (the jurisdiction), `$day` and `$month` (the day it began), `$weeks`
## since, and the arc's facts (`$away`, `$homes`, `$repaired`).

## Every arc
event-arc-raised = A matter has opened in { $place }: { $title }.
event-arc-talk = { $ok ->
        [yes] You spoke to { $with } about it, and they agreed.
       *[no] You spoke to { $with } about it, and they said no.
    }
event-arc-decision = { $ok ->
        [yes] You gave your answer.
       *[no] You let the decision pass.
    }
event-arc-ask = { $ok ->
        [yes] You kept your word on it.
       *[no] What you were asked for did not happen.
    }
outcome-arc-choice-agreed = They agreed.
outcome-arc-choice-refused = They said no. You can ask again.
action-arc-visit = Lent a hand

## After the water (arc.flood), from the day it strikes.
arc-flood = after the water
arc-flood-struck = The water came through { $hood } on { $day } { $month }. { $homes } homes are damaged, and { $away } people are sleeping somewhere else.
arc-flood-counting = The water has gone from { $hood }. { $repaired } of { $homes } damaged homes are fit to live in, and { $away } people are still not home.
arc-flood-meeting = { $weeks } weeks on, { $repaired } of { $homes } damaged homes in { $hood } are repaired. { $organiser } has called a meeting at { $hall }.
arc-flood-rebuilding = Work has started in { $hood }. { $repaired } of { $homes } damaged homes are fit to live in.
arc-flood-defended = { $repaired } of the { $homes } homes damaged in { $hood } are fit to live in again, and a flood wall has been voted through.
arc-flood-rebuilt = { $repaired } of the { $homes } homes damaged in { $hood } are fit to live in again. They stand at the same height above the water as before.
arc-flood-hollowed = { $weeks } weeks after the flood, { $repaired } of { $homes } damaged homes in { $hood } are fit to live in. Nobody is saying when the rest will be.

headline-arc-flood-struck = Flood in { $hood }: { $homes } homes damaged in { $place }
headline-arc-flood-weeks = { $weeks } weeks on, { $away } still out of their homes in { $hood }
headline-arc-flood-wall = Flood wall for { $hood } voted through
headline-arc-flood-hollowed = { $hood }, { $weeks } weeks on: { $repaired } of { $homes } homes mended

heard-flood-skips = "There's a skip outside every door in { $hood }. Carpets, mostly. You forget how much of a house is carpet."

visit-arc-flood-box = Carry boxes for the relief
outcome-arc-flood-box = You sorted donations at { $hall } for an afternoon. { $organiser } gave you the tinned food and no speech.
visit-arc-flood-works = Walk the repairs
outcome-arc-flood-works = You walked the streets of { $hood } where the work is going on. The people on the ladders saw who had come to look.

talk-arc-flood-crew = Ask for a repair crew in { $hood }
talk-arc-flood-crew-say = "{ $hood } needs a crew this week. Next quarter is no use to them."
talk-arc-flood-crew-yes = "Fine. I'll move a crew over on Monday. If anyone rings about their potholes, I'm giving them your number."
talk-arc-flood-crew-no = "Every crew I have is spoken for. Bring me a reason I can show the others, and ask me again."

skip-item-arc-flood-petition = { $organiser } took a petition round { $hood } asking when the repairs start.

legacy-arc-flood-defended = Saw { $hood } rebuilt after the flood, with a wall voted through.
legacy-arc-flood-rebuilt = Saw { $hood } rebuilt after the flood.

## Who pays first (the flood's first decision)
dilemma-flood-money = who pays first
dilemma-flood-money-situation = No repair starts in { $hood } until someone says who pays. The town could put up the money now and argue with the region afterwards, or wait for the region to offer. { $organiser } ({ $organiser_role }) has a hall full of people who cannot wait.
dilemma-flood-money-now = Press for the town's money now, and send the region the bill
dilemma-flood-money-now-after = You pressed for the town's money, and the first crews went into { $hood }. { $organiser } told the hall who had asked.
dilemma-flood-money-worst = Pay for the worst streets now, and hold the rest back
dilemma-flood-money-worst-after = You got money for the worst streets of { $hood } and held the rest back. { $organiser } asked which streets counted as the rest.
dilemma-flood-money-wait = Wait for the region: nothing starts until it answers
dilemma-flood-money-wait-after = You waited for the region. Nothing started in { $hood }, and { $organiser } stopped ringing you.
scene-flood-hall-organiser = "People came through that door all night, and I've beds for half of them. Tell me who pays for the houses, and when."

## The meeting at the hall (the flood's turn)
dilemma-flood-meeting = the meeting at the hall
dilemma-flood-meeting-situation = { $organiser } ({ $organiser_role }) has filled the hall with people from { $hood }, weeks after the water. They want a date. They also want to know whether the low streets are being rebuilt or written off.
dilemma-flood-meeting-date = Give them a date, and put your name to it
dilemma-flood-meeting-date-after = You gave the meeting a date and put your name to it. The crews in { $hood } were told the same date the next morning.
dilemma-flood-meeting-wall = Tell them it will flood again: argue for the wall, however long it takes
dilemma-flood-meeting-wall-after = You told the meeting it would flood again, and drafted the wall. People who came for a date went home without one.
dilemma-flood-meeting-honest = Say the money isn't there yet, and stay for every question
dilemma-flood-meeting-honest-after = You told the meeting the money was not there yet, and stayed until the last question. { $organiser } thanked you for not saying soon.
dilemma-flood-meeting-absent = Send apologies
dilemma-flood-meeting-absent-after = You sent apologies. { $organiser } read them out to the hall.
scene-flood-meeting-organiser = "They've heard 'soon' from everyone who has stood where you're standing. Don't say soon. Say a month, or say you don't know."

## A second roof (the flood's ask)
ask-flood-shelter = A second roof
ask-flood-shelter-detail = { $asker } ({ $asker_role }) is running the relief, and the hall is full. The town has buildings standing empty tonight, and { $official } ({ $official_role }) holds the keys.
ask-flood-shelter-need = Get { $official } to open a second building
ask-flood-shelter-kept = A second building opened the same night, and { $asker } stopped turning people away.
ask-flood-shelter-broken = No second building opened. { $asker } put the overflow in the corridor, and told them why.
ask-choice-flood-shelter = Ask { $official } to open a second building
ask-say-flood-shelter = "{ $asker } has people sleeping in the corridor. Open something with a roof tonight."
talk-ask-flood-shelter-yes = "Tonight? Fine. I'll have a caretaker there by six. Tell { $asker } to send them in order."
talk-ask-flood-shelter-no = "I'm not opening a building on your say-so. Have { $asker } put it in writing."
