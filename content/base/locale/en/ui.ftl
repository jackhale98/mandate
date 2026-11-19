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
ui-notebook-lead-gives = would tell you: { $level }
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
