import { extractTrips, neededDemand, corridorCells, dedupeCandidates, greedyCover, buildPathField, bendTrip, selectedUnion, tripCoverage, censusBaseline, rdp, polylineLength, DEFAULTS }
  from './engine.mjs';

const TERRAIN_COLORS = [
  '#1d3557', '#6c757d', '#5c6b4f', '#2d4a1e', '#7d9b4a', '#556b2f', '#3a6ea8',
];

const state = {
  meta: null, trace: null, terrainImage: null,
  params: { ...DEFAULTS },
  trips: [],
  candidates: [],
  result: null,
  showTrips: true,
  showBase: false,
  serve: null,
  base: null,
  busy: false,
};

const $ = (id) => document.getElementById(id);
const canvas = $('map');
const ctx = canvas.getContext('2d');

async function load() {
  state.meta = await (await fetch('data/meta.json')).json();
  state.trace = new Uint16Array(await (await fetch('data/trace.bin')).arrayBuffer());
  const terrain = new Uint8Array(await (await fetch('data/terrain.bin')).arrayBuffer());
  const m = state.meta;
  canvas.width = m.width / 4;
  canvas.height = m.height / 4;
  buildTerrain(terrain);
  $('status').textContent =
    `${m.agents} agents · ${m.days} days · ${m.frames} samples · real walk paths`;
  wire();
  rebuild();
}

function hex(h) { return [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16)); }

function buildTerrain(terrain) {
  const { cols, rows } = state.meta.terrain;
  const off = document.createElement('canvas');
  off.width = cols; off.height = rows;
  const c = off.getContext('2d');
  const img = c.createImageData(cols, rows);
  for (let i = 0; i < terrain.length; i++) {
    const rgb = hex(TERRAIN_COLORS[terrain[i]] ?? '#000');
    img.data.set([...rgb, 255], i * 4);
  }
  c.putImageData(img, 0, 0);
  state.terrainImage = off;
}

// Rebuild trips + solve for the current window/parameters.
function rebuild() {
  if (state.busy || !state.meta) return;
  state.busy = true;
  state.dailyDay = 0;
  const { meta, params } = state;
  const framesPerDay = Math.round(3000 / meta.sample_every);
  const fromFrame = Math.max(1, meta.frames - params.windowDays * framesPerDay);
  setStatus('extracting trips…');
  setTimeout(() => {
    const t0 = performance.now();
    state.trips = extractTrips(state.trace, meta.agents, meta.frames,
      { minStep: params.minStep, maxJump: 340, minLen: params.minLen, fromFrame });
    const cell = 16;
    const cols = Math.ceil(meta.width / cell);
    const rows = Math.ceil(meta.height / cell);
    state.grid = { cols, rows, cell };
    // Candidates = the cached paths themselves.
    const all = state.trips.map((t) => ({
      cells: corridorCells(t.pts, cols, rows, cell, params.radius),
      trip: t,
    }));
    // Demand counts every observed trip, even ones deduped out of the pool.
    const { demand, mass } = neededDemand(all, cols * rows);
    state.mass = mass;
    const { pool, merged } = dedupeCandidates(all, cols, rows, cell, params.dedupeThresh);
    state.candidates = pool;
    state.merged = merged;
    setStatus(`covering ${state.trips.length} trips → ${pool.length} candidates (${merged} merged)…`);
    const t1 = performance.now();
    state.result = greedyCover(pool, demand, mass, params.theta, params.minGainPct / 100);
    state.solveMs = performance.now() - t0;
    const mask = selectedUnion(pool, state.result.selected, cols * rows);
    state.serve = tripCoverage(state.trips, mask, cols, rows, cell);
    state.base = baselineOf(state.trips, cols, rows, cell, params.radius, demand, mass);
    paint();
    state.busy = false;
  }, 30);
}

// The game's promotion rule as the control: census trips between market
// zones, promote pairs past the traffic bar, score it against the same
// demand so the comparison is apples-to-apples.
function baselineOf(trips, cols, rows, cell, radius, demand, mass) {
  const { roads, pairs } = censusBaseline(trips, state.meta.markets);
  const cands = roads.map((t) => ({ cells: corridorCells(t.pts, cols, rows, cell, radius), trip: t }));
  const mask = selectedUnion(cands, cands.map((_, i) => i), cols * rows);
  let served = 0;
  for (let i = 0; i < mask.length; i++) served += demand[i] * mask[i];
  const serve = tripCoverage(trips, mask, cols, rows, cell);
  return { roads, count: roads.length, cov: mass ? served / mass : 0, pairs: pairs.size, serve };
}

