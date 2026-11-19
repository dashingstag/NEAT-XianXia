//! Paired disruption trials on the mature banking checkpoint with real memory inputs.
use neat_core::{Compiled, INPUTS};
use sim_core::World;
use std::collections::VecDeque;
use std::path::Path;

type Offer = ([f32; INPUTS], bool);
type Probe = [Offer; 2];

fn apply(world: &mut World, weights: [f32; 4]) {
    for (genome, brain) in world.genomes.iter_mut().zip(&mut world.brains) {
        for (slot, weight) in weights.into_iter().enumerate() {
            let source = u32::MAX - 3 + slot as u32;
            let edge = genome
                .genes
                .iter_mut()
                .find(|gene| gene.from == source)
                .unwrap();
            edge.weight = weight;
        }
        *brain = genome.compile().unwrap();
    }
}

fn disruption(
    world: &mut World,
    market: usize,
    goods: &[usize],
    held: &mut [VecDeque<sim_core::Lot>],
) {
    for (&kind, lots) in goods.iter().zip(held) {
        world.ledger.consumed[kind] += world.markets[market].stock(kind);
        world.markets[market].quotes[kind].offered = 0;
        lots.append(&mut world.markets[market].goods[kind]);
    }
}

fn restore(
    world: &mut World,
    market: usize,
    goods: &[usize],
    held: &mut [VecDeque<sim_core::Lot>],
) {
    for (&kind, lots) in goods.iter().zip(held) {
        let quantity: u64 = lots.iter().map(|lot| lot.quantity).sum();
        world.ledger.consumed[kind] -= quantity;
        world.markets[market].goods[kind].append(lots);
        world.markets[market].quotes[kind].offered += quantity;
    }
}

fn capture(world: &World, people: &[usize], probes: &mut Vec<Probe>, sample: usize) {
    for offset in 0..people.len() {
        let i = people[(sample + offset) % people.len()];
        if !world.agents.alive[i] {
            continue;
        }
        let trace = world.trace(i).unwrap();
        let Some(first) = trace.choices.first() else {
            continue;
        };
        let Some(second) = trace
            .choices
            .iter()
            .find(|other| other.action != first.action)
        else {
            continue;
        };
        if first.memory_inputs.is_none() && second.memory_inputs.is_none() {
            continue;
        }
        probes.push([first, second].map(|offer| {
            (
                offer.inputs.as_slice().try_into().unwrap(),
                offer.memory_inputs.is_some(),
            )
        }));
        break;
    }
}

fn disagreement(brains: &[Compiled], probes: &[Probe]) -> f64 {
    let mut sum = 0.0;
    for offers in probes {
        for (positive, aligned) in [(0, true), (1, true), (0, false), (1, false)] {
            let votes = brains
                .iter()
                .filter(|brain| {
                    let scores: [f32; 2] = std::array::from_fn(|i| {
                        let (inputs, reads_memory) = &offers[i];
                        let outcome = if positive == i { 0.7 } else { -0.7 };
                        let memory = if *reads_memory {
                            [outcome, 0.7, if aligned { outcome } else { -outcome }, 0.6]
                        } else {
                            [0.0; 4]
                        };
                        brain.evaluate_with_memory(inputs, &memory)
                    });
                    scores[0] >= scores[1]
                })
                .count();
            let n = brains.len();
            sum += (2 * votes * (n - votes)) as f64 / (n * (n - 1)) as f64;
        }
    }
    sum / (4 * probes.len()) as f64
}

