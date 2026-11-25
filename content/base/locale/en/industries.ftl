## Industries and named plants (DEC-110; content/base/industries.jsonc).
##
## A plant's name comes in parts. The engine sends `plant_key` (one of the
## `plant-name-*` forms below), `stem` (the town, a family name, a word or a
## district) and `works_key` (the works' noun); the client composes $plant.
## Rules for a line that takes them (STYLE_BIBLE §5):
## - $plant is an object, or the subject of a past or modal verb. It never
##   starts a sentence ("the Garrowby cannery" has a lower-case "the").
## - $product is a mass noun after "for" or "of".
## - $industry is a name ("Food processing"); $upstream is a noun phrase as
##   residents say it ("the boats"), never a subject.
## - A closing date is $month and $year.

# An industry's name, and the noun phrase residents use for it.
industry-food_processing = Food processing
talk-industry-food_processing = the food factories
industry-metals = Metals
talk-industry-metals = the steel and the foundries
industry-machinery = Machinery
talk-industry-machinery = the engineering works
industry-textiles = Textiles
talk-industry-textiles = the mills
industry-timber_paper = Timber and paper
talk-industry-timber_paper = the sawmills
industry-chemicals = Chemicals
talk-industry-chemicals = the chemical works
industry-vehicles = Vehicles
talk-industry-vehicles = the motor works
industry-electronics = Electrical goods
talk-industry-electronics = the electrical works
industry-shipbuilding = Shipbuilding
talk-industry-shipbuilding = the yards
industry-mining = Mining
talk-industry-mining = the pits
industry-quarrying = Quarrying
talk-industry-quarrying = the quarries
industry-oil_gas = Oil and gas
talk-industry-oil_gas = the oil
industry-forestry = Forestry
talk-industry-forestry = the forest
industry-fishing = Fishing
talk-industry-fishing = the boats
industry-arable = Arable farming
talk-industry-arable = the arable farms
industry-livestock = Livestock
talk-industry-livestock = the stock farms

# A works' noun (lower case: it follows a name), and what it makes.
works-cannery = cannery
product-canned_fish = canned fish
works-creamery = creamery
product-dairy = butter and cheese
works-flour_mill = flour mill
product-flour = flour
works-steelworks = steelworks
product-steel = steel
works-foundry = foundry
product-castings = castings
works-smelter = smelter
product-metal = smelted metal
works-engine_works = engine works
product-engines = engines
works-tool_works = tool works
product-machine_tools = machine tools
works-loom_works = loom works
product-looms = looms
works-woollen_mill = woollen mill
product-woollens = woollen cloth
works-hosiery_works = hosiery works
product-hosiery = hosiery
works-carpet_works = carpet works
product-carpets = carpet
works-sawmill = sawmill
product-sawn_timber = sawn timber
works-paper_mill = paper mill
product-paper = paper
works-furniture_works = furniture works
product-furniture = furniture
works-chemical_works = chemical works
product-chemicals = industrial chemicals
works-fertiliser_works = fertiliser works
product-fertiliser = fertiliser
works-motor_works = motor works
product-motor_vehicles = motor vehicles
works-wagon_works = wagon works
product-rail_wagons = rolling stock
works-appliance_works = appliance works
product-appliances = household appliances
works-instrument_works = instrument works
product-instruments = instruments
works-shipyard = shipyard
product-ships = shipping
works-boatyard = boatyard
product-boats = fishing boats
works-colliery = colliery
product-coal = coal
works-ore_mine = ore mine
product-ore = ore
works-quarry = quarry
product-stone = stone
works-brickworks = brickworks
product-bricks = brick
works-oil_field = oil field
product-oil = oil
works-gas_terminal = gas terminal
product-gas = gas
works-timber_yard = timber yard
product-timber = timber
works-fish_quay = fish quay
product-fish = fish
works-grain_coop = grain co-operative
product-grain = grain
works-estate = estate
product-crops = root crops
works-auction_mart = auction mart
product-livestock = livestock
works-dairy_farm = dairy farm
product-milk = milk

