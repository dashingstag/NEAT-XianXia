use super::{dist, Action, Candidate, MapSize, World, DT, INPUTS, NONE, WEIGHTS};
use neat_core::FRONTIER_INPUTS;

const RECOVERY_TARGET: f32 = 35.0;
pub(super) const MEAL_ENERGY: f32 = 8.0;
pub(super) const EXHAUSTION_DAMAGE_PER_SECOND: f32 = 0.4;

#[derive(Clone, Copy)]
pub(super) struct NeedPriorities {
    energy: f32,
    rest: f32,
    happiness: f32,
    social: f32,
}

impl NeedPriorities {
    pub(super) fn inputs(self, action: Action, hunger: f32, inputs: &mut [f32; INPUTS]) {
        let food = (hunger / 55.0).clamp(0.0, 1.0);
        let relevant = match action {
            Action::BuyFood
            | Action::ForageMeal
            | Action::EatCarcass
            | Action::HarvestRice
            | Action::BuyMeat => [self.energy * food, 0.0, 0.0, 0.0],
            Action::Rest => [self.energy, self.rest, 0.0, 0.0],
            Action::Recover => [self.energy * 0.8, 0.0, 0.0, 0.0],
            Action::MakeCamp => [self.energy * 0.6, self.rest * 0.6, 0.0, 0.0],
            Action::Play => [0.0, 0.0, self.happiness, 0.0],
            Action::Socialize => [0.0, 0.0, 0.0, self.social],
            _ => [0.0; 4],
        };
        inputs[FRONTIER_INPUTS..].copy_from_slice(&relevant);
    }

    pub(super) fn trace(self, exhausted: bool) -> serde_json::Value {
        serde_json::json!([
            {"need": "Energy", "urgency": self.energy, "critical": exhausted},
            {"need": "Rest", "urgency": self.rest},
            {"need": "Happiness", "urgency": self.happiness},
            {"need": "Social", "urgency": self.social}
        ])
    }
}

impl World {
    pub(super) fn hunger_per_second(&self, i: usize) -> f32 {
        if !self.is_adult(i) {
            0.18
        } else if self.agents.traits[i] & super::life::GLUTTON != 0 {
            1.05
        } else {
            0.9
        }
    }

    pub(super) fn is_food_trip(action: Action) -> bool {
        matches!(
            action,
            Action::ForageMeal | Action::HarvestRice | Action::BuyFood | Action::BuyMeat
        )
    }

    fn seconds_to_food(&self, i: usize, action: Action, travel: f32) -> f32 {
        let a = &self.agents;
        let effort = if action == Action::ForageMeal {
            1.0
        } else {
            0.5
        };
        travel / self.actor_speed(i, a.x[i], a.y[i]).max(1.0) + effort
    }

    pub(super) fn needs_food_trip(&self, i: usize) -> bool {
        let a = &self.agents;
        if a.hunger[i] >= 55.0 {
            return true;
        }
        if self.map.spec.size == MapSize::Classic
            || a.hunger[i] < 25.0
            || a.berries[i] + a.fish[i] + a.rice[i] > 0
            || a.cargo[i] > 0 && [0, 3, 8].contains(&a.cargo_kind[i])
            || self.meat_trade.pack(i) > 0
        {
            return false;
        }
        self.closest_food_patch(i).is_some_and(|id| {
            let p = &self.patches[id];
            let travel = dist(a.x[i], a.y[i], p.x, p.y);
            let seconds = self.seconds_to_food(i, Action::ForageMeal, travel);
            a.hunger[i] + self.hunger_per_second(i) * seconds >= 55.0
        })
    }

    pub(super) fn should_travel_for_food_alone(&self, i: usize) -> bool {
        self.map.spec.size != MapSize::Classic
            && if Self::is_food_trip(self.agents.action[i]) {
                self.needs_food_trip(i)
            } else {
                self.agents.hunger[i] >= 55.0
            }
    }

    pub(super) fn need_priorities(&self, i: usize) -> NeedPriorities {
        let deficit = |level: f32, threshold: f32| (1.0 - level / threshold).clamp(0.0, 1.0);
        NeedPriorities {
            energy: deficit(self.agents.energy[i], 70.0),
            rest: if self.agents.rested[i] {
                0.0
            } else {
                deficit(self.agents.rested_meter[i], 55.0)
            },
            happiness: deficit(self.effective_happiness(i), 65.0),
            social: deficit(self.social[i], 55.0),
        }
    }

