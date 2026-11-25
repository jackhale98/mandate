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
## Which build this is (title screen, "Report a problem").
ui-build-id = Build { $id }
ui-build-id-hint = Which build of the game this is: say it when you report a problem.
## The bottom bar's date on a narrow phone: the day and a short month.
ui-date-short = { $day } { $month }
month-short-1 = Jan
month-short-2 = Feb
month-short-3 = Mar
month-short-4 = Apr
month-short-5 = May
month-short-6 = Jun
month-short-7 = Jul
month-short-8 = Aug
month-short-9 = Sep
month-short-10 = Oct
month-short-11 = Nov
month-short-12 = Dec

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
ui-pocket-dock = Keep the pocket open beside the town
ui-pocket-dock-hint = Dock the pocket beside the town
ui-pocket-undock-hint = Let the pocket close when you go somewhere
ui-notebook-new-count = { $n } new in your notebook
ui-places-help = Every place in town. Choose one to go straight there and open it.
ui-town-hoods = Neighbourhoods
ui-town-hood-worry = Worried most about { $concern }
ui-people-help = Tap a name to see how they stand with you.

## Notebook (DEC-091, M6 K5): what the player knows about the town, by
## tab, with where and when each thing was learned, placeholders for what
## is not known yet and the leads that would reveal it. The lead-notebook-*
## keys are the engine's standing leads (where such things are usually
## learned); every other key here is asked for by the UI.
ui-pocket-tab-notebook = Notebook
ui-notebook-title = What you know about { $town }
ui-notebook-tabs = Notebook sections
ui-notebook-tab-people = People
ui-notebook-tab-issues = Worries
ui-notebook-tab-places = Places
ui-notebook-tab-policies = Policies
ui-notebook-tab-leads = Leads
ui-notebook-filter = Search your notebook
ui-notebook-filter-placeholder = Names, places, worries…
ui-notebook-filter-none = Nothing here matches “{ $q }”.
ui-notebook-filter-elsewhere = Matches in other sections: { $tabs }
ui-notebook-new = New
ui-notebook-new-note = { $n ->
    [one] One new entry since you last looked.
   *[other] { $n } new entries since you last looked.
}
ui-notebook-unknown = Not known yet
ui-notebook-from = { $source }, { $date }
ui-notebook-history = How you learned this
ui-notebook-lead = To find out
ui-notebook-lead-gives = { $level ->
    [heard] You would hear of this
    [known] Would raise this to { $dots } { $name }
    [detailed] Would raise this to { $dots } { $name }
   *[other] Would tell you more
}
ui-notebook-go = Go there
ui-notebook-here = You are here
ui-notebook-person-open = What you know of { $name }
ui-notebook-cares = Cares about { $issue }
ui-notebook-owes = Owes favours to { $name }
ui-notebook-rumoured = (rumoured)
ui-notebook-stake = Their stake: { $stake }
ui-notebook-top-worry = Top worry: { $a }
ui-notebook-worries-2 = Worries: { $a }, { $b }
ui-notebook-worries-3 = Worries: { $a }, { $b }, { $c }
ui-notebook-support = { $band }: net support { $pct } points
ui-notebook-support-band = { $band }
ui-notebook-rival = The rival: { $name }
ui-notebook-rival-something = Planning something
ui-notebook-rival-seat = Means to contest { $seat }
ui-notebook-rival-focus = Campaigning on { $issue }
ui-notebook-place-kind = { $kind }
ui-notebook-proposal-reason = { $reason }
ui-notebook-town-worries = The whole town
ui-notebook-hood-worries = By neighbourhood
ui-notebook-empty-people = You haven't heard of anyone yet. People's names come with meeting them, the paper and the talk in town.
ui-notebook-empty-issues = You don't know what worries people yet. Canvass a neighbourhood or listen at the diner.
ui-notebook-empty-places = Nothing hidden found yet. Every ordinary place is in the Places tab.
ui-notebook-empty-policies = You don't know who supports what yet. The party office's polls would tell you.
ui-notebook-empty-leads = No leads yet. Talk to people and they will point you somewhere.
ui-notebook-leads-help = Where to go or whom to ask to learn more. Leads are guidance, never a must.
ui-notebook-leads-open = Leads you picked up
ui-notebook-leads-standing = Ways to find out what you don't know
ui-notebook-today = { $n ->
    [one] You learned something new.
   *[other] You learned { $n } new things.
}
ui-notebook-today-open = Open the notebook
ui-local-knowledge = Start with local knowledge
ui-local-knowledge-help = You grew up here: start knowing the town's people and worries, and who really runs things.
lead-notebook-canvass = Canvass { $to_hood } to hear what worries people there.
lead-notebook-worries = Ask around at the { $to_kind }: people there talk about what worries the town.
lead-notebook-rival = The party's internal polls would show what { $rival } is planning.
lead-notebook-polls = The party's internal polls would show who supports { $policy }.

