## Base pack — English (reference locale, DEC-054)

pack-base = Mandate base content

community-general = General population

sector-agriculture = Agriculture
sector-resources = Resources
sector-manufacturing = Manufacturing
sector-construction = Construction
sector-retail = Retail & services
sector-tourism = Tourism & hospitality
sector-knowledge = Knowledge & tech
sector-public = Public sector

app-title = Mandate
app-tagline = A strategy game about one politician’s career.

## Events (keys emitted by the engine; DEC-054)
event-game-started = A new history begins (seed { $seed }).
event-year-began = The year { $year } begins.
event-turn-ended = Turn { $turn } ended.
event-regional-shock = An economic shock hits { $region } ({ $delta } residents).

## Progress stages
progress-generating-world = Generating the world…

## UI chrome (web shell)
ui-new-game = New career
ui-continue = Continue
ui-delete = Delete
ui-delete-confirm = Delete this run permanently?
ui-saved-runs = Your runs
ui-no-runs = No runs yet.
ui-world-size = Country size
ui-size-small = Small
ui-size-medium = Medium
ui-size-large = Large
ui-mode = Mode
ui-mode-ironman = Ironman
ui-mode-ironman-help = One timeline. Every turn is saved automatically.
ui-mode-story = Story
ui-mode-story-help = Lets you return to chapter checkpoints.
ui-seed = Seed (optional)
ui-start = Begin
ui-loading-engine = Starting the engine…
ui-loading = Loading…
loader-local = Local
loader-regional = Regional
loader-national = National
loader-presidency = Presidency
ui-end-turn = End turn
ui-save-quit = Save & quit
ui-turn = Turn { $turn }
ui-turn-span = { $days ->
    [one] 1 day per turn
   *[other] { $days } days per turn
}
ui-population = Population
ui-regions = Regions
ui-recent = Recent developments
ui-run-turn = Turn { $turn } · { $date }
ui-mode-label = { $mode ->
    [ironman] Ironman
   *[story] Story
}
ui-dismiss = Dismiss
ui-update-ready = A new version is ready.
ui-reload = Reload
ui-offline-ready = Ready to play offline.
ui-foundation-note = Foundation build: the world is a placeholder until world generation lands (M1).
month-1 = January
month-2 = February
month-3 = March
month-4 = April
month-5 = May
month-6 = June
month-7 = July
month-8 = August
month-9 = September
month-10 = October
month-11 = November
month-12 = December
ui-date = { $day } { $month } { $year }

## World (M1)
ui-map = Map
ui-overview = Overview
ui-close = Close
ui-region = Region
ui-climate = Climate
ui-resources = Resources
ui-hazards = Hazards
ui-neighborhoods = Neighbourhoods
ui-places = Places
ui-municipalities = Towns and cities
ui-show-towns = Show towns
ui-hide-towns = Hide towns
ui-scenario = Scenario
ui-scenario-random = Random country
ui-elevation = { $m } m elevation
ui-temperature = { $c } °C average
ui-precipitation = { $mm } mm rain per year
ui-coastal = Coastal
ui-port = Port
ui-river = On a major river
ui-capital = National capital
ui-regional-capital = Regional capital
ui-map-help = Drag to pan, pinch or scroll to zoom, tap a town for details.
ui-generating = Building your country…
ui-index = { $pct }%
scenario-industrial_decline = Declining industrial heartland
scenario-wealthy_coast = Wealthy coastal nation
scenario-tech_growth = Fast-growing tech economy
scenario-drought_farmland = Drought-prone farmland
scenario-resource_frontier = Resource-rich frontier

class-metro = Metropolis
class-city = City
class-town = Town
class-rural = Rural municipality

biome-ocean = Ocean
biome-lake = Lake
biome-beach = Coast
biome-wetland = Wetland
biome-grassland = Grassland
biome-temperate_forest = Temperate forest
biome-rainforest = Rainforest
biome-boreal = Boreal forest
biome-shrubland = Shrubland
biome-steppe = Steppe
biome-desert = Desert
biome-tundra = Tundra
biome-alpine = Alpine
biome-rock = High mountains

geo-fertility = Farmland
geo-timber = Timber
geo-minerals = Minerals
geo-fishing = Fishing
geo-energy = Energy
geo-scenic = Scenery
geo-flood = Flooding
geo-drought = Drought
geo-wildfire = Wildfire
geo-seismic = Earthquakes
geo-storm = Storms

hood-core = Centre
hood-inner = Inner district
hood-waterfront = Waterfront
hood-industrial = Industrial district
hood-suburb = Suburb
hood-rural = Countryside

place-town_hall = Town hall
place-council_chamber = Council chamber
place-diner = Diner
place-market = Market
place-school = School
place-clinic = Clinic
place-community_centre = Community centre
place-park = Park
place-news_office = Newspaper
place-party_office = Party office
place-union_hall = Union hall
place-business_association = Chamber of commerce
place-factory = Factory
place-port = Port
place-farm = Farm
place-mine = Mine
place-mill = Sawmill
place-hotel = Hotel
place-university = University
place-tech_campus = Tech campus
place-regional_assembly = Regional assembly
place-parliament = Parliament
place-executive_residence = Executive residence
place-home = Home
place-streets = Streets

progress-terrain = Raising the land…
progress-climate = Bringing the rain…
progress-biomes = Growing forests and grasslands…
progress-settlements = Founding towns…
progress-territories = Drawing municipal borders…
progress-regions = Forming regions…
progress-transport = Building roads and railways…
progress-neighborhoods = Settling neighbourhoods…
progress-districts = Drawing electoral districts…
progress-finishing = Finishing touches…
progress-done = Ready.

