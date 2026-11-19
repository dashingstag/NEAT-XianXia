//! Paired live-world coverage and an exploratory memory score pilot.
use sim_core::World;
use std::time::Instant;

fn main() {
    let mut runs = Vec::new();
    let fixture = std::env::args().nth(1);
    let seeds: &[u64] = if fixture.is_some() {
        &[0]
    } else {
        &[11, 23, 35]
    };
    for &seed in seeds {
        let initial = if let Some(path) = &fixture {
            World::load(std::path::Path::new(path)).unwrap()
        } else {
            World::new(128, seed).unwrap()
        };
        let seed = initial.seed;
        let phase_ticks = if fixture.is_some() { 1000 } else { 3000 };
        for mode in [0, 1, 3] {
            let mut world = initial.clone();
            let start = Instant::now();
            world.enable_memory_pilot(mode);
            let renewals: Vec<_> = world.patches.iter().map(|patch| patch.renewal).collect();
            let mut phases = Vec::new();
            for phase in 0..3 {
                for (patch, &renewal) in world.patches.iter_mut().zip(&renewals) {
                    patch.renewal = if phase == 1 && [0, 3].contains(&patch.kind) {
                        0.0
                    } else {
                        renewal
                    };
                }
                let before = world.stats.clone();
                for _ in 0..phase_ticks {
                    world.step(false);
                }
                world.check().unwrap();
                phases.push(serde_json::json!({
                    "phase": (["normal", "renewal_paused", "renewal_restored"][phase]),
                    "living": world.living(),
                    "trades": world.stats.trades - before.trades,
                    "harvested": world.stats.harvested - before.harvested,
                    "deaths": world.stats.deaths - before.deaths,
                }));
            }
            let run = serde_json::json!({
                "seed": seed, "readouts": mode, "ms": start.elapsed().as_millis(),
                "phases": phases, "coverage": world.memory_pilot_coverage(),
            });
            eprintln!(
                "seed={seed} mode={mode} living={} trades={}",
                world.living(),
                world.stats.trades
            );
            runs.push(run);
        }
    }
    println!("{}", serde_json::to_string_pretty(&runs).unwrap());
}
