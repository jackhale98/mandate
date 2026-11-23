## Conversations (content/base/conversations/). Every line has $speaker,
## $player, $town and $here (and $about on topics with a subject); each
## condition adds its own: $broker, $broker_place, $stake (the broker's
## stake, the town's industry), $rival, $person, $hood, $concern (and
## $concern_talk, a lower-case noun phrase for mid-sentence use), $policy,
## $support, $memory, $subject (and $subject_kind: bill, decision or none),
## $year, $n, $crisis. A place without a proper
## name comes as its kind's noun phrase (talk-place-*: "the union hall"), so
## $here, $about and $broker_place read mid-sentence. People are named, or
## "they" (never he or she) until characters have a pronoun field; say a
## name once or twice in a line, not three times.
##
## House style (docs/development/EXTENDING.md, "Writing for Mandate"):
## British English. Openers are stage directions or a first remark, and name
## the speaker; quotes, substance and spoken closers are in double quotes.
## Concern names are capitalised labels ("Housing costs"): mid-sentence, use
## $concern_talk ("the rent"). Policy names are titles ("Transit
## frequency"): put them first, after a colon, or as "the { $policy } bill",
## never bare as a sentence's subject ("Small business grants was…"). Every
## claim a line makes must be true whenever its conditions hold.

# Places without a proper name, as talk names them ($here, $about,
# $broker_place): lower-case noun phrases.
talk-place-town_hall = the town hall
talk-place-council_chamber = the council chamber
talk-place-community_centre = the community centre
talk-place-news_office = the newspaper office
talk-place-party_office = the party office
talk-place-diner = the diner
talk-place-market = the market
talk-place-union_hall = the union hall
talk-place-school = the school
talk-place-clinic = the clinic
talk-place-park = the park
talk-place-business_association = the chamber of commerce
talk-place-factory = the factory
talk-place-port = the port
talk-place-farm = the farm
talk-place-mine = the mine
talk-place-mill = the sawmill
talk-place-hotel = the hotel
talk-place-university = the university
talk-place-tech_campus = the tech campus
talk-place-regional_assembly = the regional assembly
talk-place-parliament = parliament
talk-place-executive_residence = the executive residence

# Topics (buttons).
talk-topic-worries = What are people worried about?
talk-topic-who-runs = Who really runs things here?
talk-topic-about = Ask about…
talk-topic-pitch = Pitch a policy
talk-topic-support = Ask for support
talk-topic-favour = Trade a favour
talk-topic-small-talk = Small talk

## Openers, by relationship (anyone).

# Cool, and hostile.
talk-open-cool = { $speaker } barely looks up.
talk-open-cool-2 = { $speaker } folds both arms and waits.
talk-open-cool-3 = { $speaker } lets you get halfway through the question before looking up.
talk-open-cool-4 = { $speaker } sees you coming and doesn't pretend otherwise. "Oh. It's you."
talk-open-hostile = { $speaker } offers neither a hand nor a seat.
talk-open-hostile-2 = { $speaker } looks at the time, pointedly. "You've got two minutes, and you're using them."
# Neutral.
talk-open-neutral = { $speaker } gives you a polite nod and waits for the question.
talk-open-neutral-2 = { $speaker } looks up from what they were doing. "Yes? What can I do for you?"
talk-open-neutral-3 = { $speaker } checks the time, then decides you can have a minute.
talk-open-neutral-4 = { $speaker } squints at you. "Right, then. What's this about?"
# Warm.
talk-open-warm = { $speaker } breaks into a grin. "There you are."
talk-open-warm-2 = "Just the person! I was hoping you'd come by," says { $speaker }.
talk-open-warm-3 = { $speaker } makes room for you without being asked.
# Friendly and trusting.
talk-open-trusted = { $speaker } draws you a little way off, out of earshot.
talk-open-trusted-2 = "I was going to ring you, actually," says { $speaker }, and means it.
talk-open-trusted-3 = { $speaker } leans in, voice low, as if picking up a conversation from yesterday.

## Openers, by voice and relationship.

# Blunt.
talk-open-blunt-cool = { $speaker } doesn't look up. "What now?"
talk-open-blunt-neutral = { $speaker } skips hello. "Well?"
talk-open-blunt-warm = "Right," says { $speaker }. "You've got my attention. Don't waste it."
talk-open-blunt-trusted = { $speaker } eyes you. "You'll want it straight. You usually do."
# Folksy.
talk-open-folksy-cool = { $speaker } manages a smile, just about. "Well, look who it is. I'll not pretend I'm thrilled."
talk-open-folksy-neutral = "Hello, hello," says { $speaker }. "You look like you've not sat down all day."
talk-open-folksy-warm = { $speaker } beams. "Come here, you. How are you keeping? Honestly, now."
talk-open-folksy-trusted = { $speaker } squeezes your arm. "I've been fretting about you, you know."
# Smooth.
talk-open-smooth-cool = { $speaker } smiles as though nothing had ever passed between you. "What a lovely surprise."
talk-open-smooth-neutral = "I've been hearing good things about you," says { $speaker }, and doesn't say what.
talk-open-smooth-warm = { $speaker } takes your hand and holds it a beat longer than necessary.
talk-open-smooth-trusted = "My favourite person in { $town }," says { $speaker }. "Don't tell the others."
# Guarded.
talk-open-guarded-cool = { $speaker } glances past your shoulder before answering at all.
talk-open-guarded-neutral = { $speaker } considers the question as if it might be a trap. "That depends what you're asking."
talk-open-guarded-here = { $speaker } glances round { $here } before answering, voice low.
talk-open-guarded-warm = { $speaker } waits until the nearest pair of ears has drifted off.
# Earnest.
talk-open-earnest-cool = { $speaker } meets your eye. "I'll be honest with you, because somebody ought to be."
talk-open-earnest-neutral = { $speaker } puts everything else down and gives you full attention.
talk-open-earnest-warm = "Good," says { $speaker }. "I wanted to talk to someone who might actually do something."
# Wry.
talk-open-wry-cool = { $speaker } looks you up and down. "Well. To what do I owe the displeasure?"
talk-open-wry-neutral = { $speaker } raises an eyebrow, which seems to be the whole greeting.
talk-open-wry-warm = "Well, well," says { $speaker }. "People will talk. Mostly about me, I hope."
# Anxious.
talk-open-anxious-cool = { $speaker } looks round as if hoping someone else will deal with you.
talk-open-anxious-neutral = { $speaker } starts. "Oh! Sorry, I was miles away. Is this – are you after me?"
talk-open-anxious-warm = "Oh, it's you. Good," says { $speaker }. "I mean, hello. Sorry. Hello."
# Pompous.
talk-open-pompous-cool = { $speaker } makes a show of finishing something important first.
talk-open-pompous-neutral = "Ah, it's you," says { $speaker }. "You'll have heard I'm rather in demand these days."
talk-open-pompous-warm = "Come, come," says { $speaker }, a little louder than needed. "People ought to see us talking."