function setStatus(s) { $('solve-status').textContent = s; }

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// Replays the run as the game would see it: from day 1, solving at each
// dawn over every cached path accumulated so far. The feedback loop:
// each dawn's selections become a path field, and trips starting the
// NEXT day bend toward those paths within join radius — individuals are
// inclined to follow the published network, so demand consolidates
// onto it day over day.
async function runDaily() {
  if (state.busy || !state.meta) return;
  state.busy = true;
  const { meta, params } = state;
  const fpd = Math.round(3000 / meta.sample_every);
  setStatus('extracting all trips…');
  await sleep(30);
  const t0 = performance.now();
  const raw = extractTrips(state.trace, meta.agents, meta.frames,
    { minStep: params.minStep, maxJump: 340, minLen: params.minLen, fromFrame: 1 });
  const cell = 16;
  const cols = Math.ceil(meta.width / cell);
  const rows = Math.ceil(meta.height / cell);
  state.grid = { cols, rows, cell };
  const follow = params.followPct / 100;
  const rolling = $('rolling').checked;   // expire cache like ferry_routes does
  let pool = [];        // accumulated candidates (bent as-walked geometry)
  let field = null;     // last dawn's selected network
  state.dailyStats = [];
  for (let d = 1; d <= meta.days; d++) {
    const fresh = raw.filter((t) => t.f0 >= (d - 1) * fpd && t.f0 < d * fpd);
    for (const t of fresh) {
      const pts = field ? bendTrip(t.pts, field, params.radius, follow) : t.pts;
      pool.push({
        cells: corridorCells(pts, cols, rows, cell, params.radius),
        trip: pts === t.pts ? t : { pts, len: t.len, f0: t.f0 },
      });
    }
    // Rolling expiry: cache only holds the last `windowDays` of trips —
    // paths must keep being walked to stay in the problem.
    if (rolling) pool = pool.filter((c) => c.trip.f0 >= (d - params.windowDays) * fpd);
    if (!pool.length) continue;
    state.trips = pool.map((c) => c.trip);
    const { demand, mass } = neededDemand(pool, cols * rows);
    state.mass = mass;
    const { pool: sol, merged } = dedupeCandidates(pool, cols, rows, cell, params.dedupeThresh);
    state.candidates = sol;
    state.merged = merged;
    const ts = performance.now();
    state.result = greedyCover(sol, demand, mass, params.theta, params.minGainPct / 100);
    const ms = performance.now() - ts;
    const sel = state.result.selected;
    field = buildPathField(sel.map((i) => sol[i]), cols, rows, cell);
    const mask = selectedUnion(sol, sel, cols * rows);
    state.serve = tripCoverage(state.trips, mask, cols, rows, cell);
    state.base = baselineOf(state.trips, cols, rows, cell, params.radius, demand, mass);
    const cov = mass ? state.result.served / mass : 0;
    state.dailyDay = d;
    state.dailyStats.push({ d, trips: pool.length, pool: sol.length, paths: sel.length, cov, ms, bp: state.base.count });
    paint();
    setStatus(`day ${d}/${meta.days}: ${state.result.selected.length} paths serve ${(cov * 100).toFixed(1)}%`);
    await sleep(250);
  }
  state.solveMs = performance.now() - t0;
  drawDailyChart();
  setStatus(`daily replay done: ${state.dailyStats.length} dawns solved in ${(state.solveMs / 1000).toFixed(1)}s`);
  state.busy = false;
}

function paint() {
  const { meta, grid } = state;
  ctx.drawImage(state.terrainImage, 0, 0, canvas.width, canvas.height);
  const sx = canvas.width / meta.width;
  const sy = canvas.height / meta.height;
  // All observed trips, faint — the demand the network must serve.
  if (state.showTrips) {
    ctx.strokeStyle = 'rgba(200,210,220,0.10)';
    ctx.lineWidth = 1;
    ctx.beginPath();
    for (const t of state.trips) {
      ctx.moveTo(t.pts[0] * sx, t.pts[1] * sy);
      for (let k = 2; k < t.pts.length; k += 2) ctx.lineTo(t.pts[k] * sx, t.pts[k + 1] * sy);
    }
    ctx.stroke();
  }
  // Game-rule baseline network, cyan — the control the cover is compared to.
  if (state.showBase && state.base) {
    ctx.strokeStyle = 'rgba(80,200,230,0.75)';
    ctx.lineWidth = 1.8;
    ctx.beginPath();
    for (const t of state.base.roads) {
      ctx.moveTo(t.pts[0] * sx, t.pts[1] * sy);
      for (let k = 2; k < t.pts.length; k += 2) ctx.lineTo(t.pts[k] * sx, t.pts[k + 1] * sy);
    }
    ctx.stroke();
  }
  // Selected footpaths, drawn in greedy order: early picks = core trunks.
  // Rendered as the smoothed canonical polyline the game would store.
  if (state.result) {
    const n = state.result.selected.length;
    state.result.selected.forEach((ci, rank) => {
      const pts = rdp(state.candidates[ci].trip.pts, state.params.radius);
      const heat = 1 - rank / Math.max(1, n);
      ctx.strokeStyle = `rgba(255,${Math.round(140 + 90 * heat)},60,${0.5 + 0.5 * heat})`;
      ctx.lineWidth = 2.2;
      ctx.beginPath();
      ctx.moveTo(pts[0] * sx, pts[1] * sy);
      for (let k = 2; k < pts.length; k += 2) ctx.lineTo(pts[k] * sx, pts[k + 1] * sy);
      ctx.stroke();
    });
  }
  for (const m of meta.markets) {
    ctx.fillStyle = '#ffd166';
    ctx.beginPath();
    ctx.arc(m.x * sx, m.y * sy, 4, 0, Math.PI * 2);
    ctx.fill();
    ctx.fillStyle = '#fff';
    ctx.font = '10px sans-serif';
    ctx.fillText(m.name, m.x * sx + 6, m.y * sy + 3);
  }
  stats();
}

