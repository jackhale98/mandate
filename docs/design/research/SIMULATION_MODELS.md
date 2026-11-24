# Simulation models: research review (before M4)

**Purpose:** check the engine's models against established research and comparable games before building elections, parties, and NPC AI (M4). We borrow structure and rough magnitudes, not exact realism. Mandate is a game, and DEC-059 says drama matters more than fidelity. Each finding ends with what we do about it. Adopted changes are recorded in DEC-071.

**Date:** 2026-11-18. The sources are listed at the end. Effect sizes come from abstracts and summaries, so treat them as orders of magnitude.

---

## 1. Vote choice

### Findings
- **Proximity versus directional voting.**
  - *Proximity* models say voters pick the closest candidate.
  - *Directional* models say voters reward candidates who are on their side of an issue and intense about it.
  - The empirical literature has not settled which is better.
  - Merrill and Grofman's unified work finds that pure models are usually rejected in favour of a mix of proximity, direction, and intensity components.
  - Cross-national European data (Fazekas and Méder) favour proximity overall, but its advantage shrinks as party systems polarise. Directional voting fits polarised contexts better.
- **Valence** is the non-policy part of the choice: competence, likability, scandal, and incumbency. It is a standard separate term in the utility.
- **Local elections are low-information.** Voters lean on cues such as name recognition, incumbency, ballot designations, and party (where shown). Without a party label, the incumbency cue becomes stronger.

### Implications for Mandate
DEC-028 already specifies per-cohort utility feeding a logit model. Refinements:
- **Issue term:**
  - It is a blend: `U_issue = −(1−β)·Σ wᵢ (xᵢ − cᵢ)² + β·Σ wᵢ xᵢ cᵢ`, where x are cohort attitudes and c are the candidate's *perceived* positions, both centred on the population mean.
  - The weights `w` are issue salience (the concern weights we already compute).
  - `β` rises with polarisation, measured as the attitude dispersion of the electorate. Polarised eras then reward clear, intense positions, and calm eras reward the centre.
  - This makes the "moderate or firebrand?" choice depend on context, not on a fixed best answer, which matters for the "no dominant bot strategy" checkpoint.
- **The awareness discount is multiplicative**, not additive. An unknown candidate's issue and valence terms shrink toward zero, so name recognition is a precondition for everything else. This matches the low-information findings.
- **Party labels** carry weight only where the constitution prints them on the ballot. The local tier can then be configured as nonpartisan, which turns incumbency and awareness into the main cues: a natural curveball for constitution variants.

## 2. Incumbency and retrospective voting

### Findings
- **Incumbency effects at the municipal level are large.**
  - German mayors: about 38–40 points higher probability of winning the next election (regression discontinuity design, about 25,000 elections). The effect is larger in bigger towns and for full-time mayors.
  - US mayors: the advantage is far larger in on-cycle elections than in off-cycle ones.
  - City councillors are also more likely to run and win after serving.
  - The advantage is mostly personal, not partisan.
- **Retrospective economic voting is local, and it is benchmarked.**
  - US big-city mayors gained support when *city unemployment was low relative to national unemployment*. Crime and property values did not reliably matter.
  - The effect appears mainly where the city dominates its media market.
  - South European regional elections show the same benchmarking against national conditions.

### Implications
- **Do not hard-code an incumbency bonus.** Incumbency should emerge from existing terms:
  - higher awareness from holding office;
  - a valence term built from the record;
  - fewer credible challengers, because NPCs judge their chances from polls.

  We then tune so that the emergent effect lands in a plausible band (incumbents win roughly 65–80% of local races). This becomes a sweep metric and drama check.
- **The retrospective term uses relative local performance:**
  - local unemployment change *minus* national unemployment change, plus local rent growth relative to inflation;
  - weighted more heavily for executives (mayor) than for individual councillors;
  - weighted by media dominance, approximated by the town's share of its region's population.

  Our current "experienced" opinion term uses absolute local unemployment change. It changes to the benchmarked form, so a national recession is not blamed on the mayor.

