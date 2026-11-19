# Xianxia Native Observatory

A working **Rust + Rayon simulation with a Godot viewer**, isolated from the earlier browser prototypes. The viewer is a real native Godot project; GDScript does not simulate the population. Each counted person has a distinct-weight feed-forward neural genome and a compiled evaluator.

See [BENCHMARKS.md](BENCHMARKS.md) for actual 10k/100k/1.2m measurements and their limits. The visible native viewer defaults to a manageable 10,000-agent world.

## Open the prototype

In PowerShell from this directory, start both components together:

```powershell
.\run-prototype.ps1
```

This starts/reuses the hidden backend, waits for its listening port, and opens/reuses the Godot viewer. Use a normal local PowerShell session so both processes can use the same Windows loopback connection. A backend launched in a restricted tool environment may be unreachable from a normal Godot window; this was diagnosed and corrected during testing.

For separate development terminals:

```powershell
.\run-server.ps1
```

Keep that running. In another PowerShell window:

```powershell
.\run-viewer.ps1
```

The viewer connects to `127.0.0.1:4176`. Press **Run**, **Step 1s**, or **Advance 30s**. Click a person, or select an agent by numeric ID, to inspect that individual's brain and candidate scores. Save/Restore exercise the native checkpoint system. New world supports 1,000, 10,000 and 100,000 people. The viewer renders at most 20,000 sampled individuals; the backend still owns and simulates the full population. Closing/disconnecting the viewer pauses the backend.

`run-viewer.ps1 -Godot C:\path\to\Godot.exe` accepts another compatible Godot binary. Godot 4.6.3 is available on this machine and was used for validation. Rust dependencies are locked in `Cargo.lock` and downloaded to the checkout-local `.cargo-home`. The server launcher builds the native executable if needed. Stop an old server before rebuilding its running executable on Windows.

## What runs

- **Packed agent arrays:** positions, physiology, money, rations, cargo, action/target, home and decision timing. No Godot node or thread per person.
- **Individual brains:** canonical sensor IDs, distinct seeded weights, small varied topologies, enabled/disabled genes, a shared historical innovation registry, homologous crossover, neutral identity-node splitting, connection mutation, compatibility/species grouping and compiled allocation-free evaluation.
- **Parallel sensing and inference:** a fixed Rayon pool reads immutable world state. Agents are staggered at one decision per simulated second; existing actions execute between decisions. Resolution order rotates deterministically to reduce a permanent first-agent advantage.
- **Physical economy:** berries, timber, nonrenewable stone and coastal fish stocks; carrying, travel, food consumption, energy, integer inventories and explicit biological renewal. Public bids reserve actual coins, public asks list a subset of actual lots, and execution checks availability again.
- **Three settlements:** Willow Reach, Tidewater and Stone Crossing have independent markets. Traders can physically carry purchased lots between them. Provenance distinguishes locally produced goods from imported resale.
- **Construction and roads:** real timber/stone are committed before labor. Housing blocks hold 32 beds as a scale abstraction. Sufficient actual inter-settlement delivery traffic can motivate a funded road; completed roads accelerate nearby travel. These are starter rules, not a complete organic town/country system.
- **Observed sales profiles:** product shares derive from completed purchases. There is no economy-type variable controlling policy, resources or building eligibility. A test changes descriptive labels and verifies unchanged world results.
- **Persistence:** versioned Serde/Postcard checkpoints compressed by Zstandard. Genomes, state, ownership, resource stocks, quotes and random state persist; compiled evaluators rebuild after restore. Serial and parallel continuation are checked for exact equality.
- **Godot rendering:** a procedural terrain display and three batched MultiMeshes. Binary snapshots carry metadata and 16-byte render records rather than million-agent JSON frames. Selected-agent diagnostics travel separately inside the bounded metadata.
- **Python analysis:** standard-library scripts read benchmark reports and test the actual TCP protocol. They are outside the agent execution loop.

## Benchmark and verify

```powershell
$env:CARGO_HOME = Join-Path (Get-Location) '.cargo-home'
cargo test --workspace --locked
cargo build --release --locked
.\target\release\sim-runner.exe verify --agents 10000 --steps 1000 --threads 8 --out runs\verify-10000.json --checkpoint runs\verified.snapshot
.\target\release\sim-runner.exe bench --agents 100000 --steps 100 --threads 8 --out runs\bench-100000.json
python experiments\report.py runs\verify-10000.json runs\bench-100000.json --csv runs\comparison.csv
```

With the server running and the viewer disconnected:

```powershell
python experiments\check_protocol.py
```

This checks framed commands, snapshots, save/restore and continued replay over the real network interface, then leaves a populated 10,000-agent world paused for inspection. The server supports one viewer at a time. It binds only to loopback. The viewer can save/load only the checkpoint path chosen by the server CLI, not an arbitrary path supplied through TCP.

Reports include initialization time, scheduled decisions, simulated time per wall time, p50/p95/p99 tick times, mean brain size, living population, actual trades/shipments and accounting results. Initialization, simulation and rendering are different measurements. The current 28-input brains average roughly 116 genes (earlier reports used 22 inputs / 92 genes or 12 inputs / 53 genes); its speed does **not** establish the earlier 256-gene, two-hour million-agent acceptance target. See actual `runs/*.json` measurements rather than assuming a language guarantees performance.

## Accounting and contracts

For each commodity, **initial + environmental renewal = current stocks + consumed food + committed construction + ore minted**. Commodity current stock includes patches, public lots and every person's rations/cargo, including stranded inventory on dead entities. Coin current stock includes private money, market funds and bid escrow; initial coins + explicitly minted coins = current coins. Every purchase transfers existing goods and money. Road and housing labels do not generate assets.

