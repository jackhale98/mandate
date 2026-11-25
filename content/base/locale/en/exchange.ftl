## Exchange scenes (DEC-112; docs/design/EXCHANGE_SCENES.md §9.3).
##
## The engine names every key here and gives its arguments; it writes no
## sentence. A scene's own keys start with its `text` prefix
## (`exchange-debate`):
##   <text>-prompt-<source>-<1..3>   what the other side puts to the player
##   <text>-say-<kind>-<1..2>        the player's line, first person
##   <text>-reply-<kind>             the other side's answer to that kind
##   <text>-result-<outcome>         how it ended, in the event log
##   <text>-headline-<outcome>       the paper's headline
##   <text>-open, -left, -offer      turning up, walking out, the day it is on
## The shared set (`exchange-*` with no scene) is the screen's own: labels,
## costs and risks, locked lines, how the room took it.
##
## Register: a prompt is one or two spoken sentences with a fact in them, at
## most 140 characters. A line of the player's is first person and short, at
## most 120. A label is at most 60.
##
## Arguments: the cast by role (`$rival`, `$hall`, `$chair`, `$backer`,
## `$ally`); `$law`, `$promise`, `$fact`, `$worry` (the text of the key the
## engine names: a worry reads mid-sentence, "the rent"); `$group` (an area's
## name); `$other` and `$second` (who is across, who speaks for you); `$n`,
## `$of`, `$round`; `$clock_day` and `$clock_month` (the scene's day).

## The shared set: labels (one a kind of line).
exchange-label-record = Answer with your record: { $law }
exchange-label-heard = What you heard in { $group }
exchange-label-turn = Turn it on their record
exchange-label-promise = Promise it now: { $promise }
exchange-label-act = Say you will table it: { $law }
exchange-label-own = Own it
exchange-label-deny = Deny it
exchange-label-offer = Stay for every question
exchange-label-second = Let { $second } speak for you
exchange-label-proof = Show them
exchange-label-pass = Move on

## The locked line: one a round, and always a route to it.
exchange-locked-record = Answer with your record: a law of yours in force would speak for you here.
exchange-locked-heard = What you heard on the doorstep: someone who knows { $group } could tell you what it worries about.
exchange-locked-turn-unknown = Turn it on their record: the paper's cuttings would tell you where { $other } stands.
exchange-locked-turn-nothing = Turn it on their record: there is nothing on { $other } that would stand up.
exchange-locked-promise = Promise it now: you have no promise left to give this campaign.
exchange-locked-act = Say you will table it: a post that drafts bills would let you.
exchange-locked-second = Let someone speak for you: someone with a name in town might, if you asked first.
exchange-locked-proof = Show them: you would need something in your hand.

## Costs, risks and notes. Always shown, always exact.
exchange-cost-promise = A promise, { $n } of { $of } this campaign. Breaking it costs more than keeping it gains.
exchange-cost-remembers = It sours { $other } on you, past tonight.
exchange-cost-aired = Once said, it is out. The paper will make a story of it.
exchange-cost-draft = A draft goes in under your name.
exchange-cost-retires = They cannot use it against you again tonight.
exchange-cost-standing = Costs { $n } Standing.
exchange-cost-denial = A denial on the record. If it comes apart, it comes apart in public.
exchange-cost-slot = Costs a second slot tonight.
exchange-cost-second = { $second } spends their name on you, and will mind if you lose.
exchange-cost-public = What you show is public from now on.
exchange-risk-secondhand = You have this second-hand. It may not stand up.
exchange-note-votes = You have the votes for it.
exchange-note-cools = It cools the room.

## How the room took it.
exchange-room-well = The room is with you on that.
exchange-room-some = Some of the room nods.
exchange-room-flat = The room gives you nothing back.
exchange-room-badly = You can hear the room turn.
exchange-room-failed = It does not stand up, and the room knows it.

