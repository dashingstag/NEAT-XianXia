//! Generates real walk-path data for the slime-mold trail lab.
//!
//! Runs an actual world for `--days` game days, recording every agent's
//! position every `--sample-every` ticks into `trace.bin` (u16 pairs at
//! 0.1-unit precision). While the world runs, a production-faithful
//! trail layer deposits the walked positions into a heat grid with a
//! neighbour splat and census-cadence decay — the same mechanism the
//! proposed in-game trail system would use — and reports its per-tick
//! overhead separately from the simulation's own step time.
//!
//! Outputs land in `labs/trail-lab/data/` by default:
//!   trace.bin    frames x agents x 2 u16, row-major, x then y
//!   terrain.bin  coarse u8 terrain classes at 24u cells
//!   meta.json    dimensions, markets, sampling, trail-layer stats
//!
//! cargo run --release --locked --manifest-path sim/Cargo.toml -p sim-core \
//!   --example trail_lab -- --agents 10000 --days 20 --map x4

use sim_core::{Hill, MapSize, Terrain, World};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

const DAY_TICKS: u64 = 3000;
/// Trail grid granularity, matching the game's 12u footpath quantization
/// and sitting under the 22u road-ride radius.
const TRAIL_CELL: f32 = 16.0;
/// Deposit strength; neighbours get the splat share.
const DEPOSIT: f32 = 1.0;
const SPLAT: f32 = 0.5;
/// Decay applied at each 120-tick census (the game's sample cadence).
const DECAY: f32 = 0.92;
/// Below this a cell is cold and stops counting as trail.
const COLD: f32 = 0.05;
/// Heat at which a cell counts as a promotion-worthy worn path.
const PROMOTE: f32 = 6.0;
/// Coarse terrain raster for the lab background.
const TERRAIN_CELL: f32 = 24.0;
/// Movement smaller than this per tick deposits nothing (sleepers, stalls).
const MOVED_EPSILON: f32 = 0.25;

struct TrailLayer {
    cols: usize,
    rows: usize,
    heat: Vec<f32>,
    ns: u128,
    deposits: u64,
}

impl TrailLayer {
    fn new(width: f32, height: f32) -> Self {
        let cols = (width / TRAIL_CELL).ceil() as usize;
        let rows = (height / TRAIL_CELL).ceil() as usize;
        Self {
            cols,
            rows,
            heat: vec![0.0; cols * rows],
            ns: 0,
            deposits: 0,
        }
    }
    /// One walked position -> centre cell plus the four neighbours. The
    /// splat is what lets parallel tracks merge into a single trunk.
    fn deposit(&mut self, x: f32, y: f32) {
        let cx = (x / TRAIL_CELL) as usize;
        let cy = (y / TRAIL_CELL) as usize;
        if cx >= self.cols || cy >= self.rows {
            return;
        }
        self.deposits += 1;
        self.heat[cy * self.cols + cx] += DEPOSIT;
        for (dx, dy) in [(1usize, 0usize), (0, 1), (usize::MAX, 0), (0, usize::MAX)] {
            let nx = cx.wrapping_add(dx);
            let ny = cy.wrapping_add(dy);
            if nx < self.cols && ny < self.rows {
                self.heat[ny * self.cols + nx] += SPLAT;
            }
        }
    }
    fn decay(&mut self) {
        for h in &mut self.heat {
            *h *= DECAY;
            if *h < COLD {
                *h = 0.0;
            }
        }
    }
    fn stats(&self) -> (usize, usize, f32) {
        let mut hot = 0;
        let mut promote = 0;
        let mut mass = 0.0;
        for &h in &self.heat {
            if h > 0.0 {
                hot += 1;
                mass += h;
                if h >= PROMOTE {
                    promote += 1;
                }
            }
        }
        (hot, promote, mass)
    }
}

