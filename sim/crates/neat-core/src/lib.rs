//! Feed-forward NEAT genes, historical alignment and allocation-free inference.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const INPUTS: usize = 42;
pub const MEMORY_INPUTS: usize = 4;
const MEMORY_NODE_IDS: [u32; MEMORY_INPUTS] = [u32::MAX - 3, u32::MAX - 2, u32::MAX - 1, u32::MAX];
pub const MAX_NODES: usize = 100;
pub const SENSORS: [&str; INPUTS] = [
    "need",
    "benefit",
    "risk",
    "travel",
    "price",
    "continuity",
    "food",
    "timber",
    "stone",
    "fish",
    "energy",
    "infrastructure",
    "gold_ore",
    "silver_ore",
    "copper_ore",
    "wallet",
    "currency_need",
    "load_fraction",
    "free_capacity",
    "affordability",
    "barter",
    "payment_mode",
    "night",
    "rested_meter",
    "rested_buff",
    "bed_availability",
    "night_rest",
    "sleep_access",
    "aspiration_wealth",
    "aspiration_blacksmith",
    "aspiration_family",
    "aspiration_farmer",
    "aspiration_public_service",
    "goal_alignment",
    "goal_urgency",
    "happiness",
    "coin_preference",
    "night_owl",
    "tool_benefit",
    "adult",
    "iron",
    "rice",
];

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Rng(pub u64);
impl Rng {
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
    pub fn unit(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / 16777216.0
    }
    pub fn index(&mut self, n: usize) -> usize {
        (self.next_u64() % n.max(1) as u64) as usize
    }
    pub fn signed(&mut self) -> f32 {
        self.unit() * 2.0 - 1.0
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Kind {
    Input,
    Tanh,
    Identity,
    Output,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Node {
    pub id: u32,
    pub kind: Kind,
    pub bias: f32,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Gene {
    pub innovation: u64,
    pub from: u32,
    pub to: u32,
    pub weight: f32,
    pub enabled: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Genome {
    pub generation: u32,
    pub nodes: Vec<Node>,
    pub genes: Vec<Gene>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Innovations {
    pub next_node: u32,
    pub next_gene: u64,
    pub edges: BTreeMap<(u32, u32), u64>,
    pub splits: BTreeMap<u64, u32>,
}
impl Default for Innovations {
    fn default() -> Self {
        Self {
            next_node: (INPUTS + 5) as u32,
            next_gene: 1,
            edges: BTreeMap::new(),
            splits: BTreeMap::new(),
        }
    }
}
impl Innovations {
    pub fn edge(&mut self, from: u32, to: u32) -> u64 {
        if let Some(id) = self.edges.get(&(from, to)) {
            *id
        } else {
            let id = self.next_gene;
            self.next_gene += 1;
            self.edges.insert((from, to), id);
            id
        }
    }
}
#[derive(Clone, Debug)]
pub struct CompiledNode {
    memory_input: Option<usize>,
    pub kind: Kind,
    pub bias: f32,
    pub start: usize,
    pub len: usize,
}
#[derive(Clone, Debug)]
pub struct Compiled {
    pub nodes: Vec<CompiledNode>,
    pub edges: Vec<(usize, f32)>,
    pub output: usize,
}
impl Compiled {
    pub fn evaluate(&self, inputs: &[f32; INPUTS]) -> f32 {
        self.evaluate_with_memory(inputs, &[0.0; MEMORY_INPUTS])
    }

    pub fn evaluate_with_memory(
        &self,
        inputs: &[f32; INPUTS],
        memory: &[f32; MEMORY_INPUTS],
    ) -> f32 {
        let mut values = [0.0f32; MAX_NODES];
        let mut input = 0;
        for (i, node) in self.nodes.iter().enumerate() {
            if node.kind == Kind::Input {
                if let Some(slot) = node.memory_input {
                    values[i] = memory[slot];
                } else {
                    values[i] = inputs[input];
                    input += 1;
                }
                continue;
            }
            let mut v = node.bias;
            for &(from, weight) in &self.edges[node.start..node.start + node.len] {
                v += values[from] * weight;
            }
            values[i] = if node.kind == Kind::Tanh { v.tanh() } else { v };
        }
        values[self.output]
    }
}
impl Genome {
    pub fn seeded(rng: &mut Rng, innovations: &mut Innovations) -> Self {
        let mut nodes: Vec<Node> = (0..INPUTS)
            .map(|id| Node {
                id: id as u32,
                kind: Kind::Input,
                bias: 0.0,
            })
            .collect();
        for i in 0..4 {
            nodes.push(Node {
                id: (INPUTS + i) as u32,
                kind: Kind::Tanh,
                bias: -0.3 + rng.signed() * 0.12,
            });
        }
        let output = (INPUTS + 4) as u32;
        nodes.push(Node {
            id: output,
            kind: Kind::Output,
            bias: rng.signed() * 0.08,
        });
        let mut genes = Vec::new();
        for h in 0..4 {
            for sensor in 0..INPUTS {
                let base = match sensor {
                    0 => 1.7,
                    1 => 1.1,
                    2 => -1.7,
                    3 => -0.9,
                    4 => 0.5,
                    5 => 0.4,
                    10 => 0.12,
                    11 => 0.3,
                    15 => 0.1,
                    16 => 0.35,
                    17 => -0.15,
                    18 => 0.1,
                    19 => 0.3,
                    20 => 0.1,
                    21 => 0.15,
                    22 => -0.05,
                    23 => 0.05,
                    24 => 0.05,
                    25 => 0.1,
                    26 => 1.4,
                    27 => 0.25,
                    28..=32 => 0.7,
                    33 => 0.65,
                    34 => 0.25,
                    35 => -0.05,
                    36 => 0.6,
                    37 => 0.0,
                    38 => 0.25,
                    39 => 0.05,
                    40 => 0.1,
                    41 => 0.15,
                    _ => 0.0,
                };
                let preference = if sensor == 6 + h {
                    rng.signed() * 0.7
                } else {
                    0.0
                };
                let to = (INPUTS + h) as u32;
                genes.push(Gene {
                    innovation: innovations.edge(sensor as u32, to),
                    from: sensor as u32,
                    to,
                    weight: base + preference + rng.signed() * 0.17,
                    enabled: true,
                });
            }
            let from = (INPUTS + h) as u32;
            genes.push(Gene {
                innovation: innovations.edge(from, output),
                from,
                to: output,
                weight: 0.6 + rng.unit() * 0.6,
                enabled: true,
            });
        }
        Self {
            generation: 0,
            nodes,
            genes,
        }
    }
    /// Add optional outcome-memory sensors without renumbering the existing genome.
    pub fn add_memory_inputs(&mut self, registry: &mut Innovations) {
        let Some(output) = self
            .nodes
            .iter()
            .find(|node| node.kind == Kind::Output)
            .map(|node| node.id)
        else {
            return;
        };
        for id in MEMORY_NODE_IDS {
            if self.nodes.iter().any(|node| node.id == id) {
                continue;
            }
            self.nodes.push(Node {
                id,
                kind: Kind::Input,
                bias: 0.0,
            });
            self.genes.push(Gene {
                innovation: registry.edge(id, output),
                from: id,
                to: output,
                weight: 0.0,
                enabled: true,
            });
        }
    }

    pub fn compile(&self) -> Result<Compiled, String> {
        if self.nodes.len() > MAX_NODES {
            return Err("Brain exceeds node budget".into());
        }
        if self
            .nodes
            .iter()
            .filter(|n| n.kind == Kind::Input && !MEMORY_NODE_IDS.contains(&n.id))
            .count()
            != INPUTS
        {
            return Err("Sensor schema mismatch".into());
        }
        let mut pending = self.nodes.clone();
        pending.sort_by_key(|n| n.id);
        if pending.windows(2).any(|n| n[0].id == n[1].id) {
            return Err("Duplicate node IDs".into());
        }
        for gene in self.genes.iter().filter(|g| g.enabled) {
            if !gene.weight.is_finite()
                || !self.nodes.iter().any(|n| n.id == gene.from)
                || !self
                    .nodes
                    .iter()
                    .any(|n| n.id == gene.to && n.kind != Kind::Input)
            {
                return Err("Invalid edge".into());
            }
        }
        let mut order: Vec<Node> = Vec::new();
        while !pending.is_empty() {
            let pos = pending.iter().position(|n| {
                self.genes
                    .iter()
                    .filter(|g| g.enabled && g.to == n.id)
                    .all(|g| order.iter().any(|p| p.id == g.from))
            });
            let Some(pos) = pos else {
                return Err("Cycle in feed-forward genome".into());
            };
            order.push(pending.remove(pos));
        }
        let mut edges = Vec::new();
        let mut nodes = Vec::new();
        let mut output = None;
        for (i, n) in order.iter().enumerate() {
            if !n.bias.is_finite() {
                return Err("Non-finite bias".into());
            }
            let start = edges.len();
            let mut incoming: Vec<_> = self
                .genes
                .iter()
                .filter(|g| g.enabled && g.to == n.id)
                .collect();
            incoming.sort_by_key(|g| g.innovation);
            for g in incoming {
                edges.push((order.iter().position(|p| p.id == g.from).unwrap(), g.weight));
            }
            nodes.push(CompiledNode {
                memory_input: MEMORY_NODE_IDS.iter().position(|&id| id == n.id),
                kind: n.kind,
                bias: n.bias,
                start,
                len: edges.len() - start,
            });
            if n.kind == Kind::Output {
                if output.is_some() {
                    return Err("Multiple score outputs".into());
                }
                output = Some(i);
            }
        }
        Ok(Compiled {
            nodes,
            edges,
            output: output.ok_or("Missing output")?,
        })
    }
    pub fn mutate_weights(&mut self, rng: &mut Rng, rate: f32) {
        for g in &mut self.genes {
            if rng.unit() < rate {
                g.weight = (g.weight + rng.signed() * 0.3).clamp(-3.0, 3.0);
            }
        }
        for n in &mut self.nodes {
            if n.kind != Kind::Input && rng.unit() < rate {
                n.bias = (n.bias + rng.signed() * 0.2).clamp(-3.0, 3.0);
            }
        }
    }
    pub fn add_node(&mut self, rng: &mut Rng, registry: &mut Innovations) -> bool {
        let budget = 96
            + self
                .nodes
                .iter()
                .filter(|node| MEMORY_NODE_IDS.contains(&node.id))
                .count();
        if self.nodes.len() >= budget || registry.next_node >= MEMORY_NODE_IDS[0] {
            return false;
        }
        let enabled: Vec<usize> = self
            .genes
            .iter()
            .enumerate()
            .filter(|(_, g)| g.enabled)
            .map(|(i, _)| i)
            .collect();
        if enabled.is_empty() {
            return false;
        }
        let index = enabled[rng.index(enabled.len())];
        let old = self.genes[index].clone();
        let id = if let Some(id) = registry.splits.get(&old.innovation) {
            *id
        } else {
            let id = registry.next_node;
            registry.next_node += 1;
            registry.splits.insert(old.innovation, id);
            id
        };
        if self.nodes.iter().any(|n| n.id == id) {
            return false;
        }
        self.genes[index].enabled = false;
        self.nodes.push(Node {
            id,
            kind: Kind::Identity,
            bias: 0.0,
        });
        self.genes.push(Gene {
            innovation: registry.edge(old.from, id),
            from: old.from,
            to: id,
            weight: 1.0,
            enabled: true,
        });
        self.genes.push(Gene {
            innovation: registry.edge(id, old.to),
            from: id,
            to: old.to,
            weight: old.weight,
            enabled: true,
        });
        true
    }
    pub fn add_connection(&mut self, rng: &mut Rng, registry: &mut Innovations) -> bool {
        for _ in 0..16 {
            let a = &self.nodes[rng.index(self.nodes.len())];
            let b = &self.nodes[rng.index(self.nodes.len())];
            if a.id == b.id
                || a.kind == Kind::Output
                || b.kind == Kind::Input
                || self.genes.iter().any(|g| g.from == a.id && g.to == b.id)
            {
                continue;
            }
            let gene = Gene {
                innovation: registry.edge(a.id, b.id),
                from: a.id,
                to: b.id,
                weight: rng.signed(),
                enabled: true,
            };
            self.genes.push(gene);
            if self.compile().is_ok() {
                return true;
            }
            self.genes.pop();
        }
        false
    }
    /// The fitter parent's disjoint/excess genes survive. Matching genes align by innovation.
    pub fn crossover(fitter: &Self, other: &Self, rng: &mut Rng) -> Self {
        let mut child = fitter.clone();
        child.generation = fitter.generation.max(other.generation) + 1;
        let matching: BTreeMap<u64, &Gene> =
            other.genes.iter().map(|g| (g.innovation, g)).collect();
        for g in &mut child.genes {
            if let Some(b) = matching.get(&g.innovation) {
                if rng.unit() < 0.5 {
                    g.weight = b.weight;
                }
                if !g.enabled || !b.enabled {
                    g.enabled = rng.unit() >= 0.75;
                }
            }
        }
        // Re-enabling genes can create a cycle even when both parents compile; retain a valid fallback.
        if child.compile().is_err() {
            child = fitter.clone();
            child.generation = fitter.generation.max(other.generation) + 1;
        }
        child
    }
    pub fn compatibility(&self, other: &Self) -> f32 {
        let b: BTreeMap<u64, &Gene> = other.genes.iter().map(|g| (g.innovation, g)).collect();
        let mut matching = 0;
        let mut delta = 0.0;
        for g in &self.genes {
            if let Some(o) = b.get(&g.innovation) {
                matching += 1;
                delta += (g.weight - o.weight).abs();
            }
        }
        let unmatched = self.genes.len() + other.genes.len() - 2 * matching;
        unmatched as f32 / self.genes.len().max(other.genes.len()).max(1) as f32
            + 0.4 * delta / matching.max(1) as f32
    }
}
/// Representative-based compatibility grouping; no all-pairs population comparison.
pub fn species(genomes: &[Genome], threshold: f32) -> Vec<usize> {
    let mut reps: Vec<usize> = Vec::new();
    let mut groups = Vec::with_capacity(genomes.len());
    for (i, g) in genomes.iter().enumerate() {
        let group = reps
            .iter()
            .position(|r| g.compatibility(&genomes[*r]) < threshold)
            .unwrap_or_else(|| {
                reps.push(i);
                reps.len() - 1
            });
        groups.push(group);
    }
    groups
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inputs_and_seeds_are_reproducible() {
        let mut r = Innovations::default();
        let a = Genome::seeded(&mut Rng(42), &mut r);
        let b = Genome::seeded(&mut Rng(42), &mut r);
        assert_eq!(a, b);
        assert!(a.compile().unwrap().evaluate(&[0.2; INPUTS]).is_finite());
    }
    #[test]
    fn topology_split_is_neutral_and_historical() {
        let mut reg = Innovations::default();
        let a = Genome::seeded(&mut Rng(3), &mut reg);
        let mut b = a.clone();
        let mut c = a.clone();
        let before = a.compile().unwrap().evaluate(&[0.3; INPUTS]);
        assert!(b.add_node(&mut Rng(8), &mut reg));
        assert!(c.add_node(&mut Rng(8), &mut reg));
        assert_eq!(b, c);
        assert!((before - b.compile().unwrap().evaluate(&[0.3; INPUTS])).abs() < 1e-5);
    }
    #[test]
    fn cycles_are_rejected() {
        let mut reg = Innovations::default();
        let mut g = Genome::seeded(&mut Rng(4), &mut reg);
        g.genes.push(Gene {
            innovation: 999,
            from: 16,
            to: 12,
            weight: 1.0,
            enabled: true,
        });
        assert!(g.compile().is_err());
    }
    #[test]
    fn crossing_aligns_history_not_array_positions() {
        let mut reg = Innovations::default();
        let a = Genome::seeded(&mut Rng(1), &mut reg);
        let mut b = a.clone();
        b.genes.reverse();
        let c = Genome::crossover(&a, &b, &mut Rng(5));
        assert_eq!(
            a.compile().unwrap().evaluate(&[0.4; INPUTS]),
            c.compile().unwrap().evaluate(&[0.4; INPUTS])
        );
    }
    #[test]
    fn mutation_and_crossing_stay_valid() {
        let mut reg = Innovations::default();
        let mut g = Genome::seeded(&mut Rng(8), &mut reg);
        let mut rng = Rng(2);
        for _ in 0..100 {
            g.mutate_weights(&mut rng, 0.2);
            g.add_node(&mut rng, &mut reg);
            g.add_connection(&mut rng, &mut reg);
            g = Genome::crossover(&g, &g, &mut rng);
            assert!(g.compile().unwrap().evaluate(&[0.1; INPUTS]).is_finite());
            assert!(g.genes.iter().all(|g| g.weight.abs() <= 3.0));
        }
    }
}
