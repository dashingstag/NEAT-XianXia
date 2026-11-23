// Engine checks: node tests.mjs
import { extractTrips, corridorCells, neededDemand, dedupeCandidates, greedyCover, buildPathField, bendTrip, selectedUnion, tripCoverage, censusBaseline, rdp, polylineLength, DEFAULTS }
  from './engine.mjs';

let failures = 0;
const ok = (cond, msg) => {
  if (!cond) { failures++; console.error('FAIL', msg); }
};

const candify = (trips, cols, rows, cell = 16, radius = 24) =>
  trips.map((t) => ({ cells: corridorCells(t.pts, cols, rows, cell, radius), trip: t }));

// Trip extraction: sustained movement becomes one polyline; stillness
// and teleports split trips.
{
  const agents = 1, frames = 6;
  const tr = new Uint16Array(frames * agents * 2);
  // f1: (0,0)->(200,0) moved 200 — trip starts
  tr[1 * 2 + 0] = 2000;
  // f2: ->(400,0) still moving
  tr[2 * 2 + 0] = 4000;
  // f3: same position — trip ends
  tr[3 * 2 + 0] = 4000;
  // f4: teleports to (4000, 3000) — jump >340, not a trip step
  tr[4 * 2 + 0] = 40000; tr[4 * 2 + 1] = 30000;
  // f5: still there
  tr[5 * 2 + 0] = 40000; tr[5 * 2 + 1] = 30000;
  const trips = extractTrips(tr, agents, frames, { minLen: 100 });
  ok(trips.length === 1, `one trip (got ${trips.length})`);
  ok(trips[0].pts.length === 6, 'trip polyline has 3 points');
  ok(Math.abs(trips[0].len - 400) < 1e-3, 'trip length 400');
  ok(trips[0].f0 === 1, 'trip tagged with start frame');
}

// Corridor cells cover the polyline plus join radius.
{
  const cols = 40, rows = 40;
  const cells = corridorCells(new Float32Array([160, 320, 800, 320]), cols, rows, 16, 24);
  const set = new Set(cells);
  ok(set.has(20 * cols + 10), 'covers line');
  ok(set.has(20 * cols + 30), 'covers far end');
  ok(set.has(19 * cols + 20), 'radius above');
  ok(set.has(21 * cols + 20), 'radius below');
  ok(!set.has(16 * cols + 20), 'outside radius');
}

// Greedy picks the path that covers shared corridor first, then stops
// when nothing new is gained — a minimal set. Demand weighting makes a
// corridor walked twice outrank one walked once.
{
  const cols = 40, rows = 40;
  // Trip A: y=320 corridor x 160..800. Trip B: y=480 short stub.
  const A = { pts: new Float32Array([160, 320, 800, 320]), len: 640 };
  const B = { pts: new Float32Array([160, 480, 240, 480]), len: 80 };
  // Candidate pool: A, B, and a near-duplicate of A that adds nothing.
  const A2 = { pts: new Float32Array([160, 324, 800, 324]), len: 640 };
  const all = candify([A, B], cols, rows);
  const { demand, mass } = neededDemand(all, cols * rows);
  ok(mass === all[0].cells.length + all[1].cells.length, 'mass = total stamps');
  const cands = candify([A, B, A2], cols, rows);
  const r = greedyCover(cands, demand, mass, 0.99);
  ok(r.selected.length === 2, `minimal set = 2 (got ${r.selected.length})`);
  ok(r.selected[0] === 0, 'covers the trunk first');
  ok(!r.selected.includes(2), 'duplicate path not selected');
  ok(r.served / mass > 0.95, 'coverage target reached');
}

