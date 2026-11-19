//! Compare the default owner policy with the experimental-restocking feature.
use sim_core::World;
use std::path::Path;

fn measure(mut world: World) {
    let owners: Vec<_> = world
        .urban
        .buildings
        .iter()
        .filter(|b| b.kind == 1 && b.owner != u32::MAX)
        .map(|b| b.owner as usize)
        .collect();
    let sales = |w: &World| {
        w.urban
            .buildings
            .iter()
            .filter(|b| b.kind == 1)
            .map(|b| b.sold)
            .sum::<u64>()
    };
    let start_sales = sales(&world);
    let mut open_stocked = 0;
    for day in 0..2 {
        let checkpoint = std::path::PathBuf::from(format!(
            ".work/restock-replay-{}.snapshot",
            std::process::id()
        ));
        std::fs::create_dir_all(".work").unwrap();
        world.save(&checkpoint).unwrap();
        let mut replay = World::load(&checkpoint).unwrap();
        std::fs::remove_file(checkpoint).unwrap();
        for _ in 0..3000 {
            world.step(true);
            replay.step(true);
            open_stocked += world
                .retail_supply_probe(None)
                .into_iter()
                .find(|(name, _)| *name == "open_stocked")
                .unwrap()
                .1;
        }
        world.check().unwrap();
        assert!(postcard::to_allocvec(&world).unwrap() == postcard::to_allocvec(&replay).unwrap());
        let shops = world
            .urban
            .buildings
            .iter()
            .filter(|b| b.kind == 1 && b.owner != u32::MAX);
        println!("seed={} day={} flow={:?} stocked_open_shop_ticks={} sales={} cash={} arrears={} owners_alive={}/{}",world.seed,day+1,world.retail_flow(),open_stocked,sales(&world)-start_sales,shops.clone().map(|b|b.cash).sum::<u64>(),shops.map(|b|b.arrears).sum::<u64>(),owners.iter().filter(|&&i|world.agents.alive[i]).count(),owners.len());
    }
}
fn main() {
    rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build_global()
        .unwrap();
    measure(
        World::load(Path::new(
            &std::env::args().nth(1).expect("mature checkpoint"),
        ))
        .unwrap(),
    );
}
