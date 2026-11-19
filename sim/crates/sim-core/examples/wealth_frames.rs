//! Compare full and subscribed viewer snapshots of the same synthetic town.
use sim_core::{WealthView, World};
use std::{env, path::Path, time::Instant};

fn main() {
    let path = env::args()
        .nth(1)
        .expect("supply the mature-town checkpoint");
    let initial = World::load(Path::new(&path)).expect("load fixture");
    for mode in ["full", "closed", "open-paused", "open-running"] {
        let mut world = initial.clone();
        let mut view = WealthView::default();
        view.subscribe(mode.starts_with("open"));
        let mut times = Vec::new();
        let mut bytes = 0;
        for _ in 0..101 {
            if mode == "open-running" {
                world.step(false);
            }
            let start = Instant::now();
            let frame = if mode == "full" {
                world.snapshot(20_000, Some(0), true)
            } else {
                world.viewer_snapshot(
                    20_000,
                    Some(0),
                    mode != "open-running",
                    None,
                    view.report(&world),
                )
            };
            times.push(start.elapsed().as_nanos());
            bytes += frame.len();
        }
        times.sort_unstable();
        println!(
            "{mode}: {} builds, {:.0} us median, {} bytes/frame",
            if mode == "full" { 101 } else { view.builds() },
            times[50] as f64 / 1000.,
            bytes / 101
        );
    }
}
