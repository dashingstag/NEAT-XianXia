use super::*;

const HUNT_RADIUS: f32 = 150.0;
const REACH: f32 = 7.0;
const SPEAR_TIMBER: u32 = 2;
const BOW_TIMBER: u32 = 4;
const BOW_REACH: f32 = 32.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HuntingStats {
    pub spears_crafted: u64,
    pub bows_crafted: u64,
    pub spear_encounters: u64,
    pub bow_encounters: u64,
    pub kills: u64,
    pub escapes: u64,
    pub lost_trails: u64,
    pub injuries: u64,
    pub carcass_portions_eaten: u64,
}

fn threat(animal: &wildlife::Animal, tick: u64) -> f32 {
    if wildlife::SpeciesDef::of(animal.species).predator {
        if animal.adult(tick) {
            0.35
        } else {
            0.12
        }
    } else {
        0.03
    }
}

impl World {
    pub fn hunting_stats(&self) -> HuntingStats {
        self.wildlife.hunting_stats
    }
    pub(super) fn has_hunting_weapon(&self, i: usize) -> bool {
        self.wildlife.spears.contains(&(i as u32)) || self.wildlife.bows.contains(&(i as u32))
    }

    pub(super) fn hunting_proximity(&self, i: usize) -> f32 {
        let (x, y) = (self.agents.x[i], self.agents.y[i]);
        self.wildlife
            .animals
            .iter()
            .filter(|a| a.alive)
            .map(|a| (1.0 - dist(x, y, a.x, a.y) / HUNT_RADIUS).max(0.0))
            .fold(0.0, f32::max)
    }

    pub(super) fn hunting_offers(&self, i: usize) -> Vec<Candidate> {
        if self.map.spec.size == MapSize::Classic
            || !self.agents.alive[i]
            || !self.is_adult(i)
            || self.agents.energy[i] < 12.0
        {
            return Vec::new();
        }
        let a = &self.agents;
        let hunger = a.hunger[i] / 100.0;
        let mut offers = Vec::new();
        let distance = |animal: &wildlife::Animal| dist(a.x[i], a.y[i], animal.x, animal.y);
        if a.hunger[i] > 20.0 {
            if let Some((id, d)) = self
                .carcasses
                .iter()
                .filter(|(_, stock)| **stock > 0)
                .map(|(&id, _)| (id, distance(&self.wildlife.animals[id as usize])))
                .filter(|(_, d)| *d <= HUNT_RADIUS)
                .min_by(|a, b| a.1.total_cmp(&b.1))
            {
                offers.push(hunt_offer(Action::EatCarcass, id, 0, hunger, 0.85, 0.05, d));
            }
        }
        let spear = self.wildlife.spears.contains(&(i as u32));
        let bow = self.wildlife.bows.contains(&(i as u32));
        let timber = if a.cargo_kind[i] == 1 { a.cargo[i] } else { 0 };
        let weapon_goal = self.has_goal(i, life::GoalKind::Weapon);
        let hunt_goal = self.has_goal(i, life::GoalKind::Hunt);
        if (spear || bow || timber >= SPEAR_TIMBER)
            && (weapon_goal
                || hunt_goal
                || hunger > 0.15
                || self.people[i].aspiration == life::Aspiration::Hunter)
        {
            let nearby = self
                .wildlife
                .animals
                .iter()
                .enumerate()
                .filter(|(_, animal)| animal.alive)
                .map(|(id, animal)| (id, animal, distance(animal)))
                .filter(|(_, _, d)| *d <= HUNT_RADIUS);
            let closest = nearby.clone().min_by(|a, b| a.2.total_cmp(&b.2));
            if spear || bow {
                let safer = nearby.min_by(|a, b| {
                    threat(a.1, self.tick)
                        .total_cmp(&threat(b.1, self.tick))
                        .then_with(|| {
                            wildlife::SpeciesDef::of(b.1.species)
                                .portion
                                .cmp(&wildlife::SpeciesDef::of(a.1.species).portion)
                        })
                        .then_with(|| a.2.total_cmp(&b.2))
                        .then_with(|| a.0.cmp(&b.0))
                });
                for (id, animal, d) in closest
                    .into_iter()
                    .chain(safer.filter(|option| closest.is_none_or(|near| near.0 != option.0)))
                {
                    let risk = threat(animal, self.tick) * if bow { 0.45 } else { 1.0 };
                    let portions = wildlife::SpeciesDef::of(animal.species).portion as f32;
                    let hit_chance = 1.0 - if bow { 0.38 } else { 0.30 } - risk;
                    offers.push(hunt_offer(
                        Action::HuntWildlife,
                        id as u32,
                        0,
                        hunger + 0.25,
                        0.55 + 0.12 * portions * hit_chance,
                        risk,
                        d,
                    ));
                }
            }
            if weapon_goal || closest.is_some() {
                if !spear && !bow && timber >= SPEAR_TIMBER {
                    offers.push(hunt_offer(
                        Action::CraftSpear,
                        i as u32,
                        1,
                        hunger + 0.2,
                        0.55,
                        0.0,
                        0.0,
                    ));
                }
                if !bow && timber >= BOW_TIMBER {
                    offers.push(hunt_offer(
                        Action::CraftBow,
                        i as u32,
                        1,
                        hunger + 0.2,
                        0.68,
                        0.0,
                        0.0,
                    ));
                }
            }
        }
        offers
    }

