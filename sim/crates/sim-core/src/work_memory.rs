use super::{food_memory::FoodMemory, Action, World, INPUTS};

/// The serialized order is fixed. Append future entries with a checkpoint migration.
#[derive(Clone, Copy)]
pub(super) enum WorkKind {
    Gather,
    Farm,
    Harvest,
    Rest,
    Urban,
}

impl WorkKind {
    pub(super) const ACTIONS: [Action; 5] = [
        Action::Gather,
        Action::Farm,
        Action::HarvestRice,
        Action::Rest,
        Action::UrbanWork,
    ];
}

pub(super) type WorkMemory = [FoodMemory; WorkKind::ACTIONS.len()];

/// Urban job slots are recycled. Remember the market and job category instead.
pub(super) fn urban_memory_target(market: u32, kind: u8) -> u32 {
    (market << 8) | u32::from(kind)
}

impl World {
    pub(super) fn action_memory_inputs(
        &self,
        person: usize,
        action: Action,
        target: u32,
    ) -> Option<[f32; 4]> {
        let (memory, key) = match action {
            Action::BuyFood => (&self.food_memory, target),
            Action::ForageMeal => (&self.forage_memory, target),
            Action::UrbanWork => {
                let job = self.urban.jobs.get(target as usize)?.as_ref()?;
                (
                    &self.work_memory[WorkKind::Urban as usize],
                    urban_memory_target(job.market, job.kind),
                )
            }
            _ => {
                let Some(index) = WorkKind::ACTIONS
                    .iter()
                    .position(|&candidate| candidate == action)
                else {
                    return self.commitments.inputs(person, action, target, self.tick);
                };
                (&self.work_memory[index], target)
            }
        };
        Some(memory.inputs(person, key, self.tick))
    }

    pub(super) fn score_action(
        &self,
        person: usize,
        action: Action,
        target: u32,
        inputs: &[f32; INPUTS],
    ) -> f32 {
        match self.action_memory_inputs(person, action, target) {
            Some(memory) => self.brains[person].evaluate_with_memory(inputs, &memory),
            None => self.brains[person].evaluate(inputs),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gathering_resumes_and_records_one_outcome_per_attempt() {
        let mut world = World::new(8, 47).unwrap();
        let person = 0;
        let target = world
            .patches
            .iter()
            .position(|patch| patch.kind == 0)
            .unwrap();
        let key = target as u32;
        world.agents.action[person] = Action::Gather;
        world.agents.target[person] = key;
        world.agents.cargo_kind[person] = 0;
        world.agents.x[person] = world.patches[target].x + 15.0;
        world.agents.y[person] = world.patches[target].y;
        let index = WorkKind::Gather as usize;
        world.work_memory[index].commit(person, key, 0);
        let genome = &mut world.genomes[person];
        let edge = genome.genes.len() - 4;
        genome.genes[edge].weight = 0.25;
        world.brains[person] = genome.compile().unwrap();
        world.execute(person);
        assert_eq!(
            world.action_memory_inputs(person, Action::Gather, key),
            Some([0.0; 4])
        );

        let checkpoint = std::env::temp_dir().join(format!(
            "xianxia-gather-memory-{}.snapshot",
            std::process::id()
        ));
        world.save(&checkpoint).unwrap();
        let mut replay = World::load(&checkpoint).unwrap();
        std::fs::remove_file(checkpoint).unwrap();

        for branch in [&mut world, &mut replay] {
            for _ in 0..150 {
                branch.execute(person);
                if branch.agents.cargo[person] > 0 {
                    break;
                }
            }
            let learned = branch.work_memory[index].clone();
            let values = branch
                .action_memory_inputs(person, Action::Gather, key)
                .unwrap();
            assert_eq!(branch.agents.cargo[person], 1);
            assert_eq!(values[0], 1.0);
            let inputs = [0.0; INPUTS];
            assert!(
                branch.score_action(person, Action::Gather, key, &inputs)
                    > branch.brains[person].evaluate(&inputs)
            );
            assert_eq!(
                branch.action_memory_inputs(person, Action::Farm, key),
                Some([0.0; 4])
            );
            for _ in 0..25 {
                branch.execute(person);
            }
            assert_eq!(branch.work_memory[index], learned);
            branch.check().unwrap();
        }
        assert_eq!(world.work_memory, replay.work_memory);
    }
}
