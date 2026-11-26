# Policy families (DEC-092; content/base/policy_families/*.jsonc). Slot names
# are arguments: a place, a neighbourhood or a person by name, and a sector,
# crisis or hazard as `<slot>_key`, a Fluent key of its own.

## Shared by many families

hazard-text-flood = flooding
hazard-text-storm = storm surges
hazard-text-wildfire = wildfire
axis-paid-by = Paid for by
opt-pay-budget = This year's budget
opt-pay-budget-desc = No new tax or debt, but other programmes get less.
opt-pay-bond = A bond
opt-pay-bond-desc = Borrow now and repay over the years: nothing to pay today, interest for years.
opt-pay-levy = A local levy
opt-pay-levy-desc = A small rise in the property tax pays for it. Homeowners notice.
axis-where = Where
axis-who-builds = Who builds it
opt-build-works-team = The council's works team
opt-build-works-team-desc = No contractor's margin, at the pace of the council's own staff.
opt-build-contractor = A contractor
opt-build-contractor-desc = Months faster and dearer. Local workers who lose out complain.
axis-run-by = Who runs it
opt-run-council = The council
opt-run-council-desc = Run by council staff, at the council's cost.
opt-run-charity = A local charity, on a grant
opt-run-charity-desc = Cheaper for the council. The charity does what its volunteers can.
sponsor-union-version = The union's version
sponsor-owner-version = The owner's version
sponsor-business-version = The business association's version
sponsor-residents-version = The residents' version
sponsor-builders-version = The builders' version
sponsor-neighbours-version = The neighbours' version
reason-plant-closed = { $plant } has closed

## Rescue deal for a plant

family-plant-rescue = Rescue deal for { $plant }
family-plant-rescue-desc = Help { $plant } keep its jobs in town with a wage subsidy, on terms the council sets.
param-wage-aid = Wage aid
secondary-rescue-precedent = Other firms will ask for the same.
axis-rescue-terms = Terms
opt-rescue-no-strings = No strings
opt-rescue-no-strings-desc = The money arrives quickly and does the most. Workers ask why the owners get a cheque for nothing.
opt-rescue-guarantee = A job guarantee
opt-rescue-guarantee-desc = The firm must keep its jobs for five years. More jobs stay, but the talks take two months longer and the owners grumble.
opt-rescue-clawback = Repay if they leave
opt-rescue-clawback-desc = The money comes back if the plant closes anyway. Cheaper for taxpayers, but the firm is less keen.
favour-rescue = the rescue deal
reason-plant-at-risk = while { $plant } is losing jobs
lead-plant-rescue = { $union } would know what a rescue for { $plant } would take.
lead-plant-rescue-owner = { $owner } has been asking around about help for { $plant }.

## After a closure: a fresh start

family-closure-response = After { $plant }: a fresh start
family-closure-response-desc = Stipends while the people from { $plant } retrain, and help for the firms that take them on.
reason-plant-closed-jobs = since { $plant } closed and the jobs went with it
axis-closure-support = What it pays for
opt-closure-retrain = Retraining stipends
opt-closure-retrain-desc = Money while people learn a new trade. Cheaper, and slower to put people back in work.
opt-closure-rehire = Stipends and rehiring subsidies
opt-closure-rehire-desc = Firms that hire from { $plant } get part of the wages paid. More jobs sooner; dearer, and shopkeepers ask why not them.
favour-fresh-start = the fresh start after the closure
lead-closure-response = { $union } is gathering the people laid off from { $plant }.
lead-closure-response-hall = The town hall has the list of who was laid off from { $plant }.

## Living wage deal

family-living-wage = Living wage deal with { $union }
family-living-wage-desc = A wage floor for the town, agreed with { $union } and the employers: how high, who it covers, and how fast.
reason-living-wage = while wages do not stretch to the rent
axis-living-wage-covered = Who it covers
opt-wage-everyone = Every employer in town
opt-wage-everyone-desc = Everyone on low pay gets the rise. The employers who pay it complain loudest.
opt-wage-council = The council and its contractors
opt-wage-council-desc = Only council work pays the floor, and the council pays the difference. Fewer people get the rise.
axis-living-wage-phase = How fast
opt-wage-at-once = All at once
opt-wage-at-once-desc = The new rate starts with the law. Small firms feel it at once.
opt-wage-phased = Phased over a year
opt-wage-phased-desc = The rate rises in steps over twelve months. Firms adjust; workers wait.
favour-living-wage = the living wage deal
lead-living-wage = { $union } has a figure in mind for a living wage.

