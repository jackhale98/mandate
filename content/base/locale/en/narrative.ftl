# Narrative: the director, dilemmas, requests, goals, storylines, scenes
# (DEC-033, DEC-077 §4, DEC-032, DEC-035). Arguments: each cast role's name
# as $<role> (a bill or law: its policy name), a person's role as
# $<role>_role, plus $town, $crisis, and $player.

## Storyteller profiles (DEC-033 addendum)
storyteller-classic = Classic
storyteller-classic-desc = Tension rises through each chapter to one climax. Hazards wait while your town already has its hands full.
storyteller-unfiltered = Unfiltered
storyteller-unfiltered-desc = Pure chance. Storms and shocks arrive as they fall; nothing waits its turn.
storyteller-steady = Steady
storyteller-steady-desc = Fewer crises and fewer decisions: one emergency at a time.

## Memories recorded by decisions (on the sheet; see base.ftl for $subject_kind)
memory-stood-with-me = { $subject_kind ->
    [decision] You stood with them over { $subject } ({ $year }).
   *[other] You stood with them ({ $year }).
}
memory-stood-against-me = { $subject_kind ->
    [decision] You took the other side over { $subject } ({ $year }).
   *[other] You took the other side from them ({ $year }).
}
memory-ignored-me = { $subject_kind ->
    [decision] You left them waiting over { $subject } ({ $year }).
   *[other] You left them waiting ({ $year }).
}
subject-decision = { $title }

## A memory quoted to the player's face, opening a scene's line ($memory):
## first person, the year said, not bracketed. Each is a whole sentence.
## $subject and $subject_kind as on the sheet (base.ftl).
scene-quote-backed-my-bill = { $subject_kind ->
    [bill] You backed my bill, { $subject }, in { $year }.
   *[other] You backed my bill in { $year }.
}
scene-quote-opposed-my-bill = { $subject_kind ->
    [bill] You voted against my bill, { $subject }, in { $year }.
   *[other] You voted against my bill in { $year }.
}
scene-quote-promised-favour = { $subject_kind ->
    [bill] You promised me a favour over the { $subject } bill.
    [decision] You promised me a favour over { $subject }.
   *[other] You promised me a favour, back in { $year }.
}
scene-quote-kept-promise = { $subject_kind ->
    [bill] You kept your word on the { $subject } bill.
    [decision] You kept your word over { $subject }.
   *[other] You kept your word to me once.
}
scene-quote-broke-promise = { $subject_kind ->
    [bill] You broke your word on the { $subject } bill.
    [decision] You broke your word over { $subject }.
   *[other] You broke your word to me once.
}
scene-quote-broke-ranks = { $subject_kind ->
    [bill] You broke with the party over the { $subject } bill.
   *[other] You broke with the party in { $year }.
}
scene-quote-lobbied-me = { $subject_kind ->
    [bill] You talked me round on the { $subject } bill.
    [decision] You talked me round over { $subject }.
   *[other] You talked me round once.
}
scene-quote-pressed-me = { $subject_kind ->
    [bill] You leaned on me over the { $subject } bill, and it didn't work.
    [decision] You leaned on me over { $subject }, and it didn't work.
   *[other] You leaned on me once, and it didn't work.
}
scene-quote-defeated-me = You beat me in { $year }.
scene-quote-lost-to-me = I beat you in { $year }, if you remember.
scene-quote-vetoed-my-bill = { $subject_kind ->
    [bill] You vetoed my bill, { $subject }.
   *[other] You vetoed my bill.
}
scene-quote-signed-my-bill = { $subject_kind ->
    [bill] You signed my bill, { $subject }, into law.
   *[other] You signed my bill into law.
}
scene-quote-met = We met in { $year }.
scene-quote-scandal = I read about you in the paper in { $year }.
scene-quote-stood-with-me = { $subject_kind ->
    [decision] You stood with me over { $subject }.
   *[other] You stood with me once.
}
scene-quote-stood-against-me = { $subject_kind ->
    [decision] You took the other side over { $subject }.
   *[other] You took the other side from me once.
}
scene-quote-ignored-me = { $subject_kind ->
    [decision] You left me waiting over { $subject }.
   *[other] You left me waiting once.
}

## NPC goals
goal-organise-workers = Organising the workers
goal-oppose-player = Out to stop you
goal-seek-backing = Looking for your support
goal-champion-fix = Pushing for a fix
goal-traders-help = Rallying the traders

