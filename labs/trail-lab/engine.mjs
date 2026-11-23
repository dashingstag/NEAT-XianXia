// Slime-mold trail dynamics for the trail lab. Pure model logic — no DOM.
//
// The model mirrors the proposed in-game layer: walkers deposit a splat
// kernel where they actually step, heat decays on the game's 120-tick
// census cadence, and cells under the cold threshold are forgotten. The
// "raw" comparison model deposits the same positions but never decays —
// the difference between the two is the consolidation the mechanism
// buys: one growing trunk network instead of fragment bookkeeping.

export class TrailModel {
  constructor(cols, rows, cell) {
    this.cols = cols;
    this.rows = rows;
    this.cell = cell;
    this.heat = new Float32Array(cols * rows);
    this.deposits = 0;
  }

  index(x, y) {
    const cx = Math.floor(x / this.cell);
    const cy = Math.floor(y / this.cell);
    if (cx < 0 || cy < 0 || cx >= this.cols || cy >= this.rows) return -1;
    return cy * this.cols + cx;
  }

  // Centre deposit plus the four orthogonal neighbours. The splat is the
  // diffusion stand-in: adjacent tracks bleed into each other and merge.
  depositAt(x, y, amount, splat) {
    const cx = Math.floor(x / this.cell);
    const cy = Math.floor(y / this.cell);
    if (cx < 0 || cy < 0 || cx >= this.cols || cy >= this.rows) return;
    this.deposits += 1;
    this.heat[cy * this.cols + cx] += amount;
    if (cx + 1 < this.cols) this.heat[cy * this.cols + cx + 1] += splat;
    if (cx > 0) this.heat[cy * this.cols + cx - 1] += splat;
    if (cy + 1 < this.rows) this.heat[(cy + 1) * this.cols + cx] += splat;
    if (cy > 0) this.heat[(cy - 1) * this.cols + cx] += splat;
  }

  // Deposits along the whole walked segment, not just endpoints — the
  // trace samples every N ticks, so interpolation keeps the trail solid.
  // Jumps too long to have been walked (respawns, teleports) deposit at
  // the landing point only, not across the map.
  depositSegment(x0, y0, x1, y1, amount, splat, maxSegment = Infinity) {
    const dx = x1 - x0;
    const dy = y1 - y0;
    if (Math.hypot(dx, dy) > maxSegment) {
      this.depositAt(x1, y1, amount, splat);
      return;
    }
    const steps = Math.max(1, Math.ceil(Math.hypot(dx, dy) / (this.cell * 0.5)));
    for (let s = 0; s <= steps; s++) {
      this.depositAt(x0 + (dx * s) / steps, y0 + (dy * s) / steps, amount, splat);
    }
  }

  decay(rho, cold) {
    const h = this.heat;
    for (let i = 0; i < h.length; i++) {
      const v = h[i] * rho;
      h[i] = v < cold ? 0 : v;
    }
  }

  // Heat at a world position (nearest cell). Used by probe walkers.
  sense(x, y) {
    const i = this.index(x, y);
    return i < 0 ? 0 : this.heat[i];
  }

  stats(promote) {
    let hot = 0;
    let promoted = 0;
    let mass = 0;
    for (let i = 0; i < this.heat.length; i++) {
      const v = this.heat[i];
      if (v > 0) {
        hot += 1;
        mass += v;
        if (v >= promote) promoted += 1;
      }
    }
    return { hot, promoted, mass };
  }
}

// Frame-major u16 trace: frame f, agent i -> [f*agents*2 + i*2], +1.
// Values are world units x10. Returns whether the agent moved.
export function framePair(trace, agents, frame, i, out) {
  const c = (frame * agents + i) * 2;
  const p = ((frame - 1) * agents + i) * 2;
  out.x0 = trace[p] / 10;
  out.y0 = trace[p + 1] / 10;
  out.x1 = trace[c] / 10;
  out.y1 = trace[c + 1] / 10;
  return out.x0 !== out.x1 || out.y0 !== out.y1;
}

// Replays one recorded frame into a model. Only real trips lay trail:
// the game's traffic census counts movers bound for a destination at
// least ~40u out, so a sample whose net displacement is under minTrip
// deposits nothing — milling and socializing don't wear paths.
export function depositFrame(model, trace, agents, frame, params) {
  const pair = { x0: 0, y0: 0, x1: 0, y1: 0 };
  const minTrip = params.minTrip ?? 0;
  for (let i = 0; i < agents; i++) {
    if (framePair(trace, agents, frame, i, pair)
        && Math.hypot(pair.x1 - pair.x0, pair.y1 - pair.y0) >= minTrip) {
      model.depositSegment(pair.x0, pair.y0, pair.x1, pair.y1, params.deposit, params.splat,
        params.maxSegment ?? Infinity);
    }
  }
}