    pub(super) fn in_energy_emergency(&self, i: usize) -> bool {
        let a = &self.agents;
        a.energy[i] <= 0.0 || (a.action[i] == Action::Recover && a.energy[i] < RECOVERY_TARGET)
    }

    /// How much food an available action can provide soon, allowing for effort and price.
    pub(super) fn food_readiness(action: Action, travel: f32, price: f32) -> f32 {
        let (nourishment, effort): (f32, f32) = match action {
            Action::HarvestRice => (40.0, 0.5),
            Action::ForageMeal => (40.0, 1.0),
            Action::BuyFood => (40.0, 0.5),
            Action::BuyMeat | Action::EatCarcass => (32.0, 0.5),
            _ => return 0.0,
        };
        let access = (1.0 - travel / 400.0).max(0.0);
        (nourishment / 40.0) * access / ((0.5 + effort) * (1.0 + price.max(0.0) / 4.0))
    }

    pub(super) fn recovery_offers(&self, i: usize) -> Vec<Candidate> {
        let a = &self.agents;
        let mut offers = if a.hunger[i] >= 25.0 && self.is_adult(i) && !self.must_stay_home(i) {
            self.urgent_food_offers(i)
        } else {
            Vec::new()
        };
        if !a.rested[i] {
            if let Some(site) = self.best_rest_site(i) {
                let (x, y) = self.shelter_destination(i, site);
                let travel = dist(a.x[i], a.y[i], x, y);
                offers.push(self.need_offer(i, Action::Rest, site as u32, 0, travel, 0.0));
            }
        }
        if offers.is_empty() || !offers.iter().any(|offer| offer.action == Action::Rest) {
            offers.push(self.need_offer(i, Action::Recover, NONE, 0, 0.0, 0.0));
        }
        offers
    }

    pub(super) fn urgent_food_offers(&self, i: usize) -> Vec<Candidate> {
        let a = &self.agents;
        let travel = |x, y| dist(a.x[i], a.y[i], x, y);
        let mut offers = Vec::new();
        if let Some(id) = self.closest_food_patch(i) {
            let p = &self.patches[id];
            offers.push(self.need_offer(
                i,
                Action::ForageMeal,
                id as u32,
                p.kind,
                travel(p.x, p.y),
                0.0,
            ));
        }
        if let Some(id) = self.best_farm(i, true) {
            let f = &self.farms[id];
            offers.push(self.need_offer(
                i,
                Action::HarvestRice,
                id as u32,
                8,
                travel(f.x, f.y),
                0.0,
            ));
        }
        if a.cargo[i] == 0 && a.berries[i] + a.fish[i] + a.rice[i] < 4 {
            let purchase = self
                .markets
                .iter()
                .filter_map(|m| {
                    [0, 3, 8]
                        .into_iter()
                        .filter_map(|kind| {
                            let q = self.perceived_quote(i, m, kind);
                            (q.offered > 0
                                && q.ask > 0
                                && a.coins[i] >= q.ask
                                && a.room(i) >= WEIGHTS[kind])
                                .then_some((m.id, kind, travel(m.x, m.y), q.ask as f32))
                        })
                        .min_by(|a, b| a.3.total_cmp(&b.3))
                })
                .min_by(|a, b| a.2.total_cmp(&b.2));
            if let Some((id, kind, distance, price)) = purchase {
                offers.push(self.need_offer(i, Action::BuyFood, id, kind, distance, price));
            }
        }
        if self.map.spec.size != super::MapSize::Classic {
            for c in self
                .meat_offers(i)
                .into_iter()
                .filter(|c| c.action == Action::BuyMeat)
            {
                offers.push(self.need_offer(
                    i,
                    c.action,
                    c.target,
                    c.kind,
                    c.inputs[3],
                    -c.inputs[4],
                ));
            }
        }
        offers
    }

    pub(super) fn water_report_offer(&self, i: usize) -> Option<Candidate> {
        let report = self.water.pending.iter().find(|p| p.agent == i as u32)?;
        let a = &self.agents;
        let distance = |m: &super::Market| dist(a.x[i], a.y[i], m.x, m.y);
        let market = self.markets.iter().min_by(|a, b| distance(a).total_cmp(&distance(b)))?;
        let mut offer = self.need_offer(i, Action::DeliverWaterReport, market.id, 0, distance(market), 0.0);
        offer.inputs[0] = 1.0;
        offer.inputs[2] = (self.tick.saturating_sub(report.tick) as f32 / 4800.0).min(1.0);
        offer.score = self.score_action(i, offer.action, offer.target, &offer.inputs);
        Some(offer)
    }