The initial scenario declares three personal rations and eight coins per person plus two communal rations and twenty public coins per resident. These are counted starting endowments, not recurring immigration, wages or production. Wild capacities scale with the scenario's initial population; they do not dynamically grow when population changes. Fish/plant renewal is a bounded productivity proxy; a complete fish food web, climate and farm system is future work.

The NEAT seed is deliberately small and hand-structured (four tanh hidden features with individually varied weights); these brains are not presented as trained or biologically realistic. Gene mutation, homologous crossover and representative speciation primitives are implemented and tested. There is **no full generational optimizer, live reproduction, recurrent memory or online learning** in this slice. Small prototype species grouping is not yet the bounded-island algorithm for a million evolving genomes.

## Layout

```text
crates/neat-core/       genome, innovation registry, mutation/crossover, compiler
crates/sim-core/        packed world, sensing, markets, movement, accounts, saves
crates/sim-runner/      headless CLI, benchmark reports, loopback viewer server
viewer-godot/          native scene, controls, instancing and selected-brain inspector
experiments/           Python report/protocol scripts
runs/                  measured reports, logs and local checkpoints
```

The native world does not depend on Godot or Python. Existing JavaScript/Python prototypes remain untouched as earlier reference experiments.

## Deliberate limits

This is a vertical slice, not the full scale plan. Markets use stock-responsive formula prices and known neighboring quotes, not rich learned bargaining. Resources are fields, navigation is direct-line land travel, and cached opportunities are shared by home region. There are no ships, ports, countries, true urban clustering, cultivation, taxes/militia, tool/medicine chains, uncertain sensing, births, corpse recovery or dynamic world expansion yet. Those systems remain in the separate browser prototype or on the roadmap.

The native save currently serializes a whole-world buffer; streaming region checkpoints, deep hostile-file validation, bounded long-term archives, production-grade ownership migration, per-region worker scheduling and GPU inference are later work. The existing world has no arbitrary non-player external message sources; TCP input budgets and basic checkpoint validation still apply. CPU/GPU cross-platform float replay is not claimed.

The right next step is to use these measurements and fixtures to refine regional locality and NEAT workload budgets, then port the richer jobs/leadership economy while keeping conservation and replay intact.

## Local convoys and organic settlements

Long trips (at least 180 world units) seek a convoy once per simulated second. Recruitment searches a 160-unit vicinity and accepts destinations within 64 units of the convoy endpoint. Groups hold at most 50 people. The nearest compatible waiting convoy is preferred. Other routes and full groups can reuse the same nearby gathering spot, and settlement centres remain reusable spots after departure. A new remote spot is created only when no existing spot or settlement centre is nearby. Travelers physically rally, retaining their own goods, coins, homes and task. A group departs after three seconds once assembled, or sooner when full; a lone traveler leaves individually after the wait. A nine-second rally limit prevents stragglers blocking travel forever. Solo fallback/final approach has a twelve-second recruitment cooldown. Formation travel uses 1.45 times ordinary speed, including the same road benefit. A final individual approach handles slightly different destinations. Emergencies (health below 25 or hunger over 85), death and loss of the last companion release membership.

A committed trip temporarily holds the chosen task instead of changing direction at each neural decision. The inspector explains this travel commitment. Wagon markers show actual membership counts; hollow wagons are rallying. Click a wagon to inspect its leader. Only 2,000 convoy markers and the existing bounded agent sample are transmitted; members of omitted convoys remain visible in the individual sample. This is a display limit, not a simulation limit. Spatial recruitment buckets and reusable convoy slots avoid global pairwise matching and unbounded convoy history.

The three initial trading camps remain scenario starting locations, not predeclared villages. Actual living affiliations replace fixed spawn population counts. A stable resident has accumulated twelve seconds resting or building within the home settlement housing footprint, excluding convoy time. Village status requires eight such living residents, a completed material-funded housing block and eight communal food units. Population below four removes village status; food fluctuations alone do not erase an established village.

New camps can appear away from existing markets. Local workers accumulate twelve seconds of actual resource-site work in a 96-unit cell. At least eight eligible, sufficiently fed and energetic workers must gather within 65 units of their shared center, at least 120 units from another market. Up to 32 founders each contribute two existing coins and one spare berry ration; twelve carried timber units fund communal shelters. No people, goods or money are generated. Founders change home without teleporting. The new camp trades through the same market system and receives an unbuilt connection to the closest market; roads and permanent housing still require materials and labor. The prototype currently caps settlements at 64. Founding is an explicit coordination heuristic, not an evolved neural output or a claim of complete political emergence.

Unpurchased sale listings are revocable: public construction can consume the market's unsold timber and stone and reduce the listing accordingly. Bid escrow and completed purchases remain conserved. This fixes a case where listed inventory prevented housing indefinitely.

Checkpoint schema 4 adds clock offsets, 28-input brains, rested state and reserved shelter beds. Schema 1/2/3 saves cannot be restored by this build; the pre-night live world was backed up as `runs/pre-night-live.snapshot`. Earlier backups are retained. New saves use the usual Save/Restore controls.

Validation: 22 Rust tests cover local matching, the 20-person cap, fallback, travel speed, bounded movement, death cleanup, funded camp founding, passing-crowd rejection, construction from unsold inventory, conservation and deterministic save/replay. A 10,000-agent seed-43 run over 600 simulated seconds formed 6,690 convoys, completed 315 housing blocks and promoted all three starting camps to villages, with balanced goods and money. It did not found an additional camp in that run; dynamic founding is verified by the funded-worker fixture. Scarcity and untrained brains still cause substantial deaths in longer runs. Earlier million-agent benchmark figures predate this travel extension and are not measurements of this build.


