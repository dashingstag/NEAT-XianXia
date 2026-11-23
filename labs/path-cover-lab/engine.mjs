// Minimum-path-cover selection over cached trip paths.
//
// Model: every recorded trip is a "cached path" — a proven walked
// route, the analog of the game's route_segments cache. A footpath
// network is a SUBSET of those paths. A traveller is served when their
// trip lies within ride distance of a selected path anywhere along it —
// they may join or leave mid-path, so coverage is measured by proximity
// along the whole corridor, never by endpoints.
//
// Objective: minimum number of selected paths covering >= theta of
// walked demand. Exact set cover is NP-hard; greedy (pick the path
// covering the most still-unserved demand) is the standard ~ln(n)
// approximation. It runs lazy: marginal gains only shrink, so stale
// heap entries are valid upper bounds and only popped candidates get
// rescanned — identical selections to a full rescan each round.

export const TERRAIN_CELL_SIZE = 16;

// Pulls per-agent trips out of the frame trace: a trip is a maximal run
// of samples whose per-sample displacement is a plausible walked step
// ([minStep, maxJump] — above jitter, below teleports). Trips shorter
// than minLen are discarded as local movement, not travel.
export function extractTrips(trace, agents, frames, opts = {}) {
  const { minStep = 24, maxJump = 340, minLen = 192, fromFrame = 1 } = opts;
  const trips = [];
  for (let i = 0; i < agents; i++) {
    let pts = null, f0 = 0;
    const flush = () => {
      if (pts && pts.length >= 4) {
        let len = 0;
        for (let k = 2; k < pts.length; k += 2) {
          len += Math.hypot(pts[k] - pts[k - 2], pts[k + 1] - pts[k - 1]);
        }
        if (len >= minLen) trips.push({ pts: Float32Array.from(pts), len, f0 });
      }
      pts = null;
    };
    for (let f = Math.max(1, fromFrame); f < frames; f++) {
      const c = (f * agents + i) * 2;
      const p = ((f - 1) * agents + i) * 2;
      const x = trace[c] / 10, y = trace[c + 1] / 10;
      const px = trace[p] / 10, py = trace[p + 1] / 10;
      const d = Math.hypot(x - px, y - py);
      if (d >= minStep && d <= maxJump) {
        if (!pts) { pts = [px, py, x, y]; f0 = f; }
        else pts.push(x, y);
      } else {
        flush();
      }
    }
    flush();
  }
  return trips;
}

// Cells whose centre is within `radius` of the polyline: stamps every
// ~half-cell along each segment, expanding k cells in each direction.
export function corridorCells(pts, cols, rows, cell = TERRAIN_CELL_SIZE, radius = 24) {
  const k = Math.max(0, Math.ceil(radius / cell));
  const set = new Set();
  const stamp = (x, y) => {
    const cx = Math.floor(x / cell);
    const cy = Math.floor(y / cell);
    for (let dy = -k; dy <= k; dy++) {
      for (let dx = -k; dx <= k; dx++) {
        const nx = cx + dx, ny = cy + dy;
        if (nx >= 0 && ny >= 0 && nx < cols && ny < rows) set.add(ny * cols + nx);
      }
    }
  };
  for (let s = 0; s < pts.length - 2; s += 2) {
    const [ax, ay, bx, by] = [pts[s], pts[s + 1], pts[s + 2], pts[s + 3]];
    const steps = Math.max(1, Math.ceil(Math.hypot(bx - ax, by - ay) / (cell * 0.5)));
    for (let t = 0; t <= steps; t++) stamp(ax + ((bx - ax) * t) / steps, ay + ((by - ay) * t) / steps);
  }
  return Uint32Array.from(set);
}

// Demand-weighted requirement: each stamp of a trip's corridor is ~one
// cell of walked distance inside join range, so a cell's demand is the
// trip count through it and mass is total walked cell-visits. Coverage
// is then a fraction of walked distance, not just walked area — busy
// corridors outrank a path somebody took once.
export function neededDemand(cands, size) {
  const demand = new Float32Array(size);
  let mass = 0;
  for (const c of cands) {
    for (const idx of c.cells) { demand[idx] += 1; mass++; }
  }
  return { demand, mass };
}

const cellOf = (x, y, cols, cell) => Math.floor(y / cell) * cols + Math.floor(x / cell);

