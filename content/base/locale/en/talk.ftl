## Conversations (content/base/conversations/). Every line has $speaker,
## $player, $town and $here (and $about on topics with a subject); each
## condition adds its own: $broker, $broker_place, $stake (the broker's
## stake, the town's industry), $rival, $person, $hood, $concern (and
## $concern_talk, a lower-case noun phrase for mid-sentence use), $policy,
## $support, $memory, $subject (and $subject_kind: bill, decision or none),
## $year, $n, $crisis. Every line also has $office and $chamber (who signs
## and who votes where the conversation is: "the mayor", "the council"); a
## line for named roles may have the role's facts: $post ("the mayor", "a
## councillor") and $body for someone in a post, $work (where they work),
## $mystake (the industry their occupation speaks for). A place without a proper
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

# Posts and bodies, as talk names them mid-sentence (from the constitution's
# name keys: $office and an office's $post, talk-<office name key>; a seat's
# $post, talk-<body name key>-member; $body and $chamber, talk-<body name key>).
talk-body-council-member = a councillor
talk-office-mayor = the mayor
talk-body-regional-assembly-member = an assembly member
talk-office-governor = the governor
talk-body-house-member = a representative
talk-body-senate-member = a senator
talk-office-president = the president
talk-body-council = the council
talk-body-regional-assembly = the regional assembly
talk-body-house = the House
talk-body-senate = the Senate

# Topics (buttons).
talk-topic-open = What's on your mind?
talk-topic-worries = What are people worried about?
# The whole jurisdiction the conversation is in, as a subject (by tier).
talk-whole-local = The whole town
talk-whole-regional = The whole region
talk-whole-national = The whole country
ui-talk-kind-jurisdiction = All over
talk-topic-who-runs = Who really runs things here?
talk-topic-about = Ask about…
talk-topic-pitch = Pitch a policy
talk-topic-support = Ask for support
talk-topic-favour = Trade a favour
talk-topic-small-talk = Small talk
# The subject step: everything the person has no particular reason to know.
ui-dlg-something-else = Something else…
ui-dlg-something-else-hint = Anything else you know of. They may not know much about it.

## Openers, by relationship (anyone).

# Cool, and hostile.
talk-open-cool = { $speaker } barely looks up.
talk-open-cool-2 = { $speaker } folds both arms and waits.
talk-open-cool-3 = { $speaker } lets you stand there a while before looking up.
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
# A familiar face (DEC-114 §5): greeted by name.
talk-open-familiar-face = { $speaker } knows you on sight. "{ $player }, isn't it? Thought so."
talk-open-familiar-face-2 = { $speaker } waves you over. "{ $player }! I know your face by now."
talk-open-familiar-nod = { $speaker } nods as you come over, the way people do with someone they see about. "{ $player }."
talk-open-familiar-nod-2 = "Back again, { $player }," says { $speaker }, pleased to see you.
talk-open-familiar-ours = "There's our { $player }," says { $speaker }, and shifts along to make room.

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
talk-open-guarded-neutral = { $speaker } looks at you as if you might be a trap. "That depends what you're after."
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
talk-open-clergy = { $speaker } gives you their whole attention and doesn't hurry you, which almost nobody does.

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
# A neighbourhood the speaker's post covers (by report), and one they work in.
talk-worries-about-hood-office = "{ $hood }? I get the letters from there, and most of them are about { $concern_talk }. I know the complaint; I couldn't walk you to the door."
talk-worries-about-hood-office-2 = "What comes in from { $hood } is { $concern_talk }, mostly. It's in the post every week."
talk-worries-about-hood-office-3 = "From what reaches me, { $hood } is fretting about { $concern_talk }. You'd want someone on the doorstep for the rest."
talk-worries-about-hood-works = "I don't live in { $hood }, but I work there. What I hear all day is { $concern_talk }."
talk-worries-about-hood-works-2 = "{ $hood }? I'm there every working day. From what people say, it's { $concern_talk }."
# The whole jurisdiction ($about: its name): a rumour, plainly, from the desk.
talk-worries-whole-heard = "The whole of { $about }? From what I hear, it's { $concern_talk }. That's what comes up, whoever you ask."
talk-worries-whole-known = "Across { $about }, it's { $concern_talk }. Every part of it says so in its own words."
talk-worries-whole-known-2 = "If you want one answer for all of { $about }, it's { $concern_talk }."
talk-worries-whole-desk = "All of { $about } comes across my desk, and the biggest pile is about { $concern_talk }. It is the same from every part of it."