    fn need_offer(
        &self,
        i: usize,
        action: Action,
        target: u32,
        kind: usize,
        travel: f32,
        price: f32,
    ) -> Candidate {
        let a = &self.agents;
        let hunger = a.hunger[i];
        let mut inputs = [0.0; INPUTS];
        let readiness = Self::food_readiness(action, travel, price);
        let food = matches!(
            action,
            Action::ForageMeal | Action::HarvestRice | Action::BuyFood | Action::BuyMeat
        );
        inputs[0] = if food { hunger / 55.0 } else { 0.45 };
        inputs[1] = match action {
            Action::Rest => 0.5,
            Action::Recover => 0.25,
            _ => 0.7,
        };
        inputs[2] = if food {
            if self.map.spec.size == MapSize::Classic {
                0.0
            } else {
                let seconds = self.seconds_to_food(i, action, travel);
                let projected = hunger + self.hunger_per_second(i) * seconds;
                ((projected - 55.0) / 30.0).clamp(0.0, 2.0)
            }
        } else {
            (hunger / 85.0).clamp(0.0, 1.0)
        };
        inputs[3] = if food && self.map.spec.size != MapSize::Classic {
            (self.seconds_to_food(i, action, travel) / 60.0).min(2.0)
        } else {
            (travel / 1000.0).min(1.0)
        };
        inputs[4] = -(price / 8.0).min(1.0);
        inputs[5] = if a.action[i] == action && a.target[i] == target {
            0.25
        } else {
            0.0
        };
        if food && self.map.spec.size != MapSize::Classic {
            let seconds = self.seconds_to_food(i, action, travel);
            let until_damage = (85.0 - hunger).max(0.0) / self.hunger_per_second(i).max(0.001);
            inputs[6] = ((until_damage + a.health[i] / 0.8 - seconds) / 60.0).clamp(-1.0, 1.0);
            if a.action[i] == action && a.target[i] == target {
                inputs[5] = self.water.progress.get(i).copied().flatten()
                    .map_or(0.25, |(_, _, tick)| 0.25 / (1.0 + self.tick.saturating_sub(tick) as f32 / 240.0));
            }
        }
        inputs[10] = a.energy[i] / 100.0;
        inputs[25] = if action == Action::Rest { 1.0 } else { 0.0 };
        inputs[neat_core::FOOD_READINESS_INPUT] = readiness;
        if matches!(action, Action::BuyFood | Action::BuyMeat) {
            inputs[2] += self.market_information_risk(i, &self.markets[target as usize]);
        }
        self.need_priorities(i).inputs(action, hunger, &mut inputs);
        Candidate {
            action,
            target,
            kind,
            inputs,
            score: self.score_action(i, action, target, &inputs),
        }
    }

    pub(super) fn restore_meal_energy(&mut self, i: usize) {
        self.agents.energy[i] = (self.agents.energy[i] + MEAL_ENERGY).min(100.0);
    }

    pub(super) fn execute_recover(&mut self, i: usize) {
        // Catching one's breath works outdoors, but a bed gives much faster recovery.
        if self.agents.energy[i] < RECOVERY_TARGET {
            self.agents.energy[i] = (self.agents.energy[i] + DT * 1.25).min(RECOVERY_TARGET);
        }
    }