// The game's corridor census, position-only edition: an agent counts a
// trip A->B when it moves from within `radius` of market A to within
// `radius` of market B — the same 96u proximity the game's
// sample_traffic uses to attribute a traveler to a corridor.
export class Census {
  constructor(agents, markets) {
    this.markets = markets;
    this.radius = 96;
    this.near = new Int32Array(agents).fill(-1);
    this.trips = new Map(); // "a<b" -> count
  }

  update(trace, agents, frame) {
    const mk = this.markets;
    for (let i = 0; i < agents; i++) {
      const c = (frame * agents + i) * 2;
      const x = trace[c] / 10;
      const y = trace[c + 1] / 10;
      let now = -1;
      let best = this.radius;
      for (const m of mk) {
        const d = Math.hypot(m.x - x, m.y - y);
        if (d < best) { best = d; now = m.id; }
      }
      const prev = this.near[i];
      if (now !== prev) {
        if (prev >= 0 && now >= 0) {
          const k = prev < now ? `${prev}<${now}` : `${now}<${prev}`;
          this.trips.set(k, (this.trips.get(k) ?? 0) + 1);
        }
        this.near[i] = now;
      }
    }
  }

  top(n = 5) {
    return [...this.trips.entries()].sort((a, b) => b[1] - a[1]).slice(0, n);
  }
}

// Jones-model probe walker: three forward sensors read the trail map,
// the strongest reading steers, and the goal bearing wins when no trail
// is under the sensors. Demonstrates mid-path joining — a walker spawned
// off-trail slides onto the nearest trunk and rides it.
export class ProbeWalker {
  constructor(x, y, gx, gy) {
    this.x = x;
    this.y = y;
    this.gx = gx;
    this.gy = gy;
    this.heading = Math.atan2(gy - y, gx - x);
    this.arrived = false;
    this.distance = 0;
  }

  step(model, params) {
    if (this.arrived) return;
    const { sensorAngle, sensorOffset, speed, trailPull, minHeat } = params;
    const goalA = Math.atan2(this.gy - this.y, this.gx - this.x);
    // Forward-left, forward, forward-right sensors.
    const senses = [-sensorAngle, 0, sensorAngle].map((a) =>
      model.sense(this.x + Math.cos(this.heading + a) * sensorOffset,
                  this.y + Math.sin(this.heading + a) * sensorOffset));
    let steer = goalA;
    let riding = false;
    const best = Math.max(...senses);
    const left = Math.hypot(this.gx - this.x, this.gy - this.y);
    // Ride the trail only while it still leads toward the goal — same as
    // the game's corridor check dropping out near the destination.
    if (best > minHeat && left > sensorOffset) {
      const bi = senses.indexOf(best);
      const trailA = this.heading + [-sensorAngle, 0, sensorAngle][bi];
      const diff = wrapAngle(trailA - goalA);
      if (Math.abs(diff) < Math.PI / 2) {
        steer = goalA + trailPull * diff;
        riding = true;
      }
    }
    const turn = wrapAngle(steer - this.heading);
    // Trail riding turns gently; a beeline to goal pivots freely — a
    // walker can turn in place, the cap only keeps corridor motion smooth.
    this.heading += riding ? Math.max(-0.35, Math.min(0.35, turn)) : turn;
    const move = Math.min(speed, left);
    this.x += Math.cos(this.heading) * move;
    this.y += Math.sin(this.heading) * move;
    this.distance += move;
    if (left < 24) this.arrived = true;
  }
}

function wrapAngle(a) {
  while (a > Math.PI) a -= Math.PI * 2;
  while (a < -Math.PI) a += Math.PI * 2;
  return a;
}

// Default parameters, chosen from the game: cell and promote bar match
// the Rust layer; decay is applied per deposit frame (the lab replays at
// 60-tick samples, the game's census cadence is 120).
export const DEFAULTS = {
  deposit: 0.5,
  splat: 0.25,
  decay: 0.96,
  cold: 0.05,
  promote: 6.0,
  sensorAngle: 0.6,    // ~35 degrees, inside Jones' working band
  sensorOffset: 140,   // roughly one step ahead at replay speed
  speed: 160,          // ~typical mover displacement per 60-tick sample
  trailPull: 0.6,
  minHeat: 0.5,
  maxSegment: 340,  // ~mover top speed across one 60-tick sample
  minTrip: 200,     // long hauls only — must stay under maxSegment or nothing walks this far in one sample
};