## Economy (M2)
ui-economy = Economy
ui-recession = Global recession
ui-unemployment = Unemployment
ui-inflation = Inflation
ui-interest-rate = Interest rate
ui-growth = Growth (annualised)
ui-debt = National debt / output
ui-output = Annual output
ui-months-ago = { $n ->
    [0] this month
    [one] 1 month ago
   *[other] { $n } months ago
}
ui-trend-12m = { $what }, last 12 months
ui-overlay = Map layer
ui-overlay-terrain = Terrain
ui-overlay-unemployment = Unemployment
ui-legend-low = Low
ui-legend-high = High
ui-local-economy = Local economy
ui-typical-income = Typical income
ui-communities = Communities
ui-rent = Average rent / month
ui-home-price = Average home price
ui-jobs-by-sector = Jobs by sector
ui-local-budget = Town budget
ui-budget = Annual budget
ui-local-debt = Debt
ui-last-balance = Last year's balance

commodity-agriculture = farm produce
commodity-minerals = minerals
commodity-energy = energy

hazard-flood = Flooding
hazard-drought = Drought
hazard-wildfire = Wildfire
hazard-storm = A storm
hazard-earthquake = An earthquake

event-history-began = Recorded history begins in { $year }.
event-global-recession = A global recession begins (demand down { $depth }%).
event-global-recovery = The global economy begins to recover.
event-commodity-spike = Prices of { $commodity } jump { $change }%.
event-commodity-slump = Prices of { $commodity } slump ({ $change }%).
event-plant-closure = { $sector } layoffs in { $town }: { $jobs } jobs lost.
event-rate-rise = The central bank raises rates to { $rate }%.
event-rate-cut = The central bank cuts rates to { $rate }%.
event-disaster = { $hazard } strikes { $town } (severity { $severity }%).
event-budget-surplus = The { $year } national budget closed in surplus. Debt is { $debt }% of output.
event-budget-deficit = The { $year } national budget closed in deficit. Debt is { $debt }% of output.
progress-history = Living through twenty years of history…

## Constitution (M3)
constitution-federal_presidential = Federal Presidential Republic
body-council = Town council
body-regional-assembly = Regional assembly
body-house = House of Representatives
body-senate = Senate
office-mayor = Mayor
office-governor = Governor
office-president = President
# A jurisdiction (DEC-098): a town or a region by its name ($name).
jurisdiction-municipality = { $name }
jurisdiction-region = { $name }
jurisdiction-nation = the nation

## Policies (M3)
policy-transit = Transit frequency
policy-transit-desc = Run buses and trams more often and on more routes.
policy-affordable-housing = Affordable housing programme
policy-affordable-housing-desc = The town builds homes for people on modest incomes.
policy-upzoning = Upzoning
policy-upzoning-desc = Allow taller and denser buildings, so private builders can add homes.
policy-small-business = Small business grants
policy-small-business-desc = Subsidise wages at shops, cafés, and hotels.
policy-tech-incubator = Tech incubator
policy-tech-incubator-desc = Subsidise start-ups and knowledge jobs.
policy-police = Police staffing
policy-police-desc = Pay for more officers on the street, a service the town funds every year.
policy-flood = Flood defences
policy-flood-desc = Build levees, barriers, and drainage.
policy-wage-floor = Local wage floor
policy-wage-floor-desc = Require employers to pay above the going rate for low-paid work.
policy-parks = Parks and green space
policy-parks-desc = Create and maintain parks, trees, and playgrounds.
policy-hardship = Hardship fund
policy-hardship-desc = Pay a yearly grant to low-income residents.

param-service-level = Service level
param-homes = New homes per year
param-density = Extra density allowed
param-subsidy = Wage subsidy
param-protection = Protection level
param-wage-uplift = Wage floor above market
param-payment = Payment per person
unit-level = level
unit-homes-per-year = homes / year
unit-percent-wages = % of wages
unit-percent = %
unit-currency-per-year = per year

secondary-transit-congestion = Better buses draw people in, and rents rise with them.
secondary-transit-taxes = Ongoing cost competes with other services.
secondary-housing-rents = Rents ease over time as supply grows.
secondary-housing-neighbours = Some neighbours object to new estates nearby.
secondary-upzoning-character = Older streets change character.
secondary-upzoning-construction = A building boom adds construction jobs.
secondary-grants-deadweight = Some grants pay for jobs that would have existed anyway.
secondary-incubator-rents = Subsidised start-ups hire people other employers wanted.
secondary-police-liberties = Heavier policing worries civil-liberties groups.
secondary-flood-landscape = Barriers alter the waterfront.
secondary-wage-floor-jobs = Some low-paid jobs may disappear.
secondary-parks-upkeep = Parks need upkeep every year.
secondary-hardship-work = Critics say it may discourage taking low-paid work.
policy-property-tax = Property tax change
policy-property-tax-desc = Raise or cut the local property tax to fund programmes or ease bills.
param-tax-change = Change in rate
unit-percent-points = percentage points
secondary-property-tax-owners = Homeowners feel it first: homes are worth less, and builders start fewer.

