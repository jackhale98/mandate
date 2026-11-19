## The main screen (DEC-088): the walkable street, the town map overlay
## (DEC-086), the bar, the pocket's tabs and the night between turns.
## UI strings only; the engine never asks for these.

# The town map overlay.
ui-town-map = Town map
ui-town-map-open = Map
ui-town-map-close = Close the map
ui-town-map-pick = tap a place to walk there
ui-town-map-label = Map of { $town }: { $n } places to go. Tap one to walk there; the pocket's Places tab has the same places as text.
ui-town-map-help = Drag to pan, pinch or scroll to zoom. Arrow keys move between places, Enter goes there.
ui-town-map-fit = Show the whole town
ui-town-map-zoom-in = Zoom in
ui-town-map-zoom-out = Zoom out
ui-town-map-waiting = { $what } waiting
ui-town-map-vote = Vote to cast
ui-town-map-quiet = { $place }: nothing to do here yet.
ui-town-planning = Laying out the town…
# The map overlay's two maps.
ui-map-scope = Which map
ui-map-scope-town = Town
ui-map-scope-country = Country
ui-map-country-title = The country
ui-map-country-pick = tap a town to see it

# The street.
ui-town-street = Street
ui-town-street-label = The streets of { $town }, drawn. Tap a building to walk to it, or walk with the arrow keys or W A S D (Shift to hurry) and press Enter at a door to go in. The pocket's Places tab has every place as text.
ui-town-street-help = Tap a building to walk there. Drag to look around, pinch or scroll to zoom. With a keyboard: arrows or W A S D walk, Enter goes in.
ui-town-street-loading = Drawing the streets…
# The hint at a door the player stands at (keyboard walking).
ui-town-street-enter = Enter: { $place }
ui-town-where-zone = You are in { $zone }

# The night between turns.
ui-passage-day = A day later
ui-passage-week = A week later
ui-passage-weeks = { $n } weeks later
ui-passage-days = { $n } days later
ui-passage-working = The days pass…
ui-passage-skip = Tap to carry on

# The bar and the pocket.
ui-skip-places = Skip to Places
ui-game-bar = Game
ui-agenda-choose = Choose an action.
ui-pocket-tabs = Pocket sections
ui-pocket-tab-today = Today
ui-pocket-tab-town = Town
ui-pocket-tab-places = Places
ui-pocket-tab-people = People
ui-pocket-dock = Keep the pocket open beside the town
ui-pocket-dock-hint = Dock the pocket beside the town
ui-pocket-undock-hint = Let the pocket close when you go somewhere
ui-notebook-new-count = { $n } new in your notebook
ui-places-help = Every place in town. Choose one to go straight there and open it.
ui-town-hoods = Neighbourhoods
ui-town-hood-worry = Worried most about { $concern }
ui-people-town = Who matters in town
ui-people-help = Tap a name to see how they stand with you.
ui-people-learned = You learned { $n ->
    [one] one new thing
   *[other] { $n } new things
} this turn.

## Talk panel (DEC-093): talking to someone by topic, and the worry raised
## on a canvass (DEC-091). `talk-why-*` are the engine's reasons a topic
## can't be asked now (`TopicView.unavailable_key`).

talk-why-time = You're out of time this turn.
talk-why-late-used = You've already used the late slot this turn.
talk-why-late-here = Late at night, people only talk where private meetings are held.
talk-why-needs-subject = Choose what to ask about first.
talk-why-role = They're not the person to ask about that.
talk-why-place = That isn't talked about here.
talk-why-not-now = That isn't something to ask them now.
talk-why-owe = You already owe them too much.
talk-why-away = They aren't here. Go to them first.
talk-why-about = You know of nothing like that in town.
talk-why-other = You can't ask that right now.
ui-talk = Talk
ui-talk-to = Talk to { $name }
ui-talk-with = Talking with { $name }
ui-talk-with-stranger = Talking with someone you haven't met
ui-talk-topics = What to ask
ui-talk-ask = Ask
ui-talk-about = About
ui-talk-about-none = Nothing in particular
ui-talk-odds = { $pct }% chance
ui-talk-asked = asked this week
ui-talk-cost-half = A conversation takes half a slot. { $left ->
    [one] One slot left this turn.
   *[other] { $left } slots left this turn.
}
ui-talk-cost-slot = A conversation takes a slot. { $left ->
    [one] One slot left this turn.
   *[other] { $left } slots left this turn.
}
ui-talk-cost-half-left = This one uses the half slot left from your last conversation.
ui-talk-cost-late = Late at night it uses the late slot, and costs health.
ui-talk-out-of-time = You're out of time this turn. End the turn to talk again.
ui-talk-late = Late at night (the late slot)
ui-talk-away = They're at { $place }.
ui-talk-go = Go to { $place }
ui-talk-nowhere = You can't find them anywhere you can go right now.
ui-talk-opinion-hostile = Hostile towards you
ui-talk-opinion-cool = Cool towards you
ui-talk-opinion-neutral = Neutral towards you
ui-talk-opinion-warm = Warm towards you
ui-talk-opinion-friendly = Friendly towards you
ui-talk-said = What was said
ui-talk-you-asked = You: { $topic }
ui-talk-you-asked-about = You: { $topic } ({ $about })
ui-talk-learned = What you learned
ui-talk-learned-none = Nothing new this time.
ui-talk-in-notebook = Added to your notebook.
ui-talk-kind-place = Places
ui-talk-kind-person = People
ui-talk-kind-hood = Neighbourhoods
ui-talk-kind-bill = Bills
ui-talk-kind-template = Policies
ui-talk-kind-rival = Your rival
ui-talk-kind-proposal = Proposals
ui-talk-leads = Leads
ui-talk-lead-go = Go there
ui-talk-warmer = { $name } seems warmer towards you.
ui-talk-cooler = { $name } seems cooler towards you.
ui-talk-again = Ask something else
ui-talk-street-hint = Enter: talk to { $name }
ui-talk-street-someone = Someone
ui-talk-street-near = { $name } is here. Press Enter to talk.
ui-canvass-worry = Talk about
ui-canvass-worry-none = No particular issue
ui-canvass-worry-top = their top worry
ui-canvass-worry-town = a town worry
ui-canvass-fit-strong = a strong fit
ui-canvass-fit-fair = a fair fit
ui-canvass-fit-weak = a weak fit
ui-canvass-effect = { $issue } here: { $fit }, { $times } the effect.
ui-canvass-effect-band = { $issue } here: { $fit }.
ui-canvass-effect-unknown = How well { $issue } goes down here isn't known yet.
ui-canvass-general = A general canvass, on no worry in particular.
ui-canvass-no-worries = You don't know what worries people here yet.
ui-canvass-landed-strong = You raised { $issue }: it struck a chord.
ui-canvass-landed-fair = You raised { $issue }: it went down fairly well.
ui-canvass-landed-weak = You raised { $issue }: it fell flat here.