## Openers, by role.
talk-open-press = { $speaker } flips a notebook to a clean page, then thinks better of it and closes it.
talk-open-politician = { $speaker } falls into step beside you. "Walk with me. I've a committee at the top of the hour."
talk-open-clergy = { $speaker } hears the question all the way through, which almost nobody does.

## Memory quotes ($subject: what it was about, $subject_kind: bill, a
## policy's name; decision, a decision's title; none, nothing in particular;
## $year). Every line has a variant without a subject.
talk-quote-broke-promise = { $subject_kind ->
    [bill] "You made me a promise once, over the { $subject } bill, back in { $year }. I remember."
    [decision] "You made me a promise once, over { $subject }, back in { $year }. I remember."
   *[other] "You made me a promise once, back in { $year }. I remember."
}
talk-quote-broke-promise-2 = { $subject_kind ->
    [bill] "{ $year }. The { $subject } bill. You looked me in the eye and said you'd see it through."
   *[other] "{ $year }. You looked me in the eye and said you'd see it through."
}
talk-quote-kept-promise = { $subject_kind ->
    [bill] "You kept your word on the { $subject } bill. That counts for something here."
    [decision] "You kept your word over { $subject }. That counts for something here."
   *[other] "You kept your word to me. That counts for something here."
}
talk-quote-kept-promise-2 = { $subject_kind ->
    [bill] "People told me you'd forget about the { $subject } bill once the votes were in. You didn't."
   *[other] "People told me you'd forget your promise once the votes were in. You didn't."
}
talk-quote-backed-my-bill = { $subject_kind ->
    [bill] "You stood up for my bill, { $subject }, when it wasn't fashionable. I haven't forgotten."
   *[other] "You stood up for my bill when it wasn't fashionable. I haven't forgotten."
}
talk-quote-opposed-my-bill = { $subject_kind ->
    [bill] "You voted against my bill, { $subject }. I'm still waiting to hear why."
   *[other] "You voted against my bill. I'm still waiting to hear why."
}
talk-quote-vetoed-my-bill = { $subject_kind ->
    [bill] "That was my bill you vetoed, in { $year }: { $subject }. All that work, and one stroke of a pen."
   *[other] "That was my bill you vetoed, in { $year }. All that work, and one stroke of a pen."
}
talk-quote-signed-my-bill = { $subject_kind ->
    [bill] "You signed my bill, { $subject }, into law. I've the photograph on my wall, if you must know."
   *[other] "You signed my bill into law. I've the photograph on my wall, if you must know."
}
talk-quote-promised-favour = { $subject_kind ->
    [bill] "You promised me a favour over the { $subject } bill. I'm not chasing. I'm just saying."
    [decision] "You promised me a favour over { $subject }. I'm not chasing. I'm just saying."
   *[other] "You promised me a favour. I'm not chasing. I'm just saying."
}
talk-quote-broke-ranks = { $subject_kind ->
    [bill] "You broke with the party over the { $subject } bill. Some of us had to explain that on the doorstep."
   *[other] "You broke with the party. Some of us had to explain that on the doorstep."
}
talk-quote-lobbied-me = { $subject_kind ->
    [bill] "You talked me round on the { $subject } bill. Don't make a habit of it."
    [decision] "You talked me round over { $subject }. Don't make a habit of it."
   *[other] "You talked me round once. Don't make a habit of it."
}
talk-quote-pressed-me = { $subject_kind ->
    [bill] "Last time you leaned on me, over the { $subject } bill, it didn't work. Just so you know."
    [decision] "Last time you leaned on me, over { $subject }, it didn't work. Just so you know."
   *[other] "Last time you leaned on me, it didn't work. Just so you know."
}
talk-quote-defeated-me = "You beat me in { $year }. Fair and square, I'm told."
talk-quote-lost-to-me = "I beat you in { $year }, if you remember. I do."
talk-quote-stood-with-me = { $subject_kind ->
    [decision] "When it came to { $subject }, you were on my side. I've told people."
    [bill] "When it came to the { $subject } bill, you were on my side. I've told people."
   *[other] "You were on my side in { $year }. I've told people."
}
talk-quote-stood-with-me-2 = { $subject_kind ->
    [decision] "I've not forgotten { $subject }. You turned up for me, and plenty didn't."
   *[other] "I've not forgotten { $year }. You turned up for me, and plenty didn't."
}
talk-quote-stood-against-me = { $subject_kind ->
    [decision] "You went the other way on { $subject }. I've not decided yet what that tells me."
    [bill] "You went the other way on the { $subject } bill. I've not decided yet what that tells me."
   *[other] "You went the other way, back in { $year }. I've not decided yet what that tells me."
}
talk-quote-stood-against-me-2 = { $subject_kind ->
    [decision] "Over { $subject }, you chose the other lot. I'm still a bit sore, if I'm honest."
   *[other] "You chose the other lot, back in { $year }. I'm still a bit sore, if I'm honest."
}
talk-quote-ignored-me = { $subject_kind ->
    [decision] "I waited on you over { $subject }. Weeks, I waited."
   *[other] "I waited on you, back in { $year }. Weeks, I waited."
}
talk-quote-blamed-me = "You put your own mess on my doorstep. I'll be living that down for years."
talk-quote-scandal = "I read about you in the paper. Everyone did."

