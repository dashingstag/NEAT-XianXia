# Recommended technology stack

## Decision

Use **Rust for the authoritative headless simulation, Godot for the world viewer, and Python for experiments and offline analysis**. Use **packed agent arrays and Rayon** for the first native implementation. Add **wgpu compute** only if measurements show it helps the heterogeneous NEAT workload. Keep simulation, rendering and analytics separately replaceable.

This is a recommendation and architecture decision, not a performance result. The 1.2-million-agent targets in `scale-benchmark-spec.json` still require measurement. Pin compatible stable releases when implementation starts; do not automatically follow library `latest` or development branches.

## Stack by responsibility

| Responsibility | Primary choice | Reason and boundary |
| --- | --- | --- |
| Authoritative simulation | Rust, stable toolchain, Cargo workspace | Explicit data layout, regional ownership, checked resource accounting, native execution and a headless build |
| Entity storage | Typed structure-of-arrays pools, stable index/generation IDs | Compact agent data and predictable iteration; no per-agent heap object, string map or actor thread |
| Parallel computation | One configured Rayon worker pool | Parallel perception, inference and regional work; explicit request/commit phases |
| NEAT | Project-owned Rust genotype/compiler/evaluator, validated against small reference fixtures | Variable topology, innovation/speciation, per-agent state and simulation-specific budgets need direct control |
| Navigation | Regional road/sea graph, cached hierarchical routing, local spatial grids | Reuse routes and search locally; avoid a full pathfinder/physics object per person |
| Viewer | Godot 4.x, compatible with the existing 4.6 project configuration | Scene/UI tooling, world inspection, terrain, instancing and presentation remain outside the core |
| Rendering strategy | Region-batched MultiMesh/2D instancing and distant overlays | Display thousands of visible agents with few scene objects; select individuals by stable ID |
| Initial viewer connection | Versioned binary snapshots and commands over loopback TCP | Independent processes, restartable viewer, no Godot dependency in the core, reusable headless runner |
| Optional in-process bridge | Small Rust GDExtension adapter using godot-rust | Consider after measuring IPC; retain the same batch snapshot/command API |
| Checkpoints | Versioned Serde/Postcard region files, Zstandard compression, atomic manifest | Explicit schema migrations, streaming saves and complete brain/RNG state |
| Metadata | SQLite | Scenario catalog, save index, experiment metadata and reports; not the simulation's per-tick state store |
| Trade analytics | Batched Parquet files and DuckDB queries | Completed-sales composition, provenance, throughput and reproducible offline comparisons |
| Experiments | Python, NumPy, existing test/plot tools; neat-python for small reference cases | Coordinate native runs and examine outputs without a Python call for every agent |
| Tests and profiling | Rust unit/property tests, Criterion, headless workload runner, tracing/Tracy | Conservation/replay first; measure allocations, memory, decision latency and scaling explicitly |
| Optional GPU inference | wgpu + WGSL kernels | Portable desktop compute path; CPU implementation remains the semantic reference and fallback |

This is an implementation stack, not a request to install all components immediately. The first native slice only needs Rust, Rayon, serialization, tests and a command-line runner; Godot and analytics follow the first valid headless benchmark.

## Why Rust for this core

The difficult part is the combination of many independent brains, changing inventories, market reservations, cross-region ownership, births/deaths and reproducible parallel decisions. Rust's ownership model helps express non-overlapping mutable regional work. It does not prevent economic logic errors: transaction tests and authoritative execution still have to prevent double spending or duplicated cargo.

