use super::{food_memory::FoodMemory, Action};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub(super) struct Commitments {
    actions: BTreeMap<u8, FoodMemory>,
    trades: BTreeMap<usize, (u32, usize)>,
    deliveries: BTreeMap<usize, (Action, usize, u32)>,
}

impl Commitments {
    const ACTIONS: [Action; 13] = [
        Action::Trade,
        Action::Deliver,
        Action::OrderGoods,
        Action::Build,
        Action::BuildTavern,
        Action::BuildSchool,
        Action::BuildBank,
        Action::BuildGraveyard,
        Action::Court,
        Action::CareFamily,
        Action::BorrowBusiness,
        Action::RepayLoan,
        Action::CollectEstate,
    ];

    pub(super) fn inputs(
        &self,
        person: usize,
        action: Action,
        target: u32,
        tick: u64,
    ) -> Option<[f32; 4]> {
        Self::ACTIONS.contains(&action).then(|| {
            self.actions
                .get(&(action as u8))
                .map_or([0.0; 4], |memory| memory.inputs(person, target, tick))
        })
    }

    pub(super) fn commit(&mut self, person: usize, action: Action, target: u32, kind: usize) {
        for (&key, memory) in &mut self.actions {
            if key != action as u8 {
                memory.cancel(person);
            }
        }
        if Self::ACTIONS.contains(&action) {
            self.actions
                .entry(action as u8)
                .or_default()
                .commit(person, target, kind);
        }
    }

    pub(super) fn resolve(
        &mut self,
        person: usize,
        action: Action,
        target: u32,
        kind: usize,
        tick: u64,
        success: bool,
    ) {
        if let Some(memory) = self.actions.get_mut(&(action as u8)) {
            memory.resolve(person, target, kind, tick, success);
        }
    }

    pub(super) fn cancel(&mut self, person: usize, action: Action) {
        if let Some(memory) = self.actions.get_mut(&(action as u8)) {
            memory.cancel(person);
        }
    }

    pub(super) fn forget(&mut self, person: usize) {
        for memory in self.actions.values_mut() {
            memory.forget(person);
        }
        self.trades.remove(&person);
        self.deliveries.retain(|_, (_, owner, _)| *owner != person);
    }

    pub(super) fn cancel_trade(&mut self, person: usize) {
        self.trades.remove(&person);
    }

    pub(super) fn trade_pickup(&mut self, person: usize, target: u32, kind: usize) {
        if let Some(memory) = self.actions.get_mut(&(Action::Trade as u8)) {
            if memory.take_attempt(person, (target, kind)) {
                self.trades.insert(person, (target, kind));
            }
        }
    }

    pub(super) fn trade_sale(
        &mut self,
        person: usize,
        target: u32,
        kind: usize,
        tick: u64,
        delivered: bool,
        finished: bool,
    ) {
        let Some(&planned) = self.trades.get(&person) else {
            return;
        };
        if planned != (target, kind) {
            self.trades.remove(&person);
        } else if finished || !delivered {
            self.trades.remove(&person);
            self.actions
                .entry(Action::Trade as u8)
                .or_default()
                .observe(person, target, tick, delivered);
        }
    }

    pub(super) fn dispatch(&mut self, person: usize, action: Action, target: u32, job: usize) {
        if let Some(memory) = self.actions.get_mut(&(action as u8)) {
            if memory.take_attempt(person, (target, 1)) {
                self.deliveries.insert(job, (action, person, target));
            }
        }
    }

    pub(super) fn delivery_finished(&mut self, job: usize, tick: u64) {
        if let Some((action, person, target)) = self.deliveries.remove(&job) {
            self.actions
                .entry(action as u8)
                .or_default()
                .observe(person, target, tick, true);
        }
    }

    pub(super) fn delivery_cancelled(&mut self, job: usize) {
        self.deliveries.remove(&job);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trade_only_learns_after_sale_and_interrupted_orders_are_neutral() {
        let mut state = Commitments::default();
        state.commit(1, Action::Trade, 3, 2);
        state.trade_pickup(1, 3, 2);
        state.commit(1, Action::Deliver, 3, 2);
        state.trade_sale(1, 3, 2, 12, true, false);
        assert_eq!(state.inputs(1, Action::Trade, 3, 12), Some([0.0; 4]));
        state.trade_sale(1, 3, 2, 13, true, true);
        let learned = state.clone();
        state.trade_sale(1, 3, 2, 14, true, true);
        assert_eq!(state, learned);
        assert_eq!(state.inputs(1, Action::Trade, 3, 13).unwrap()[0], 1.0);

        state.commit(1, Action::Trade, 4, 2);
        state.trade_pickup(1, 4, 2);
        state.cancel_trade(1);
        state.trade_sale(1, 4, 2, 14, true, true);
        assert!(
            state.inputs(1, Action::Trade, 4, 14).unwrap()[1]
                < state.inputs(1, Action::Trade, 3, 14).unwrap()[1]
        );

        state.commit(1, Action::OrderGoods, 9, 1);
        state.dispatch(1, Action::OrderGoods, 9, 4);
        state.commit(1, Action::Rest, 0, 0);
        state.delivery_cancelled(4);
        state.delivery_finished(4, 15);
        assert_eq!(state.inputs(1, Action::OrderGoods, 9, 15), Some([0.0; 4]));
    }
}

// Frozen layout of this module's save payloads in formats v15-v33 (#97).
// Generated from the traced schema; never edit a field of a Legacy* type.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct LegacyCommitments {
    pub actions: BTreeMap<u8, super::food_memory::LegacyFoodMemory>,
    pub trades: BTreeMap<usize, (u32, usize)>,
    pub deliveries: BTreeMap<usize, (crate::legacy_types::LegacyAction, usize, u32)>,
}
impl From<LegacyCommitments> for Commitments {
    fn from(old: LegacyCommitments) -> Self {
        Self {
            actions: old
                .actions
                .into_iter()
                .map(|(k1, x1)| (k1, x1.into()))
                .collect(),
            trades: old.trades,
            deliveries: old
                .deliveries
                .into_iter()
                .map(|(k1, x1)| {
                    (k1, {
                        let (x2_0, x2_1, x2_2) = x1;
                        (x2_0.into(), x2_1, x2_2)
                    })
                })
                .collect(),
        }
    }
}
#[cfg(test)]
impl From<Commitments> for LegacyCommitments {
    fn from(live: Commitments) -> Self {
        Self {
            actions: live
                .actions
                .into_iter()
                .map(|(k1, x1)| (k1, x1.into()))
                .collect(),
            trades: live.trades,
            deliveries: live
                .deliveries
                .into_iter()
                .map(|(k1, x1)| {
                    (k1, {
                        let (x2_0, x2_1, x2_2) = x1;
                        (x2_0.into(), x2_1, x2_2)
                    })
                })
                .collect(),
        }
    }
}