## Office (M3)
ui-office = Office
role-councillor = Councillor
role-mayor = Mayor
role-private = Out of office
ui-ward = Ward { $n }
ui-mandate = Mandate
ui-money = Campaign funds
ui-slots = { $left ->
    [0] No time left this turn
    [one] 1 slot left this turn
   *[other] { $left } slots left this turn
}
ui-late-night = Late night available
ui-late-night-used = Late night used
ui-tracks = Pressure tracks
track-viability = Viability
track-record = Record of work
track-standing = Standing
track-legal_jeopardy = Legal jeopardy
track-threat = Threat
track-health = Health
ui-town-opinion = Town opinion
ui-gov-approval = Council and mayor approval
ui-you-in-ward = You, in your ward
ui-you-in-town = You, across town
ui-awareness = know you
ui-approval = net approval
ui-concerns = What people worry about
concern-jobs = Jobs
concern-housing = Housing costs
concern-environment = Environment
concern-taxes = Taxes
concern-services = Public services
concern-recovery = Rebuilding after the disaster
# Concerns as people name them in conversation: lower-case noun phrases
# that read mid-sentence.
talk-concern-jobs = jobs
talk-concern-housing = the rent
talk-concern-environment = the pollution
talk-concern-taxes = the tax bills
talk-concern-services = the state of the services
talk-concern-recovery = the rebuilding
ui-agenda = Agenda
ui-agenda-help = You have three slots a turn. Each action shows what it costs in time and health, and where it is done: choosing one walks you there (walking is free). A late-night meeting uses an extra slot but costs health and carries legal risk.
action-canvass = Canvass a neighbourhood
action-town-hall = Hold a town hall
action-press = Talk to the press
action-fundraise = Fundraise
action-rest = Rest
action-advertise = Buy local ads
action-lobby = Lobby
action-trade-favor = Trade a favour
ui-late-night-meeting = Late-night meeting
ui-about-bill = About a bill (optional)
ui-none = None
ui-neighbourhood = Neighbourhood
ui-do = Go
outcome-drafted = Draft created.
outcome-amended = Bill amended.
outcome-introduced = Bill introduced. First vote in { $days } days.
outcome-withdrawn = Bill withdrawn.
outcome-lobby-success = { $name } was persuaded.
outcome-lobby-fail = { $name } was not convinced.
outcome-favor-traded = { $name } will support you. You owe a favour.
outcome-town-hall = The town hall meeting in { $place } drew a crowd.
outcome-press = Your statement ran in the local press.
outcome-canvass = You knocked on doors in { $place }.{ $fresh ->
        [recent] { " " }People here saw you not long ago, so it won fewer of them over.
        [tired] { " " }People here have seen a lot of you lately: it won few of them over.
       *[new] { "" }
    }
outcome-fundraise = You raised { $amount }.
outcome-rest = You rested and recovered.
outcome-advertise = Your ads are running ({ $amount } spent).
outcome-filed-council = You will stand for your ward at the next election.
outcome-filed-mayor = You will seek your party's nomination for mayor.
outcome-filed-retire = You will not stand at the next election.
ui-bills = Bills and laws
ui-no-bills = You have not proposed anything yet.
ui-propose = Propose a policy
ui-catalog = Policy ideas
ui-draft = Create draft
ui-introduce = Introduce to council
ui-withdraw = Withdraw
ui-amend = Save changes
ui-amend-open = Amend
ui-why = Why?
ui-est-cost = Estimated cost { $amount } / year
ui-funded = { $pct }% funded
ui-progress = Implementation { $pct }%
ui-next-vote = { $stage } on { $date }
ui-whip = Expected yes { $yes } of { $seats } (needs { $needed })
# Who signs a bill ($office: the office's label) and how they lean.
ui-signer-lean = { $office }: { $lean }
ui-support = Expected public support
ui-support-low = Low income
ui-support-middle = Middle income
ui-support-high = High income
ui-support-overall = Voters overall
ui-secondary = Side effects
ui-ledger = What it has done
ui-votes = Votes
ui-vote-record = { $yes }–{ $no } { $result }
ui-passed = passed
ui-failed = failed
ui-vetoed = vetoed
ui-clauses = Riders
ui-earmark = Project in ward { $n }
ui-add-earmark = Add a project for a ward
ui-sunset = Expires after { $years } years
ui-add-sunset = Add an expiry date
ui-pilot = Pilot in { $place } only
ui-add-pilot = Make it a pilot
ui-remove = Remove
ui-select-bill = Bill
ui-no-pending = No bills before the council. Draft one at the town hall.
ui-yes-chance = { $pct }% yes
ui-yes = Yes
ui-no = No
ui-you = You
ui-members = Members
ui-recent-actions = What you did recently
ui-impl-months = { $n ->
    [one] Takes 1 month to set up
   *[other] Takes { $n } months to set up
}
ui-late-night-short = Late night
ui-whip-help = Lobbying uses a slot. A late-night meeting uses your extra slot: better odds, but it costs health and adds legal risk. Trading a favour wins a vote for sure, but you will owe them.
ui-amend-cost = Amending a bill already before the council uses a slot and halves the support you have built for it.
ui-odds = { $pct }% odds
ui-favors = { $n ->
    [0] No favours
    [one] Owes you 1 favour
   *[other] Owes you { $n } favours
}
ui-owe = { $n ->
    [one] You owe them a favour
   *[other] You owe them { $n } favours
}
ui-opinion-of-you = Opinion of you { $n }
ui-pragmatic = Pragmatist
ui-ideologue = Conviction politician
status-draft = Draft
status-under_consideration = Before the council
status-enacted = Law (awaiting funds)
status-in_effect = In effect
status-ended = Ended
end-defeated = Defeated
end-vetoed = Vetoed
end-withdrawn = Withdrawn
end-repealed = Repealed
end-expired = Expired
stage-vote-council = Council vote
stage-sign-mayor = Mayor's signature
lean-yes = Yes
lean-lean_yes = Leaning yes
lean-undecided = Undecided
lean-lean_no = Leaning no
lean-no = No
signer-sign = will sign
signer-lean_sign = leaning to sign
signer-undecided = undecided
signer-lean_veto = leaning to veto
signer-veto = will veto
## A law's ledger (audit fix 5): counts the law made itself are exact
## ("started", "supported", "created", "paid"); what it added to the town,
## net of what it displaced, is an estimate and says so.
ledger-homes_built = Homes started
ledger-homes_started = Homes started
ledger-homes_building = Homes still being built
ledger-homes_added = Homes added, net (estimate)
ledger-funded_pct = Funded (%)
ledger-service_gain_pct = Service level gain (%)
ledger-extra_building_pct = Extra private building (%)
ledger-wage_uplift_pct = Low-wage uplift (%)
ledger-low_income_added_pct = Low incomes raised, net (estimate, %)
ledger-damage_reduction_pct = Damage reduction (%)
ledger-paid_per_person = Paid per person so far
ledger-tax_change_pts = Tax change (points)
ledger-rent_cap_pct = Most rents may rise a year (%)
ledger-rent_held_pct = Rents held below the market (estimate, %)
ledger-borrowed_pct = Borrowed (% of its cost)
ledger-jobs_supported_agriculture = Farm jobs supported
ledger-jobs_supported_resources = Resource jobs supported
ledger-jobs_supported_manufacturing = Factory jobs supported
ledger-jobs_supported_construction = Construction jobs supported
ledger-jobs_supported_retail = Retail jobs supported
ledger-jobs_supported_tourism = Tourism jobs supported
ledger-jobs_supported_knowledge = Knowledge jobs supported
ledger-jobs_supported_public = Public-sector jobs supported
ledger-jobs_created_agriculture = Farm jobs created
ledger-jobs_created_resources = Resource jobs created
ledger-jobs_created_manufacturing = Factory jobs created
ledger-jobs_created_construction = Construction jobs created
ledger-jobs_created_retail = Retail jobs created
ledger-jobs_created_tourism = Tourism jobs created
ledger-jobs_created_knowledge = Knowledge jobs created
ledger-jobs_created_public = Public-sector jobs created
ledger-jobs_added_agriculture = Farm jobs added, net (estimate)
ledger-jobs_added_resources = Resource jobs added, net (estimate)
ledger-jobs_added_manufacturing = Factory jobs added, net (estimate)
ledger-jobs_added_construction = Construction jobs added, net (estimate)
ledger-jobs_added_retail = Retail jobs added, net (estimate)
ledger-jobs_added_tourism = Tourism jobs added, net (estimate)
ledger-jobs_added_knowledge = Knowledge jobs added, net (estimate)
ledger-jobs_added_public = Public-sector jobs added, net (estimate)
effect-inspection = Cleaner air; local industry loses { $output }% of its orders for { $months } months
response-inspection = demanded an inspection
conviction-pro-redistribution = For redistribution
conviction-anti-redistribution = Against redistribution
conviction-pro-tax_tolerance = Accepts taxes
conviction-anti-tax_tolerance = Tax-averse
conviction-pro-social_liberalism = Socially liberal
conviction-anti-social_liberalism = Socially conservative
conviction-pro-environment = Green
conviction-anti-environment = Growth before green
conviction-pro-law_and_order = Tough on crime
conviction-anti-law_and_order = Civil libertarian
conviction-pro-localism = Localist
conviction-anti-localism = Centraliser
conviction-pro-tradition = Traditionalist
conviction-anti-tradition = Modernizer
conviction-pro-reform_appetite = Reformer
conviction-anti-reform_appetite = Defender of the status quo
category-infrastructure = Infrastructure
category-housing = Housing
category-administration = Administration
category-economic_development = Economic development
category-safety = Safety
category-environment = Environment
category-social = Social
category-education = Education
category-health = Health
event-policy-introduced = A new bill in { $town }: { $policy }.
event-policy-enacted = { $town } passed the { $policy } bill.
event-policy-override = { $town } overrode the mayor's veto of the { $policy } bill ({ $yes }–{ $no }).
event-policy-vetoed = The mayor of { $town } vetoed the { $policy } bill.
event-policy-defeated = { $town } voted down the { $policy } bill ({ $yes }–{ $no }).
event-policy-expired = The { $policy } law expired in { $town }.
event-policy-withdrawn = The { $policy } bill was withdrawn in { $town }.
event-campaign-opened = Campaign season has begun. Local elections are on { $day } { $month_key } { $year }.
event-polls-published = New polls are out.
event-player-elected = You won your race in { $town }.
event-player-elected-mayor = You were elected mayor of { $town }.
event-player-defeated = You lost your race in { $town }. { $name } won.
event-mayor-ousted = { $town } threw out its mayor: { $name } won.
event-mayor-elected = { $name } was elected mayor of { $town }.
event-councillor-ousted = { $name } unseated a sitting councillor in { $town }.
event-councillor-elected = { $name } won a council seat in { $town }.
event-election-held = Local elections were held ({ $races } races).