## The audience strip and the result card.
exchange-group-elsewhere = elsewhere
exchange-class-0 = lower incomes
exchange-class-1 = middle incomes
exchange-class-2 = higher incomes
exchange-answer-yes = They have an answer to this.
exchange-answer-no = They have no answer to this.
exchange-answer-maybe = They may have an answer to this.
exchange-outcome-won = Won
exchange-outcome-split = Split
exchange-outcome-lost = Lost
exchange-outcome-absent = Empty chair

## A time skip stops when a scene opens.
exchange-interrupt = You are expected somewhere. If you skip past the day, your chair is empty.

## A round in the event log and in "Why?".
event-arc-exchange-round = { $stood ->
        [yes] Round { $round }: you said your piece, and it stood.
       *[no] Round { $round }: you said your piece, and it did not stand up.
    }
event-arc-exchange-unanswered = Round { $round } went unanswered.

## The debate (arc.rival, the showdown).
exchange-debate-offer = { $rival } has called a debate at { $hall } on { $clock_day } { $clock_month }. If you are not there, your chair stays empty.
exchange-debate-open = You take your place at { $hall }. { $rival } is across from you, and the room is full.
exchange-debate-left = You walked out of { $hall } before it was over.

## The rival's prompts. Three wordings each; the engine draws one.
exchange-debate-prompt-broken_promise-1 = "'{ $promise }.' Your words, on your leaflet. Tell this hall what became of them."
exchange-debate-prompt-broken_promise-2 = "You stood up and promised: '{ $promise }'. It did not happen. Why believe the next one?"
exchange-debate-prompt-broken_promise-3 = "'{ $promise }.' People here kept the leaflet. When did you stop meaning it?"
exchange-debate-prompt-vote_against_lean-1 = "You voted on { $law }. Ask { $group } what that vote did for them."
exchange-debate-prompt-vote_against_lean-2 = "{ $group } wanted one thing from you on { $law }, and you gave them the other."
exchange-debate-prompt-vote_against_lean-3 = "When { $law } came up, { $group } watched how you voted. They remember."
exchange-debate-prompt-failed_law-1 = "{ $law } was yours. Where is it now, and what did anyone get from it?"
exchange-debate-prompt-failed_law-2 = "You put your name to { $law }. Name one street that is better for it."
exchange-debate-prompt-failed_law-3 = "{ $law } was meant to be your answer. People are still waiting for it to start."
exchange-debate-prompt-thin_record-1 = "What have you done? I have looked. There is a name on a door and very little behind it."
exchange-debate-prompt-thin_record-2 = "You ask for votes on your record. So read it out. It will not take us long."
exchange-debate-prompt-thin_record-3 = "People in this hall work for a living. What have you done with your time?"
exchange-debate-prompt-scandal-1 = "The paper has been full of it: { $fact }. This hall is owed an answer tonight."
exchange-debate-prompt-scandal-2 = "Before anything else, there is the story about { $fact }. Is it true?"
exchange-debate-prompt-scandal-3 = "You would like us to forget { $fact }. Nobody here has."
exchange-debate-prompt-hidden_fact-1 = "There is something this hall has not been told. It concerns you, and { $fact }."
exchange-debate-prompt-hidden_fact-2 = "I would rather have left this alone. People should hear about { $fact } before they vote."
exchange-debate-prompt-hidden_fact-3 = "Ask why your candidate has never mentioned { $fact }. I can tell you why."
exchange-debate-prompt-absent-1 = "This campaign has put questions to you, and you let them pass. Why should tonight be different?"
exchange-debate-prompt-absent-2 = "People came to you these last weeks and went away with nothing. They are in this hall."
exchange-debate-prompt-absent-3 = "You have had your chances to speak up this campaign. You were busy, I hear."
exchange-debate-prompt-governing_record-1 = "Your party runs this town. Go to { $group } and ask how that is working out."
exchange-debate-prompt-governing_record-2 = "You cannot stand apart from your own side. { $group } knows who has been in charge."
exchange-debate-prompt-governing_record-3 = "What is wrong in { $group } went wrong with your people in the chairs."
exchange-debate-prompt-top_worry-1 = "Let me ask what this room came to ask. On { $worry }: what have you done, and what will you do?"
exchange-debate-prompt-top_worry-2 = "Every hand I can see is up about { $worry }. What is your answer?"
exchange-debate-prompt-top_worry-3 = "One question keeps coming from the floor, and it is { $worry }. What do you say?"

