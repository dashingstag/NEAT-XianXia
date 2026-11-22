# Testing

Run these from the repository root with Python 3.11 or later.

| Aim | Command |
| --- | --- |
| Check the installed toolchain | `python testing/run.py doctor` |
| Run the full verification suite | `python testing/run.py test all` |
| Run one suite | `python testing/run.py test sim` (also `world`, `viewer`, `connection`, `protocol`, `replay`, `isolation`, `labs`) |
| Measure a mature simulation | `python testing/run.py bench --scenario mature-town` |
| Run a multi-seed Frontier test | `python testing/run.py soak --map x4 --seed 42 --seeds 3 --steps 30000` |
| Run tests while editing | `python testing/run.py test all --background` |
| Queue and inspect runs | `python testing/run.py queue add test all` and `python testing/run.py queue list` |
| Preview the town-shop fixture | `python testing/run.py preview --fixture town-shop` |

The runner prints the evidence directory for each run. Background and queued
runs use their own source snapshots. Existing `tools/dev.py` commands use the
same runner and continue to work.

## Where the checks live

- [Protocol, bank and relationship checks](protocol/) use runner-owned servers.
- [Godot fixture and benchmark scripts](../viewer/bench/) use the viewer's
  `res://` paths. For a standalone render benchmark, run
  `godot --path viewer --script res://bench/render_bench.gd`.
- [Rust simulation](../sim/crates/sim-core/) and [NEAT](../sim/crates/neat-core/)
  keep their tests in their crates so Cargo discovers them.
- [Browser lab tests](../labs/) stay with their JavaScript modules.
- [Saved-world and viewer fixtures](../fixtures/) provide known inputs.
- [Developer experiments](../sim/crates/sim-core/examples/) are opt-in Cargo examples.
