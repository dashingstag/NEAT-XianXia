// Engine checks: node tests.mjs
import { TrailModel, ProbeWalker, Census, depositFrame, framePair, DEFAULTS } from './engine.mjs';

let failures = 0;
const ok = (cond, msg) => {
  if (!cond) { failures++; console.error('FAIL', msg); }
};

// Deposit raises centre and splat neighbours.
{
  const m = new TrailModel(10, 10, 16);
  m.depositAt(80, 80, 1, 0.5);
  const i = m.index(80, 80);
  ok(m.heat[i] === 1, 'centre deposit');
  ok(m.heat[i + 1] === 0.5, 'right splat');
  ok(m.heat[i - 1] === 0.5, 'left splat');
  ok(m.heat[i + 10] === 0.5, 'down splat');
  ok(m.deposits === 1, 'deposit count');
}

// Segment deposit fills the walked line, not just endpoints.
{
  const m = new TrailModel(20, 4, 16);
  m.depositSegment(0, 32, 160, 32, 1, 0);
  let covered = 0;
  for (let x = 2; x < 150; x += 8) if (m.sense(x, 32) > 0) covered++;
  ok(covered > 15, `segment coverage ${covered}/19`);
}

// Teleport-length jumps deposit at the landing only, not across the map.
{
  const m = new TrailModel(40, 40, 16);
  m.depositSegment(8, 8, 500, 500, 1, 0, 340);
  ok(m.sense(500, 500) > 0, 'landing deposited');
  ok(m.sense(250, 250) === 0, 'no phantom midpoint');
}

// Decay shrinks heat and forgets cold cells.
{
  const m = new TrailModel(4, 4, 16);
  m.depositAt(8, 8, 1, 0);
  m.decay(0.5, 0.05);
  ok(Math.abs(m.sense(8, 8) - 0.5) < 1e-6, 'decay halves');
  m.decay(0.01, 0.05);
  ok(m.sense(8, 8) === 0, 'cold cell forgotten');
}

// framePair reads x/y and reports movement.
{
  const agents = 2;
  const tr = new Uint16Array(2 * agents * 2); // 2 frames
  tr[0] = 10; tr[1] = 20;          // frame0 agent0 (1.0, 2.0)
  tr[4] = 30; tr[5] = 40;          // frame1 agent0 (3.0, 4.0)
  const out = {};
  ok(framePair(tr, agents, 1, 0, out) === true, 'moved pair');
  ok(out.x0 === 1 && out.y0 === 2 && out.x1 === 3 && out.y1 === 4, 'pair values');
  ok(framePair(tr, agents, 1, 1, out) === false, 'still pair');
}

// A probe dropped beside a hot line converges onto it.
{
  const m = new TrailModel(40, 40, 16);
  for (let x = 0; x < 40; x++) for (let s = 0; s < 40; s++) m.depositAt(x * 16 + 8, 320, 1, 0.5);
  const p = new ProbeWalker(100, 100, 600, 320);
  for (let i = 0; i < 400 && !p.arrived; i++) p.step(m, DEFAULTS);
  ok(p.arrived, 'probe reached goal');
  ok(p.distance < Math.hypot(500, 220) * 1.6, `probe detour bounded (${p.distance.toFixed(0)})`);
}

// depositFrame feeds a model from a trace frame, honoring minTrip.
{
  const agents = 3;
  const tr = new Uint16Array(2 * agents * 2);
  tr[6] = 100; tr[7] = 100;   // frame1 agent0 -> (10,10), 14u trip
  tr[8] = 2000; tr[9] = 5000; // frame1 agent1 -> (200,500), 530u trip
  const m = new TrailModel(40, 40, 16);
  depositFrame(m, tr, agents, 1, { ...DEFAULTS, minTrip: 48 });
  ok(m.sense(200, 500) > 0, 'long trip deposited');
  ok(m.sense(10, 10) === 0, 'short wiggle skipped');
  const m2 = new TrailModel(40, 40, 16);
  depositFrame(m2, tr, agents, 1, { ...DEFAULTS, minTrip: 0 });
  ok(m2.sense(10, 10) > 0, 'minTrip 0 deposits all');
}

// Census counts market-to-market trips at the game's 96u radius.
{
  const markets = [{ id: 0, x: 0, y: 0 }, { id: 1, x: 500, y: 0 }];
  const agents = 1;
  const tr = new Uint16Array(3 * agents * 2);
  // frame1: inside market 0 at (5,0)
  tr[2] = 50; tr[3] = 0;
  // frame2: inside market 1 at (500,5)
  tr[4] = 5000; tr[5] = 50;
  const c = new Census(agents, markets);
  c.update(tr, agents, 1);
  c.update(tr, agents, 2);
  ok(c.trips.get('0<1') === 1, 'trip 0->1 counted');
}

console.log(failures === 0 ? 'all engine checks passed' : `${failures} failure(s)`);
process.exit(failures ? 1 : 0);