// Near-duplicate merge: trips that begin and end in the same coarse
// endpoint cells and share >= thresh centreline cells are one observed
// route walked many times. Keeps the longest trip as representative —
// the demand array still counts every trip, so merged trips keep their
// weight. Shrinks the candidate pool before the solve.
export function dedupeCandidates(cands, cols, rows, cell = TERRAIN_CELL_SIZE, thresh = 0.6) {
  const qc = cell * 3;
  const qcols = Math.ceil((cols * cell) / qc) + 1;
  const endKey = (x, y) => Math.floor(y / qc) * qcols + Math.floor(x / qc);
  const buckets = new Map();
  for (const c of cands) {
    const p = c.trip.pts;
    const a = endKey(p[0], p[1]);
    const b = endKey(p[p.length - 2], p[p.length - 1]);
    const key = a < b ? `${a}:${b}` : `${b}:${a}`;
    const bucket = buckets.get(key);
    if (bucket) bucket.push(c);
    else buckets.set(key, [c]);
  }
  const jaccard = (s, t) => {
    const [small, big] = s.size < t.size ? [s, t] : [t, s];
    let both = 0;
    for (const v of small) if (big.has(v)) both++;
    return both / (s.size + t.size - both);
  };
  const pool = [];
  let merged = 0;
  for (const bucket of buckets.values()) {
    const reps = [];
    for (const c of bucket) {
      // Cached on the trip — sweeps and daily replays dedupe it again.
      const sig = c.trip.sig ??
        (c.trip.sig = new Set(corridorCells(c.trip.pts, cols, rows, cell, 0)));
      const rep = reps.find((r) => jaccard(sig, r.sig) >= thresh);
      if (rep) {
        merged++;
        if (c.trip.len > rep.cand.trip.len) rep.cand = c;
      } else {
        reps.push({ sig, cand: c });
      }
    }
    for (const r of reps) pool.push(r.cand);
  }
  return { pool, merged };
}

// Greedy minimum path cover over demand. `minGainFrac` (fraction of
// mass) is a floor: when the best remaining path buys less than that,
// stop — keeps the solver from chasing the long tail with junk paths.
// After the greedy picks, a drop pass removes any selected path whose
// UNIQUE demand is below the floor — later picks can make early ones
// redundant. Candidates and demand share the same corridor
// rasterization, so a selected path serves any trip within `radius` of
// ANY part of it — mid-path join/leave for free.
export function greedyCover(candidates, demand, mass, theta = 0.95, minGainFrac = 0) {
  const covered = new Uint8Array(demand.length);
  const used = new Uint8Array(candidates.length);
  let selected = [];
  const coverage = [];
  let served = 0;
  const target = mass * theta;
  const floor = Math.max(mass * minGainFrac, 1e-9);

  const marginal = (c) => {
    let g = 0;
    for (const idx of candidates[c].cells) if (!covered[idx]) g += demand[idx];
    return g;
  };

  // Max-heap of [storedGain, candidate]. Stored gains are computed once
  // and only decrease as cells get covered, so they're upper bounds:
  // pop, recompute the true marginal, and only accept it once it still
  // tops the heap — then it provably beats every other true gain.
  const heap = [];
  const push = (item) => {
    heap.push(item);
    let i = heap.length - 1;
    while (i > 0) {
      const p = (i - 1) >> 1;
      if (heap[p][0] >= item[0]) break;
      heap[i] = heap[p];
      i = p;
    }
    heap[i] = item;
  };
  const pop = () => {
    const top = heap[0];
    const last = heap.pop();
    if (heap.length) {
      let i = 0;
      for (;;) {
        const l = 2 * i + 1, r = l + 1;
        let m = i;
        if (l < heap.length && heap[l][0] > heap[m][0]) m = l;
        if (r < heap.length && heap[r][0] > heap[m][0]) m = r;
        if (m === i) break;
        heap[i] = heap[m];
        i = m;
      }
      heap[i] = last;
    }
    return top;
  };

  for (let c = 0; c < candidates.length; c++) {
    let g = 0;
    for (const idx of candidates[c].cells) g += demand[idx];
    if (g > 0) push([g, c]);
  }

  while (served < target && heap.length) {
    const [, c] = pop();
    if (used[c]) continue;
    const g = marginal(c);
    if (heap.length && g < heap[0][0]) { push([g, c]); continue; }
    if (g <= floor) break;
    used[c] = 1;
    selected.push(c);
    for (const idx of candidates[c].cells) {
      if (!covered[idx]) { covered[idx] = 1; served += demand[idx]; }
    }
    coverage.push(served / mass);
  }

  // Drop pass: a selected path whose unique demand is under the floor
  // was made redundant by later picks.
  let dropped = 0;
  if (selected.length > 1) {
    const coverCount = new Uint16Array(demand.length);
    for (const ci of selected) {
      for (const idx of candidates[ci].cells) coverCount[idx]++;
    }
    const keep = new Set();
    for (const ci of selected) {
      let unique = 0;
      for (const idx of candidates[ci].cells) if (coverCount[idx] === 1) unique += demand[idx];
      if (unique <= floor) {
        dropped++;
        served -= unique;
        for (const idx of candidates[ci].cells) coverCount[idx]--;
      } else {
        keep.add(ci);
      }
    }
    selected = selected.filter((ci) => keep.has(ci));
  }

  return { selected, coverage, served, dropped };
}

