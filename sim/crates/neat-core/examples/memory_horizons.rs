//! Compare two time scales of outcome memory as candidate inputs to NEAT.
use neat_core::{Compiled, Genome, Innovations, Rng, INPUTS};

const POPULATION: usize = 40;
const GENERATIONS: usize = 40;
const ACTIONS: usize = 3;
const WARMUP: usize = 20;
const RECOVERY: usize = 18;
const BUY: usize = 1;
// These four production sensors are unused in this synthetic scenario.
const MEMORY_SLOTS: [usize; 4] = [12, 13, 14, 17];

#[derive(Clone, Copy)]
struct Mode {
    name: &'static str,
    short: bool,
    long: bool,
}

const MODES: [Mode; 4] = [
    Mode {
        name: "none",
        short: false,
        long: false,
    },
    Mode {
        name: "short",
        short: true,
        long: false,
    },
    Mode {
        name: "long",
        short: false,
        long: true,
    },
    Mode {
        name: "both",
        short: true,
        long: true,
    },
];

struct Round {
    offers: [[f32; INPUTS]; ACTIONS],
    surprise: [f32; ACTIONS],
}

fn episode(seed: u64, shortage: usize, loss: f32) -> Vec<Round> {
    let mut rng = Rng(seed);
    (0..WARMUP + shortage + RECOVERY)
        .map(|tick| {
            let in_shortage = (WARMUP..WARMUP + shortage).contains(&tick);
            let mut offers = [[0.0; INPUTS]; ACTIONS];
            let mut surprise = [0.0; ACTIONS];
            for (action, offer) in offers.iter_mut().enumerate() {
                offer[0] = 0.5;
                offer[1] = 0.35 + 0.30 * rng.unit();
                offer[3] = 0.15 + 0.30 * rng.unit();
                offer[[6, 15, 31][action]] = 1.0; // Forage, buy, farm.
                let bonus = match action {
                    BUY if in_shortage => -loss,
                    BUY => 0.25,
                    2 => 0.08,
                    _ => 0.02,
                };
                surprise[action] = bonus + 0.10 * rng.signed();
            }
            Round { offers, surprise }
        })
        .collect()
}

#[derive(Clone, Copy, Default)]
struct Estimate {
    sum: f32,
    confidence: f32,
}

impl Estimate {
    fn update(&mut self, observation: Option<f32>, gain: f32, decay: f32) {
        if let Some(value) = observation {
            self.sum = (1.0 - gain) * self.sum + gain * value;
            self.confidence = (1.0 - gain) * self.confidence + gain;
        } else {
            self.sum *= decay;
            self.confidence *= decay;
        }
    }

    fn inputs(self) -> [f32; 2] {
        if self.confidence < 0.01 {
            return [0.0; 2];
        }
        [
            (2.0 * self.sum / self.confidence).clamp(-1.0, 1.0),
            self.confidence,
        ]
    }
}

fn rollout(brain: &Compiled, mode: Mode, rounds: &[Round]) -> (f32, Vec<usize>) {
    let mut short = [Estimate::default(); ACTIONS];
    let mut long = [Estimate::default(); ACTIONS];
    let mut payoff = 0.0;
    let mut actions = Vec::with_capacity(rounds.len());
    for round in rounds {
        let action = (0..ACTIONS)
            .map(|action| {
                let mut offer = round.offers[action];
                let fast = if mode.short {
                    short[action].inputs()
                } else {
                    [0.0; 2]
                };
                let slow = if mode.long {
                    long[action].inputs()
                } else {
                    [0.0; 2]
                };
                for (slot, value) in MEMORY_SLOTS.into_iter().zip(fast.into_iter().chain(slow)) {
                    offer[slot] = value;
                }
                (action, brain.evaluate(&offer))
            })
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .unwrap()
            .0;
        actions.push(action);
        let chosen = &round.offers[action];
        payoff += chosen[1] - 0.4 * chosen[3] + round.surprise[action];
        for candidate in 0..ACTIONS {
            let observation = (candidate == action).then_some(round.surprise[action]);
            short[candidate].update(observation, 0.35, 0.80);
            long[candidate].update(observation, 0.07, 0.97);
        }
    }
    (payoff / rounds.len() as f32, actions)
}