## Three viewing levels

Use **1 Overview**, **2 Features**, or **3 Animation**, or zoom continuously with the mouse wheel. Middle-button dragging pans. Detail buttons center the camera on the selected individual; Overview restores the full map. Fidelity follows magnification: below 2x uses the original batched dots/wagons, 2x to below 5x shows individual features, and 5x or closer adds gait, swinging arms, and working/tool motions. Buttons choose 1x, 3x and 8x; the wheel reaches 12x. Pausing freezes action animation. Individuals have deterministic clothing and skin/hair variations, belts, packs, pouches and task tools. Terrain, houses and mines deliberately retain schematic placeholders at every level.

Close views reveal convoy members rather than replacing them with a wagon. Drawing is culled to the camera and capped at 2,500 detailed sampled people; the overview still uses the existing maximum 20,000-person snapshot sample. This does not change authoritative simulation, brain evaluation or accounts. At populations above the snapshot sample limit, close views are sampled too; viewport-specific streaming is future work.

## Weight, mines, currency and barter

Capacity is currently a per-person 32, 36, 40, 44 or 48 weight units (a deterministic scenario variation, not an evolved body trait). Personal rations and cargo both count. A nonempty currency wallet weighs exactly 1 regardless of its balance; an empty wallet weighs 0. Transfers check the resulting load, and world validation checks every inventory against capacity. There is one cargo commodity at a time plus separate food rations and wallet; richer containers are future work.

| Item | Weight per unit | Minted copper-equivalent units |
| --- | ---: | ---: |
| Berries | 1 | - |
| Timber | 4 | - |
| Stone | 3 | - |
| Fish | 2 | - |
| Gold ore | 5 | 100 |
| Silver ore | 4 | 10 |
| Copper ore | 3 | 1 |
| Nonempty currency wallet | 1 total | Any balance |

Three finite, nonrenewing mines on the hill supply ore. Mining uses the same physical harvest/travel rules as other resources. A person can carry ore to a market and choose **Mint**, consuming ore once and creating the corresponding recorded currency. Prices and balances use integer copper-equivalent units; separate coin denominations, mint ownership, refining costs and fluctuating metal exchange rates are not modeled. All mine reserves and starting money are declared scenario endowments. Currency growth is limited by the remaining ore and tracked separately from ordinary transfers.

**Barter** exchanges carried goods for available market food, even with no money. The market uses its bid/ask values, rounds incoming goods upward, rechecks both inventories and capacity, and creates no currency or credit. The current barter slice supports goods-for-food, not arbitrary bundles or direct person-to-person negotiation. Markets record incoming bartered goods in realized product sales and preserve their origin.

Construction supplies a concrete traded service: **Build** pays one existing treasury currency unit for each one-second work installment; **WorkForFood** offers one food unit for a four-second installment (the last installment may finish a shorter remaining job). No payment happens without an active unfinished project, payment inventory and carrying room. Public bidding leaves a small cash reserve for wages. Food payment transfers inventory, and actual consumption is recorded only when it is eaten. Free founding coordination remains a separate heuristic; the prototype does not yet model every type of service.

The NEAT candidate scorer now has 22 inputs: the original 12 plus gold/silver/copper identity, wallet balance, currency need, load fraction, remaining capacity, affordability, barter mode and payment mode. Feasible outputs include mining through Gather, Mint, Barter, currency-paid Build and WorkForFood alongside existing actions. Affordability and weight are enforced before choices and again when executing, so mutated weights cannot authorize free purchases or overloading. Up to 16 candidates preserve a feasible Rest alternative. These remain individually varied seed brains, not trained economic agents.

Validation for this extension: **28 Rust tests passed**, including finite-mine minting, no double mint, constant wallet weight, overloaded harvest/purchase rejection, cashless barter, currency/food service payment, new candidate sensors, conservation and serial/parallel checkpoint replay. The actual TCP save/restore replay passed. All three fidelity levels were inspected in the live connected Godot viewer. A release-mode 10,000-person, 300-second run (`runs/currency-10000.json`) had balanced accounts, minted 133,060 currency units from finite ore, completed 10,053 barter exchanges and 3,222 paid service installments, and had no deaths. Food service payment was verified in its controlled test; that run selected none. Its p95 simulation tick was 0.9707 ms; this excludes rendering and does not establish the million-person target for the expanded brains.

## Zoom renderer optimization

The detail view now uses one shared 16-appearance, 19-pose character atlas baked once into a transparent SubViewport. One textured MultiMesh draws the visible detailed people. A canvas shader chooses idle, walking, working, tools or dead poses and interpolates movement between snapshots; per-frame animation updates two shared uniforms rather than redrawing every body part in GDScript. Features and animated fidelity remain separate. The same 2,500-person detail cap applies, and the selected sampled person is retained even in a crowded view.

Snapshots rebuild a 64-unit spatial index. Camera changes are coalesced once per frame and query nearby cells; hidden overview buffers are not rebuilt while zoomed in. The map is redrawn only for a snapshot or camera change. HUD styling is cached, individual state arrays are updated without whole-array duplication, and the detail buffer is reused. The client processes the newest complete snapshot after a backlog and retains command replies; inspector text refreshes at most four times per second. This changes presentation only: no Rust state, economy, brain scheduling or save schema changed.

An actual Windows/OpenGL rendering fixture uses 10,000 synthetic records, 2,500 visible living detailed people, 120 measured frames after 30 warm-up frames, the same scripted snapshot/zoom sequence, and disabled vsync. It does not connect to the backend. On the RTX 5060 Ti in this environment:

| Measurement | Previous renderer | Batched renderer |
| --- | ---: | ---: |
| Mean frame time | 195.53 ms | 1.39 ms |
| p95 frame time | 295.85 ms | 7.14 ms |
| Median draw calls | 27,510 | 10 |

These are isolated dense-scene rendering measurements, not a guarantee of whole-game FPS or million-agent performance. The atlas startup is outside the measured warm frames. Reports are `runs/zoom-legacy.json` and `runs/zoom-optimized.json`. The baseline is retained only in `viewer-godot/bench/legacy_field.gd`. Reproduce with Godot `--path viewer-godot --script res://bench/render_bench.gd --resolution 900x640 --disable-vsync -- --out ABSOLUTE_REPORT_PATH`, adding `--legacy` for the baseline. The fixture asserts the unchanged visible count, selected-person retention and packed instance-buffer shape. The updated live viewer was reopened on the existing backend without resetting the world; the detail view was visually checked and the startup log was clean.


## Day/night, beds, camps and rested status

One prototype day is **300 simulated seconds**, starting at 06:00. Night is 18:00-06:00. All land travel, including solo movement, roads, convoy assembly/travel and the approach to a bed, uses **65% of daytime speed** at night. The rested buff does not cancel this global travel penalty. Dawn/dusk blend the map tint; the clock reports day, hour and travel multiplier. **Dawn** and **Night** change the clock phase for experiments without advancing physics, ecology or hunger. They can advance the displayed calendar when setting an earlier hour. The speed controls change how fast the whole simulation runs, not the day length in simulated seconds.

A person may actually sleep only in a **reserved bed** at a settlement or a constructed roadside camp. Arriving at a market or choosing a rest action is insufficient. Each completed housing block supplies **32 beds**; unbuilt camps have no automatic bed capacity. A bed is reserved before travel and cannot be allocated twice. Leaving the rest task or dying releases it. Every actual housing block is shown, with stable positions in expanding rings, and sleepers are placed at individual bed positions inside its footprint. Indoor figures are compact reclining representations. The physical layout and resting destinations use the same placement formula; coastline/world bounds are respected. Population by itself does not generate beds. The UI lists capacity/reservations separately from people actually sleeping.

A person carrying **four timber** can spend five seconds constructing a **four-bed roadside camp**, outside the current settlement housing footprint and at least 30 units from another camp. Timber is consumed through the construction ledger. Camps do not create people, currency or market stock and do not become settlements automatically. Nearby people can reserve their free beds. There is a prototype limit of 256 roadside camps; shelter reservations use constant-time free-slot pools rather than searching every bed. Expanded settlements may eventually grow around an older camp. If no bed or camp-building option is feasible, Wait is an idle fallback: it provides no rest or energy recovery.

The selected person's **rested meter** runs from 0 to 100 and is displayed as a bar. Initial value is 40, with no buff. Sleeping fills it at five points per simulated second. Reaching 80 grants the rested buff and its blue/gold symbol; further sleep can fill it to 100. The meter drains one point per awake second. The buff persists until the meter reaches zero, allowing a person to remain rested below 80 after waking. Death clears the active status. These thresholds are explicit starter rules, not evolved physiological traits.

While **unrested**, actual sleep also restores five energy per second. While **rested**, energy no longer increases from sleep, and energy deductions from awake fatigue, travel, harvesting, paid/food work and camp construction are suppressed. Hunger and starvation health loss still apply. Without the buff, awake fatigue costs 0.08 energy per second in addition to existing task costs. This distinguishes energy from the time-limited rested benefit and prevents gaining energy merely by waiting in the field.

The six new canonical neural inputs are night, rested meter, active rested buff, bed availability, night-rest affinity and sleep access. Default seed weights and the rest opportunity's need signal favor resting at night. Rest, MakeCamp and Wait are explicit feasible choices. Reserved-bed availability is checked again when resolving competing requests. Trip commitment remains in effect for people already in a convoy. Brains are still untrained varied seeds; sleeping schedules are influenced rather than forcibly assigned.

Validation: **36 Rust tests passed**, covering clock wrap/controls, night speed, bed-free denial, 32-bed contention, release on departure/death, housing growth, recovery at the actual bed, buff immunity/expiry, funded camp construction, neural night preference, binary rest flags and exact night/shelter checkpoint continuation. The real TCP check also passed phase control and clock/shelter save/replay. The final release 10,000-person, 300-second report (`runs/day-night-10000.json`) had balanced accounts, zero deaths, 315 completed housing blocks and 9,993 rested people at dawn. Its p95 simulation tick was 1.2701 ms; this excludes rendering. Dynamic camps are verified by the funded-camp fixture; that 10,000-person run did not build one. The rendered 2,500-person fixture still used 10 median draw calls and averaged 1.59 ms/frame (`runs/zoom-night-optimized.json`), an isolated dense rendering measurement rather than whole-game FPS.

The old live world remains in `runs/pre-night-live.snapshot` (schema 3). The updated night-cycle world was saved and restored while improving the housing layout; its pre-layout state is retained as `runs/night-before-layout.snapshot` (schema 4). Save/Restore now includes clock phase, meter/buff state, camps, free-bed pools and every reservation. The atlas gained one sleeping pose and per-instance status symbols while retaining the batched renderer.


## Gravediggers, village NEAT brains and job boards

Each settlement now owns an independently seeded NEAT genome, innovation state and compiled policy evaluator. Its municipal sensor meanings are separate from the individual's 28-input schema. Once per simulated second it evaluates feasible **Wait**, **FundGraveyard**, **PostGravedigger** and **RaiseReward** choices. Inputs include nearby/remote bodies, age of the oldest unburied body, market sales revenue, completed wage expenses, smoothed income/expense rates, available treasury cash, food/housing coverage, construction materials, cemetery space and staffing pressure. Candidate scores and sensor values are exposed in job-board metadata; clicking a physical board narrows the panel to that village's policy. These are varied seed genomes using NEAT infrastructure, not trained policies or a new generational optimizer.