Use one simulation coordinator and region-owned data, with read snapshots for workers. Workers emit requests into bounded buffers. The resolver arbitrates contested stock and commits a deterministic batch. A single Rayon pool runs computation; do not also launch a full thread pool for each region or embed another competing ECS scheduler. Rayon provides data-parallel iterators and configurable task execution, but deterministic transaction order must be designed separately. [Rayon documentation](https://docs.rs/rayon/latest/rayon/).

Store jobs, duty and current tasks as ordinary fields/handles. Use numeric product and action IDs internally; resolve names through catalogs at the viewer boundary. Sparse relationship slots and cold history archives prevent biographies from occupying hot agent arrays. IDs include generation counters so a dead agent's reused slot cannot silently inherit its orders or relationships.

Do not turn every economic action into an async task. Tokio, HTTP and network services may be useful in a future server shell; they are not the million-agent scheduler.

## Why explicit arrays before a general ECS

Most of the first million agents share the same basic state: identity, position, needs, inventory, task and brain. Their hottest operations are predictable numerical passes. Typed arrays make that memory budget and traversal clear without constructing a new general-purpose ECS framework.

`bevy_ecs` is a credible benchmark alternative: it supplies system scheduling and parallel execution. Compare it with the array kernel during the 10k/100k spike, especially if component diversity grows. Adopt it if its workflow and measured performance justify it; do not mix two authoritative entity identities or nest Bevy's and Rayon's pools in the first version. [Bevy ECS scheduling](https://docs.rs/bevy_ecs/latest/bevy_ecs/schedule/).

This is not a claim that a custom implementation is automatically faster. Keep entity-pool code small, testable and domain-specific. Avoid unsafe memory tricks until profiling identifies a real need.

## NEAT implementation boundary

Use a compact Rust genome representation and a compiled execution plan. Compile on birth/mutation rather than during each decision. Batch exact-topology matches where useful while retaining a correct path for mostly unique topologies. Each person retains individual weights and acquired state.

Implement historical gene alignment, compatibility/speciation and structural mutation correctly before adding runtime optimizations. Use `neat-python` to build small reference experiments and fixtures, with explicitly aligned activation functions, aggregation, node ordering and recurrent timing. Different NEAT implementations need not produce identical evolution trajectories. [NEAT-Python documentation](https://neat-python.readthedocs.io/en/latest/).

A general tensor training framework is not the default runtime: the workload is millions of small, variable-topology policies, and NEAT evolves structure and weights. Benchmark padded/topology-bucketed tensors if appropriate rather than assuming standard large-network throughput applies. Evolutionary training and living-world inference remain separate modules.

## Godot is the viewer, not the population database

The existing checkout contains a Godot project configured for 4.6. It is a useful integration starting point, not evidence of a completed frontend. Keep the simulation portable and headless even if the game viewer changes later.

Use GDScript for menus, camera, selections and UI. Native simulation data supplies visible positions, colors, animation tags, health and IDs in batches. Do not create a `Node`, `NavigationAgent`, rigid body or script instance for every simulated person. Physics in the viewer is optional for presentation; authoritative movement and hazards stay in the core.

Godot's MultiMesh batches many instances, but one MultiMesh is spatially indexed as a whole. Split batches by region/material so a single world-sized batch does not render all off-screen instances. Country zoom uses activity/density overlays and transport graphs; zooming never changes who receives a brain decision. [Godot MultiMesh documentation](https://docs.godotengine.org/en/stable/classes/class_multimesh.html).

Start with an out-of-process viewer and a small framed protocol: binary packed arrays for snapshots, versioned commands for pause/step/selection/scenario edits. Never send a million-agent JSON document each frame. At 20,000 visible agents × an illustrative 32-byte snapshot record × 10 snapshots/second, the raw stream is about 6.4 MB/s before framing. Measure encoding, copying, decoding and response latency; the arithmetic is not an achieved throughput result.

If measured IPC overhead matters, add a thin GDExtension wrapper around the same Rust library and exchange packed arrays through double-buffered snapshots. Godot supports native shared libraries through GDExtension; Rust bindings are community-maintained and their engine/API compatibility must be pinned and tested. Do not assume an untested binding/engine pair works. [Godot GDExtension](https://docs.godotengine.org/en/stable/engine_details/engine_api/gdextension/index.html), [godot-rust compatibility](https://godot-rust.github.io/book/toolchain/compatibility.html).

## Persistence and analysis

The active world lives in memory, not in SQL rows updated for each agent each tick. Use integer/fixed-point goods and money, checked arithmetic and authoritative transfer records. Analytics receives committed facts; it cannot write inferred city types back into the world.

Save versioned regional records containing entities, inventories, genomes, neural state, random streams, ownership, events and pending reservations. Stream them to compressed files, then atomically publish a completed manifest. Serde/Postcard do not supply automatic schema migration; write explicit format versions, migration paths, corruption checks and replay fixtures. [Serde](https://serde.rs/), [Postcard wire specification](https://postcard.jamesmunns.com/).

Use SQLite for local metadata rather than operating a database server for a desktop experiment. SQLite is designed for embedded/local application storage. Keep simulation throughput independent of metadata-query latency. [SQLite appropriate uses](https://www.sqlite.org/whentouse.html).

Write completed sales and provenance summaries in batches. Parquet/DuckDB support offline economic analysis; online 30/90/365-day sales profiles use compact in-memory incremental summaries. This prevents a database scan on every NPC decision. Maintain a bounded detailed diagnostic window and a deliberate archival policy, rather than unbounded per-agent logs.

Python launches runs, configures experiments, compares results and produces figures. Prefer the CLI and files initially. Add PyO3 only if Python needs efficient in-memory batch access, never per-agent callbacks in the main loop. [PyO3 guide](https://pyo3.rs/).

## GPU strategy

First establish a correct scalar CPU evaluator and parallel CPU evaluator. Profile candidate scoring, sensing, memory access and transaction resolution separately. A GPU cannot fix a global market scan or a serialized inventory lock.

If inference dominates, prototype wgpu/WGSL compute with resident weights, topology buckets, batched observations and asynchronous results. Measure transfer costs and compilation churn. Keep money/stock commitment on the authoritative resolver. Do not assume wgpu and Godot automatically share GPU buffers or scheduling. [wgpu project](https://wgpu.rs/).

CUDA is a possible later NVIDIA-specific backend when measurements justify the portability cost. FLAME GPU is a useful research comparison but adds a particular agent/message architecture and platform constraints; it is not the default foundation of this economy. Distributed execution is later work, after the single-machine ownership/transaction model is proven.

## Alternatives considered

| Stack | Assessment for this project |
| --- | --- |
| **Rust + packed arrays/Rayon + Godot** | Recommended: core ownership/data-layout control, independent headless research, existing Godot direction; requires a deliberate viewer bridge and NEAT work |
| **C++20 + Flecs + Godot** | Strongest alternative: mature native ecosystem, multithreaded ECS and official Godot C++ bindings; requires careful memory/lifetime discipline and the same economic correctness work |
| **C# + Unity Entities/Burst** | Credible if choosing Unity as the product frontend; strong data-oriented tooling, but changes the engine direction and still needs custom NEAT/economy architecture |
| **Full Bevy** | Credible all-Rust alternative; useful if one-language integration is more valuable than Godot's editor/UI workflow; does not remove the need for bounded queries and an independent core |
| **Python + NumPy/GPU frameworks** | Keep for experiments and reference cases; using it as the authoritative individual-agent loop would require moving the expensive work into native/GPU kernels anyway |
| **Browser TypeScript/WASM/WebGPU** | Useful portable viewer or later deployment target; the desktop core avoids making browser memory/threading/device constraints the first scaling requirement |

Flecs explicitly supports multithreaded systems; it is a serious option, not rejected as incapable. The recommendation is about this project's priorities rather than a claim Rust universally outperforms C++. [Flecs systems](https://www.flecs.dev/flecs/Systems.html).

## Suggested code boundaries

```text
native/
  crates/
    sim-types/       # IDs, units, catalogs, command/snapshot schemas
    sim-core/        # regions, ecology, ownership, jobs, markets, polities
    neat-core/       # genomes, innovations, speciation, compiled CPU inference
    sim-runner/      # headless CLI, scenarios, checkpoints, workload reports
    sim-view/        # viewport extraction and loopback protocol
    sim-gpu/         # optional; added only after the inference benchmark
viewer-godot/        # Godot scenes, camera, instancing and inspector
experiments/         # Python configs, reference NEAT cases, statistical analysis
fixtures/            # small-world conservation and deterministic replay cases
```

`sim-core` and `neat-core` must not depend on Godot, SQLite, a web framework or Python. The Godot viewer and optional browser tools consume snapshots and send validated commands. The existing JavaScript and Python models supply fixtures and migration adapters, not calls inside the native per-agent loop.

## First implementation decision gate

Build a headless 10k-agent slice first: goods, a funded market, regional movement, one production chain, seeded brains and a replayable checkpoint. Then generate distinct NEAT populations at 100k/300k/1.2m and measure the full workload profiles. Add a Godot viewport after the core can produce correct snapshots.

At the 100k gate compare packed arrays/Rayon against a small ECS implementation and benchmark diverse-topology CPU inference against the optional GPU spike if needed. Keep the stack only if measured development costs, memory and throughput support the roadmap. No engine, language or framework by itself guarantees one million economically meaningful agents.