## Dilemmas: titles are noun phrases (they are also what memories are about,
## so the people in them quote them: "You stood with me over { $title }").
## Name the business, not the player's view of it: "the truce offer", not
## "the rival's offer", which reads oddly from the rival's own mouth.
dilemma-plant-layoffs = the layoffs at the plant
dilemma-plant-layoffs-situation = Jobs are going in { $town }, and the town is choosing sides. { $owner } ({ $owner_role }) wants the council to hold off on anything that raises an employer's costs. { $steward } ({ $steward_role }) wants you on the picket line. In { $hood }, people are asking where you stand.
dilemma-plant-layoffs-back-owner = Back { $owner }: no new costs on employers while jobs are at risk
dilemma-plant-layoffs-back-owner-after = You sided with { $owner }. The party's business wing noticed, and so did { $steward }, who has not forgotten. Low-income voters in your ward cooled on you.
dilemma-plant-layoffs-back-workers = Stand with the workers and draft a hardship fund for laid-off families
dilemma-plant-layoffs-back-workers-after = You drafted a hardship fund and stood with { $steward }. Laid-off families took heart; { $owner } and the party's donors did not.
dilemma-plant-layoffs-broker-talks = Spend a day getting { $owner } and { $steward } into one room
dilemma-plant-layoffs-broker-talks-after = You kept { $owner } and { $steward } at one table for most of a day. Nothing was signed, and neither of them blamed you for that.
dilemma-plant-layoffs-stay-out = Stay out of it
dilemma-plant-layoffs-stay-out-after = You said nothing. { $steward } took it personally, and { $hood } noticed the silence. { $owner } was relieved.

dilemma-workers-campaign = the union's campaign
dilemma-workers-campaign-situation = { $steward } is organising the laid-off workers of { $town } and plans a march past the council chamber. They want you at the front.
dilemma-workers-campaign-march = March with them
dilemma-workers-campaign-march-after = You marched at the front. Working families saw you there, and so did the party, which did not like it.
dilemma-workers-campaign-statement = Send a statement of support
dilemma-workers-campaign-statement-after = Your statement was read out at the rally. It was warm, and it was not you.
dilemma-workers-campaign-decline = Stay away
dilemma-workers-campaign-decline-after = You stayed away. { $steward } marched without you and told the crowd why; better-off voters were quietly glad.

dilemma-housing-squeeze = the housing squeeze
dilemma-housing-squeeze-situation = Rents in { $town } have outrun wages. { $organiser } ({ $organiser_role }) brings families from { $hood } who are one rise away from leaving.
dilemma-housing-squeeze-upzone = Draft an upzoning bill so builders can add homes
dilemma-housing-squeeze-upzone-after = You drafted upzoning. Renters liked the idea; homeowners worried about towers next door.
dilemma-housing-squeeze-build = Draft an affordable housing programme
dilemma-housing-squeeze-build-after = You drafted a housing programme. { $organiser } is grateful; better-off voters wonder who pays, and the party worries about the bill.
dilemma-housing-squeeze-wait = Wait for the market to settle
dilemma-housing-squeeze-wait-after = You waited. Families in { $hood } did not feel heard, and { $organiser } said so. Homeowners liked the calm.

dilemma-disaster-relief = the rebuilding
dilemma-disaster-relief-situation = After the disaster, { $hood } is still clearing up. { $helper } ({ $helper_role }) is running a relief effort out of a borrowed hall and asks what you will do.
dilemma-disaster-relief-drive = Put your own campaign money and a day's work into the relief drive
dilemma-disaster-relief-drive-after = You spent campaign money and a long day hauling boxes. { $hood } saw who turned up.
dilemma-disaster-relief-defences = Draft flood defences, for the long term
dilemma-disaster-relief-defences-after = You drafted flood defences. It will take years, and people clearing up today wanted help now.
dilemma-disaster-relief-leave = Leave it to the town's emergency services
dilemma-disaster-relief-leave-after = You left it to the services and kept your strength. { $helper } noticed you were not there, and so did { $hood }.