# The country map's layers (DEC-089): names, units and notes from the
# registry (`map_layers.jsonc`), then the picker, legend, readout and table.
map-layer-population = Population
map-layer-population-unit = Residents
map-layer-density = Density
map-layer-density-unit = Residents per km²
map-layer-unemployment = Unemployment
map-layer-unemployment-unit = Share of workers without a job
map-layer-unemployment-note = Grey: close to the national rate of { $mid }. Blue is lower, red is higher.
map-layer-median-income = Median income
map-layer-median-income-unit = A year, per working-age adult
map-layer-rent = Rent
map-layer-rent-unit = A month, for a typical home
map-layer-industry = Main industry
map-layer-industry-unit = What each town's jobs are known for
map-layer-industry-note = The industry largest for its size compared with the country, public services aside.
map-layer-industry-detail = { $pct }% of local jobs
map-layer-resource = Resources
map-layer-resource-unit = Each town's strongest natural resource
map-layer-resource-note = The resource each town is richest in compared with the rest of the country: farmland, timber, minerals, fishing, energy or scenery.
map-layer-resource-detail = index { $pct } of 100
ui-map-legend = Legend
ui-map-legend-under = under { $hi }
ui-map-legend-range = { $lo }–{ $hi }
ui-map-legend-over = { $lo } and over
ui-map-legend-other = Other
ui-map-legend-unknown = Unknown
ui-map-legend-towns = { $n ->
    [one] 1 town
   *[other] { $n } towns
}
ui-map-per-km2 = { $n }/km²
ui-map-readout-hint = Tap a town to read its figure.
ui-map-as-of = As of { $date }
ui-map-list = List
ui-map-list-show = Show the towns as a list
ui-map-list-hide = Back to the map
ui-map-table-caption = Towns by { $layer }
ui-map-table-town = Town
ui-map-table-sort = Sort by { $col }
ui-map-table-open = Open { $town }
ui-map-class = Class { $n } of { $total }
ui-map-details = Details
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
ui-talk-kind-place = Places
ui-talk-kind-person = People
ui-talk-kind-hood = Neighbourhoods
ui-talk-kind-bill = Bills
ui-talk-kind-template = Policies
ui-talk-kind-rival = Your rival
ui-talk-kind-proposal = Proposals
# A neighbourhood with its town's name, named as the town centre (M6 note 19).
ui-hood-centre = { $centre_town } town centre
ui-talk-warmer = { $name } seems warmer towards you.
ui-talk-cooler = { $name } seems cooler towards you.
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

## Dialogue mode (DEC-093): a conversation as a dialogue box at the bottom
## of the screen with speech bubbles over the speakers (in the street) or
## beside a portrait (anywhere else). The topics and the replies are the
## engine's; these are the box's own words.

