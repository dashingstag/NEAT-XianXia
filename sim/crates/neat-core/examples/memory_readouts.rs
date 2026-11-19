//! Measure the incremental value of candidate-specific memory readouts.
use neat_core::{Compiled, Genome, Innovations, Rng, INPUTS};
use std::time::Instant;

const POP: usize = 40;
const GENERATIONS: usize = 40;
const ROUNDS: usize = 64;
const ACTIONS: usize = 3;
const SLOTS: [[usize; 4]; 3] = [[7, 8, 9, 12], [13, 14, 17, 18], [20, 21, 22, 23]];
const CONTEXT: [[usize; 3]; 3] = [[24, 25, 26], [27, 28, 29], [30, 32, 33]];

struct Round {
    offers: [[f32; INPUTS]; ACTIONS],
    outcomes: [[f32; 3]; ACTIONS],
    priorities: [f32; 3],
}

fn episode(seed: u64, pressure: [f32; 3]) -> Vec<Round> {
    let mut rng = Rng(seed);
    let periods = [5, 13, 27];
    let mut regimes = [[0.0; 3]; ACTIONS];
    (0..ROUNDS)
        .map(|tick| {
            let priorities = std::array::from_fn(|k| pressure[k] * (0.25 + 0.75 * rng.unit()));
            let mut offers = [[0.0; INPUTS]; ACTIONS];
            let mut outcomes = [[0.0; 3]; ACTIONS];
            for action in 0..ACTIONS {
                offers[action][0] = 0.5;
                offers[action][1] = 0.4 + 0.2 * rng.unit();
                offers[action][3] = 0.2 + 0.2 * rng.unit();
                offers[action][[6, 15, 31][action]] = 1.0;
                for k in 0..3 {
                    if tick % periods[k] == 0 {
                        regimes[action][k] = 0.42 * rng.signed();
                    }
                    outcomes[action][k] = regimes[action][k] + 0.05 * rng.signed();
                    offers[action][CONTEXT[k][action]] = priorities[k];
                }
            }
            Round {
                offers,
                outcomes,
                priorities,
            }
        })
        .collect()
}

#[derive(Clone, Copy, Default)]
struct Horizon {
    weighted: f32,
    confidence: f32,
}

impl Horizon {
    fn update(&mut self, observation: Option<f32>, gain: f32, decay: f32) {
        if let Some(value) = observation {
            self.weighted = (1.0 - gain) * self.weighted + gain * value;
            self.confidence = (1.0 - gain) * self.confidence + gain;
        } else {
            self.weighted *= decay;
            self.confidence *= decay;
        }
    }

    fn read(self) -> [f32; 2] {
        if self.confidence < 0.01 {
            [0.0; 2]
        } else {
            [
                (2.0 * self.weighted / self.confidence).clamp(-1.0, 1.0),
                self.confidence,
            ]
        }
    }
}

fn rollout(brain: &Compiled, mask: usize, rounds: &[Round]) -> (f64, f64) {
    let pooled = mask == 8;
    let active: Vec<_> = (0..3)
        .filter(|&k| (pooled && k == 0) || mask & (1 << k) != 0)
        .collect();
    let mut memory = [[[Horizon::default(); 2]; 3]; ACTIONS];
    let mut payoff = 0.0;
    let mut regret = 0.0;
    for round in rounds {
        let mut values = [0.0; ACTIONS];
        let mut scores = [0.0; ACTIONS];
        for (action, offer) in round.offers.iter().enumerate() {
            let mut inputs = *offer;
            values[action] = f64::from(
                offer[1] - 0.4 * offer[3]
                    + round.outcomes[action]
                        .iter()
                        .zip(round.priorities)
                        .map(|(value, priority)| value * priority)
                        .sum::<f32>(),
            );
            for &k in &active {
                let [short, long] = memory[action][k];
                for (slot, value) in SLOTS[k]
                    .into_iter()
                    .zip(short.read().into_iter().chain(long.read()))
                {
                    inputs[slot] = value;
                }
            }
            scores[action] = brain.evaluate(&inputs);
        }
        let chosen = (0..ACTIONS)
            .max_by(|&a, &b| scores[a].total_cmp(&scores[b]))
            .unwrap();
        payoff += values[chosen];
        regret += values.into_iter().fold(f64::NEG_INFINITY, f64::max) - values[chosen];
        for (action, channels) in memory.iter_mut().enumerate() {
            for &k in &active {
                let outcome = (action == chosen).then(|| {
                    if pooled {
                        let [stock, fulfillment, _] = round.outcomes[action];
                        (stock * round.priorities[0] + fulfillment * round.priorities[1])
                            / (round.priorities[0] + round.priorities[1])
                    } else {
                        round.outcomes[action][k]
                    }
                });
                channels[k][0].update(outcome, 0.35, 0.80);
                channels[k][1].update(outcome, 0.07, 0.97);
            }
        }
    }
    (payoff / rounds.len() as f64, regret / rounds.len() as f64)
}

