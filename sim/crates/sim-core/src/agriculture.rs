use super::*;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FertileRegion {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub fertility: f32,
    pub moisture: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    fn planted() -> World {
        let mut w = World::new(12, 42).unwrap();
        for (kind, q) in [(1, 4), (0, 2)] {
            let patch = w.patches.iter_mut().find(|p| p.kind == kind).unwrap();
            patch.stock -= q;
            w.markets[0].add(kind, q, 0);
        }
        w.post_farm(0);
        assert_eq!(w.farms.len(), 1);
        w
    }
    #[test]
    fn rice_never_matures_before_fifteen_full_days_and_only_yields_once() {
        let mut w = planted();
        w.farms[0].tended_days = 15;
        w.tick = 15 * rest::DAY_TICKS - 10;
        w.farm_step();
        assert_eq!(w.farms[0].harvest_remaining, 0);
        assert_eq!(w.ledger.renewed[8], 0);
        w.set_hour(20.0).unwrap();
        w.farm_step();
        assert_eq!(w.ledger.renewed[8], 0);
        w.tick = 15 * rest::DAY_TICKS;
        w.farm_step();
        let yield_count = w.farms[0].yield_per_crop;
        assert_eq!(w.farms[0].harvest_remaining, yield_count);
        w.farm_step();
        assert_eq!(w.ledger.renewed[8], yield_count);
        w.check().unwrap();
    }
    #[test]
    fn mature_crop_refunds_unused_wages_and_reseeds_existing_plot() {
        let mut w = planted();
        let money = w.coins();
        w.farms[0].tended_days = 15;
        w.tick = 15 * rest::DAY_TICKS;
        w.farm_step();
        assert_eq!(w.farms[0].escrow, 0);
        assert_eq!(w.coins(), money);
        let rice = w.farms[0].harvest_remaining;
        w.farms[0].harvest_remaining = 0;
        w.markets[0].add(8, rice, 0);
        w.post_farm(0);
        assert_eq!(w.farms.len(), 1);
        assert!(!w.farms[0].mature);
        assert_eq!(w.farms[0].escrow, 2);
        assert_eq!(w.coins(), money);
        w.check().unwrap();
    }
    #[test]
    fn poor_soil_or_untended_crops_do_not_produce_rice() {
        let mut w = planted();
        w.tick = 20 * rest::DAY_TICKS;
        w.farm_step();
        assert_eq!(w.farms[0].harvest_remaining, 0);
        let mut dry = World::new(12, 42).unwrap();
        for r in &mut dry.fertility {
            r.fertility = 0.2;
            r.moisture = 0.1;
        }
        for (k, q) in [(1, 4), (0, 2)] {
            let p = dry.patches.iter_mut().find(|p| p.kind == k).unwrap();
            p.stock -= q;
            dry.markets[0].add(k, q, 0);
        }
        assert!(!dry.can_post_farm(0));
        dry.post_farm(0);
        assert!(dry.farms.is_empty());
        dry.check().unwrap();
    }
    #[test]
    fn tending_is_paid_once_per_day_and_harvest_is_carried_physically() {
        let mut w = planted();
        w.agents.target[0] = 0;
        w.agents.x[0] = w.farms[0].x;
        w.agents.y[0] = w.farms[0].y;
        w.agents.timer[0] = 2.0;
        let coins = w.coins();
        let wallet = w.agents.coins[0];
        w.work_memory[work_memory::WorkKind::Farm as usize].commit(0, 0, 8);
        w.execute_farm(0, Action::Farm);
        assert_eq!(w.agents.coins[0], wallet + 2);
        assert_eq!(w.coins(), coins);
        assert_eq!(w.action_memory_inputs(0, Action::Farm, 0).unwrap()[0], 1.0);
        let after_shift = w.work_memory.clone();
        w.execute_farm(0, Action::Farm);
        assert_eq!(w.work_memory, after_shift);
        assert_eq!(w.agents.coins[0], wallet + 2);
        w.farms[0].tended_days = 15;
        w.tick = 15 * rest::DAY_TICKS;
        w.farm_step();
        let before = w.farms[0].harvest_remaining;
        w.agents.timer[0] = 1.0;
        w.work_memory[work_memory::WorkKind::Harvest as usize].commit(0, 0, 8);
        w.execute_farm(0, Action::HarvestRice);
        assert_eq!(
            w.action_memory_inputs(0, Action::HarvestRice, 0).unwrap()[0],
            1.0
        );
        assert_eq!(w.farms[0].harvest_remaining, before - 1);
        assert_eq!(w.agents.cargo[0], 1);
        assert_eq!(w.agents.cargo_kind[0], 8);
        w.check().unwrap();
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Farm {
    pub market: u32,
    pub x: f32,
    pub y: f32,
    pub fertility: f32,
    pub moisture: f32,
    pub planted: u64,
    pub tended_days: u32,
    pub last_tended: u64,
    pub mature: bool,
    pub harvest_remaining: u64,
    pub yield_per_crop: u64,
    pub escrow: u64,
}
impl World {
    fn seeds_available(&self, m: usize) -> bool {
        self.markets[m].stock(8) >= 2 || self.markets[m].stock(0) >= 2
    }
    pub(super) fn farm_food_pressure(&self, m: usize) -> f32 {
        (1.0 - (self.markets[m].stock(0) + self.markets[m].stock(3) + self.markets[m].stock(8))
            as f32
            / (self.markets[m].population.max(1) as f32 * 5.0))
            .clamp(0.0, 1.0)
    }
    pub(super) fn can_post_farm(&self, m: usize) -> bool {
        if self.markets[m].coins < 2 {
            return false;
        }
        let mut count = 0;
        for f in self.farms.iter().filter(|f| f.market as usize == m) {
            count += 1;
            if !f.mature
                && f.escrow == 0
                && f.last_tended != (self.tick - f.planted) / rest::DAY_TICKS
            {
                return true;
            }
            if f.mature && f.escrow == 0 && f.harvest_remaining == 0 && self.seeds_available(m) {
                return true;
            }
        }
        count < 4
            && self.farm_food_pressure(m) > 0.15
            && self.markets[m].stock(1) >= 4
            && self.seeds_available(m)
            && self.fertility.iter().any(|r| {
                r.fertility >= 0.75
                    && self.farm_ground(r.x, r.y, 75.0)
                    && r.moisture >= 0.65
                    && dist(r.x, r.y, self.markets[m].x, self.markets[m].y) < 800.0
            })
    }
    fn take_seeds(&mut self, m: usize) {
        let k = if self.markets[m].stock(8) >= 2 { 8 } else { 0 };
        let mut left = 2;
        while left > 0 {
            left -= self.markets[m].take(k, left).unwrap().quantity;
        }
        self.ledger.crafted_in[k] += 2;
        self.markets[m].quotes[k].offered = self.markets[m].quotes[k]
            .offered
            .min(self.markets[m].stock(k));
    }
    pub(super) fn post_farm(&mut self, m: usize) {
        if !self.can_post_farm(m) {
            return;
        }
        if let Some(index) = self.farms.iter().position(|f| {
            f.market as usize == m
                && f.escrow == 0
                && (!f.mature && f.last_tended != (self.tick - f.planted) / rest::DAY_TICKS
                    || f.mature && f.harvest_remaining == 0)
        }) {
            if self.farms[index].mature {
                if !self.seeds_available(m) {
                    return;
                }
                self.take_seeds(m);
                let f = &mut self.farms[index];
                f.planted = self.tick;
                f.tended_days = 0;
                f.last_tended = u64::MAX;
                f.mature = false;
            }
            self.markets[m].coins -= 2;
            self.farms[index].escrow = 2;
            return;
        }
        if self.farms.iter().filter(|f| f.market as usize == m).count() >= 4
            || self.markets[m].stock(1) < 4
            || !self.seeds_available(m)
        {
            return;
        }
        let Some(region) = self
            .fertility
            .iter()
            .filter(|r| {
                r.fertility >= 0.75 && r.moisture >= 0.65 && self.farm_ground(r.x, r.y, 75.0)
            })
            .min_by(|a, b| {
                dist(a.x, a.y, self.markets[m].x, self.markets[m].y).total_cmp(&dist(
                    b.x,
                    b.y,
                    self.markets[m].x,
                    self.markets[m].y,
                ))
            })
            .cloned()
        else {
            return;
        };
        let index = self.farms.iter().filter(|f| f.market as usize == m).count();
        let x = region.x + (index % 2) as f32 * 50.0 - 25.0;
        let y = region.y + (index / 2) as f32 * 50.0 - 25.0;
        let scale = self
            .patches
            .iter()
            .find(|p| p.kind == 0)
            .map(|p| p.capacity / 600)
            .unwrap_or(1)
            .max(1);
        self.take_seeds(m);
        let mut left = 4;
        while left > 0 {
            left -= self.markets[m].take(1, left).unwrap().quantity;
        }
        self.ledger.built[1] += 4;
        self.markets[m].quotes[1].offered = self.markets[m].quotes[1]
            .offered
            .min(self.markets[m].stock(1));
        self.markets[m].coins -= 2;
        self.farms.push(Farm {
            market: m as u32,
            x,
            y,
            fertility: region.fertility,
            moisture: region.moisture,
            planted: self.tick,
            tended_days: 0,
            last_tended: u64::MAX,
            mature: false,
            harvest_remaining: 0,
            yield_per_crop: 256 * scale,
            escrow: 2,
        });
    }
    pub(super) fn best_farm(&self, i: usize, harvest: bool) -> Option<usize> {
        self.farms
            .iter()
            .enumerate()
            .filter(|(_, f)| {
                if harvest {
                    f.mature
                        && f.harvest_remaining > 0
                        && self.agents.room(i) > 0
                        && (self.agents.cargo[i] == 0 || self.agents.cargo_kind[i] == 8)
                } else {
                    !f.mature
                        && f.escrow >= 2
                        && f.last_tended != (self.tick - f.planted) / rest::DAY_TICKS
                        && self.agents.cargo[i] == 0
                }
            })
            .min_by(|(_, a), (_, b)| {
                dist(a.x, a.y, self.agents.x[i], self.agents.y[i]).total_cmp(&dist(
                    b.x,
                    b.y,
                    self.agents.x[i],
                    self.agents.y[i],
                ))
            })
            .map(|(id, _)| id)
    }
    pub(super) fn execute_farm(&mut self, i: usize, action: Action) {
        let id = self.agents.target[i] as usize;
        if id >= self.farms.len() || !self.is_adult(i) || self.must_stay_home(i) {
            return;
        }
        if !self.farm_ground(self.farms[id].x, self.farms[id].y, 26.0) {
            return;
        }
        let speed = self.work_factor(i, action, 8);
        self.agents.timer[i] += DT * speed;
        let f = &mut self.farms[id];
        if action == Action::Farm {
            let day = (self.tick - f.planted) / rest::DAY_TICKS;
            if f.mature || f.escrow < 2 || f.last_tended == day {
                self.agents.timer[i] = 0.0;
                self.work_memory[work_memory::WorkKind::Farm as usize]
                    .resolve(i, id as u32, 8, self.tick, false);
                return;
            }
            if self.agents.timer[i] < 2.0 {
                return;
            }
            f.last_tended = day;
            f.tended_days += 1;
            f.escrow -= 2;
            self.agents.coins[i] += 2;
            self.markets[f.market as usize].expenses_total += 2;
            let paid_market = f.market as usize;
            self.people[i].farm_shifts += 1;
            self.people[i].jobs += 1;
            self.stats.farm_shifts += 1;
            self.work_memory[work_memory::WorkKind::Farm as usize]
                .resolve(i, id as u32, 8, self.tick, true);
            self.agents.timer[i] = 0.0;
            self.agents.drain_energy(i, 0.5);
            self.credit_education_bonus(i, paid_market, 2);
        } else {
            if !f.mature || f.harvest_remaining == 0 {
                self.work_memory[work_memory::WorkKind::Harvest as usize]
                    .resolve(i, id as u32, 8, self.tick, false);
                return;
            }
            if self.agents.room(i) == 0
                || self.agents.cargo[i] > 0 && self.agents.cargo_kind[i] != 8
                || self.agents.timer[i] < 0.5
            {
                return;
            }
            self.agents.cargo_kind[i] = 8;
            self.agents.cargo[i] += 1;
            self.agents.origin[i] = f.market;
            f.harvest_remaining -= 1;
            self.agents.timer[i] = 0.0;
            self.people[i].harvested += 1;
            self.stats.rice_harvested += 1;
            self.work_memory[work_memory::WorkKind::Harvest as usize]
                .resolve(i, id as u32, 8, self.tick, true);
            self.agents.drain_energy(i, 0.1);
        }
    }
    pub(super) fn farm_step(&mut self) {
        if self.tick % 10 != 0 {
            return;
        }
        let valid: Vec<bool> = self
            .farms
            .iter()
            .map(|f| self.farm_ground(f.x, f.y, 26.0))
            .collect();
        for (f, valid) in self.farms.iter_mut().zip(valid) {
            if valid
                && !f.mature
                && self.tick.saturating_sub(f.planted) >= 15 * rest::DAY_TICKS
                && f.tended_days >= 15
            {
                self.markets[f.market as usize].coins += f.escrow;
                f.escrow = 0;
                f.mature = true;
                f.harvest_remaining = f.yield_per_crop;
                self.ledger.renewed[8] += f.yield_per_crop;
            }
        }
    }
    pub(super) fn farm_summary(&self) -> Vec<serde_json::Value> {
        self.farms.iter().enumerate().map(|(id,f)|serde_json::json!({"id":id,"market":f.market,"x":f.x,"y":f.y,"crop":"rice","fertility":f.fertility,"moisture":f.moisture,"age_days":self.tick.saturating_sub(f.planted) as f32/rest::DAY_TICKS as f32,"tended_days":f.tended_days,"days_until_mature":(15.0-self.tick.saturating_sub(f.planted) as f32/rest::DAY_TICKS as f32).max(0.0),"mature":f.mature,"harvest_remaining":f.harvest_remaining,"farmer_reward":f.escrow})).collect()
    }
}
