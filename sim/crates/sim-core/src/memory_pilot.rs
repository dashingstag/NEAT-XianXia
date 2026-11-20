use super::{Action, World};
use std::collections::BTreeMap;

const ACTIONS: usize = Action::NominateSelf as usize + 1;
type Key = (usize, u8, u32, usize);

#[derive(Clone, Debug, Default)]
pub(super) struct Trial {
    mode: usize,
    pub(super) coverage: BTreeMap<u8, Coverage>,
    memory: BTreeMap<Key, Estimate>,
    pending: BTreeMap<usize, Key>,
}

#[derive(Clone, Debug, Default, serde::Serialize)]
pub(super) struct Coverage {
    offered: u64,
    chosen: u64,
    observed_progress: u64,
    updates: u64,
}

#[derive(Clone, Debug, Default)]
struct Estimate {
    short: [f32; 3],
    long: [f32; 3],
    tick: u64,
}

impl Trial {
    pub(super) fn adjust(&self, key: Key, tick: u64, inputs: &[f32; super::INPUTS]) -> f32 {
        if self.mode == 0 {
            return 0.0;
        }
        let Some(e) = self.memory.get(&key) else {
            return 0.0;
        };
        let age = tick.saturating_sub(e.tick) as f32;
        let blend = |k: usize| {
            0.7 * e.short[k] * (-age / 300.0).exp() + 0.3 * e.long[k] * (-age / 3000.0).exp()
        };
        let value = if self.mode == 1 {
            blend(0)
        } else {
            let reliability = 0.25 + 0.30 * inputs[0].clamp(0.0, 1.0);
            let cost = 0.10 + 0.20 * inputs[2].clamp(0.0, 1.0);
            reliability * blend(0) + (1.0 - reliability - cost) * blend(1) + cost * blend(2)
        };
        0.30 * value.clamp(-1.0, 1.0)
    }

    pub(super) fn commit(
        &mut self,
        i: usize,
        candidate: super::Candidate,
        offers: [u16; ACTIONS],
        admitted: bool,
    ) {
        for (action, count) in offers.into_iter().enumerate() {
            if count > 0 {
                self.coverage.entry(action as u8).or_default().offered += u64::from(count);
            }
        }
        if admitted {
            let key = (i, candidate.action as u8, candidate.target, candidate.kind);
            self.coverage.entry(key.1).or_default().chosen += 1;
            self.pending.insert(i, key);
        }
    }

    fn update(&mut self, key: Key, tick: u64, observation: [f32; 3]) {
        let coverage = self.coverage.entry(key.1).or_default();
        coverage.observed_progress += 1;
        if self.mode == 0 {
            return;
        }
        coverage.updates += 1;
        let estimate = self.memory.entry(key).or_default();
        let values = if self.mode == 1 {
            [
                0.35 * observation[0] + 0.50 * observation[1] + 0.15 * observation[2],
                0.0,
                0.0,
            ]
        } else {
            observation
        };
        let age = tick.saturating_sub(estimate.tick) as f32;
        for (k, value) in values.into_iter().enumerate() {
            estimate.short[k] = 0.65 * estimate.short[k] * (-age / 300.0).exp() + 0.35 * value;
            estimate.long[k] = 0.93 * estimate.long[k] * (-age / 3000.0).exp() + 0.07 * value;
        }
        estimate.tick = tick;
    }
}

#[derive(PartialEq)]
struct Observable {
    goods: [u32; 4],
    coins: u64,
    tools: [u8; 4],
    residence: u32,
    energy: f32,
    hunger: f32,
    happiness: f32,
    action: Action,
    jobs: u64,
    projects: u64,
}

impl World {
    /// Run a transient live-world pilot. Uses score adjustments and observable progress proxies.
    pub fn enable_memory_pilot(&mut self, readouts: usize) {
        assert!(matches!(readouts, 0 | 1 | 3));
        self.watch.memory_trial = Some(Trial {
            mode: readouts,
            ..Trial::default()
        });
    }

    pub fn memory_pilot_coverage(&self) -> serde_json::Value {
        serde_json::to_value(
            &self
                .watch
                .memory_trial
                .as_ref()
                .expect("enabled memory pilot")
                .coverage,
        )
        .unwrap()
    }

    fn observe_memory_progress(&self, i: usize) -> Observable {
        let a = &self.agents;
        Observable {
            goods: [a.berries[i], a.fish[i], a.rice[i], a.cargo[i]],
            coins: a.coins[i],
            tools: a.tools[i],
            residence: a.residence[i],
            energy: a.energy[i],
            hunger: a.hunger[i],
            happiness: self.people[i].happiness,
            action: a.action[i],
            jobs: self.stats.jobs_completed,
            projects: self.stats.buildings + self.stats.graveyards_built + self.stats.roads,
        }
    }

    pub(super) fn execute_with_memory_pilot(&mut self, i: usize) {
        let key = self
            .watch
            .memory_trial
            .as_ref()
            .and_then(|trial| trial.pending.get(&i).copied())
            .filter(|&(_, action, target, _)| {
                self.agents.action[i] as u8 == action && self.agents.target[i] == target
            });
        let before = key.map(|_| self.observe_memory_progress(i));
        self.execute(i);
        let (Some(key), Some(before)) = (key, before) else {
            return;
        };
        let after = self.observe_memory_progress(i);
        let gain = after.goods.iter().zip(before.goods).any(|(&a, b)| a > b)
            || after.coins > before.coins
            || after.tools != before.tools
            || after.residence != before.residence
            || after.energy > before.energy
            || after.hunger < before.hunger
            || after.happiness > before.happiness
            || after.jobs > before.jobs
            || after.projects > before.projects;
        if gain {
            let cost = if after.coins < before.coins || after.energy < before.energy {
                -0.5
            } else {
                0.0
            };
            let availability = if after.action != before.action {
                1.0
            } else {
                0.5
            };
            let trial = self.watch.memory_trial.as_mut().unwrap();
            trial.pending.remove(&i);
            trial.update(key, self.tick, [availability, 1.0, cost]);
        }
    }
}
