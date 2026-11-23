//! Write a synthetic checkpoint with enough buildings to force several
//! 256-row `building_batch` frames in the protocol suite.
//!
//! Usage: stream_fixture <source checkpoint> <output checkpoint> <building count>
//!
//! The extra buildings are empty, unowned, unfinished shells copied from the
//! source's first building and spaced around its market. They hold no cash,
//! goods or wares, so money and ware conservation are unchanged. The output
//! is test data only and is never checked in.
use sim_core::World;
use std::path::Path;

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [source, output, count] = args.as_slice() else {
        return Err("Usage: stream_fixture <source> <output> <building count>".into());
    };
    let count: usize = count.parse().map_err(|e| format!("count: {e}"))?;
    let mut world = World::load(Path::new(source))?;
    let template = world
        .urban
        .buildings
        .first()
        .cloned()
        .ok_or("Source checkpoint has no building to copy")?;
    let mut shell = template.clone();
    shell.ready = false;
    shell.progress = 0.0;
    shell.needs_clear = false;
    shell.advertised = false;
    shell.owner = u32::MAX;
    shell.renter = u32::MAX;
    shell.shelter = u32::MAX;
    shell.price = 0;
    shell.rent = 0;
    shell.leased = 0;
    shell.arrears = 0;
    shell.last_rent = 0;
    shell.cash = 0;
    shell.raw = Default::default();
    shell.wares = Default::default();
    shell.made = 0;
    shell.sold = 0;
    let mut added = 0usize;
    while world.urban.buildings.len() < count {
        let mut b = shell.clone();
        // A 20-column grid of 12u cells to one side of the template.
        b.x = template.x + 40.0 + (added % 20) as f32 * 12.0;
        b.y = template.y + 40.0 + (added / 20) as f32 * 12.0;
        b.line = (added % 4) as u8;
        world.urban.buildings.push(b);
        added += 1;
    }
    world.check()?;
    world.save(Path::new(output))?;
    println!(
        "{{\"buildings\":{},\"added\":{},\"tick\":{}}}",
        world.urban.buildings.len(),
        added,
        world.tick
    );
    Ok(())
}