## Campaign (M4)
ui-campaign = Campaign
ui-campaign-season = Campaign season
ui-next-election = Next local election
ui-days-left = { $n ->
    [0] Today
    [one] 1 day
   *[other] { $n } days
}
ui-campaign-opens = campaign opens { $date }
ui-your-party = Your party
ui-independent = Independent
ui-your-plans = Your plans
# Filing ($post: the post's label, as on the ballot).
ui-file-defend = Stand again: { $post }
ui-file-post = Run for { $post }
ui-file-retire = Stand down
ui-filing-help = Candidates are fixed when the campaign opens.
ui-filing-closed = Nominations have closed for this election.
ui-nomination-standing = Your party will only back you for { $post } with Standing of { $min } or more.
ui-nomination-renomination = Your party will not renominate you with Standing below { $min }.
ui-not-running = You are not on the ballot this time.
# The filing form: who holds each post, whether they stand again (in words:
# the party's draw is not shown), and the Standing the party asks.
ui-nomination-holder = Held by { $holder }
ui-nomination-vacant = Nobody holds it now.
ui-holder-usually-stands = usually stands again
ui-holder-term-limited = has served the term limit
ui-holder-retiring = is expected to retire
ui-nomination-backs-holder = Your party backs its own holder when they stand again.
ui-nomination-warn-seat = If { $holder } stands again, the party nominates them and you stand for your seat instead.
ui-nomination-warn-none = If { $holder } stands again, the party nominates them and you will not be on the ballot.
ui-nomination-gate = Your party asks Standing of { $min } for this. Yours is { $standing }.
# A nomination the party refused when the campaign opened (play note 20):
# the reason, then what the player stands for now.
nomination-refused = { $why } { $then }
nomination-why-holder = Your party has nominated { $holder } again for { $post }.
nomination-why-standing = Your party will not back you for { $post } with Standing of { $standing }. It asks { $min }.
nomination-why-party = Your party is putting no one up for { $post } this time.
nomination-why-other = Your party did not nominate you for { $post }.
nomination-then-seat = You are standing for your seat in { $district }.
nomination-then-post = You are standing for { $instead } again.
nomination-then-none = You are not on the ballot.
# A districted seat by its number ($tier: the post's tier); any other post
# goes by its constitution label.
ui-post-seat = { $tier ->
    [local] Ward { $n }
   *[other] District { $n }
}
ui-your-race = Your race
ui-incumbent = incumbent
ui-know-you = Voters who know you
ui-polls = Polls
ui-no-polls = No polls yet. The first arrive two weeks into the campaign.
ui-polls-note = Polls are estimates from samples of voters. They can all be wrong in the same direction.
ui-poll-trend = Poll trend
ui-poll-table = Show polls as a table
ui-date-col = Date
ui-pollster = Pollster
ui-undecided-col = Undecided
ui-undecided = { $pct }% undecided
ui-other-races = Other races in town
ui-last-results = Last election
ui-turnout = Turnout { $pct }%
ui-winner = Winner
ui-final-poll = final polls { $pct }%
ui-why-result = Why it went this way
ui-why-result-help = How much each factor added to (or took from) the winner's lead, in points.
ui-why-lost-help = How much each factor helped you (+) or cost you (−) against the winner, in points.
factor-issues = Issues and positions
factor-party = Party loyalty
factor-valence = Candidate quality
factor-record = Record in office
factor-campaign = Campaigning
factor-shock = Late swing and luck
ui-you-won = You won.
ui-you-lost = You lost.
ui-election-night = Election night
# Election night for a player on no ballot, and what became of the post they
# held ($post: as on the ballot; $winner: who won it).
ui-night-not-running = You were not on the ballot.
ui-seat-held = You were re-elected: { $post }.
ui-seat-lost = You lost { $post } to { $winner }.
ui-seat-passed = { $winner } now holds { $post }, the post you held.
ui-seat-not-contested = { $post } was not up this time. You still hold it.
ui-night-other-seats = The other seats
ui-night-won-by = won by { $winner }
ui-continue-game = Continue
ui-parties = Parties

