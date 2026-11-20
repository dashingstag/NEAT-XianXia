# NEAT Xianxia

NEAT Xianxia is a playable settlement simulation where people with evolving neural networks compete for finite resources, form families, take jobs and build trading towns. Rust runs the world. A Godot observatory lets you follow a villager, inspect their choices and watch the economy develop.

The native game and three earlier browser experiments are included here. For a first contribution, pick a reproducible scenario in the simulation, interface, visuals or world generation. [Contributor guide](CONTRIBUTING.md) · [Open issues](https://github.com/dashingstag/NEAT-XianXia/issues) · [Windows verification](https://github.com/dashingstag/NEAT-XianXia/actions/workflows/verify.yml)

## Run from source

Use Python 3.11 or later, Rust/Cargo 1.99.1, Godot 4.6.3 standard and Node 24 for the labs. On Windows the Rust MSVC target needs Visual Studio C++ Build Tools. Put Godot on PATH as `godot` or set `GODOT_BIN` to the full executable path. The first Cargo build fetches the locked dependencies. For offline builds, point `CARGO_HOME` to a populated cache and set `CARGO_NET_OFFLINE=true`.

```text
python tools/dev.py doctor
python tools/dev.py play
python tools/dev.py test all
```

The launcher opens a paused world or resumes the last explicitly saved world. Use **Run** to start and **Save** before closing the viewer. See [player and release instructions](docs/PLAY.md).

On Windows, double-click [launch-frontier.bat](launch-frontier.bat) for a fresh Frontier X4 game or [launch-new-game.bat](launch-new-game.bat) for a fresh Classic game. The launcher prints its profile path under the run's evidence folder. Each launch can run alongside other instances and has its own save. Set `XIANXIA_PYTHON` to a Python 3.11+ executable if you need a specific interpreter.

## Find your area

| Directory | Responsibility |
|---|---|
| sim/crates/neat-core | Neural genomes, mutation, crossover and compiled inference |
| sim/crates/sim-core | Authoritative people, families, relationships, councils, markets, town banks, wildlife, resources and deterministic world updates |
| sim/crates/sim-runner | TCP server, command handling, snapshots, scenario generation and benchmarks |
| viewer | Godot inspector, map, camera, animation and three zoom fidelities |
| world | Named launch and benchmark scenarios. Terrain placement rules currently live in sim-core. |
| contracts and fixtures | Wire and save compatibility, recorded synthetic town, and historical bank-world save |
| labs | Runnable Mind Observatory, Architecture Lab and Willow Crossing experiments |
| tools and .github/workflows | Shared local entry point, protocol checks and Windows CI |
| skills | Project-owned workflows, with discovery copies for supported agent clients |

## Check a change

| Command after `python tools/dev.py` | Evidence |
|---|---|
| `test isolation` | Two distinct backends reset and save their worlds. An intentional interruption closes one while the other loads and advances. |
| `test replay` | Load the old town, save a midpoint, compare full checkpoint SHA-256 digests after two 50-tick continuations. |
| `test protocol` | Fresh-world command exercise, bank replay, relationship reciprocity and regenerated town frame. |
| `test sim`, `test world`, `test labs` | Rust workspace suite, focused terrain checks or all three JavaScript suites. |
| `test viewer`, `preview --fixture town-shop` | Headless inspector and picking assertions, or interactive read-only town preview. |
| `test connection` | Launch an actual headless Godot viewer against its own backend and require a decoded, balanced world. Also works from the extracted runtime ZIP. |
| `bench --scenario mature-town` | Headless step timings on a populated town. Viewer frame time requires a separate graphics run. |
| `package` | Runtime ZIP containing the backend, Godot project, launcher and release metadata. Requires Python and Godot on the target machine. |

Every command prints its own evidence directory under `.work/`. Override with `XIANXIA_RUNS`. Build artifacts follow Cargo's normal target-directory settings. See [agent instructions](AGENTS.md), [compatibility contract](contracts/README.md), and [handoff status](docs/HANDOFF.md). Earlier plans and prototype notes in docs are historical context.

## License

The project uses the [MIT license](LICENSE). Bundled Awesome Copilot resources retain their [upstream license and attribution](third_party/awesome-copilot/PROVENANCE.md).

## Decision and terrain experiments

Run the decision audit from the repository root with `cargo run --release --locked --manifest-path sim/Cargo.toml -p sim-core --example candidate_audit -- fixtures/bank-v12.snapshot`. Each output row describes a six-game-hour window and compares the observed parallel run with an unobserved serial continuation. Sampling alternates between the whole scheduled cohort and people within 120 world units of a market. Count reaching the root candidate limit separately from losing an offer. Domain-specific preselection happens earlier. Retail probes count staffed and stocked shop-ticks, buyer affordability and carrying capacity, restocking opportunities, admitted actions, and completed sales. The mature fixture also runs owner-only samples and separate stock-only, opening-only and combined controls. [Retail availability findings](docs/RETAIL_AVAILABILITY.md)

For a synthetic world with a moved hill, run `cargo run --release --locked --manifest-path sim/Cargo.toml -p sim-runner -- verify --agents 256 --steps 10 --hill-file world/shifted-hill.json --checkpoint .work/shifted/world.snapshot`. Set XIANXIA_PLAYER_DIR to that separate profile directory and use the normal play launcher to view it. Terrain is initialized before forest and farm placement and travels with the checkpoint.