fn evolve(seed: u64, mask: usize, pressure: [f32; 3]) -> Vec<Compiled> {
    evolve_with(
        seed,
        |seed| episode(seed, pressure),
        |brain, rounds| rollout(brain, mask, rounds).0,
    )
}

fn evolve_with<T>(
    seed: u64,
    make_episode: impl Fn(u64) -> T,
    evaluate: impl Fn(&Compiled, &T) -> f64,
) -> Vec<Compiled> {
    let mut rng = Rng(seed);
    let mut innovations = Innovations::default();
    let mut population: Vec<_> = (0..POP)
        .map(|_| {
            let mut genome = Genome::seeded(&mut rng, &mut innovations);
            for gene in &mut genome.genes {
                if SLOTS.iter().flatten().any(|&slot| slot as u32 == gene.from) {
                    gene.weight = 0.0;
                }
            }
            genome
        })
        .collect();
    for generation in 0..GENERATIONS {
        let training =
            [0, 1, 2].map(|offset| make_episode(seed * 1000 + generation as u64 * 3 + offset));
        let mut ranking: Vec<_> = population
            .iter()
            .enumerate()
            .map(|(i, genome)| {
                let brain = genome.compile().unwrap();
                let fitness: f64 = training.iter().map(|rounds| evaluate(&brain, rounds)).sum();
                (i, fitness)
            })
            .collect();
        ranking.sort_by(|a, b| b.1.total_cmp(&a.1));
        population = (0..POP)
            .map(|_| {
                let a = ranking[rng.index(POP / 4)].0;
                let b = ranking[rng.index(POP / 4)].0;
                let mut child = Genome::crossover(&population[a], &population[b], &mut rng);
                child.mutate_weights(&mut rng, 0.15);
                if rng.index(20) == 0 {
                    child.add_node(&mut rng, &mut innovations);
                }
                child
            })
            .collect();
    }
    population
        .iter()
        .map(|genome| genome.compile().unwrap())
        .collect()
}

fn main() {
    const SEEDS: [u64; 8] = [11, 23, 35, 47, 59, 71, 83, 95];
    let filter = std::env::args().nth(1);
    if matches!(filter.as_deref(), Some("travel" | "travel-shift")) {
        travel_experiment(filter.as_deref() == Some("travel-shift"));
        return;
    }
    println!("scenario,readouts,seed,payoff,oracle_regret,ns_per_decision");
    for (scenario, pressure) in [
        ("three_factors", [1.0, 1.0, 1.0]),
        ("one_factor", [1.0, 0.0, 0.0]),
        ("market", [1.0, 1.0, 0.0]),
    ] {
        if filter.as_deref().is_some_and(|name| name != scenario) {
            continue;
        }
        let counts = if scenario == "market" { 0..=2 } else { 0..=3 };
        for count in counts {
            let masks: Vec<usize> = if scenario == "three_factors" {
                (0usize..8)
                    .filter(|mask| mask.count_ones() == count)
                    .collect()
            } else if scenario == "market" && count == 1 {
                vec![8]
            } else {
                vec![(1 << count) - 1]
            };
            let mut summary = [0.0; 3];
            for seed in SEEDS {
                let held_out = [0, 1, 2, 3, 4, 5, 6, 7]
                    .map(|offset| episode(900_000 + seed * 10 + offset, pressure));
                let mut seed_metrics = [0.0; 3];
                for &mask in &masks {
                    let brains = evolve(seed, mask, pressure);
                    let start = Instant::now();
                    let (mut payoff, mut regret) = (0.0, 0.0);
                    for brain in &brains {
                        for rounds in &held_out {
                            let (p, r) = rollout(brain, mask, rounds);
                            payoff += p;
                            regret += r;
                        }
                    }
                    let episodes = (brains.len() * held_out.len()) as f64;
                    let values = [
                        payoff / episodes,
                        regret / episodes,
                        start.elapsed().as_nanos() as f64 / (episodes * ROUNDS as f64),
                    ];
                    for (total, value) in seed_metrics.iter_mut().zip(values) {
                        *total += value / masks.len() as f64;
                    }
                }
                for (total, value) in summary.iter_mut().zip(seed_metrics) {
                    *total += value / SEEDS.len() as f64;
                }
                report(scenario, count, &seed.to_string(), seed_metrics);
            }
            report(scenario, count, "mean", summary);
        }
    }
}

fn report(scenario: &str, count: u32, seed: &str, [payoff, regret, timing]: [f64; 3]) {
    println!("{scenario},{count},{seed},{payoff:.5},{regret:.5},{timing:.1}");
}

// Three persistent destinations. Their stock, fulfillment and local income vary
// independently. Route times are symmetric, and the agent chooses only while idle.
fn travel_episode(seed: u64, rotating: bool) -> (Vec<Round>, [[usize; ACTIONS]; ACTIONS], usize) {
    let mut rng = Rng(seed ^ 0xfe04);
    let mut routes = [[0; ACTIONS]; ACTIONS];
    for a in 0..ACTIONS {
        for b in a + 1..ACTIONS {
            let cost = 1 + rng.index(3);
            routes[a][b] = cost;
            routes[b][a] = cost;
        }
    }
    let mut rounds = episode(seed, [1.0; 3]);
    let start = rng.index(ACTIONS);
    if rotating {
        let first = rng.index(ACTIONS);
        for (tick, round) in rounds.iter_mut().enumerate() {
            let favored = (first + tick / 16) % ACTIONS;
            for (site, outcomes) in round.outcomes.iter_mut().enumerate() {
                for value in outcomes {
                    let shift = if site == favored { 0.28 } else { -0.24 };
                    *value = (*value + shift).clamp(-0.55, 0.55);
                }
            }
        }
    }
    (rounds, routes, start)
}