function stats() {
  const { result, trips, mass, meta } = state;
  if (!result) { $('stats').textContent = 'no solve yet'; return; }
  const n = result.selected.length;
  const netLen = result.selected.reduce((s, ci) => s + state.candidates[ci].trip.len, 0);
  const tripLen = trips.reduce((s, t) => s + t.len, 0);
  const cov = mass ? result.served / mass : 0;
  const scope = state.dailyDay
    ? `day <b>${state.dailyDay}</b> of ${meta.days}d (cumulative, appeal ${state.params.followPct}%) · `
    : `window: last <b>${state.params.windowDays}d</b> of ${meta.days}d · `;
  $('stats').innerHTML =
    scope +
    `trips <b>${trips.length}</b> (${(tripLen / 1000).toFixed(1)}ku walked) ` +
    `→ <b>${state.candidates.length}</b> candidates (${state.merged} merged)<br>` +
    `walked demand <b>${(mass / 1000).toFixed(0)}k</b> cell-visits · served <b>${(cov * 100).toFixed(1)}%</b> ` +
    `(target ${(state.params.theta * 100).toFixed(0)}%)<br>` +
    `selected footpaths <b>${n}</b>` +
    `${result.dropped ? ` <span title="redundant after later picks">(−${result.dropped} dropped)</span>` : ''}` +
    ` = ${(netLen / 1000).toFixed(2)}ku network ` +
    `(${(netLen / Math.max(1, tripLen) * 100).toFixed(1)}% of walked length)<br>` +
    `demand/path <b>${(mass / Math.max(1, n) / 1000).toFixed(1)}k</b> · ` +
    `solve ${(state.solveMs / 1000).toFixed(2)}s`;
  if (state.serve) {
    $('stats').innerHTML +=
      `<br>fully-served trips <b>${(state.serve.fullPct * 100).toFixed(0)}%</b> · ` +
      `walked distance on-network <b>${(state.serve.onNetPct * 100).toFixed(0)}%</b>`;
  }
  if (state.base) {
    const b = state.base;
    $('stats').innerHTML +=
      `<br>game census rule: <b>${b.count}</b> roads · ` +
      `<b>${(b.cov * 100).toFixed(1)}%</b> demand · ` +
      `<b>${(b.serve.fullPct * 100).toFixed(0)}%</b> trips fully served`;
  }
  setStatus(`solved: ${n} paths serve ${(cov * 100).toFixed(1)}% of walked demand`);
  drawCoverageCurve();
}

// Paths-per-day (gold) vs the game-rule baseline (cyan) and demand
// served (blue) from the daily replay.
function drawDailyChart() {
  const c = $('chart');
  const g = c.getContext('2d');
  g.clearRect(0, 0, c.width, c.height);
  const s = state.dailyStats ?? [];
  if (s.length < 2) return;
  const maxPaths = Math.max(1, ...s.map((v) => Math.max(v.paths, v.bp ?? 0)));
  const X = (i) => 10 + (i / (s.length - 1)) * (c.width - 20);
  const Yp = (v) => c.height - 8 - (v / maxPaths) * (c.height - 16);
  const Yc = (v) => c.height - 8 - v * (c.height - 16);
  g.strokeStyle = '#ffd166';
  g.beginPath();
  s.forEach((v, i) => { const x = X(i), y = Yp(v.paths); i ? g.lineTo(x, y) : g.moveTo(x, y); });
  g.stroke();
  g.strokeStyle = '#50c8e6';
  g.beginPath();
  s.forEach((v, i) => { const x = X(i), y = Yp(v.bp ?? 0); i ? g.lineTo(x, y) : g.moveTo(x, y); });
  g.stroke();
  g.strokeStyle = '#7fb2ff';
  g.beginPath();
  s.forEach((v, i) => { const x = X(i), y = Yc(v.cov); i ? g.lineTo(x, y) : g.moveTo(x, y); });
  g.stroke();
  g.fillStyle = '#8fa1b5';
  g.font = '10px sans-serif';
  g.fillText('paths/day (gold) · game rule (cyan) · demand served (blue)', 10, 12);
}

