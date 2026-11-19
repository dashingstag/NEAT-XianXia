//! Controlled comparison of brain adaptation strategies using real NEAT genomes.
use neat_core::{Compiled, Genome, Innovations, Rng, INPUTS};

const POP: usize = 48;
const GENERATIONS: usize = 36;
const SHIFT: usize = 24;
const STEPS: usize = 30;
const ACTIONS: usize = 3;
const BINS: usize = 27;

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Baseline,
    Symmetric,
    Memory,
    Archive,
}

#[derive(Clone, Copy)]
struct Round {
    inputs: [[f32; INPUTS]; ACTIONS],
    reward: [f32; ACTIONS],
}

fn scenario(seed: u64, regime: [f32; ACTIONS]) -> Vec<Round> {
    let mut rng = Rng(seed);
    (0..STEPS)
        .map(|_| {
            let mut inputs = [[0.0; INPUTS]; ACTIONS];
            let mut reward = [0.0; ACTIONS];
            for (a, input) in inputs.iter_mut().enumerate() {
                let travel = 0.15 + 0.35 * rng.unit();
                let benefit = 0.35 + 0.30 * rng.unit();
                input[0] = 0.5;
                input[1] = benefit;
                input[3] = travel;
                // Food, wallet and farmer alignment distinguish forage, trade and farm.
                input[[6, 15, 31][a]] = 1.0;
                reward[a] = benefit - travel * 0.4 + regime[a] + rng.signed() * 0.12;
            }
            Round { inputs, reward }
        })
        .collect()
}

fn run(brain: &Compiled, rounds: &[Round], memory: bool) -> (f32, [usize; ACTIONS]) {
    let mut estimate = [0.0; ACTIONS];
    let mut counts = [0; ACTIONS];
    let mut total = 0.0;
    for round in rounds {
        let action = (0..ACTIONS)
            .map(|a| {
                let bonus = if memory {
                    estimate[a] + 0.18 / (1.0 + counts[a] as f32)
                } else {
                    0.0
                };
                (a, brain.evaluate(&round.inputs[a]) + bonus)
            })
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .unwrap()
            .0;
        counts[action] += 1;
        let payoff = round.reward[action];
        total += payoff;
        let visible = round.inputs[action][1] - round.inputs[action][3] * 0.4;
        estimate[action] = (estimate[action] * 0.65 + (payoff - visible) * 0.35).clamp(-0.4, 0.4);
    }
    (total / rounds.len() as f32, counts)
}

fn niche(counts: [usize; ACTIONS]) -> usize {
    let bucket = |count: usize| (count * 3 / (STEPS + 1)).min(2);
    bucket(counts[0]) * 9 + bucket(counts[1]) * 3 + bucket(counts[2])
}

#[derive(Default)]
struct Metrics {
    local_before: f64,
    local_early: f64,
    local_late: f64,
    transfer: f64,
    niches: f64,
    disagreement: f64,
}