## Incubator at a campus

family-incubator = Incubator at { $campus }
family-incubator-desc = Help start-ups grow at { $campus } with a wage subsidy for new firms in { $sector_key }.
reason-incubator = while young people leave to find work
axis-incubator-terms = Terms
opt-incubator-grants = Grants
opt-incubator-grants-desc = Firms get the subsidy outright. The most firms come.
opt-incubator-stake = A council stake
opt-incubator-stake-desc = The council takes a share in the firms it helps. Some money comes back; fewer firms want the deal.
opt-incubator-local = Hire local
opt-incubator-local-desc = Firms must hire people from town. Three months longer to agree, fewer firms, and more local jobs among them.
sponsor-campus-version = The campus's version
favour-incubator = the incubator
lead-incubator = { $head } at { $campus } wants room for new firms.
lead-incubator-campus = They talk about start-ups at { $campus }.

## Safety at the mine

family-mine-safety = Safety at { $mine }
family-mine-safety-desc = Inspections underground at { $mine }, and who carries them out.
reason-mine-safety = while { $mine } is working
reason-mine-closed = { $mine } has closed
param-inspections = Inspections
secondary-mine-output = Inspections slow the work at the face.
axis-mine-inspector = Who inspects
opt-mine-council = Council inspectors
opt-mine-council-desc = Inspectors the council pays. Thorough, and the owners resent them.
opt-mine-company = The company, reporting to the council
opt-mine-company-desc = The company checks itself and sends the figures in. Cheap, and the miners doubt it.
axis-mine-scope = What it covers
opt-mine-inspections = Inspections only
opt-mine-inspections-desc = Checks at the face and on the shafts.
opt-mine-training = Inspections and training
opt-mine-training-desc = The council pays for safety training too. Dearer and slower to start; more miners stay in work.
favour-mine-safety = the safety rules at the mine
lead-mine-safety = { $union } has a list of what goes wrong underground at { $mine }.
lead-mine-safety-manager = { $manager } runs { $mine } and knows what an inspection would cost.

## Port expansion

family-port-expansion = Port expansion at { $port }
family-port-expansion-desc = New berths at { $port }: more trade and work through { $hood }, and more lorries.
reason-port-expansion = while { $port } could take more ships
param-berths = New berths
unit-berths = berths
secondary-port-lorries = Lorries to and from { $port } fill the streets of { $hood }.
opt-port-dues = Harbour dues
opt-port-dues-desc = Ships pay dues on the new berths. Money back for the council; a little less trade.
sponsor-harbour-version = The harbourmaster's version
favour-port = the port expansion
lead-port-expansion = { $harbourmaster } at { $port } has plans for more berths.

## Hotel bed tax

family-bed-tax = Hotel bed tax at { $hotel }
family-bed-tax-desc = A charge per night on rooms at { $hotel } and the town's other hotels, and what the money pays for.
reason-bed-tax = while the town is short of money
param-bed-tax = Charge per night
unit-currency-per-night = per night
secondary-bed-tax-visitors = Some visitors stay one town over.
axis-bed-tax-spend = What it pays for
opt-bed-tax-general = The general budget
opt-bed-tax-general-desc = All of it goes to the council's budget. The most money; nothing visitors can see.
opt-bed-tax-streets = Streets and bins
opt-bed-tax-streets-desc = The streets visitors walk get cleaned and mended. It costs some of the money.
opt-bed-tax-promotion = Promoting the town
opt-bed-tax-promotion-desc = The money brings more visitors in time. Slow to show, and the hotels like it.
sponsor-hotel-version = The hotel's version
favour-bed-tax = the bed tax
lead-bed-tax = { $manager } at { $hotel } has heard talk of a bed tax.

## Flood wall

