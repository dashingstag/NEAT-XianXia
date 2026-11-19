//! Compare food-memory policies during a local stockout using live simulation.
use neat_core::{Compiled, Rng, INPUTS, MEMORY_INPUTS};
use sim_core::{Action, World};

const FOOD: [usize; 3] = [0, 3, 8];
const ACTIVE: Weights = [0.12, 0.0, 0.06, 0.0];
const SEEDS: [u64; 3] = [11, 23, 35];
type Weights = [f32; MEMORY_INPUTS];
type Probe = [[f32; INPUTS]; 2];

#[derive(Default)]
struct Phase {
    purchases: u64,
    forage: usize,
    hungry: usize,
    first_purchase: Option<usize>,
}

struct Outcome {
    phases: [Phase; 3],
    fitness: Vec<[usize; 3]>,
}

fn apply_weights(world: &mut World, weights: &[Weights]) {
    for ((genome, brain), weights) in world.genomes.iter_mut().zip(&mut world.brains).zip(weights) {
        let start = genome.genes.len() - weights.len();
        for (edge, &weight) in genome.genes[start..].iter_mut().zip(weights) {
            assert!(edge.from >= u32::MAX - 3);
            edge.weight = weight;
        }
        *brain = genome.compile().unwrap();
    }
}

fn run(
    mut world: World,
    market: usize,
    durations: [usize; 3],
    mut probes: Option<&mut Vec<Probe>>,
) -> Outcome {
    let cohort: Vec<_> = (0..world.agents.len())
        .filter(|&i| world.agents.home[i] == market as u32)
        .collect();
    let mut fitness = vec![[0; 3]; world.agents.len()];
    let mut before = vec![0; cohort.len()];
    let mut escrow = FOOD.map(|_| std::collections::VecDeque::new());
    let mut phases = std::array::from_fn(|_| Phase::default());
    for (index, (phase, duration)) in phases.iter_mut().zip(durations).enumerate() {
        if index == 2 {
            for (kind, lots) in FOOD.into_iter().zip(&mut escrow) {
                let held: u64 = lots.iter().map(|lot: &sim_core::Lot| lot.quantity).sum();
                world.ledger.consumed[kind] -= held;
                world.markets[market].goods[kind].append(lots);
                world.markets[market].quotes[kind].offered += held;
            }
        }
        for tick in 0..duration {
            if index == 1 {
                for (kind, lots) in FOOD.into_iter().zip(&mut escrow) {
                    let stock = world.markets[market].stock(kind);
                    world.ledger.consumed[kind] += stock;
                    world.markets[market].quotes[kind].offered = 0;
                    lots.append(&mut world.markets[market].goods[kind]);
                }
            }
            for (&i, previous) in cohort.iter().zip(&mut before) {
                *previous = world.people[i].trades;
            }
            world.step(false);
            for (&i, &previous) in cohort.iter().zip(&before) {
                if world.agents.action[i] == Action::BuyFood && world.people[i].trades > previous {
                    phase.purchases += world.people[i].trades - previous;
                    phase.first_purchase.get_or_insert(tick + 1);
                }
                let alive = world.agents.alive[i];
                let hungry = alive && world.agents.hunger[i] >= 55.0;
                phase.forage += usize::from(alive && world.agents.action[i] == Action::ForageMeal);
                phase.hungry += usize::from(hungry);
                fitness[i][index] += if alive { usize::from(hungry) } else { 2 };
            }
            if tick % 25 == 0 {
                if let Some(probes) = probes.as_mut().filter(|probes| probes.len() < 48) {
                    for &i in &cohort {
                        let h = world.agents.hunger[i];
                        if world.agents.alive[i] && (30.0..55.0).contains(&h) {
                            let trace = world.trace(i).unwrap();
                            let offer = |action| {
                                trace
                                    .choices
                                    .iter()
                                    .find(|c| c.action == action)
                                    .map(|c| c.inputs.as_slice().try_into().unwrap())
                            };
                            if let (Some(buy), Some(forage)) =
                                (offer("BuyFood"), offer("ForageMeal"))
                            {
                                probes.push([buy, forage]);
                                break;
                            }
                        }
                    }
                }
            }
        }
        world.check().unwrap();
    }
    Outcome { phases, fitness }
}