    pub(super) fn execute_play(&mut self, i: usize) {
        let mut joy = 8.0;
        if let Some(site) = self.landmarks.get(self.agents.target[i] as usize) {
            let (x, y) = (self.agents.x[i], self.agents.y[i]);
            let (tx, ty) = (site.x, site.y);
            let distance = dist(x, y, tx, ty);
            if distance > 6.0 {
                let step = (self.actor_speed(i, x, y) * DT).min(distance);
                self.walk_toward(i, tx, ty, step);
                self.agents.drain_energy(i, DT * 0.07);
                return;
            }
            joy = 12.0;
        }
        self.agents.timer[i] += DT;
        if self.agents.timer[i] >= 3.0 {
            self.people[i].happiness = (self.people[i].happiness + joy).min(100.0);
            self.agents.timer[i] = 0.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_meal_trip_takes_priority_before_critical_hunger() {
        let mut w =
            World::new_with_size(2, 42, super::super::Hill::default(), MapSize::X25).unwrap();
        w.people[0].initial_age_days = 25 * 365;
        w.agents.berries[0] = 0;
        w.agents.hunger[0] = 35.0;
        let (x, y) = (w.agents.x[0], w.agents.y[0]);
        for patch in w.patches.iter_mut().filter(|p| [0, 3].contains(&p.kind)) {
            patch.x = x + 1_100.0;
            patch.y = y;
        }
        assert!(w.needs_food_trip(0));
        let (offers, n) = w.offer(0);
        assert!(n > 0);
        assert!(offers[..n]
            .iter()
            .all(|offer| World::is_food_trip(offer.action)));
        assert!(offers[..n]
            .iter()
            .any(|offer| offer.action == Action::ForageMeal && offer.inputs[2] > 0.0));
        w.agents.action[0] = Action::ForageMeal;
        assert!(w.should_travel_for_food_alone(0));
        w.agents.berries[0] = 1;
        assert!(!w.needs_food_trip(0));
    }

    #[test]
    fn active_energy_costs_spend_health_after_energy_runs_out() {
        let mut world = World::new(2, 42).unwrap();
        world.agents.traits[0] = 0;
        world.agents.rested[0] = false;
        world.agents.energy[0] = 1.0;
        world.agents.health[0] = 5.0;
        world.agents.drain_energy(0, 3.0);
        assert_eq!(world.agents.energy[0], 0.0);
        assert_eq!(world.agents.health[0], 3.0);
        world.agents.drain_energy(0, 1.0);
        assert_eq!(world.agents.health[0], 2.0);
        world.agents.drain_energy(0, 3.0);
        assert_eq!(world.agents.health[0], 0.0);
        world.step(false);
        assert!(!world.agents.alive[0]);
        assert_ne!(world.agents.corpse[0], NONE);
        world.check().unwrap();
    }

    #[test]
    fn zero_energy_damages_health_until_recovery_or_death() {
        let mut world = World::new(2, 42).unwrap();
        for i in 0..2 {
            world.agents.energy[i] = 0.0;
            world.agents.hunger[i] = 0.0;
            world.agents.action[i] = Action::Wait;
        }
        world.agents.health[0] = 50.0;
        world.agents.health[1] = DT * EXHAUSTION_DAMAGE_PER_SECOND / 2.0;
        world.step(false);
        let expected = 50.0 - DT * EXHAUSTION_DAMAGE_PER_SECOND;
        assert_eq!(world.agents.health[0], expected);
        assert!(world.agents.alive[0]);
        assert_eq!(world.agents.health[1], 0.0);
        assert!(!world.agents.alive[1]);
        assert_ne!(world.agents.corpse[1], NONE);

        world.agents.energy[0] = 20.0;
        world.step(false);
        assert_eq!(world.agents.health[0], expected);
        assert_eq!(world.stats.deaths, 1);
        world.check().unwrap();
    }

    #[test]
    fn low_villager_needs_feed_the_relevant_neat_nodes_in_priority_order() {
        let mut world = World::new(2, 42).unwrap();
        let mut inputs = [0.0; INPUTS];
        world.agents.energy[0] = 60.0;
        world
            .need_priorities(0)
            .inputs(Action::Rest, 0.0, &mut inputs);
        let rested_score = world.brains[0].evaluate(&inputs);
        world.agents.energy[0] = 10.0;
        let needs = world.need_priorities(0);
        needs.inputs(Action::Rest, 0.0, &mut inputs);
        assert!(world.brains[0].evaluate(&inputs) > rested_score);
        assert!(inputs[FRONTIER_INPUTS] > 0.8 && inputs[FRONTIER_INPUTS + 1] > 0.0);
        needs.inputs(Action::Gather, 0.0, &mut inputs);
        assert_eq!(&inputs[FRONTIER_INPUTS..], &[0.0; 4]);
        assert_eq!(needs.trace(false)[2]["need"], "Happiness");
        assert_eq!(needs.trace(false)[3]["need"], "Social");
    }

    #[test]
    fn exhaustion_overrides_work_and_recovers_without_a_bed() {
        let mut world = World::new(2, 42).unwrap();
        world.agents.energy[0] = 0.0;
        world.agents.hunger[0] = 0.0;
        world.agents.action[0] = Action::Gather;
        let (offers, count) = world.offer(0);
        assert_eq!(count, 1);
        assert_eq!(offers[0].action, Action::Recover);
        for _ in 0..120 {
            world.step(false);
        }
        assert_eq!(world.agents.action[0], Action::Recover);
        assert!(world.agents.energy[0] > 12.0);
        assert!(!world.agents.rested[0]);
        assert!(!world.agents.sleeping[0]);
        world.check().unwrap();
    }

    #[test]
    fn exhaustion_uses_food_when_hungry_and_a_bed_when_available() {
        let mut world = World::new(2, 42).unwrap();
        world.agents.energy[0] = 0.0;
        world.agents.hunger[0] = 60.0;
        let recovery = world.recovery_offers(0);
        assert!(recovery.iter().any(|c| c.action == Action::Recover));
        let meal = recovery
            .iter()
            .find(|c| c.action == Action::ForageMeal)
            .unwrap();
        assert_eq!(meal.action, Action::ForageMeal);
        world.agents.x[0] = world.patches[meal.target as usize].x;
        world.agents.y[0] = world.patches[meal.target as usize].y;
        world.agents.action[0] = meal.action;
        world.agents.target[0] = meal.target;
        world.agents.timer[0] = 1.0;
        world.execute_personal(0, Action::ForageMeal);
        assert_eq!(world.agents.energy[0], MEAL_ENERGY);
        assert_eq!(world.agents.hunger[0], 20.0);
        world.agents.energy[0] = 0.0;
        world.markets[0].housing_blocks = 1;
        world.sync_shelters();
        assert_eq!(world.recovery_offers(0)[0].action, Action::Rest);
        world.agents.hunger[0] = 60.0;
        let (offers, n) = world.offer(0);
        assert!(offers[..n].iter().any(|offer| offer.action == Action::Rest));
        assert!(offers[..n].iter().any(|offer| {
            offer.action == Action::ForageMeal
                && offer.inputs[neat_core::FOOD_READINESS_INPUT] > 0.0
        }));
        assert!(offers[..n].iter().all(|offer| offer.score.is_finite()));
    }

    #[test]
    fn recovery_replans_when_a_food_source_runs_out() {
        let mut world = World::new(2, 42).unwrap();
        world.agents.energy[0] = 0.0;
        world.agents.hunger[0] = 60.0;
        let original = world
            .recovery_offers(0)
            .into_iter()
            .find(|offer| offer.action == Action::ForageMeal)
            .unwrap();
        world.patches[original.target as usize].stock = 0;
        let options = world.recovery_offers(0);
        assert!(!options.iter().any(|offer| {
            offer.action == Action::ForageMeal && offer.target == original.target
        }));
        assert!(options.iter().any(|offer| offer.action == Action::Recover));
        assert!(options
            .iter()
            .any(|offer| offer.action == Action::ForageMeal));
    }

    #[test]
    fn play_and_village_socializing_relieve_their_own_needs() {
        let mut world = World::new(2, 42).unwrap();
        world.people[0].happiness = 10.0;
        world.agents.traits[0] = 0;
        let home = world.agents.home[0] as usize;
        world.agents.home[1] = home as u32;
        world.markets[home].population = 2;
        for i in 0..2 {
            world.agents.x[i] = world.markets[home].x;
            world.agents.y[i] = world.markets[home].y;
        }
        let (offers, count) = world.offer(0);
        let offered = |action| {
            offers[..count]
                .iter()
                .find(|offer| offer.action == action)
                .unwrap()
        };
        assert!(offered(Action::Play).inputs[FRONTIER_INPUTS + 2] > 0.8);
        assert_eq!(offered(Action::Socialize).inputs[FRONTIER_INPUTS + 3], 1.0);

        world.agents.action[0] = Action::Play;
        world.agents.timer[0] = 3.0;
        world.execute(0);
        assert_eq!(world.people[0].happiness, 18.0);
        assert_eq!(world.social[0], 0.0);

        world.agents.action[0] = Action::Socialize;
        world.agents.target[0] = NONE;
        world.agents.timer[0] = 5.0;
        world.agents.x[1] += 20.0;
        world.execute(0);
        assert_eq!(world.social[0], 0.0);
        world.agents.x[1] = world.agents.x[0];
        world.agents.timer[0] = 5.0;
        world.execute(0);
        assert_eq!(world.social[0], 8.0);
    }
}