## What people are worried about.

# Deflections (the safe answers), by role.
talk-worries-shrug = "Same as everyone, I suppose. Bills, mostly."
talk-worries-weather = "Couldn't tell you what the rest of { $town } thinks."
talk-worries-fb-politician = "If I knew what worried people, I'd have a bigger majority. Ask them yourself."
talk-worries-fb-party = "If the branch knew what people wanted, we'd have won more often."
talk-worries-fb-workers = "Ask the members. Oh, that's right – you'd have to come to a meeting."
talk-worries-fb-business = "My worries are my own, and the bank's."
talk-worries-fb-trade = "People tell me all sorts across the counter. It doesn't go any further."
talk-worries-fb-farm = "Rain, prices and the bank. Same as every year."
talk-worries-fb-press = "Buy the paper. It's all in there, and it costs less than my time."
talk-worries-fb-clergy = "What people tell me in confidence stays that way. I'm sure you understand."
talk-worries-fb-care = "I hear plenty. It isn't mine to pass on."
talk-worries-fb-folk = "Me? I keep myself to myself. The stallholders at the market hear more than I do."

# The town's worries: a rumour, plainly, and the speaker's own street.
talk-worries-rumour = "From what I hear, it's { $concern_talk }. People don't talk about much else."
talk-worries-rumour-2 = "In passing? People moan about { $concern_talk }, over and over."
talk-worries-rumour-3 = "Depends who you ask, but mostly { $concern_talk }. That's what's said in the queues, anyway."
talk-worries-town = "It's { $concern_talk }. Ask anyone in { $town } and you'll get that answer before you've finished the question."
talk-worries-town-2 = "If you want one thing, it's { $concern_talk }. Everything else is a long way behind."
talk-worries-town-3 = "Honestly? You hear about { $concern_talk } at every table and in every queue."
talk-worries-press = "Every other letter we get is about { $concern_talk }. Nobody writes in about the things that are going well."
talk-worries-street = "Round my way it's { $concern_talk }, every time. Ask anyone on my street."
talk-worries-town-talk = "Ask ten people in { $town } what's wrong and nine will say { $concern_talk }. The tenth hasn't been paying attention."
talk-worries-town-rent = "It's the rent. Everyone I know hands over more of their wages for it than they did a few years ago."
talk-worries-home = "In { $hood }? It's { $concern_talk }, plain and simple. Knock on any door on my road and you'll hear it."
talk-worries-home-2 = "I live in { $hood }, and I could name you six households on my street losing sleep over { $concern_talk }."
talk-worries-home-3 = "You want the truth about { $hood }? It's { $concern_talk }, at the kitchen table every night, and nobody's fixed it."
talk-worries-about-hood-heard = "{ $hood }? Not my end of town, but I hear it's mostly { $concern_talk } over there."
talk-worries-about-hood-home = "{ $hood }? That's my way. Mostly { $concern_talk }, from what the neighbours say."
talk-worries-about-hood-unknown = "{ $about }? Not my end of town, so I'd only be guessing. Ask someone who lives there."
talk-worries-about-hood-unknown-2 = "{ $about }? I couldn't tell you. I'm never over that way; you'd want someone from there."
talk-worries-about-hood-unknown-3 = "Don't ask me about { $about }. I only know my own street, and that's bad enough."
talk-worries-about-hood = "{ $hood }? I'd know; it's home. It's { $concern_talk }, and you could set your watch by the complaints."

# What the facts are doing to people (anyone).
talk-worries-jobs = "Another lot laid off. You stop asking where, after a while. You just ask who."
talk-worries-disaster = "People are still clearing up. You'd think it'd be the damage that gets to them, but it's the waiting."
talk-worries-broke = "They say the town's broke. Nobody can tell me where the money went, and that worries me more than the being broke."
talk-worries-air = "Some days you can taste the air. People have stopped opening their windows."

