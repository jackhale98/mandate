## Asks and promises (DEC-097 §2; ENGAGEMENT §3). Each ask has a title, the
## situation (`-detail`), what keeps it (`-need`), and how it ended (`-kept`,
## `-broken`). Arguments: the cast by role (`$asker`, `$asker_role`,
## `$asker_hood`, `$bill`, …), `$asker_occupation` (the asker's occupation id
## without its prefix, for variants), `$history` (`kept`, `broken` or `none`:
## how the asker's last ask ended), `$crisis`, `$town`, `$days`, `$place`
## (where to go), `$answer` (a promise's chosen bill), `$constituency`.

## Casework
ask-casework = { $asker_occupation ->
        [teacher] A school place
        [nurse] A school place
        [student] A school place
        [retiree] A benefits form
        [farmer] A planning objection
        [farmhand] A planning objection
        [shopkeeper] A planning objection
        [diner_owner] A planning objection
       *[other] A blocked drain
    }
ask-casework-detail = { $history ->
        [kept] { $asker } ({ $asker_role }) remembers you sorted something out before, and has another problem.{ " " }
        [broken] { $asker } ({ $asker_role }) has not forgotten the last time you said you would help.{ " " }
       *[none] { "" }
    }{ $asker_occupation ->
        [teacher] { $asker } has a family in { $asker_hood } turned down for a school place, and the appeal is going nowhere.
        [nurse] { $asker } has a child in { $asker_hood } turned down for a school place, and the appeal is going nowhere.
        [student] { $asker }'s younger brother has been turned down for a school place in { $asker_hood }, and the appeal is going nowhere.
        [retiree] { $asker } has been waiting months for a benefits claim in { $asker_hood }. The town hall keeps losing the form.
        [farmer] { $asker } is fighting a planning application next to their land in { $asker_hood } and cannot get anyone at the town hall to answer.
        [farmhand] { $asker } is fighting a planning application at the edge of { $asker_hood } and cannot get anyone at the town hall to answer.
        [shopkeeper] { $asker } is fighting a planning application next to their shop in { $asker_hood } and cannot get anyone at the town hall to answer.
        [diner_owner] { $asker } is fighting a planning application next to their diner in { $asker_hood } and cannot get anyone at the town hall to answer.
       *[other] The drain on { $asker }'s street in { $asker_hood } has been blocked for weeks. Every time it rains, the road floods.
    }
ask-casework-need = Sort it out at the town hall
ask-casework-kept = You sorted out { $asker }'s problem. Word gets around { $asker_hood }.
ask-casework-broken = You never sorted out { $asker }'s problem. They told the neighbours.

## Vote with me (was the request "the vote between friends")
ask-vote-with-me = A vote between friends
ask-vote-with-me-detail = { $history ->
        [kept] { $asker } ({ $asker_role }) has counted on you before, and it worked. Now there is their bill, { $bill }, and they need your vote.
        [broken] { $asker } ({ $asker_role }) asks for your vote on their bill, { $bill }, though they remember the last time.
       *[none] { $asker } ({ $asker_role }) counts you as a friend and asks for your vote on their bill, { $bill }.
    }
ask-vote-with-me-need = Vote yes on the { $bill } bill
ask-vote-with-me-kept = You voted for { $asker }'s bill. They owe you one.
ask-vote-with-me-broken = You promised { $asker } your vote on the { $bill } bill and did not give it. They will remember.

## The party counts heads
ask-party-budget = The party counts heads
ask-party-budget-detail = { $asker } ({ $asker_role }) is counting the party's votes on the { $bill } bill. With money tight, they need to know you will vote with the party.
ask-party-budget-need = Vote with the party on the { $bill } bill
ask-party-budget-kept = You voted with the party on the { $bill } bill. { $asker } noticed.
ask-party-budget-broken = You told { $asker } you would vote with the party on the { $bill } bill, and you did not.

