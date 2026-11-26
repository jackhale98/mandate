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
talk-arc-flood-crew-owe = "I can find you a crew. I'll want something back one day, and I won't have to remind you what for."
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

## The meeting at the hall (the flood's turn): the scene is `exchange-flood-*`
## in exchange.ftl; the date is given to the organiser in person.
talk-arc-flood-date = Give a date for the repairs, and put your name to it
talk-arc-flood-date-say = "Tell them every home in { $hood } will have its repairs started by the date I give you. Hold me to it."
talk-arc-flood-date-known = "You carried boxes at this hall in the first weeks, so they'll hear it from you. Miss it, and they'll hear that too."
talk-arc-flood-date-yes = "I'll read it out, and I'll read out your name with it. They've had 'soon' from everyone else."

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
arc-rival-lead-up = { $rival } has called you out to a public debate at { $hall } before the vote.
arc-rival-showdown = The debate with { $rival } at { $hall } is on { $clock_day } { $clock_month }.
headline-arc-rival-statement = { $rival } opens a campaign in { $place }
headline-arc-rival-challenge = { $rival } calls for a public debate at { $hall }
skip-item-arc-rival-dare = { $rival } told the paper you would not dare face the hall.
talk-arc-rival-who = Ask who is behind your rival
talk-arc-rival-who-say = "Who is backing { $rival }? I would rather know now."
talk-arc-rival-who-yes = "The usual names, and one or two who'd rather you didn't know. Sit down and I'll go through them."
talk-arc-rival-who-no = "I keep out of that. Ask someone who owes you less."
visit-arc-rival-walk = Walk the streets with someone who lives there
outcome-arc-rival-walk = You walked { $ward } with someone who has lived there for years. You heard what people there are worried about, in their own words.
talk-arc-rival-backer-owe = "I'll say it. One day I'll ask you for something, and you'll remember tonight."
arc-rival-won = You won the race you fought against { $rival }.
arc-rival-lost = You lost the race you fought against { $rival }.
arc-rival-cooled = The rivalry with { $rival } has cooled.

## The favour (arc.favour_debt). Arguments: `$creditor`, `$colleague`, `$other`;
## while their bill is up, `$law` (it) and `$bind` (`seat`, `party` or
## `promise`: what a yes would cost); `$clock_day` and `$clock_month` (their day).
arc-favour = the favour
arc-favour-debt = You owe { $creditor } a favour.
arc-favour-reminder = { $creditor } has reminded you, pleasantly, of what you owe.
arc-favour-called-in = { $creditor } wants your vote for { $law } by { $clock_day } { $clock_month }, and it would cost you. { $bind ->
        [seat] Your own streets are against it.
        [party] Your party is against it.
       *[promise] It breaks a promise you made.
    }
arc-favour-repaid = You paid { $creditor } back.
arc-favour-deferred = { $creditor } has agreed to wait for another vote. The debt stands.
arc-favour-burned = { $creditor } called the favour in and did not get paid.
arc-favour-settled = { $creditor } has stopped mentioning the favour.

talk-arc-favour-want = Ask what they will want for the favour
talk-arc-favour-want-say = "I know I owe you. What will you want for it?"
talk-arc-favour-want-yes = "A vote, when it matters to me. I'll tell you which. You'll know it when you see my name on it."
talk-arc-favour-want-no = "You'll find out when I ask. That's how it works."
talk-arc-favour-after = Ask what { $creditor } is after
talk-arc-favour-after-say = "I owe { $creditor } one. What are they after, do you think?"
talk-arc-favour-after-yes = "A bill of their own, and the votes for it. When it comes up, they'll be counting yours."
talk-arc-favour-after-no = "Ask them. I don't speak for other people's debts."
talk-arc-favour-cover = Ask them to give { $creditor } their vote in your place
talk-arc-favour-cover-say = "{ $creditor } needs one more vote, and mine would cost me. Would you give them yours?"
talk-arc-favour-cover-yes = "I'll vote with { $creditor }. Then it's me you owe, and I'll want it back."
talk-arc-favour-cover-no = "Your debt, your vote. I'm not getting into it."
talk-arc-favour-time = Ask them to wait for another vote
talk-arc-favour-time-say = "Not this one. Give me the next bill of yours, and I'll be there for it."
talk-arc-favour-time-yes = "The next one, then. I won't ask twice."
talk-arc-favour-time-no = "I need this one. The next one isn't the one I asked for."
talk-arc-favour-cost = Tell them what the vote would cost you
talk-arc-favour-cost-say = "I owe you, and I'm not disputing it. But a yes on this would cost me, and you know why."
talk-arc-favour-cost-cold = "Everything costs something. You took the favour when it suited you."
talk-arc-favour-cost-yes = "I know what it costs you. That's what makes it a favour."