fn arg<T: std::str::FromStr>(args: &[String], name: &str, default: T) -> T {
    args.windows(2)
        .find(|w| w[0] == name)
        .and_then(|w| w[1].parse().ok())
        .unwrap_or(default)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let agents: usize = arg(&args, "--agents", 10_000);
    let days: u64 = arg(&args, "--days", 20);
    let seed: u64 = arg(&args, "--seed", 42);
    let sample_every: u64 = arg(&args, "--sample-every", 60);
    let map = args
        .windows(2)
        .find(|w| w[0] == "--map")
        .map(|w| w[1].as_str())
        .unwrap_or("x4");
    let timing_only = args.iter().any(|a| a == "--timing-only");
    let out = arg::<String>(
        &args,
        "--out",
        "labs/trail-lab/data".to_string(),
    );
    let size = match map {
        "classic" => MapSize::Classic,
        "x4" => MapSize::X4,
        "x25" => MapSize::X25,
        "x100" => MapSize::X100,
        other => {
            eprintln!("unknown map {other}");
            std::process::exit(2);
        }
    };

    println!(
        "world: {agents} agents, {days} days ({} ticks), map {map}, seed {seed}",
        days * DAY_TICKS
    );
    let t0 = Instant::now();
    let mut w = World::new_with_size(agents, seed, Hill::default(), size)
        .unwrap_or_else(|e| {
            eprintln!("world init failed: {e}");
            std::process::exit(1);
        });
    let (width, height) = (w.map.spec.width(), w.map.spec.height());
    println!("init: {:?}  map {width}x{height}  markets {}", t0.elapsed(), w.markets.len());

    let frames = (days * DAY_TICKS / sample_every) as usize;
    let mut trace = Vec::with_capacity(frames * agents * 2);
    let mut trail = TrailLayer::new(width, height);
    let mut last_x = w.agents.x.clone();
    let mut last_y = w.agents.y.clone();
    let quant = |v: f32| (v * 10.0).clamp(0.0, 65535.0) as u16;

    let mut step_ns = 0u128;
    let mut daily = Vec::new();
    for _ in 0..days * DAY_TICKS {
        let s = Instant::now();
        w.step(true);
        step_ns += s.elapsed().as_nanos();

        let s = Instant::now();
        // Deposit only real movement: sleepers, the stalled and the dead
        // lay no trail, same as they would in game.
        for i in 0..w.agents.len() {
            let (x, y) = (w.agents.x[i], w.agents.y[i]);
            let moved = (x - last_x[i]).abs() + (y - last_y[i]).abs() > MOVED_EPSILON;
            if moved && w.agents.alive[i] {
                trail.deposit(x, y);
            }
            last_x[i] = x;
            last_y[i] = y;
        }
        if w.tick % 120 == 0 {
            trail.decay();
        }
        trail.ns += s.elapsed().as_nanos();

        if w.tick % sample_every == 0 {
            for i in 0..w.agents.len() {
                trace.push(quant(w.agents.x[i]));
                trace.push(quant(w.agents.y[i]));
            }
        }
        if w.tick % DAY_TICKS == 0 {
            let (hot, promote, mass) = trail.stats();
            daily.push(serde_json::json!({
                "day": w.tick / DAY_TICKS,
                "hot_cells": hot,
                "promote_cells": promote,
                "trail_mass": mass,
                "markets": w.markets.len(),
                "living": w.agents.alive.iter().filter(|&&a| a).count(),
            }));
            println!(
                "day {}: {:?}/tick  hot={hot} promote={promote} mass={mass:.0}",
                w.tick / DAY_TICKS,
                step_ns / (w.tick as u128 + 1) / 1000
            );
        }
    }
    let total = t0.elapsed();
    println!(
        "done: {:?} total | step {:>7.3} ms/tick | trail layer {:>7.4} ms/tick ({:.2}% of step) | {} deposits",
        total,
        step_ns as f64 / (days * DAY_TICKS) as f64 / 1e6,
        trail.ns as f64 / (days * DAY_TICKS) as f64 / 1e6,
        100.0 * trail.ns as f64 / step_ns.max(1) as f64,
        trail.deposits,
    );

    if timing_only {
        return;
    }

    let dir = PathBuf::from(&out);
    std::fs::create_dir_all(&dir).unwrap();
    // trace.bin: u16 LE, frame-major [frame][agent][x,y]
    let mut buf = Vec::with_capacity(trace.len() * 2);
    for v in &trace {
        buf.extend_from_slice(&v.to_le_bytes());
    }
    write(&dir.join("trace.bin"), &buf);

    // terrain.bin: u8 terrain classes at TERRAIN_CELL resolution
    let tcols = (width / TERRAIN_CELL).ceil() as usize;
    let trows = (height / TERRAIN_CELL).ceil() as usize;
    let mut raster = Vec::with_capacity(tcols * trows);
    for r in 0..trows {
        for c in 0..tcols {
            let t = w.map.terrain_at(
                c as f32 * TERRAIN_CELL + TERRAIN_CELL * 0.5,
                r as f32 * TERRAIN_CELL + TERRAIN_CELL * 0.5,
            );
            raster.push(t as u8);
        }
    }
    write(&dir.join("terrain.bin"), &raster);

    // Final heat frame so the viewer can show the settled network.
    let mut heat16 = Vec::with_capacity(trail.heat.len() * 2);
    for &h in &trail.heat {
        heat16.extend_from_slice(&((h * 100.0).clamp(0.0, 65535.0) as u16).to_le_bytes());
    }
    write(&dir.join("trail.bin"), &heat16);

    let markets: Vec<_> = w
        .markets
        .iter()
        .map(|m| serde_json::json!({"id": m.id, "name": m.name, "x": m.x, "y": m.y, "population": m.population}))
        .collect();
    let (hot, promote, mass) = trail.stats();
    let meta = serde_json::json!({
        "seed": seed,
        "agents": agents,
        "days": days,
        "map": map,
        "width": width,
        "height": height,
        "sample_every": sample_every,
        "frames": trace.len() / (agents * 2),
        "trail": {
            "cell": TRAIL_CELL,
            "cols": trail.cols,
            "rows": trail.rows,
            "deposit": DEPOSIT,
            "splat": SPLAT,
            "decay_per_120": DECAY,
            "cold": COLD,
            "promote": PROMOTE,
            "heat_scale": 100,
            "hot_cells": hot,
            "promote_cells": promote,
            "trail_mass": mass,
            "deposits": trail.deposits,
        },
        "terrain": {"cell": TERRAIN_CELL, "cols": tcols, "rows": trows},
        "timing": {
            "total_s": total.as_secs_f64(),
            "step_ms_per_tick": step_ns as f64 / (days * DAY_TICKS) as f64 / 1e6,
            "trail_ms_per_tick": trail.ns as f64 / (days * DAY_TICKS) as f64 / 1e6,
        },
        "daily": daily,
        "markets": markets,
    });
    write(&dir.join("meta.json"), meta.to_string().as_bytes());
    println!("wrote {}", dir.display());
}

fn write(path: &Path, bytes: &[u8]) {
    let mut f = std::fs::File::create(path)
        .unwrap_or_else(|e| panic!("create {}: {e}", path.display()));
    f.write_all(bytes)
        .unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
}

// Keep Terrain referenced for the explicit type in the raster loop.
#[allow(dead_code)]
fn _assert_terrain(_: Terrain) {}
