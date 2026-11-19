use super::*;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tavern {
    pub market: u32,
    pub x: f32,
    pub y: f32,
    pub ready: bool,
    pub progress: f32,
    pub escrow: u64,
    pub visitors: u32,
    pub guests: Vec<u32>,
}
impl World {
    pub(super) fn can_fund_tavern(&self, m: usize) -> bool {
        let town = &self.markets[m];
        town.village
            && town.housing_blocks > 0
            && town.stock(1) >= 12
            && town.stock(2) >= 6
            && town.coins >= 20
            && !self.taverns.iter().any(|t| t.market as usize == m)
    }
    pub(super) fn fund_tavern(&mut self, m: usize) {
        if !self.can_fund_tavern(m) {
            return;
        }
        let town = &mut self.markets[m];
        for (k, q) in [(1, 12), (2, 6)] {
            let mut left = q;
            while left > 0 {
                left -= town.take(k, left).expect("reserved materials").quantity;
            }
            self.ledger.built[k] += q;
            town.quotes[k].offered = town.quotes[k].offered.min(town.stock(k));
        }
        town.coins -= 20;
        self.taverns.push(Tavern {
            market: m as u32,
            x: town.x + 35.0,
            y: town.y - 25.0,
            ready: false,
            progress: 0.0,
            escrow: 20,
            visitors: 0,
            guests: vec![],
        });
    }
    pub(super) fn best_tavern(&self, i: usize, building: bool) -> Option<usize> {
        if !self.is_adult(i) || self.must_stay_home(i) || self.agents.cargo[i] > 0 {
            return None;
        }
        if !building && self.social[i] >= 95.0 {
            return None;
        }
        self.taverns
            .iter()
            .enumerate()
            .filter(|(id, t)| {
                t.market == self.agents.home[i]
                    && if building {
                        !t.ready && t.escrow > 0
                    } else {
                        t.ready
                            && (t.visitors < 32
                                || self.agents.action[i] == Action::Socialize
                                    && self.agents.target[i] as usize == *id)
                    }
            })
            .min_by(|(_, a), (_, b)| {
                dist(self.agents.x[i], self.agents.y[i], a.x, a.y).total_cmp(&dist(
                    self.agents.x[i],
                    self.agents.y[i],
                    b.x,
                    b.y,
                ))
            })
            .map(|(id, _)| id)
    }
    pub(super) fn execute_social(&mut self, i: usize, action: Action) {
        let id = self.agents.target[i] as usize;
        if id >= self.taverns.len() || !self.is_adult(i) || self.must_stay_home(i) {
            return;
        }
        if action == Action::BuildTavern {
            if self.taverns[id].ready {
                return;
            }
            self.taverns[id].progress += DT * self.work_factor(i, action, 1);
            if self.taverns[id].progress >= 20.0 {
                let t = &mut self.taverns[id];
                t.ready = true;
                let paid = t.escrow;
                let town = t.market as usize;
                self.agents.coins[i] += t.escrow;
                self.markets[t.market as usize].expenses_total += t.escrow;
                t.escrow = 0;
                self.people[i].jobs += 1;
                self.stats.jobs_completed += 1;
                self.commitments
                    .resolve(i, Action::BuildTavern, id as u32, 1, self.tick, true);
                self.credit_education_bonus(i, town, paid);
            }
            self.agents.drain_energy(i, DT * 0.1);
            return;
        }
        let t = &self.taverns[id];
        if !t.ready || t.visitors < 2 || !t.guests.contains(&(i as u32)) {
            self.agents.timer[i] = 0.0;
            return;
        }
        self.agents.timer[i] += DT;
        if self.agents.timer[i] >= 5.0 {
            let gain = if self.agents.traits[i] & life::GREGARIOUS != 0 {
                12.0
            } else {
                8.0
            };
            self.social[i] = (self.social[i] + gain).min(100.0);
            self.agents.timer[i] = 0.0;
            let peers = t.guests.clone();
            for j in peers {
                let j = j as usize;
                if self.agents.action[j] == Action::Socialize
                    && self.agents.target[j] == id as u32
                    && dist(
                        self.agents.x[i],
                        self.agents.y[i],
                        self.agents.x[j],
                        self.agents.y[j],
                    ) <= 12.0
                {
                    self.relationship_contact(i, j, relationships::Context::Tavern);
                }
            }
        }
    }
    pub(super) fn social_step(&mut self) {
        if self.tick % 10 != 0 {
            return;
        }
        for t in &mut self.taverns {
            t.visitors = 0;
            t.guests.clear();
        }
        for i in 0..self.agents.len() {
            if !self.agents.alive[i] || self.agents.action[i] != Action::Socialize {
                continue;
            }
            if let Some(t) = self.taverns.get_mut(self.agents.target[i] as usize) {
                if t.ready
                    && t.visitors < 32
                    && dist(self.agents.x[i], self.agents.y[i], t.x, t.y) <= 6.0
                {
                    t.visitors += 1;
                    t.guests.push(i as u32);
                }
            }
        }
        if self.tick % rest::DAY_TICKS == 0 {
            for i in 0..self.agents.len() {
                if !self.agents.alive[i] {
                    continue;
                }
                if self.agents.traits[i] & life::SOCIALITE != 0 && self.social[i] >= 60.0 {
                    self.people[i].happiness = (self.people[i].happiness + 2.0).min(100.0);
                }
                self.social[i] = (self.social[i] - 2.0).max(0.0);
            }
        }
    }
    pub(super) fn validate_social(&self) -> Result<(), String> {
        if self.social.len() != self.agents.len()
            || self
                .social
                .iter()
                .any(|v| !v.is_finite() || !(0.0..=100.0).contains(v))
        {
            return Err("Invalid social scores".into());
        }
        for t in &self.taverns {
            if t.market as usize >= self.markets.len()
                || !t.x.is_finite()
                || !t.y.is_finite()
                || !t.progress.is_finite()
                || t.escrow > 20
                || t.ready && t.escrow != 0
            {
                return Err("Invalid tavern".into());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn supplied() -> World {
        let mut w = World::new(8, 42).unwrap();
        w.markets[0].village = true;
        w.markets[0].housing_blocks = 1;
        w.markets[0].population = 64;
        for i in 0..w.agents.len() {
            w.markets[0].coins += w.agents.coins[i];
            w.agents.coins[i] = 0;
        }
        w.agents.target[0] = 0;
        w.agents.traits[0] = 0;
        for (k, q) in [(1, 28), (2, 10)] {
            w.patches.iter_mut().find(|p| p.kind == k).unwrap().stock -= q;
            w.markets[0].add(k, q, 0);
        }
        w
    }
    #[test]
    fn housing_and_taverns_require_materials_and_completed_labor() {
        let mut empty = World::new(8, 42).unwrap();
        empty.markets[0].village = true;
        empty.markets[0].housing_blocks = 1;
        empty.fund_tavern(0);
        empty.start_projects();
        assert!(empty.taverns.is_empty());
        assert!(empty.markets[0].project.is_none());
        let mut w = supplied();
        let money = w.coins();
        w.start_projects();
        assert_eq!(w.markets[0].housing_blocks, 1);
        assert_eq!(w.markets[0].stock(1), 12);
        assert_eq!(w.markets[0].stock(2), 6);
        w.fund_tavern(0);
        assert_eq!(w.taverns.len(), 1);
        assert!(!w.taverns[0].ready);
        w.fund_tavern(0);
        assert_eq!(w.taverns.len(), 1);
        assert_eq!(w.coins(), money);
        assert_eq!(w.markets[0].stock(1), 0);
        for _ in 0..210 {
            w.execute_social(0, Action::BuildTavern);
        }
        assert!(w.taverns[0].ready);
        assert_eq!(w.coins(), money);
        w.markets[0].project.as_mut().unwrap().progress = 10.0;
        w.start_projects();
        assert_eq!(w.markets[0].housing_blocks, 2);
        w.check().unwrap();
        for i in 0..120 {
            let (x, y) = rest::house_origin(i);
            assert!(dist(0.0, 0.0, x, y) > 125.0);
        }
    }
    #[test]
    fn socializing_requires_arrival_company_and_has_no_default_penalty() {
        let mut w = supplied();
        w.fund_tavern(0);
        for _ in 0..210 {
            w.execute_social(0, Action::BuildTavern);
        }
        for i in 0..2 {
            w.agents.action[i] = Action::Socialize;
            w.agents.target[i] = 0;
            w.agents.x[i] = w.taverns[0].x;
            w.agents.y[i] = w.taverns[0].y;
            w.agents.traits[i] = 0;
        }
        w.agents.x[1] += 100.0;
        w.social_step();
        w.agents.timer[0] = 5.0;
        w.execute_social(0, Action::Socialize);
        assert_eq!(w.social[0], 0.0);
        w.agents.x[1] = w.taverns[0].x;
        w.social_step();
        w.agents.traits[1] = life::GREGARIOUS;
        for i in 0..2 {
            w.agents.timer[i] = 5.0;
            w.execute_social(i, Action::Socialize);
        }
        assert_eq!(w.social[0], 8.0);
        assert_eq!(w.social[1], 12.0);
        w.people[0].happiness = 50.0;
        w.people[1].happiness = 50.0;
        w.agents.traits[1] = life::SOCIALITE;
        w.social[0] = 0.0;
        w.social[1] = 70.0;
        w.tick = rest::DAY_TICKS;
        w.social_step();
        assert_eq!(w.people[0].happiness, 50.0);
        assert_eq!(w.people[1].happiness, 52.0);
        w.agents.traits[0] = life::COMMUNITY_MINDED;
        w.social[0] = 60.0;
        assert!((w.work_factor(0, Action::Farm, 8) - 1.1).abs() < 0.001);
        w.check().unwrap();
    }
    #[test]
    fn tavern_and_social_state_checkpoint_replays_exactly() {
        let mut w = supplied();
        w.fund_tavern(0);
        w.social[0] = 72.0;
        let path =
            std::env::temp_dir().join(format!("xianxia-social-{}.snapshot", std::process::id()));
        w.save(&path).unwrap();
        let mut restored = World::load(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        for _ in 0..100 {
            w.step(false);
            restored.step(true);
        }
        assert_eq!(
            postcard::to_allocvec(&w).unwrap(),
            postcard::to_allocvec(&restored).unwrap()
        );
        w.check().unwrap();
    }
    #[test]
    fn children_can_combine_multiple_traits_from_both_parents() {
        let mut w = World::new(2, 42).unwrap();
        w.agents.traits[0] = life::SOCIALITE | life::NIGHT_OWL;
        w.agents.traits[1] = life::GREGARIOUS | life::NIGHT_OWL;
        let mut combined = false;
        for _ in 0..32 {
            w.birth(1, 0);
            let child = w.agents.len() - 1;
            let t = w.agents.traits[child];
            assert_ne!(t & life::NIGHT_OWL, 0);
            assert_eq!(
                t & !(life::SOCIALITE | life::GREGARIOUS | life::NIGHT_OWL),
                0
            );
            combined |= t & life::SOCIALITE != 0 && t & life::GREGARIOUS != 0;
            assert_eq!(w.social[child], 0.0);
        }
        assert!(combined);
        w.validate_social().unwrap();
        w.check().unwrap();
    }
}
