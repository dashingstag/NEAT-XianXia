//! Personal outcome memory for resolved food-acquisition attempts.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const TARGET_LIMIT: usize = 8;
const LESSON_LIMIT: usize = 4;
// Three recent failures exceed this evidence floor. Two fresh failures
// reach only 0.5775, and older observations count for less.
const FAILURE_EVIDENCE: f32 = 0.60;
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
        let old = self.confidence(tick, 0) * 0.65;
        self.evidence[0] = old + 0.35;
        self.mean[0] = (old * self.mean[0] + 0.35 * outcome) / self.evidence[0];

        self.evidence[1] = self.confidence(tick, 1);
        if outcome > 0.0 {
            self.evidence[1] = 0.0;
        } else if -self.mean[0] * self.evidence[0] >= FAILURE_EVIDENCE {
            self.mean[1] = -1.0;
            self.evidence[1] = self.evidence[1].max(self.evidence[0]);
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
        let mut lessons: Vec<_> = memory.targets.iter()
            .filter(|(_, estimate)| estimate.confidence(tick, 1) > 0.001)
            .map(|(&id, estimate)| (estimate.tick, id))
            .collect();
        if lessons.len() > LESSON_LIMIT {
            lessons.sort_unstable();
            for (_, id) in &lessons[..lessons.len() - LESSON_LIMIT] {
                memory.targets.get_mut(id).unwrap().evidence[1] = 0.0;
            }
        }
    }

    pub(super) fn inputs(&self, person: usize, target: u32, tick: u64) -> [f32; 4] {
        let Some(memory) = self.people.get(&person) else {
            return [0.0; 4];
        };
        let specific = memory.targets.get(&target).copied().unwrap_or_default();
        let mut values = [0.0; 4];
        for (horizon, pair) in values.chunks_exact_mut(2).enumerate() {
            let local = specific.confidence(tick, horizon);
            let general = if horizon == 0 {
                (1.0 - local) * memory.general.confidence(tick, 0) * 0.35
            } else {
                0.0
            };
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
    fn v15_world_memory_and_map_layouts_load() {
        let mut world = World::new(8, 315).unwrap();
        world.food_memory.observe(0, 0, world.tick, true);
        let path = std::env::temp_dir().join(format!(
            "xianxia-v15-layout-{}.snapshot",
            std::process::id()
        ));
        for has_memory in [false, true] {
            let payload = if has_memory {
                postcard::to_allocvec(&(&world, &world.food_memory)).unwrap()
            } else {
                postcard::to_allocvec(&world).unwrap()
            };
            let mut bytes = b"XXSAVE15".to_vec();
            bytes.extend((payload.len() as u64).to_le_bytes());
            bytes.extend(zstd::stream::encode_all(payload.as_slice(), 1).unwrap());
            std::fs::write(&path, bytes).unwrap();
            let restored = World::load(&path).unwrap();
            assert_eq!(
                restored.food_memory.inputs(0, 0, world.tick)[0],
                if has_memory { 1.0 } else { 0.0 }
            );
        }
        let generated =
            World::new_with_size(8, 315, crate::Hill::default(), crate::MapSize::X4).unwrap();
        for has_exploration in [false, true] {
            let payload = if has_exploration {
                postcard::to_allocvec(&(&generated, &generated.map, &generated.explored)).unwrap()
            } else {
                postcard::to_allocvec(&(&generated, &generated.map)).unwrap()
            };
            let mut bytes = b"XXSAVE15".to_vec();
            bytes.extend((payload.len() as u64).to_le_bytes());
            bytes.extend(zstd::stream::encode_all(payload.as_slice(), 1).unwrap());
            std::fs::write(&path, bytes).unwrap();
            let restored = World::load(&path).unwrap();
            assert_eq!(restored.map.spec, generated.map.spec);
            assert_eq!(restored.explored.chunks.is_empty(), !has_exploration);
            restored.save(&path).unwrap();
            let replay = World::load(&path).unwrap();
            assert_eq!(replay.map.spec, generated.map.spec);
            assert_eq!(replay.explored.chunks, restored.explored.chunks);
        }
        std::fs::remove_file(path).unwrap();
    }

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
    fn repeated_failures_promote_and_success_clears_the_lesson() {
        let mut memory = FoodMemory::default();
        for tick in [10, 20, 30] {
            memory.commit(0, 4, 0);
            memory.resolve(0, 4, 0, tick, false);
            assert_eq!(memory.inputs(0, 4, tick)[2], if tick == 30 { -1.0 } else { 0.0 });
        }
        assert_eq!(memory.inputs(0, 5, 30)[2..], [0.0; 2]);
        memory.commit(0, 4, 0);
        memory.resolve(0, 4, 0, 40, true);
        assert_eq!(memory.inputs(0, 4, 40)[2..], [0.0; 2]);
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