## Their price, in another coin (the favour's ask).
ask-favour-statement = A word in public
ask-favour-statement-detail = { $asker } ({ $asker_role }) would take a word from you in public for { $bill } in place of your vote, and call the debt square.
ask-favour-statement-need = Say in public that you support { $bill }
ask-favour-statement-kept = You spoke up for { $bill }, and { $asker } called the debt square.
ask-favour-statement-broken = You said you would speak up for { $bill }, and you did not. { $asker } noticed.

## The plant (arc.plant_closure). Arguments: `$plant` (its name, as a noun
## with its article, never opening a sentence), `$jobs` (its jobs now; once
## closed, those that went), `$steward`, `$owner`, `$hood` (its district),
## `$hall`, `$site` (its district's name); under notice `$closes_day`,
## `$closes_month`, `$closes_year` (the date).
arc-plant = the plant
arc-plant-short-time = Short weeks at { $plant } since { $month } { $year }, and nobody has said for how long.
arc-plant-notice = A closure notice is up at { $plant }: it shuts on { $closes_day } { $closes_month } { $closes_year } unless orders come back.
arc-plant-hall = { $steward } has called the members of { $plant } to { $hall }. The notice runs out on { $closes_day } { $closes_month }.
arc-plant-eve = The last weeks of the notice at { $plant }. It shuts on { $closes_day } { $closes_month } unless orders come back.
arc-plant-reprieved = The notice at { $plant } has been lifted. { $jobs } jobs stay, on short weeks for now.
arc-plant-scare = Full weeks are back at { $plant }. The short time ended before any notice went up.
arc-plant-answered = The gates of { $plant } have shut, and { $jobs } jobs went with them. Help for those laid off was in place first.
arc-plant-closed = The gates of { $plant } have shut, and { $jobs } jobs went with them.

headline-arc-plant-notice = Closure notice at { $plant }: { $jobs } jobs at risk
headline-arc-plant-reprieved = Closure notice lifted at { $plant }
headline-arc-plant-closed = Gates shut at { $plant }: { $jobs } jobs go in { $place }

heard-plant-hours = "Short weeks at the works again, and nobody upstairs will say for how long."

visit-arc-plant-tour = Ask to be shown round the works
outcome-arc-plant-tour = { $owner } showed you round { $plant }. Part of the floor stood idle, and the order board had gaps in it.
visit-arc-plant-gate = Stand at the gate when the shifts change
outcome-arc-plant-gate = You stood at the gate of { $plant } as the shifts changed. { $steward } saw you there, and so did the people going in.

talk-arc-plant-need = Ask what would keep the plant open
talk-arc-plant-need-say = "What would it take to keep { $plant } open?"
talk-arc-plant-need-yes = "Off the record? Orders. Failing that, costs. Short weeks from the floor and something from round here would buy months."
talk-arc-plant-need-no = "I can't go through the numbers with you. Not yet."
talk-arc-plant-short = Ask them to put short weeks to the members
talk-arc-plant-short-say = "Would the members take short weeks, if it kept the gates open?"
talk-arc-plant-short-yes = "Four days, I can put to them. A pay cut, I can't. I think they'll take it."
talk-arc-plant-short-no = "We've not seen you at the gate. I don't carry that to the members for someone I don't know."
talk-arc-plant-case = Ask them to make the case for keeping the plant
talk-arc-plant-case-say = "If the floor takes short weeks, and there's help from round here, will you make the case for keeping { $plant }?"
talk-arc-plant-case-yes = "With that in writing, I can buy you months. I can't promise more than months."
talk-arc-plant-case-no = "Make the case with what? Bring me something I can put on paper."

skip-item-arc-plant-ballot = { $steward } balloted the members at { $plant }. They stopped the line for a week.

legacy-arc-plant-reprieved = Kept { $plant } open when it was under notice.
legacy-arc-plant-answered = Saw { $plant } close, with help in place for those laid off.

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

## The zoning fight (arc.zoning): a builder's plans in a neighbourhood whose
## rules will not let them build that high. The meeting is `exchange-zoning-*`.
## Cast: `$hood`, `$developer`, `$organiser`, `$neighbour`, `$hall`, `$member`
## (optional: a member of the body that makes the rules, by `$member_role`).
arc-zoning = the plans for { $hood }
arc-zoning-plans = Since { $day } { $month }, { $developer } has had plans for new homes in { $hood }. The rules there say two storeys, and the drawings say more.
arc-zoning-split = { $hood } has split over the plans. { $organiser } wants the homes built; { $neighbour } wants the street left as it is.
arc-zoning-meeting = { $neighbour } has called a meeting at { $hall } about the plans for { $hood }, and the hall will be full.
arc-zoning-chamber = { $weeks } weeks on, the plans for { $hood } wait on the rules. Only a bill changes them.
arc-zoning-upzoned = The rules in { $hood } were loosened, and { $developer } can build. Some of the street is glad.
arc-zoning-public-homes = Homes are going up in { $place } on public money. { $developer } has not said what happens to the plans for { $hood }.
arc-zoning-voted-down = The bill to loosen the rules was voted down. { $developer } has the land in { $hood } and nothing to build on it.
arc-zoning-kept = Nothing was put forward, and { $hood } stays as it is. { $neighbour } says the street was listened to; { $organiser } says the rents will answer for it.
arc-zoning-shelved = Nobody put the rules forward. { $developer } has sold the land in { $hood }, and nothing has been built.