family-flood-wall = Flood wall for { $hood }
family-flood-wall-desc = Build a wall and drains to protect { $hood } against { $hazard_key }.
param-wall-height = Wall height
secondary-wall-river-view = The wall cuts { $hood } off from the water.
secondary-wall-long-works = Years of works along the whole waterfront.
opt-wall-hood = { $hood } only
opt-wall-hood-desc = Cheaper and quicker. The streets next door stay exposed.
opt-wall-riverfront = The whole waterfront
opt-wall-riverfront-desc = Protects { $hood } and the neighbourhoods beside it. It costs more and takes six months longer.
opt-pay-developer-levy = A levy on developers
opt-pay-developer-levy-desc = Builders by the water pay towards it. Fewer new homes get built.
opt-build-landowners = The landowners, under a council order
opt-build-landowners-desc = Owners by the water build their own banks: cheap for the council, patchier protection, and angry owners.
favour-contract = the flood wall contract
reason-flood-risk = while { $hood } is at risk of { $hazard_key }
lead-flood-wall = People in the streets of { $hood } still talk about the last flood.

## Rebuild after a disaster

family-rebuild = Rebuild { $hood }
family-rebuild-desc = Rebuild the homes { $hood } lost, to the old standard or a better one.
reason-rebuild = since the disaster hit { $hood }
axis-rebuild-standard = Standard
opt-rebuild-like = Like for like
opt-rebuild-like-desc = The homes go back as they were. Quicker and cheaper; just as exposed next time.
opt-rebuild-higher = A higher standard
opt-rebuild-higher-desc = Raised floors and stronger roofs against { $hazard_key }. Dearer, and three months longer.
opt-rebuild-hood = { $hood } first
opt-rebuild-hood-desc = The money goes where the damage is worst. It starts sooner.
opt-rebuild-town = Anywhere in town
opt-rebuild-town-desc = Homes go up wherever there is land. Families from { $hood } may have to move.
favour-rebuild = the rebuilding contracts
lead-rebuild = People in the streets of { $hood } are still counting what the disaster took.

## Water for the farms

family-farm-water = Water for the farms
family-farm-water-desc = Help the farms around { $farm } through dry years, and choose how the water is shared.
reason-farm-water = while the farms around { $farm } are short of water
param-water-aid = Help for the farms
secondary-farm-water-town = Town households pay for water the farms use.
axis-farm-water-source = Where the water comes from
opt-water-meters = Meters and fair shares
opt-water-meters-desc = Every farm gets a metered share. Cheap; the old families say the land was always theirs to water.
opt-water-reservoir = A new reservoir
opt-water-reservoir-desc = A reservoir by { $farm }: a year longer to build, more water, and a store against fire.
sponsor-farmers-version = The farmers' version
favour-farm-water = the water deal for the farms
lead-farm-water = { $farmer } at { $farm } can show you how dry the fields are.

## Licences for the market

family-market-licences = Licences for { $market }
family-market-licences-desc = Decide how many stalls { $market } may have, when it opens, what traders pay, and who keeps order.
param-stalls = Change in licensed stalls
unit-stalls = stalls
secondary-market-vans = Busier market days fill the side streets with vans.
axis-market-hours = Opening hours
opt-market-daytime = Daytime only
opt-market-daytime-desc = The market keeps its hours. Nothing changes for the neighbours.
opt-market-evening = An evening market
opt-market-evening-desc = More trade and visitors, and noise late into the evening for { $hood }.
axis-market-fee = Licence fee
opt-fee-low = A low fee
opt-fee-low-desc = More traders take a stall. The council collects little.
opt-fee-high = A high fee
opt-fee-high-desc = The fees pay for the market's upkeep. Fewer traders can afford a stall.
axis-market-order = Who keeps order
opt-order-inspectors = Council inspectors
opt-order-inspectors-desc = Rules are kept evenly, at a cost to the council.
opt-order-traders = The traders themselves
opt-order-traders-desc = Cheap, and the traders like it. Neighbours doubt they will police themselves.
sponsor-traders-version = The traders' version
favour-market = the market licences
reason-market-dispute = while jobs are scarce and { $market } wants to grow
lead-market-licences = { $traders } at { $market } has views on the stall licences.

## A community centre