    pub(super) fn execute_hunting(&mut self, i: usize, action: Action) {
        let id = self.agents.target[i];
        if self.map.spec.size == MapSize::Classic
            || !self.agents.alive[i]
            || !self.is_adult(i)
            || self.agents.energy[i] < 12.0
        {
            self.finish_hunt_action(i, action, id, "unavailable");
            return;
        }
        if matches!(action, Action::CraftSpear | Action::CraftBow) {
            let (cost, owners, crafted) = if action == Action::CraftBow {
                (
                    BOW_TIMBER,
                    &mut self.wildlife.bows,
                    &mut self.wildlife.hunting_stats.bows_crafted,
                )
            } else {
                (
                    SPEAR_TIMBER,
                    &mut self.wildlife.spears,
                    &mut self.wildlife.hunting_stats.spears_crafted,
                )
            };
            if self.agents.cargo_kind[i] == 1
                && self.agents.cargo[i] >= cost
                && owners.insert(i as u32)
            {
                *crafted += 1;
                self.agents.cargo[i] -= cost;
                self.ledger.consumed[1] += cost as u64;
                self.finish_hunt_action(i, action, id, "crafted");
            } else {
                self.finish_hunt_action(i, action, id, "unavailable");
            }
            return;
        }
        let Some(animal) = self.wildlife.animals.get(id as usize) else {
            self.finish_hunt_action(i, action, id, "unavailable");
            return;
        };
        let bow = action == Action::HuntWildlife && self.wildlife.bows.contains(&(i as u32));
        let valid = if action == Action::HuntWildlife {
            animal.alive && (bow || self.wildlife.spears.contains(&(i as u32)))
        } else {
            !animal.alive && self.carcasses.get(&id).is_some_and(|&stock| stock > 0)
        };
        if !valid {
            self.finish_hunt_action(i, action, id, "unavailable");
            return;
        }
        let (x, y) = (self.agents.x[i], self.agents.y[i]);
        let d = dist(x, y, animal.x, animal.y);
        if d > HUNT_RADIUS {
            let outcome = if action == Action::HuntWildlife {
                self.wildlife.hunting_stats.lost_trails += 1;
                "lost_trail"
            } else {
                "out_of_range"
            };
            self.finish_hunt_action(i, action, id, outcome);
            return;
        }
        let reach = if bow { BOW_REACH } else { REACH };
        if d > reach {
            let step = (self.actor_speed(i, x, y) * DT).min(d - reach + 0.1);
            self.agents.x[i] += (animal.x - x) / d * step;
            self.agents.y[i] += (animal.y - y) / d * step;
            self.agents.drain_energy(i, DT * 0.07);
            return;
        }
        if action == Action::EatCarcass {
            *self.carcasses.get_mut(&id).expect("available carcass") -= 1;
            self.agents.hunger[i] = (self.agents.hunger[i] - 32.0).max(0.0);
            self.restore_meal_energy(i);
            self.people[i].meals += 1;
            self.wildlife.hunting_stats.carcass_portions_eaten += 1;
            self.finish_hunt_action(i, action, id, "ate_one_portion");
            return;
        }

        let danger = threat(animal, self.tick) * if bow { 0.45 } else { 1.0 };
        let misses = if bow { 0.38 } else { 0.30 };
        if bow {
            self.wildlife.hunting_stats.bow_encounters += 1;
        } else {
            self.wildlife.hunting_stats.spear_encounters += 1;
        }
        let roll = self.wildlife.rng.unit();
        self.agents.drain_energy(i, 4.0);
        let outcome = if roll < danger {
            self.agents.health[i] = (self.agents.health[i] - 15.0).max(0.0);
            "injured"
        } else if roll < danger + misses {
            let animal = &mut self.wildlife.animals[id as usize];
            let (min_x, max_x, min_y, max_y) = self.map.spec.wild_bounds();
            let heading = if d > 0.1 {
                (animal.y - y).atan2(animal.x - x)
            } else {
                animal.heading
            };
            animal.action = 2;
            animal.target = NONE;
            animal.timer = 0.0;
            animal.goal_x = (animal.x + 80.0 * heading.cos()).clamp(min_x, max_x);
            animal.goal_y = (animal.y + 80.0 * heading.sin()).clamp(min_y, max_y);
            "escaped"
        } else {
            let animal = &mut self.wildlife.animals[id as usize];
            animal.alive = false;
            animal.pregnancy = None;
            self.wildlife.ledger.deaths += 1;
            self.carcasses
                .insert(id, wildlife::SpeciesDef::of(animal.species).portion * 4);
            self.urban.progress.entry(i as u32).or_default().hunted += 1;
            "killed"
        };
        self.finish_hunt_action(i, action, id, outcome);
    }

