# Path Cover Lab — minimum footpath set over cached trips

A different approach from the slime-mold lab: instead of growing a trail
field, this lab **selects the minimum set of footpaths from the paths
already walked** — the analog of choosing which cached routes become
the region's footpath network.

Data is shared with `../trail-lab`: real recorded walks from a
10,000-agent, 20-day run of the actual Rust simulation.

## Open

```text
npm start        # http://127.0.0.1:4176  (data served from ../trail-lab/data)
npm test         # engine checks
```

## The problem

- Every trip extracted from the trace is a **cached path** — a proven
  walked route. Trips are maximal runs of plausible per-sample steps
  (24–340u per 60-tick sample) with length ≥ `minLen`.
- The **demand** is demand-weighted: every stamp of a trip's corridor
  counts as ~one cell of walked distance, so a cell's demand is the
  trip count through it. Busy corridors outrank one-off routes.
- A **footpath network** = a subset of the cached paths. A traveller is
  served when their route lies within `radius` of a selected path
  anywhere along it — **joining or breaking mid-path is the coverage
  rule**, not an exception: only position matters, never endpoints.
- **Objective**: fewest selected paths covering ≥ θ of walked demand.

Exact set cover is NP-hard; the lab runs a **lazy greedy**
approximation — marginal gains only shrink, so a max-heap of stale
upper bounds skips the full rescan each round while selecting exactly
what naive greedy would. Before solving, near-duplicate trips
(same coarse endpoints, ≥60% shared centreline) merge into one
candidate; after solving, a drop pass removes picks whose unique
demand fell under the `min gain` floor, and the same floor stops the
greedy before it chases the long tail.

## Reading the output

- `selected footpaths` — the minimal set size; compare against the
  candidate count (post-dedup) to see consolidation directly.
- `demand/path` — serving efficiency of the chosen network in
  walked cell-visits.
- The coverage curve shows diminishing returns: the knee is where the
  "real" network ends and rare variants begin.
- Window slider scopes to the last N days (default 3) — trips before
  the window are excluded, matching "the last 3 days".
- **Sim daily** replays the run from day 1, solving at each dawn over
  the accumulated cache. With `path appeal` > 0, trips that start the
  next day bend toward the selected network inside join radius —
  walkers are inclined to follow published paths, so demand
  consolidates onto the trunks and the network converges faster than
  the static replay shows.

## Mapping to the game

| Lab piece | Game analog |
|---|---|
| trip polyline | `route_segments` entry — a cached successful route |
| radius 24u | 22u road-ride radius / corridor join distance |
| θ coverage target | traffic bar for road promotion, expressed as area |
| greedy pick order | trunk-first network growth — early picks are roads, late are lanes |
| solve time on the page | the offline batch a "promote footpaths" pass would run at dawn |

The candidate pool being the observed trips themselves is the
conservative version: production could add merged/smoothed corridor
polylines as extra candidates and cover more area per pick.
