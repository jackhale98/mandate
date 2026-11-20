# Media, scandals, and the legal path (DEC-034, DEC-006). The engine emits
# keys and facts; every sentence a player reads is here (DEC-054).

## Outlets. A town paper is named after its office (DEC-079).
outlet-local_paper = { $outlet_place }
outlet-regional_paper = { $outlet_place } Regional News
outlet-national_broadcaster = National Broadcasting
outlet-social = Chatter

## What a hidden fact is (noun phrases: headlines and memories quote them).
fact-bribe = a cash bribe
fact-campaign-funds = campaign money spent on household bills
fact-leaked-file = a leaked confidential file
fact-donor-deal = a donor's quiet deal
fact-back-room-deal = a late-night back-room deal
fact-late-night-money = late-night fundraising
fact-buried-report = a buried inspection report
fact-donation = an undeclared donor's favour
fact-smear = a doctored quote in an attack ad
fact-consultancy-fee = a developer's "consultancy fee"
fact-steered-contract = a contract steered to a donor
fact-paid-crowd = a crowd paid to fill a town hall
fact-job-for-vote = a council vote traded for a relative's job

## Borderline and illegal choices.
dilemma-pollution-bury = Have the inspection report quietly shelved
dilemma-pollution-bury-after = The report went into a drawer. { $owner } was grateful, the plant kept running, and the file is still somewhere.
dilemma-attack-ad-smear = Run it with a doctored quote that hits harder
dilemma-attack-ad-smear-after = The doctored ad ran. It landed hard on { $rival }, and the party loved it. The original quote is still on tape somewhere.

dilemma-consultancy-fee = the consultancy fee
dilemma-consultancy-fee-situation = { $developer } ({ $developer_role }) wants your support at the council and offers a "consultancy fee" for your advice. Cash, no paperwork.
dilemma-consultancy-fee-take = Take the fee
dilemma-consultancy-fee-take-after = You took { $developer }'s money. Your campaign chest is fuller; you owe them, and a bank somewhere has a record of the transfer.
dilemma-consultancy-fee-decline = Decline it
dilemma-consultancy-fee-decline-after = You turned the fee down. { $developer } took it badly; your party liked that you kept clean.
dilemma-consultancy-fee-report = Report the offer to the town's auditors
dilemma-consultancy-fee-report-after = You reported the offer. The town heard you cannot be bought; { $developer } will not forget it, and the party winced at the publicity.
dilemma-consultancy-fee-lapse = Say nothing and let it pass
dilemma-consultancy-fee-lapse-after = You never answered. { $developer } took the silence as a no.
scene-consultancy-developer = { $developer }: "You know how it works. Advice is worth paying for."
scene-consultancy-developer-fresh = { $developer }: "We haven't worked together yet. We could start as we mean to go on."

dilemma-steer-contract = the contract
dilemma-steer-contract-situation = The { $law } contract is out to tender. { $donor } ({ $donor_role }) has a firm in the running, and a generous campaign donation ready if it wins.
dilemma-steer-contract-steer = Make sure their firm wins
dilemma-steer-contract-steer-after = You steered the { $law } contract to { $donor }'s firm. The donation arrived the next week. So did the losing bidders' questions.
dilemma-steer-contract-tender = Insist on an open tender, and sit on the panel
dilemma-steer-contract-tender-after = You sat through the tender panel. It took a day; the contract went to the best bid, and { $donor } was not pleased.
dilemma-steer-contract-leave = Leave it to the clerk
dilemma-steer-contract-leave-after = You left it to the clerk. { $donor } felt you could have done more.
scene-contract-donor = { $donor }: "Nobody needs to know who suggested it."
scene-contract-donor-fresh = { $donor }: "I support people who support this town."

dilemma-paid-crowd = the empty hall
dilemma-paid-crowd-situation = Tomorrow's town hall meeting looks thin. { $fixer } ({ $fixer_role }) can fill the seats with paid supporters who cheer on cue.
dilemma-paid-crowd-pay = Pay them to fill the hall
dilemma-paid-crowd-pay-after = The hall was packed and loud, and the photos looked great. Twenty people have a story about how they were paid.
dilemma-paid-crowd-honest = Spend the day drumming up real supporters
dilemma-paid-crowd-honest-after = You spent the day on the phone and on the doorsteps. The hall was half full, and every one of them meant it.
dilemma-paid-crowd-skip = Hold it as it is
dilemma-paid-crowd-skip-after = The hall was thin. The photos were worse.
scene-paid-crowd-fixer = { $fixer }: "Twenty people, fifty each, and they bring signs."

