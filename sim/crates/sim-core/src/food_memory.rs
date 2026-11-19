//! Personal outcome memory for resolved food-acquisition attempts.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const TARGET_LIMIT: usize = 8;
const SHORT_HALF_LIFE: f32 = 6_000.0;
const LONG_HALF_LIFE: f32 = 63_000.0;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub(super) struct FoodMemory {
    people: BTreeMap<usize, PersonMemory>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
struct PersonMemory {
    pending: Option<(u32, usize)>,
    general: Estimate,
    targets: BTreeMap<u32, Estimate>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
struct Estimate {
    tick: u64,
    mean: [f32; 2],
    evidence: [f32; 2],
}

impl Estimate {
    fn confidence(self, tick: u64, horizon: usize) -> f32 {
        let age = tick.saturating_sub(self.tick) as f32;
        let half_life = [SHORT_HALF_LIFE, LONG_HALF_LIFE][horizon];
        self.evidence[horizon] * (-age / half_life).exp2()
    }

    fn observe(&mut self, tick: u64, outcome: f32) {
        for (horizon, gain) in [0.35, 0.07].into_iter().enumerate() {
            let old = self.confidence(tick, horizon) * (1.0 - gain);
            self.evidence[horizon] = old + gain;
            self.mean[horizon] = (old * self.mean[horizon] + gain * outcome) / (old + gain);
        }
        self.tick = tick;
    }
}

impl FoodMemory {
    pub(super) fn commit(&mut self, person: usize, target: u32, kind: usize) {
        self.people.entry(person).or_default().pending = Some((target, kind));
    }

    pub(super) fn forget(&mut self, person: usize) {
        self.people.remove(&person);
    }

    pub(super) fn cancel(&mut self, person: usize) {
        if let Some(memory) = self.people.get_mut(&person) {
            memory.pending = None;
        }
    }

    pub(super) fn resolve(
        &mut self,
        person: usize,
        target: u32,
        kind: usize,
        tick: u64,
        success: bool,
    ) {
        self.resolve_at(person, (target, kind), target, tick, success);
    }

    pub(super) fn resolve_at(
        &mut self,
        person: usize,
        attempt: (u32, usize),
        target: u32,
        tick: u64,
        success: bool,
    ) {
        if self.take_attempt(person, attempt) {
            self.observe(person, target, tick, success);
        }
    }

    pub(super) fn take_attempt(&mut self, person: usize, attempt: (u32, usize)) -> bool {
        let Some(memory) = self.people.get_mut(&person) else {
            return false;
        };
        if memory.pending != Some(attempt) {
            return false;
        }
        memory.pending = None;
        true
    }

    pub(super) fn observe(&mut self, person: usize, target: u32, tick: u64, success: bool) {
        let memory = self.people.entry(person).or_default();
        let outcome = if success { 1.0 } else { -1.0 };
        memory.general.observe(tick, outcome);
        if !memory.targets.contains_key(&target) && memory.targets.len() == TARGET_LIMIT {
            let oldest = *memory
                .targets
                .iter()
                .min_by_key(|(_, value)| value.tick)
                .unwrap()
                .0;
            memory.targets.remove(&oldest);
        }
        memory
            .targets
            .entry(target)
            .or_default()
            .observe(tick, outcome);
    }

    pub(super) fn inputs(&self, person: usize, target: u32, tick: u64) -> [f32; 4] {
        let Some(memory) = self.people.get(&person) else {
            return [0.0; 4];
        };
        let specific = memory.targets.get(&target).copied().unwrap_or_default();
        let mut values = [0.0; 4];
        for (horizon, pair) in values.chunks_exact_mut(2).enumerate() {
            let local = specific.confidence(tick, horizon);
            let general = (1.0 - local) * memory.general.confidence(tick, horizon) * 0.35;
            let confidence = local + general;
            if confidence > 0.001 {
                pair[0] = (local * specific.mean[horizon] + general * memory.general.mean[horizon])
                    / confidence;
                pair[1] = confidence;
            }
        }
        values
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Action, Lot, World};

    #[test]
    fn purchase_memory_replays_and_changes_the_next_offer_once() {
        let mut world = World::new(8, 310).unwrap();
        let person = 0;
        let market = world.agents.home[person];
        let m = &mut world.markets[market as usize];
        m.goods[0].push_back(Lot {
            quantity: 5,
            origin: market,
        });
        m.quotes[0].offered = 5;
        m.quotes[0].ask = 1;
        world.agents.berries[person] = 0;
        world.agents.x[person] = m.x + 22.0;
        world.agents.y[person] = m.y;
        world.agents.action[person] = Action::BuyFood;
        world.agents.target[person] = market;
        world.agents.cargo_kind[person] = 0;
        world.tick = 50;
        world.food_memory.commit(person, market, 0);
        // Simulate a descendant that evolved a nonzero memory weight.
        let genome = &mut world.genomes[person];
        let edge = genome.genes.len() - 4;
        genome.genes[edge].weight = 0.25;
        world.brains[person] = genome.compile().unwrap();

        world.execute(person);
        assert_eq!(
            world.food_memory.inputs(person, market, world.tick),
            [0.0; 4]
        );
        let path = std::env::temp_dir().join(format!(
            "xianxia-food-memory-{}.snapshot",
            std::process::id()
        ));
        world.ledger.initial = std::array::from_fn(|kind| {
            world.stocks()[kind]
                + world.ledger.consumed[kind]
                + world.ledger.built[kind]
                + world.ledger.minted_ore[kind]
                + world.ledger.crafted_in[kind]
                - world.ledger.renewed[kind]
                - world.ledger.crafted_out[kind]
        });
        world.save(&path).unwrap();
        let mut replay = World::load(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        assert_eq!(world.food_memory, replay.food_memory);

        for branch in [&mut world, &mut replay] {
            for _ in 0..100 {
                branch.execute(person);
                if branch.agents.berries[person] > 0 {
                    break;
                }
            }
            assert_eq!(branch.agents.berries[person], 1);
            let learned = branch.food_memory.clone();
            let memory = learned.inputs(person, market, branch.tick);
            assert_eq!(memory[0], 1.0);
            assert!(memory[1] > memory[3]);
            let trace = branch.trace(person).unwrap();
            let offer = trace
                .choices
                .iter()
                .find(|offer| offer.action == "BuyFood")
                .unwrap();
            assert_eq!(offer.memory_inputs, Some(memory));
            let neutral_score =
                branch.brains[person].evaluate(offer.inputs.as_slice().try_into().unwrap());
            assert!(offer.score > neutral_score);
            branch.execute(person);
            assert_eq!(branch.food_memory, learned);
        }
        assert_eq!(world.food_memory, replay.food_memory);
    }

    #[test]
    fn cancelled_attempt_gives_no_evidence_and_failed_attempt_gives_one_update() {
        let mut memory = FoodMemory::default();
        memory.commit(0, 4, 0);
        memory.cancel(0);
        memory.resolve(0, 4, 0, 1, false);
        assert_eq!(memory.inputs(0, 4, 1), [0.0; 4]);
        memory.commit(0, 4, 0);
        memory.resolve(0, 4, 0, 2, false);
        let once = memory.clone();
        memory.resolve(0, 4, 0, 2, false);
        assert_eq!(memory, once);
        assert_eq!(memory.inputs(0, 4, 2)[0], -1.0);
        assert!(memory.inputs(0, 4, 6_002)[1] < memory.inputs(0, 4, 2)[1]);
        assert!(memory.inputs(0, 5, 2)[1] < memory.inputs(0, 4, 2)[1]);
    }
}