Bodies are assigned to the nearest market when recorded. Nearby pressure uses the settlement housing footprint plus 70 units; remote bodies contribute less pressure. An indexed corpse record has one of three states: on the ground, carried, or buried. A body can have only one worker claim. Municipal opportunity lists keep the nearest 64 available bodies; this bounds individual job searches. Councils can grow advertised staffing toward one worker per four unburied bodies, up to 24 haul contracts per village. Fiscal and material feasibility remain hard constraints regardless of neural weights.

A village can commit **8 timber and 4 stone**, plus an escrowed **8-currency completion bounty**, to a graveyard-building contract. Individuals evaluate that service through the same risk/reward candidate scorer as other work. The worker visits the board, walks to the plot and supplies 20 seconds of labor. This is the construction task within the gravedigger job family. The first completed plot holds 64 bodies. Full plots can be expanded by 64 spaces through another material-funded building contract; existing graves persist. Unfinished work survives a worker abandoning the job, and its materials are not charged twice.

A gravedigger contract reserves both a corpse and the existing currency reward before work starts. The worker first visits the board, travels to the body, spends one second collecting it, physically carries it to the graveyard and spends three seconds burying it. The body adds **20 carrying-weight units**. The reward transfers from escrow to the worker only on completion. Dead people's personal inventories and currency remain accounted for as frozen estates; this slice transports remains, not estate goods, and does not implement looting, heirs or decomposition. Carried bodies are shown as burial bundles/coffins; the original ground corpse is hidden while carried and after burial. Cemetery markers and counts show completed burials. Full burial registries persist in checkpoints but snapshots expose counts, keeping network metadata bounded.

Individuals can decline jobs based on benefit, reward, distance, nighttime risk, health, energy, food and carrying room. Normal choices are held during an accepted contract, with cancellation for death or urgent health/hunger/energy conditions. Cancellation drops a carried body at the worker's location, releases its claim and refunds the unearned reward. No teleport to the graveyard, free wage minting or resurrection is used. Unfilled advertised work can raise rewards after ten simulated seconds, up to 32 currency units; the difference must be funded for all open positions, while accepted contracts keep their promised reward. Pressure, income and candidate utility influence municipal decisions rather than guaranteeing every village always hires.

Each village has a clickable wooden **job posting board** near its market. The panel shows bodies, vacancies, compensation, active workers, treasury and advertisement escrow, income/expense rates, village brain size, policy scores and blocking conditions. **All job boards** restores the multi-village view. **Test: 5 nearby deaths** is an explicitly labeled scenario control: it kills up to five nearby existing people at the focused village (Willow Reach if no village is focused). It does not create people or supplies. Use Run to observe work; testing is optional and this control changes the current simulation.

A separate reproducible headless demonstration avoids changing the live world:

```powershell
.\target\release\sim-runner.exe civic-demo --agents 1000 --steps 1800 --threads 8 --out runs\civic-demo.json --checkpoint runs\civic-demo.snapshot
```

This warms an ordinary world for 30 seconds, creates 12 nearby test deaths, then runs the existing neural agents and municipal policies. The measured release run built one graveyard, buried all 12 bodies and completed 13 contracts, with balanced resources/currency and exact checkpoint continuation. Its p95 simulation tick was 0.1519 ms for 1,000 allocated people; it is not a million-person performance claim.

**44 Rust tests passed**, including municipal genome independence/income inputs, complete neural construction/haul/burial flow, unique corpse claims, escrow conservation, carrying limits, reward escalation, unfunded-job rejection, worker death/drop/refund and exact active-contract save/replay. The actual TCP check also passed mortality scenarios and job-board/graveyard checkpoint continuation on a temporary server. The live viewer was checked for job-board display and corrected upright sprites. Character rendering now flips the QuadMesh UV vertically *inside each atlas cell*, preserving pose selection and batched GPU animation; status badges and carried bundles use those corrected coordinates too.

Checkpoint **schema 5** includes municipal genomes, corpses, contracts, income counters and graveyards. Schema-4 day/night saves migrate automatically using explicit field moves, retaining NPC genomes, physiology, accounts, residences, shelter reservations, clock and current tick. Existing dead people become ground-corpse records; their earlier exact death time is unknown, so age tracking begins at migration. Revenues/expenses begin tracking from the upgrade; earlier monetary history is not reconstructed. The live pre-upgrade save is retained as `runs/pre-civic-live.snapshot`. Migration was verified against a real schema-4 checkpoint with 10,000 people without advancing its tick. New saves use schema 5; schemas 1-3 remain unsupported by the current loader.


## People, households, farms and tools (schema 6)

The world is now 3,072 by 1,920 units, with the starter settlements three times farther apart and buildings retaining their physical size. Berry patches have bounded capacity and replenishment: 600 units and three units per simulated second per initial population scale (one scale per 1,000 starting people). Coastal and hill settlements gain nearby berry patches. Newborns do not increase environmental regeneration. Iron deposits are finite and do not regenerate.

Select a person to see their generated given name and surname, gender, age, spouse, children, pregnancy, traits, happiness, lifelong aspiration, and up to five expiring goals. Every individual receives exactly one aspiration: accumulate 1,000 gold coins (100,000 copper units), craft 20 tools, raise two children, harvest 256 rice, or complete 20 public jobs. Aspirations and candidate-specific goal alignment/urgency enter the 42-input NEAT evaluator. Goals arise from aspirations, local jobs, farms and seeded personal choices; completing a measured target adds eight happiness, expiry removes two, and completing an aspiration adds 20 once.