## A colleague's event
ask-colleague-event = A colleague in a close race
ask-colleague-event-detail = { $asker } ({ $asker_role }) says { $ally } ({ $ally_role }) is in a close race and would value being seen with you on the doorsteps where they live.
ask-colleague-event-need = Canvass with { $ally } within { $days } days
ask-colleague-event-kept = You knocked on doors with { $ally }. The party was grateful.
ask-colleague-event-broken = You never turned up for { $ally }. The party noticed.

## Say something
ask-say-something = Say something about { $crisis }
ask-say-something-detail = { $asker } ({ $asker_role }) is writing about { $crisis } and wants to know where you stand.{ $history ->
        [kept] { " " }You gave them a straight answer last time.
        [broken] { " " }Last time you promised a statement and never gave one.
       *[none] { "" }
    }
ask-say-something-need = Make a statement to the press
ask-say-something-kept = You said your piece on { $crisis }, and { $asker } ran it.
ask-say-something-broken = { $asker } wrote that you had nothing to say about { $crisis }.

## The traders' meeting (was the request "the traders' meeting")
ask-attend-meeting = The traders' meeting
ask-attend-meeting-detail = { $asker } ({ $asker_role }) is rallying { $town }'s traders over the lost jobs and asks you to come to their meeting at { $place }.
ask-attend-meeting-need = Go to { $place }
ask-attend-meeting-kept = You came to the traders' meeting. { $asker } will not forget it.
ask-attend-meeting-broken = The traders met without you. { $asker } made sure everyone knew you had promised to come.

## The picket line (was the request "the workers' campaign")
ask-join-picket = The picket line
ask-join-picket-detail = { $asker } ({ $asker_role }) is organising the laid-off workers of { $town } and asks you to come to { $place }, then say publicly that the town needs those jobs back.
ask-join-picket-need = Go to { $place }, then make a statement about jobs
ask-join-picket-kept = You stood with the workers and said so in public. The party grumbled.
ask-join-picket-broken = You promised the workers your voice and never gave it. { $asker } said so on the picket line.

## A public meeting here
ask-town-hall-here = A meeting in { $asker_hood }
ask-town-hall-here-detail = { $asker } ({ $asker_role }) says people in { $asker_hood } feel nobody listens to them, and asks you to hold a public meeting there.
ask-town-hall-here-need = Hold a town hall in { $asker_hood }
ask-town-hall-here-kept = You held a meeting in { $asker_hood }, as you said you would.
ask-town-hall-here-broken = You promised { $asker_hood } a meeting and never held it.

## Draft an answer (a big ask)
ask-draft-answer = An answer to { $crisis }
ask-draft-answer-detail = { $asker } ({ $asker_role }) says talk will not fix { $crisis }. They want a bill put forward, and they want it from you.
ask-draft-answer-need = Introduce a bill that answers { $crisis }
ask-draft-answer-kept = You put a bill forward, as you told { $asker } you would. They owe you for it.
ask-draft-answer-broken = You promised { $asker } a bill on { $crisis }, and none came.

## The broker's vote (a big ask)
ask-broker-vote = A word from { $asker }
ask-broker-vote-detail = { $asker } ({ $asker_role }) has an interest in the { $bill } bill and lets you know, over coffee, that a yes would be remembered.
ask-broker-vote-need = Vote yes on the { $bill } bill
ask-broker-vote-kept = You voted yes on the { $bill } bill. { $asker } remembers favours.
ask-broker-vote-broken = You let { $asker } down on the { $bill } bill. They remember that too.

## After the disaster
ask-visit-after-flood = Come and see { $asker_hood }
ask-visit-after-flood-detail = { $asker } ({ $asker_role }) lives in { $asker_hood }, which took the worst of it. People there want to see the people they elected on their streets.
ask-visit-after-flood-need = Canvass or hold a town hall in { $asker_hood }
ask-visit-after-flood-kept = You went to { $asker_hood } when it mattered.
ask-visit-after-flood-broken = { $asker_hood } waited for you, and you did not come.