# What their work shows them (expertise, DEC-095).
talk-worries-nurse-budget = "We're short on nights, and the agency staff cost double. When the money's tight, it's the waiting room that pays."
talk-worries-nurse-air = "I've had more children in with bad chests this year than I can remember. You don't need a report to tell you why."
talk-worries-nurse-jobs = "You see it at the clinic before it's in any figures. People put off coming in, because a day off is a day's pay."
talk-worries-nurse-rent = "Half of what comes through our door isn't medical. It's damp flats, and worry, and parents skipping meals so the kids don't."
talk-worries-teacher-rent = "Families keep moving further out, where the rent's lower. I lose children from my register halfway through the year."
talk-worries-teacher-budget = "We're buying our own glue sticks. I'm not exaggerating; there's a receipt in my bag."
talk-worries-teacher-jobs = "The children know before the grown-ups say anything. You can tell whose parent's been laid off by who stops bringing a snack."
talk-worries-trade-jobs = "Takings are down. People still come in; they just put things back on the shelf."
talk-worries-trade-tax = "The rates are the first bill I open and the last I can pay. One more rise and I'm working for the council, not for myself."
talk-worries-trade-rent = "My Saturday help is moving away, because the family can't make the rent. That's another one I've trained up for somewhere else."
talk-worries-workers-jobs = "We've had lay-off notices on the board. Every shift, someone asks me if they're next, and I don't know what to tell them."
talk-worries-workers-closure = "When the plant shut, it wasn't just the jobs. It was the overtime, the suppliers, the café across the road. It runs down the street like a crack in a windscreen."
talk-worries-workers-rent = "The pay rise we fought a year for went straight to the landlords. My members have done the sums."
talk-worries-business-tax = "Every member I speak to says the same. The rates are the first line they look at, and the last one they can cut."
talk-worries-business-budget = "The town's books are a mess, and everyone knows it. Investors read the same papers we do."
talk-worries-manager-jobs = "I've signed more redundancy letters this year than I care to count. I read every one before it goes."
talk-worries-farm-air = "Something's not right with the water up top. The cattle won't drink from the far trough, and they've more sense than most."
talk-worries-farm-rent = "The young ones can't afford to live out here any more, so they don't. Who's going to farm it when I can't?"
talk-worries-farm = "What we get at the gate barely moves. What we pay for everything else does."
talk-worries-clergy-jobs = "The food bank in our hall used to open once a week. Now it's twice, and the queue starts before we've unlocked."
talk-worries-clergy-disaster = "We've had families sleeping on the hall floor since the disaster. Good people, who had homes until it happened."
talk-worries-clergy = "Loneliness, if you want the honest answer. It never makes the papers, but it fills my week."
talk-worries-press-budget = "Our postbag is all bins and potholes and library hours. When the money goes, it's the small things people notice first."
talk-worries-folk-tax = "The rates bill came. I sat with it at the kitchen table for an hour, doing sums I already knew the answer to."
talk-worries-folk-rent = "My grandson works full time and he's still in his old bedroom. Can't find a flat he can afford."
talk-worries-folk-rent-2 = "We're four in a one-bed flat. The landlord calls it cosy."
talk-worries-folk-jobs = "My neighbour got laid off. Twenty-odd years at the same place, and then a letter on a Friday."
talk-worries-folk-disaster = "Half our street's still waiting on builders. We've had a tarpaulin for a roof longer than I'd like to say."

## Who really runs things.

# Deflections, by role (some point somewhere).
talk-runs-deflect = "Runs things? Nobody runs anything round here. Ask at the diner if you want gossip."
talk-runs-nobody = "The council, supposedly. Isn't that you lot?"
talk-runs-easier = "Who runs anything? Ask me something easier."
talk-runs-fb-diner = "People talk in here all day. Most of it's rubbish and the rest is libel."
talk-runs-fb-politician = "Ask at the party office. They keep a list of the people they daren't upset."
talk-runs-fb-politician-2 = "Officially, we do. You'll find out what that's worth."
talk-runs-fb-party = "Hard work and good candidates. Next question."
talk-runs-fb-workers = "Not the people who do the work. I can tell you that much for nothing."
talk-runs-fb-business = "The market. Supply and demand. Anyone who tells you different is selling something."
talk-runs-fb-trade = "Not me. I can't even get anyone to fix the drain outside."
talk-runs-fb-farm = "Whoever's in the town, and the town never asks us."
talk-runs-fb-press = "If I knew for certain, I'd be printing it, not telling you for nothing."
talk-runs-fb-clergy = "I see people at their best and their worst, but rarely at their most powerful. The community centre sees more of that than I do."
talk-runs-fb-care = "Not us, that's for certain. Ask at the community centre; they know who gets their way round here."
talk-runs-fb-folk = "Couldn't tell you. The stallholders at the market know everyone's business, mind."

# A rumour, a name, the detail.
talk-runs-rumour = "People say nothing much happens without { $broker } hearing about it first. That's all I know."
talk-runs-rumour-2 = "There's a name you hear: { $broker }. Whether it's true, I couldn't say."
talk-runs-rumour-3 = "{ $broker }, if you listen to gossip. And I do, a bit."
talk-runs-named = "{ $broker }. Nothing gets decided in this town without someone checking with them first."
talk-runs-named-2 = "Officially? The council. In practice, { $broker }."
talk-runs-named-3 = "{ $broker }. Not the loudest in the room. Never needs to be."
talk-runs-named-where = "{ $broker }. Spend an afternoon at { $broker_place } and watch who gets offered a chair."
talk-runs-stake = "Follow the money in { $stake } and you'll find { $broker } at the end of it. Every time."
talk-runs-stake-2 = "Ask yourself why every decision about { $stake } seems to suit { $broker } so well. Then ask them, and watch the face."
talk-runs-stake-3 = "Everything round here comes back to { $stake }, and { $broker } has a hand in all of it. Cross them and you'll find doors shutting you didn't know were open."
talk-runs-workers = "{ $broker }. Every time we sit down with management, they've been in the room first. You can tell by how little's left to negotiate."
talk-runs-farm = "Out here you learn who to ask before you open a gate. In town, you ask { $broker }."
talk-runs-party = "{ $broker }. And yes, the branch has noticed, and no, there's nothing in the rulebook about it."

