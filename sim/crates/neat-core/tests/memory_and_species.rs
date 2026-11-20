//! Integration coverage for the outcome-memory pathway, speciation distance
//! and genome semantics that the crate's unit tests do not exercise.
use neat_core::{species, Genome, Innovations, Kind, Rng, INPUTS, MEMORY_INPUTS};

fn seeded() -> (Genome, Innovations) {
    let mut reg = Innovations::default();
    (Genome::seeded(&mut Rng(11), &mut reg), reg)
}

#[test]
fn memory_inputs_drive_the_score() {
    let (mut g, mut reg) = seeded();
    let inputs: Vec<u32> = g
        .nodes
        .iter()
        .filter(|n| n.kind == Kind::Input)
        .map(|n| n.id)
        .collect();
    g.add_memory_inputs(&mut reg);
    let memory_ids: Vec<u32> = g
        .nodes
        .iter()
        .filter(|n| n.kind == Kind::Input && !inputs.contains(&n.id))
        .map(|n| n.id)
        .collect();
    assert_eq!(memory_ids.len(), MEMORY_INPUTS);
    for gene in g.genes.iter_mut().filter(|g| memory_ids.contains(&g.from)) {
        gene.weight = 0.9;
    }
    // Wiring the same ids twice must not grow the genome.
    let nodes = g.nodes.len();
    g.add_memory_inputs(&mut reg);
    assert_eq!(g.nodes.len(), nodes);

    let brain = g.compile().unwrap();
    let sensors = [0.25; INPUTS];
    let mut memory = [0.0; MEMORY_INPUTS];
    memory[1] = 0.8;
    let baseline = brain.evaluate(&sensors);
    assert_eq!(baseline, brain.evaluate_with_memory(&sensors, &[0.0; MEMORY_INPUTS]));
    let recalled = brain.evaluate_with_memory(&sensors, &memory);
    assert!((recalled - baseline - 0.72).abs() < 1e-5);
}

#[test]
fn disabled_gene_matches_zero_weight() {
    let (g, _reg) = seeded();
    let target = g
        .genes
        .iter()
        .position(|gene| gene.enabled && gene.weight != 0.0)
        .expect("seeded genome has a live weighted gene");
    let mut disabled = g.clone();
    disabled.genes[target].enabled = false;
    let mut zeroed = g.clone();
    zeroed.genes[target].weight = 0.0;
    let sensors = [0.35; INPUTS];
    assert_eq!(
        disabled.compile().unwrap().evaluate(&sensors),
        zeroed.compile().unwrap().evaluate(&sensors)
    );
}

#[test]
fn speciation_respects_threshold_boundaries() {
    let (a, _reg) = seeded();
    let mut reg = Innovations::default();
    let b = Genome::seeded(&mut Rng(12), &mut reg);
    // Compatibility is never negative, so a zero threshold isolates every genome.
    assert_eq!(species(&[a.clone(), b], 0.0), vec![0, 1]);
    // An identical genome is always within a positive threshold.
    assert_eq!(species(&[a.clone(), a.clone()], 1.0), vec![0, 0]);
    assert_eq!(a.compatibility(&a), 0.0);
}

#[test]
fn rng_streams_are_bounded_and_deterministic() {
    let mut a = Rng(7);
    let mut b = Rng(7);
    for _ in 0..128 {
        assert_eq!(a.next_u64(), b.next_u64());
    }
    let mut r = Rng(13);
    for _ in 0..512 {
        let unit = r.unit();
        assert!((0.0..1.0).contains(&unit));
        assert!((-1.0..1.0).contains(&r.signed()));
        assert!(r.index(17) < 17);
    }
}
