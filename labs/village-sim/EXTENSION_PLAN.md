# Willow Crossing extension and test plan

The longer-term target is now defined in [SCALE_AND_EMERGENCE_PLAN.md](SCALE_AND_EMERGENCE_PLAN.md): 1.2 million individual NEAT agents, organic settlements, polities, roads, shipping and sales-derived economic profiles. The jobs/council/market/militia extension has since been implemented; this earlier phased plan remains background for the remaining near-term work.

Extend the existing world in small, testable releases. Preserve finite resources, real travel, authoritative action checks, and the resource ledger. Keep the original simulation as a comparison baseline.

## Current baseline

- The 13 engine tests pass on review.
- Caravans, village founding, gathering, construction, local food/coin trade, hunting, death, and capped renewal work.
- The current world uses utility scores, not a neural network. Its villagers know global resource/monster locations; perception is not yet spatially limited.
- Saved 30-minute runs for seeds 7, 42, and 109 finish with 4–5 survivors after 14 arrivals, with balanced resource ledgers.
- At 14 residents, normal meal demand is approximately 21.5 food/minute. Maximum berry renewal is about 13 food/minute; wildlife adds a limited amount. Starting supplies temporarily hide this deficit. Starvation is not automatically evidence of bad AI.

The next version should explain whether a village failed because food did not exist, food was inaccessible, labor was misallocated, food was hoarded, or villagers chose unreasonable risks.

## Release 1 — Reproducible world and failure diagnosis

**Build**

- Replace the hidden random-number-generator closure with explicit serializable state. Split ecology and actor randomness so unrelated rendering or scheduling changes do not alter creature reproduction.
- Add save/import, scenario configuration, pause/step, and deterministic replay from a snapshot. Keep one fixed simulation step across UI speeds and batch runs.
- Record job start/end times, cancellations, reasons for failure, food location, travel/work/idle time, starvation deaths, combat deaths, and unmet needs.
- Add food supply/demand forecasts and settlement history charts. Separate accessible food, food in transit, wild food, and future biological production.
- Centralize authoritative action validation and inventory transfers. Enforce a real carry capacity instead of the current approximate load score; validate competing claims at execution.

**Tests / completion criteria**

- A saved world continued for 1,000 ticks matches uninterrupted execution, including ecology, jobs, inventories and events.
- The same tick count at 1× and 8× produces the same engine state.
- Harvesting the final unit, simultaneous attacks, interrupted construction, and seller departure never duplicate goods or make inventories negative.
- Dead actors release jobs and claims; abandoned goods remain accounted for.
- Every death has a cause and every failed job has a reason. Conservation holds every tick, not just at run end.

## Release 2 — Logistics and a sustainable food option

**Build**

- Introduce explicit gatherer, hauler, builder and hunter preferences without permanently locking villagers into a profession.
- Add expiring resource/job reservations, bounded commitment, and emergency interruption for starvation, exhaustion and danger. Avoid ten villagers traveling to a bush with one berry.
- Add a granary with finite capacity, a visible food reserve target, and fair access to communal emergency food. Track overflow and spoilage as explicit sinks.
- Add small farm plots: timber/stone construction, finite seed stock, sowing labor, water availability, soil fertility, sunlight, crop growth time, and harvest labor. Water supply must have a capacity and recharge model; crop output must be recorded as biological renewal supported by consumed inputs.
- Villagers must choose between eating reserves, foraging, planting, hauling, building and hunting. Farming must have opportunity costs and can fail during drought.

**Tests / completion criteria**

- Unplanted, immature, unwatered, exhausted-soil or already-harvested plots cannot produce crops.
- Seeds and water are never spent twice. No stock exceeds storage capacity; all spoilage is accounted for.
- Reservations expire on death, timeout or invalid opportunity and do not create permanent deadlocks.
- At least one declared normal scenario can sustain its population for 100 simulated days without imports. This is an existence test, not a requirement that every seed or policy succeeds.
- Drought and overpopulation scenarios can still fail; there is no automatic rescue or unexplained food grant.

## Release 3 — Meaningful markets and shared settlement choices

**Build**

- Give people explicit ownership of private goods and distinguish it from communal contributions. Show how work is rewarded and how emergency aid differs from a purchase.
- Put finite offers at the market for food, timber and stone, with reserve prices and limited budgets. Add jade only when it has a genuine buyer or crafting use.
- Execute trades at the market using reserved offers, with both sides still able to fulfill the exchange. Clear expired orders and return reserved inventory.
- Replace the fixed house → market → houses sequence with settlement proposals based on shortages, bed demand and reserves. Reserve resources before funding and return recoverable materials if construction is cancelled.

