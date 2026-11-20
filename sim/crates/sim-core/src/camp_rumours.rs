use super::*;
use std::collections::{BTreeMap, BTreeSet};

const RUMOUR_LIFETIME: u64 = 30 * rest::DAY_TICKS;
const CAMP_OBSERVATION_RANGE: f32 = 90.0;
const TAVERN_SERVICE_RANGE: f32 = 6.0;
const TAVERN_NOTICE_RANGE: f32 = 120.0;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub(super) struct CampRumours {
    camps: BTreeMap<u32, u64>,
    people: BTreeSet<(u32, u32)>,
    taverns: BTreeSet<(u32, u32)>,
}

impl World {
    pub(super) fn announce_new_camp(&mut self, camp: u32, founders: &[usize]) {
        self.camp_rumours
            .camps
            .insert(camp, self.tick + RUMOUR_LIFETIME);
        self.camp_rumours
            .people
            .extend(founders.iter().map(|&i| (i as u32, camp)));
    }

    fn fresh_camp_rumour(&self, camp: u32) -> bool {
        self.camp_rumours
            .camps
            .get(&camp)
            .is_some_and(|&expiry| self.tick < expiry)
    }

    pub(super) fn observe_camp_rumours(&mut self, i: usize) {
        if !self.agents.alive[i] || self.camp_rumours.camps.is_empty() {
            return;
        }
        let a = &self.agents;
        for (&id, &expiry) in &self.camp_rumours.camps {
            let camp = &self.markets[id as usize];
            if self.tick < expiry && dist(a.x[i], a.y[i], camp.x, camp.y) <= CAMP_OBSERVATION_RANGE
            {
                self.camp_rumours.people.insert((i as u32, id));
            }
        }
        if let Some(tavern) = self.taverns.iter().position(|site| {
            site.ready && dist(a.x[i], a.y[i], site.x, site.y) <= TAVERN_SERVICE_RANGE
        }) {
            self.exchange_camp_rumours(i, tavern as u32);
        }
    }

    fn exchange_camp_rumours(&mut self, i: usize, tavern: u32) {
        let known: Vec<_> = self
            .camp_rumours
            .people
            .range((i as u32, 0)..=(i as u32, u32::MAX))
            .map(|&(_, camp)| camp)
            .filter(|&camp| self.fresh_camp_rumour(camp))
            .collect();
        let heard: Vec<_> = self
            .camp_rumours
            .taverns
            .range((tavern, 0)..=(tavern, u32::MAX))
            .map(|&(_, camp)| camp)
            .filter(|&camp| self.fresh_camp_rumour(camp))
            .collect();
        self.camp_rumours
            .taverns
            .extend(known.into_iter().map(|camp| (tavern, camp)));
        self.camp_rumours
            .people
            .extend(heard.into_iter().map(|camp| (i as u32, camp)));
    }

    pub(super) fn expire_camp_rumours(&mut self) {
        if self.tick % rest::DAY_TICKS != 0 {
            return;
        }
        let rumours = &mut self.camp_rumours;
        rumours.camps.retain(|_, deadline| self.tick < *deadline);
        rumours.people.retain(|&(i, camp)| {
            self.agents.alive[i as usize] && rumours.camps.contains_key(&camp)
        });
        rumours
            .taverns
            .retain(|&(_, camp)| rumours.camps.contains_key(&camp));
    }

    fn food_on_person(&self, i: usize) -> u32 {
        let a = &self.agents;
        a.berries[i]
            + self.meat_trade.pack(i) as u32
            + a.fish[i]
            + a.rice[i]
            + if [0, 3, 8].contains(&a.cargo_kind[i]) {
                a.cargo[i]
            } else {
                0
            }
    }

    fn can_migrate_to_camp(&self, i: usize, camp: u32) -> bool {
        let a = &self.agents;
        let p = &self.people[i];
        let Some(destination) = self.markets.get(camp as usize) else {
            return false;
        };
        if a.home[i] == camp
            || !a.alive[i]
            || !self.is_adult(i)
            || self.must_stay_home(i)
            || a.hunger[i] >= 45.0
            || a.health[i] < 55.0
            || p.spouse != NONE
            || p.children
                .iter()
                .any(|&child| a.alive[child as usize] && !self.is_adult(child as usize))
            || !self.fresh_camp_rumour(camp)
            || !self.camp_rumours.people.contains(&(i as u32, camp))
        {
            return false;
        }
        let trip = dist(a.x[i], a.y[i], destination.x, destination.y);
        let seconds = trip / self.actor_speed(i, a.x[i], a.y[i]).max(1.0);
        let rate = if a.traits[i] & life::GLUTTON != 0 {
            1.05
        } else {
            0.9
        };
        self.food_on_person(i) >= 1 + ((a.hunger[i] + seconds * rate) / 40.0).floor() as u32
    }

