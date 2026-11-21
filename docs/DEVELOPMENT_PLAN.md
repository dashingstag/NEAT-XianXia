# Frontier development plan

Drafted 19 November 2026. Goal: grow the settlement simulation into a procedurally generated frontier where ecology, new towns and cultivation emerge.

## Progress log and issues

Branch `frontier/m0-foundations`. All commits keep the user's uncommitted `viewer/main.gd` goods-table change out of staging.

- Cell-hash collision confirmed by `distant_dwellers_do_not_fuse_into_one_founding_group`; fixed with a 4096-stride cell key. Filed as GitHub issue [#17](https://github.com/dashingstag/NEAT-XianXia/issues/17) — resolved by commit `b94ff3a`, close after merge to main.
- Legacy `Agents`/`Market` frozen into `legacy_v13` types; readers v6-v13 use them (`641b0ff`).
- `INPUTS` widened 42 to 48 with reserved sensors 42-47; genome migration shifts hidden/output IDs on pre-15 saves; XXSAVE15 introduced (`1b53e06`).
- Snapshot frame v2: animals moved from per-frame JSON to 16-byte records; viewer and protocol check updated (`2403ce5`). Godot-side decode unverified until a Godot install is available.
- Action IDs reserved on the enum comment: 39-41 hunt/combat, 42-45 taming, 46-49 guild, 50-53 founding, 54-57 qi, 58-63 spare. `sim-runner soak` added (`59a249a`): `--seeds N --steps M` reports per-seed population/trade/ecology plus a world digest.
- Re-scoped: spatial grid and bounds-literal cleanup moved to M1 (natural home with the map rewrite); per-family candidate caps deferred until new action families land in M3.
- Blocked: `python tools/dev.py doctor` fails — Godot executable not installed; all viewer/protocol/interactive checks pending.
- M0 merged to main (`4ba285d`) and pushed; issue #17 closed with resolution comment.
- M1 started on `frontier/m1-generated-world`: `worldmap` module with seeded, order-independent chunked generation (elevation/moisture/qi, terrain classes, MapSize Classic/x4/x25/x100, mundane homeland qi ring, persisted spec only — `0feb01a`). Still to land: generated placement of forests/patches/fertility for non-Classic sizes, scenario size option, viewer streaming/minimap, fog of war, spatial grid + bounds-literal cleanup.
- M1 placement landed (`cfed6bf`): `World::new_with_size` sites markets inside the homeland on fertile/grass terrain, samples patches and mines from terrain classes, `MapSpec` owns the bounds helpers (classic constants preserved), `expand_map_and_food` skips legacy 3x scaling off-Classic, `sim-runner --map classic|x4|x25|x100` for bench/verify/soak. Generated X4 runs clean in soak; 125 tests pass.
- M1 continued on `frontier/m1-scenario-and-fog`: scenario `map` option wired through `sim-runner --map` and `dev.py play --scenario` (opt-in frontier-x4/x25/x100 scenarios in `world/scenarios.toml`; Classic stays default), `reset` preserves map size, and per-settlement fog of war (`Exploration` — per-market visited-chunk sets, persisted, exposed as `explored_chunks` in snapshot metadata). Still open: viewer chunk streaming/minimap/fog rendering (Godot blocked), traded maps / guild use of exploration.
- Wildlife hot loop de-quadratized on `frontier/m1-spatial-grid`: `ForestStats` buckets animals/grass/food/population per forest once per decision round (semantically identical — mating already required same forest), replacing per-animal full scans.

## Player-facing goals

1. Opt-in procedural world up to 100x the current map area (10x per side, 30,720 x 19,200 units; revisit if "100x" meant per side).
2. Predators (fox, wolf, tiger, boar, pheasant) with their own NEAT brains. Weak when young; lethality grows with age, condition and kill experience.
3. Animals near spirit veins accumulate qi and awaken into spirit beasts with abilities (mist step, dread howl, iron hide, thunder pounce, regrowth). Beasts are the world's first cultivators.
4. People hunt animals and predators; weapons, injury, escape and contested carcasses. Taming (pups, feeding), heritable tameness (wolves to dogs), tame breeding and wild/tame crossing.
5. Groups deliberately found camps that grow hamlet to village to town, with council genomes inherited from the parent town.
6. Adventurers' guild halls in towns post escrowed quests: explore, hunt, patrol, escort, rescue.
7. Cultivation starts undiscovered. A seed-specific hidden table maps posture/breathing/place/substance combinations to qi effects; people experiment, remember, teach with copying errors, and write scrolls. The first true technique must follow the first town's guild, not precede it.
8. Player nudges: meteor, spirit vein, spirit herb grove, spirit ore deposit, awaken beast. Each is ledgered as an explicit player addition and chronicled as heaven-sent.
9. Developer-only "Dao Table" tab behind `tools/dev.py play --dev`: hidden table, remembered techniques marked true/partial/superstition, hidden qi affinity, qi map overlay, spark sites, discovery timeline, beast qi. Slow channel, on request only; release builds off. It hides the table from the game UI, not from save-file inspection.

## Constraints found in code

- `INPUTS = 42` is fully used and hidden-node IDs start at 42 (`neat-core`). Widening sensors renumbers every saved genome: one migration, land spare sensors once.
- `legacy_v13` and similar readers deserialise live `Agents`/`Market`; freeze those layouts into legacy modules before adding fields.
- Snapshot builds a full world summary every 100 ms including all animals as JSON; split fast frames from slow reports before the big map (`docs/ARCHITECTURE_REVIEW.md` item 2).
- Action IDs are 6-bit on the wire (63 max); 39 used. Reserve the remaining IDs per milestone; cap candidates per family below the 64 limit.
- Camp cell hash `x/96 + 16*(y/96)` collides across a 29-cell-wide map; confirm with a failing test and file an issue. Spatial grid replaces it.
- Bounds literals (2650, 1890, 2670) are scattered across wildlife, landscape and industry; replace with `WORLD_WIDTH`/`WORLD_HEIGHT` helpers.
- Saves are positional postcard (XXSAVE14). Batch all new persistent fields behind XXSAVE15 in as few version bumps as possible.
- Shared files (`lib.rs`, `urban.rs`, `main.gd`) get one integration owner per change for tick order, shared IDs, wire and save layout; new systems get their own module and viewer panel file (AGENTS.md).

## Blockers to fix first

1. Genome migration to widen `INPUTS`, with reserved spare sensors (dread, qi feeling, predator threat, ...) so it happens once.
2. Freeze `Agents`/`Market` into legacy save readers; introduce XXSAVE15.
3. Fast/slow snapshot split; animals as compact binary records.
4. Headless multi-seed soak runner: N seeds x M simulated years reporting population, births, deaths, trade. Emergent targets (no extinction, discovery timing) become measured tests.

## Milestones

Each ends with a visible result, `python tools/dev.py test all`, replay digest check, save-migration review and a `mature-town` bench comparison.

- **M0 Enablers.** All four blockers above plus bounds helpers, spatial grid, per-family candidate caps, action reservations. Exit: v12 fixture loads and replays; bench not slower.
- **M1 Generated world.** Chunked generator (seeded per chunk, order-independent, unexplored chunks not saved), persisted map with generator version, size presets Classic/x4 (x25/x100 later), mundane homeland ring with no qi, guaranteed spark sites in an expedition ring, viewer chunk streaming and minimap, fog of war per settlement, traded maps.
- **M2 Wilderness.** Species data table, den-based free ranges, fox/wolf/pheasant, carcasses with finite meat, age-scaled lethality. Exit: 5-seed soak, no extinction in 5 years.
- **M3 Humans vs wild.** Open hunting with spear/bow tools, kill/escape/injury outcomes, group hunts, meat/pelts/fangs in trade, dread sensor; then taming, tameness trait, tame breeding. Exit: contested-carcass test; lineage in inspector.
- **M4 Towns and guild.** Settlement tiers, founding parties recruited via tavern relationships, council genome inheritance, ruins on failure, guild hall and quests. Exit: first town forms across seeds and its guild pays out an explore quest.
- **M5 Qi layer.** Qi density field, spirit ore, vein flow rates, beast accumulation and awakening with 2 abilities, player nudges, dev qi overlay. Exit: awakening only outside homeland unless nudged.
- **M6 Discovery.** Hidden per-seed table, `Experiment` action with feeling sensors, per-person technique memory, oral teaching with errors, scrolls, tavern rumours, chronicle with first-discoverer naming, full Dao Table tab. Exit: first technique never precedes the first guild and follows within a year on most seeds.
- **M7 Scale and depth.** x25/x100 presets, coarse distant-region simulation if profiling requires it (replay-exactness risk), sects founded by masters, deviation injuries, hybrids, remaining abilities/species, meteor-pattern decoding.

## Order and parallelism

M0 sequential, single owner. After M1, wilderness (M2-M3) and society (M4) split into separate branches; viewer panels alongside. M5-M6 need both merged. Deferred from the first pass: hybrids, tiger/boar, 3 of 5 abilities, meteor decoding, sects, x100 preset.

## Open design notes

- "Discovery within a year of the first guild" and predator-prey coexistence are tuning targets, not scripts; budget soak-and-tune time in M2, M4 and M6.
- Qi is conserved: veins have flow rates, spirit stones are consumed, cores drop from killed beasts.
- Suspected camp cell-hash collision: confirm with a failing test in M0, then replace with the spatial grid.

## M8 Water routes: stranded reports, ports and shipping

Blocked movers become a demand signal that villages answer with
infrastructure. Ordered phases; each ends with a replay-exact Classic
suite and a generated-map scenario test.

### P1 Stranded reporting

- Transient stuck detection: `walk_toward` already returns nothing when
  `steer` fails; track per-agent progress in a `#[serde(skip)]` map
  (`index -> (x, y, tick)`). An agent with an active Travel/Gather/Rest
  destination whose position has not moved >= 24 units in ~600 ticks is
  stranded at water.
- A stranded agent records a `WaterBlock { x, y, target_x, target_y,
  tick, reports }` on their home market (deduped by 96u cells, capped
  at 64 entries, persisted -> XXSAVE16). On their next arrival inside
  the settlement radius the report is delivered (physical reporting);
  agents then retry or fall back per existing action logic.
- Exit test: seed a walker facing deep water; assert a block is
  recorded and delivered on home arrival.

### P2 Ports

- `Port { id, market, x, y, pair, progress, ships: Vec<u32> }` persisted
  on World (part of the same v16 bump).
- Far-shore resolution: from the block position toward the blocked
  target, sample `terrain_at` every 32u up to 4096u; first `buildable`
  land is the landing site, nudged dry. No land => report expires
  unreached (open ocean or unreachable island).
- Village planning: a zoned market with a delivered block and no port
  within 300u of the site posts two build jobs on the existing
  `urban.jobs` board with a new kind (5 "Shipwright" -> reuses the
  builder pipeline: wage escrow, timber+stone material delivery,
  phases). Port build ~= 24 wood, 8 stone.
- Far-side catch: the landing port cannot be built until the shore is
  reachable. If the routing grid finds a land path, a work expedition
  (convoy) is commissioned to walk there. Otherwise the far port job is
  queued until the first ship exists (see P3) and then ferries 2
  builders + materials across.
- Ports render as dock sites in `map_layers` metadata + viewer glyph
  (new `port.png` in the xianxia glyph set).

### P3 Ships and assemblies

- `Ship { id, port, members: Vec<u32> (cap 100), x, y, state }`
  persisted; built by a "Build ship" carpenter job costing 40 timber
  from market stock at the port.
- Demand: an agent whose target market/site is unreachable over land
  but reachable via a port pair joins the port assembly list; when >= 8
  wait (or an expedition convoy commits), the ship departs with up to
  100 aboard.
- Movement: ship = convoy-on-water. Same member-pinning math, but its
  `steer` variant refuses land cells instead of water. Speed 1.5x walk,
  night slowdown applies.
- Disembark: members scatter at the landing dock; the ship becomes a
  persistent shuttle: it returns when >= 8 queue at the far port or
  after an idle timeout (one ferry per pair initially).
- Viewer: ship glyph (`ship.png`, junk-rigged sail) drawn from
  snapshot `ships[]`; assembly shown as a crowd at the dock.
- Exit tests: stuck NPC -> block -> jobs -> port pair -> ship built ->
  passengers disembark on the far shore within a scenario run; Classic
  untouched (no blocks, no ports).

### P4 Animals respect water

- `wildlife.rs` movement (single `a.x += dir*step` site) goes through a
  `walk_animal` that refuses `Terrain::Water` with axis-slide steering;
  rivers stay crossable (they already swim-read fine at reduced speed).
- Flee targets and mate/wander goals are clamped dry via `dry_ground`
  when they land in water.
- Exit test: an animal driven toward open water never stands on Water
  over a long chase; classic forests unchanged.

### Ordering and dependencies

- P1+P4 are independent (separate commits ok). P2 needs P1. P3 needs
  P2. Save bump to XXSAVE16 lands once with P2 (ports + water_blocks).
- Deliberately out: fares/tolls, ship combat, fishing from boats,
  multi-ship fleets, ocean currents. Guild/quests later reuse ports as
  quest waypoints (M4), and spirit islands arrive with the qi layer (M5).

### P5 Routed detours for stalled movers (shipped)

The shoreline steering ladder caps at +-165 degrees, so a walker pressed
against a real lake oscillated in place until the 600-tick report path
fired — crowds pooling at water edges were movers that never attempted
to walk around the obstacle.

- After 240 stalled ticks under the 24-unit progress probe,
  compute_detour runs the road-router A* from the mover's position and
  stores the waypoint path in water.detours (agent idx, or
  0x8000_0000|slot for convoys). walk_toward and departed convoys steer
  toward a pure-pursuit lookahead 16 units along the path.
- Arclength progress is monotonic (Detour.progress): a mover shoved off
  the corridor aims forward instead of flipping between adjacent segment
  projections. Recompute retry is 480 ticks for live detours, 2400 for
  unreachable markers.
- Truly unreachable targets leave an empty-path backoff entry and fall
  through to the pending-report / port / ferry chain unchanged — the
  detour only replaces waiting where walking genuinely works.
- Router fixes this exposed: the A* goal cell can no longer be
  force-passed (it emitted flooded centres as waypoints) — it snaps to
  the nearest passing cell with a clear final leg instead; the start
  cell keeps force-pass since the mover stands there. clear_shot samples
  at 4-unit mover-step resolution (16-unit strides straddled thin bands
  steering still hit). steer gains a 180-degree candidate so pocketed
  movers back out.

Exit test: stalled_walkers_route_around_lakes — a pinned walker gets
waypoints and arrives on the far shore; the e2e chain re-qualified its
span against the surveyed endpoints.

### Remaining water-edge gaps and their handling

- Island destinations (no dry path at all) are intentionally slow: they
  wait on the report -> port -> ferry chain, then queue at the dock.
  Agents standing in a port queue are waiting for a ship by design.
- Queue hygiene: a queued rider whose target becomes land-reachable, or
  who starves/dies, should be dropped — currently only ship_step prunes
  dead/unfit riders at board time. Add a periodic queue sweep inside
  update_water_reports if crowding at piers shows up in play.
- Animal detours are not implemented — animals get water bounds only
  (P4). If wildlife piles at shores, reuse compute_detour with a
  wildlife-namespaced key rather than duplicating the follower.
- Ferry legs that fail mid-route (ship beached on a headland the water
  A* missed) currently stall with riders aboard; a ship-side progress
  probe mirroring track_stranded is the planned guard.

### P6 Shoreline crowd root causes (fix batch)

A 3000-agent diagnostic (diag_shore_stall, 4000 ticks) traced the
visible shoreline pile to five compounding causes, each fixed:

- post_farm validated only the fertility-region centre; the four +-25
  unit grid cells could land in open water. One wet farm kept winning
  best_farm as the nearest plot and drew hundreds of workers to pin at
  its shore. Cells are now nudged dry and must pass farm_ground
  themselves; best_farm also refuses non-buildable plots so legacy wet
  farms stop attracting.
- The 256-entry carried-report cap gated the whole stall probe: once it
  saturated, no walker accumulated stall time, computed a detour, or
  filed a report again. The cap now bounds only new pushes.
- Carried reports never expired and bearers stayed pinned at the same
  shore, so deliveries were ~0. Reports now expire after ~2 days, and a
  carrier's personal_destination resolves to its home market until the
  report lands — the crowd physically walks home instead of pooling.
- ForageMeal and CareFamily required a 5-unit arrival even for fish
  patches placed in open water; they now share Gather's 24-unit bank
  reach for water destinations.
- The stall probe measured displacement only: sliding 24 units along a
  shore reset it forever. A probe point now resets only when the mover
  is measurably closer to its target; same for convoy probes.
- Orphaned detour entries (dead walkers, released convoy slots) were
  never evicted; the 512-entry detour map filled and every new strander
  silently failed compute_detour. update_water_reports now sweeps them.

Diag result (X4 seed 7, 3000 agents): unmoved agents at tick 4000 fell
579 -> 233 (remaining are resting/waiting/convoy-pinned), wet farms 1 ->
0, wet-action targets 199 -> 0, delivered blocks ~0 -> 47.