## Second my bill
ask-second-my-bill = Hear me out
ask-second-my-bill-detail = { $asker } ({ $asker_role }) is pushing the { $bill } bill to fix what is wrong in { $town }. They want to talk it through with you, then count on your vote.
ask-second-my-bill-need = Talk to { $asker }, then vote yes on the { $bill } bill
ask-second-my-bill-kept = You heard { $asker } out and voted for the { $bill } bill.
ask-second-my-bill-broken = You told { $asker } you would back the { $bill } bill. You did not.

## Services
ask-fund-the-school = Speak up for services
ask-fund-the-school-detail = { $asker } ({ $asker_role }) says the cuts are biting in { $asker_hood }: fewer staff, longer waits. They ask you to say so in public before the next budget.
ask-fund-the-school-need = Make a statement about services
ask-fund-the-school-kept = You spoke up for the town's services, as { $asker } asked.
ask-fund-the-school-broken = You said nothing about services, though you told { $asker } you would.

## Meet the newcomer
ask-meet-the-newcomer = Meet { $newcomer }
ask-meet-the-newcomer-detail = { $asker } ({ $asker_role }) wants { $newcomer } ({ $newcomer_role }) to feel welcome in { $town }, and asks you to sit down with them.
ask-meet-the-newcomer-need = Talk to { $newcomer }
ask-meet-the-newcomer-kept = You met { $newcomer }. { $asker } was pleased.
ask-meet-the-newcomer-broken = You never met { $newcomer }. { $asker } had vouched for you.

## Keep it quiet (from the second year)
ask-keep-it-quiet = Leave it alone
ask-keep-it-quiet-detail = { $asker } ({ $asker_role }) has heard you have been asking about them, and asks you, quietly, to leave it alone.
ask-keep-it-quiet-need = Leak nothing about { $asker } for { $days } days
ask-keep-it-quiet-kept = You left it alone. { $asker } owes you for your silence.
ask-keep-it-quiet-broken = You went to the press about { $asker } after all. They will not forgive it.

## An introduction
ask-introduce-me = An introduction
ask-introduce-me-detail = { $asker } ({ $asker_role }) would like to meet { $official } ({ $official_role }), and asks you to put in a word first.
ask-introduce-me-need = Talk to { $official }, then to { $asker }
ask-introduce-me-kept = You put in a word with { $official }, and told { $asker } so.
ask-introduce-me-broken = You never spoke to { $official } for { $asker }.

## Asks in conversation (talk hooks, S4): an ask whose next step is a talk
## puts a choice of its own in the conversation with that person. `ask-choice-*`
## is the button, `ask-say-*` what the player says, `talk-ask-*` the reply.
## Arguments: the ask's people by role (`$asker`, `$official`, `$newcomer`),
## a bill by role (`$bill`), `$speaker`, `$player`, `$town`.
ask-choice-introduce-official = Put in a word for { $asker }
ask-say-introduce-official = "{ $asker } would like half an hour with you. I said I'd ask."
talk-ask-introduce-warm = "For { $asker }? Of course. Tell them to come by on Thursday. I'll keep the afternoon free."
talk-ask-introduce-curt = "Fine. Ten minutes, and they bring whatever it is in writing."
talk-ask-introduce-decline = "I don't take meetings on your say-so. Ask me again when we're on better terms."
ask-choice-introduce-asker = Tell { $asker } it is arranged
ask-say-introduce-asker = "I spoke to { $official }. They'll see you."
talk-ask-introduce-thanks = "They will? Thank you. I'll ring the office first thing tomorrow."
ask-choice-second-my-bill = Hear { $asker } out on the { $bill } bill
ask-say-second-my-bill = "Go on, then. What does the { $bill } bill do?"
talk-ask-second-my-bill = "Thank you. Here's the page that matters, and here's who it helps. Read the rest on the bus."
talk-ask-second-my-bill-cool = "I didn't think you'd come. Ten minutes, then. I've timed it."
ask-choice-meet-newcomer = Welcome { $newcomer } to { $town }
ask-say-meet-newcomer = "{ $asker } said I should come and say hello. Welcome to { $town }."
talk-ask-meet-newcomer-warm = "{ $asker } sent you? Good. You're the first person here to ask how I'm finding it."
talk-ask-meet-newcomer = "Thanks for coming over. I'm still learning where everything is. Which day is market day?"
## What a choice did (the conversation's outcome line), and the ask's note
## after a no.
outcome-ask-choice-agreed = { $name } agreed. That step is done.
outcome-ask-choice-kept = { $name } agreed. You kept your word to { $asker }.
outcome-ask-choice-refused = { $name } said no. The ask is still open.
ui-ask-talk-refused = { $name } said no last time. Ask again when they think better of you.
## A conversation with nothing behind it says so (never an empty box).
talk-nothing-to-say = "I've nothing to say about that."