dilemma-fiscal-hole = the town's budget hole
dilemma-fiscal-hole-situation = { $town } is borrowing to pay its bills. Mayor { $mayor } wants cuts and wants your vote for them. { $steward } ({ $steward_role }) says cuts will close services people rely on.
dilemma-fiscal-hole-cuts = Back the mayor's cuts
dilemma-fiscal-hole-cuts-after = You backed the cuts. The party welcomed your discipline; { $steward } and low-income families saw it differently.
dilemma-fiscal-hole-tax = Draft a property tax rise instead
dilemma-fiscal-hole-tax-after = You drafted a tax rise to keep services open. { $steward } is pleased; better-off homeowners are not.
dilemma-fiscal-hole-refuse = Refuse both
dilemma-fiscal-hole-refuse-after = You refused both. Your ward liked that you did not pick their pocket; the mayor and the party saw a councillor with no answer.

dilemma-tax-revolt = the tax bills
dilemma-tax-revolt-situation = Property tax is the talk of { $town }. { $trader } ({ $trader_role }) is collecting signatures for a cut. Mayor { $mayor } says the services it pays for are already thin.
dilemma-tax-revolt-cut = Draft a property tax cut
dilemma-tax-revolt-cut-after = You drafted a tax cut. { $trader } signed you up as a friend; the mayor did not, and families who rely on services worry.
dilemma-tax-revolt-defend = Defend the services the tax pays for
dilemma-tax-revolt-defend-after = You defended the services. { $trader } took it badly; the mayor and low-income families took note.
dilemma-tax-revolt-nothing = Say nothing
dilemma-tax-revolt-nothing-after = You kept your head down. { $trader } read it as a no, and so did the homeowners signing the petition.

dilemma-pollution = the pollution row
dilemma-pollution-situation = Pollution has become one of { $town }'s worries. { $voice } ({ $voice_role }) wants an inspection of { $owner }'s operation. { $owner } ({ $owner_role }) says an inspection will cost jobs.
dilemma-pollution-inspection = Demand an inspection
dilemma-pollution-inspection-after = You demanded an inspection. { $voice } thanked you; { $owner } will not, and workers fear for their jobs.
dilemma-pollution-jobs = Put jobs first
dilemma-pollution-jobs-after = You put jobs first. { $owner } is relieved; { $voice } and voters worried about the air are not.
dilemma-pollution-wait = Wait for better data
dilemma-pollution-wait-after = You asked for more data. { $voice } heard a delay; { $owner } heard a reprieve.

dilemma-building-protest = the building work
dilemma-building-protest-situation = The { $law } works have torn up streets in { $hood }. { $organiser } ({ $organiser_role }) has organised a protest and wants the work slowed down.
dilemma-building-protest-residents = Side with the residents and press the council to slow the work
dilemma-building-protest-residents-after = You sided with { $hood }. The residents cheered; the council and your party did not.
dilemma-building-protest-defend = Defend the project
dilemma-building-protest-defend-after = You defended the project. The party liked it; { $organiser } and { $hood } did not.
dilemma-building-protest-meet = Spend time with them on the street
dilemma-building-protest-meet-after = You spent an afternoon on the street with the protesters. It did not stop the diggers, but it softened the mood.
dilemma-building-protest-ignore = Ignore it
dilemma-building-protest-ignore-after = You stayed away and kept your evening. { $organiser } told the local paper you never came.

dilemma-favour-called-in = the favour called in
dilemma-favour-called-in-situation = { $creditor } has not forgotten the favour you owe. Their bill, { $bill }, is before the council, and they expect your vote.
dilemma-favour-called-in-pay = Vote for the bill and repay the favour
dilemma-favour-called-in-pay-after = You voted for the { $bill } bill and paid your debt. { $creditor } got the vote and counts you good for your word.
dilemma-favour-called-in-refuse = Vote against the bill
dilemma-favour-called-in-refuse-after = You voted against the { $bill } bill while you still owed { $creditor }. They called it a betrayal.
dilemma-favour-called-in-dodge = Abstain and hope it blows over
dilemma-favour-called-in-dodge-after = You abstained. Nobody was fooled: { $creditor } counts it as a promise broken.

dilemma-party-line = the party line
dilemma-party-line-situation = Your party has told you how to vote on the { $bill } bill, sponsored by { $sponsor }. Your ward leans the other way.
dilemma-party-line-party = Vote with the party
dilemma-party-line-party-after = You voted with the party. The whips were pleased; your ward was not.
dilemma-party-line-ward = Vote with your ward
dilemma-party-line-ward-after = You voted with your ward against the party line. The ward noticed; so did the whips.
dilemma-party-line-abstain = Abstain
dilemma-party-line-abstain-after = You abstained. It pleased no one.