headline-arc-zoning-plans = Plans for new homes in { $hood } go beyond the rules
headline-arc-zoning-upzoned = Rules loosened: new homes can go up in { $hood }
headline-arc-zoning-public-homes = Public homes to go up as the { $hood } plans stall
headline-arc-zoning-voted-down = Members turn down the bill behind the { $hood } homes
headline-arc-zoning-kept = { $hood } plans go no further
headline-arc-zoning-shelved = Land in { $hood } sold on, and the plans with it

heard-zoning-plans = "Seen the drawings for { $hood }? Four floors where the yard is. Half of us have waited years for a flat we can afford; the rest count parking spaces."

visit-arc-zoning-site = Walk the site with the neighbours
outcome-arc-zoning-site = You walked the site in { $hood } with whoever came out. Half of them showed you where the light would go, and the other half asked when they could put their names down.

talk-arc-zoning-drawings = Ask to see the drawings for { $hood }
talk-arc-zoning-drawings-say = "People in { $hood } are talking about ten floors. Show me what you actually want to build."
talk-arc-zoning-drawings-yes = "Four floors, set back from the road, with a share for local people. Take a copy. I'd rather they argued with the drawings than the rumour."
talk-arc-zoning-drawings-no = "I'll show them to whoever is voting on the rules. Until then they stay in my office."

talk-arc-zoning-objections = Hear what the street fears
talk-arc-zoning-objections-say = "Tell me what worries you about the plans. All of it."
talk-arc-zoning-objections-known = "You walked it, so you saw. Four floors, and the back gardens go dark by three. Forty years I've lived here. Nobody asked us before they drew it."
talk-arc-zoning-objections-yes = "The light, the parking, and a builder who'll be gone when the cracks show. I've lived here forty years, and nobody asked us before they drew it."

talk-arc-zoning-speak = Ask { $organiser } to speak at the meeting
talk-arc-zoning-speak-say = "There'll be a meeting about the plans. Would you stand up and say why the homes matter?"
talk-arc-zoning-speak-yes = "I'll speak. I've a list of families in { $hood } paying half of what they earn in rent. They can hear it from me."
talk-arc-zoning-speak-no = "I'll be there, but I won't stand up for you. I don't know yet whose side you're on."

talk-arc-zoning-count = Ask { $member } how they will vote on the rules
talk-arc-zoning-count-say = "If a bill to loosen the rules in { $hood } comes to a vote, where will you be?"
talk-arc-zoning-count-yes = "Bring me the bill and the numbers on rents, and I'll read both. I'm not against homes. I'm against being rushed."
talk-arc-zoning-count-no = "My side will vote it down, and I'll be with them. Count me out before you count anyone else."

skip-item-arc-zoning-petition = { $neighbour } took a petition round { $hood } against the plans. Most doors signed.

legacy-arc-zoning-upzoned = Changed the rules so new homes could go up in { $hood }.
legacy-arc-zoning-public-homes = Saw public homes built when the { $hood } plans stalled.
legacy-arc-zoning-kept = Spoke for { $hood } as it was when a builder wanted more.

## The homes (the zoning fight's ask, from the organiser)
ask-zoning-homes = A bill for the homes
ask-zoning-homes-detail = { $asker } ({ $asker_role }) says talk will not build a single flat. The rules have to change, and they want the bill that changes them put forward by you.
ask-zoning-homes-need = Put forward a bill that loosens the rules or builds homes
ask-zoning-homes-kept = Your bill is down for a vote, and { $asker } has told everyone on the waiting list.
ask-zoning-homes-broken = No bill came. { $asker } told the waiting list who had been asked.

## The street (the zoning fight's ask, from the neighbour)
ask-zoning-street = A word for the street
ask-zoning-street-detail = { $asker } ({ $asker_role }) wants you to say in public that the street will be heard before anything is built.
ask-zoning-street-need = Say in public where you stand on homes
ask-zoning-street-kept = You said it in public. { $asker } cut it out of the paper and pinned it up in the shop window.
ask-zoning-street-broken = You said nothing in public. { $asker } took that as an answer.