family-community-centre = A community centre for { $hood }
family-community-centre-desc = A hall for { $hood }: classes, advice and a warm room, and who runs it.
reason-community-centre = while { $hood } has nowhere to meet
param-centre-size = Size
secondary-centre-upkeep = The building needs heating and a caretaker every year.
lead-community-centre = { $organiser } has been asking for a hall in { $hood } for years.
lead-community-centre-streets = People in { $hood } say there is nowhere to go of an evening.

## A park

family-park = A park for { $hood }
family-park-desc = Green space for { $hood }, and where the land comes from.
reason-park = while { $hood } has no green space
param-park-size = Size
secondary-park-plots = Some want the land for homes instead.
axis-park-land = The land
opt-park-lot = A vacant lot
opt-park-lot-desc = The council already owns it. Cheap and small.
opt-park-buyout = Buy out the corner
opt-park-buyout-desc = The council buys homes and clears a bigger space. Dearer, slower, and the owners are not happy.
opt-park-developer = A developer, beside new flats
opt-park-developer-desc = A builder lays out the park next to new flats. More homes, less green.
favour-park = the park
lead-park = People in { $hood } walk a long way to see a tree.

## Night buses

family-night-buses = Night buses for { $hood }
family-night-buses-desc = Buses after dark between { $hood } and the town, for the people on late shifts.
reason-night-buses = while { $hood } is cut off after dark
param-routes = Routes
unit-routes = routes
secondary-night-buses-noise = Older residents complain of buses at two in the morning.
opt-buses-hood = { $hood } only
opt-buses-hood-desc = The routes serve { $hood }. Cheaper, and a month quicker.
opt-buses-and-next = { $hood } and its neighbours
opt-buses-and-next-desc = The routes run through the neighbourhoods beside { $hood } too. Dearer.
opt-buses-council = The council's buses
opt-buses-council-desc = Council drivers on council pay.
opt-buses-contractor = A bus company, on contract
opt-buses-contractor-desc = Cheaper, with fewer buses than promised, and drivers on lower pay.
lead-night-buses = Shift workers in { $hood } walk home in the dark.

## Clinic hours

family-clinic-hours = Clinic hours at { $clinic }
family-clinic-hours-desc = Longer opening at { $clinic }, so people in { $hood } and beyond see a doctor sooner.
reason-clinic-hours = while the wait for an appointment grows
param-clinic-hours = Extra hours
unit-hours-per-week = hours / week
secondary-clinic-staff = Nurses pulled onto late shifts leave other work undone.
axis-clinic-when = When
opt-clinic-evenings = Evenings
opt-clinic-evenings-desc = Open until eight on weekdays.
opt-clinic-weekends = Weekends
opt-clinic-weekends-desc = Saturday and Sunday mornings suit working families. Dearer to staff, and two months to arrange.
axis-clinic-staff = Who staffs it
opt-clinic-council = The clinic's own staff
opt-clinic-council-desc = Overtime for the people already there.
opt-clinic-agency = An agency
opt-clinic-agency-desc = Agency doctors start sooner and cost more. The regular staff resent them.
sponsor-clinic-version = The clinic's version
lead-clinic-hours = { $doctor } at { $clinic } has a plan for longer hours.
lead-clinic-hours-clinic = The waiting room at { $clinic } is full by nine.

## Clean air

family-clean-air = Clean air at { $plant }
family-clean-air-desc = Cut what { $plant } puts into the air over { $hood }, and decide who pays.
reason-clean-air = while { $hood } breathes what { $plant } puts out
param-air-standard = Standard
secondary-clean-air-costs = The owners say the rules cost them work.
axis-clean-air-how = How
opt-air-grant = Council grant for filters
opt-air-grant-desc = The council pays for the filters. The owners agree, and the taxpayers pay.
opt-air-order = A council order
opt-air-order-desc = The firm must pay for it. Six months of argument; the owners and some workers are angry.
opt-air-hood = { $hood }
opt-air-hood-desc = Measured and enforced in { $hood }. Quicker to start.
opt-air-downwind = { $hood } and downwind
opt-air-downwind-desc = The neighbourhoods beside { $hood } are counted too.
favour-clean-air = the clean-air rules
lead-clean-air = { $residents } in { $hood } has been counting the days the washing comes in grey.
lead-clean-air-streets = People in { $hood } blame { $plant } for the air.

