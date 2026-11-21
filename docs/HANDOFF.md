# Repository handoff

The source import includes all three Rust crates, all save readers, Godot scenes/scripts/shaders and benchmarks, Python protocol checks, all three browser labs, architecture and economy notes, and two synthetic fixtures. At import, the main runtime edits were isolated startup, port discovery and viewer connection. Game rules and simulation data structures came across from the playable prototype.

During local verification, both 50-tick continuations produced the full checkpoint digest `f49faf44acbd22ef0d7d6ab67430b2d65a5237cfad5e49c7c405c3991493facf`. The protocol suite checked 211 active loans and 5,899 relationships in its synthetic worlds. A separate headless Godot process connected to the backend from an extracted runtime ZIP and displayed a balanced snapshot. File comparison confirmed that the NEAT and simulation-core Rust source matched the original checkout byte for byte.

The handoff was imported into the public dashingstag/NEAT-XianXia repository under its existing MIT license. The checked-in synthetic fixtures support repeatable tests. Build caches, temporary evidence and personal saves belong outside the source history.

## Reproduce the handoff

1. Run `python tools/dev.py doctor` and `python tools/sync_skills.py --check`.
2. Run `python tools/dev.py test all`. Review result.json, replay.json, isolation.json and viewer.txt in the printed directory. The first import passed 116 Rust tests, the browser suites, four focused landscape tests, replay, concurrent isolation, bank and relationship protocol tests, and the Godot fixture covering 45 buildings, 9 zones and three fidelity levels on Windows.
3. Run `python tools/dev.py package`, extract the ZIP, and use the included PLAY.md. Use a fresh profile or a copy of a checkpoint to rehearse a deployment.
4. Once hosted, enable the verification workflow on the first branch, confirm the Windows job actually passes and choose required checks in the repository settings. Client-specific skill discovery needs a trial in each team's installed client.

## Independent workstreams at import

- **Logic** — instrument dropped NPC candidates and record the resolved outcome for a watched person. Verify ledger and replay.
- **UI** — extract the person inspector using the existing offline frame for navigation.
- **Visuals** — extract a terrain drawing layer and measure zoom on a graphics-capable machine.
- **World** — replace the duplicated hill geometry with one versioned map definition and verify farm exclusion and site picking.

The original [infrastructure proposal](INFRASTRUCTURE_PLAN.md) records later profiling and release work. Generated maps and on-request map layers are now implemented; a general slow-report channel and a mature million-person soak remain future development. [Benchmarks](BENCHMARKS.md) and [prototype history](PROTOTYPE_HISTORY.md) describe historical measurements and behavior. Use the root [README](../README.md) for current commands and paths.