ui-dlg-stranger = Someone you haven't met
ui-dlg-cost-half = ½ slot · { $left } left
ui-dlg-cost-slot = 1 slot · { $left } left
ui-dlg-cost-late = The late slot
ui-dlg-out-of-time = Out of time
ui-dlg-odds = { $pct }%
ui-dlg-choices = What to say
ui-dlg-continue = Continue ({ $n } of { $total })
ui-dlg-goodbye = Goodbye
ui-dlg-back = Back
ui-dlg-search = Search
ui-dlg-search-none = Nothing matches “{ $q }”.
ui-dlg-asked-about = { $topic }: { $about }
ui-dlg-says = { $name }: { $line }
ui-dlg-noted = Noted: { $what }
ui-dlg-noted-more = Noted: { $what }, and { $more } more
ui-dlg-noted-open = Open your notebook
## Report a problem
ui-report = Report a problem
ui-report-intro = Something wrong? Save a report file and send it to the developers. With it they can replay your game exactly as it happened.
ui-report-contains = The file holds:
ui-report-item-run = this game's seed and settings, and every move you made in it
ui-report-item-save = the game as it is now (its save)
ui-report-item-errors = the last few error messages, if there were any
ui-report-item-device = your screen size and your browser's name
ui-report-item-note = what you write below
ui-report-privacy = Nothing personal beyond that. Nothing is sent anywhere: the file is saved on this device (or offered to your phone's share sheet), and you choose whom to send it to.
ui-report-describe = What happened? (optional)
ui-report-describe-hint = What you did, what you expected, and what you saw instead.
ui-report-save = Save report file
ui-report-working = Preparing the report…
ui-report-done = Saved { $file } ({ $size }).
ui-report-shared = Shared { $file } ({ $size }).
ui-report-failed = The report could not be made.
ui-report-no-game = No game is open: the report holds your latest saved game, if there is one.
ui-report-engine-failed = The game engine stopped. The report holds your latest saved game instead.

## Interiors
# Walking into a place from the street (DEC-088): the room, its hotspots, the door.
ui-interior-label = Inside { $place }. Arrow keys walk; Enter talks to the person beside you, uses the counter or desk you stand at, or goes out at the door; Escape goes out.
ui-interior-leave = Leave
ui-interior-seat = Take your seat
ui-interior-vote = Go to the vote
ui-interior-door = Out
ui-interior-entered = You go into { $place }.
ui-interior-left = You step back out into the street.
ui-interior-hint-person = Enter: talk to { $name }
ui-interior-hint-hotspot = Enter: { $what }
ui-interior-hint-door = Enter: out to the street
ui-interior-near-person = { $name } is beside you. Press Enter to talk.
ui-interior-near-hotspot = { $what }. Press Enter to use it.
ui-interior-near-door = The door. Press Enter to go out.
ui-interior-read = Read it
ui-interior-read-note = Open what this place has on file.
ui-interior-nothing = Nothing to do here just now.
ui-interior-staff = Staff
interior-hotspot-speak = The lectern
interior-hotspot-vote = Your seat
interior-hotspot-clerk = The clerk's table
interior-hotspot-window1 = Window 1: filings
interior-hotspot-window2 = Window 2: drafting
interior-hotspot-notices = Notice board
interior-hotspot-leaflets = Budget leaflets
interior-hotspot-offices = The offices
interior-hotspot-forms = Forms counter
interior-hotspot-booth_mid = A quiet booth
interior-hotspot-booth_front = The front booth
interior-hotspot-booth_back = The back booth
interior-hotspot-counter = The counter

## Daily routines (S1, DEC-031): parts of the day, and where people were seen
day-part-morning = in the morning
day-part-afternoon = in the afternoon
day-part-evening = in the evening
day-part-late-night = late at night
seen-last = Last seen at { $place }, { $part }.
seen-usually = Usually at { $place } { $part }.
lead-usually-at = Usually at { $to_place } ({ $to_kind }) { $part }.
lead-usually-at-kind = Usually at { $to_place } { $part }.
seen-place-home = home
seen-place-unknown = a place you have not named
lead-usually-on-streets = Usually on the streets of { $to_hood } { $part }.

## Notebook people and markers: the Notebook's People section (the single
## place for people: key figures pinned, then everyone else you know) and the
## name tags' markers in the street and in rooms.
ui-people-key = Key figures
ui-people-others = Everyone else you know
ui-people-power = Power, plans and gossip
ui-people-go = Go to them
ui-people-go-to = Go to { $name }
ui-tag-can-talk = { $name }, can talk
ui-tag-has-news = { $name }, has something new

## What matters now (DEC-113; POCKET_REWORK §3.3): the engine's reasons a
## person or a place matters (`NowReasonView.key`), an arc's clock and what
## comes next (`NextUpView.key`). Each is read on the row of the person or
## place it is about, so it names nobody. 34 characters at most in English.
# A person.
now-ask-offer = Has asked you for something
now-ask-promised = You gave them your word
now-ask-talk = An ask needs a word with them
now-ask-named = Named in something asked of you
now-arc-talk = A word with them could move it
now-decision = In a decision waiting on you
now-vote-undecided = Undecided on your bill
now-vote-member = Has a vote on your bill
now-here = Here with you now
now-rival = Your rival in this election
# A place.
now-place-scene = A decision can be faced here
now-place-ask = Something asked of you is here
now-place-beat = There is something to do here
now-place-here = You are here
# The clock of something under way ($days: 2 or more).
now-clock-today = Moves on today
now-clock-tomorrow = Moves on tomorrow
now-clock-days = Moves on in { $days } days
# The next dated thing, when nothing is waiting on the player.
now-next-vote = A bill comes to a vote
now-next-ask = A promise of yours falls due
now-next-arc = Something under way moves on
now-next-election = Election day

## Knowledge in words (DEC-113; DEC-091: knowledge is not a checklist): the
## engine's notes (`note` on a person, a neighbourhood, a place, a bill and
## a notebook entry) and the lead toward an arc only heard of. One whole
## sentence or two, said only where the gap or the fact changes something
## the player could do. $month is a month's name.
know-person-name-only = You only know the name.
know-person-lobby = You hardly know them, so lobbying them works less often.
know-hood-worries = You haven't heard what worries { $hood }.
know-bill-support = You don't know how your voters stand on this. The estimate is rough.
know-figures-stale = These figures are from { $month } { $year }. They may have moved.
know-place-shut = It has shut. Nothing can be done here.
know-arc-lead = Something is going on in { $where }. People there would know more.