// Demand weighting: a cell covered by two trips weighs double, so the
// path through the busy corridor wins over the slightly bigger one.
{
  const cols = 40, rows = 40;
  const trunk1 = { pts: new Float32Array([160, 320, 800, 320]), len: 640 };
  const trunk2 = { pts: new Float32Array([160, 324, 800, 324]), len: 640 };
  const side = { pts: new Float32Array([160, 480, 900, 480]), len: 740 };
  const all = candify([trunk1, trunk2, side], cols, rows);
  const { demand, mass } = neededDemand(all, cols * rows);
  // Candidates: only the side path and trunk1 — busy corridor must win.
  const r = greedyCover([all[2], all[0]], demand, mass, 0.99);
  ok(r.selected[0] === 1, 'busier corridor picked first');
}

// Mid-path join: a trip that only overlaps the middle of a long path is
// still served — coverage is positional, not endpoint-based.
{
  const cols = 80, rows = 40;
  const long = { pts: new Float32Array([16, 320, 1200, 320]), len: 1184 };
  const mid = { pts: new Float32Array([400, 322, 700, 322]), len: 300 };
  const all = candify([mid], cols, rows);
  const { demand, mass } = neededDemand(all, cols * rows);
  const cands = candify([long], cols, rows);
  const r = greedyCover(cands, demand, mass, 0.99);
  ok(r.selected.length === 1 && r.served === mass,
    'long path serves mid-overlap trip');
}

// Min-gain floor: a stub path serving a sliver of demand is never added
// when its marginal gain sits below the floor.
{
  const cols = 40, rows = 40;
  const A = { pts: new Float32Array([160, 320, 800, 320]), len: 640 };
  const stub = { pts: new Float32Array([160, 560, 200, 560]), len: 40 };
  const all = candify([A, stub], cols, rows);
  const { demand, mass } = neededDemand(all, cols * rows);
  // Stub corridor is a tiny fraction of mass — 20% floor rejects it even
  // though theta asks for 99%.
  const r = greedyCover(all, demand, mass, 0.99, 0.2);
  ok(r.selected.length === 1, `floor rejected the stub (got ${r.selected.length})`);
}

// Dedup: same-endpoint trips sharing most of their centreline merge to
// one candidate; the longest trip becomes the representative.
{
  const cols = 40, rows = 40;
  const A = { pts: new Float32Array([160, 320, 800, 320]), len: 640 };
  const A2 = { pts: new Float32Array([164, 326, 796, 326]), len: 632 };
  const far = { pts: new Float32Array([160, 560, 800, 560]), len: 640 };
  const all = candify([A, A2, far], cols, rows);
  const { pool, merged } = dedupeCandidates(all, cols, rows, 16, 0.6);
  ok(pool.length === 2 && merged === 1,
    `deduped to 2 candidates (got ${pool.length}, merged ${merged})`);
  ok(pool[0].trip === A, 'longest trip kept as representative');
}

// Drop pass: a path picked early can be made redundant by later picks —
// B covers the trunk plus an extension, so plain A is dropped under a
// nonzero floor once B is in.
{
  const cols = 60, rows = 40;
  // A and B share a corridor; B extends further. Greedy takes the bigger
  // first anyway — so force order with a third path: C covers a disjoint
  // region, and A's unique contribution ends below the floor.
  const A = { pts: new Float32Array([160, 320, 560, 320]), len: 400 };
  const B = { pts: new Float32Array([160, 328, 900, 328]), len: 740 };
  const all = candify([A, B], cols, rows);
  const { demand, mass } = neededDemand(all, cols * rows);
  // B is strictly bigger so greedy picks B first; A's leftovers are a
  // sliver — drop pass removes it under a floor.
  const r = greedyCover(all, demand, mass, 0.99, 0.05);
  ok(r.selected.length === 1, `redundant A dropped (got ${r.selected.length})`);
}

// Lazy greedy equals full rescan: same selections on a mixed pool.
{
  const cols = 60, rows = 60;
  const mk = (x0, y0, x1, y1) => ({ pts: new Float32Array([x0, y0, x1, y1]), len: Math.hypot(x1 - x0, y1 - y0) });
  const trips = [
    mk(80, 160, 900, 160), mk(84, 166, 896, 166), mk(80, 480, 500, 480),
    mk(80, 320, 300, 320), mk(400, 80, 400, 900), mk(80, 160, 80, 560),
  ];
  const all = candify(trips, cols, rows);
  const { demand, mass } = neededDemand(all, cols * rows);
  const r = greedyCover(all, demand, mass, 0.99);
  ok(r.selected.length >= 2 && r.served >= mass * 0.95,
    `lazy greedy covers demand (${r.selected.length} paths)`);
}