dilemma-donor-offer = the donation
dilemma-donor-offer-situation = { $donor } ({ $donor_role }) offers a generous donation to your campaign. In return, they would like to be able to call you.
dilemma-donor-offer-accept = Take the money
dilemma-donor-offer-accept-after = You took { $donor }'s money. Your campaign is richer; you owe them, and a file somewhere notes the gift.
dilemma-donor-offer-decline = Decline, politely but firmly
dilemma-donor-offer-decline-after = You turned { $donor } down. They were offended; working voters liked hearing that you cannot be bought.
dilemma-donor-offer-lapse = Don't answer, and let the offer lapse
dilemma-donor-offer-lapse-after = You let the offer lapse. { $donor } felt snubbed, and the party wondered why you left money on the table.

dilemma-rival-offer = the truce offer
dilemma-rival-offer-situation = { $rival } ({ $rival_role }) suggests a quiet pact: no attacks from either side until the votes are counted.
dilemma-rival-offer-truce = Accept the pact
dilemma-rival-offer-truce-after = You shook on it. { $rival } warmed to you, you owe them for their restraint, and your party thinks you have gone soft.
dilemma-rival-offer-refuse = Refuse, in public
dilemma-rival-offer-refuse-after = You refused, loudly. The party liked the fight; { $rival } heard it in front of everyone.
dilemma-rival-offer-ignore = Ignore it
dilemma-rival-offer-ignore-after = You never answered and slept on it. { $rival } took the silence as an answer.

## Requests (an NPC's goal needs the player): the three became asks (asks.ftl).

## Scenes: what people say when you meet them about it
scene-plant-steward = "{ $memory } And now you want to talk about the plant?"
scene-plant-steward-fresh = "We've never really spoken, you and I. Today would be a good day to start."
scene-plant-owner = "Every cost the council adds is another shift I can't keep."
scene-workers-steward = "{ $memory } Are you marching with us or not?"
scene-workers-steward-fresh = "We're marching on Saturday. The front row has room."
scene-housing-organiser = "{ $memory } These families can't wait another year."
scene-housing-organiser-fresh = "Come and meet the people on this street. Then tell me about the market."
scene-disaster-helper = "{ $memory } The hall's full of donations, and there are four of us to sort them. Pick up a box."
scene-disaster-helper-fresh = "We're short of everything. Whatever you can bring, bring it."
scene-fiscal-mayor = "{ $memory } I need your vote on the cuts."
scene-fiscal-mayor-fresh = "The numbers don't lie. Something has to give."
scene-fiscal-steward = "Cut the clinic hours and see who you meet at the next election."
scene-tax-trader = "{ $memory } Will you sign the petition?"
scene-tax-trader-fresh = "Everyone on this street has signed. Will you?"
scene-pollution-voice = "{ $memory } The children here can taste the air."
scene-pollution-voice-fresh = "Smell that? We'd like someone to measure it."
scene-pollution-owner = "Inspect us into the ground and see where your voters work then."
scene-protest-organiser = "{ $memory } Have you seen what they've done to our street?"
scene-protest-organiser-fresh = "Nobody asked us. Six months of diggers and nobody asked."
scene-favour-creditor = "{ $memory } I'm calling it in."
scene-favour-creditor-fresh = "You said you'd owe me one. This is the one."
scene-party-sponsor = "{ $memory } The party needs you on this one."
scene-party-sponsor-fresh = "The whips are counting. Don't make them count you twice."
scene-donor-donor = "{ $memory } I'd like to help your campaign."
scene-donor-donor-fresh = "I've been watching your work. I'd like to help."
scene-rival-rival = "{ $memory } Let's keep this civil until the count."
scene-rival-rival-fresh = "We don't need to tear each other apart. Not yet."

## Storylines are arcs now (DEC-109): their lines are in arcs.ftl (`arc-rival-*`,
## `arc-favour-*`, `arc-plant-*`).

