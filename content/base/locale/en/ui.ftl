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