// Feedback layer: once the dawn pass selects footpaths, later walkers
// are inclined to follow them. Selected paths are stamped into a grid
// of {x, y, dirx, diry} samples so trip points can find the nearest
// path point within join range.
export function buildPathField(cands, cols, rows, cell = TERRAIN_CELL_SIZE) {
  const grid = new Map();
  for (const c of cands) {
    const p = c.trip.pts;
    for (let s = 0; s + 3 < p.length; s += 2) {
      const [ax, ay, bx, by] = [p[s], p[s + 1], p[s + 2], p[s + 3]];
      const len = Math.hypot(bx - ax, by - ay) || 1;
      const dx = (bx - ax) / len, dy = (by - ay) / len;
      const steps = Math.max(1, Math.ceil(len / (cell * 0.5)));
      for (let t = 0; t <= steps; t++) {
        const x = ax + ((bx - ax) * t) / steps, y = ay + ((by - ay) * t) / steps;
        const key = Math.floor(y / cell) * cols + Math.floor(x / cell);
        let b = grid.get(key);
        if (!b) grid.set(key, (b = []));
        b.push([x, y, dx, dy]);
      }
    }
  }
  return { grid, cell, cols, rows };
}

// Pulls interior trip points toward the nearest path stamp within
// `radius`, when the path runs in a compatible direction (|dot|>0.5 —
// footpaths serve both ways, but a crossing walker isn't "following").
// `follow` is the inclination strength: 0 = ignore paths, 1 = snap on.
// Endpoints stay put — travellers join and leave mid-path.
export function bendTrip(pts, field, radius, follow) {
  if (!field || follow <= 0) return pts;
  const { grid, cell, cols } = field;
  const k = Math.max(1, Math.ceil(radius / cell));
  const r2 = radius * radius;
  const out = Float32Array.from(pts);
  for (let s = 2; s < pts.length - 2; s += 2) {
    const x = pts[s], y = pts[s + 1];
    let hx = pts[s + 2] - pts[s - 2], hy = pts[s + 3] - pts[s - 1];
    const hl = Math.hypot(hx, hy) || 1;
    hx /= hl; hy /= hl;
    const cx = Math.floor(x / cell), cy = Math.floor(y / cell);
    let best = null, bd2 = r2;
    for (let dy = -k; dy <= k; dy++) {
      for (let dx = -k; dx <= k; dx++) {
        const bucket = grid.get((cy + dy) * cols + (cx + dx));
        if (!bucket) continue;
        for (const st of bucket) {
          const d2 = (st[0] - x) * (st[0] - x) + (st[1] - y) * (st[1] - y);
          if (d2 < bd2 && Math.abs(st[2] * hx + st[3] * hy) > 0.5) { bd2 = d2; best = st; }
        }
      }
    }
    if (best) {
      out[s] += (best[0] - x) * follow;
      out[s + 1] += (best[1] - y) * follow;
    }
  }
  return out;
}

// Union mask of the selected corridors — what the published network
// physically covers.
export function selectedUnion(cands, selectedIdx, size) {
  const mask = new Uint8Array(size);
  for (const ci of selectedIdx) {
    for (const idx of cands[ci].cells) mask[idx] = 1;
  }
  return mask;
}