## Director events and developments (the story so far)
event-decision-opened = A decision awaits you: { $title }.
event-decision-answered = You decided on { $title }.
event-decision-defaulted = Time ran out on { $title }.
event-decision-lapsed = The moment passed on { $title }.
event-storyline-advanced = A storyline moved on.
event-development-crisis = { $town } faces a crisis: { $crisis }.
event-development-crisis-worse = The crisis in { $town } is getting worse: { $crisis }.
event-development-goal = { $name } ({ $role }): { $goal }.
event-development-rival = { $name } is your rival now.
event-chapter-began = A new chapter begins: { $kind }.
event-track-warning = Warning: { $track } is in danger.
event-director-hazard-held = The director held back a hazard in { $town }.
event-fact-discoverable = Something you did may come to light.
chapter-kind-session = a year in office
chapter-kind-campaign = the campaign

## Aftermath: what each effect did
effect-opinion = { $name }: opinion of you { $delta }
effect-approval = Approval of you ({ $who }): { $delta }
effect-approval-hood = Approval of you in { $hood }: { $delta }
effect-awareness = Name recognition ({ $who }): { $delta }
effect-money = Campaign funds: { $delta }
effect-health = Health: { $delta }
effect-slots = Takes { $n } of your slots this turn
effect-standing = Standing with your party: { $delta }
effect-legal = Legal jeopardy: { $delta }
effect-record = Record of work: { $delta }
effect-favour-owed = You owe { $name } a favour
effect-favour-owed-to-you = { $name } owes you a favour
effect-repaid = You repaid { $name }
effect-vote = Your vote on the { $bill } bill: { $vote }
effect-draft = A draft bill: { $bill }
effect-skipped = Not possible any more: { $what }
effect-learn = You learn about { $what }
effect-learned = You learned about { $what }
knows-town-concerns = what the town worries about
knows-hood-concerns = what the neighbourhood worries about
knows-broker = who really runs things
knows-person = someone in town
knows-rival-plans = the rival's plans
knows-policy-support = who backs a policy
knows-situational = a proposal on the table
knows-place = a place few people know
effect-who-ward = your ward
effect-who-town = the town
effect-who-hood = { $hood }
effect-who-class-0 = low-income voters
effect-who-class-1 = middle-income voters
effect-who-class-2 = better-off voters
effect-who-classes = { $who } ({ $classes })
vote-yes = yes
vote-no = no
vote-abstain = abstain

## Interface
ui-decisions = Decisions
ui-decisions-none = Nothing needs your answer right now.
ui-decision-count = { $n ->
    [one] 1 decision
   *[other] { $n } decisions
}
ui-decide-by = Decide by { $date }
ui-decide-days = { $n ->
    [0] decide today
    [1] 1 day left
   *[other] { $n } days left
}
ui-if-you-dont = If you don't answer: { $option }
ui-who-involved = Who is involved
ui-choose = Choose
ui-chosen = Decided
ui-aftermath = What happened
ui-aftermath-default = You did not answer in time.
ui-why-these = Why this came up
ui-scene-at = At { $place }
ui-scene-go = Meet them there
ui-scene-hint = Going there adds the conversation; you can decide from here.
ui-story-so-far = Story so far
ui-chapter = Chapter { $n }: { $kind }
ui-tension = Tension
ui-storylines = Storylines
ui-decision-kind-climax = Climax
ui-decision-kind-significant = Decision
ui-decision-kind-request = Request
ui-waiting-here = Waiting for you here
ui-costs = Costs
ui-gains = Gains
ui-not-enough-time = Needs more time than you have left this turn
ui-not-enough-money = Needs more money than you have
alert-decisions = Decisions waiting for you: { $n }.
alert-track-warning = { $track } is in danger. Act before it ends your career.
why-crisis = { $crisis } in { $town }
why-concern = Among the town's worries: { $concern }
why-election = Your seat is up soon
why-storyline = Part of a storyline: { $title }
why-memory = You and { $name } have history
why-goal = What { $name } is after: { $goal }
why-ward = It is happening in your ward
why-pressure = At stake: { $track }
vote-with-party = with your party
vote-with-ward = with your ward
event-director-intervention = The storyteller held chance back ({ $lever }) in { $town }.
favour-decision = A promise over { $subject }