# Officeholders: what reaches them in the post ($post, $work).
talk-worries-office-heard = "As { $post } I hear it before most: { $concern_talk }. I'd rather not put it more strongly than that to you yet."
talk-worries-office = "What reaches me as { $post } is { $concern_talk }, more than anything else. I read all of it. I can't fix all of it."
talk-worries-office-desk = "Every complaint in { $town } ends up at { $work } sooner or later, and the tallest pile is { $concern_talk }. I could show you the folder."

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
talk-worries-teacher-care = "We've lost two teachers this year and nobody's replaced them. The classes just get bigger."
talk-worries-nurse-care = "The list for the clinic is longer every month. People wait so long they end up in casualty instead."
talk-worries-prices = "Everything costs more than it did a year ago, and the pay hasn't kept up. People are cutting back on things they used to take for granted."
talk-worries-trust = "After everything that's come out this year, people don't believe a word from the town hall."
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

## What's on your mind? (The open question: an ask of theirs, their main
## worry, or gossip; the engine chooses.)
talk-mind-nothing = "Nothing I'd trouble you with."
talk-mind-busy = "Getting through the day, mostly. Ask me another time."
talk-mind-ask = "Since you ask: I put something to you, and I'm waiting to see how it comes out. That's what's on my mind."
talk-mind-ask-2 = "You know what's on my mind. I asked you for something. I'd sooner see it done than talk about it."
talk-mind-worry-heard = "Honestly? It's { $concern_talk }, same as most people round { $hood }. I'll not bore you with the rest."
talk-mind-worry = "It's { $concern_talk }. I think about it more than I'd like, and so does everyone I know in { $hood }."
talk-mind-worry-detailed = "I'll tell you what keeps me awake: { $concern_talk }. In { $hood } I can count the households it's hit on both hands and start again."
talk-mind-gossip-heard = "I could tell you a thing or two about { $person }. Not today, though."
talk-mind-gossip = "Since you ask: { $person }. I've known them a while. I'd get to know them before you need them."
talk-mind-gossip-crooked = "Keep this to yourself: I don't trust { $person }. Nothing I could prove. I've just learned to count my change."
talk-mind-gossip-ambitious = "It's { $person } I keep thinking about. They want something bigger than what they've got, and they're not patient about it."
talk-mind-gossip-grudge = "A word about { $person }: they're still angry about something, and they don't let go. I'd find out what before you need them."
talk-mind-gossip-warm = "I've been meaning to say: { $person } is one of the good ones. If you need a kindness done quietly, start there."

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