// Parameter sweep: appeal × radius daily replays, terminal values
// tabulated — this is what turns slider fiddling into an experiment.
async function runSweep() {
  if (state.busy || !state.meta) return;
  state.busy = true;
  const { meta, params } = state;
  const fpd = Math.round(3000 / meta.sample_every);
  const cell = 16, cols = Math.ceil(meta.width / cell), rows = Math.ceil(meta.height / cell);
  setStatus('sweep: extracting trips…');
  await sleep(30);
  const raw = extractTrips(state.trace, meta.agents, meta.frames,
    { minStep: params.minStep, maxJump: 340, minLen: params.minLen, fromFrame: 1 });
  const radii = [16, 24, 48, 96];
  const appeals = [0, 50, 100];
  const rolling = $('rolling').checked;
  const table = [];
  for (const r of radii) {
    const row = { r, cells: [] };
    for (const a of appeals) {
      setStatus(`sweep: join ${r}u · appeal ${a}%…`);
      await sleep(0);
      let pool = [], field = null, res = null, mass = 0;
      for (let d = 1; d <= meta.days; d++) {
        for (const t of raw) {
          if (t.f0 < (d - 1) * fpd || t.f0 >= d * fpd) continue;
          const pts = field ? bendTrip(t.pts, field, r, a / 100) : t.pts;
          pool.push({
            cells: corridorCells(pts, cols, rows, cell, r),
            trip: pts === t.pts ? t : { pts, len: t.len, f0: t.f0 },
          });
        }
        if (rolling) pool = pool.filter((c) => c.trip.f0 >= (d - params.windowDays) * fpd);
        if (!pool.length) continue;
        const { demand, mass: m } = neededDemand(pool, cols * rows);
        mass = m;
        const { pool: sol } = dedupeCandidates(pool, cols, rows, cell, params.dedupeThresh);
        res = greedyCover(sol, demand, m, params.theta, params.minGainPct / 100);
        field = buildPathField(res.selected.map((i) => sol[i]), cols, rows, cell);
      }
      row.cells.push(res ? { paths: res.selected.length, cov: mass ? res.served / mass : 0 } : null);
    }
    table.push(row);
  }
  renderSweep(table, appeals);
  setStatus('sweep done');
  state.busy = false;
}

function renderSweep(table, appeals) {
  const head = appeals.map((a) => `<th>appeal ${a}%</th>`).join('');
  const body = table.map((row) =>
    `<tr><td>join ${row.r}u</td>` +
    row.cells.map((c) => `<td>${c ? `${c.paths} paths · ${(c.cov * 100).toFixed(0)}%` : '—'}</td>`).join('') +
    '</tr>').join('');
  $('sweep-out').innerHTML = `<table><tr><th></th>${head}</tr>${body}</table>`;
}

function drawCoverageCurve() {
  const c = $('chart');
  const g = c.getContext('2d');
  g.clearRect(0, 0, c.width, c.height);
  const cov = state.result?.coverage ?? [];
  if (cov.length < 2) return;
  g.strokeStyle = '#ffd166';
  g.beginPath();
  cov.forEach((v, i) => {
    const x = 10 + (i / (cov.length - 1)) * (c.width - 20);
    const y = c.height - 8 - v * (c.height - 16);
    i ? g.lineTo(x, y) : g.moveTo(x, y);
  });
  g.stroke();
  g.fillStyle = '#8fa1b5';
  g.font = '10px sans-serif';
  g.fillText('coverage vs paths added', 10, 12);
}

function wire() {
  $('solve').onclick = rebuild;
  $('daily').onclick = runDaily;
  $('sweep-btn').onclick = runSweep;
  $('trips-toggle').onchange = (e) => { state.showTrips = e.target.checked; paint(); };
  $('baseline-net').onchange = (e) => { state.showBase = e.target.checked; paint(); };
  for (const [id, key, f] of [
    ['window', 'windowDays', parseInt],
    ['minLen', 'minLen', parseInt],
    ['radius', 'radius', parseInt],
    ['theta', 'theta', parseFloat],
    ['minGain', 'minGainPct', parseFloat],
    ['follow', 'followPct', parseFloat],
  ]) {
    $(id).oninput = (e) => {
      state.params[key] = f(e.target.value);
      $(id + '-v').textContent = e.target.value;
      rebuild();
    };
  }
}

load().catch((e) => {
  $('status').textContent = `no data: ${e.message} — run the trail_lab generator first`;
});