#[derive(Default)]
struct TravelMetrics {
    payoff: f64,
    trips: usize,
    visits: usize,
    travel_ticks: usize,
    visited: [bool; ACTIONS],
}

fn travel_rollout(
    brain: &Compiled,
    mask: usize,
    world: &(Vec<Round>, [[usize; ACTIONS]; ACTIONS], usize),
) -> TravelMetrics {
    const SERVICE: usize = 2;
    let (rounds, routes, start) = world;
    let readouts = match mask {
        0 => 0,
        8 => 1,
        7 => 3,
        _ => unreachable!(),
    };
    let mut position = *start;
    let mut tick = 0;
    let mut memory = [[[Horizon::default(); 2]; 3]; ACTIONS];
    let mut metrics = TravelMetrics::default();

    while tick + SERVICE <= rounds.len() {
        let round = &rounds[tick];
        let destination = (0..ACTIONS)
            .filter(|&site| tick + routes[position][site] + SERVICE <= rounds.len())
            .map(|site| {
                let mut inputs = round.offers[site];
                inputs[3] = routes[position][site] as f32 / 10.0;
                for (k, slots) in SLOTS.iter().enumerate().take(readouts) {
                    let [short, long] = memory[site][k];
                    for (slot, value) in slots
                        .iter()
                        .zip(short.read().into_iter().chain(long.read()))
                    {
                        inputs[*slot] = value;
                    }
                }
                (site, brain.evaluate(&inputs))
            })
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .unwrap()
            .0;
        let travel = routes[position][destination];
        metrics.trips += usize::from(travel > 0);
        metrics.travel_ticks += travel;
        metrics.visits += 1;
        metrics.visited[destination] = true;
        tick += travel + SERVICE;
        position = destination;

        let result = &rounds[tick - 1];
        let outcomes = result.outcomes[destination];
        // Score the visit against the priorities the agent saw when it committed.
        let impact: f32 = outcomes
            .iter()
            .zip(round.priorities)
            .map(|(outcome, priority)| outcome * priority)
            .sum();
        metrics.payoff += f64::from(round.offers[destination][1] + impact - 0.025 * travel as f32);
        for (site, records) in memory.iter_mut().enumerate() {
            for (k, [short, long]) in records.iter_mut().enumerate().take(readouts) {
                short.update(None, 0.35, 0.80f32.powi((travel + SERVICE) as i32));
                long.update(None, 0.07, 0.97f32.powi((travel + SERVICE) as i32));
                if site == destination {
                    let observation = if readouts == 1 {
                        impact / round.priorities.iter().sum::<f32>()
                    } else {
                        outcomes[k]
                    };
                    short.update(Some(observation), 0.35, 0.80);
                    long.update(Some(observation), 0.07, 0.97);
                }
            }
        }
    }
    metrics.payoff /= rounds.len() as f64;
    metrics
}

fn travel_experiment(rotating: bool) {
    let seeds = [11, 23, 35];
    println!("readouts,seed,payoff_per_tick,trips_per_episode,visits_per_episode,sites_per_episode,travel_tick_share");
    for (readouts, mask) in [(0, 0), (1, 8), (3, 7)] {
        let mut aggregate = [0.0; 5];
        for seed in seeds {
            let brains = evolve_with(
                seed,
                |seed| travel_episode(seed, rotating),
                |brain, world| travel_rollout(brain, mask, world).payoff,
            );
            let worlds: Vec<_> = (0..24)
                .map(|offset| travel_episode(2_000_000 + seed * 100 + offset, rotating))
                .collect();
            let mut values = [0.0; 5];
            for brain in &brains {
                for world in &worlds {
                    let m = travel_rollout(brain, mask, world);
                    values[0] += m.payoff;
                    values[1] += m.trips as f64;
                    values[2] += m.visits as f64;
                    values[3] += m.visited.into_iter().filter(|&visited| visited).count() as f64;
                    values[4] += m.travel_ticks as f64 / world.0.len() as f64;
                }
            }
            let samples = (brains.len() * worlds.len()) as f64;
            let values = values.map(|v| v / samples);
            for (total, value) in aggregate.iter_mut().zip(values) {
                *total += value / seeds.len() as f64;
            }
            report_travel(readouts, &seed.to_string(), values);
        }
        report_travel(readouts, "mean", aggregate);
    }
}

fn report_travel(readouts: usize, seed: &str, metrics: [f64; 5]) {
    print!("{readouts},{seed}");
    for metric in metrics {
        print!(",{metric:.5}");
    }
    println!();
}