# Asked of the broker.
talk-runs-self = "You're looking at the answer."
talk-runs-self-2 = "Me, if I'm doing it properly. You'd never notice."
talk-runs-coy = "People do say my name. People say all sorts."
talk-runs-coy-2 = "Me? I just get asked a lot of questions. Like this one."
talk-runs-broker-cool = "Ask the voters. I'm told that's how it works."

## About a subject.

# Deflections, by role.
talk-about-shrug = "Can't say I know much about that."
talk-about-dunno = "You'd have to ask someone else. I wouldn't know where to start."
talk-about-fb-politician = "I'll not be quoted on that. Not by you."
talk-about-fb-party = "The branch has no view on that. Officially."
talk-about-fb-workers = "I've opinions on that. You haven't earned them yet."
talk-about-fb-business = "For that, you'd have to make an appointment."
talk-about-fb-trade = "Not my department. I just serve what's in front of me."
talk-about-fb-farm = "Can't help you there. I'm out on the land most days."
talk-about-fb-press = "Off the record, I know nothing. On the record, even less."
talk-about-fb-clergy = "That's not really mine to talk about."
talk-about-fb-care = "Sorry. I've a shift to get through, and that's not a quick answer."
talk-about-fb-folk = "I mind my own business, mostly."

# A person: a name, and what they are like.
talk-about-person-heard = "{ $person }? I know who you mean. I'll leave it at that."
talk-about-person = "{ $person }? Everyone round here knows them. Knowing what they'll do next is another matter."
talk-about-person-warm = "{ $person }? Kind as anything. Remembers your birthday, and your mother's."
talk-about-person-cold = "{ $person } is civil. Don't mistake it for friendly."
talk-about-person-bold = "{ $person } doesn't wait to be asked. Charges in, sorts it out, apologises later. Sometimes."
talk-about-person-timid = "{ $person } will agree with whoever spoke last. Make sure it's you."
talk-about-person-straight = "{ $person } is straight. You mightn't like the answer, but it'll be the real one."
talk-about-person-crooked = "Count your fingers after you shake hands with { $person }. I'm only half joking."
talk-about-person-loyal = "{ $person } sticks by people. Even the ones who don't deserve it."
talk-about-person-ambitious = "{ $person } wants the next job before they've done this one. Everybody can see it but them."
talk-about-person-grudge = "Don't cross { $person }. Not ever. They forget nothing."
talk-about-person-charmer = "{ $person } could talk the birds out of the trees, and has."
talk-about-person-vain = "{ $person } likes seeing their own name in the paper more than most people like their dinner."
talk-about-person-cautious = "{ $person } won't commit to the weather without checking first."
talk-about-person-reformer = "{ $person } wants to change everything. Some of it even needs changing."
talk-about-person-maverick = "{ $person } does whatever they like, and lets everyone else catch up."

# A neighbourhood or a place (the speaker's own end of town).
talk-about-hood-heard = "Round { $hood } it's mostly { $concern_talk }, I'm told. But don't quote me."
talk-about-hood = "{ $hood }'s my end of town. If you're asking what keeps people up at night, it's { $concern_talk }."
talk-about-hood-detailed = "I've lived in { $hood } long enough to tell you: { $concern_talk }, first and last. Anyone who says different hasn't knocked on a door there."

