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

## The rival (arc.rival): the standing arc of a campaign. Arguments: `$rival`.
arc-rival = the rival
arc-rival-shadow = The campaign is open, and { $rival } turns up wherever you go.
arc-rival-offer = { $rival } has made you an offer.
arc-rival-pact = The pact with { $rival } is holding.
arc-rival-showdown = With the vote close, { $rival } wants a showdown.
arc-rival-won = You won the race you fought against { $rival }.
arc-rival-lost = You lost the race you fought against { $rival }.
arc-rival-cooled = The rivalry with { $rival } has cooled.

## The favour (arc.favour_debt). Arguments: `$creditor`.
arc-favour = the favour
arc-favour-debt = You owe { $creditor } a favour.
arc-favour-reminder = { $creditor } has reminded you, pleasantly, of what you owe.
arc-favour-called-in = { $creditor } has a bill coming to a vote, and has called the favour in.
arc-favour-repaid = You paid { $creditor } back with your vote.
arc-favour-burned = { $creditor } called the favour in and did not get your vote.
arc-favour-settled = { $creditor } has stopped mentioning the favour.

## The plant (arc.plant_closure). Arguments: `$steward`, `$owner`, `$hood`.
arc-plant = the plant
arc-plant-layoffs = The plant is laying people off. { $steward } and { $owner } are both looking for allies.
arc-plant-walkout = With you on { $owner }'s side, { $steward } has called a walkout.
arc-plant-organising = { $steward } is organising the people the plant laid off.
arc-plant-answered = The job losses in { $place } have slowed, and help for the people laid off has been voted through.
arc-plant-resolution = The job losses in { $place } have slowed.
arc-plant-gone = Two years on, the jobs lost in { $place } have not come back.
headline-arc-plant-layoffs = Plant layoffs begin in { $place }

## The budget round (arc.budget_round): the standing arc of a session.
## Arguments: `$proposer` (whoever proposes the budget), `$member` (a colleague
## with a vote on it), `$hall`, `$hood`; `$books` (`short`, `over` or `level`) and
## `$gap` (last year's balance, in per cent of revenue); `$backers` of `$members`
## (the others with a vote who would back the player); `$clock_day` and
## `$clock_month` (the day of the vote).
arc-budget = the budget round
arc-budget-estimates = { $books ->
        [short] { $place } spent { $gap } per cent more than it took last year. The vote on next year's budget is on { $clock_day } { $clock_month }.
        [over] { $place } took { $gap } per cent more than it spent last year. The vote on next year's budget is on { $clock_day } { $clock_month }.
       *[level] { $place } closed last year level. The vote on next year's budget is on { $clock_day } { $clock_month }.
    }
arc-budget-line = The budget vote is on { $clock_day } { $clock_month }. { $backers } of the { $members } others with a vote would back a change you moved.
arc-budget-turn-hole = { $books ->
        [short] { $place } closed last year { $gap } per cent short. The vote on { $clock_day } { $clock_month } closes the gap with cuts or with the rates.
       *[level] { $place } closed last year a little short. The vote on { $clock_day } { $clock_month } closes the gap with cuts or with the rates.
    }
arc-budget-turn-surplus = { $books ->
        [over] { $place } closed last year { $gap } per cent over. The vote on { $clock_day } { $clock_month } decides where the money goes.
       *[level] { $place } closed last year with a little to spare. The vote on { $clock_day } { $clock_month } decides where it goes.
    }
arc-budget-turn-deal = No party in { $place } has a vote to spare on the budget. The vote is on { $clock_day } { $clock_month }.
arc-budget-turn-line = { $member } has an amendment down: more for { $hood }, out of the reserve. The vote is on { $clock_day } { $clock_month }.
arc-budget-carried = The budget for { $place } passed with your change in it. { $backers } of the { $members } others with a vote backed it.
arc-budget-defeated = Your change to the budget fell: { $backers } of the { $members } others with a vote backed it. The budget passed without it.
arc-budget-passed = The budget for { $place } passed as { $proposer } wrote it.
arc-budget-late = No budget passed in { $place }. The new year's spending is held a fiftieth under the estimates.

headline-arc-budget-estimates = { $place } sets budget vote for { $clock_day } { $clock_month }
headline-arc-budget-carried = { $place } budget passes with a late change
headline-arc-budget-passed = { $place } passes its budget unchanged
headline-arc-budget-late = No budget for { $place }: vote finds no majority

visit-arc-budget-papers = Read the estimates
outcome-arc-budget-papers = You read the estimates at { $hall }, line by line. Now you can say which page a number is on.

talk-arc-budget-room = Ask where the estimates leave room
talk-arc-budget-room-say = "If someone moved a change, where is there room for it?"
talk-arc-budget-room-yes = "One place, and it isn't large. Ask for one thing and I can live with it. Ask for three and I'll fight all of them."
talk-arc-budget-room-no = "The estimates are on the table at { $hall }. Read them first, then ask me again."

skip-item-arc-budget-names = { $member } went round the others with a vote, counting heads on the budget.

legacy-arc-budget-carried = Carried a change to the budget of { $place }.