fn evolve(initial: &World, schedules: usize, matched: bool) -> Vec<Weights> {
    const GENERATIONS: usize = 10;
    let generations = if matched {
        GENERATIONS * 3 / schedules
    } else {
        GENERATIONS
    };
    let mut rng = Rng(initial.seed ^ 0x700d);
    let mut weights = vec![[0.0; MEMORY_INPUTS]; initial.genomes.len()];
    for generation in 0..generations {
        let market = if matched {
            (generation * schedules / 3) % 3
        } else {
            generation % 3
        };
        let mut world = initial.clone();
        apply_weights(&mut world, &weights);
        let mut fitness = vec![0.0f64; initial.agents.len()];
        for schedule in 0..schedules {
            let durations = if matched {
                // Both strategies receive the same 30 schedules and simulated tick budget.
                let trial = generation * schedules + schedule;
                let shortage = 1_400 + (trial * 173) % 1_900;
                [5_100 - shortage, shortage, 2_500]
            } else {
                [
                    2_100 + generation * 90 + schedule * 400,
                    1_800 + generation % 4 * 280 + schedule * 470,
                    2_500,
                ]
            };
            let outcome = run(world.clone(), market, durations, None);
            for (score, trial) in fitness.iter_mut().zip(outcome.fitness) {
                *score += if matched {
                    // Give shortage and recovery equal weight despite differing durations.
                    (trial[1] as f64 / durations[1] as f64 + trial[2] as f64 / durations[2] as f64)
                        * 0.5
                } else {
                    trial.into_iter().sum::<usize>() as f64
                };
            }
        }
        let mut cohort: Vec<_> = (0..initial.agents.len())
            .filter(|&i| initial.agents.home[i] == market as u32)
            .map(|i| (i, rng.next_u64()))
            .collect();
        cohort.sort_by(|&(a, tie_a), &(b, tie_b)| {
            fitness[a].total_cmp(&fitness[b]).then(tie_a.cmp(&tie_b))
        });
        let elites: Vec<_> = cohort[..cohort.len() / 4]
            .iter()
            .map(|&(i, _)| weights[i])
            .collect();
        weights = weights
            .iter()
            .enumerate()
            .map(|(slot, _)| {
                if slot < elites.len() {
                    return elites[slot];
                }
                let a = elites[rng.index(elites.len())];
                let b = elites[rng.index(elites.len())];
                std::array::from_fn(|k| {
                    let inherited = if rng.index(2) == 0 { a[k] } else { b[k] };
                    if rng.unit() < 0.5 {
                        (inherited + rng.signed() * 0.14).clamp(-0.65, 0.65)
                    } else {
                        inherited
                    }
                })
            })
            .collect();
    }
    weights
}

fn probe_metrics(brains: &[Compiled], probes: &[Probe]) -> (f64, f64) {
    let histories = [
        [[0.0; 4]; 2],
        [[-0.8, 0.7, 0.3, 0.5], [0.6, 0.7, 0.2, 0.5]],
        [[0.6, 0.7, 0.2, 0.5], [-0.8, 0.7, 0.3, 0.5]],
    ];
    let mut disagreement = 0.0;
    let mut switched = 0;
    for [buy, forage] in probes {
        let mut buy_votes = [0; 3];
        for brain in brains {
            let choices = histories.map(|[m_buy, m_forage]| {
                brain.evaluate_with_memory(buy, &m_buy)
                    > brain.evaluate_with_memory(forage, &m_forage)
            });
            for (votes, choice) in buy_votes.iter_mut().zip(choices) {
                *votes += usize::from(choice);
            }
            switched += usize::from(choices[1] != choices[2]);
        }
        for n in buy_votes {
            disagreement +=
                (2 * n * (brains.len() - n)) as f64 / (brains.len() * (brains.len() - 1)) as f64;
        }
    }
    let count = probes.len().max(1);
    (
        disagreement / (3 * count) as f64,
        switched as f64 / (count * brains.len()) as f64,
    )
}

fn main() {
    let mode = std::env::args().nth(1);
    if matches!(mode.as_deref(), Some("evolve" | "matched")) {
        let matched = mode.as_deref() == Some("matched");
        println!("seed,policy,stockout_hungry,restock_hungry,restock_purchases,restock_first_buy,probes,pair_disagreement,history_switch");
        for seed in SEEDS {
            let initial = World::new(128, seed).unwrap();
            let single = evolve(&initial, 1, matched);
            let multi = evolve(&initial, 3, matched);
            let neutral = vec![[0.0; MEMORY_INPUTS]; initial.genomes.len()];
            let fixed = vec![ACTIVE; initial.genomes.len()];
            let mut probes = Vec::new();
            for (label, weights) in [
                ("neutral", neutral),
                ("fixed", fixed),
                ("single", single),
                ("multi", multi),
            ] {
                let mut world = initial.clone();
                apply_weights(&mut world, &weights);
                let held_out = [
                    (0, [3_380, 3_640, 3_000]),
                    (1, [1_740, 1_100, 3_000]),
                    (2, [4_700, 4_850, 3_000]),
                ];
                let mut totals = [0u64; 3];
                let mut recovery = Vec::new();
                for (market, schedule) in held_out {
                    let outcome = run(
                        world.clone(),
                        market,
                        schedule,
                        (label == "neutral").then_some(&mut probes),
                    );
                    let [_, stockout, restocked] = outcome.phases;
                    totals[0] += stockout.hungry as u64;
                    totals[1] += restocked.hungry as u64;
                    totals[2] += restocked.purchases;
                    recovery.push(
                        restocked
                            .first_purchase
                            .map_or("-".to_string(), |tick| tick.to_string()),
                    );
                }
                let (disagreement, switched) = probe_metrics(&world.brains, &probes);
                let [stockout, restock, purchases] = totals;
                println!("{seed},{label},{stockout},{restock},{purchases},{},{},{disagreement:.4},{switched:.4}",
                    recovery.join("/"), probes.len());
            }
        }
        return;
    }

    println!("seed,policy,phase,purchases,forage_ticks,hungry_agent_ticks,first_purchase_tick");
    for seed in SEEDS {
        let initial = World::new(128, seed).unwrap();
        for (policy, weights) in [("neutral", [0.0; MEMORY_INPUTS]), ("active", ACTIVE)] {
            let mut world = initial.clone();
            apply_weights(&mut world, &vec![weights; initial.genomes.len()]);
            let outcome = run(world, 0, [3_000; 3], None);
            for (name, phase) in ["normal", "stockout", "restocked"]
                .into_iter()
                .zip(outcome.phases)
            {
                let first = phase
                    .first_purchase
                    .map(|tick| tick.to_string())
                    .unwrap_or_default();
                println!(
                    "{seed},{policy},{name},{},{},{},{first}",
                    phase.purchases, phase.forage, phase.hungry
                );
            }
        }
    }
}
