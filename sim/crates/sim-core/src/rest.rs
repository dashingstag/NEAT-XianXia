//! Bounded shelter reservations, deterministic daylight and fatigue rules.
use super::*;
pub const DAY_TICKS: u64 = 3000;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Shelter {
    pub x: f32,
    pub y: f32,
    pub settlement: Option<u32>,
    pub beds: Vec<u32>,
    pub free: Vec<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    fn housed(n: usize) -> World {
        let mut w = World::new(n, 42).unwrap();
        w.markets[0].housing_blocks = 1;
        w.sync_shelters();
        w
    }
    fn sleeper(w: &mut World, i: usize) {
        w.agents.action[i] = Action::Rest;
        w.agents.target[i] = 0;
        w.sync_rest_reservations();
        let (x, y) = w.shelter_destination(i, 0);
        w.agents.x[i] = x;
        w.agents.y[i] = y;
    }
    #[test]
    fn full_day_wraps_and_phase_controls_do_not_advance_physics() {
        let mut w = World::new(1, 42).unwrap();
        assert_eq!(w.hour(), 6.0);
        assert!(!w.is_night());
        w.set_hour(20.0).unwrap();
        assert!(w.is_night());
        assert_eq!(w.tick, 0);
        let h = w.hour();
        w.tick += DAY_TICKS;
        assert_eq!(w.hour(), h);
        assert!(w.set_hour(f32::NAN).is_err());
        assert!(w.set_hour(24.0).is_err());
    }
    #[test]
    fn night_slows_individuals_roads_and_convoys_equally() {
        let mut day = World::new(1, 42).unwrap();
        day.agents.traits[0] = 0;
        day.agents.action[0] = Action::Gather;
        day.agents.target[0] = 0;
        day.agents.cargo_kind[0] = 0;
        day.agents.x[0] = day.patches[0].x - 80.0;
        day.agents.y[0] = day.patches[0].y;
        let mut night = day.clone();
        night.set_hour(20.0).unwrap();
        let x = day.agents.x[0];
        day.execute(0);
        night.execute(0);
        assert!(((night.agents.x[0] - x) / (day.agents.x[0] - x) - 0.65).abs() < 0.001);
        day.roads[0].built = true;
        night.roads[0].built = true;
        // Convoy movement and rest travel share this same road-aware speed function.
        assert!((night.speed_at(180.0, 230.0) / day.speed_at(180.0, 230.0) - 0.65).abs() < 0.001);
    }
    #[test]
    fn rest_is_impossible_without_a_real_reserved_bed() {
        let mut w = World::new(1, 42).unwrap();
        w.agents.energy[0] = 20.0;
        w.agents.x[0] = w.markets[0].x;
        w.agents.y[0] = w.markets[0].y;
        w.execute_rest(0);
        assert_eq!(w.agents.energy[0], 20.0);
        assert!(!w.agents.sleeping[0]);
        w.sync_rest_reservations();
        assert_eq!(w.agents.action[0], Action::Wait);
        assert_eq!(w.shelters[0].beds.len(), 0);
        w.validate_rest().unwrap();
    }
    #[test]
    fn capacity_releases_on_departure_and_death_and_grows_with_houses() {
        let mut w = housed(40);
        for i in 0..40 {
            w.agents.action[i] = Action::Rest;
            w.agents.target[i] = 0;
        }
        w.sync_rest_reservations();
        assert_eq!(
            w.shelters[0].beds.iter().filter(|&&i| i != NONE).count(),
            32
        );
        assert_eq!(
            w.agents
                .action
                .iter()
                .filter(|&&a| a == Action::Wait)
                .count(),
            8
        );
        w.agents.action[0] = Action::Gather;
        w.agents.alive[1] = false;
        w.sync_rest_reservations();
        assert_eq!(w.shelters[0].free.len(), 2);
        w.markets[0].housing_blocks = 2;
        w.sync_shelters();
        assert_eq!(w.shelters[0].beds.len(), 64);
        w.validate_rest().unwrap();
    }
    #[test]
    fn recovery_requires_arrival_and_rest_without_the_buff() {
        let mut w = housed(1);
        w.agents.energy[0] = 20.0;
        w.agents.rested_meter[0] = 79.0;
        w.work_memory[work_memory::WorkKind::Rest as usize].commit(0, 0, 0);
        sleeper(&mut w, 0);
        let home = (w.markets[0].x, w.markets[0].y);
        assert!(dist(w.agents.x[0], w.agents.y[0], home.0, home.1) > 50.0);
        w.execute_rest(0);
        assert!(w.agents.sleeping[0]);
        assert_eq!(w.agents.energy[0], 20.5);
        assert_eq!(w.action_memory_inputs(0, Action::Rest, 0), Some([0.0; 4]));
        w.execute_rest(0);
        assert!(w.agents.rested[0]);
        assert_eq!(w.agents.rested_meter[0], 80.0);
        assert_eq!(w.action_memory_inputs(0, Action::Rest, 0).unwrap()[0], 1.0);
        let energy = w.agents.energy[0];
        w.execute_rest(0);
        assert_eq!(w.agents.energy[0], energy);
        w.agents.drain_energy(0, 50.0);
        assert_eq!(w.agents.energy[0], energy);
        w.agents.action[0] = Action::Wait;
        w.agents.rested_meter[0] = 0.01;
        w.sync_rest_reservations();
        w.step(false);
        assert!(!w.agents.rested[0]);
        assert!(w.agents.energy[0] < energy);
    }
    #[test]
    fn camps_require_materials_and_are_not_created_inside_settlements() {
        let mut w = World::new(1, 42).unwrap();
        assert!(!w.can_make_camp(0));
        let patch = w.patches.iter_mut().find(|p| p.kind == 1).unwrap();
        patch.stock -= 4;
        w.agents.cargo_kind[0] = 1;
        w.agents.cargo[0] = 4;
        w.agents.x[0] = w.markets[0].x;
        w.agents.y[0] = w.markets[0].y;
        assert!(!w.can_make_camp(0));
        w.agents.x[0] = 360.0;
        w.agents.y[0] = 80.0;
        assert!(w.can_make_camp(0));
        w.agents.timer[0] = 5.0;
        w.execute_make_camp(0);
        assert_eq!(w.shelters.len(), 4);
        assert_eq!(w.shelters[3].beds.len(), 4);
        assert_eq!(w.agents.cargo[0], 0);
        w.sync_rest_reservations();
        w.check().unwrap();
        w.validate_rest().unwrap();
    }
    #[test]
    fn default_rest_score_increases_at_night_and_rest_flags_reach_the_viewer() {
        let mut w = housed(1);
        sleeper(&mut w, 0);
        w.agents.hunger[0] = 0.0;
        let (day, n) = w.offer(0);
        let day_score = day[..n]
            .iter()
            .find(|c| c.action == Action::Rest)
            .unwrap()
            .score;
        w.set_hour(20.0).unwrap();
        let (night, n) = w.offer(0);
        let rest = night[..n]
            .iter()
            .find(|c| c.action == Action::Rest)
            .unwrap();
        assert!(rest.score > day_score);
        assert_eq!(rest.inputs[26], 1.0);
        w.agents.rested[0] = true;
        w.execute_rest(0);
        let bytes = w.snapshot(1, Some(0), true);
        let json_len = u32::from_le_bytes(bytes[24..28].try_into().unwrap()) as usize;
        assert_ne!(bytes[32 + json_len + 12] & 64, 0);
        assert_ne!(bytes[32 + json_len + 13] & 128, 0);
    }
    #[test]
    fn night_shelter_reservations_and_rest_status_replay_exactly() {
        let mut a = housed(12);
        sleeper(&mut a, 0);
        a.set_hour(20.0).unwrap();
        a.execute_rest(0);
        let path =
            std::env::temp_dir().join(format!("xianxia-rest-{}.snapshot", std::process::id()));
        a.save(&path).unwrap();
        let mut b = World::load(&path).unwrap();
        for _ in 0..180 {
            a.step(false);
            b.step(true);
        }
        assert_eq!(a.agents, b.agents);
        assert_eq!(a.shelters, b.shelters);
        assert_eq!(a.clock_offset, b.clock_offset);
        a.check().unwrap();
        a.validate_rest().unwrap();
        std::fs::remove_file(path).unwrap();
    }
}
impl Shelter {
    pub(super) fn new(x: f32, y: f32, settlement: Option<u32>, capacity: usize) -> Self {
        Self {
            x,
            y,
            settlement,
            beds: vec![NONE; capacity],
            free: (0..capacity as u32).rev().collect(),
        }
    }
}
impl Agents {
    pub fn drain_energy(&mut self, i: usize, amount: f32) {
        let amount = amount
            * if self.traits[i] & life::HARDY != 0 {
                0.7
            } else {
                1.0
            };
        if !self.rested[i] {
            self.energy[i] = (self.energy[i] - amount).max(0.0);
        }
    }
}
impl World {
    pub fn hour(&self) -> f32 {
        ((self.tick + self.clock_offset + 750) % DAY_TICKS) as f32 / DAY_TICKS as f32 * 24.0
    }
    pub fn is_night(&self) -> bool {
        let hour = self.hour();
        !(6.0..18.0).contains(&hour)
    }
    pub fn night_speed(&self) -> f32 {
        if self.is_night() {
            0.65
        } else {
            1.0
        }
    }
    pub fn set_hour(&mut self, hour: f32) -> Result<(), String> {
        if !hour.is_finite() || !(0.0..24.0).contains(&hour) {
            return Err("Hour must be 0..24".into());
        }
        let desired = (hour / 24.0 * DAY_TICKS as f32).round() as u64;
        let current = (self.tick + self.clock_offset + 750) % DAY_TICKS;
        self.clock_offset += (desired + DAY_TICKS - current) % DAY_TICKS;
        Ok(())
    }
    pub(super) fn clock_summary(&self) -> serde_json::Value {
        let h = self.hour();
        let sunlight = if (7.0..17.0).contains(&h) {
            1.0
        } else if (5.0..7.0).contains(&h) {
            0.35 + (h - 5.0) * 0.325
        } else if (17.0..19.0).contains(&h) {
            1.0 - (h - 17.0) * 0.325
        } else {
            0.35
        };
        serde_json::json!({"day":(self.tick+self.clock_offset+750)/DAY_TICKS+1,"hour":h,"night":self.is_night(),"speed_multiplier":self.night_speed(),"sunlight":sunlight,"cycle_seconds":DAY_TICKS as f32*DT,"rested_people":self.agents.rested.iter().zip(&self.agents.alive).filter(|(r,a)|**r&&**a).count(),"sleeping_people":self.agents.sleeping.iter().filter(|r|**r).count()})
    }
    pub(super) fn shelter_summary(&self) -> Vec<serde_json::Value> {
        self.shelters.iter().enumerate().map(|(id,s)|serde_json::json!({"id":id,"x":s.x,"y":s.y,"settlement":s.settlement,"capacity":s.beds.len(),"reserved":s.beds.len()-s.free.len(),"camp":s.settlement.is_none()})).collect()
    }
    pub(super) fn sync_shelters(&mut self) {
        for m in &self.markets {
            let capacity = (m.housing_blocks as usize * 32).min(1_200_032);
            if let Some(s) = self
                .shelters
                .iter_mut()
                .find(|s| s.settlement == Some(m.id))
            {
                let old = s.beds.len();
                if capacity > old {
                    s.beds.resize(capacity, NONE);
                    s.free.extend((old as u32..capacity as u32).rev());
                }
            } else {
                self.shelters
                    .push(Shelter::new(m.x, m.y, Some(m.id), capacity));
            }
        }
    }
    pub(super) fn release_bed(&mut self, i: usize) {
        let a = &mut self.agents;
        if let Some(s) = self.shelters.get_mut(a.rest_site[i] as usize) {
            if let Some(owner) = s.beds.get_mut(a.bed_slot[i] as usize) {
                if *owner == i as u32 {
                    *owner = NONE;
                    s.free.push(a.bed_slot[i]);
                }
            }
        }
        a.rest_site[i] = NONE;
        a.bed_slot[i] = NONE;
        a.sleeping[i] = false;
    }
    pub(super) fn sync_rest_reservations(&mut self) {
        for i in 0..self.agents.len() {
            if self.agents.rest_site[i] != NONE
                && (!self.agents.alive[i]
                    || !self.can_use_urban_shelter(i, self.agents.rest_site[i] as usize)
                    || self.agents.action[i] != Action::Rest
                    || self.agents.target[i] != self.agents.rest_site[i])
            {
                self.release_bed(i);
            }
        }
        let n = self.agents.len();
        for j in 0..n {
            let i = (j + self.tick as usize % n) % n;
            if !self.agents.alive[i]
                || self.agents.action[i] != Action::Rest
                || self.agents.rest_site[i] != NONE
            {
                continue;
            }
            if !self.can_use_urban_shelter(i, self.agents.target[i] as usize) {
                self.agents.action[i] = Action::Wait;
                continue;
            }
            let Some(s) = self.shelters.get_mut(self.agents.target[i] as usize) else {
                self.agents.action[i] = Action::Wait;
                continue;
            };
            if let Some(slot) = s.free.pop() {
                s.beds[slot as usize] = i as u32;
                self.agents.rest_site[i] = self.agents.target[i];
                self.agents.bed_slot[i] = slot;
            } else {
                self.agents.action[i] = Action::Wait;
                self.agents.sleeping[i] = false;
                self.work_memory[work_memory::WorkKind::Rest as usize].resolve(
                    i,
                    self.agents.target[i],
                    0,
                    self.tick,
                    false,
                );
            }
        }
    }
    pub(super) fn best_rest_site(&self, i: usize) -> Option<usize> {
        let a = &self.agents;
        self.shelters
            .iter()
            .enumerate()
            .filter(|(id, s)| {
                self.can_use_urban_shelter(i, *id)
                    && (!s.free.is_empty() || a.rest_site[i] == *id as u32)
                    && (!self.must_stay_home(i) || s.settlement.is_some())
            })
            .min_by(|(_, u), (_, v)| {
                dist(a.x[i], a.y[i], u.x, u.y).total_cmp(&dist(a.x[i], a.y[i], v.x, v.y))
            })
            .map(|(id, _)| id)
    }
    pub(super) fn shelter_destination(&self, i: usize, site: usize) -> (f32, f32) {
        let s = &self.shelters[site];
        if self.agents.rest_site[i] != site as u32 {
            return (s.x, s.y);
        }
        let slot = self.agents.bed_slot[i];
        if let Some(b) = self
            .urban
            .buildings
            .iter()
            .find(|b| b.shelter == site as u32)
        {
            return (
                b.x - 5.0 + (slot % 2) as f32 * 4.0,
                b.y + (slot / 2) as f32 * 3.0,
            );
        }
        if s.settlement.is_some() {
            // One stable house position per actual 32-bed block.
            let (hx, hy) = house_origin(slot / 32);
            let bed = slot % 32;
            (
                (s.x + hx).clamp(5.0, 2670.0) + 2.0 + (bed % 8) as f32 * 1.9,
                (s.y + hy).clamp(5.0, 1900.0) + 3.0 + (bed / 8) as f32 * 2.5,
            )
        } else {
            (
                s.x + (slot % 2) as f32 * 4.0 - 2.0,
                s.y + (slot / 2) as f32 * 3.0,
            )
        }
    }
    pub(super) fn rest_description(&self, i: usize) -> String {
        let Some(s) = self.shelters.get(self.agents.rest_site[i] as usize) else {
            return "No bed reserved".into();
        };
        if let Some(m) = s.settlement {
            format!(
                "{}  /  house {}  /  bed {}",
                self.markets[m as usize].name,
                self.agents.bed_slot[i] / 32 + 1,
                self.agents.bed_slot[i] % 32 + 1
            )
        } else {
            format!(
                "Roadside camp {}  /  bed {}",
                self.agents.rest_site[i],
                self.agents.bed_slot[i] + 1
            )
        }
    }
    pub(super) fn execute_rest(&mut self, i: usize) {
        let was_sleeping = self.agents.sleeping[i];
        self.agents.sleeping[i] = false;
        let site = self.agents.rest_site[i] as usize;
        let Some(s) = self.shelters.get(site) else {
            return;
        };
        if s.beds.get(self.agents.bed_slot[i] as usize) != Some(&(i as u32)) {
            return;
        }
        let (x, y) = self.shelter_destination(i, site);
        let d = dist(self.agents.x[i], self.agents.y[i], x, y);
        if d > 1.0 {
            let speed = self.actor_speed(i, self.agents.x[i], self.agents.y[i]);
            let step = (speed * DT).min(d);
            self.agents.x[i] += (x - self.agents.x[i]) / d * step;
            self.agents.y[i] += (y - self.agents.y[i]) / d * step;
            self.agents.drain_energy(i, DT * 0.07);
            return;
        }
        let a = &mut self.agents;
        a.sleeping[i] = true;
        if !a.rested[i] {
            a.energy[i] = (a.energy[i] + DT * 5.0).min(100.0);
        }
        a.rested_meter[i] = (a.rested_meter[i] + DT * 5.0).min(100.0);
        if a.rested_meter[i] >= 80.0 {
            if !a.rested[i] {
                self.people[i].rest_cycles += 1;
                self.work_memory[work_memory::WorkKind::Rest as usize].resolve(
                    i,
                    site as u32,
                    0,
                    self.tick,
                    true,
                );
            }
            a.rested[i] = true;
        }
        if a.hunger[i] < 40.0 {
            a.health[i] = (a.health[i] + DT * 0.3).min(100.0);
        }
        if !was_sleeping {
            self.relationship_sleep_arrival(i, site);
        }
    }
    pub(super) fn can_make_camp(&self, i: usize) -> bool {
        let a = &self.agents;
        a.cargo_kind[i] == 1
            && a.cargo[i] >= 4
            && a.energy[i] > 2.0
            && self
                .shelters
                .iter()
                .filter(|s| s.settlement.is_none())
                .count()
                < 256
            && self
                .markets
                .iter()
                .all(|m| dist(a.x[i], a.y[i], m.x, m.y) > settlement_radius(m.housing_blocks))
            && self
                .shelters
                .iter()
                .filter(|s| s.settlement.is_none())
                .all(|s| dist(a.x[i], a.y[i], s.x, s.y) > 30.0)
            && a.x[i] < 2700.0
    }
    pub(super) fn execute_make_camp(&mut self, i: usize) {
        if !self.can_make_camp(i) {
            if let Some(site) = self.best_rest_site(i) {
                if dist(
                    self.agents.x[i],
                    self.agents.y[i],
                    self.shelters[site].x,
                    self.shelters[site].y,
                ) < 40.0
                {
                    self.agents.action[i] = Action::Rest;
                    self.agents.target[i] = site as u32;
                    return;
                }
            }
            self.agents.action[i] = Action::Wait;
            return;
        }
        self.agents.timer[i] += DT;
        self.agents.drain_energy(i, DT * 0.1);
        if self.agents.timer[i] < 5.0 {
            return;
        }
        self.agents.cargo[i] -= 4;
        self.ledger.built[1] += 4;
        let site = self.shelters.len() as u32;
        self.shelters
            .push(Shelter::new(self.agents.x[i], self.agents.y[i], None, 4));
        self.agents.action[i] = Action::Rest;
        self.agents.target[i] = site;
        self.agents.timer[i] = 0.0;
    }
    pub(super) fn validate_rest(&self) -> Result<(), String> {
        if self.shelters.len() > 320 {
            return Err("Too many shelters".into());
        }
        let mut seen = vec![false; self.agents.len()];
        for (site, s) in self.shelters.iter().enumerate() {
            if !s.x.is_finite()
                || !s.y.is_finite()
                || s.beds.len() > 1_200_032
                || s.settlement
                    .is_some_and(|id| id as usize >= self.markets.len())
            {
                return Err("Invalid shelter".into());
            }
            let mut free = vec![false; s.beds.len()];
            for &slot in &s.free {
                if slot as usize >= free.len()
                    || free[slot as usize]
                    || s.beds[slot as usize] != NONE
                {
                    return Err("Invalid free beds".into());
                }
                free[slot as usize] = true;
            }
            for (slot, &owner) in s.beds.iter().enumerate() {
                if owner == NONE {
                    if !free[slot] {
                        return Err("Missing free bed".into());
                    }
                    continue;
                }
                let i = owner as usize;
                if i >= seen.len()
                    || seen[i]
                    || self.agents.rest_site[i] != site as u32
                    || self.agents.bed_slot[i] != slot as u32
                {
                    return Err("Invalid bed reservation".into());
                }
                seen[i] = true;
            }
        }
        for (i, &has_bed) in seen.iter().enumerate() {
            if has_bed != (self.agents.rest_site[i] != NONE)
                || !self.agents.rested_meter[i].is_finite()
                || !(0.0..=100.0).contains(&self.agents.rested_meter[i])
            {
                return Err("Invalid rested state".into());
            }
        }
        Ok(())
    }
}

pub(super) fn house_origin(mut house: u32) -> (f32, f32) {
    let original = house;
    let mut ring = 0;
    while house >= 12 + ring * 6 {
        house -= 12 + ring * 6;
        ring += 1;
    }
    let angle = house as f32 * std::f32::consts::TAU / (12 + ring * 6) as f32
        + ring as f32 * 0.47
        + ((original * 53 % 17) as f32 - 8.0) * 0.008;
    let radius = 140.0 + ring as f32 * 42.0 + (original * 37 % 19) as f32 - 9.0;
    (angle.cos() * radius, angle.sin() * radius)
}
pub(super) fn settlement_radius(houses: u32) -> f32 {
    let mut remaining = houses.saturating_sub(1);
    let mut ring = 0;
    while remaining >= 12 + ring * 6 {
        remaining -= 12 + ring * 6;
        ring += 1;
    }
    (140.0 + ring as f32 * 42.0 + 35.0).max(175.0)
}