    pub(super) fn finish_hunt_action(
        &mut self,
        i: usize,
        action: Action,
        target: u32,
        outcome: &str,
    ) {
        if action == Action::HuntWildlife {
            let stats = &mut self.wildlife.hunting_stats;
            match outcome {
                "killed" => stats.kills += 1,
                "escaped" => stats.escapes += 1,
                "injured" => stats.injuries += 1,
                _ => {}
            }
        }
        if self.watch.person == Some(i) {
            if let Some(decision) = self
                .watch
                .last
                .as_mut()
                .filter(|d| d["action"] == format!("{action:?}") && d["target"] == target)
            {
                decision["resolution_tick"] = self.tick.into();
                decision["outcome"] = outcome.into();
            }
        }
        self.agents.action[i] = Action::Wait;
        self.agents.target[i] = NONE;
        self.agents.timer[i] = 0.0;
    }
}

fn hunt_offer(
    action: Action,
    target: u32,
    kind: usize,
    need: f32,
    benefit: f32,
    risk: f32,
    travel: f32,
) -> Candidate {
    let mut inputs = [0.0; INPUTS];
    inputs[..4].copy_from_slice(&[need, benefit, risk, travel]);
    Candidate {
        action,
        target,
        kind,
        inputs,
        score: 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frontier() -> World {
        let mut world = World::new_with_size(32, 42, Hill::default(), MapSize::X4).unwrap();
        for person in 0..2 {
            world.people[person].initial_age_days = 25 * 365;
            world.agents.energy[person] = 90.0;
            world.agents.hunger[person] = 50.0;
        }
        world
    }

    fn commit(world: &mut World, person: usize, action: Action, animal: usize) {
        world.agents.action[person] = action;
        world.agents.target[person] = animal as u32;
        world.capture_decision(&Request {
            agent: person,
            candidate: Candidate {
                action,
                target: animal as u32,
                ..Candidate::default()
            },
            counts: None,
            memory_offers: None,
        });
    }

    fn next_roll(world: &mut World, low: f32, high: f32) {
        world.wildlife.rng = (0..100)
            .map(Rng)
            .find(|rng| {
                let roll = rng.clone().unit();
                (low..high).contains(&roll)
            })
            .unwrap();
    }

    #[test]
    fn neat_can_compare_nearer_risk_with_farther_yield() {
        let mut w = frontier();
        let (x, y) = (w.agents.x[0], w.agents.y[0]);
        for animal in &mut w.wildlife.animals {
            animal.x = x + HUNT_RADIUS * 2.0;
        }
        for (id, species, distance) in [(0, 3, 15.0), (1, 1, 80.0), (2, 0, 100.0)] {
            let animal = &mut w.wildlife.animals[id];
            animal.species = species;
            animal.x = x + distance;
            animal.y = y;
        }
        w.agents.hunger[0] = 35.0;
        w.wildlife.spears.insert(0);
        let (offers, n) = w.offer(0);
        let hunt: Vec<_> = offers[..n]
            .iter()
            .filter(|c| c.action == Action::HuntWildlife)
            .collect();
        assert_eq!(hunt.iter().map(|c| c.target).collect::<Vec<_>>(), [0, 1]);
        assert!(hunt[0].inputs[2] > hunt[1].inputs[2]);
        assert!(hunt[0].inputs[3] < hunt[1].inputs[3]);
        assert!(hunt[0].inputs[1] < hunt[1].inputs[1]);
        assert!(hunt.iter().all(|c| c.score.is_finite()));
    }

    #[test]
    fn spear_hunt_and_contested_last_portion_conserve_resources() {
        let mut w = frontier();
        let (x, y) = (w.wildlife.animals[0].x, w.wildlife.animals[0].y);
        let timber = w
            .patches
            .iter_mut()
            .find(|p| p.kind == 1 && p.stock >= 2)
            .unwrap();
        timber.stock -= 2;
        w.agents.cargo_kind[0] = 1;
        w.agents.cargo[0] = 2;
        w.agents.x[0] = x + 25.0;
        w.agents.y[0] = y;
        assert!(w
            .hunting_offers(0)
            .iter()
            .any(|c| c.action == Action::CraftSpear));
        commit(&mut w, 0, Action::CraftSpear, 0);
        w.execute(0);
        assert_eq!(w.agents.cargo[0], 0);
        assert!(w.wildlife.spears.contains(&0));
        assert!(w
            .hunting_offers(0)
            .iter()
            .any(|c| c.action == Action::HuntWildlife));

        w.watch_person(Some(0));
        commit(&mut w, 0, Action::HuntWildlife, 0);
        next_roll(&mut w, 0.9, 1.0);
        w.execute(0);
        assert!(w.wildlife.animals[0].alive);
        assert!(w.agents.x[0] < x + 25.0);
        for _ in 0..30 {
            w.execute(0);
        }
        assert!(!w.wildlife.animals[0].alive);
        assert_eq!(w.carcasses[&0], 4);
        assert_eq!(w.carcass_sites()[0]["id"], 0);
        assert_eq!(w.carcass_sites()[0]["species"], "Rabbit");
        assert_eq!(w.carcass_sites()[0]["portions"], 4);
        assert_eq!(w.last_watched_decision().unwrap()["outcome"], "killed");

        for person in 0..2 {
            w.agents.x[person] = x;
            w.agents.y[person] = y;
        }
        for _ in 0..3 {
            commit(&mut w, 0, Action::EatCarcass, 0);
            w.execute(0);
        }
        assert_eq!(w.carcasses[&0], 1);
        let meals = [w.people[0].meals, w.people[1].meals];
        for person in 0..2 {
            commit(&mut w, person, Action::EatCarcass, 0);
        }
        for person in 0..2 {
            w.execute(person);
        }
        assert_eq!(w.carcasses[&0], 0);
        assert!(w.carcass_sites().as_array().unwrap().is_empty());
        assert_eq!(w.people[0].meals, meals[0] + 1);
        assert_eq!(w.people[1].meals, meals[1]);
        w.check().unwrap();

        let path =
            std::env::temp_dir().join(format!("xianxia-hunting-{}.snapshot", std::process::id()));
        w.save(&path).unwrap();
        let loaded = World::load(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        assert_eq!(
            w.checkpoint_payload().unwrap(),
            loaded.checkpoint_payload().unwrap()
        );
    }

    #[test]
    fn bow_crafting_and_range_are_reflected_in_the_hunting_counters() {
        let mut w = frontier();
        let (x, y) = (w.wildlife.animals[0].x, w.wildlife.animals[0].y);
        w.agents.x[0] = x + 24.0;
        w.agents.y[0] = y;
        let timber = w
            .patches
            .iter_mut()
            .find(|p| p.kind == 1 && p.stock >= 4)
            .unwrap();
        timber.stock -= 4;
        w.agents.cargo_kind[0] = 1;
        w.agents.cargo[0] = 4;
        let (offers, count) = w.offer(0);
        assert!(offers[..count].iter().any(|c| c.action == Action::CraftBow));
        commit(&mut w, 0, Action::CraftBow, 0);
        w.execute(0);
        assert!(w.wildlife.bows.contains(&0));
        assert_eq!(w.agents.cargo[0], 0);
        assert_eq!(w.hunting_stats().bows_crafted, 1);

        next_roll(&mut w, 0.9, 1.0);
        commit(&mut w, 0, Action::HuntWildlife, 0);
        w.execute(0);
        assert_eq!(w.agents.x[0], x + 24.0);
        assert!(!w.wildlife.animals[0].alive);
        assert_eq!(w.hunting_stats().bow_encounters, 1);
        assert_eq!(w.hunting_stats().spear_encounters, 0);
        assert_eq!(w.hunting_stats().kills, 1);
        w.check().unwrap();

        let path =
            std::env::temp_dir().join(format!("xianxia-bow-{}.snapshot", std::process::id()));
        w.save(&path).unwrap();
        let restored = World::load(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        assert!(restored.wildlife.bows.contains(&0));
        assert_eq!(restored.hunting_stats(), w.hunting_stats());
    }

    #[test]
    fn an_encounter_can_end_in_escape_or_injury() {
        let mut w = frontier();
        let target = w
            .wildlife
            .animals
            .iter()
            .position(|a| a.species == 2)
            .unwrap();
        let animal = &w.wildlife.animals[target];
        w.agents.x[0] = animal.x;
        w.agents.y[0] = animal.y;
        w.wildlife.spears.insert(0);
        w.watch_person(Some(0));
        next_roll(&mut w, 0.40, 0.55);
        commit(&mut w, 0, Action::HuntWildlife, target);
        w.execute(0);
        assert_eq!(w.last_watched_decision().unwrap()["outcome"], "escaped");
        assert_eq!(w.wildlife.animals[target].action, 2);
        let health = w.agents.health[0];
        next_roll(&mut w, 0.0, 0.20);
        commit(&mut w, 0, Action::HuntWildlife, target);
        w.execute(0);
        assert_eq!(w.last_watched_decision().unwrap()["outcome"], "injured");
        assert!(w.agents.health[0] < health);
        assert!(w.wildlife.animals[target].alive);
        assert!(!w.carcasses.contains_key(&(target as u32)));
    }
}
