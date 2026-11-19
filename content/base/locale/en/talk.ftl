## Conversations (content/base/conversations/). Every line has $speaker,
## $player, $town and $here (and $about on topics with a subject); each
## condition adds its own: $broker, $broker_place, $stake (the broker's
## stake), $rival, $person, $hood, $concern, $policy, $support, $memory,
## $subject, $year, $n, $crisis. Lines use names, not pronouns, until
## characters have a pronoun field.

# Topics (buttons).
talk-topic-worries = What are people worried about?
talk-topic-who-runs = Who really runs things here?
talk-topic-about = Ask about…
talk-topic-pitch = Pitch a policy
talk-topic-support = Ask for support
talk-topic-favour = Trade a favour
talk-topic-small-talk = Small talk

# Openers, by relationship.
talk-open-cool = { $speaker } keeps working while you talk.
talk-open-cool-2 = { $speaker } folds both arms and waits.
talk-open-neutral = { $speaker } puts the kettle on.
talk-open-neutral-2 = { $speaker } nods you over to a chair.
talk-open-warm = { $speaker } waves you into the back office and shuts the door.
talk-open-warm-2 = { $speaker } breaks into a grin. "There you are."

# Memory quotes.
talk-quote-broke-promise = "You made me a promise once, over { $subject }, back in { $year }. I remember."
talk-quote-kept-promise = "You kept your word on { $subject }. That counts for something here."

# What people are worried about.
talk-worries-shrug = "Same as everyone, I suppose. Bills, mostly."
talk-worries-weather = "Couldn't tell you what the rest of { $town } thinks."
talk-worries-rumour = "People keep going on about { $concern }, from what I hear."
talk-worries-town = "Ask anyone in { $town }: it's { $concern }. That's what keeps people up at night."
talk-worries-home = "Round { $hood } it's { $concern }, plain and simple. Ask any of the neighbours."

# Who really runs things.
talk-runs-deflect = "Runs things? Nobody runs anything round here. Ask at the diner if you want gossip."
talk-runs-nobody = "The council, supposedly. Isn't that you lot?"
talk-runs-rumour = "People say { $broker } has the mayor's ear. I couldn't tell you more than that."
talk-runs-named = "{ $broker }. Nothing gets decided in this town without someone checking with { $broker } first."
talk-runs-stake = "{ $broker }, at { $broker_place }. And { $broker } has money riding on { $stake }, so watch what happens there."
talk-runs-self = "You're sitting across from the answer."

# About a subject.
talk-about-shrug = "Can't say I know much about { $about }."
talk-about-dunno = "{ $about }? You'd have to ask someone else."
talk-about-person = "{ $person }? Everyone round here knows { $person }."
talk-about-hood = "{ $hood }? It's { $concern } there, mostly."
talk-about-rival = "{ $rival } is after your job, and isn't shy about it."
talk-about-bill = "{ $policy }? From what I hear, { $support }."

# Pitching, support, favours.
talk-pitch-consider = "I'll think about it. No promises."
talk-pitch-listen = "Go on, then. I'm listening."
talk-support-see = "Let me see what I can do."
talk-support-depends = "That depends on what's in it for us."
talk-favour-ledger = "Favours are how this place works. I'll remember."
talk-favour-noted = "Noted. Don't make me chase you for it."
talk-favour-owed = "You owe me { $n ->
    [one] a favour
   *[other] { $n } favours
} already."

# Small talk.
talk-small-weather = "Grim weather we're having."
talk-small-family = "The kids are well, thanks for asking."
talk-small-warm = "Always good to see you, { $player }."

# Closers, by voice.
talk-close-blunt-1 = That's the long and short of it.
talk-close-blunt-2 = Don't make me say it twice.
talk-close-smooth-1 = But you didn't hear that from me, of course.
talk-close-smooth-2 = Let's keep this between friends.
talk-close-guarded-1 = I've said more than I should.
talk-close-guarded-2 = Best not to repeat that.
talk-close-plain-1 = Anyway, that's how I see it.
talk-close-plain-2 = For what it's worth.

# Argument fallbacks and values.
talk-subject-that = that business
talk-stake-town = this town
talk-support-for = most people are for it
talk-support-against = most people are against it
talk-support-split = people are split

# Leads.
lead-ask-diner = The diner hears everything. Someone there might say who really runs this town.

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
action-talk = Talk