## Campaign promises
promise-answer-crisis = Get the { $answer } bill passed
promise-answer-crisis-detail = You promise { $constituency } that you will get the { $answer } bill passed to answer { $crisis }.
promise-answer-crisis-need = Sponsor the { $answer } bill and see it enacted
promise-answer-crisis-kept = The { $answer } bill is law. You kept your word to { $constituency }.
promise-answer-crisis-broken = You promised { $constituency } the { $answer } bill. It never came.
promise-no-tax-rise = No rise in taxes
promise-no-tax-rise-detail = You promise { $constituency } that you will not vote for or propose any rise in the town's taxes.
promise-no-tax-rise-need = Vote against every tax rise
promise-no-tax-rise-kept = You kept taxes where they were, as you promised.
promise-no-tax-rise-broken = You promised no tax rise and backed one anyway. Your opponents will not let anyone forget it.
promise-town-halls = A meeting in every neighbourhood
promise-town-halls-detail = You promise { $constituency } a public meeting in every one of its neighbourhoods within half a year.
promise-town-halls-need = Hold a town hall in each neighbourhood
promise-town-halls-kept = You held a meeting in every neighbourhood, as you promised.
promise-town-halls-broken = You promised a meeting in every neighbourhood. Some are still waiting.
promise-jobs = Jobs first
promise-jobs-detail = You promise { $constituency } that you will put jobs first, and get a bill for them passed.
promise-jobs-need = Sponsor a jobs bill and see it enacted
promise-jobs-kept = Your jobs bill is law. You kept your word.
promise-jobs-broken = You promised jobs first. No jobs bill of yours became law.
promise-services = Save the town's services
promise-services-detail = You promise { $constituency } that you will fight for the town's services, and get a bill for them passed.
promise-services-need = Sponsor a services bill and see it enacted
promise-services-kept = Your services bill is law. You kept your word.
promise-services-broken = You promised to save the town's services. No bill of yours did it.
promise-clean-politics = Clean politics
promise-clean-politics-detail = You promise { $constituency } clean hands: no back-room deals, nothing to hide.
promise-clean-politics-need = Keep your record clean
promise-clean-politics-kept = Your term ended with nothing to hide. You kept your word.
promise-clean-politics-broken = Something you hid came to light. So much for clean politics.

## A crisis mid-sentence (`$crisis` in asks), by crisis kind
ask-crisis-plant_closure = the job losses
ask-crisis-disaster_recovery = the recovery from the disaster
ask-crisis-housing_squeeze = the housing squeeze
ask-crisis-fiscal_hole = the hole in the town's finances

## A place to go when it has no name of its own (`$place`), by place kind
ask-place-union_hall = the union hall
ask-place-market = the market
ask-place-business_association = the chamber of commerce

## Who a promise is made to
ask-constituency = the voters of ward { $number }
ask-electorate = { $electorate_tier ->
    [national] the country's voters
   *[other] the voters of { $electorate }
}

## Why an ask ended without a winner or a loser
ask-void-left = They have left town.
ask-void-withdrawn = The bill was withdrawn.
ask-void-no-vote = You no longer have a vote on it.
ask-void-half-kept = Your bill failed its vote: you tried.
ask-void-lost = You lost the race: the promise no longer stands.