# A place: from inside (the speaker's own workplace), and what it is to
# everyone else ($about: the place, mid-sentence: "the newspaper office").
talk-about-place-own-press = "You mean { $about }? Two desks, one kettle and a deadline every night. I'm chasing something now that nobody wants me to finish."
talk-about-place-own-press-2 = "The editor at { $about } reads every line twice and cuts half. You'd be amazed who rings up asking us not to run something. Some of them sit on the council."
talk-about-place-own-editor = "I run { $about }, for my sins: four staff, falling sales and a printer that hates me. Everybody wants to be in the paper until they're in it."
talk-about-place-own-editor-2 = "What won't run at { $about }? Anything I can't stand up twice. What will? Whatever the advertisers like least, oddly enough."
talk-about-place-own-steward = "You mean { $about }? I've the keys, the kettle and the grievances. Thursday nights it's standing room only, and everyone wants to know what management's planning."
talk-about-place-own-steward-2 = "Half my week at { $about } is people's rotas and the other half is their overtime. The members trust it there. It's the one room management never comes into."
talk-about-place-own-organiser = "I practically live at { $about }. Toddlers in the morning, the food bank at noon, a residents' meeting at night, and a roof that leaks on all three."
talk-about-place-own-organiser-2 = "Everything that happens in this town happens at { $about } first, usually without a budget. Whoever turns up to help, that's who runs it."
talk-about-place-own-clergy = "I keep a room at { $about } on Sundays and an open door the rest of the week. People come in for the tea and stay for whatever's on their mind."
talk-about-place-own-diner = "You mean { $about }? That's mine. I open at six, and by seven I know who's on the early shift and who's courting. The coffee's terrible. Nobody minds."
talk-about-place-own-diner-2 = "Councillors on table four, the union on table six, and nobody sits at table five. I don't ask questions. I just keep { $about } running and the pot full."
talk-about-place-own-shop = "Twenty years on the same pitch at { $about }. You see a town change from behind a till: what people put back, and what they buy on tick."
talk-about-place-own-shop-2 = "The pitch fees at { $about } went up again, and the drains still flood on a wet Saturday. Saturdays are good, mind. The rest of the week I talk to the pigeons."
talk-about-place-own-manager = "I run { $about }. Two shifts, an order book I check every morning, and a head office that would close us on a spreadsheet if I let it."
talk-about-place-own-manager-2 = "People think { $about } is just noise and wages. It's three hundred families' wages and one bad quarter from trouble."
talk-about-place-own-business = "I chair the lot at { $about }. Breakfast meetings, sponsorship forms, and members who want the council off their backs and on their side at once."
talk-about-place-own-business-2 = "What goes on at { $about }? Mostly lunches. The real business is done in the car park afterwards."
talk-about-place-own-party = "Come up to { $about } some time: leaflets to the ceiling, a membership list I guard with my life, and a phone that only rings when someone's in trouble."
talk-about-place-own-party-2 = "Selection meetings at { $about } get hotter than any election. I've seen friendships end over the order of names on a leaflet."
talk-about-place-own-teacher = "At { $about } it's thirty to a class, a roof that leaks over the science room, and a head who fights for every penny. The children are brilliant."
talk-about-place-own-teacher-2 = "Parents' evening at { $about } tells you more about this town than any council report. You hear who's working nights and who's moving away."
talk-about-place-own-nurse = "At { $about } we see everyone sooner or later. The queue's out of the door by eight and the phones never stop. We manage. Just."
talk-about-place-own-nurse-2 = "Work at { $about } long enough and you can tell which streets people come from by what they come in with."
talk-about-place-own-farm = "Out at { $about } it's up before light and the weather decides the rest. Nobody from town comes out unless the road's flooded."
talk-about-place-own-shop-steward = "On the floor at { $about } I'm the one they come to when the rota's wrong or the line's too fast. Management sees me coming and finds a meeting to be in."
talk-about-place-own-harbourmaster = "Every boat in and out of { $about } goes through my book. Fewer than there were, and the dredging's three years overdue."
talk-about-place-own-farmhand = "Out at { $about } I do whatever wants doing: fences, feed, the lambing if it comes early. The pay's seasonal. The work isn't."
talk-about-place-own-head-teacher = "I run { $about }. Every spring I write the same letter about the roof, and every spring the answer is next year."
talk-about-place-own-gp = "At { $about } I've ten minutes a patient and twice as many patients as minutes. People wait three weeks to tell me what they could have told me on day one."
talk-about-place-own-dean = "At { $about } I spend half my time on the students and the other half on the budget. The budget wins most weeks."
talk-about-place-own-student = "{ $about }? Lectures in the morning, a shift behind a bar at night, and an essay somewhere in the middle. Most of us won't stay once we finish."
talk-about-place-own-hotel = "I manage { $about }. Weddings on Saturday, sales reps midweek, and once a year a conference that thinks this town is the seaside."
talk-about-place-own-park = "I keep { $about }: the beds, the paths and the benches. You can tell how the town's doing by who sits on them on a weekday afternoon."
talk-about-place-own-founder = "We started { $about } in a spare room. Now it's forty people, and every one of them was poached from somewhere else."
talk-about-place-own-engineer = "At { $about } I write code for people three time zones away. Good money, and most of my team have never set foot in the town hall."
talk-about-place-own-councillor = "In { $about }? Long evenings, longer papers, and a vote that's settled in the corridor before anyone stands up."
talk-about-place-own-mayor = "I've an office at { $about } with a view of the car park and a diary full until spring. The building leaks. So does everything said in it."
talk-about-place-own-regular = "I'm at { $about } most days. Same seat, same faces. You hear more there than you'd think."
talk-about-place-own-cool = "You mean { $about }? I'm there every day, and what goes on there is my business, not yours."
talk-about-place-town-hall = "You mean { $about }? Forms in, money out, and a queue either way. Everyone ends up there sooner or later."
talk-about-place-chamber = "Go and sit in the gallery at { $about } one evening. You'll learn more from who talks to whom in the break than from the votes."
talk-about-place-centre = "Most of what matters round here happens at { $about }: toddler groups, the food bank, residents' meetings. It runs on volunteers and biscuits."
talk-about-place-news = "The people at { $about } print what they can stand up, and what sells. Ring them with a story and you'll be in it, one way or another."
talk-about-place-party = "Everyone knows what goes on at { $about }: leaflets, phone banks and long arguments about whose name goes first."
talk-about-place-diner = "Everyone ends up at { $about } sooner or later. Sit near the counter and you'll hear half the town's business by lunchtime."
talk-about-place-market = "On a Saturday at { $about } you'll meet the whole town, and every one of them has an opinion on the price of something."
talk-about-place-union = "Thursday nights at { $about } it's standing room only. If the workers are angry about something, that's where you hear it first."
talk-about-place-school = "Ask any parent about { $about } and you'll get the same two answers: the teachers are good, and the building isn't."
talk-about-place-clinic = "Everyone's been to { $about }, and everyone's waited. The staff are run off their feet, and nobody blames them."
talk-about-place-park = "On a fine day half the town's in { $about }. On a wet one it's dog walkers and the odd councillor hoping to be seen."
talk-about-place-business = "Behind the door at { $about } it's breakfasts and sponsorship forms. The members want two things from the council: less of it, and more of it."
talk-about-place-works = "This town's wages come out of { $about }, one way or another. People grumble about it, and they'd grumble a lot more if it shut."
talk-about-place-port = "There's always something coming in or going out at { $about }. Ask the people who work it how long that'll last."
talk-about-place-farm = "Out at { $about } they're up before light, and nobody from town goes out there unless the road's flooded."
talk-about-place-any = "Plenty of people go to { $about }. Most of them could tell you more about it than I can; it's not somewhere I spend my days."