// Trip-level serve quality: the fraction of each trip's centreline that
// lies inside the selected union — how much of a journey the network
// actually carries. Cell-level demand coverage can hide trips that are
// only half served; a traveller needs most of a journey to be on-path.
export function tripCoverage(trips, mask, cols, rows, cell = TERRAIN_CELL_SIZE) {
  let full = 0, onNet = 0, total = 0;
  for (const t of trips) {
    const cl = corridorCells(t.pts, cols, rows, cell, 0);
    let cov = 0;
    for (const idx of cl) if (mask[idx]) cov++;
    const frac = cl.length ? cov / cl.length : 0;
    if (frac >= 0.8) full++;
    onNet += frac * t.len;
    total += t.len;
  }
  return {
    fullPct: trips.length ? full / trips.length : 0,
    onNetPct: total ? onNet / total : 0,
  };
}

// The game's own promotion rule, run as a baseline solver for
// comparison: census trips between market proximity zones (proxR, the
// game's 96u); each unordered pair reaching `bar` trips gets a road,
// represented by the longest observed trip between them.
export function censusBaseline(trips, markets, proxR = 96, bar = 8) {
  const near = (x, y) => {
    for (let i = 0; i < markets.length; i++) {
      const m = markets[i];
      const dx = x - m.x, dy = y - m.y;
      if (dx * dx + dy * dy <= proxR * proxR) return i;
    }
    return -1;
  };
  const pairs = new Map();
  for (const t of trips) {
    const p = t.pts;
    const a = near(p[0], p[1]);
    const b = near(p[p.length - 2], p[p.length - 1]);
    if (a < 0 || b < 0 || a === b) continue;
    const key = a < b ? a * 1024 + b : b * 1024 + a;
    const e = pairs.get(key);
    if (!e) pairs.set(key, { count: 1, best: t });
    else { e.count++; if (t.len > e.best.len) e.best = t; }
  }
  const roads = [];
  for (const e of pairs.values()) if (e.count >= bar) roads.push(e.best);
  return { roads, pairs };
}

// Ramer–Douglas–Peucker: the canonical smoothed polyline a game would
// actually store, dropping points within eps of the chord.
export function rdp(pts, eps = 16) {
  const n = pts.length / 2;
  if (n <= 2) return Float32Array.from(pts);
  const keep = new Uint8Array(n);
  keep[0] = keep[n - 1] = 1;
  const stack = [[0, n - 1]];
  while (stack.length) {
    const [a, b] = stack.pop();
    const ax = pts[2 * a], ay = pts[2 * a + 1];
    const dx = pts[2 * b] - ax, dy = pts[2 * b + 1] - ay;
    const len = Math.hypot(dx, dy) || 1;
    let worst = -1, wd = eps;
    for (let i = a + 1; i < b; i++) {
      const px = pts[2 * i] - ax, py = pts[2 * i + 1] - ay;
      const d = Math.abs(dx * py - dy * px) / len;
      if (d > wd) { wd = d; worst = i; }
    }
    if (worst > 0) { keep[worst] = 1; stack.push([a, worst], [worst, b]); }
  }
  const out = [];
  for (let i = 0; i < n; i++) if (keep[i]) out.push(pts[2 * i], pts[2 * i + 1]);
  return Float32Array.from(out);
}

export function polylineLength(pts) {
  let len = 0;
  for (let k = 2; k < pts.length; k += 2) {
    len += Math.hypot(pts[k] - pts[k - 2], pts[k + 1] - pts[k - 1]);
  }
  return len;
}

export const DEFAULTS = {
  minStep: 24,      // per-sample displacement that counts as walking
  maxJump: 340,     // above this is a teleport, trip splits
  minLen: 192,      // trips shorter than this are local movement
  radius: 24,       // join distance, ~ the game's 22u road-ride radius
  theta: 0.95,      // walked-demand coverage target
  minGainPct: 0.2,  // stop/drop when a path buys < this % of demand
  dedupeThresh: 0.6,// centreline Jaccard that merges duplicate trips
  followPct: 60,    // inclination: how strongly walkers bend to paths
  windowDays: 3,    // trips counted from the last N days
};
