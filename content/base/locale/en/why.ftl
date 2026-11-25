## "Why?" panels (the causal spine, DEC-108; CAUSAL_SPINE §6 and §9). The
## engine answers with typed values and these lines word them; a step's own
## sentence is the event's line (`event-…`). Every variant of the engine's
## types needs a line here: `mandate-wasm/tests/why.rs` fails when one has
## none. No line names an office, a body, a market or an industry (DEC-098):
## a name comes from an argument or from the step's own sentence.

## How a step joins the later one above it. The panel reads newest first, so
## each phrase leads from the later event down to the earlier one.
why-link-caused = because of
why-link-continues = after
why-link-prompted = prompted by
why-link-revealed = which brought to light
why-link-ended = which ended

## The player's own act (`PlayerAct`): under the step that names it, and in
## the line at the top of the panel.
why-player-start = You began your career.
why-player-vote = You declared your vote on a bill.
why-player-file = You filed your plans for the election.
why-player-draft = You drafted a bill.
why-player-amend = You amended a bill.
why-player-introduce = You introduced a bill.
why-player-withdraw = You withdrew a bill.
why-player-move = You went somewhere.
why-player-meet = You met someone.
why-player-answer = You answered a decision.
why-player-let_pass = You let a decision pass, so its default happened.
why-player-talk = You talked to someone.
why-player-respond_ask = You answered what someone asked of you.
why-player-promise = You made a promise.
why-player-lobby = You lobbied someone.
why-player-trade_favor = You traded a favour.
why-player-town_hall = You held a town hall.
why-player-press = You talked to the press.
why-player-canvass = You canvassed a neighbourhood.
why-player-fundraise = You raised money.
why-player-rest = You rested.
why-player-advertise = You bought advertising.
why-player-take_bribe = You took a bribe.
why-player-dip_funds = You paid your own bills from campaign funds.
why-player-leak_file = You leaked a file.
why-player-visit = You paid a visit.
why-player-casework = You took up someone's problem.
why-player-exchange = You took the floor.

## Someone else's own choice (`$name` as the player knows them).
why-actor = { $name }'s own choice.
why-actor-unnamed = Someone's own choice. You have not heard who.

## A scheduled process applying its standing rule (`Process`).
why-process-calendar = The calendar: time moved on.
why-process-markets = The economy's monthly round of demand, jobs and wages.
why-process-central_bank = The central bank's rule for setting interest rates.
why-process-budget = The budget year coming round.
why-process-lawmaking = The rules for making law: debate, votes, signing and expiry.
why-process-agenda = The agenda: proposals come up, and drop off, as conditions call for them.
why-process-elections = The election calendar: campaigns, polls and election day.
why-process-parties = A party's own rules for whom it puts forward.
why-process-director = The storyteller, which chooses what is put in front of you and when.
why-process-press = The press at work: reporting, digging and correcting.
why-process-justice = Prosecutors and courts following their procedure.
why-process-asks = What people ask of you, and the deadlines that come with it.
why-process-society = Residents making up their own minds.
why-process-knowledge = What being there, or holding a post, lets you learn.
why-process-arcs = A matter in town running its own course.
why-process-update = The game itself was updated.

## A standing condition the engine's own formula names (`Pressure`). The
## first five are the weakest factor of a sector's demand when a plant
## closes (always below its normal level then); the step's sentence names
## the sector.
why-pressure-world_demand = What weighed most on the sector: weak world demand.
why-pressure-commodity_prices = What weighed most on the sector: low commodity prices.
why-pressure-depletion = What weighed most on the sector: the resource it lives on is running down.
why-pressure-competitiveness = What weighed most on the sector: the country's industry has lost ground to its competitors.
why-pressure-labour_costs = What weighed most on the sector: high local labour costs.
why-pressure-track = One of your pressure tracks had reached danger.
# An industry's demand (DEC-110): the weakest term when a plant goes on short
# time, gets its notice or closes; the last two are what opens a works.
why-pressure-external_demand = What weighed most on the industry: orders from abroad for what it makes had fallen.
why-pressure-home_demand = What weighed most on the industry: people at home were buying less of what it makes.
why-pressure-input_costs = What weighed most on the industry: its materials and fuel had grown dear.
why-pressure-supply = What weighed most on the industry: less was coming in from the industries nearby that it runs on.
why-pressure-water = What weighed most on the industry: the region was short of water.
why-pressure-slack_labour = People in the town were out of work, and idle hands draw a new works.
why-pressure-boom = Demand for what the works makes had run high for two years.

## Uncaused inside the game by design, and saying so (`Shock`).
why-exogenous-world_economy = The world economy: nothing here caused it.
why-exogenous-commodities = World commodity markets: nothing here caused it.
why-exogenous-hazard = Weather and the ground: nobody caused it.
why-exogenous-race = The part of every race that nobody controls.

## Where a chain starts when nothing above applies.
why-history = How the world already was when your story began.
why-forgotten = Something earlier that the record no longer keeps.
why-unknown = The record does not say: this was written down before causes were kept.

## Interface: the control and the panel.
ui-why-ask = Why?
ui-why-led = What led to this
ui-why-about = { $label } ({ $what })
ui-why-title = Why this happened
ui-why-loading = Looking back through the record…
ui-why-gone = The record no longer keeps how this came about.
ui-why-player = Something you did is behind this.
ui-why-chain = What led to it, newest first
ui-why-branch = And “{ $step }”
ui-why-more = More lies further back.
ui-why-further = Go further back
ui-why-name-unknown = someone you have not heard of
