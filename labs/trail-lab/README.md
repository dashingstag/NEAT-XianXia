# Trail Lab — slime mold routing on real walks

Demonstrates slime-mold-inspired trail formation on **actual gameplay
walk paths**: a real 10,000-agent world is run for 20 game days by the
Rust simulation, every agent's position is recorded, and this lab
replays those walks through the proposed trail mechanism.

This is not a synthetic dot simulation — the movement data comes from
the same `World::step` the game runs, on generated X4 terrain with
water, markets, camps and resource patches.

## Generate the data (~90 min at 10k agents)

```text
cargo run --release --locked --manifest-path sim/Cargo.toml -p sim-core \
  --example trail_lab -- --agents 10000 --days 20 --map x4
```

Writes `data/trace.bin` (positions every 60 ticks, u16 x10),
`data/terrain.bin`, `data/trail.bin` and `data/meta.json`. Options:
`--agents`, `--days`, `--map x4|classic|x25|x100`, `--seed`,
`--sample-every`, `--out`, `--timing-only`.

Smaller probe: `--agents 2000 --days 3 --timing-only` skips file output.

## Open

```text
npm start        # http://127.0.0.1:4175
npm test         # engine checks
```

## What it shows

- **Trail network vs raw walked cells**: the mold view deposits each
  walked segment with a neighbour splat and decays heat on the game's
  120-tick census cadence; the raw view accumulates the same positions
  forever. The consolidation percentage and the chart quantify "fewer
  footpaths" directly.
- **Mid-path joining**: cyan probe walkers use Jones' 3-sensor steering
  (forward-left/centre/right ~20u ahead) blended with their goal
  bearing. Spawned off-corridor, they slide onto the nearest trunk and
  ride it — no path endpoints involved.
- **Live parameters**: deposit, splat, decay, promote bar and trail pull
  are sliders; rewinding is Reset → Play.

## Mapping to the game

| Lab piece | Game analog |
|---|---|
| deposit only on trips ≥96u | `sample_traffic` counts travelers ≥40u out bound for another market; the router only engages past 96u |
| corridor census line | `Road.traffic`: trips counted between 96u market proximity zones |
| splat neighbours | merges parallel tracks → one trunk |
| decay / cold threshold | `FerryRouteCache::expire` LRU decay |
| promote bar | `publish_traffic` count ≥ 8 → `Road` record |
| probe trail pull | `corridor_or_goal` steering weight, capped below detours |
| 16u cells | between 12u footpath quantization and 22u road-ride radius |

In-run overhead measured by the generator: **~0.09 ms/tick for the
trail layer vs ~92 ms/tick for the sim** at 10k agents on X4 — 0.10% of
step cost, comfortably realtime.

## Files

`engine.mjs` — TrailModel (deposit/splat/decay/stats), trace frame
replay, ProbeWalker 3-sensor steering. `app.js` — loading, rendering,
controls. `server.mjs` — allowlist static server on 4175. `data/` —
generated artifacts (bins are gitignored).