dilemma-personal-bills = the household bills
dilemma-personal-bills-situation = You are worn out and the bills at home are piling up. { $treasurer } ({ $treasurer_role }) keeps the campaign's card, and nobody checks it very closely.
dilemma-personal-bills-use = Put the household bills on the campaign card
dilemma-personal-bills-use-after = The bills are paid and you slept for the first time in weeks. The card statements are filed with the campaign's accounts.
dilemma-personal-bills-ask = Ask the party for an advance
dilemma-personal-bills-ask-after = The party advanced you a little, and made a note of who asked.
dilemma-personal-bills-carry-on = Grit your teeth and carry on
dilemma-personal-bills-carry-on-after = You carried on. It showed.
scene-bills-treasurer = { $treasurer }: "It's your campaign. Who's going to look?"

dilemma-job-for-relative = the nephew's job
dilemma-job-for-relative-situation = { $sponsor } needs one more vote for the { $bill } bill, and mentions a job opening at the town hall for your nephew.
dilemma-job-for-relative-deal = Vote yes; your nephew gets the job
dilemma-job-for-relative-deal-after = You voted for the { $bill } bill, and your nephew started on Monday. Your family is relieved. The hiring file has both your names in it.
dilemma-job-for-relative-refuse = Refuse the deal
dilemma-job-for-relative-refuse-after = You refused. { $sponsor } will remember; your party liked the backbone.
dilemma-job-for-relative-noncommittal = Change the subject
dilemma-job-for-relative-noncommittal-after = You changed the subject. { $sponsor } took the hint, and not kindly.
scene-relative-sponsor = { $sponsor }: "Family is family. One vote."
scene-relative-sponsor-fresh = { $sponsor }: "We should help each other."

## Raised decisions: a story about you, an investigation of you.
dilemma-scandal-response = the newspaper story
dilemma-scandal-response-situation = { $outlet } has the story: { $fact }. How you answer decides how long it runs.
dilemma-scandal-response-deny = Deny it flatly
dilemma-scandal-response-deny-after = You denied it. If they had little, the story died; if they had a lot, the denial became the story.
dilemma-scandal-response-explain = Give a full explanation
dilemma-scandal-response-explain-after = You spent a day explaining. Some of it landed; the story cooled.
dilemma-scandal-response-apologise = Apologise in public
dilemma-scandal-response-apologise-after = You apologised. The story ended faster, the party was embarrassed, and prosecutors read the apology too.
dilemma-scandal-response-counter = Hit back at the paper
dilemma-scandal-response-counter-after = You went after { $outlet }. Your supporters rallied; the paper will dig harder now.
dilemma-scandal-response-nothing = Say nothing
dilemma-scandal-response-nothing-after = You said nothing. The story ran its course.

dilemma-legal-response = the investigation
dilemma-legal-response-situation = Prosecutors have opened an investigation into { $fact }. You can help them, fight them, or point them elsewhere.
dilemma-legal-response-cooperate = Cooperate fully
dilemma-legal-response-cooperate-after = You cooperated. They found more, but charges are less likely and any sentence lighter. The party hated the headlines.
dilemma-legal-response-lawyer = Hire a good lawyer
dilemma-legal-response-lawyer-after = You hired a lawyer. It was not cheap; your odds at every stage improved.
dilemma-legal-response-stonewall = Stonewall
dilemma-legal-response-stonewall-after = You gave them nothing. If they had little, it may fade; if they had a lot, obstruction makes it worse.
dilemma-legal-response-blame = Blame { $ally }
dilemma-legal-response-blame-after = You pointed investigators at { $ally }. It may save you; { $ally } will never forgive it.
dilemma-legal-response-wait = Wait and see
dilemma-legal-response-wait-after = You waited. The investigation went on without you.