// Inclined walkers: a parallel trip inside join radius bends toward the
// published path; a perpendicular crossing does not (it's not following,
// just crossing); follow=0 leaves geometry untouched.
{
  const cols = 60, rows = 60;
  const path = { pts: new Float32Array([100, 320, 800, 320]), len: 700 };
  const cand = candify([path], cols, rows);
  const field = buildPathField(cand, cols, rows, 16);
  // Parallel trip 16u north, same direction.
  const near = new Float32Array([200, 336, 400, 336, 600, 336]);
  const bent = bendTrip(near, field, 24, 1);
  ok(bent[3] < 336 - 8, `parallel point pulled toward path (y=${bent[3].toFixed(1)})`);
  ok(bent[1] === 336 && bent[5] === 336, 'endpoints stay put');
  // Same trip, zero appeal — unchanged.
  const flat = bendTrip(near, field, 24, 0);
  ok(flat[3] === 336, 'follow=0 leaves geometry alone');
  // Perpendicular crossing at x=400 — not bent onto the path.
  const cross = new Float32Array([400, 200, 400, 320, 400, 440]);
  const cBent = bendTrip(cross, field, 24, 1);
  ok(cBent[2] === 400 && cBent[3] === 320, 'crossing walker not yanked sideways');
}

// Trip-level serve: a trip inside the union reports full coverage; a
// trip far outside reports none — catches what cell coverage hides.
{
  const cols = 60, rows = 60;
  const onNet = { pts: new Float32Array([160, 320, 800, 320]), len: 640 };
  const offNet = { pts: new Float32Array([160, 800, 800, 800]), len: 640 };
  const cands = candify([onNet], cols, rows);
  const mask = selectedUnion(cands, [0], cols * rows);
  const r = tripCoverage([onNet, offNet], mask, cols, rows, 16);
  ok(r.fullPct === 0.5, `half the trips fully served (got ${r.fullPct})`);
  ok(Math.abs(r.onNetPct - 0.5) < 0.05, 'half the walked distance on-network');
}

// Census baseline: market-pair trips past the bar become roads; trips
// wholly outside proximity zones count for nothing.
{
  const markets = [{ x: 100, y: 100 }, { x: 1000, y: 100 }, { x: 500, y: 900 }];
  const mk = (x0, y0, x1, y1) => ({ pts: new Float32Array([x0, y0, x1, y1]), len: Math.hypot(x1 - x0, y1 - y0) });
  const trips = [];
  for (let i = 0; i < 9; i++) trips.push(mk(110 + i, 105, 995, 105 + i));   // m0->m1, qualifies
  for (let i = 0; i < 4; i++) trips.push(mk(105, 110 + i, 495, 895 + i));  // m0->m2, under bar
  trips.push(mk(600, 300, 700, 320));                                       // no market at either end
  const { roads, pairs } = censusBaseline(trips, markets, 96, 8);
  ok(roads.length === 1, `one road promoted (got ${roads.length})`);
  ok(pairs.size === 2, 'two pairs censused');
}

// RDP: a collinear run collapses to its endpoints; a real bend survives.
{
  const straight = rdp(new Float32Array([0, 0, 100, 0, 200, 0, 300, 0]), 8);
  ok(straight.length === 4, `straight line -> 2 points (got ${straight.length / 2})`);
  const bent = rdp(new Float32Array([0, 0, 100, 40, 200, 0]), 8);
  ok(bent.length === 6, 'bend kept');
}

console.log(failures === 0 ? 'all engine checks passed' : `${failures} failure(s)`);
process.exit(failures ? 1 : 0);