## Rent relief

family-rent-relief = Rent relief for { $hood }
family-rent-relief-desc = Help for tenants in { $hood } and beyond while rents bite: payments, a cap on rises, or both.
reason-rent-relief = while rents take more than people can pay
axis-rent-form = What it does
opt-rent-payments = Relief payments
opt-rent-payments-desc = Money for tenants on low incomes. Rents themselves keep rising.
opt-rent-freeze = Payments and a cap on rises
opt-rent-freeze-desc = A cap on how fast rents may rise, too. Older landlords are furious, and fewer homes come to let.
opt-rent-hood = { $hood } only
opt-rent-hood-desc = Where rents bite hardest. Cheaper, and a month quicker.
opt-rent-town = The whole town
opt-rent-town-desc = Every tenant who qualifies, wherever they live.
sponsor-tenants-version = The tenants' version
sponsor-landlords-version = The landlords' version
favour-rent-relief = the rent relief
lead-rent-relief = { $tenants } is organising the tenants of { $hood }.
lead-rent-relief-streets = Tenants in { $hood } talk about little else but the rent.

## Close the gap

family-fiscal-bridge = Close the gap
family-fiscal-bridge-desc = A levy to close the hole in the town's budget, and who is protected from it.
reason-fiscal-bridge = while the town spends more than it takes in
axis-bridge-protect = Who it protects
opt-bridge-poorest = The poorest households
opt-bridge-poorest-desc = A rebate for those on low incomes. It costs some of what the levy raises.
opt-bridge-services = The services
opt-bridge-services-desc = No rebate: the money keeps the services open, which takes a couple of months to show.
axis-bridge-who-pays = Who pays
opt-bridge-flat = Every home, at one rate
opt-bridge-flat-desc = The most money, and every household feels it.
opt-bridge-progressive = The dearest homes pay more
opt-bridge-progressive-desc = A little less money. Owners of big houses notice.
favour-fiscal-bridge = closing the budget gap
lead-fiscal-bridge = The budget papers at the town hall show how big the gap is.

## Overheard: talk of a proposal on the table (overheard.jsonc, `proposal`)

heard-proposal-plant-rescue = "They say the council might pay part of the wages at { $proposal_plant } to keep it going. About time."
heard-proposal-closure-response = "Since { $proposal_plant } shut, there's talk of money while people retrain. My brother would take it."
heard-proposal-living-wage = "The union wants a proper wage floor for the whole town. The bosses won't like that."
heard-proposal-incubator = "Somebody wants the council to back new firms at { $proposal_campus }. Jobs for the young ones, maybe."
heard-proposal-mine-safety = "There's talk of inspectors at { $proposal_mine } again. Somebody has to go down and look."
heard-proposal-port-expansion = "They want more berths at { $proposal_port }. More work, and more lorries through { $proposal_hood }."
heard-proposal-bed-tax = "Somebody wants a charge on every room at { $proposal_hotel }. The visitors would pay, not us."
heard-proposal-flood-wall = "They're talking about a wall along { $proposal_hood }. Before the water comes in, I hope."
heard-proposal-rebuild = "There's talk of money to rebuild { $proposal_hood }. Some of the houses are still boarded up."
heard-proposal-farm-water = "The farms around { $proposal_farm } want water. The question is who pays for it."
heard-proposal-market = "The traders want more stalls at { $proposal_market }. The neighbours want fewer vans."
heard-proposal-community-centre = "There's talk of a hall for { $proposal_hood }. Somewhere for the kids to go of an evening."
heard-proposal-park = "Somebody wants a park in { $proposal_hood }. There's not a tree for half a mile."
heard-proposal-night-buses = "There's talk of night buses out to { $proposal_hood }. My shift ends at midnight."
heard-proposal-clinic-hours = "They might keep { $proposal_clinic } open later. You can't get an appointment before work."
heard-proposal-clean-air = "There's talk of making { $proposal_plant } clean up what it puts in the air. You can taste it in { $proposal_hood }."
heard-proposal-rent-relief = "There's talk of help with the rent for { $proposal_hood }. Not before time."
heard-proposal-fiscal-bridge = "The council's short of money. They're talking about a levy to close the gap."