## More dilemmas
dilemma-jobs-plea = the jobless figures
dilemma-jobs-plea-situation = Jobs come up in every conversation in { $town }. { $voice } ({ $voice_role }) asks what you will do for people out of work in { $hood }. { $employer } ({ $employer_role }) says a wage subsidy would let them hire.
dilemma-jobs-plea-subsidy = Draft a wage subsidy for small employers
dilemma-jobs-plea-subsidy-after = You drafted a wage subsidy. { $employer } is pleased; the party worries about the cost, and the jobless wonder when it will reach them.
dilemma-jobs-plea-fair = Organise a jobs fair yourself
dilemma-jobs-plea-fair-after = You spent a day and some campaign money on a jobs fair in { $hood }. A few people found work; many more saw you try.
dilemma-jobs-plea-wait = Say the market will recover
dilemma-jobs-plea-wait-after = You said the market would recover. { $voice } heard a shrug, and so did working families.
dilemma-attack-ad = the attack ad
dilemma-attack-ad-situation = Your party has an attack ad ready on { $rival }. It is sharp, it is mostly fair, and it is your call.
dilemma-attack-ad-run = Pay for it and run it
dilemma-attack-ad-run-after = The ad ran. Everyone knows your name now. { $rival } has stopped being polite about you, and better-off voters winced.
dilemma-attack-ad-clean = Keep the campaign clean
dilemma-attack-ad-clean-after = You pulled the ad. The party thinks you are going soft; { $rival } noticed the courtesy.
dilemma-attack-ad-none = Leave it unanswered
dilemma-attack-ad-none-after = You never answered. The ad sat in a drawer and the party decided you could not make up your mind.
dilemma-debate = the debate
dilemma-debate-situation = { $rival } challenges you to a public debate at the community centre, a week before the vote.
dilemma-debate-accept = Accept the challenge
dilemma-debate-accept-after = You debated { $rival } in front of a packed hall. The whole town heard you.
dilemma-debate-decline = Decline
dilemma-debate-decline-after = You declined. { $rival } debated an empty chair, and your ward noticed.
scene-jobs-voice = "{ $memory } People here need work, not speeches."
scene-jobs-voice-fresh = "Half this street is out of work. What are you going to do?"
scene-jobs-employer = "Help with the wage bill and I'll take on three more."
scene-attack-ad = The ad is on the desk at the party office, waiting for your signature.
scene-debate-rival = "{ $memory } Let the voters see us side by side."
scene-debate-rival-fresh = "One stage, two candidates. Unless you're afraid of the questions."

## Bills in progress and the favour ledger (M5d part 2)
effect-rider-ward-project = A project for your ward goes into the { $bill } bill
effect-rider-ward-project-for = A project for { $name }'s ward goes into the { $bill } bill
effect-rider-sunset = The { $bill } bill gets a four-year sunset
effect-call-in = { $name } repays the favour they owe you, on the { $bill } bill

dilemma-amendment-for-vote = a vote for a project
dilemma-amendment-for-vote-situation = Your bill, { $bill }, is one vote short, and { $swing } could be that vote. The price: a project in their ward, written into the bill.
dilemma-amendment-for-vote-project = Write in their ward project
dilemma-amendment-for-vote-project-after = The { $bill } bill now pays for a project in { $swing }'s ward. They are on board; other wards grumble at the cost, and so does your party.
dilemma-amendment-for-vote-sunset = Offer a four-year sunset clause instead
dilemma-amendment-for-vote-sunset-after = You put a four-year sunset on the { $bill } bill. { $swing } is warmer, and the law, if it passes, will need renewing.
dilemma-amendment-for-vote-merits = Let it pass on its merits
dilemma-amendment-for-vote-merits-after = You kept the { $bill } bill clean and spent the evening at home. { $swing } took it as a no.

dilemma-sponsor-rider = a project for your ward
dilemma-sponsor-rider-situation = { $sponsor } wants your yes on the { $bill } bill, and offers to write in a project for your ward.
dilemma-sponsor-rider-take = Take the project and vote yes
dilemma-sponsor-rider-take-after = Your ward gets its project in the { $bill } bill, and you voted yes. Other wards' councillors noticed what it cost.
dilemma-sponsor-rider-ward = Vote as your ward wants, no project
dilemma-sponsor-rider-ward-after = You voted on the { $bill } bill as your ward leans, and turned down the project. { $sponsor } expected better.
dilemma-sponsor-rider-decline = Decline and abstain
dilemma-sponsor-rider-decline-after = You abstained on the { $bill } bill. { $sponsor } read it as a snub.

