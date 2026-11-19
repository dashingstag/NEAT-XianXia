# Measured prototype checks

These are local measurements of the implemented prototype, not the full acceptance profile in the scale plan.

Machine: AMD Ryzen 5 7500F, 6 physical cores / 12 logical processors, approximately 31.6 GiB RAM. Native runs used eight Rayon workers and release optimization. Rust/rayon did the simulation and neural inference on the CPU. The visible Godot viewer uses the available GPU for drawing; GPU neural inference is not implemented.

| Headless workload | Simulated duration | Step-loop wall time | p95 tick | Living agents at end |
| --- | ---: | ---: | ---: | ---: |
| 10,000 agents / 1,000 ticks | 100 seconds | 0.403 s | 0.589 ms | 10,000 |
| 100,000 agents / 100 ticks | 10 seconds | 0.284 s | 3.355 ms | 100,000 |
| 1,200,000 agents / 100 ticks | 10 seconds | 3.336 s | 37.307 ms | 1,200,000 |

The large run executed 12 million scheduled individual decisions. Brains averaged approximately 52.36 retained connection genes, with four initial hidden features and a small number of structural variations. The program performs one scheduled decision per living agent per simulated second, scores the currently feasible candidates, and executes movement/actions at ten ticks per simulated second. Detailed reports expose the actual counts.

Initialization is separate: the 1.2-million-agent world took about 5.93 seconds to create. A separate ten-tick memory probe observed a peak process working set of 4,677,996,544 bytes (about 4.36 GiB), sampled every 100 ms. This is process working set, not a measurement of whole-system memory or a two-hour soak peak.

All measured runs finished with balanced food, timber, stone, fish and coin accounts. The longer 10,000-agent scenario generated completed sales, cross-settlement shipments, housing blocks and a funded road. The short large-population run is mainly an allocation/inference/action throughput check and does not establish long-term economic behavior.

## Correctness and integration

- 15 Rust tests: seeded brains; neutral historical node splits; cycle rejection; homologous crossover; bounded valid mutation; integer conservation under competition; serial/parallel equality; observational labels; exact checkpoint replay; bounded representative render sampling; duplicate-road prevention; and framed-command validation.
- Real TCP protocol test: commands and binary snapshots passed; save/restore plus fifty continued ticks reproduced the original state.
- Godot 4.6.3: project parsed and ran headlessly, then the native window displayed live populations, individual candidate scores, market profiles and a balanced ledger through the real Rust connection.
- Combined launcher: confirmed that it reuses the running backend/viewer instead of creating conflicting copies.

## Evidence

- `runs/verify-10000.json` and `runs/verify-10000.log`
- `runs/bench-100000.json`
- `runs/bench-1200000-short.json`
- `runs/bench-1200000-memory-probe.json` and `runs/bench-1200000-memory.json`
- `runs/comparison.csv`
- `runs/protocol-check.json`
- `runs/hardware-cpu.json` and `runs/hardware-memory.json`
- `runs/build-manifest.json`

## What remains unproven

The standard target budget was around 256 connection genes and much more topology diversity. This prototype uses much smaller controllers. Candidate counts vary with feasibility rather than forcing eight evaluations for every person. The test did not include a two-hour soak, million-agent recurrent state/evolution, dense-city contention, ship/port congestion, production-scale region ownership migration, streamed checkpoints, or measured viewer FPS. Its three-market model is intentionally simpler than the planned economy.

Those differences matter: this is a working native architecture and a promising short scale measurement, not a claim that the complete million-agent Xianxia game is finished.
