//! Compare the default owner policy with the experimental-restocking feature.
use sim_core::World;
use std::{collections::BTreeMap, path::Path};

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
    let mut funnel = BTreeMap::<String, u64>::new();
    let mut pending = false;
    for day in 0..2 {
        let checkpoint = std::path::PathBuf::from(format!(
            ".work/restock-replay-{}.snapshot",
            std::process::id()
        ));
        std::fs::create_dir_all(".work").unwrap();
        world.save(&checkpoint).unwrap();
        let mut replay = World::load(&checkpoint).unwrap();
        std::fs::remove_file(checkpoint).unwrap();
        for turn in 0..3000 {
            if !pending {
                let cohort = (world.tick as usize + 1) % 10;
                let cohort_size = world.agents.alive.len().saturating_sub(cohort).div_ceil(10);
                let actor = (cohort_size > 0)
                    .then(|| cohort + 10 * ((turn / 10 + turn) % cohort_size))
                    .filter(|&i| world.agents.alive[i]);
                world.watch_person(actor);
                if let Some(i) = actor {
                    for (name, count) in world.retail_supply_probe(Some(i)) {
                        if matches!(name, "affordable" | "fits" | "urban_candidates") && count > 0 {
                            *funnel.entry(name.into()).or_default() += 1;
                        }
                    }
                }
            }
            world.step(true);
            if let Some(d) = world.last_watched_decision() {
                if !pending && d["decision_tick"] == world.tick {
                    *funnel.entry("sampled_decisions".into()).or_default() += 1;
                    let counts = &d["candidate_counts"]["ShopGoods"];
                    if counts[0].as_u64().unwrap_or(0) > 0 {
                        *funnel
                            .entry(format!(
                                "admitted_winner:{}",
                                d["action"].as_str().unwrap_or("unknown")
                            ))
                            .or_default() += 1;
                    }
                    if counts[1].as_u64().unwrap_or(0) > 0 {
                        *funnel.entry("omitted_at_cap".into()).or_default() += 1;
                    }
                    pending = d["action"] == "ShopGoods";
                }
            }
            if pending {
                match world.last_watched_decision() {
                    Some(d) if d["action"] == "ShopGoods" && d["outcome"].is_null() => {}
                    Some(d) if d["action"] == "ShopGoods" => {
                        *funnel
                            .entry(format!("checkout:{}", d["outcome"]))
                            .or_default() += 1;
                        pending = false;
                    }
                    _ => {
                        *funnel
                            .entry("decision_replaced_before_checkout".into())
                            .or_default() += 1;
                        pending = false;
                    }
                }
            }
            replay.step(true);
            open_stocked += world
                .retail_supply_probe(None)
                .into_iter()
                .find(|(name, _)| *name == "open_stocked")
                .unwrap()
                .1;
        }
        world.check().unwrap();
        assert!(world.checkpoint_payload().unwrap() == replay.checkpoint_payload().unwrap());
        println!("buyer_funnel={funnel:?} pending={pending}");
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