# By role, from where they stand. Officeholders: their own office and the
# body first ($post, $chamber, $office), the name once they tell it plainly.
talk-runs-office = "On paper, { $chamber } votes and { $office } signs, and I'm { $post }, so part of that is me. Who gets a say before the vote is another question. I'd want to know you better before I answered it."
talk-runs-office-2 = "I'm { $post }. I can tell you how a thing is meant to be decided: { $chamber } votes on it and { $office } signs it. How it is decided, I keep for people I trust."
talk-runs-office-named = "I'm { $post }, so you get the official answer first: { $chamber } votes and { $office } signs. The other answer is { $broker }. Very little comes to a vote here without them hearing of it first."
talk-runs-office-named-2 = "From where I sit at { $work }, most votes are settled before anyone stands up to speak. { $broker } is the name you want. I don't hand it to everyone."
# The union, the party, business, the press, the farm ($work, $mystake).
talk-runs-workers-rumour = "There's a name that comes up whenever management goes quiet on us: { $broker }. I'd not swear to more than that yet."
talk-runs-workers-stake = "Anything that touches { $mystake } goes past { $broker } before it reaches us. My members' jobs get settled in rooms I'm not let into, and they're in every one."
talk-runs-party-rumour = "The branch has a name it rings before it decides anything: { $broker }. You didn't have that from a party officer."
talk-runs-business-rumour = "In business you learn whose call to return first. Round here that's { $broker }. I'll leave it there for now."
talk-runs-business = "{ $broker }. Nothing that matters gets signed at { $work } until I know where they stand on it. I'm not the only one who works that way."
talk-runs-business-stake = "It comes down to { $mystake }. { $broker } decides who gets a hearing on that and who waits. I've done both."
talk-runs-press-rumour = "There's a name we never quite get into print: { $broker }. Make of that what you like."
talk-runs-press = "{ $broker }. When a story dies at { $work }, it's usually after a call from that direction. I've taken some of those calls myself."
talk-runs-farm-rumour = "There's one name you hear at the gate when a decision goes against us: { $broker }. I've not met anyone who'd say it louder than that."
talk-runs-farm-stake = "Out here it's { $mystake }, and in town it's who lets you sell. That's { $broker }. I've had to go and ask, more than once."
# Far from power: the visible power, from where they stand (safe answers).
talk-runs-visible-organiser = "If you go by the letters I write, it's { $office } and { $chamber }. The people I work with never get past the front desk, so that's as far up as I can see."
talk-runs-visible-business = "From { $work }, it's { $office } and { $chamber }: they grant the licences and sign the contracts. If someone decides for them, I've not been introduced."
talk-runs-visible-press = "On the record, it's { $office } and { $chamber }: that's who we quote. Off it, I'm still asking the question you've just asked me."
talk-runs-visible-care = "From where I work, it's { $office } and { $chamber }: theirs are the decisions we live with at { $work }. If someone stands behind them, they've never needed anything from us."
talk-runs-visible-trade = "From behind a counter, it's { $office } and { $chamber }: the licences, the parking, the opening hours. Whether anyone tells them what to do, nobody's said so in here."
talk-runs-visible-farmhand = "Out here it's whoever sets the price at the gate. In town it's { $office } and { $chamber }, as far as anyone tells the hands."
talk-runs-visible-folk = "I'd have said { $office } and { $chamber }: theirs are the names on the letters. If it's someone else, nobody's told our street."

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
# Leads to who would know ($to_person, $to_hood).
lead-ask-person-broker = { $to_person } is in a position to know who really runs this town. Ask them.
lead-ask-person-hood = { $to_person } would know what worries people there. Ask them.
lead-ask-streets-hood = The people who live in { $to_hood } can tell you what worries them. Ask on its streets.
lead-ask-person-bill = { $to_person } would know where people stand on it. Ask them.
lead-ask-person-about = The surest way to learn about { $to_person } is to meet them.

# Outcomes of a conversation ($name: who you talked to).
outcome-talk = You talked with { $name }.
outcome-talk-again = You asked { $name } again. Nothing new came of it.
outcome-talk-miss = { $name } couldn't help. It cost you no time.
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

# What knowing the power broker opens (M6 note 27). Each label says the price.
broker-choice-lobby = Ask them to bring the members round on { $bill } ({ $members } members, about { $votes } votes; { $price ->
    [call_in] calls in the favour they owe you
   *[owe] you will owe them a favour
})
broker-say-lobby = "{ $bill } comes up soon. A word from you would carry the members. What would it take?"
broker-reply-lobby = "Leave it with me. You'll remember this, of course."
broker-reply-lobby-no = "I don't see why I'd spend my favours on you."
outcome-broker-lobby = { $broker } will lean on the members. You owe them for it.
outcome-broker-lobby-refused = { $broker } would not help you.
broker-choice-favour = Do them a favour (they will owe you one; people who resent them will notice)
broker-say-favour = "If there's something your people need from me, ask."
broker-reply-favour = "Good of you. I won't forget it."
outcome-broker-favour = You did { $broker } a favour. They owe you one.
broker-choice-oppose = Oppose them openly (popular with those who resent them, costly with those who depend on them)
broker-say-oppose = "You don't run this town. I'll say so in public."
broker-reply-oppose = "Say what you like. See who still takes your calls."
outcome-broker-oppose = You stood against { $broker } in the open. They will not forget it.
broker-choice-expose = Tell them what { $broker } is up to ({ $risk }% chance it backfires on you)
broker-say-expose = "Someone should write about how { $broker } really runs this town."
broker-reply-expose = "Give me what you have. If it stands up, we'll run it."
outcome-broker-expose = The story about { $broker } will run. The members are keeping their distance from them.
outcome-broker-expose-misfired = The story about { $broker } fell apart, and it made you look bad.

