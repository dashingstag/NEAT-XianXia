//! Sample committed decisions using the same observer as the inspector.
use serde_json::{json, Value};
use sim_core::World;
use std::{collections::BTreeMap, path::Path};

fn sample(initial: &World, hour: f32, owners_only: bool) -> Value {
    let mut w = initial.clone();
    w.set_hour(hour).unwrap();
    let mut control = w.clone();
    let start = w.tick;
    let mut sampled = 0;
    let mut near_market = 0;
    let mut reached = 0;
    let mut truncated = 0;
    let mut max_seen = 0;
    let mut omitted = BTreeMap::<String, u64>::new();
    let mut first = None;
    let mut shop_offered = 0;
    let mut retail = BTreeMap::<&str, (u64, u64)>::new();
    let completed = |world: &World| {
        world
            .urban
            .buildings
            .iter()
            .filter(|b| b.kind == 1)
            .map(|b| b.sold)
            .sum::<u64>()
    };
    let previous_sales = completed(&w);
    let mut shop_chosen = 0;
    let mut order_offered = 0;
    let mut order_winners = BTreeMap::<String, u64>::new();
    for turn in 0..750usize {
        // Rotate within the next scheduled decision cohort. Every other tick
        // specifically samples actors within 120 world units of a market.
        let cohort = (w.tick as usize + 1) % 10;
        let candidates = (cohort..w.agents.alive.len())
            .step_by(10)
            .filter(|&i| {
                w.agents.alive[i]
                    && (!owners_only
                        || w.urban
                            .buildings
                            .iter()
                            .any(|b| b.kind == 1 && b.ready && b.owner == i as u32))
                    && (owners_only
                        || turn % 2 == 1
                        || w.markets
                            .iter()
                            .any(|m| (w.agents.x[i] - m.x).hypot(w.agents.y[i] - m.y) <= 120.0))
            })
            .collect::<Vec<_>>();
        let actor = candidates
            .get((turn / 10 * 17 + turn) % candidates.len().max(1))
            .copied();
        let nearby = actor.is_some_and(|i| {
            w.markets
                .iter()
                .any(|m| (w.agents.x[i] - m.x).hypot(w.agents.y[i] - m.y) <= 120.0)
        });
        let probe = w.retail_supply_probe(actor);
        for (name, count) in probe {
            let (ticks, total) = retail.entry(name).or_default();
            *total += count;
            *ticks += u64::from(count > 0);
        }
        w.watch_person(actor);
        w.step(true);
        control.step(false);
        let Some(d) = w
            .last_watched_decision()
            .filter(|d| d["decision_tick"] == w.tick)
        else {
            continue;
        };
        sampled += 1;
        shop_chosen += usize::from(d["action"] == "ShopGoods");
        near_market += usize::from(nearby);
        if d["candidate_counts"]["OrderGoods"][0].as_u64().unwrap_or(0) > 0 {
            order_offered += 1;
            *order_winners
                .entry(d["action"].as_str().unwrap().to_owned())
                .or_default() += 1;
        }
        let mut admitted = 0;
        let mut lost = 0;
        for (action, pair) in d["candidate_counts"].as_object().unwrap() {
            let n = pair[0].as_u64().unwrap();
            admitted += n;
            if action == "ShopGoods" && n > 0 {
                shop_offered += 1;
            }
            let skip = pair[1].as_u64().unwrap();
            if skip > 0 {
                *omitted.entry(action.clone()).or_default() += skip;
                lost += skip;
            }
        }
        max_seen = max_seen.max(admitted);
        reached += usize::from(admitted == sim_core::CANDIDATES as u64);
        if lost > 0 {
            truncated += 1;
            first.get_or_insert_with(|| d.clone());
        }
    }
    assert!(sampled > 0);
    assert!(
        postcard::to_allocvec(&w).unwrap() == postcard::to_allocvec(&control).unwrap(),
        "observer changed replay for seed {} at tick {}",
        w.seed,
        w.tick
    );
    let measurements: BTreeMap<_, _> = retail
        .into_iter()
        .map(|(name, (ticks, total))| (name, json!({"ticks":ticks,"total":total})))
        .collect();
    json!({"owners_only":owners_only,"order_offered":order_offered,"winners_when_order_offered":order_winners,"retail":measurements,"completed_retail_sales":completed(&w)-previous_sales,"shop_chosen":shop_chosen,"seed":w.seed,"start_tick":start,"ticks":750,"start_hour":hour,"population_at_start":initial.agents.alive.iter().filter(|v|**v).count(),"sampled":sampled,"near_market":near_market,"shop_offered":shop_offered,"max_admitted":max_seen,"reached_cap":reached,"truncated":truncated,"omitted_by_action":omitted,"first_omission":first,"replay_equal":true})
}

fn main() {
    rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build_global()
        .unwrap();
    let fixture = std::env::args()
        .nth(1)
        .expect("pass the mature-town checkpoint");
    let mature = World::load(Path::new(&fixture)).unwrap();
    for hour in [10.0, 21.0] {
        println!("{}", sample(&mature, hour, false));
    }
    for hour in [10.0, 21.0] {
        println!("{}", sample(&mature, hour, true));
    }
    // Same initial state and actor sampling isolate stock from staffed hours.
    for (name, stock, open) in [
        ("stock only", true, false),
        ("open only", false, true),
        ("open and stocked", true, true),
    ] {
        let mut shopping = mature.clone();
        for b in shopping
            .urban
            .buildings
            .iter_mut()
            .filter(|b| b.kind == 1 && b.ready && b.owner != u32::MAX)
        {
            if open {
                b.last_open = shopping.tick;
            }
            if stock {
                // Preserve specialty. Seed a catalog product with its accounted source.
                match b.line {
                    1 => {
                        b.raw[9] += 20;
                        shopping.ledger.initial[9] += 20;
                    }
                    line => {
                        let ware = match line {
                            0 => 1,
                            2 => 3,
                            _ => 4,
                        };
                        b.wares[ware] += 20;
                        shopping.urban.produced[ware] += 20;
                    }
                }
            }
        }
        shopping.check().unwrap();
        let mut report = sample(&shopping, 10.0, false);
        report["scenario"] = name.into();
        println!("{report}");
    }
    for seed in [17, 42, 77] {
        let mut world = World::new(1000, seed).unwrap();
        world.prepare_urban_demo().unwrap();
        for _ in 0..1500 {
            world.step(true);
        }
        for hour in [10.0, 21.0] {
            println!("{}", sample(&world, hour, false));
        }
    }
}