fn trial(seed: u64, mode: Mode) -> Metrics {
    let mut genetics = Rng(seed);
    let mut selection = Rng(seed ^ 0x8502);
    let mut innovations = Innovations::default();
    let mut population: Vec<_> = (0..POP)
        .map(|_| Genome::seeded(&mut genetics, &mut innovations))
        .collect();
    let probes = scenario(501, [0.0; ACTIONS]);
    let transfer = [
        scenario(seed ^ 8000, [0.24, -0.25, 0.02]),
        scenario(seed ^ 6000, [-0.26, 0.07, 0.24]),
    ];
    let mut archive: [Option<(f32, Genome)>; BINS] = std::array::from_fn(|_| None);
    let mut metrics = Metrics::default();
    let memory = mode == Mode::Memory;

    for generation in 0..GENERATIONS {
        let regime = if generation < SHIFT {
            [0.0, 0.22, 0.04]
        } else {
            [0.04, -0.30, 0.19]
        };
        let rounds = scenario(seed * 1000 + generation as u64, regime);
        let mut ratings = Vec::new();
        let mut occupied = [false; BINS];
        let mut votes = [[0usize; ACTIONS]; STEPS];
        for genome in &population {
            let brain = genome.compile().unwrap();
            let (score, _) = run(&brain, &rounds, memory);
            let (_, counts) = run(&brain, &probes, false);
            let bin = niche(counts);
            occupied[bin] = true;
            ratings.push(score);
            // Frozen-policy probes measure inherited diversity independently of memory.
            for (round, votes) in probes.iter().zip(&mut votes) {
                let action = (0..ACTIONS)
                    .max_by(|&a, &b| {
                        brain
                            .evaluate(&round.inputs[a])
                            .total_cmp(&brain.evaluate(&round.inputs[b]))
                    })
                    .unwrap();
                votes[action] += 1;
            }
            if mode == Mode::Archive {
                // Score archived strategies across fixed environments, including scarce trade.
                // Reuse the probe inputs so scores remain comparable across generations.
                let quality = [[0.0, 0.22, 0.04], [0.04, -0.18, 0.13], [0.15, 0.0, -0.05]]
                    .into_iter()
                    .map(|regime| run(&brain, &scenario(501, regime), false).0)
                    .sum::<f32>()
                    / 3.0;
                if archive[bin]
                    .as_ref()
                    .is_none_or(|(best, _)| quality > *best)
                {
                    archive[bin] = Some((quality, genome.clone()));
                }
            }
            if generation >= GENERATIONS - 4 {
                metrics.transfer += transfer
                    .iter()
                    .map(|episode| run(&brain, episode, memory).0 as f64)
                    .sum::<f64>()
                    / (4 * POP * transfer.len()) as f64;
            }
        }
        let avg = ratings.iter().map(|&score| score as f64).sum::<f64>() / POP as f64;
        match generation {
            g if (SHIFT - 4..SHIFT).contains(&g) => metrics.local_before += avg / 4.0,
            g if (SHIFT..SHIFT + 4).contains(&g) => metrics.local_early += avg / 4.0,
            g if g >= GENERATIONS - 4 => {
                metrics.local_late += avg / 4.0;
                metrics.niches += occupied.iter().filter(|&&used| used).count() as f64 / 4.0;
                metrics.disagreement += votes
                    .iter()
                    .map(|counts| {
                        let agreeing: usize = counts.iter().map(|&n| n * n.saturating_sub(1)).sum();
                        1.0 - agreeing as f64 / (POP * (POP - 1)) as f64
                    })
                    .sum::<f64>()
                    / (4 * STEPS) as f64;
            }
            _ => {}
        }

        let mut ranked: Vec<_> = (0..POP).collect();
        ranked.sort_by(|&a, &b| ratings[b].total_cmp(&ratings[a]));
        let elites: Vec<_> = archive.iter().flatten().map(|(_, genome)| genome).collect();
        population = (0..POP)
            .map(|slot| {
                let first = selection.index(POP / 4);
                let second = selection.index(POP / 4);
                // Model a systematic topology preference for the locally stronger parent.
                let a = &population[ranked[first.min(second)]];
                let b = &population[ranked[first.max(second)]];
                let flip = selection.index(2) == 0;
                let mut child = if !elites.is_empty() && slot % 5 == 0 {
                    // Mutate a protected strategy directly to preserve coadapted genes.
                    let mut child = elites[selection.index(elites.len())].clone();
                    child.generation += 1;
                    child
                } else if mode == Mode::Symmetric && flip {
                    Genome::crossover(b, a, &mut genetics)
                } else {
                    Genome::crossover(a, b, &mut genetics)
                };
                child.mutate_weights(&mut genetics, 0.15);
                if genetics.index(20) == 0 {
                    child.add_node(&mut genetics, &mut innovations);
                }
                child
            })
            .collect();
    }
    metrics
}

fn main() {
    const SEEDS: [u64; 8] = [11, 23, 35, 47, 59, 71, 83, 95];
    println!("mode,seed,pre_shift,early_shift,late_shift,held_out,niches,disagreement");
    for (label, mode) in [
        ("baseline", Mode::Baseline),
        ("symmetric", Mode::Symmetric),
        ("memory", Mode::Memory),
        ("archive", Mode::Archive),
    ] {
        let mut sum = [0.0; 6];
        for seed in SEEDS {
            let m = trial(seed, mode);
            let values = [
                m.local_before,
                m.local_early,
                m.local_late,
                m.transfer,
                m.niches,
                m.disagreement,
            ];
            for (total, value) in sum.iter_mut().zip(values) {
                *total += value;
            }
            print!("{label},{seed}");
            for value in values {
                print!(",{value:.4}");
            }
            println!();
        }
        print!("{label},mean");
        for value in sum {
            print!(",{:.4}", value / SEEDS.len() as f64);
        }
        println!();
    }
}
