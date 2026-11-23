# Integrating minimum-cover footpaths into the sim

Plan for bringing the lab-validated approach into `sim-core`. The lab
(`labs/path-cover-lab`) demonstrated: demand-weighted corridor coverage,
lazy greedy set cover with a min-gain floor, candidate dedup, mid-path
join/leave as the coverage rule, and inclination feedback. This file is
the gap list turned into phases, in dependency order.

## Context

- `FerryRouteCache.footpaths` holds proven dry-route segments (48u
  chunks, quantized endpoints, refcounted, capped at 1024) — today a
  visualization cache, never consumed for steering.
- `record_footpath` receives the *full* route polyline at the call site
  and discards it after shredding (`travel.rs`).
- `path_lookahead` / `detour_target` / `road_corridor_target` already
  implement mid-path join via pure pursuit.
- `publish_traffic` runs once per dawn — the natural solve slot.
- `corridor_or_goal` precedence: live detour > built road corridor >
  goal. Footpaths have no branch.

## Phase 0 — Candidate retention

- Extend `record_footpath` to also store the route polyline:
  `route_polylines: BTreeMap<[u32;4], Vec<(f32,f32)>>` keyed like
  `route_segments`, same eviction path as `remove_footpath`, bounded
  (e.g. 4096 entries, reuse `order` LRU).
- Demand model: keep segment refcounts as the demand proxy — each
  candidate corridor's demand = sum of refcounts on its covered segment
  cells. No new per-tick collection cost.
- Cost: near zero — one extra store per successful route lookup.

## Phase 1 — Dawn cover solve

- New `solve_footpath_cover()` called from `publish_traffic`:
  rasterize candidate corridors on a 16u grid, demand = refcount
  totals per cell, lazy greedy over candidates, min-gain floor, drop
  pass — direct port of `labs/path-cover-lab/engine.mjs`.
- Determinism: candidates iterate in `BTreeMap` key order; the heap is
  order-stable on equal gains (tiebreak by key).
- Parameters (lab-tuned starting points): join radius 24u, θ = 0.9,
  minGain = 0.2% of demand mass, dedupe Jaccard ≥ 0.6 on coarse
  endpoints.
- Restructure the cap: selected corridors + bounded evidence pool
  replaces the flat 1024-segment `LIMIT`.

## Phase 2 — Trail storage and consumption

- `trail_paths: Vec<Vec<(f32,f32)>>` stored inside `ferry_routes`
  (soft state, rebuilt from evidence — see Phase 4), exposed to the
  viewer through the existing `cached_footpaths` channel so corridors
  render where segments render today.
- `corridor_or_goal` gains a branch between road corridor and raw
  goal: nearest trail corridor within ~24u whose bearing roughly
  agrees with the goal → project + `path_lookahead` 40u ahead (same
  ride semantics as roads).
- Spatial index: cell → trail ids, rebuilt at dawn with the solve —
  keeps the per-mover lookup ~O(1), comparable to
  `road_corridor_target`.

## Phase 3 — Feedback hygiene and expiry

- Suppress `record_footpath` for routes whose mover is currently
  corridor-riding (or whose segment keys a live trail covers) —
  prevents the cache refilling with the confetti the cover removed.
- Expiry: corridors carrying no fresh demand for N consecutive dawns
  drop back to evidence. Mirrors the lab's rolling-cache mode; reuse
  the refcount decrement machinery.

## Phase 4 — Legality, determinism, persistence

- Store walked polylines as-is — always dry-legal by construction.
  If RDP smoothing is wanted for storage compactness, validate every
  simplified leg with `clear_shot`; on failure keep the original leg.
- `ferry_routes` is `#[serde(skip)]` soft state: keep trails the same —
  they re-derive from deterministic routed lookups within a dawn or two
  of load. No save-layout change, no migration.
- `verify-replay`: scenario covering a mover joining a trail that was
  selected pre-save; confirm identical post-load decisions after
  re-derivation.

## Phase 5 — Validation

- Unit tests: cover determinism, projection join at mid-path,
  ride-suppression, expiry demotion.
- `profile-simulation` A/B on mature-town: instrument
  `route_between`/`compute_detour` call counts and step ms — the lag
  claim is "corridor reuse displaces A* calls," measure it, don't
  assume it.
- Expected cost: dawn solve ~10–100ms per 3000-tick day at 10k agents
  (~0.03ms/tick amortized); per-mover lookup ~ grid read + projection.

## Open decisions

- Trail speed bonus: v1 suggestion is none — positional aid only, so
  trails never outrank built roads. A partial bonus (40ish vs 54) is a
  follow-up.
- Camp/non-market endpoints: corridors are endpoint-agnostic — that's
  the advantage over `Road` records, keep it.
- Whether trails can feed promotion: a corridor whose demand stays
  high across many dawns is a candidate for a real `Road` — the lab's
  "trunk" tier. Optional v2.

## What the lab contributes vs. what must be proven in-situ

The lab proves the *algorithm* selects sensible networks over real
walk data and that inclination feedback consolidates them. It cannot
prove the lag win (that needs the instrumented A/B above) or the
feedback loop (real steering, not bent traces) — those are Phase 2/5
acceptance criteria.
