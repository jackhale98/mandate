## The town map's layers (DEC-086, M6 note 11): what the town map can show
## over the town. Names, legend lines and notes come from the registry
## (`town_map_layers.jsonc`); the engine names the legend's entries, the
## filters and the choices (`town-map-…`); the rest (`ui-town-map-…`) is the
## picker, the legend, the person card and the list, asked for by the UI. The
## country map's layers are in `ui.ftl` (`map-layer-…`).

# Registry: each layer's name, its legend line and its note. $tier is the
# town's tier in the constitution, so a district is named in its words.
town-map-layer-districts = { $tier ->
    [local] Wards
   *[other] Districts
}
town-map-layer-districts-unit = Who holds each seat
town-map-layer-districts-note = Tap a portrait for the person, and a way to reach them.
town-map-layer-people = People
town-map-layer-people-unit = People you have heard of, where you know to find them
town-map-layer-people-note = A number is several people at one place. Tap it for the list.
town-map-layer-worries = Worries
town-map-layer-worries-unit = What each neighbourhood worries about, as far as you know
town-map-layer-worries-note = Striped: you have not heard what worries people there.

# Engine: legend entries, filters and the worries layer's choices.
town-map-legend-vacant = Vacant seat
town-map-legend-no-party = No party
town-map-filter-all = Everyone
town-map-filter-met = People you know
town-map-param-worry = Worry
town-map-worry-top = Top worry
town-map-worry-rank-top = Top worry
town-map-worry-rank-high = In the top three
town-map-worry-rank-low = A lesser worry

# UI: the picker and the list.
ui-town-map-layer-none = Places
ui-town-map-filter = Who to show
ui-town-map-list-show = Show this layer as a list
ui-town-map-list-caption = { $layer } in { $town }, as a list
ui-town-map-layer-empty = Nothing to show here yet.

# UI: districts.
ui-town-map-area-yours = Your seat
ui-town-map-area-home = You live here
ui-town-map-area-hoods = Covers { $list }
ui-town-map-area-vacant = The seat is vacant.

# UI: people.
ui-town-map-person = { $name }, { $role }
ui-town-map-person-open = Open their card
ui-town-map-where-unknown = You do not know where to find them yet.
ui-town-map-unplaced = Whereabouts not known
ui-town-map-cluster = { $n } people at { $place }
ui-town-map-cluster-title = At { $place }

# UI: worries.
ui-town-map-unknown = Not known yet
ui-town-map-readout-hint = Tap a neighbourhood to read it.
ui-town-map-readout-worries = Worries you know of: { $list }