# The rival.
talk-about-rival = "{ $rival } is after the same thing you are, and makes no secret of it. Everybody in the chamber knows."
talk-about-rival-close = "Between us: { $rival } thinks you're beatable, and is planning on it. Prove that wrong and you'll sleep better."
talk-about-rival-name = "{ $rival }? I know the name. Ask me again when I know you better."
talk-about-rival-local = "{ $rival }? Wants to get on, that one. I'd keep an eye over your shoulder."
talk-about-rival-bold = "{ $rival }? Bold as brass. If they want something, the whole town hears about it."

# A bill or a policy.
talk-about-bill = "{ $policy }? From what I hear, { $support }."
talk-about-bill-2 = "{ $policy }? On the doorstep, { $support }. I'd not bet my seat on more than that."
talk-about-bill-heard = "{ $policy }? I've heard it both ways. I'll make my own mind up."

## Pitching a policy ($about: the policy).
talk-pitch-consider = "I'll think about it. No promises."
talk-pitch-listen = "Go on, then. I'm listening."
talk-pitch-fb-politician = "Send it over. I'll read it properly, which is more than most of them will."
talk-pitch-fb-party = "Has the group seen this? Don't bring me things the group hasn't seen."
talk-pitch-fb-workers = "Who pays for it, and who gets the work? Answer me that and I'll listen to the rest."
talk-pitch-fb-business = "{ $about }. What does it cost, who pays, and when? That's all my members will ask."
talk-pitch-fb-trade = "If it brings more people through the door, I'm for it. If it means more forms, I'm not."
talk-pitch-fb-farm = "Will it do anything for anyone past the last bus stop? Usually these things don't."
talk-pitch-fb-press = "Is this on the record? Because if it is, you'll want to slow down."
talk-pitch-fb-clergy = "I'll think on it. And I'll ask the people it's meant to help, if that's all right with you."
talk-pitch-fb-care = "If it puts one more pair of hands where I work, you've got me. If it's leaflets, you haven't."
talk-pitch-fb-folk = "{ $about }. Sounds expensive. Is it?"
talk-pitch-cool = "You've a nerve, coming to me with this."
talk-pitch-cool-2 = "Sell it to me, then. I could do with a laugh."
talk-pitch-trusted = "You know I'll hear you out. Just don't oversell it. You're better when you don't."
talk-pitch-support = "{ $policy }. The word on the street is that { $support }. Persuade me that's enough."
talk-pitch-owed = "You want my ear on top of what you already owe me? Bold."

## Asking for support.
talk-support-see = "Let me see what I can do."
talk-support-depends = "That depends on what's in it for us."
talk-support-fb-politician = "I'll look at the whip before I look at you. Nothing personal."
talk-support-fb-party = "Support from the branch comes through selection, not over a cup of tea."
talk-support-fb-workers = "The members decide who we back, not me. But I'll tell them you asked."
talk-support-fb-business = "We don't do endorsements. We do lunches. Come to one."
talk-support-fb-trade = "I've room in my window for a poster. Whoever fixes the parking gets it."
talk-support-fb-farm = "Out here we vote for whoever turns up. You've turned up. That's a start."
talk-support-fb-press = "The paper backs nobody until the last week. You know that."
talk-support-fb-clergy = "I won't tell anyone how to vote. I'll think kindly of you, which is free."
talk-support-fb-care = "I can't put my name to anything. Not in my job."
talk-support-fb-folk = "My vote? Maybe. My neighbours'? That's a bigger ask."
talk-support-cool = "Support you? After everything? You've got some neck."
talk-support-trusted = "You've earned a fair hearing from me. More than fair."
talk-support-owed = "Ask me again when you've paid back what you owe."
talk-support-owes = "I owe you one, I know. I'll think about it properly, I promise."
talk-support-kept = "You kept your word to me once. I'd like to keep mine."
talk-support-bill = "{ $policy }? The public's view: { $support }. I'll go where the voters go."

## Favours (politicians).
talk-favour-ledger = "Favours are how this place works. I'll remember."
talk-favour-ledger-2 = "I'll put it in the book. Don't look like that; everybody keeps a book."
talk-favour-noted = "Noted. Don't make me chase you for it."
talk-favour-owed = "You owe me { $n ->
    [one] a favour
   *[other] { $n } favours
} already."
talk-favour-owes = "I owe you, I know. Call it in when it counts, not before."
talk-favour-cool = "A favour? From me? I'd want it in writing first."
talk-favour-trusted = "That's what friends are for. I'll still write it down."