Marriage requires two willing, compatible adults of opposite gender with a marriage goal and a shared aspiration or another shared goal. Parents, children and siblings cannot marry. Pregnancy lasts 270 simulated days. Return-home rules begin two days before the third trimester, and the mother stays at her settlement through birth and two postpartum days. Birth waits for physical arrival. Children inherit their father's surname, a crossed and mutated parental brain, parental traits and a partly inherited aspiration; they start with no money, tools or resources. Children stay home and consume real household/public food. Household food transfers require the supplying parent to be at the same settlement. Initial inhabitants are adults aged 18–55; age advances on actual simulation ticks (365 days/year). Old-age mortality, infant-specific art, education and full kinship beyond immediate family remain future extensions.

Traits have mechanical effects: night owl removes individual night travel penalties; fleet/slow change travel speed; hardy reduces energy use; clumsy slows work; glutton increases hunger; coin grabber supplies a preference for currency over barter. Trait preferences never bypass inventory, pregnancy, age or physical travel constraints. Severe hunger triggers a legal emergency forage action that can consume food directly at a resource patch even with full hands, preventing future brain generations from choosing work until starvation while food is reachable.

Each initial settlement has nearby fertile land and an iron deposit. Councils can fund up to four rice plots per settlement using four timber, two existing seeds and an escrowed two-coin farming wage. Rice requires **15 complete simulated days and 15 daily tending visits**; changing the displayed time of day does not advance growth. An untended field can take longer. Harvesters physically carry the mature crop, and reseeding consumes existing food. Unused tending wages are refunded at maturity. Use **1 day** or **15 crop days** to advance actual simulation time; Pause cancels an advance.

A housed settlement provides a basic workbench. Blacksmiths use existing iron and timber to craft wood axes, pickaxes, farm hoes and shovels; tools are bought/equipped, weigh 4/5/3/4 units, and improve the matching work rate by 50%. Production is limited by local demand, materials, worker access and treasury funds. The prototype now accounts for 13 products, including rice and tools, with explicit material transformations and no currency creation outside minting.

Schema 4 and 5 saves migrate once: positions scale with the larger map, original people/stocks/deaths remain, generated identity data is added, brain sensor IDs are remapped, and new environmental deposits are explicitly entered into the initial resource ledger. Save backups should be kept before migration.

Reproduce the household/crop integration scenario independently of your live world:

```powershell
.\target\release\sim-runner.exe life-demo --agents 64 --steps 48000 --threads 8 --out runs\family-crop.json --checkpoint runs\family-crop.snapshot
```

This explicit fixture starts with one married, late-pregnancy couple to exercise birth without waiting 270 days. Normal new worlds do not receive forced marriages or pregnancies. Automated checks cover inheritance, adult/kinship restrictions, trimester travel, bounded goals, hunger with full cargo, paid daily tending, 15-day maturation, tool accounting and deterministic checkpoint continuation. Long-run population balance and million-entity throughput with these new systems require further measurement; earlier benchmark results predate this extension.


Validation for this extension (2026-11-18): 60 automated tests passed. The separate 64-person, 16-day family fixture ended with 65 living people, one birth, 256 rice harvested, zero deaths, balanced goods/money and exact checkpoint continuation. A new 10,000-person world ran 3,000 ticks (one full day) with 10,000 survivors, 750 marriages, 12 pregnancies, 853 crafted tools and balanced accounts; p95 native tick was 2.09 ms on this machine. This is short-run evidence, not a long-term population guarantee. The separate Windows/OpenGL rendering fixture retained 2,500 detailed instances from 10,000 sampled people, averaging 2.22 ms/frame (p95 9.81 ms); it does not measure a million-agent simulation. Reports are in `runs/family-crop-final.json`, `runs/life-10000-final.json` and `runs/life-render-final.json`. The real TCP test also verifies 42 sensors, 13 products, exact family/farm replay, and an actual 3,000-tick day advance.


## Village centres, taverns and social traits (schema 7)

Housing uses stable, staggered outer rings starting about 140 world units from each village centre, with varied angles and setbacks. The viewer and native bed destinations use the same placement formula. Existing houses are repositioned; their stocks and bed counts are preserved. A **new 32-bed housing block requires 16 timber, four stone and ten units of construction labour**. All materials must exist before the project can begin; consumed materials are recorded in the construction ledger. No houses appear merely because population increases.

Village NEAT councils can fund one central tavern after a village has housing, 12 timber, six stone and 20 copper units for its construction reward. Materials are committed once, cash is escrowed, and 20 units of physical construction work must finish before opening. The completing worker receives the reserved reward. A tavern is a free public social amenity in this prototype; brewing, alcohol, refreshments and operating costs are not yet simulated. The board shows construction/open status and guests; click a tavern to inspect its village.

Adults can choose the new Socialize action through their existing candidate-scoring brain. It uses candidate-specific need and benefit inputs derived from social score and traits; sensor IDs remain unchanged. Participants must travel to an open tavern, and at least two guests must be present. Up to 32 admitted guests gain eight social points per five seconds, capped at 100. Scores fade by two points per full simulated day. **Low social score never directly reduces happiness, energy, health, wages or work rate.** Emergency food, childhood and pregnancy rules still apply.

Additional inheritable traits:

- **Socialite:** score of at least 60 grants two happiness at the daily update; lower scores cause no happiness loss.
- **Gregarious:** gains 12 social points per visit interval instead of eight.
- **Community minded:** score of at least 60 gives a 10% work-rate bonus for farming, harvesting and public construction/burial work; below that threshold the ordinary work rate applies.