    pub(super) fn camp_rumour_offers(&self, i: usize) -> Vec<Candidate> {
        if self.camp_rumours.camps.is_empty() {
            return Vec::new();
        }
        let a = &self.agents;
        let travel = |x, y| dist(a.x[i], a.y[i], x, y);
        let known: Vec<_> = self
            .camp_rumours
            .people
            .range((i as u32, 0)..=(i as u32, u32::MAX))
            .map(|&(_, camp)| camp)
            .filter(|&camp| self.fresh_camp_rumour(camp))
            .collect();
        let make = |action, target, distance: f32, need, benefit| {
            let mut inputs = [0.0; INPUTS];
            inputs[0] = need;
            inputs[1] = benefit;
            inputs[3] = (distance / 1000.0_f32).min(1.0);
            inputs[5] = if a.action[i] == action && a.target[i] == target {
                0.25
            } else {
                0.0
            };
            inputs[10] = a.energy[i] / 100.0;
            Candidate {
                action,
                target,
                kind: 0,
                inputs,
                score: self.score_action(i, action, target, &inputs),
            }
        };
        let mut offers = Vec::new();
        let tavern = self
            .taverns
            .iter()
            .enumerate()
            .filter_map(|(j, site)| {
                let distance = travel(site.x, site.y);
                if !site.ready {
                    return None;
                }
                let can_share = distance <= 1300.0
                    && known.iter().any(|&camp| {
                        camp != site.market
                            && !self.camp_rumours.taverns.contains(&(j as u32, camp))
                    });
                let can_hear = distance <= TAVERN_NOTICE_RANGE
                    && self
                        .camp_rumours
                        .taverns
                        .range((j as u32, 0)..=(j as u32, u32::MAX))
                        .any(|&(_, camp)| self.fresh_camp_rumour(camp) && !known.contains(&camp));
                (can_share || can_hear).then_some((j, distance))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((id, distance)) = tavern {
            offers.push(make(Action::DiscussCamp, id as u32, distance, 0.65, 0.75));
        }
        for camp in known
            .into_iter()
            .filter(|&camp| self.can_migrate_to_camp(i, camp))
        {
            let site = &self.markets[camp as usize];
            let home = &self.markets[a.home[i] as usize];
            let aspirational = matches!(
                self.people[i].aspiration,
                life::Aspiration::Homeowner | life::Aspiration::Farmer
            );
            let housing_pressure =
                1.0 - (home.housing_blocks as f32 * 32.0 / home.population.max(1) as f32).min(1.0);
            let need = 0.15 + 0.55 * housing_pressure + if aspirational { 0.45 } else { 0.0 };
            offers.push(make(
                Action::MigrateToCamp,
                camp,
                travel(site.x, site.y),
                need,
                0.85,
            ));
        }
        offers.sort_by(|a, b| b.score.total_cmp(&a.score).then(a.target.cmp(&b.target)));
        offers.truncate(3);
        offers
    }

    pub(super) fn camp_rumour_destination(&self, i: usize) -> Option<(f32, f32)> {
        let target = self.agents.target[i] as usize;
        match self.agents.action[i] {
            Action::DiscussCamp => self
                .taverns
                .get(target)
                .filter(|t| t.ready)
                .map(|t| (t.x, t.y)),
            Action::MigrateToCamp => self.markets.get(target).map(|m| (m.x, m.y)),
            _ => None,
        }
    }

    pub(super) fn execute_camp_rumour_action(&mut self, i: usize, action: Action) {
        let target = self.agents.target[i];
        let valid = match action {
            Action::DiscussCamp => {
                self.agents.alive[i] && self.is_adult(i) && !self.must_stay_home(i)
            }
            Action::MigrateToCamp => self.can_migrate_to_camp(i, target),
            _ => false,
        };
        if let Some((x, y)) = self.camp_rumour_destination(i).filter(|_| valid) {
            let (from_x, from_y) = (self.agents.x[i], self.agents.y[i]);
            let d = dist(from_x, from_y, x, y);
            if d > TAVERN_SERVICE_RANGE {
                let step = (self.actor_speed(i, from_x, from_y) * DT).min(d - 5.0);
                self.agents.x[i] += (x - from_x) / d * step;
                self.agents.y[i] += (y - from_y) / d * step;
                self.agents.drain_energy(i, DT * 0.07);
                return;
            }
            if action == Action::DiscussCamp {
                self.exchange_camp_rumours(i, target);
            } else if self.markets[target as usize].population > 0 {
                self.release_bed(i);
                self.agents.home[i] = target;
                self.agents.residence[i] = 0;
                self.agents.dwell_ticks[i] = 0;
            }
        }
        self.agents.action[i] = Action::Wait;
        self.agents.target[i] = NONE;
        self.agents.timer[i] = 0.0;
    }

    pub(super) fn camp_rumour_summary(&self) -> serde_json::Value {
        serde_json::json!(self
            .camp_rumours
            .taverns
            .iter()
            .filter(|&&(_, camp)| self.fresh_camp_rumour(camp))
            .map(
                |&(tavern, camp)| serde_json::json!({"tavern":tavern,"camp":camp,
                "name":self.markets[camp as usize].name,"expires":self.camp_rumours.camps[&camp]})
            )
            .collect::<Vec<_>>())
    }

    pub(super) fn validate_camp_rumours(&self) -> Result<(), String> {
        let valid = |camp: u32| self.camp_rumours.camps.contains_key(&camp);
        if self
            .camp_rumours
            .camps
            .keys()
            .any(|&i| i as usize >= self.markets.len())
            || self
                .camp_rumours
                .people
                .iter()
                .any(|&(i, camp)| i as usize >= self.agents.len() || !valid(camp))
            || self
                .camp_rumours
                .taverns
                .iter()
                .any(|&(i, camp)| i as usize >= self.taverns.len() || !valid(camp))
        {
            return Err("Invalid campsite rumour state".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn founders_spread_news_via_taverns_and_residents_can_move() {
        let mut world = World::new(12, 42).unwrap();
        world.patches[0].x = 330.0;
        world.patches[0].y = 180.0;
        for i in 0..8 {
            let a = &mut world.agents;
            a.x[i] = 330.0;
            a.y[i] = 180.0;
            a.cargo_kind[i] = 1;
            a.cargo[i] = 2;
            a.action[i] = Action::Gather;
            a.dwell_cell[i] = 3 + 4096;
            a.dwell_ticks[i] = 120;
            a.target[i] = 0;
        }
        let resident = 8;
        let home = world.agents.home[resident];
        let (x, y) = (
            world.markets[home as usize].x,
            world.markets[home as usize].y,
        );
        world.taverns.push(social::Tavern {
            market: home,
            x,
            y,
            ready: true,
            progress: 20.0,
            escrow: 0,
            visitors: 0,
            guests: Vec::new(),
        });
        world.agents.x[resident] = x;
        world.agents.y[resident] = y;
        world.people[resident].initial_age_days = 25 * 365;
        world.agents.berries[resident] = 25;
        world.agents.hunger[resident] = 0.0;
        world.ledger.initial = world.stocks();
        world.tick = 10;
        world.settlement_step();
        let camp = 3;
        assert_eq!(world.markets.len(), 4);
        assert!(world.camp_rumours.people.contains(&(0, camp)));
        assert!(!world.camp_rumours.taverns.contains(&(0, camp)));
        world.observe_camp_rumours(resident);
        let choice = |w: &World| {
            let (offers, n) = w.offer_with_limit(resident, None, CANDIDATES);
            offers[..n]
                .iter()
                .any(|c| c.action == Action::MigrateToCamp && c.target == camp)
        };
        assert!(!choice(&world));

        world.people[0].initial_age_days = 25 * 365;
        world.agents.action[0] = Action::DiscussCamp;
        world.agents.target[0] = 0;
        world.execute(0);
        assert!(world.camp_rumour_summary().as_array().unwrap().is_empty());
        for _ in 0..3000 {
            if world.agents.action[0] != Action::DiscussCamp {
                break;
            }
            world.execute(0);
        }
        assert!(world.camp_rumours.taverns.contains(&(0, camp)));
        assert!(!choice(&world));
        world.observe_camp_rumours(resident);
        assert!(choice(&world));
        world.tick = 18;
        world.people[resident].aspiration = life::Aspiration::Homeowner;
        assert_eq!(
            world.request(resident).unwrap().candidate.action,
            Action::MigrateToCamp
        );
        world.agents.action[resident] = Action::MigrateToCamp;
        world.agents.target[resident] = camp;
        world.execute(resident);
        assert_eq!(world.agents.home[resident], home);
        for _ in 0..3000 {
            if world.agents.action[resident] != Action::MigrateToCamp {
                break;
            }
            world.execute(resident);
        }
        assert_eq!(world.agents.home[resident], camp);
        world.check().unwrap();

        let save =
            std::env::temp_dir().join(format!("camp-rumours-{}.snapshot", std::process::id()));

        world.save(&save).unwrap();
        let mut restored = World::load(&save).unwrap();
        std::fs::remove_file(&save).unwrap();
        assert_eq!(world.camp_rumours, restored.camp_rumours);

        restored.tick += RUMOUR_LIFETIME;
        assert!(restored
            .camp_rumour_summary()
            .as_array()
            .unwrap()
            .is_empty());
        restored.tick = restored.tick.next_multiple_of(rest::DAY_TICKS);
        restored.expire_camp_rumours();
        assert_eq!(restored.camp_rumours, CampRumours::default());
    }
}