## Small talk.
talk-small-concern = "Don't get me started on { $concern_talk }. No, really, don't; I'll be here all day."
talk-small-weather = "Grim weather we're having."
talk-small-family = "The kids are well, thanks for asking."
talk-small-warm = "Always good to see you. Don't be a stranger."
talk-small-trusted = "You look tired. Are you sleeping? You're not, are you."
talk-small-cool = "I'm not here to chat, if it's all the same to you."
talk-small-cool-2 = "Weather's fine. Anything else?"
talk-small-bus = "Have you seen the state of the bus shelter? Someone's had another go at it."
talk-small-ducks = "Someone's been feeding the ducks bread again. There's a sign and everything."
talk-small-match = "Did you see the match on Saturday? No, nor did I. I heard it, though. The whole street did."
talk-small-disaster = "Everybody's got a story from when it hit. I've stopped telling mine."
talk-small-councillor = "Did you sit through the whole planning item last week? I think I lost a year of my life."
talk-small-mayor = "I've cut so many ribbons this year I've started carrying my own scissors."
talk-small-party = "There's a quiz night for branch funds. You're on my team; I need someone who knows the capitals."
talk-small-steward = "Darts team lost again. We've a new captain, and it isn't helping."
talk-small-organiser = "The youth club roof's leaking again. I've a bake sale to plan and a bucket to empty."
talk-small-business = "Golf on Sunday, if the rain holds off. It never does."
talk-small-manager = "Machines are like children. When they go quiet, that's when you worry."
talk-small-shopkeeper = "Had someone in this morning wanting to return half a melon. Half."
talk-small-diner = "Someone ordered the special and asked what was in it after they'd eaten it. Never ask after."
talk-small-farm = "Lost a ewe last week. You never get used to it, whatever anybody says."
talk-small-press = "Do you know how hard it is to fill a paper in a quiet week? You'd not believe what's made the front."
talk-small-clergy = "We've a jumble sale on Saturday. Bring something. Last time somebody brought a canoe."
talk-small-nurse = "Twelve-hour shift. I'm running on biscuits and spite."
talk-small-teacher = "Thirty reports to write. Every one of them 'a pleasure to teach'. Most of them are, honestly."
talk-small-resident = "Our dog's learned to open the back gate. We're very proud and very tired."
talk-small-retiree = "I've taken the allotment on again. The slugs are winning, but I'm not done."

## Closers, by voice.
talk-close-blunt-1 = "That's it. That's all there is."
talk-close-blunt-2 = "Don't make me say it twice."
talk-close-smooth-1 = "But you didn't hear that from me, of course."
talk-close-smooth-2 = "Let's keep this between friends."
talk-close-smooth-3 = { $speaker } smiles, as though you'd agreed to something.
talk-close-smooth-4 = "Remember who told you first. That's all I ask."
talk-close-smooth-5 = "Lovely to see you. Do come again when you've something to offer."
talk-close-pompous-1 = "I did say as much at the last public meeting, but nobody writes these things down."
talk-close-pompous-2 = "You may quote me. Most people do."
talk-close-pompous-3 = { $speaker } pauses, apparently for applause.
talk-close-pompous-4 = "I'm sure you'll find that useful. People generally do."
talk-close-guarded-1 = "That's as far as I'll go."
talk-close-guarded-2 = "Best not to repeat that."
talk-close-guarded-3 = "And I'd rather you didn't mention where you heard it."
talk-close-guarded-4 = { $speaker } looks round once more before letting you go.
talk-close-guarded-5 = "I'd rather leave it there, if it's all the same."
talk-close-earnest-1 = "Sorry. I get carried away. But I mean it."
talk-close-earnest-2 = "Write that down, would you? I'd like somebody to remember."
talk-close-earnest-3 = "I'm not saying it to be difficult. I'm saying it because it's true."
talk-close-earnest-4 = { $speaker } holds your eye a moment longer than is comfortable.
talk-close-wry-1 = "Anyway. Lovely weather for it."
talk-close-wry-2 = "It might never happen. It usually does, mind."
talk-close-wry-3 = "I'd say I was joking, but you'd know."
talk-close-wry-4 = { $speaker } seems to find the whole business faintly funny.
talk-close-anxious-1 = "Sorry. I don't know why I told you all that."
talk-close-anxious-2 = "You won't say it was me, will you?"
talk-close-anxious-3 = "I've probably got it wrong. I usually do."
talk-close-anxious-4 = { $speaker } fiddles with a coat button until you go.
talk-close-anxious-5 = "Sorry. Was that – sorry. I'll let you get on."
talk-close-folksy-1 = "Now go on, before you catch your death."
talk-close-folksy-2 = "Mind how you go, love."
talk-close-folksy-3 = "My nan used to say a watched pot never boils. Same goes for councils."
talk-close-folksy-4 = "And eat something. You look half-starved."
talk-close-plain-1 = "Anyway, that's how I see it."
talk-close-plain-2 = "For what it's worth."
talk-close-plain-3 = "Anyway. You did ask."

# Argument fallbacks and values.
talk-subject-that = that business
talk-stake-town = this town
talk-support-for = most people are for it
talk-support-against = most people are against it
talk-support-split = people are split

# Leads.
lead-ask-diner = The diner hears everything. Someone there might say who really runs this town.
lead-ask-market = The stallholders at the market know everyone's business. One of them might know who really runs this town.
lead-ask-community-centre = The community centre sees who turns up when something needs doing, and who pays for it.
lead-ask-party-office = The party office keeps a list of the people it daren't upset. Someone there might show you.

# Outcomes of a conversation ($name: who you talked to).
outcome-talk = You talked with { $name }.
outcome-talk-again = You asked { $name } again. Nothing new came of it.
outcome-talk-pitch-success = { $name } came round a little on the bill.
outcome-talk-pitch-fail = { $name } wasn't persuaded.
outcome-talk-pitch-noted = { $name } heard you out.
outcome-talk-support-backed = { $name } will back the bill.
outcome-talk-support-endorsed = { $name } will speak up for you in public.
outcome-talk-support-endorsed-favour = { $name } will speak up for you, and expects a favour in return.
outcome-talk-support-refused = { $name } turned you down.
outcome-talk-favour-offered = You promised { $name } a favour for their support.
outcome-talk-favour-called-in = { $name } repaid the favour on the bill.
outcome-talk-favour-none = { $name } has nothing to trade.
outcome-talk-small-talk = You passed the time with { $name }.

# The action log.
action-talk = Talk to someone