Individuals can hold several traits, including existing advantages and drawbacks. Each parental trait is inherited independently: a trait shared by both parents is retained, while a trait found in only one parent has a 50% chance. If all rolls miss, one available parental trait is selected uniformly. Children start with zero social score and inherit no social progress. Schema 4–6 saves retain people, buildings, assets and world positions, receive zero initial social scores and a deterministic subset of the new social traits, and save as schema 7. Their houses adopt the new shared layout without charging construction costs again.

Validation: 65 automated tests passed. A separate 10,000-person, 6,000-tick run formed and built a tavern through normal council/individual decisions, had 30 admitted guests at the final snapshot, retained all 10,000 people and balanced all material/currency accounts. See `runs/social-10000.json`. Tests separately verify no unfunded houses or taverns, labour before opening, no default low-social happiness penalty, Socialite/Gregarious/Community-minded effects, inheritance combining both parents, and exact social/amenity checkpoint continuation.


Convoy/placement update: waiting groups sharing an anchor render as one gathering marker (its displayed count is everyone waiting there, potentially several route groups; each departing group is capped at 50). The seven-column formation keeps all 50 members within the assembly radius. Graveyards are selected on land about 200 units beyond the current housing footprint, choosing among radial candidates for separation from other settlements. Existing graveyards within 150 units of expanding housing are moved outward at the next civic update; their burial registries, remains, construction state and rewards are retained. In a crowded map the candidate with greatest settlement clearance is used. Tests cover 50-person caps, physical rallying, nearest compatible group selection, shared spots for different routes, persistent settlement anchors, outward graveyard placement and intact buried remains. Total automated checks: 68 passed.


## Small forests and NEAT wildlife (schema 8)

The former single large woodland is replaced with small groves around the actual berry and timber patches. Grove radii are 90–140 world units; resource quantities, resource positions, settlements and housing are preserved. Each grove starts with six rabbits and four deer, with both sexes represented. Migration adds this explicitly seeded starting ecosystem once; loading a schema-8 save never replenishes extinct animals or consumed grass.

Each animal owns a distinct genome and compiled NEAT evaluator. Animal policy uses its own historical innovation registry and sensor meanings, separate from human/municipal meanings: need, benefit, risk, distance, fertility, continuity, species, hunger, nutritional energy, health, nearby humans, grass cover, local density, night and adulthood. These occupy 16 meaningful channels in the shared 42-slot evaluator; remaining inputs are zero. Six candidate actions are scored: graze, eat berries, flee from nearby people, rest, explore, and approach a compatible mate. Legal feeding requires appetite or depleted reserves, arrival and actual available food. Animals move continuously; they do not teleport to food or partners. NEAT decisions run once per second and movement/metabolism at the normal 10 Hz.

Grass is **inedible to people** and is not a market product. Each grove has five grass patches with 60 units each. Photosynthesis renews up to 0.02 units per second per patch in daylight and 0.006 at night, capped at patch capacity. A separate exact grass ledger tracks initial growth, regrowth, consumption and remaining stock. Rabbits eat one unit per meal; deer eat three. Animals can also consume nearby berries from the same finite patches used by people; those meals decrement the ordinary berry stock and food-consumption ledger. No meat or other human goods are created when an animal dies. Wildlife counts and grass/berry consumption appear above the job boards.

Breeding requires healthy, well-fed adults of the same species and opposite sexes, local food, physical proximity and time together. Parent/child and sibling pairs are rejected. Rabbits mature at 90 days and gestate for 30 days, producing two young; deer mature at 365 days and gestate for 200 days, producing one fawn. Maternal reserves fund birth, and food shortages can delay or prevent it. Post-birth breeding cooldowns are 45 days for rabbits and 365 days for deer. Offspring inherit crossed parental genomes with weight mutation and occasional topology mutation. They inherit lineage and their home grove, start young, and must forage for real food. Juvenile nursing and weaning are simplified to independent foraging in this prototype. Starvation and species-specific old-age death are possible.

To keep this prototype bounded, breeding stops at 40 living animals per home grove, 1,024 living animals globally or 4,096 historical animal records. Home-grove dispersal, hunting, predators and carcass/soil cycles are future work. Animals share the authoritative simulation clock, so day/time-of-day visual controls never skip gestation.

Click an animal or use **Wildlife** to focus/cycle through living animals. The inspector shows species, sex, age, health, hunger, energy, pregnancy, parents, NEAT generation and feasible-action scores. Overview uses white rabbit and tan deer markers; closer views show ears/antlers and simple movement animation. People remain selectable through their numeric inspector or by clicking them.

Validation: 76 automated tests passed, including actual stock consumption after arrival, cross-species/juvenile/kinship breeding restrictions, rabbit and deer births, inherited valid brains, starvation, exact wildlife/pregnancy checkpoint replay and schema-7 migration preserving human assets. A separate ordinary 34-day simulation (64 people; no forced wildlife pregnancies) produced 132 births and five deaths, ending with 300 rabbits and 107 deer. Grass accounting balanced: 8,400 initial + 15,688 grown = 23,254 eaten + 834 remaining; animals also consumed 2,101 berries from the shared human ledger. See `runs/wildlife-34days-debug.json`. This validates the observed scenario, not long-term ecosystem equilibrium or million-entity performance.


## Landscape, site inspection and resource editing (schema 9)

Forest zones now clear the rocky hill and stone patches and leave a 90-unit buffer beyond the settlement housing footprint. On new worlds and migration, a deterministic placement pass moves groves outward together with their associated berry/timber patches, grass and wildlife; stocks, animal lineage and pregnancy state are retained. NPC resource target IDs remain valid. Forest trees use a cached, seed-based random scatter within each grove rather than a ring pattern. Forests do not move continuously during play; later settlement expansion can approach the existing woodland.