# Lobbying is a conversation (M6 note 29). Each argument on a bill is a
# choice: its label, the player's line, the member's answer (said before
# the roll; the outcome line says how it went), its costs and its outcome.
# $speaker is the member, $bill the bill ($bill_key), $side `for` or
# `against`, $broker the broker, $members and $votes those brought round.
ui-dlg-lobby-bill = Argue { $bill }
ui-dlg-lobby-hint = What you can argue depends on what you know of them. Each choice shows its chance and its cost.
ui-dlg-lobby-done = You have argued that bill with them as far as you can this week.
talk-why-day = That is a daytime talk. After hours, choose to meet after hours.
lobby-choice-plain = Make the case { $side ->
    [against] against it
   *[for] for it
}
lobby-choice-ward = Argue from what their ward thinks of it
lobby-choice-party = Point to their party's line
lobby-choice-word = Remind them you kept your word to them
lobby-choice-broker = Tell them { $broker } wants it too
lobby-choice-rally = Ask them to bring others round ({ $members } members, about { $votes } votes)
lobby-choice-call-in = Call in the favour they owe you
lobby-choice-owe = Offer them a favour in return
lobby-choice-after-hours = Meet after hours (better odds, a back-room deal)
lobby-say-plain = "{ $bill } comes to a vote soon. I'd like you { $side ->
    [against] to vote it down
   *[for] behind it
}."
lobby-say-ward = "Think how { $bill } lands in your ward. Your voters will remember how you voted."
lobby-say-party = "Your party has a line on { $bill }. I'm asking you to keep to it."
lobby-say-word = "I did what you asked of me. Trust me on { $bill }."
lobby-say-broker = "{ $broker } wants the same as I do on { $bill }. You know what that means."
lobby-say-rally = "People listen to you. Would you bring a few of them round on { $bill }?"
lobby-say-call-in = "You owe me one. I'm calling it in on { $bill }."
lobby-say-owe = "Vote with me on { $bill } and I'll owe you one."
lobby-say-after-hours = "Not here. Let's talk properly tonight, somewhere quiet."
lobby-reply-plain = "I've heard a lot of pitches on this one. I'll think about it."
lobby-reply-ward = "My ward talks of little else. Let me weigh it."
lobby-reply-party = "The party has its view. So do I."
lobby-reply-word = "You did keep your word. I'll give you that."
lobby-reply-broker = "Does { $broker } now. That might change things."
lobby-reply-rally = "I can make a few calls. No promises."
lobby-reply-call-in = "Fair enough. I owe you, and I pay what I owe."
lobby-reply-owe = "A favour from you. I'll hold you to it."
lobby-reply-after-hours = "All right. Not here, though."
lobby-cost-time = half a slot
lobby-cost-late = your late slot, −{ $health } health
lobby-cost-legal = legal risk +{ $legal }
lobby-cost-owe = you will owe { $name } a favour
lobby-cost-call-in = spends the favour { $name } owes you
lobby-cost-fail = if it fails, { $name } thinks less of you
outcome-lobby-won = { $name } came round on { $bill }.
outcome-lobby-lost = { $name } was not persuaded on { $bill }.
outcome-lobby-called-in = { $name } paid back the favour on { $bill }.
outcome-lobby-owe = { $name } will move on { $bill }. You owe them a favour.
outcome-lobby-rallied = { $name } came round on { $bill } and will work on { $members } others. You owe them a favour.