## Memory.
memory-blamed-me = You blamed them for your own wrongdoing ({ $year }).
scene-quote-blamed-me = You put your own mess on my doorstep, back in { $year }.

## Shady actions (DEC-034, DEC-073): the risk is shown before acting.
action-take-bribe = Take an envelope
action-dip-funds = Pay bills from campaign funds
action-leak-file = Leak a rival's file
tradeoff-take-bribe = Cash for your campaign tonight, from someone who will want something. It is a crime.
tradeoff-dip-funds = The campaign card pays your household bills: rest without losing a day. It is misconduct, and there is a paper trail.
tradeoff-leak-file = A reporter gets your rival's confidential file. If there is dirt in it, it comes out now; if not, a rumour starts. Leaking it is misconduct.
outcome-take-bribe = { $name } slipped you an envelope: { $amount } for the campaign.
outcome-dip-funds = The bills are paid from the campaign card. You feel better.
outcome-leak-file-exposed = The file had something in it. { $name }'s secret is out.
outcome-leak-file-rumour = There was nothing solid in { $name }'s file, but a rumour is spreading.
ui-risk-exposure = Risk: { $risk } chance this comes out within a year ({ $severity }).
ui-shady-no-offer = Nobody is offering tonight.
ui-shady-no-rival = You have no rival to leak about.
ui-shady-pays = Pays { $amount }.
ui-shady-heals = Health +{ $n }.
ui-shady-costs = Costs { $amount }.
service-newspaper = The newspaper

## Risk bands and severity.
risk-low = a low
risk-real = a real
risk-high = a high
severity-ethics = an ethics lapse
severity-misconduct = misconduct in office
severity-crime = a crime

## Decision trade-off lines.
effect-hidden = Risk: { $risk } chance this comes out within a year ({ $severity })
effect-hidden-done = Out of sight for now: { $risk } chance it comes out within a year
effect-responded = You answered: { $response }
response-none = said nothing
response-deny = denied it
response-explain = explained
response-apologise = apologised
response-counter-attack = hit back
response-cooperate = cooperated
response-lawyer = hired a lawyer
response-stonewall = stonewalled
response-blame-ally = blamed an ally
effect-respond-none = The story runs its course
effect-respond-deny = A flat denial
effect-respond-explain = A full explanation
effect-respond-apologise = A public apology
effect-respond-counter-attack = An attack on the paper
effect-legal-none = The investigation goes on
effect-legal-cooperate = Full cooperation
effect-legal-lawyer = A good lawyer
effect-legal-stonewall = Stonewalling
effect-legal-blame-ally = Blaming an ally
effect-respond-none-gain = Nothing you say can be quoted back at you
effect-respond-none-cost = The story runs its course
effect-respond-deny-gain = If they have little, the story dies
effect-respond-deny-cost = If they have a lot, the denial unravels, and so does your standing with the party
effect-respond-explain-gain = The story cools
effect-respond-explain-cost = Some of it will be quoted back at you
effect-respond-apologise-gain = The story ends sooner
effect-respond-apologise-cost = Prosecutors read it as an admission
effect-respond-counter-attack-gain = Your side rallies
effect-respond-counter-attack-cost = The paper digs harder from now on
effect-legal-none-gain = Nothing you say can be used against you
effect-legal-none-cost = The investigation goes on without you
effect-legal-cooperate-gain = Charges less likely, and any sentence lighter
effect-legal-cooperate-cost = They find more than they had
effect-legal-lawyer-gain = Better odds at every stage
effect-legal-lawyer-cost = Hiring one is noticed
effect-legal-stonewall-gain = Helps if they have little
effect-legal-stonewall-cost = If they have a lot, obstruction makes it worse
effect-legal-blame-ally-gain = Charges much less likely
effect-legal-blame-ally-cost = Your ally takes the heat, and never forgets it

