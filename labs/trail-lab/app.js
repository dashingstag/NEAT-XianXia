import { TrailModel, ProbeWalker, Census, depositFrame, DEFAULTS } from './engine.mjs';

const TERRAIN_COLORS = [
  '#1d3557', // Water
  '#6c757d', // Rock
  '#5c6b4f', // Marsh
  '#2d4a1e', // Forest
  '#7d9b4a', // Fertile
  '#556b2f', // Grass
  '#3a6ea8', // River
];

const state = {
  meta: null,
  trace: null,
  terrain: null,
  frame: 1,
  playing: false,
  framesPerSecond: 8,
  showAgents: true,
  showProbes: true,
  view: 'mold', // 'mold' | 'raw' | 'split'
  params: { ...DEFAULTS },
  mold: null,
  raw: null,
  probes: [],
  history: [],
};

const $ = (id) => document.getElementById(id);
const canvas = $('map');
const ctx = canvas.getContext('2d');
const rawCanvas = $('map-raw');
const rawCtx = rawCanvas.getContext('2d');

async function load() {
  state.meta = await (await fetch('data/meta.json')).json();
  state.trace = new Uint16Array(await (await fetch('data/trace.bin')).arrayBuffer());
  state.terrain = new Uint8Array(await (await fetch('data/terrain.bin')).arrayBuffer());
  const m = state.meta;
  state.mold = new TrailModel(m.trail.cols, m.trail.rows, m.trail.cell);
  state.raw = new TrailModel(m.trail.cols, m.trail.rows, m.trail.cell);
  state.census = new Census(m.agents, m.markets);
  canvas.width = rawCanvas.width = m.width / 4;
  canvas.height = rawCanvas.height = m.height / 4;
  drawTerrain();
  spawnProbes(24);
  $('status').textContent =
    `${m.agents} agents · ${m.days} days · ${m.frames} samples · ` +
    `${m.timing.step_ms_per_tick.toFixed(1)} ms/step · trail layer ` +
    `${m.timing.trail_ms_per_tick.toFixed(3)} ms/tick in-game`;
  wire();
  requestAnimationFrame(tick);
}

function drawTerrain() {
  const { terrain, meta } = state;
  const { cols, rows } = meta.terrain;
  const off = document.createElement('canvas');
  off.width = cols;
  off.height = rows;
  const c = off.getContext('2d');
  const img = c.createImageData(cols, rows);
  for (let i = 0; i < terrain.length; i++) {
    const rgb = hex(TERRAIN_COLORS[terrain[i]] ?? '#000');
    img.data[i * 4] = rgb[0];
    img.data[i * 4 + 1] = rgb[1];
    img.data[i * 4 + 2] = rgb[2];
    img.data[i * 4 + 3] = 255;
  }
  c.putImageData(img, 0, 0);
  state.terrainImage = off;
}

function hex(h) {
  return [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16));
}

// Heat -> colour. Rational mapping v/(v+promote) never saturates: faint
// traffic haze stays a dim ember while real trunks burn gold, and cells
// over the promote bar get a bright core so the worn-path skeleton reads
// separately from ambient walking.
function renderHeat(target, model) {
  const { cols, rows } = model;
  const off = document.createElement('canvas');
  off.width = cols;
  off.height = rows;
  const c = off.getContext('2d');
  const img = c.createImageData(cols, rows);
  const promote = state.params.promote;
  for (let i = 0; i < model.heat.length; i++) {
    const v = model.heat[i];
    if (v <= 0) continue;
    const o = i * 4;
    if (v >= promote) {
      // Trunk: bright gold, near-opaque.
      img.data[o] = 255;
      img.data[o + 1] = 214;
      img.data[o + 2] = 110;
      img.data[o + 3] = 235;
    } else {
      const t = v / (v + promote);
      img.data[o] = 120 + t * 120;
      img.data[o + 1] = 55 + t * 90;
      img.data[o + 2] = 20;
      img.data[o + 3] = 20 + t * 160;
    }
  }
  c.putImageData(img, 0, 0);
  const scale = canvas.width / cols;
  target.imageSmoothingEnabled = true;
  target.drawImage(off, 0, 0, cols * scale, rows * scale);
}

function renderScene(target, model, t) {
  const { meta } = state;
  target.drawImage(state.terrainImage, 0, 0, canvas.width, canvas.height);
  renderHeat(target, model);
  const sx = canvas.width / meta.width;
  const sy = canvas.height / meta.height;
  // Moving agents as dots, interpolated between the recorded frames.
  if (state.showAgents && t < meta.frames) {
    target.fillStyle = 'rgba(255,255,255,0.55)';
    const f = Math.floor(t);
    const frac = t - f;
    const agents = meta.agents;
    const tr = state.trace;
    for (let i = 0; i < agents; i += 2) {
      const a = (Math.max(0, f - 1) * agents + i) * 2;
      const b = (f * agents + i) * 2;
      const x = (tr[a] + (tr[b] - tr[a]) * frac) / 10;
      const y = (tr[a + 1] + (tr[b + 1] - tr[a + 1]) * frac) / 10;
      target.fillRect(x * sx, y * sy, 2, 2);
    }
  }
  // Markets.
  target.font = '10px sans-serif';
  for (const m of meta.markets) {
    target.fillStyle = '#ffd166';
    target.beginPath();
    target.arc(m.x * sx, m.y * sy, 4, 0, Math.PI * 2);
    target.fill();
    target.fillStyle = '#fff';
    target.fillText(m.name, m.x * sx + 6, m.y * sy + 3);
  }
  // Probe walkers riding the mold network.
  if (state.showProbes) {
    target.fillStyle = '#7ef0ff';
    for (const p of state.probes) {
      target.fillRect(p.x * sx - 1.5, p.y * sy - 1.5, 3, 3);
    }
  }
}