dilemma-whip-deal = the party's deal
dilemma-whip-deal-situation = { $whip } has a deal: vote with the party on the { $bill } bill, against your ward's mood, and the party pays toward your campaign.
dilemma-whip-deal-take = Take the deal
dilemma-whip-deal-take-after = You voted with the party on the { $bill } bill. The money arrived; your ward noticed which way you went.
dilemma-whip-deal-haggle = Haggle: less money, but the whip owes you one
dilemma-whip-deal-haggle-after = A long evening at the party office: less money, but { $whip } now owes you one. You voted with the party on the { $bill } bill.
dilemma-whip-deal-refuse = Refuse and vote with your ward
dilemma-whip-deal-refuse-after = You voted with your ward on the { $bill } bill. { $whip } marked you down as against the party.
dilemma-whip-deal-counted = Say nothing; the whip counts you in
dilemma-whip-deal-counted-after = You never answered, so { $whip } counted you with the party on the { $bill } bill: no money, and your ward saw how you voted.

dilemma-call-in-favour = the favour owed
dilemma-call-in-favour-situation = { $debtor } owes you a favour, and the { $bill } bill is short of votes.
dilemma-call-in-favour-call = Call it in quietly
dilemma-call-in-favour-call-after = { $debtor } paid what they owed: their vote leans towards the { $bill } bill now. The debt is clear, and so is the slight chill.
dilemma-call-in-favour-public = Call it in where others can see
dilemma-call-in-favour-public-after = Everyone saw { $debtor } come round on the { $bill } bill. It looks like work; { $debtor } feels used.
dilemma-call-in-favour-save = Save it for something bigger
dilemma-call-in-favour-save-after = You kept the favour in your pocket. { $debtor } was relieved; the party wonders why the { $bill } bill is stuck.

dilemma-favour-for-vote = a favour for a vote
dilemma-favour-for-vote-situation = { $sponsor } needs votes for the { $bill } bill and offers you a favour in return.
dilemma-favour-for-vote-deal = Vote yes; they owe you one
dilemma-favour-for-vote-deal-after = You voted for the { $bill } bill, and { $sponsor } owes you a favour. Your ward will judge the vote on its own.
dilemma-favour-for-vote-ward = Vote as your ward wants
dilemma-favour-for-vote-ward-after = You voted on the { $bill } bill as your ward leans and took nothing for it. { $sponsor } wanted more.
dilemma-favour-for-vote-not-yet = Say you need to think
dilemma-favour-for-vote-not-yet-after = You put { $sponsor } off. They took the hint.

dilemma-creditor-against-rival = the debt and the bill
dilemma-creditor-against-rival-situation = { $creditor } is calling in the favour you owe: vote down { $rival }'s bill, { $bill }.
dilemma-creditor-against-rival-sink = Vote it down and clear the debt
dilemma-creditor-against-rival-sink-after = You voted against the { $bill } bill. The debt to { $creditor } is paid; { $rival } knows who sank it.
dilemma-creditor-against-rival-merits = Vote on the merits
dilemma-creditor-against-rival-merits-after = You voted on the { $bill } bill as your ward leans, debt or no debt. { $creditor } calls it a promise broken.
dilemma-creditor-against-rival-dodge = Abstain
dilemma-creditor-against-rival-dodge-after = You abstained on the { $bill } bill. Neither side thanks you.

scene-amendment-swing = "{ $memory } Put my ward in your bill and you have my vote."
scene-amendment-swing-fresh = "I could vote for your bill. My ward would need a reason to let me."
scene-rider-sponsor = "{ $memory } There's room in this bill for something in your ward."
scene-rider-sponsor-fresh = "Help me get this through, and your ward gets something out of it."
scene-whip-whip = "{ $memory } The party looks after those who look after it."
scene-whip-whip-fresh = "One vote, with the party. The campaign fund is generous to friends."
scene-call-in-debtor = "{ $memory } I know what I owe you. Is this when you want it?"
scene-call-in-debtor-fresh = "I haven't forgotten I owe you. Just tell me when."
scene-favour-vote-sponsor = "{ $memory } Vote with me on this, and I'll be there when you need me."
scene-favour-vote-sponsor-fresh = "I need one more yes. I don't forget people who help."
scene-creditor-rival-creditor = "{ $memory } You owe me. Sink that bill and we're square."
scene-creditor-rival-creditor-fresh = "You remember our arrangement. Here's how you settle it."