**Tests / completion criteria**

- Trades conserve goods and money, cannot oversell, and cannot fill after an offer has expired or been withdrawn.
- A shortage can affect offers/prices without creating money or food. Penniless hungry villagers and communal aid are observable policy cases.
- Construction cannot consume protected emergency reserves without an explicit settlement policy.
- A village without coin can still gather, eat and survive through communal access; markets improve distribution rather than manufacture wealth.

## Release 4 — Spatial awareness, dangerous hunts and social memory

**Build**

- Add walkable terrain, simple pathfinding, sensing radius, remembered resource sightings, and route danger. Unknown locations require exploration.
- Give creatures territories, movement, warning behavior and pursuit/leash rules. Permit group hunts, rescue and carrying the wounded home.
- Estimate encounter cost from health, equipment, allies, escape distance and uncertainty. Keep reckless attacks physically possible; do not present a heuristic risk score as a calibrated death probability.
- Add trust, remembered assistance/betrayal and personal injury histories. Acquired memories affect choices and remain separate from inherited traits.

**Tests / completion criteria**

- Actors cannot walk through blocked terrain, gather remotely, or act on an unseen target with perfect current information.
- Retreat can save someone or fail for a concrete spatial reason. Inaccessible targets trigger replanning rather than endless pursuit.
- Multiple hunters share one creature's finite loot. A simultaneous kill/death produces exactly one carcass and the correct dropped pack.
- Helping and harming change the recipient's memory; children or newly created actors do not inherit personal grudges.

## Release 5 — Connect neural policies and controlled evolution

Only begin after outcomes, action feasibility, logistics and accounting are stable.

**Build**

- Expose the same sensed action–target candidates to both the utility baseline and an optional neural scorer.
- Evolve policy parameters and predispositions while leaving inventories, physical rules, execution and accounting outside the genome.
- Reuse ancestry-aligned crossover, bounded mutation and neutral topology additions from Prototype II, with schema/version compatibility checks.
- Keep life-state progression, births, acquired memories and policy inheritance as separate systems. Add cultivation only when it has explicit time, energy and material costs and meaningful alternatives.

**Tests / completion criteria**

- Mutated policies cannot bypass range, resources, capacities, death or construction costs.
- Evaluate on long episodes, different initial populations and unseen map/ecology seeds. Reserve separate training and evaluation seeds.
- Compare with the utility baseline using survival, fulfilled needs, actual consumption/production, harm, ecological depletion and behavioral diversity. Never reward a mere request to aid, trade, build or cultivate.
- Report degenerate habits, sensitivity to small changes, excessive switching and constant-action policies even if scalar fitness improves.
- Improved training reward alone is not sufficient to declare a better policy.

## Regression harness used throughout

Maintain a saved baseline and run paired scenarios against it. Start with 30 fixed seeds × 100 simulated days, then increase coverage when failures or new mechanisms justify it. Include:

| Scenario | What it challenges |
| --- | --- |
| Normal woodland | Founding, food balance, hauling, construction and trade |
| Poor food / high population | Shortage response, rationing, farming and risk |
| No renewal | Depletion, finite reserves and eventual failure without new inputs |
| Drought then recovery | Crop failure, conservation, delayed ecological recovery |
| Dangerous hill | Avoidance, combat mortality, retreat and rescue |
| Empty quarry | No free stone or unpayable construction |
| Isolated caravan | Population threshold and finite immigration |
| No coin / unequal wealth | Communal access, distribution and market limitations |

For every run, retain the configuration, seed, fixed-step count, ledger, population curve, cause-of-death totals, food-days remaining, work/travel/idle fractions and a replay checkpoint for failures. Run engine batches in a background worker and show progress/cancellation in the UI.

Browser verification covers pause/resume, speed changes, map selection, inspector accuracy, saving/loading, scenario controls, charts and small-screen layout. It supplements engine checks; it does not replace conservation and deterministic replay tests.

## Recommended next implementation

Implement **Release 1 first**, then a narrow Release 2 consisting of reservations, food reserve management and one properly costed farm plot. This provides a measurable survival loop and a reliable experiment harness before expanding markets, social behavior or evolution.