New-world gold, silver, copper and iron mines start with **50,000 ore units per deposit** and zero renewal. Existing nonempty deposits gain a one-time 50,000-unit reserve on migration, explicitly credited to initial resource supply. Previously exhausted deposits stay exhausted. At zero stock, the mine graphic and map hit target disappear. Its stable internal record remains so active targets and saves do not acquire invalid IDs; no ore is restored when loading a schema-9 save.

The right panel now uses three tabs:

- **Inspect:** full-height details, wellbeing meters for living entities, a map-pick filter, person-ID selection, and Centre/Follow controls. Select houses, settlement centres, taverns, farms, graveyards, camps, job boards, built roads, forest zones or resource patches. Buildings show their purpose, capacity, progress and costs rather than a raw state dump.
- **Village:** job postings, civic pressures, wildlife and market summaries.
- **Add resources:** choose berries or a gold/silver/copper/iron mine, set its initial amount/capacity, press Place on map, and click clear land. Dragging still pans; Escape cancels. The server rejects water, invalid amounts, overlapping sites/buildings, mines inside forests and berries on rocky ground. Defaults are 600 berries or 50,000 ore; amounts may be 1–1,000,000. Berry sites renew one unit/second up to capacity; mines never renew. These are explicit player world edits and are recorded as introduced supply, not market income or minted currency. Added resources persist in saves.

**Centre** moves the camera to the selected entity or site, with a minimum 3x magnification. **Follow** tracks the selected person/animal using authoritative positions, or stays centred on a building/site; changing selection changes the target. Dragging releases follow. Wheel zoom remains available. The map-pick filter helps select structures in crowded settlements; use Forests to inspect a whole forest zone.

Validation: 79 automated tests passed. New checks verify forest/stone/town clearance, larger finite deposits, resource-editor validation and conservation, physical exhaustion and no refill after save/restore. The TCP check exercises adding ore, authoritative camera-target coordinates and persistence of the added site. Saved worlds are backed up before migration.


## Willow woodland, soil and schools (schema 10)

Woodland placement now strongly favours Willow Reach: eight of every ten grove placements target its surrounding district, while the other two favour Tidewater and Stone Crossing. Placement still clears rocky terrain and leaves a 90-unit buffer beyond the housing footprint. Migration moves associated woodland resources and wildlife without changing their stocks or lineage. Player berry sites on open ground outside the old forest zones stay in place. Ore reserves are not replenished by this migration.

Fertile regions and existing plots that overlap rock are relocated to suitable nearby soil, retaining crop ages, tended days, escrow and any mature rice. New rice plots require suitable soil across their footprint. Tending and crop maturation both reject rocky ground, so a manually invalid plot cannot produce rice.

A village with a living child and housing can have its NEAT council fund a school if it has **20 timber, eight stone and 30 copper**. Materials are committed once; the 30-copper construction reward is escrowed until 25 units of physical labour are complete. Schools appear near the village centre and can be selected in the building inspector. No building appears for free when a child is born.

An open school can post a teacher vacancy for an uneducated child when the treasury can reserve 12 copper. Teachers must be **adults aged 50 or above, or Educated adults**. The base wage is ten copper per completed teaching day; the reserve also covers an educated teacher's rounded-up daily bonus. Normal NEAT choices, physical arrival, energy, hunger and pregnancy rules apply. A class needs a qualified teacher and actual attending pupils together at the school. Classes run during 07:00–17:00, take ten seconds, and can teach up to 32 pupils once per simulated day. Pupils need at least eight seconds of that class to gain a school day; travelling, absent or unstaffed pupils gain no progress.

Children aged **5–17** can choose Attend School in their protected settlement activity set. After **ten attended days**, they receive Educated and a one-time happiness reward. Education is acquired through attendance and is deliberately excluded from biological trait inheritance; other parental traits still combine as before. A new lifelong aspiration, **Become a teacher**, guides teaching choices and completes after the first taught class. Older saves retain existing lifelong aspirations; the new aspiration is available in new worlds and newborns.

Educated individuals earn **15% additional coin entitlement on village-paid jobs**: construction, gravedigging, paid farming, toolmaking, school work and teaching. The bonus does not apply to sales, minting or food-only wages. It is transferred from the paying village, never minted. Fractional copper entitlements accumulate per person and village, so ten two-copper jobs earn three extra copper rather than losing the fraction on each payment. If a treasury temporarily lacks free coins, the unpaid entitlement remains and is settled when funds become available. The person inspector shows the balance. School progress, wage claims, postings and classrooms persist and replay exactly.

The separate school scenario starts with a child aged eight, a qualified 55-year-old teacher candidate, one material-funded house and an existing-money treasury transfer. It does not alter the live world or create free materials/currency. All subsequent school funding, construction, attendance and teaching use ordinary council and individual policies:

```powershell
.\target\release\sim-runner.exe school-demo --agents 64 --steps 39000 --threads 8 --out runs\school-final.json --checkpoint runs\school-final.snapshot
```

Validation covers school costs, qualification boundaries, attendance before graduation, the teacher aspiration, exact fractional wage bonuses, acquired-trait inheritance rules, forest distribution, rock-proof farms, player berry-site preservation and schema-9 migration without mine refills. The ordinary 13-day school fixture completed ten lessons and graduated one child with balanced material/currency accounts. See `runs/school-final.json` and `runs/education-tests-final.log`.


## Zoned towns and private industry

See [URBAN_ECONOMY.md](URBAN_ECONOMY.md) for district thresholds, property and leases, manufacturing recipes, private delivery contracts, NEAT profiles, save migration and the isolated `urban-demo` scenario. Use the viewer's **Zones** toggle to inspect districts.