- **As built (audit fix 1, 2026-11-23; `SIMULATION_AUDIT.md`, "After fix 1"):**
  - *The record* in a candidate's utility is `retrospective_weight × executive × media × ((approval − the nation's approval) + retro_economy_weight × economy)`, carried in full by the incumbent and in part by the governing party's nominee. The old hidden `× 0.1` is gone, so the weight in content is the weight that applies. Approval is judged against the nation's mean approval, not against ½, so a contented country does not hand every incumbent a free bonus.
  - *The economy* term is benchmarked twice: the year's change in local unemployment against the nation's, plus `retro_level_weight` × how far the town's rate now sits above the nation's. A closure is therefore not forgiven within a year.
  - *Approval remembers:* the opinion step's experienced term adds `opinion.experience.unemployment_gap` × (the town's rate − the nation's rate) beside the change term, and each neighbourhood feels both through its own unemployment factor (its income classes and the sectors its workers lost, as the jobs worry reads). A closure costs the government most in the estate by the plant, and keeps costing until the town is back at the nation's rate.
  - *The race's luck* is smaller (`race_shock_sd` 0.12, `national_swing_sd` 0.15), so the record, not the shock, is what election night most often names when an incumbent loses.
  - *The record of visible work* (DEC-076) counts delivery, not presence: canvassing adds nothing to it (`career.record_gain.canvass` 0), so a councillor who only knocks on doors meets the thin-record attack. Approval of the player weighs more (`player_approval_weight` 2.5) and persuasion per canvass is small (0.03), in line with §5. Name recognition can be given diminishing returns (`awareness_known`, `awareness_beyond`; off in the base pack, because every active style is well known by election day and the cap did not separate them).

## 3. Party identification

### Findings
- **Stable or a running tally?**
  - Party ID is either a stable predisposition (traditionalists) or a running tally of retrospective evaluations (Fiorina).
  - Bartels argues that partisanship filters how events are perceived; the tally does not drive it.
  - **Mover–stayer** models fit US, British, and Canadian panels best: a durable core plus a flexible group.

### Implications
- Each cohort holds a **party attachment vector** with two parts:
  - a *stayer share* (durable, changes only with generational replacement and rare realignments);
  - a *mover share* (updates slowly from retrospective evaluations, roughly a 5–10% pull per year).
- **Perception filter:** a cohort's partisans discount bad news about their own party's officeholders, for example a 30–50% reduction in retrospective blame.
- **Results:** stable bases, swing voters who decide elections, and polarisation that makes blame stick less. These are known dynamics, and they are cheap to compute.

## 4. Turnout

### Findings
- **The calculus of voting** (Riker and Ordeshook) is `R = P·B + D − C`. Because P is tiny, the *duty* term D and the cost term C dominate.
- **Resources** (education, income, age) explain the class and age gaps in turnout.
- **Habit:** past voting predicts future voting.
- **Aggregate meta-analyses** (Geys; Cancela and Geys; Stockemer):
  - turnout is consistently raised by compulsory voting, election importance, and smaller population size;
  - closeness and electoral system show no clear effect.
- **Timing:** on-cycle (concurrent) elections have much higher turnout than off-cycle ones (de Benedictis-Kessner).

### Implications
- Cohort turnout propensity = **duty** (age and trust) + **resources** (class) + **habit** (last turnout, decaying) + **election importance** (tier, and whether it is concurrent) − **cost** (weather and hazard events, distance) + **mobilisation** (canvassing, organisation).
- Closeness gets only a small weight.
- The constitution data gains an "election timing" field (concurrent or standalone), which changes local turnout and therefore incumbency.

## 5. Campaign effects

### Findings
- Kalla and Broockman's meta-analysis of 49 field experiments finds that the best estimate of campaign contact's **persuasive** effect in US *general* elections is **zero**.
- Effects do appear:
  - in primaries and ballot measures;
  - when a candidate takes unusually unpopular positions and campaigns target persuadable voters;
  - from early contact measured immediately, which then **decays**.
- Mobilisation (turnout) effects of canvassing are better established than persuasion effects.

### Implications (a design guard, not just realism)
- **Canvassing and town halls mainly raise awareness and turnout.** Persuasion from contact:
  - has diminishing returns per neighbourhood;
  - decays over months (a half-life of about 2 months);
  - is larger in nonpartisan races and primaries than in partisan general elections.
- Lasting opinion change comes from **record** (experienced effects of laws), **valence events** (scandals, crises handled), and **media**.
- **Result:** "spam canvass" cannot be a dominant strategy. Governing well and building a record matters, which is the game's thesis.
- **M3 follow-up:** canvassing currently adds a flat +0.06 approval each time. It becomes decaying, diminishing persuasion in M4.

## 6. Polling error

### Findings
- Jennings and Wlezien (more than 30,000 polls, 351 elections, 45 countries, 1942–2017) find that poll error has **not** grown over time.
- Error is much larger 150–200 days before an election than near it.
- A large share of error variance sits at the **election** level (shared across all polls), not just per poll.

### Implications (refines DEC-029)
- Poll = truth + **election-wide shared bias** (one seeded draw per election, shrinking as the election nears) + **house effect** (per pollster, persistent) + **sampling noise** (from sample size).
- A typical mean absolute error is about 2 points on election eve and about twice that months out.
- The shared bias produces "all the polls were wrong" nights. This is organic drama and fair, because it is visible in hindsight.

## 7. Legislative bargaining

### Findings
- Groseclose and Snyder ("Buying Supermajorities") show that when rival vote-buyers compete, **minimal winning coalitions should be rare**, and supermajorities are often cheaper and safer. Banks corrected a detail; the core result stands.

### Implications
- NPC coalition-builders (whips, mayors) target a **margin above the threshold** (about one or two votes of insurance) when the opposition can still counter-lobby.
- The player's whip view already shows expected yes against needed. NPC opponents now counter-lobby, so "exactly enough" becomes risky. This creates tension in close votes.

## 8. NPC decision-making

### Findings
- **Utility AI** (Dave Mark and Kevin Dill; the Infinite Axis Utility System):
  - each consideration is normalised to 0–1 through a **response curve** (linear, logistic, or quadratic, with explicit inflection points);
  - an action's score is the product of its considerations;
  - selection is either the highest score or a **weighted random choice among the top n**.
  - It is data-driven, debuggable, and widely used.
- **Crusader Kings III:**
  - personality traits drive AI choices;
  - opinion between characters gates interactions;
  - acting *against* one's traits causes stress, so characters mostly act in character but will break character for a valuable enough prize.

### Implications (DEC-030 implementation)
- NPC actions are scored with **considerations plus response curves defined in content** (JSONC), each with a debug trace. This makes the "Why did they do that?" inspector straightforward.
- **Selection:** weighted random among the top 3, using the NPC's keyed RNG stream, so it is deterministic and not perfectly predictable.
- **Personality fit:** actions carry trait affinities. Acting against one's personality multiplies the score by a "reluctance" factor, and doing it anyway costs the NPC Standing or Health. This mirrors CK3 stress and uses our existing tracks.
- **Symmetry is kept:** NPCs use the same action set, slots, and odds as the player.

## 9. Migration

### Findings
- **Gravity models** fit internal migration very well (adjusted R² of about 0.8–0.9). The distance-decay exponent γ is typically between 0.5 and 2 (about 0.8–0.9 between urban areas in one study) and depends on how the geography is divided up.
- The **radiation model** (Simini et al.) uses intervening opportunities but underestimates long-distance moves in internal migration.
- **Victoria 3** splits migration into economic migration (towards higher standard of living and available jobs) and turmoil-driven mass migration.

### Implications
- Our M2 migration pools movers nationally and has no distance term, so a move across the country is as likely as one to the next town. This is the biggest realism gap in M2.
- **Fix (M4, small):** keep origin pools per municipality and weight destinations by `attractiveness × housing room × size^½ × (1 + d/d₀)^−γ`, with γ ≈ 1.2 and d₀ about a quarter of the map width, using distances between town centres.
- **Result:** regional labour markets, and a stronger regional identity when a town declines and its people move to the regional city.
- Our displacement after disasters already corresponds to Victoria 3's turmoil-driven channel.

## 10. Demography

### Findings
- Professional projections use the **cohort-component method** (a Leslie matrix: age-specific fertility, survival, and migration).
- Modern mortality work uses Lee–Carter or Kannisto models rather than Gompertz curves.
- UN probabilistic projections drive age-specific rates from two summary parameters: **total fertility rate** and **life expectancy**.

### Implications
- Our 12 cohorts (age bands × class) are a coarse cohort-component model. That is fine at our scale. No structural change.
- **Refinement (low priority, M7 balance pass):** express content parameters as total fertility rate and life expectancy per class, and derive the per-band rates from them. Era trajectories (ageing societies, baby booms) then become two dials.

## 11. Map generation

- **Our pipeline** already follows established practice:
  - priority-flood hydrology (Barnes et al.);
  - Whittaker biomes;
  - rank-size settlements;
  - cost-distance territories.
- **Azgaar's generator:** biome-dependent population potential and growth-rate-limited expansion of states and cultures. It is comparable to our settlement scoring and region growing.
- **No change needed.**

## 12. Comparable games

| Game | Takeaway | Use in Mandate |
|---|---|---|
| Democracy 4 | Voter groups with overlapping membership; policies feed a visible effects network, so cause and effect are inspectable | Our "Why?" panels and ledgers serve the same purpose. Keep explanations first-class. |
| Victoria 3 | Pops with standard of living drive migration, politics, and political strength; strength scales with wealth and the franchise | Political strength (lobbying power) should scale with class and organisation. Use it for interest groups in M5. |
| Crusader Kings III | Personality traits, opinion, and stress for acting out of character | NPC utility AI with reluctance and stress (§8) |

---

## Summary of adopted changes (DEC-071)

| # | Change | Milestone |
|---|---|---|
| 1 | Issue utility blends proximity and directional voting, with β rising with polarisation | M4 |
| 2 | Awareness multiplies issue and valence terms; party labels only where the constitution prints them | M4 |
| 3 | Incumbency emerges from awareness, record, and challenger entry, tuned to a 65–80% re-election band (sweep metric) | M4 |
| 4 | Retrospective judgment is benchmarked against national conditions, executive-weighted, and media-weighted | M4 |
| 5 | Party attachment as mover–stayer, with a partisan perception filter | M4 |
| 6 | Turnout from duty, resources, habit, importance, cost, and mobilisation; an election-timing field in constitutions | M4 |
| 7 | Campaign contact: mainly awareness and turnout; persuasion decays and has diminishing returns | M4 |
| 8 | Polls: shared election bias, house effects, and sampling noise, with error shrinking toward election day | M4 |
| 9 | NPC whips seek a margin above the threshold and counter-lobby | M4 |
| 10 | NPC utility AI: content-defined response curves, weighted random among the top 3, personality reluctance | M4 |
| 11 | Gravity-model migration with distance decay | M4 (small M2 fix) |
| 12 | Demography parameterised by fertility rate and life expectancy | M7 (balance pass) |

## Sources
- Merrill and Grofman, *A Unified Theory of Voting*, empirical comparison chapter: https://www.cambridge.org/core/books/unified-theory-of-voting/comparing-the-empirical-fit-of-the-directional-and-proximity-models-for-voter-utility-functions/9C6549C2A35CE1B6404D24D3C031E2E4
- Proximity and directional theory compared (Maastricht): https://cris.maastrichtuniversity.nl/en/publications/proximity-and-directional-theory-compared-taking-discriminant-pos/
- Adams, Bishin and Dow, directional voting in US Senate elections: https://coolheights.ucr.edu:443/publications/Adams.Bishin.Dow.Journal.of.Politics.pdf
- Lewis and King, "No Evidence on Directional vs. Proximity Voting": https://gking.harvard.edu/files/spatial.pdf
- Hopkins and Pettingill, "Retrospective Voting in Big-City US Mayoral Elections": https://www.cambridge.org/core/journals/political-science-research-and-methods/article/retrospective-voting-in-bigcity-us-mayoral-elections/4F5A92CBF7ED555B3CE56E0E7B7BECAF
- Freier, incumbency in German mayoral elections (DIW): https://www.diw.de/documents/publikationen/73/diw_01.c.377721.de/dp1147.pdf
- de Benedictis-Kessner, "Off-Cycle and Out of Office": https://www.hks.harvard.edu/publications/cycle-and-out-office-election-timing-and-incumbency-advantage
- Gerber and Hopkins, parties in US cities (NBER): https://www.nber.org/papers/w13535.pdf
- Incumbency and competitiveness in city council elections: https://www.cambridge.org/core/services/aop-cambridge-core/content/view/6F564D1531EABB172025806FB9189CE5/S0008423920000384a.pdf/incumbency-and-competitiveness-in-city-council-elections-how-accurate-are-voter-perceptions.pdf
- Ballot cues in low-information elections: https://stars.library.ucf.edu/facultybib2000/6411
- Bartels, "Beyond the Running Tally": https://calgara.github.io/PolS5310_Spring2021/Bartels%20-%202002%20-%20Beyond%20the%20Running%20Tally.pdf
- Dynamic panel models of partisanship (Electoral Studies 2011): https://polisci.osu.edu/sites/default/files/2020-02/Electoral%20Studies%202011.pdf
- Turnout meta-analysis (Cancela and Geys): https://biopen.bi.no/bi-xmlui/handle/11250/2389104
- Stockemer, aggregate turnout review: https://www.cambridge.org/core/services/aop-cambridge-core/content/view/2CCC1F9A8B742953B2D10C87C13D9F12/S0017257X16000300a.pdf/what_affects_voter_turnout_a_review_articlemetaanalysis_of_aggregate_research.pdf
- Turnout lecture notes (Oxford): https://users.ox.ac.uk/~nuff0084/polsoc/TurnoutLecture.pdf
- Kalla and Broockman, "The Minimal Persuasive Effects of Campaign Contact": https://papers.ssrn.com/abstract=3042867
- Jennings and Wlezien, "Election polling errors across time and space": https://eprints.soton.ac.uk/421260/1/JenningsWlezien_PollingErrors.pdf
- Groseclose and Snyder, "Buying Supermajorities": https://ideas.repec.org/a/cup/apsrev/v90y1996i02p303-315_20.html
- Utility systems overview: https://en.wikipedia.org/wiki/Utility_system
- Mark and Dill, "Improving AI Decision Modeling Through Utility Theory" (slides): https://api.sambuz.com/doc/improving-improving-ai-decision-modeling-ai-decision-ppt-presentation-788939
- CK3 Dev Diary #31, stress: https://admin-forum.paradoxplaza.com/forum/developer-diary/ck3-dev-diary-31-a-stressful-situation.1399764/page-2
- Poot et al., gravity model of migration review: https://www.redalyc.org/journal/289/28966571004/html/
- Simini et al., radiation model: https://alphaxiv.org/abs/1111.0586
- Kluge, Levermann and Schewe, radiation model and internal migration: https://publications.pik-potsdam.de/pubman/item/item_27793_3/component/file_27803/27793oa.pdf
- Victoria 3 Dev Diary #17, migration: https://admin-forum.paradoxplaza.com/forum/developer-diary/victoria-3-dev-diary-17-migration.1492928
- Victoria 3 Dev Diary #13, standard of living: https://admin-forum.paradoxplaza.com/forum/developer-diary/victoria-3-dev-diary-13-standard-of-living.1489327/page-12
- Democracy (video game): https://en.wikipedia.org/wiki/Democracy_(video_game)
- Age-specific rates for probabilistic projections: https://ar5iv.arxiv.org/html/1503.05215
- US Census Bureau, cohort-component projection method: https://webarchive.library.unt.edu/web/20130228080229/http:/www.census.gov/population/international/data/idb/estandproj.pdf
- Azgaar's Fantasy Map Generator: https://github.com/Azgaar/Fantasy-Map-Generator/wiki/Quick-Start-Tutorial