fn evolve(seed: u64, mode: Mode) -> Vec<Genome> {
    let mut rng = Rng(seed);
    let mut innovations = Innovations::default();
    let mut population: Vec<_> = (0..POPULATION)
        .map(|_| {
            let mut genome = Genome::seeded(&mut rng, &mut innovations);
            for gene in &mut genome.genes {
                if MEMORY_SLOTS.contains(&(gene.from as usize)) {
                    gene.weight = 0.0;
                }
            }
            genome
        })
        .collect();

    for generation in 0..GENERATIONS {
        let training_seed = seed * 1000 + generation as u64 * 2;
        let training = [
            episode(training_seed, 4 + generation % 5, 0.30),
            episode(training_seed + 1, 18 + generation % 9, 0.35),
        ];
        let mut ranking: Vec<_> = population
            .iter()
            .enumerate()
            .map(|(index, genome)| {
                let brain = genome.compile().unwrap();
                let fitness: f32 = training
                    .iter()
                    .map(|rounds| rollout(&brain, mode, rounds).0)
                    .sum();
                (index, fitness)
            })
            .collect();
        ranking.sort_by(|a, b| b.1.total_cmp(&a.1));
        population = (0..POPULATION)
            .map(|_| {
                let a = ranking[rng.index(POPULATION / 4)].0;
                let b = ranking[rng.index(POPULATION / 4)].0;
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
}

fn score(population: &[Genome], mode: Mode, seed: u64, shortage: usize) -> [f64; 6] {
    let rounds = episode(800_000 + seed, shortage, 0.43);
    let mut votes = vec![[0usize; ACTIONS]; rounds.len()];
    let mut payoff = 0.0;
    for genome in population {
        let (reward, choices) = rollout(&genome.compile().unwrap(), mode, &rounds);
        payoff += f64::from(reward);
        for (counts, action) in votes.iter_mut().zip(choices) {
            counts[action] += 1;
        }
    }
    let buy_share = |start: usize, end: usize| {
        votes[start..end]
            .iter()
            .map(|counts| counts[BUY])
            .sum::<usize>() as f64
            / (population.len() * (end - start)) as f64
    };
    let disagreement: f64 = votes
        .iter()
        .map(|counts| {
            let agreeing = counts
                .iter()
                .map(|&n| n * n.saturating_sub(1))
                .sum::<usize>();
            1.0 - agreeing as f64 / (population.len() * (population.len() - 1)) as f64
        })
        .sum();
    let restock = WARMUP + shortage;
    [
        payoff / population.len() as f64,
        buy_share(WARMUP - 5, WARMUP),
        buy_share(WARMUP, WARMUP + shortage.min(5)),
        buy_share(restock, restock + 5),
        buy_share(rounds.len() - 5, rounds.len()),
        disagreement / rounds.len() as f64,
    ]
}

fn main() {
    let seeds = [11, 23, 35, 47, 59, 71, 83, 95];
    let tests = [("brief", 5), ("prolonged", 28)];
    println!("mode,shortage,seed,payoff,buy_before,buy_early_shortage,buy_early_restock,buy_late_restock,disagreement");
    for mode in MODES {
        let mut totals = [[0.0; 6]; 2];
        for seed in seeds {
            let population = evolve(seed, mode);
            for (index, (label, duration)) in tests.into_iter().enumerate() {
                let values = score(&population, mode, seed, duration);
                for (total, value) in totals[index].iter_mut().zip(values) {
                    *total += value;
                }
                report(mode.name, label, &seed.to_string(), values);
            }
        }
        for ((label, _), totals) in tests.into_iter().zip(totals) {
            report(
                mode.name,
                label,
                "mean",
                totals.map(|total| total / seeds.len() as f64),
            );
        }
    }
}

fn report(mode: &str, shortage: &str, seed: &str, metrics: [f64; 6]) {
    print!("{mode},{shortage},{seed}");
    for metric in metrics {
        print!(",{metric:.4}");
    }
    println!();
}
