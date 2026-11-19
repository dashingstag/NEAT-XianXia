//! Funded classrooms, qualified teachers and fractional education wage entitlements.
use super::*;
use std::collections::BTreeMap;
pub const EDUCATED: u16 = 1024;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct School {
    pub market: u32,
    pub x: f32,
    pub y: f32,
    pub ready: bool,
    pub progress: f32,
    pub build_escrow: u64,
    pub wage_escrow: u64,
    pub teacher: u32,
    pub last_taught: u64,
    pub session_day: u64,
    pub lesson_seconds: f32,
    pub students: BTreeMap<u32, f32>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Education {
    pub schools: Vec<School>,
    pub attendance: Vec<u16>,
    pub teaching_shifts: Vec<u64>,
    pub children: Vec<u32>,
    pub pupils: Vec<u32>,
    pub uneducated_children: Vec<u32>,
    pub bonus_claims: BTreeMap<(u32, u32), u64>,
    pub graduated: u64,
    pub lessons: u64,
    pub bonuses_paid: u64,
}
impl World {
    pub fn prepare_school_demo(&mut self) -> Result<(), String> {
        if self.agents.len() < 2 {
            return Err("School demo requires two adults".into());
        }
        self.people[0].gender = 0;
        self.people[1].gender = 1;
        self.people[0].initial_age_days = 55 * 365;
        self.people[0].aspiration = life::Aspiration::Teacher;
        self.people[0].goals.clear();
        self.agents.home[0] = 0;
        self.agents.home[1] = 0;
        self.agents.x[0] = self.markets[0].x;
        self.agents.y[0] = self.markets[0].y;
        self.birth(1, 0);
        let child = self.agents.len() - 1;
        self.people[child].initial_age_days = 8 * 365;
        self.agents.x[child] = self.markets[0].x;
        self.agents.y[child] = self.markets[0].y;
        for (kind, house, school) in [(1, 16, 20), (2, 4, 8)] {
            let qty = house + school;
            let p = self
                .patches
                .iter_mut()
                .find(|p| p.kind == kind && p.stock >= qty)
                .ok_or("Insufficient scenario resources")?;
            p.stock -= qty;
            self.ledger.built[kind] += house;
            self.markets[0].add(kind, school, 0);
        }
        self.markets[0].housing_blocks = 1;
        self.markets[0].village = true;
        self.markets[0].cancel_quotes();
        let mut needed = 200;
        for i in 0..self.agents.len() {
            let q = self.agents.coins[i].min(needed);
            self.agents.coins[i] -= q;
            self.markets[0].coins += q;
            needed -= q;
            if needed == 0 {
                break;
            }
        }
        self.sync_education();
        self.education.children[0] = 1;
        self.check()
    }
    pub(super) fn sync_education(&mut self) {
        self.education.attendance.resize(self.agents.len(), 0);
        self.education.teaching_shifts.resize(self.agents.len(), 0);
        self.education.children.resize(self.markets.len(), 0);
        self.education.pupils.resize(self.markets.len(), 0);
        self.education
            .uneducated_children
            .resize(self.markets.len(), 0);
    }
    pub fn teacher_eligible(&self, i: usize) -> bool {
        i < self.people.len()
            && self.agents.alive[i]
            && self.is_adult(i)
            && (self.people[i].age_days(self.tick) >= 50 * 365
                || self.agents.traits[i] & EDUCATED != 0)
    }
    fn pupil_eligible(&self, i: usize) -> bool {
        self.agents.alive[i]
            && !self.is_adult(i)
            && self.people[i].age_days(self.tick) >= 5 * 365
            && self.agents.traits[i] & EDUCATED == 0
    }
    pub(super) fn can_fund_school(&self, m: usize) -> bool {
        let t = &self.markets[m];
        t.village
            && t.housing_blocks > 0
            && self.education.children.get(m).copied().unwrap_or(0) > 0
            && t.stock(1) >= 20
            && t.stock(2) >= 8
            && t.coins >= 30
            && !self
                .education
                .schools
                .iter()
                .any(|s| s.market as usize == m)
    }
    pub(super) fn fund_school(&mut self, m: usize) {
        if !self.can_fund_school(m) {
            return;
        }
        let t = &mut self.markets[m];
        for (k, q) in [(1, 20), (2, 8)] {
            let mut left = q;
            while left > 0 {
                left -= t.take(k, left).expect("funded school materials").quantity;
            }
            self.ledger.built[k] += q;
            t.quotes[k].offered = t.quotes[k].offered.min(t.stock(k));
        }
        t.coins -= 30;
        self.education.schools.push(School {
            market: m as u32,
            x: t.x - 38.0,
            y: t.y - 28.0,
            ready: false,
            progress: 0.0,
            build_escrow: 30,
            wage_escrow: 0,
            teacher: NONE,
            last_taught: u64::MAX,
            session_day: u64::MAX,
            lesson_seconds: 0.0,
            students: BTreeMap::new(),
        });
    }
    pub(super) fn can_post_teacher(&self, m: usize) -> bool {
        self.markets[m].coins >= 12
            && self
                .education
                .uneducated_children
                .get(m)
                .copied()
                .unwrap_or(0)
                > 0
            && self.education.schools.iter().any(|s| {
                s.market as usize == m
                    && s.ready
                    && s.wage_escrow == 0
                    && s.last_taught != self.tick / rest::DAY_TICKS
            })
    }
    pub(super) fn post_teacher(&mut self, m: usize) {
        if !self.can_post_teacher(m) {
            return;
        }
        if let Some(s) = self.education.schools.iter_mut().find(|s| {
            s.market as usize == m
                && s.ready
                && s.wage_escrow == 0
                && s.last_taught != self.tick / rest::DAY_TICKS
        }) {
            self.markets[m].coins -= 12;
            s.wage_escrow = 12;
        }
    }
    pub(super) fn best_school(&self, i: usize, action: Action) -> Option<usize> {
        if action != Action::AttendSchool
            && (self.agents.cargo[i] > 0 || self.agents.coins[i] == 0 && self.agents.room(i) == 0)
        {
            return None;
        }
        self.education
            .schools
            .iter()
            .enumerate()
            .filter(|(id, s)| {
                if s.market != self.agents.home[i] {
                    return false;
                }
                match action {
                    Action::BuildSchool => {
                        self.is_adult(i)
                            && !self.must_stay_home(i)
                            && !s.ready
                            && s.build_escrow > 0
                    }
                    Action::Teach => {
                        self.teacher_eligible(i)
                            && !self.must_stay_home(i)
                            && self.agents.hunger[i] < 65.0
                            && self.agents.energy[i] > 8.0
                            && s.ready
                            && s.wage_escrow >= 10
                            && s.last_taught != self.tick / rest::DAY_TICKS
                            && (s.teacher == NONE || s.teacher == i as u32)
                            && (7.0..17.0).contains(&self.hour())
                            && self
                                .education
                                .pupils
                                .get(s.market as usize)
                                .copied()
                                .unwrap_or(0)
                                > 0
                    }
                    Action::AttendSchool => {
                        self.pupil_eligible(i)
                            && s.ready
                            && s.wage_escrow > 0
                            && s.last_taught != self.tick / rest::DAY_TICKS
                            && (7.0..17.0).contains(&self.hour())
                    }
                    _ => {
                        let _ = id;
                        false
                    }
                }
            })
            .map(|(id, _)| id)
            .next()
    }
    pub(super) fn execute_education(&mut self, i: usize, action: Action) {
        let id = self.agents.target[i] as usize;
        if id >= self.education.schools.len() {
            return;
        }
        if action == Action::BuildSchool {
            if !self.is_adult(i) || self.must_stay_home(i) || self.education.schools[id].ready {
                return;
            }
            self.education.schools[id].progress += DT * self.work_factor(i, action, 1);
            if self.education.schools[id].progress >= 25.0 {
                let s = &mut self.education.schools[id];
                s.ready = true;
                let reward = s.build_escrow;
                s.build_escrow = 0;
                self.agents.coins[i] += reward;
                let m = s.market as usize;
                self.markets[m].expenses_total += reward;
                self.people[i].jobs += 1;
                self.credit_education_bonus(i, m, reward);
            }
            return;
        }
        if action == Action::Teach && self.teacher_eligible(i) && !self.must_stay_home(i) {
            let s = &mut self.education.schools[id];
            if s.teacher == NONE || s.teacher == i as u32 {
                s.teacher = i as u32;
            }
        }
    }
    pub(super) fn credit_education_bonus(&mut self, i: usize, m: usize, base: u64) {
        if self.agents.traits[i] & EDUCATED == 0 {
            return;
        }
        let claim = self
            .education
            .bonus_claims
            .entry((i as u32, m as u32))
            .or_default();
        *claim += base * 15;
        let pay = (*claim / 100).min(self.markets[m].coins);
        *claim -= pay * 100;
        self.markets[m].coins -= pay;
        self.markets[m].expenses_total += pay;
        self.agents.coins[i] += pay;
        self.education.bonuses_paid += pay;
    }
    pub(super) fn education_step(&mut self) {
        if self.tick % 10 != 0 {
            return;
        }
        self.sync_education();
        self.education.children.fill(0);
        self.education.pupils.fill(0);
        self.education.uneducated_children.fill(0);
        for i in 0..self.agents.len() {
            if self.agents.alive[i] && !self.is_adult(i) {
                let m = self.agents.home[i] as usize;
                self.education.children[m] += 1;
                if self.agents.traits[i] & EDUCATED == 0 {
                    self.education.uneducated_children[m] += 1;
                }
                if self.pupil_eligible(i) {
                    self.education.pupils[m] += 1;
                }
            }
        }
        let day = self.tick / rest::DAY_TICKS;
        let open = (7.0..17.0).contains(&self.hour());
        for id in 0..self.education.schools.len() {
            let t = self.education.schools[id].teacher;
            let valid = t != NONE
                && self.teacher_eligible(t as usize)
                && !self.must_stay_home(t as usize)
                && self.agents.action[t as usize] == Action::Teach
                && self.agents.target[t as usize] == id as u32
                && self.agents.hunger[t as usize] < 70.0
                && self.agents.energy[t as usize] > 1.0;
            if !valid {
                self.education.schools[id].teacher = NONE;
            }
            if self.education.schools[id].session_day != day {
                let s = &mut self.education.schools[id];
                s.session_day = day;
                s.students.clear();
                s.lesson_seconds = 0.0;
            }
            let s = &self.education.schools[id];
            if !open
                || !valid
                || !s.ready
                || s.last_taught == day
                || s.wage_escrow < 10
                || dist(
                    self.agents.x[t as usize],
                    self.agents.y[t as usize],
                    s.x,
                    s.y,
                ) > 10.0
            {
                continue;
            }
            let mut pupils = vec![];
            for offset in 0..self.agents.len() {
                let j = (offset + day as usize) % self.agents.len();
                if self.pupil_eligible(j)
                    && self.agents.home[j] == s.market
                    && self.agents.action[j] == Action::AttendSchool
                    && self.agents.target[j] == id as u32
                    && dist(self.agents.x[j], self.agents.y[j], s.x, s.y) < 15.0
                {
                    pupils.push(j);
                    if pupils.len() == 32 {
                        break;
                    }
                }
            }
            if pupils.is_empty() {
                continue;
            }
            let s = &mut self.education.schools[id];
            s.lesson_seconds += 1.0;
            for j in pupils {
                *s.students.entry(j as u32).or_default() += 1.0;
            }
            if s.lesson_seconds < 10.0 {
                continue;
            }
            let m = s.market as usize;
            let refund = s.wage_escrow - 10;
            s.wage_escrow = 0;
            s.last_taught = day;
            let graduates: Vec<_> = s
                .students
                .iter()
                .filter(|(_, secs)| **secs >= 8.0)
                .map(|(j, _)| *j as usize)
                .collect();
            s.students.clear();
            s.lesson_seconds = 0.0;
            s.teacher = NONE;
            self.markets[m].coins += refund;
            self.agents.coins[t as usize] += 10;
            self.markets[m].expenses_total += 10;
            self.people[t as usize].jobs += 1;
            self.education.teaching_shifts[t as usize] += 1;
            self.education.lessons += 1;
            self.credit_education_bonus(t as usize, m, 10);
            for j in graduates {
                self.education.attendance[j] += 1;
                if self.education.attendance[j] >= 10 && self.agents.traits[j] & EDUCATED == 0 {
                    self.agents.traits[j] |= EDUCATED;
                    self.education.graduated += 1;
                    self.people[j].happiness = (self.people[j].happiness + 10.0).min(100.0);
                }
            }
        }
        // Unpaid education entitlements retain their fractional value and village ownership.
        for (&(i, m), claim) in &mut self.education.bonus_claims {
            if !self.agents.alive[i as usize]
                || self.agents.coins[i as usize] == 0 && self.agents.room(i as usize) == 0
            {
                continue;
            }
            let pay = (*claim / 100).min(self.markets[m as usize].coins);
            *claim -= pay * 100;
            self.markets[m as usize].coins -= pay;
            self.markets[m as usize].expenses_total += pay;
            self.agents.coins[i as usize] += pay;
            self.education.bonuses_paid += pay;
        }
    }
    pub(super) fn school_summary(&self) -> serde_json::Value {
        serde_json::json!(self.education.schools.iter().enumerate().map(|(id,s)|serde_json::json!({"id":id,"market":s.market,"x":s.x,"y":s.y,"ready":s.ready,"progress":s.progress,"teacher":if s.teacher==NONE{None}else{Some(s.teacher)},"teacher_opening":s.ready&&s.wage_escrow>0&&s.teacher==NONE,"base_wage":10,"wage_escrow":s.wage_escrow,"build_escrow":s.build_escrow,"capacity":32,"children":self.education.children.get(s.market as usize).copied().unwrap_or(0),"lesson_seconds":s.lesson_seconds})).collect::<Vec<_>>())
    }
    pub(super) fn validate_education(&self) -> Result<(), String> {
        if self.education.attendance.len() != self.agents.len()
            || self.education.teaching_shifts.len() != self.agents.len()
        {
            return Err("Invalid education pool".into());
        }
        if self
            .education
            .bonus_claims
            .keys()
            .any(|(i, m)| *i as usize >= self.agents.len() || *m as usize >= self.markets.len())
        {
            return Err("Invalid education wage claim".into());
        }
        for s in &self.education.schools {
            if s.market as usize >= self.markets.len()
                || s.wage_escrow > 12
                || s.build_escrow > 30
                || s.teacher != NONE && s.teacher as usize >= self.agents.len()
            {
                return Err("Invalid school".into());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn family() -> World {
        let mut w = World::new(2, 42).unwrap();
        w.people[0].gender = 0;
        w.people[1].gender = 1;
        w.people[0].initial_age_days = 55 * 365;
        w.people[0].aspiration = life::Aspiration::Teacher;
        w.birth(1, 0);
        w.people[2].initial_age_days = 8 * 365;
        w.agents.home[2] = 0;
        w.agents.home[0] = 0;
        w.agents.traits[0] = 0;
        w.markets[0].village = true;
        w.markets[0].housing_blocks = 1;
        for (k, q) in [(1, 20), (2, 8)] {
            w.patches.iter_mut().find(|p| p.kind == k).unwrap().stock -= q;
            w.markets[0].add(k, q, 0);
        }
        w.markets[0].coins += 500;
        w.ledger.initial_coins += 500;
        w.tick = 20;
        w.set_hour(9.0).unwrap();
        w.education_step();
        w
    }
    #[test]
    fn teacher_qualification_is_age_or_education_and_always_requires_adulthood() {
        let mut w = family();
        w.people[0].initial_age_days = 49 * 365;
        assert!(!w.teacher_eligible(0));
        w.people[0].initial_age_days = 50 * 365;
        assert!(w.teacher_eligible(0));
        w.people[0].initial_age_days = 18 * 365;
        w.agents.traits[0] |= EDUCATED;
        assert!(w.teacher_eligible(0));
        w.people[0].initial_age_days = 17 * 365;
        assert!(!w.teacher_eligible(0));
    }
    #[test]
    fn school_requires_child_materials_cash_and_real_labor() {
        let mut empty = World::new(2, 42).unwrap();
        assert!(!empty.can_fund_school(0));
        empty.fund_school(0);
        assert!(empty.education.schools.is_empty());
        let mut w = family();
        let money = w.coins();
        let wood = w.markets[0].stock(1);
        w.fund_school(0);
        assert_eq!(w.markets[0].stock(1), wood - 20);
        assert!(!w.education.schools[0].ready);
        assert_eq!(w.coins(), money);
        w.fund_school(0);
        assert_eq!(w.education.schools.len(), 1);
        w.agents.target[0] = 0;
        for _ in 0..260 {
            w.execute_education(0, Action::BuildSchool);
        }
        assert!(w.education.schools[0].ready);
        assert_eq!(w.coins(), money);
        w.check().unwrap();
    }
    #[test]
    fn ten_physically_attended_days_educate_child_and_complete_teacher_aspiration() {
        let mut w = family();
        let money = w.coins();
        w.fund_school(0);
        w.agents.target[0] = 0;
        for _ in 0..260 {
            w.execute_education(0, Action::BuildSchool);
        }
        for day in 0..10 {
            w.tick = day * rest::DAY_TICKS + 20;
            w.set_hour(9.0).unwrap();
            w.post_teacher(0);
            assert!(w.education.schools[0].wage_escrow >= 10);
            let (x, y) = (w.education.schools[0].x, w.education.schools[0].y);
            for i in [0, 2] {
                w.agents.x[i] = x;
                w.agents.y[i] = y;
                w.agents.target[i] = 0;
            }
            w.agents.action[0] = Action::Teach;
            w.agents.action[2] = Action::AttendSchool;
            w.execute_education(0, Action::Teach);
            for _ in 0..10 {
                w.tick += 10;
                w.education_step();
            }
            assert_eq!(w.education.attendance[2], day as u16 + 1);
        }
        assert_ne!(w.agents.traits[2] & EDUCATED, 0);
        assert_eq!(w.education.teaching_shifts[0], 10);
        assert_eq!(w.coins(), money);
        w.life_step();
        assert!(w.people[0].aspiration_done);
        w.people[2].initial_age_days = 18 * 365;
        assert!(w.teacher_eligible(2));
        w.check().unwrap();
    }
    #[test]
    fn wage_bonus_is_exact_across_small_jobs_and_never_mints_cash() {
        let mut w = family();
        w.agents.traits[0] |= EDUCATED;
        let coins = w.coins();
        let wallet = w.agents.coins[0];
        for _ in 0..10 {
            w.credit_education_bonus(0, 0, 2);
        }
        assert_eq!(w.agents.coins[0], wallet + 3);
        assert_eq!(w.coins(), coins);
        w.agents.coins[1] += w.markets[0].coins;
        w.markets[0].coins = 0;
        w.credit_education_bonus(0, 0, 100);
        assert_eq!(w.education.bonus_claims[&(0, 0)], 1500);
        assert_eq!(w.coins(), coins);
        w.check().unwrap();
    }
    #[test]
    fn educated_is_acquired_and_not_automatically_inherited() {
        let mut w = family();
        w.agents.traits[0] = EDUCATED | life::NIGHT_OWL;
        w.agents.traits[1] = EDUCATED | life::NIGHT_OWL;
        w.birth(1, 0);
        let child = w.agents.len() - 1;
        assert_eq!(w.agents.traits[child] & EDUCATED, 0);
        assert_ne!(w.agents.traits[child] & life::NIGHT_OWL, 0);
        w.check().unwrap();
    }
    #[test]
    fn rocky_farms_never_produce_and_layout_prefers_willow() {
        let mut w = family();
        let counts = w
            .wildlife
            .forests
            .iter()
            .fold([0usize; 3], |mut counts, f| {
                let m = w
                    .markets
                    .iter()
                    .enumerate()
                    .min_by(|(_, a), (_, b)| {
                        dist(f.x, f.y, a.x, a.y).total_cmp(&dist(f.x, f.y, b.x, b.y))
                    })
                    .unwrap()
                    .0;
                counts[m] += 1;
                counts
            });
        assert!(
            counts[0] > counts[1] * 2 && counts[0] > counts[2] * 2,
            "{counts:?}"
        );
        assert!(w.fertility.iter().all(|r| w.farm_ground(r.x, r.y, 75.0)));
        w.farms.push(Farm {
            market: 2,
            x: 1500.0,
            y: 1380.0,
            fertility: 1.0,
            moisture: 1.0,
            planted: 0,
            tended_days: 15,
            last_tended: 0,
            mature: false,
            harvest_remaining: 0,
            yield_per_crop: 256,
            escrow: 0,
        });
        w.tick = 20 * rest::DAY_TICKS;
        let rice = w.ledger.renewed[8];
        w.farm_step();
        assert!(!w.farms[0].mature);
        assert_eq!(w.ledger.renewed[8], rice);
        w.relocate_fertile_land();
        assert!(w.farm_ground(w.farms[0].x, w.farms[0].y, 26.0));
        w.check().unwrap();
    }
    #[test]
    fn school_bonus_claims_and_classroom_replay_exactly() {
        let mut w = family();
        w.fund_school(0);
        w.education.attendance[2] = 4;
        w.agents.traits[0] |= EDUCATED;
        w.credit_education_bonus(0, 0, 2);
        let path =
            std::env::temp_dir().join(format!("xianxia-school-{}.snapshot", std::process::id()));
        w.save(&path).unwrap();
        let mut restored = World::load(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        for _ in 0..30 {
            w.step(false);
            restored.step(true);
        }
        assert_eq!(
            postcard::to_allocvec(&w).unwrap(),
            postcard::to_allocvec(&restored).unwrap()
        );
        w.check().unwrap();
    }
}