## Headlines (an outlet's story; `name` is its subject).
headline-scandal-breaks = { $name } accused: { $fact }
headline-scandal = More on { $name } and { $fact }
headline-response-none = { $name } silent on { $fact }
headline-response-deny = { $name } denies { $fact }
headline-response-explain = { $name } explains { $fact }
headline-response-apologise = { $name } apologises for { $fact }
headline-response-counter-attack = { $name } hits back over { $fact } claims
headline-backfire = { $name }'s denial unravels: { $fact }
headline-case-investigation = Prosecutors look into { $name } over { $fact }
headline-case-charged = { $name } charged over { $fact }
headline-case-dropped = Inquiry into { $name } dropped
headline-case-acquitted = { $name } acquitted
headline-case-convicted = { $name } convicted over { $fact }: { $sentence }
headline-rumour = Unverified claims about { $name } spread online
headline-correction = Claims about { $name } were false, paper says
headline-statement = { $name } speaks to the press
headline-statement-bill = { $name } makes the case for { $bill }
# Town news (a story about something that happened in town: the event's
# arguments; `name` is a race's winner). Third person, no full stop.
headline-plant-closure = { $sector } layoffs in { $town }: { $jobs } jobs lost
headline-disaster = { $hazard } strikes { $town }
headline-policy-enacted = { $policy } bill passes in { $town }, { $yes }–{ $no }
headline-policy-override = { $town } overrides the veto on the { $policy } bill, { $yes }–{ $no }
headline-policy-vetoed = { $policy } bill vetoed in { $town }
headline-policy-defeated = { $policy } bill voted down in { $town }, { $yes }–{ $no }
headline-player-elected = { $name } wins in { $town }
headline-player-elected-mayor = { $name } elected mayor of { $town }
headline-player-defeated = { $name } wins in { $town }
headline-mayor-ousted = { $town } throws out its mayor: { $name } wins
headline-mayor-elected = { $name } elected mayor of { $town }
headline-councillor-ousted = { $name } unseats a sitting councillor in { $town }
headline-councillor-elected = { $name } wins a council seat in { $town }
sentence-fine = a fine
sentence-removed = removal from office
sentence-prison = prison
sentence-disqualified = disqualification from office
framing-hostile = hostile
framing-straight = straight
framing-sympathetic = sympathetic
credibility-low = low credibility
credibility-medium = fair credibility
credibility-high = high credibility

## Scandals and cases in the pocket.
alert-scandal = A story about you is running: { $fact }.
alert-case = You face a case: { $stage }.
case-stage-investigation = under investigation
case-stage-charged = charged, awaiting trial
case-stage-adjourned = trial adjourned
case-stage-dropped = dropped
case-stage-acquitted = acquitted
case-stage-convicted = convicted
heat-fading = fading
heat-running = running
heat-burning = burning
ending-prison = a long prison sentence
ending-disqualified = disqualification from office

## Events.
event-scandal-broke = Scandal: { $name } and { $fact } ({ $town }).
event-case-opened = Prosecutors opened an investigation into { $name }.
event-case-developed = The case against { $name }: { $stage }.
event-player-imprisoned = You were sentenced to { $years } years in prison, until { $year }.
event-player-released = You were released from prison.
event-career-ended = Your career is over: { $reason }.
event-rumour-spread = A rumour about { $name } began to spread.
event-rumour-corrected = The rumour about { $name } was corrected.

## The newspaper, the pocket, and the end of a run (web).
ui-newspaper-empty = Nothing about your town this week.
ui-newspaper-note = Outlets choose what to run and how to frame it: coverage is not the truth.
ui-about-you = about you
ui-news = In the news
ui-legal = Scandals and the law
ui-scandals = Stories about you
ui-cases = Cases
ui-scandal-line = { $fact } ({ $severity }), since { $date }
ui-scandal-over = over
ui-case-line = { $fact }: { $stage }
ui-case-next = next step by { $date }
ui-case-odds-charge = { $risk } chance of charges
ui-case-odds-convict = { $risk } chance of conviction
ui-years = { $n ->
    [one] { $n } year
   *[other] { $n } years
}
ui-in-prison = You are in prison until { $date }.
ui-in-prison-why = Convicted over { $fact }. Out of politics for { $n ->
    [one] a year
   *[other] { $n } years
}; ending the turn lets the time pass. You can run again when you are out.
ui-serve-sentence = Serve your sentence
ui-career-over = Your career is over
ui-career-over-why = Convicted over { $fact }: { $reason }.
ui-career-over-note = The run has ended. The full Legacy screen comes later; your save keeps the story so far.