fn main() {
    let initial = World::load(Path::new("fixtures/bank-v12.snapshot")).unwrap();
    let market = initial
        .urban
        .buildings
        .iter()
        .fold(
            vec![0usize; initial.markets.len()],
            |mut counts, building| {
                counts[building.market as usize] += 1;
                counts
            },
        )
        .into_iter()
        .enumerate()
        .max_by_key(|&(_, count)| count)
        .unwrap()
        .0;
    let people: Vec<_> = (0..initial.agents.len())
        .filter(|&i| initial.agents.alive[i] && initial.agents.home[i] == market as u32)
        .collect();
    if std::env::args().nth(1).as_deref() == Some("survival") {
        survival_experiment(&initial, market);
        return;
    }
    eprintln!(
        "seed={}, market={}, cohort={}",
        initial.seed,
        market,
        people.len()
    );

    println!("shock,policy,phase,hunger,dead_agent_ticks,living,trades,shipments,urban_jobs,loan_approvals,principal_paid");
    for (scenario, goods) in [
        ("food", &[0, 3, 8][..]),
        ("lumber", &[1][..]),
        ("combined", &[0, 1, 3, 8][..]),
    ] {
        let mut probes = Vec::new();
        for (policy, weights) in [
            ("neutral", [0.0; 4]),
            ("short", [0.18, 0.0, 0.0, 0.0]),
            ("long", [0.0, 0.0, 0.18, 0.0]),
            ("both", [0.12, 0.0, 0.06, 0.0]),
        ] {
            let mut world = initial.clone();
            apply(&mut world, weights);
            let mut held = vec![VecDeque::new(); goods.len()];
            for (phase, duration) in [("warmup", 1_000), ("shutdown", 1_200), ("recovery", 1_800)] {
                if phase == "recovery" {
                    restore(&mut world, market, goods, &mut held);
                }
                let before = world.stats.clone();
                let jobs = world.urban.completed;
                let approvals = world.banking.approved.values().sum::<u64>();
                let principal = world.banking.principal_paid;
                let mut hunger = 0;
                let mut dead_ticks = 0;
                for tick in 0..duration {
                    if phase == "shutdown" {
                        disruption(&mut world, market, goods, &mut held);
                    }
                    world.step(false);
                    dead_ticks += people.iter().filter(|&&i| !world.agents.alive[i]).count();
                    hunger += people
                        .iter()
                        .filter(|&&i| world.agents.alive[i] && world.agents.hunger[i] >= 55.0)
                        .count();
                    if policy == "neutral" && phase == "recovery" && tick % 60 == 0 {
                        capture(&world, &people, &mut probes, tick / 60);
                    }
                }
                world.check().unwrap();
                let living = people.iter().filter(|&&i| world.agents.alive[i]).count();
                println!(
                    "{scenario},{policy},{phase},{hunger},{dead_ticks},{living},{},{},{},{},{}",
                    world.stats.trades - before.trades,
                    world.stats.shipments - before.shipments,
                    world.urban.completed - jobs,
                    world.banking.approved.values().sum::<u64>() - approvals,
                    world.banking.principal_paid - principal
                );
            }
            println!(
                "probe,{scenario},{policy},{},{:.5}",
                probes.len(),
                disagreement(&world.brains, &probes)
            );
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Recovery {
    deaths: usize,
    hungry_ticks: usize,
    dead_ticks: usize,
    shipments: u64,
}

impl std::ops::AddAssign for Recovery {
    fn add_assign(&mut self, other: Self) {
        self.deaths += other.deaths;
        self.hungry_ticks += other.hungry_ticks;
        self.dead_ticks += other.dead_ticks;
        self.shipments += other.shipments;
    }
}

fn recovery(initial: &World, market: usize, goods: &[usize], weights: [f32; 4]) -> Recovery {
    let mut world = initial.clone();
    apply(&mut world, weights);
    let people: Vec<_> = (0..world.agents.len())
        .filter(|&i| world.agents.alive[i] && world.agents.home[i] == market as u32)
        .collect();
    let mut held = vec![VecDeque::new(); goods.len()];
    let mut result = Recovery::default();
    for _ in 0..650 {
        world.step(false);
    }
    for _ in 0..900 {
        disruption(&mut world, market, goods, &mut held);
        world.step(false);
    }
    world.check().unwrap();
    restore(&mut world, market, goods, &mut held);
    let start_shipments = world.stats.shipments;
    for _ in 0..1350 {
        world.step(false);
        for &person in &people {
            if !world.agents.alive[person] {
                result.dead_ticks += 1;
            } else if world.agents.hunger[person] >= 55.0 {
                result.hungry_ticks += 1;
            }
        }
    }
    result.deaths = people
        .iter()
        .filter(|&&person| !world.agents.alive[person])
        .count();
    result.shipments = world.stats.shipments - start_shipments;
    world.check().unwrap();
    result
}

fn continuation(initial: &World, ticks: usize) -> World {
    let mut world = initial.clone();
    for _ in 0..ticks {
        world.step(false);
    }
    world.check().unwrap();
    world
}

fn survival_experiment(initial: &World, market: usize) {
    let training_later = continuation(initial, 700);
    let held_first = continuation(initial, 1_600);
    let held_second = continuation(initial, 2_800);
    let training = [(initial, &[0, 3, 8][..]), (&training_later, &[1][..])];
    let held_out = [
        (&held_first, &[0, 1, 3, 8][..]),
        (&held_second, &[0, 3, 8][..]),
    ];
    let candidates = [
        ("neutral", [0.0; 4]),
        ("mild_short", [0.06, 0.0, 0.0, 0.0]),
        ("short", [0.18, 0.0, 0.0, 0.0]),
        ("long", [0.0, 0.0, 0.18, 0.0]),
        ("both", [0.12, 0.0, 0.06, 0.0]),
        ("mild_both", [0.04, 0.0, 0.02, 0.0]),
    ];
    println!("split,selection,weights,scenario,deaths,hungry_ticks,dead_ticks,shipments");
    let scores = candidates.map(|(name, weights)| {
        let mut total = Recovery::default();
        for (index, (checkpoint, goods)) in training.iter().enumerate() {
            let score = recovery(checkpoint, market, goods, weights);
            print_recovery("train", "-", name, &index.to_string(), score);
            total += score;
        }
        total
    });
    let hunger_first = (0..scores.len())
        .min_by_key(|&i| (scores[i].hungry_ticks, scores[i].deaths))
        .unwrap();
    let survival_first = (0..scores.len())
        .min_by_key(|&i| {
            (
                scores[i].deaths,
                scores[i].hungry_ticks + scores[i].dead_ticks,
            )
        })
        .unwrap();
    for (label, index) in [
        ("neutral", 0),
        ("hunger_first", hunger_first),
        ("survival_first", survival_first),
    ] {
        let (name, weights) = candidates[index];
        let mut total = Recovery::default();
        for (scenario, (checkpoint, goods)) in held_out.iter().enumerate() {
            let score = recovery(checkpoint, market, goods, weights);
            print_recovery("held", label, name, &scenario.to_string(), score);
            total += score;
        }
        print_recovery("held", label, name, "total", total);
    }
}

fn print_recovery(split: &str, label: &str, weights: &str, scenario: &str, score: Recovery) {
    println!(
        "{split},{label},{weights},{scenario},{},{},{},{}",
        score.deaths, score.hungry_ticks, score.dead_ticks, score.shipments
    );
}