## Outcomes of the asks' commands
outcome-ask-accepted = You said yes: { $ask }.
outcome-ask-declined = You said no: { $ask }.
outcome-promised = You made a promise: { $promise }.
outcome-casework-sorted = You sorted out { $name }'s problem at the services desk.
outcome-casework-chased = You chased it up for { $name }, but it is not sorted yet. ({ $odds }% chance each try.)
action-casework = Sort it out

## A favour owed over an ask (the favour ledger)
favour-ask = A promise kept or made over an ask

## The Legacy record
legacy-campaign_promise-kept = Kept a campaign promise: { $promise }.
legacy-campaign_promise-broken = Broke a campaign promise: { $promise }.

## Events
event-player-deed = You got something done.
event-ask-offered = Someone asked you for something: { $ask }.
event-ask-accepted = You made a promise: { $ask }.
event-ask-resolved = { $outcome ->
        [kept] You kept your word: { $ask }.
        [broken] You broke your word: { $ask }.
        [declined] You said no: { $ask }.
       *[void] It no longer stands: { $ask }.
    }

## Interface: the Today block, casework, the promise picker, the story sheet (E7)
ui-asks = Asks and promises
ui-asks-none = Nobody is waiting on you for anything.
ui-asks-recent = Lately
ui-ask-count = { $n ->
    [one] 1 ask or promise waiting
   *[other] { $n } asks or promises waiting
}
ui-ask-from = From { $who }
ui-ask-asker-party = { $party }, through { $name }
ui-ask-respond-by = Answer by { $date } · { $n ->
    [0] today
    [one] 1 day left
   *[other] { $n } days left
}
ui-ask-due = Due { $date } · { $n ->
    [0] today
    [one] 1 day left
   *[other] { $n } days left
}
ui-ask-next = Next: { $need }
ui-ask-progress = { $done } of { $total } done
ui-ask-watch = Nothing to do but keep to it until the deadline.
ui-ask-accept = Accept
ui-ask-decline = Decline
ui-ask-accept-label = Accept: { $ask }
ui-ask-decline-label = Decline: { $ask }
ui-ask-go-label = Go to { $place } for: { $ask }
ui-ask-talk = Talk to { $name }
ui-ask-casework = Sort it out
ui-ask-casework-label = Sort it out at the town hall: { $ask } ({ $pct }% chance)
ui-ask-odds = { $pct }% chance it is sorted this try; a try takes a slot.
ui-ask-details = What it means
ui-ask-if-kept = If you keep it
ui-ask-if-broken = If you break it
ui-ask-if-declined = If you say no, or do not answer
ui-ask-public = Public
ui-ask-big = Big ask
ui-ask-promise = Campaign promise
ui-ask-status-offered = Waiting for your answer
ui-ask-status-accepted = Promised
ui-ask-status-kept = Kept
ui-ask-status-broken = Broken
ui-ask-status-declined = Declined
ui-ask-status-lapsed = Lapsed unanswered
ui-ask-status-void = No longer stands
ui-casework = Casework
ui-casework-help = The services desk sorts out residents' problems you have promised to help with.
ui-casework-none = No casework waits on you.
ui-promise-make = Make a campaign promise
ui-promise-picker = Campaign promises
ui-promise-picker-intro = Promise the voters something. It wins support where the worry is felt; breaking it later costs you, in public.
ui-promise-made = { $made } of { $max } promises made in this race
ui-promise-to = Made to { $voters }
ui-promise-choose = Promise this
ui-promise-choose-label = Promise: { $promise }
ui-promise-none = There is nothing more to promise in this race.
ui-promise-due-term = Due at the end of the term
ui-promise-due-budget-year = Due by the end of the next budget year
ui-promise-due-days = Due { $due_days } days after election day
ui-story-promises = Promises
ui-story-promises-none = You have made no campaign promises yet.
ui-story-promises-tally = Made { $made } · kept { $kept } · broken { $broken }
ui-story-asks-tally = Asks: accepted { $accepted } · kept { $kept } · broken { $broken } · declined { $declined }