## Trade-offs (DEC-073)
tradeoff-canvass = A long day on doorsteps: the deepest contact there is. Builds name recognition and turnout in one neighbourhood; slow but lasting.
tradeoff-town-hall = Reaches a neighbourhood and puts a bill in the spotlight, for better or worse.
tradeoff-press = Reaches the whole town a little, but a gaffe costs goodwill.
tradeoff-fundraise = Money and party goodwill, but low-income voters notice who you court.
tradeoff-rest = Recovers health; below 35 health you lose a slot each turn.
tradeoff-advertise = Costs { $cost } per buy, no time. Reaches every neighbourhood thinly; the effect fades and does little for turnout.
ui-risk-hostile = Hostile-crowd risk { $pct }%.
ui-risk-gaffe = Gaffe risk { $pct }%.
ui-risk-late-money = Late-night money adds legal risk.
ui-buy-ads = Buy ads ({ $cost }) · { $left } left this turn
ui-tired = { $n ->
    [one] You are worn out: one fewer slot until you recover.
   *[other] You are exhausted: { $n } fewer slots until you recover.
}
outcome-town-hall-hostile = The town hall meeting in { $place } turned hostile.
outcome-press-gaffe = Your remarks were taken badly. The clip is everywhere.
ui-your-vote = Your vote
ui-party-line-yes = Your party expects you to vote yes.
ui-party-line-no = Your party expects you to vote no.
ui-party-line-free = A free vote: your party has no line.
ui-ward-view = Your ward's view: { $lean }. { $for }% for, { $against }% against ({ $certainty }).
ward-lean-strongly_against = strongly against
ward-lean-against = against
ward-lean-leans_against = leans against
ward-lean-split = split
ward-lean-leans_for = leans for
ward-lean-for = for
ward-lean-strongly_for = strongly for
ward-certainty-rough = rough estimate
ward-certainty-fair = fair estimate
ward-certainty-exact = exact
# Why the party line is what it is (the line itself: ui-party-line-*).
party-reason-fits = It fits their stance on { $issue }.
party-reason-cuts = It cuts against their stance on { $issue }.
party-reason-own = It is their own member's bill.
party-reason-governing = It is the governing party's bill.
party-reason-rival = It is a rival party's bill.
party-reason-free = Nothing in their platform settles it.
party-reason-free-torn = Their stance on { $issue } and who proposed it pull in opposite directions.
party-issue-redistribution = inequality and welfare
party-issue-tax_tolerance = taxes and spending
party-issue-social_liberalism = social questions
party-issue-environment = the environment
party-issue-law_and_order = policing and safety
party-issue-localism = local self-reliance
party-issue-tradition = tradition and community
party-issue-reform_appetite = reforming government
ui-owe-sponsor = You owe { $name } a favour: voting yes repays it; voting no will be remembered.
ui-vote-yes = Vote yes
ui-vote-no = Vote no
ui-vote-abstain = Abstain
ui-vote-help = Your vote is public and final. Defying the party costs Standing.
ui-voted-yes = You voted yes
ui-voted-no = You voted no
ui-voted-abstain = You abstained
outcome-vote-yes = You declared your vote: yes.
outcome-vote-no = You declared your vote: no.
outcome-vote-abstain = You declared you will abstain.
event-rules-updated = The game was updated (version { $version }). New rules apply from today.
event-proposal-opened = A new proposal came onto the table.
event-proposal-lapsed = A proposal left the table.
ui-cost-line = { $slots ->
    [one] 1 slot
   *[other] { $slots } slots
}{ $health ->
    [0] .
   *[other] , −{ $health } health.
}
ui-tradeoffs = Trade-offs
sidefx-disruption = Disruption while it is built ({ $size }).
sidefx-grievance = Resented by { $group } ({ $size }).
sidefx-labour_supply = { $group } work a little less ({ $size }).
sidefx-group-low = low-income residents
sidefx-group-low-older = older low-income residents
sidefx-group-middle = middle-income residents
sidefx-group-middle-older = older middle-income residents
sidefx-group-high = better-off residents
sidefx-group-high-older = older, better-off homeowners
sidefx-group-all = residents
sidefx-group-all-older = older residents and homeowners
sidefx-size-small = small
sidefx-size-moderate = moderate
sidefx-size-large = large
sidefx-size-relief = welcomed instead
event-player-deselected = Your party has refused to renominate you. You are off the ballot.
ui-party-patience = Your party is losing patience with your record. Below { $n } Standing it will not renominate you.

