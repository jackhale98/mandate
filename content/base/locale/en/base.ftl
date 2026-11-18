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
app-tagline = You can only do what you can get others to do.

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
preset-industrial_decline = Declining industrial heartland
preset-wealthy_coast = Wealthy coastal nation
preset-tech_growth = Fast-growing tech economy
preset-drought_farmland = Drought-prone farmland
preset-resource_frontier = Resource-rich frontier

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
policy-police-desc = Hire and deploy more officers.
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

secondary-transit-congestion = Less traffic downtown; more noise along new routes.
secondary-transit-taxes = Ongoing cost competes with other services.
secondary-housing-rents = Rents ease over time as supply grows.
secondary-housing-neighbours = Some neighbours object to new estates nearby.
secondary-upzoning-character = Older streets change character.
secondary-upzoning-construction = A building boom adds construction jobs.
secondary-grants-deadweight = Some grants pay for jobs that would have existed anyway.
secondary-incubator-rents = Success can push up rents near the incubator.
secondary-police-liberties = Heavier policing worries civil-liberties groups.
secondary-flood-landscape = Barriers alter the waterfront.
secondary-wage-floor-jobs = Some low-paid jobs may disappear.
secondary-parks-upkeep = Parks need upkeep every year.
secondary-hardship-work = Critics say it may discourage taking low-paid work.
policy-property-tax = Property tax change
policy-property-tax-desc = Raise or cut the local property tax to fund programmes or ease bills.
param-tax-change = Change in rate
unit-percent-points = percentage points
secondary-property-tax-owners = Homeowners feel it first; renters see it later in rents.

## Office (M3)
ui-office = Office
ui-council = Council
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
concern-safety = Safety
concern-environment = Environment
concern-taxes = Taxes
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
outcome-town-hall = The town hall in { $place } drew a crowd.
outcome-press = Your statement ran in the local press.
outcome-canvass = You knocked on doors in { $place }.
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
ui-mayor-lean = Mayor: { $lean }
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
mayor-sign = will sign
mayor-lean_sign = leaning to sign
mayor-undecided = undecided
mayor-lean_veto = leaning to veto
mayor-veto = will veto
ledger-homes_built = Homes started
ledger-funded_pct = Funded (%)
ledger-service_gain_pct = Service level gain (%)
ledger-extra_building_pct = Extra private building (%)
ledger-wage_uplift_pct = Low-wage uplift (%)
ledger-damage_reduction_pct = Damage reduction (%)
ledger-paid_per_person = Paid per person so far
ledger-tax_change_pts = Tax change (points)
ledger-jobs_supported_retail = Retail jobs supported
ledger-jobs_supported_tourism = Tourism jobs supported
ledger-jobs_supported_knowledge = Knowledge jobs supported
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
event-policy-introduced = { $policy } was introduced in { $town }.
event-policy-enacted = { $town } passed { $policy } ({ $yes }–{ $no }).
event-policy-override = { $town } overrode the mayor's veto of { $policy } ({ $yes }–{ $no }).
event-policy-vetoed = The mayor of { $town } vetoed { $policy }.
event-policy-defeated = { $policy } was defeated in { $town } ({ $yes }–{ $no }).
event-policy-expired = { $policy } expired in { $town }.
event-policy-withdrawn = { $policy } was withdrawn in { $town }.
event-campaign-opened = Campaign season has begun. Local elections are on { $day } { $month_key } { $year }.
event-polls-published = New polls are out.
event-player-elected = You were re-elected in { $town }.
event-player-elected-mayor = You were elected mayor of { $town }!
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
ui-file-defend = Defend my seat
ui-file-council = Run for my old ward
ui-file-mayor = Run for mayor
ui-file-retire = Stand down
ui-filing-help = Candidates are fixed when the campaign opens.
ui-filing-closed = Nominations have closed for this election.
ui-mayor-needs-standing = Your party will only back you for mayor with Standing of 40 or more.
ui-not-running = You are not on the ballot this time.
ui-race-mayor = Mayor
ui-race-ward = Ward { $n }
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
outcome-town-hall-hostile = The town hall in { $place } turned hostile.
outcome-press-gaffe = Your remarks were taken badly. The clip is everywhere.
ui-your-vote = Your vote
ui-party-line-yes = Your party expects you to vote yes.
ui-party-line-no = Your party expects you to vote no.
ui-party-line-free = A free vote: your party has no line.
ui-ward-view = Your ward's view: { $n }
ui-owe-sponsor = You owe { $name } a favour: voting yes repays it; voting no will be remembered.
ui-vote-yes = Vote yes
ui-vote-no = Vote no
ui-vote-abstain = Abstain
ui-vote-help = Your vote is public and final. Defying the party costs Standing; your ward will remember.
ui-voted-yes = You voted yes
ui-voted-no = You voted no
ui-voted-abstain = You abstained
outcome-vote-yes = You declared your vote: yes.
outcome-vote-no = You declared your vote: no.
outcome-vote-abstain = You declared you will abstain.
event-rules-updated = The game was updated (version { $version }). New rules apply from today.
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
overheard-jobs = "Another lay-off notice at the plant. Who's hiring around here?"
overheard-housing = "The rent went up again. My kids can't afford to live here."
overheard-safety = "I don't let them walk home alone after dark any more."
overheard-environment = "Have you seen the river lately? Something's not right."
overheard-taxes = "Every bill that comes through the door is bigger than the last."
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
short-draft = Draft bills
short-file = Election filing
short-whip_exact = Exact whip
short-town_data_detailed = Statistics
short-budget_papers = Budget papers
short-internal_polls = Internal polls
short-overheard = Overheard talk