dilemma-budget-line = where you stand on the estimates
dilemma-budget-line-situation = { $proposer } ({ $proposer_role }) wants the estimates through as they stand. { $member } ({ $member_role }) is asking who will speak for a change before the vote. { $hood } is where you have fewest friends.
dilemma-budget-line-press = Press for more in { $hood }, on the record
dilemma-budget-line-press-after = You asked for more in { $hood } with the minutes running. { $proposer } wrote it down and said nothing.
dilemma-budget-line-fall-in = Tell { $proposer } the estimates have your vote
dilemma-budget-line-fall-in-after = You told { $proposer } the estimates had your vote. { $hood } was not mentioned.
dilemma-budget-line-wait = Say nothing until the vote
dilemma-budget-line-wait-after = You kept your counsel. { $member } put you down as undecided.
scene-budget-line-member = "Everyone in that room wants one more thing in the estimates. I'm asking which of them will say so out loud. Will you?"

dilemma-budget-hole = the hole in the books
dilemma-budget-hole-situation = { $town } spent more than it took last year, and the budget has to close the gap. { $proposer } ({ $proposer_role }) wants it done with cuts. { $member } ({ $member_role }) would sooner raise the money from householders.
dilemma-budget-hole-tax = Draft a rise in the rates to close it
dilemma-budget-hole-tax-after = You drafted a rise in the rates. { $member } was glad of it, and { $proposer } asked who would explain it on the doorstep.
dilemma-budget-hole-cuts = Move the cuts yourself: three per cent off every line
dilemma-budget-hole-cuts-after = You moved three per cent off every line. { $proposer } thanked you for saying it first. { $member } did not.
dilemma-budget-hole-leave = Leave it to { $proposer }
dilemma-budget-hole-leave-after = You left the gap to { $proposer }. The budget went to the vote as written.
scene-budget-hole-proposer = "We spent more than came in. I can take it off every line or I can put the rates up, and I know which one passes. I need your vote for it."

dilemma-budget-surplus = the money left over
dilemma-budget-surplus-situation = { $town } took more than it spent last year. { $proposer } ({ $proposer_role }) wants it kept in reserve. { $member } ({ $member_role }) has a list of what it could pay for, and { $hood } is on it.
dilemma-budget-surplus-spend = Move that it is spent: three per cent more on every line
dilemma-budget-surplus-spend-after = You moved that the money be spent. { $member } read out the list, and { $proposer } asked what happens in a bad year.
dilemma-budget-surplus-relief = Draft a cut in the rates instead
dilemma-budget-surplus-relief-after = You drafted a cut in the rates. Homeowners will see it on a bill. { $member } crossed { $hood } off the list.
dilemma-budget-surplus-bank = Leave it in reserve
dilemma-budget-surplus-bank-after = You left the money where it was. { $proposer } was content, and { $member } kept the list.
scene-budget-surplus-member = "There's money left over, and { $proposer } wants to sit on it. I've a list. Help me move it before the vote."

dilemma-budget-deal = the votes for the budget
dilemma-budget-deal-situation = No party has a vote to spare on the budget. { $member } ({ $member_role }) can bring votes across, and wants someone to sit down first. { $proposer } ({ $proposer_role }) wants it passed before the year turns.
dilemma-budget-deal-broker = Sit down with { $member } and find the votes
dilemma-budget-deal-broker-after = You spent an afternoon with { $member } going through the budget a page at a time. Your own party asked what you had given away.
dilemma-budget-deal-whip = Hold your party's line: let them come to you
dilemma-budget-deal-whip-after = You held the line. Your party counted you in, and { $member } stopped returning your calls.
dilemma-budget-deal-wait = Keep out of the count
dilemma-budget-deal-wait-after = You kept out of it. { $proposer } noticed who was missing from the count.
scene-budget-deal-member = "You can't lose a single vote, and nor can we. I can bring mine across, but not for nothing. Sit down with me before the vote."

dilemma-budget-row = the amendment
dilemma-budget-row-situation = { $member } ({ $member_role }) has an amendment down: more for { $hood }, taken from the reserve. { $proposer } ({ $proposer_role }) says the reserve is for a bad year. Both have asked how you will vote.
dilemma-budget-row-member = Vote for the amendment, and say why
dilemma-budget-row-member-after = You voted for the amendment and said why. { $member } shook your hand. { $proposer } did not look up.
dilemma-budget-row-proposer = Vote it down: the reserve stays where it is
dilemma-budget-row-proposer-after = You voted the amendment down. { $proposer } thanked you, and { $member } told { $hood } how you voted.
dilemma-budget-row-abstain = Abstain
dilemma-budget-row-abstain-after = You abstained. { $member } and { $proposer } each counted you as lost.
scene-budget-row-member = "It's one line: more for { $hood }, out of the reserve. I need to know tonight whether you're with it."

ask-budget-second = A word before the vote
ask-budget-second-detail = { $asker } ({ $asker_role }) wants a change to the estimates and cannot get a hearing. They want you to put it to { $proposer } ({ $proposer_role }) before the vote.
ask-budget-second-need = Put { $asker }'s change to { $proposer }
ask-budget-second-kept = You put the change to { $proposer }, and { $asker } got a hearing.
ask-budget-second-broken = The vote came and you had not raised it. { $asker } found out from the agenda.
ask-choice-budget-second = Put { $asker }'s change to the estimates
ask-say-budget-second = "{ $asker } has a change to the estimates. Hear it before the vote, not after."
talk-ask-budget-second-yes = "Send { $asker } in on Thursday. I'll listen. I'm not promising to like it."
talk-ask-budget-second-no = "I've heard from { $asker } already, at length. Not again before the vote."