## Places as the interface (M5a, DEC-079)
ui-pocket = Pocket
ui-pocket-links = Elsewhere
ui-town = Town
ui-back-to-town = Back to town
ui-town-help = tap a place to go there
ui-streets-of = Streets of { $hood }
ui-you-are-here = You are here
ui-you-are-at = You are at: { $place }.
ui-home-hood = Home
ui-your-ward = Your ward
ui-also-here = Also here: { $list }
ui-not-here = You are not here. What is done here is listed below; the details are only on site.
ui-go-here = Go here
ui-do-here = Do it
ui-nothing-here = Nothing to do here yet.
ui-walking-to = Walking to { $place }…
ui-tap-to-skip = Tap to skip
ui-skip-walk = Skip the walk
ui-route-here = Done here.
ui-route-to = Done at { $place }: you will walk there first.
ui-slots-short = Time
ui-alerts = Alerts
ui-alert-count = { $n ->
    [one] 1 alert
   *[other] { $n } alerts
}
ui-your-bills = Your bills
ui-your-drafts = Your drafts
ui-no-drafts = No drafts. Pick a policy idea below to draft one.
ui-awaits-your-vote = awaits your vote
ui-whip-about = About { $yes } of { $seats } yes (needs { $needed })
ui-whip-in-chamber = The exact whip count, member by member, is in the council chamber.
ui-headlines = Headline numbers
ui-headlines-note = Rounded, as of last month. Precise figures by neighbourhood are at the town hall.
ui-top-concerns = Most on people's minds: { $list }.
ui-town-concerns = Top concerns: { $list }
ui-detail-at-town-hall = Rounded figures. The town hall's statistics desk has the detail: jobs by sector, the budget, and trends.
ui-catalog-help = Drafting is free. Introducing a draft to the council costs a slot, in the council chamber.
ui-agenda-places = Places for council business
ui-agenda-draft = Draft a bill (town hall)
ui-agenda-vote = Vote and whip (council chamber)
ui-agenda-file = Election filing (town hall)
ui-colleague = Colleague
ui-cost-late = Late-night slot, −{ $health } health.
ui-risk-late-deal = Back-room deals add legal risk.
ui-odds-in-chamber = Odds are shown in the council chamber.
ui-late-at-diner = Diner, late night
ui-vacancy = Vacant homes
ui-by-neighbourhood = By neighbourhood
ui-top-concern = Top concern
ui-programmes = Programmes in force
ui-internal-poll-note = Party polling: tighter than the public polls (about ±{ $pts } points), refreshed weekly.
ui-overheard-intro = What people in { $hood } are saying:
tradeoff-lobby = A private word with a colleague about a bill: it may or may not persuade them.
tradeoff-trade-favor = Wins their support for sure, but you will owe them a favour (repaid by backing their bills).
action-vote = Vote on bills
action-introduce = Introduce bills
action-amend = Amend bills
action-withdraw = Withdraw bills
action-draft = Draft bills
action-file = Election filing
service-whip_exact = Exact whip count
service-town_data_detailed = Town statistics
service-budget_papers = Budget papers
service-internal_polls = Internal polls
service-overheard = Overheard
outcome-moved = You went to the { $kind }.
alert-vote-needed = { $n ->
    [one] A bill before the council awaits your vote.
   *[other] { $n } bills before the council await your vote.
}
alert-tired = { $n ->
    [one] You are worn out: 1 slot fewer each turn until you rest.
   *[other] You are exhausted: { $n } slots fewer each turn until you rest.
}
alert-party-patience = Your party is losing patience. Below { $n } Standing it will not renominate you.
alert-legal = Your legal exposure is serious.
alert-campaign = Campaign season: { $n } days to the election.
short-canvass = Canvass
short-town_hall = Town hall meetings
short-press = Press
short-advertise = Ads
short-fundraise = Fundraise
short-lobby = Lobby
short-trade_favor = Favours
short-rest = Rest
short-vote = Vote
short-introduce = Introduce
short-amend = Amend
short-withdraw = Withdraw
short-take_bribe = An envelope
short-dip_funds = Campaign card
short-leak_file = Leak a file
short-draft = Draft bills
short-file = Election filing
short-whip_exact = Exact whip
short-town_data_detailed = Statistics
short-budget_papers = Budget papers
short-internal_polls = Internal polls
short-overheard = Overheard talk
short-casework = Casework
short-newspaper = Today's paper