## The player's lines. Two wordings each.
exchange-debate-say-record-1 = "Look at { $law }. It is in force, it is mine, and it is doing its work."
exchange-debate-say-record-2 = "I will answer with what is on the books. { $law } is law because I carried it."
exchange-debate-say-heard-1 = "I have stood on doorsteps in { $group }. What I heard there was { $worry }, every time."
exchange-debate-say-heard-2 = "{ $group } told me what keeps it awake: { $worry }. I listened, and I am here."
exchange-debate-say-turn-1 = "Since records are the subject: { $other }, shall we read yours?"
exchange-debate-say-turn-2 = "{ $other } has a record too. It is shorter than the speeches."
exchange-debate-say-promise-1 = "Then hold me to this, in front of everyone: { $promise }."
exchange-debate-say-promise-2 = "Write it down. { $promise }. You can count the days."
exchange-debate-say-act-1 = "I will table { $law } myself. The draft goes in this week."
exchange-debate-say-act-2 = "Talk costs nothing. { $law } will be on the clerk's desk with my name on it."
exchange-debate-say-own-1 = "That is true, and it is on me. I got it wrong."
exchange-debate-say-own-2 = "I will not dress it up. It happened, and I answer for it."
exchange-debate-say-second-1 = "You have heard enough from me. { $second } asked to say a word."
exchange-debate-say-second-2 = "Do not take my word for it. { $second } knows this town better than either of us."
exchange-debate-say-pass-1 = "I have said what I have to say on that. Next question."
exchange-debate-say-pass-2 = "We could go round on that all night. Let us move on."

## The rival's answers, where they hold one.
exchange-debate-reply-record = "And who paid for it? Ask the people { $law } left worse off."
exchange-debate-reply-promise = "Another promise. This hall has heard your promises before."
exchange-debate-reply-turn = "You would know. Your own record has the same hole in it."
exchange-debate-reply-act = "You have had years in that chair. Now there is a draft?"
exchange-debate-reply-second = "A kind word from a friend. { $second } does not live where these people live."

## How it ended: the event log's line and the paper's headline.
exchange-debate-result-won = You took the debate at { $hall }. { $rival } left before the chairs were stacked.
exchange-debate-result-split = The debate at { $hall } ended even. You won some of the room and lost the rest.
exchange-debate-result-lost = The debate at { $hall } went to { $rival }.
exchange-debate-result-absent = { $rival } debated an empty chair at { $hall }. You were not there.
exchange-debate-headline-won = Debate at { $hall }: a clear win over { $rival }
exchange-debate-headline-split = Debate at { $hall } splits the room
exchange-debate-headline-lost = { $rival } takes the debate at { $hall }
exchange-debate-headline-absent = { $rival } debates an empty chair at { $hall }

## Preparing for it (the showdown's other beats).
talk-arc-rival-rehearse = Ask for a rehearsal before the debate
talk-arc-rival-rehearse-say = "Play { $rival } for an hour. Hit me with whatever they will."
talk-arc-rival-rehearse-yes = "Sit down, then. First question: what have you done? No, slower. Again."
talk-arc-rival-rehearse-no = "I have not the time this week. Ask me when we know each other better."
talk-arc-rival-backer = Ask them to speak for you at the debate
talk-arc-rival-backer-say = "Would you say a word for me at { $hall }? They listen to you there."
talk-arc-rival-backer-yes = "I will stand up and say it. Do not make me regret it."
talk-arc-rival-backer-no = "My name is all I have in this town. I am not lending it yet."
visit-arc-rival-cuttings = Read the cuttings on your rival
outcome-arc-rival-cuttings = You went through the cuttings on { $rival } at { $paper }: every vote and every quote, back to the start.