function spawnProbes(n) {
  const { meta } = state;
  const mkts = meta.markets;
  state.probes = [];
  for (let k = 0; k < n; k++) {
    const a = mkts[k % mkts.length];
    const b = mkts[(k + 1) % mkts.length];
    // Spawn offset from the direct line so they must find and join a trail.
    const ox = (Math.sin(k * 2.3) * 220);
    const oy = (Math.cos(k * 1.7) * 220);
    state.probes.push(new ProbeWalker(a.x + ox, a.y + oy, b.x, b.y));
  }
}

let last = 0;
let acc = 0;
function tick(now) {
  const dt = Math.min(0.1, (now - last) / 1000);
  last = now;
  if (state.playing) {
    acc += dt * state.framesPerSecond;
    const { meta, mold, raw, params } = state;
    while (acc >= 1 && state.frame < meta.frames) {
      acc -= 1;
      depositFrame(mold, state.trace, meta.agents, state.frame, params);
      depositFrame(raw, state.trace, meta.agents, state.frame, { ...params, decay: 1, splat: 0 });
      state.census.update(state.trace, meta.agents, state.frame);
      // Deposit-cadence decay: each recorded frame is 60 ticks; the game's
      // census fires every 120, so decay every other frame.
      if (state.frame % 2 === 0) mold.decay(params.decay, params.cold);
      for (const p of state.probes) p.step(mold, params);
      state.frame += 1;
      if (state.frame % 50 === 0) recordDay();
    }
    if (state.frame >= meta.frames) {
      state.playing = false;
      $('play').textContent = 'Replay';
    }
  }
  paint();
  requestAnimationFrame(tick);
}

function paint() {
  const split = state.view === 'split';
  rawCanvas.style.display = split ? 'block' : 'none';
  canvas.style.width = split ? '49%' : '100%';
  const model = state.view === 'raw' ? state.raw : state.mold;
  renderScene(ctx, model, state.frame);
  if (split) renderScene(rawCtx, state.raw, state.frame);
  const ms = state.mold.stats(state.params.promote);
  const rs = state.raw.stats(state.params.promote);
  $('day').textContent = `day ${Math.ceil(state.frame / 50)} / ${state.meta.days}`;
  $('stats').innerHTML =
    `mold: <b>${ms.promoted}</b> trunk cells (${ms.hot} warm) · ` +
    `raw: <b>${rs.promoted}</b> (${rs.hot} warm) · ` +
    `consolidation <b>${rs.hot > 0 ? ((1 - ms.hot / rs.hot) * 100).toFixed(1) : 0}%</b><br>` +
    `probes riding: ${state.probes.filter((p) => !p.arrived).length} active, ` +
    `${state.probes.filter((p) => p.arrived).length} arrived · ` +
    `deposits ${state.mold.deposits.toLocaleString()}<br>` +
    `corridor census (market→market trips): ` +
    (state.census.top(4).map(([k, n]) => `${k.replace('<', '→')}: <b>${n}</b>`).join(' · ') || 'none yet');
  drawChart();
}

function recordDay() {
  const ms = state.mold.stats(state.params.promote);
  const rs = state.raw.stats(state.params.promote);
  state.history.push({ day: state.history.length + 1, moldHot: ms.hot, rawHot: rs.hot });
}

function drawChart() {
  const c = $('chart');
  const g = c.getContext('2d');
  const W = c.width;
  const H = c.height;
  g.clearRect(0, 0, W, H);
  if (state.history.length < 2) return;
  const max = Math.max(...state.history.map((h) => h.rawHot), 1);
  const line = (key, color) => {
    g.strokeStyle = color;
    g.beginPath();
    state.history.forEach((h, i) => {
      const x = (i / (state.meta.days - 1)) * (W - 20) + 10;
      const y = H - 8 - (h[key] / max) * (H - 16);
      i ? g.lineTo(x, y) : g.moveTo(x, y);
    });
    g.stroke();
  };
  line('rawHot', '#8899aa');
  line('moldHot', '#ffd166');
}

function wire() {
  $('play').onclick = () => {
    if (state.frame >= state.meta.frames) reset();
    state.playing = !state.playing;
    $('play').textContent = state.playing ? 'Pause' : 'Play';
  };
  $('reset').onclick = reset;
  $('speed').oninput = (e) => (state.framesPerSecond = +e.target.value);
  for (const id of ['deposit', 'splat', 'decay', 'promote', 'trailPull', 'minTrip']) {
    $(id).oninput = (e) => {
      state.params[id] = +e.target.value;
      $(id + '-v').textContent = e.target.value;
    };
  }
  $('agents-toggle').onchange = (e) => (state.showAgents = e.target.checked);
  $('probes-toggle').onchange = (e) => (state.showProbes = e.target.checked);
  document.querySelectorAll('input[name=view]').forEach((r) => {
    r.onchange = () => (state.view = document.querySelector('input[name=view]:checked').value);
  });
}

function reset() {
  state.frame = 1;
  state.mold.heat.fill(0);
  state.raw.heat.fill(0);
  state.mold.deposits = 0;
  state.raw.deposits = 0;
  state.census = new Census(state.meta.agents, state.meta.markets);
  state.history = [];
  spawnProbes(24);
  acc = 0;
}

load().catch((e) => {
  $('status').textContent = `no data: ${e.message} — generate it first (see README.md)`;
});