# Characters (DEC-031): traits, occupations, and memories.
trait-grudge_holder = Holds grudges
trait-forgiving = Forgiving
trait-vain = Vain
trait-zealous = Zealous
trait-reformer = Reformer
trait-loyal_soldier = Loyal soldier
trait-maverick = Maverick
trait-charmer = Charmer
trait-schemer = Schemer
trait-cautious = Cautious
occupation-union_steward = Union steward
occupation-business_leader = Business association chair
occupation-editor = Newspaper editor
occupation-party_chair = Party branch chair
occupation-plant_manager = Plant manager
occupation-farmer = Farmer
occupation-journalist = Journalist
occupation-shopkeeper = Shopkeeper
occupation-diner_owner = Diner owner
occupation-organiser = Community organiser
occupation-clergy = Vicar
occupation-teacher = Teacher
occupation-nurse = Nurse
occupation-resident = Local resident
occupation-retiree = Retiree
# A character's memories of the player, on their sheet ("them" is the
# character). $subject is what it was about and $subject_kind what kind of
# thing that is: `bill` (a policy's name, a title), `decision` (a decision's
# title, a noun phrase), `seat`, or `none` (nothing in particular: the line
# leaves the subject out). $year is the year it happened. A speaker quoting
# one to the player's face uses `scene-quote-…` (narrative.ftl).
memory-backed-my-bill = { $subject_kind ->
    [bill] You backed their bill, { $subject } ({ $year }).
   *[other] You backed their bill ({ $year }).
}
memory-opposed-my-bill = { $subject_kind ->
    [bill] You voted against their bill, { $subject } ({ $year }).
   *[other] You voted against their bill ({ $year }).
}
memory-promised-favour = { $subject_kind ->
    [bill] You promised them a favour over the { $subject } bill ({ $year }).
    [decision] You promised them a favour over { $subject } ({ $year }).
   *[other] You promised them a favour ({ $year }).
}
memory-kept-promise = { $subject_kind ->
    [bill] You kept your promise on the { $subject } bill ({ $year }).
    [decision] You kept your promise over { $subject } ({ $year }).
   *[other] You kept a promise to them ({ $year }).
}
memory-broke-promise = { $subject_kind ->
    [bill] You broke your promise on the { $subject } bill ({ $year }).
    [decision] You broke your promise over { $subject } ({ $year }).
   *[other] You broke a promise to them ({ $year }).
}
memory-broke-ranks = { $subject_kind ->
    [bill] You broke ranks with the party on the { $subject } bill ({ $year }).
   *[other] You broke ranks with the party ({ $year }).
}
memory-lobbied-me = { $subject_kind ->
    [bill] You talked them round on the { $subject } bill ({ $year }).
    [decision] You talked them round over { $subject } ({ $year }).
   *[other] You talked them round ({ $year }).
}
memory-pressed-me = { $subject_kind ->
    [bill] You pressed them on the { $subject } bill and got nowhere ({ $year }).
    [decision] You pressed them over { $subject } and got nowhere ({ $year }).
   *[other] You pressed them and got nowhere ({ $year }).
}
memory-defeated-me = You beat them at the polls ({ $year }).
memory-lost-to-me = They beat you at the polls ({ $year }).
memory-vetoed-my-bill = { $subject_kind ->
    [bill] You vetoed their bill, { $subject } ({ $year }).
   *[other] You vetoed their bill ({ $year }).
}
memory-signed-my-bill = { $subject_kind ->
    [bill] You signed their bill, { $subject }, into law ({ $year }).
   *[other] You signed their bill into law ({ $year }).
}
memory-met = You first met ({ $year }).
memory-scandal = The scandal ({ $year }).

# Town character (DEC-077 §1): live crises.
crisis-plant_closure = Jobs disappearing
crisis-disaster_recovery = Recovering from disaster
crisis-housing_squeeze = Housing squeeze
crisis-fiscal_hole = Fiscal hole
# Issues, as a character's area of expertise (one per attitude).
issue-redistribution = Inequality and welfare
issue-tax_tolerance = Taxes and spending
issue-social_liberalism = Social questions
issue-environment = The environment
issue-law_and_order = Policing and safety
issue-localism = Local self-reliance
issue-tradition = Tradition and community
issue-reform_appetite = Reforming government

# Character sheets (DEC-031).
role-candidate = Candidate
subject-office = the post of { $post }
subject-ward = Ward { $n }
favour-bill = Support on { $subject }
favour-support = Backing you asked for
personality-ambition-high = Ambitious
personality-ambition-low = Content where they are
personality-integrity-high = Principled
personality-integrity-low = Flexible with the truth
personality-warmth-high = Warm
personality-warmth-low = Cold
personality-boldness-high = Bold
personality-boldness-low = Cautious by nature
personality-loyalty-high = Loyal
personality-loyalty-low = A free agent
opinion-hostile = Hostile to you
opinion-cool = Cool toward you
opinion-neutral = Neutral about you
opinion-warm = Warm toward you
opinion-friendly = Firmly on your side
trust-low = Doesn't trust you
trust-some = Trusts you a little
trust-high = Trusts you
bond-family = Family
bond-friend = Friend
bond-ally = Ally
bond-rival = Rival
bond-mentor = Mentor
bond-employer = Works for them
bond-patron = Patron
tier-principal = Major figure
tier-named = Local figure
tier-ambient = Passer-by
ui-people-here = People here
ui-people-here-none = Nobody you could stop for a word right now.
ui-meet = Say hello
ui-character = Who they are
ui-personality = Character
ui-traits = Known for
ui-expertise = Knows about
ui-relationship = With you
ui-favours = Favours
ui-owes-you = { $n ->
    [one] Owes you a favour
   *[other] Owes you { $n } favours
}
ui-you-owe-them = { $n ->
    [one] You owe them a favour
   *[other] You owe them { $n } favours
}
ui-remembers = Remembers
ui-remembers-nothing = Nothing about you yet.
ui-age-home = { $age }, lives in { $hood }
ui-found-at = Found at { $place }
ui-ward-n = Ward { $n }
ui-your-rival = Your rival
ui-power-broker = Power broker
outcome-met = You introduced yourself to { $name }.