# Markets and goods (the seam for foreign trade).
market-world = The rest of the world
good-farm_produce = Farm produce
good-fish = Fish
good-food = Food
good-minerals = Coal and ores
good-stone = Stone and brick
good-fuels = Oil and gas
good-timber = Timber and paper
good-metals = Metals
good-machinery = Machinery
good-vehicles = Vehicles
good-electronics = Electrical goods
good-textiles = Textiles
good-chemicals = Chemicals
good-ships = Ships

# A plant's name by form ($stem, $works), and how it is named once it has shut.
plant-name-place = the { $stem } { $works }
plant-name-family = { $stem }’s { $works }
plant-name-word = the { $stem } { $works }
plant-name-hood = the { $stem } { $works }
plant-old = the old { $works }
plant-small-firms = Smaller firms

# Labels on a place's sheet and in the town summary.
plant-label-industry = Industry
plant-label-makes = Makes
plant-label-status = Status
plant-label-why = Why
plant-label-employers = Main employers

# A plant's status ($month, $year: since when, or the closing date).
plant-status-running = Working full time
plant-status-short_time = On short time since { $month } { $year }
plant-status-threatened = Under notice. It closes in { $month } { $year }.
plant-status-closed = Closed in { $month } { $year }

# The terms of an industry's demand, as labels.
factor-cycle = World trade
factor-world = Orders for the product
factor-price = Selling prices
factor-input = Cost of materials and fuel
factor-stock = What is left to take
factor-water = Water
factor-competitiveness = Foreign competition
factor-labour_cost = Local wage costs
factor-supply = Local supply
factor-home = Demand at home

# Why a plant is on short time or worse: the weakest term of its industry's
# demand. One sentence each; $upstream only in the supply line.
plant-why-cycle = Trade is down everywhere, and the order book at { $plant } went with it.
plant-why-world = Orders for { $product } are down everywhere, not only in { $town }.
plant-why-price = The price of { $product } has fallen below what it costs { $plant } to turn out.
plant-why-input = Materials and fuel cost more than they did, and the price of { $product } has not followed.
plant-why-stock = There is less left to take each year, and it costs more to reach.
plant-why-water = The dry spell has cut what the land around { $town } gives.
plant-why-competitiveness = Foreign works sell { $product } for less than { $plant } can.
plant-why-labour_cost = Wages in { $town } have risen faster than the price of { $product }.
plant-why-supply = Less is coming in from { $upstream }, and { $plant } ran on it.
plant-why-home = People at home are buying less, and { $plant } sold most of its { $product } here.

# The event log and the news of the town.
event-plant-short-time = Short time at { $plant } in { $town }: { $jobs } jobs on reduced hours.
event-plant-threatened = Closure notice at { $plant } in { $town }: { $jobs } jobs to go in { $month } { $year }.
event-plant-reprieved = Closure notice lifted at { $plant } in { $town }. { $jobs } jobs stay.
event-plant-closed = The gates shut at { $plant } in { $town }: { $jobs } jobs lost.
event-plant-opened = A new { $works } opened in { $town } with { $jobs } jobs.
event-plant-reopened = The { $works } in { $town } reopened with { $jobs } jobs.
event-industry-slump = Orders fell across { $industry }: down { $change }%.
event-industry-surge = Orders rose across { $industry }: up { $change }%.

headline-plant-short-time = Short time at { $plant }: { $jobs } on reduced hours
headline-plant-threatened = { $town } to lose { $plant }: { $jobs } jobs to go by { $month }
headline-plant-reprieved = Reprieve for { $plant }: { $jobs } jobs stay in { $town }
headline-plant-closed = Gates shut at { $plant }: { $jobs } jobs gone in { $town }
headline-plant-opened = New { $works } for { $town }: { $jobs } jobs

skip-item-plant-short-time = Short time began at { $plant }.
skip-item-plant-threatened = A closure notice was served at { $plant }: { $jobs } jobs, to go in { $month } { $year }.
skip-item-plant-reprieved = The closure notice at { $plant } was lifted.
skip-item-plant-closed = The gates shut at { $plant }: { $jobs } jobs lost.
skip-item-plant-opened = A { $works } opened with { $jobs } jobs.