# Town character (DEC-077 §1).
ui-town-card = The town
ui-defining-industry = Defining industry
ui-industry-share = { $pct }% of jobs
ui-live-crisis = Live crisis
ui-no-crisis = No crisis right now
crisis-severity-mild = mild
crisis-severity-serious = serious
crisis-severity-severe = severe
crisis-trend-worsening = worsening
crisis-trend-easing = easing
crisis-trend-steady = steady
ui-crisis-since = since { $month }/{ $year }
ui-power-broker-line = Power broker
ui-rival-line = Your rival
ui-priorities = Local priorities
ui-nobody = —
## Run export and import (DEC-082)
ui-export = Export
ui-export-label = Export { $name } to a file
ui-export-done = Saved { $file }. Keep it somewhere safe, or import it on another device.
ui-export-shared = Shared { $file }.
ui-export-failed = The run could not be exported.
ui-import = Import a run
ui-import-help = Restore a run from a { $ext } file: a backup, or a run from another device.
ui-import-drop = Or drop a { $ext } file anywhere on this page.
ui-import-drop-active = Drop the file to import it.
ui-import-working = Checking the file…
ui-import-added = Run imported. You can continue it above.
ui-import-replaced = Run updated to the newer save from the file.
ui-import-copied = This older Story save was added as a separate run. Your current run is unchanged.
ui-import-error-not-mandate = This is not a Mandate run file.
ui-import-error-too-new = This run was saved by a newer version of Mandate. Update the app, then import it again.
ui-import-error-corrupted = This file is damaged or incomplete. Try exporting the run again.
ui-import-error-incompatible = This run can't be loaded under the current rules of the game.
ui-import-error-rewind = This is an older save of an Ironman run you already have here. Ironman runs can't go back in time, so it was not imported. Your current run is unchanged.
ui-import-error-failed = The run could not be imported.

## Situational policies (DEC-077 §3)
policy-closure-response = Closure response package
policy-closure-response-desc = Pay people who lost their jobs while they retrain, and subsidise the firms that take them on.
policy-rebuild-fund = Disaster rebuild fund
policy-rebuild-fund-desc = Rebuild damaged homes, to a higher standard against the next flood or storm.
policy-emergency-rent = Emergency rent measure
policy-emergency-rent-desc = Rent relief for low-income tenants now, with an optional freeze on rent rises.
policy-fiscal-bridge = Fiscal bridge levy
policy-fiscal-bridge-desc = A temporary property levy to close the town's budget hole, with a rebate for the poorest households.
param-stipend = Retraining stipend per person
param-rehire = Wage subsidy for rehiring
param-rebuilt-homes = Homes rebuilt per year
param-rent-relief = Rent relief per household
param-rent-freeze = Rent freeze strictness
param-levy = Levy
param-rebate = Rebate per low-income resident
secondary-closure-retraining = Retraining takes months before people are back in work.
secondary-closure-subsidy = Subsidies can keep weak firms going past their time.
secondary-rebuild-standard = Rebuilt homes stand up better to the next flood or storm.
secondary-rebuild-wait = Rebuilding draws workers away from other employers in town.
secondary-rent-relief = Relief reaches tenants now, not in years.
secondary-rent-supply = A freeze slows new building; rents can jump when it lifts.
secondary-bridge-temporary = Sold as temporary; levies have a way of staying.
secondary-bridge-credit = Lenders breathe easier, and the town's borrowing costs ease.
unlock-closure = after a plant closure or heavy job losses
unlock-disaster = after a disaster strikes the town
unlock-rent = while rents squeeze the town
unlock-fiscal = while the town's finances are in a hole
ui-policy-situational = Called for now: { $reason }.
ui-policy-locked = Only on the table { $reason }.
ui-catalog-situational = Not on the table now
ui-catalog-situational-help = These answer particular situations. They open when the town is in one.
ui-preview-updating = Updating estimate…
ui-budget-share = { $pct }% of the town budget
ui-storyteller = Storyteller
ui-storyteller-label = { $name } storyteller

# Places with purpose (PL1): the verbs of the visit action.
verb-tour_works = Tour the works
verb-listen_at_gate = Listen at the gate
verb-photo_op_gate = Photo op at the gate
verb-visit_service = Look round
verb-talk_to_head = Talk to whoever runs it
verb-walk_fields = Walk the fields
verb-be_seen = Be seen
verb-meet_young_staff = Lunch with the young staff
verb-public_gallery = Sit in the public gallery
action-visit = A visit
action-verb-tour-works = Toured the works
action-verb-listen-at-gate = Listened at the gate
action-verb-photo-op-gate = Photo op at the gate
action-verb-visit-service = Looked round
action-verb-talk-to-head = Talked to whoever runs it
action-verb-walk-fields = Walked the fields
action-verb-be-seen = Was seen about
action-verb-meet-young-staff = Lunch with the young staff
action-verb-public-gallery = Sat in the public gallery
outcome-verb-tour_works = You toured { $place }. { $outlook ->
        [growing] The order book is full and they are taking people on.
        [shedding] The manager talks about "difficult months". Shifts are being cut.
       *[other] Work is steady.
    }
outcome-verb-listen_at_gate = You stood at the gate of { $place } as the shift changed, and listened.
outcome-verb-photo_op_gate = Hard hat on, hand shaken: the photographer got the picture at { $place }.
outcome-verb-photo_op_gate-heckled = The photo at { $place } went wrong: workers who fear for their jobs heckled you on camera.
outcome-verb-visit_service = You looked round { $place } and saw how stretched it is.
outcome-verb-talk_to_head = You sat down with { $name } at { $place }.
outcome-verb-walk_fields = You walked the fields at { $place } with the people who work them.
outcome-verb-be_seen = You took a turn round { $place } and said a few hellos.
outcome-verb-meet_young_staff = You had lunch with the young staff at { $place }.
outcome-verb-public_gallery = You watched from the public gallery at { $place }.
visit-blocked-hours = Not at this time of day.
visit-blocked-late = Not late at night.
visit-blocked-situation = Not while things stand as they do.
visit-blocked-nobody = Nobody you need is here just now.
visit-blocked-time = Not enough time left this turn.
visit-blocked-away = Go there first.
ui-verbs = Things to do here
ui-verb-cost-half = ½ slot
ui-verb-cost-slot = 1 slot
ui-verb-risk = Risk { $pct }%.
ui-verb-done = Do it
occupation-shop_steward = Shop steward
occupation-harbourmaster = Harbourmaster
occupation-farmhand = Farmhand
occupation-head_teacher = Head teacher
occupation-gp = Doctor
occupation-dean = Dean
occupation-student = Student
occupation-hotel_manager = Hotel manager
occupation-park_keeper = Park keeper
occupation-tech_founder = Founder
occupation-software_engineer = Software engineer
