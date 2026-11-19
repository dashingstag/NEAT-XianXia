use super::*;
use std::collections::BTreeMap;
pub const NIGHT_OWL: u16 = 1;
pub const COIN_GRABBER: u16 = 2;
pub const FLEET: u16 = 4;
pub const HARDY: u16 = 8;
pub const CLUMSY: u16 = 16;
pub const SLOW: u16 = 32;
pub const GLUTTON: u16 = 64;
pub const SOCIALITE: u16 = 128;
pub const GREGARIOUS: u16 = 256;
pub const COMMUNITY_MINDED: u16 = 512;
const SURNAMES: [&str; 24] = [
    "Lin", "Chen", "Zhao", "Li", "Wang", "Liu", "Zhang", "Shen", "Han", "Gu", "Lu", "Su", "Bai",
    "Ye", "Mo", "Xu", "Jiang", "Qin", "Feng", "Tang", "Xiao", "He", "Song", "Yun",
];
const GIVEN: [&str; 24] = [
    "Qing", "Lan", "Wei", "Jun", "Mei", "An", "Tao", "Hua", "Xiu", "Yu", "Ren", "Yue", "Jin",
    "Ning", "Lei", "Lian", "Shan", "Rong", "Xin", "Zhen", "Min", "Bo", "Fen", "Kai",
];
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Aspiration {
    Wealth,
    Blacksmith,
    Family,
    Farmer,
    Service,
    Teacher,
    Homeowner,
    Shopkeeper,
    Artisan,
    Hunter,
    Haulier,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn full_hands_cannot_block_emergency_meals() {
        let mut w = World::new(2, 42).unwrap();
        let patch = w.patches.iter().position(|p| p.kind == 0).unwrap();
        let timber = w.patches.iter_mut().find(|p| p.kind == 1).unwrap();
        timber.stock -= 4;
        w.agents.cargo[0] = 4;
        w.agents.cargo_kind[0] = 1;
        w.agents.hunger[0] = 80.0;
        w.agents.x[0] = w.patches[patch].x;
        w.agents.y[0] = w.patches[patch].y;
        let (offers, n) = w.offer(0);
        assert_eq!(n, 1);
        assert_eq!(offers[0].action, Action::ForageMeal);
        w.agents.action[0] = Action::ForageMeal;
        w.agents.target[0] = patch as u32;
        w.agents.timer[0] = 1.0;
        w.forage_memory.commit(0, patch as u32, 0);
        w.execute_personal(0, Action::ForageMeal);
        assert_eq!(w.agents.hunger[0], 40.0);
        let learned = w.forage_memory.inputs(0, patch as u32, w.tick);
        assert_eq!(learned[0], 1.0);
        assert_eq!(w.food_memory.inputs(0, patch as u32, w.tick), [0.0; 4]);
        w.execute_personal(0, Action::ForageMeal);
        assert_eq!(w.forage_memory.inputs(0, patch as u32, w.tick), learned);
        assert_eq!(w.agents.cargo[0], 4);
        w.check().unwrap();
    }
    #[test]
    fn sixteen_day_family_and_crop_flow_survives_without_free_resources() {
        let mut w = World::new(64, 42).unwrap();
        w.prepare_family_demo().unwrap();
        for _ in 0..48_000 {
            w.step(false);
        }
        assert_eq!(
            w.stats.deaths,
            0,
            "{:?}",
            w.agents
                .alive
                .iter()
                .enumerate()
                .filter(|(_, a)| !**a)
                .map(|(i, _)| (
                    i,
                    w.people[i].age_days(w.tick),
                    w.agents.hunger[i],
                    w.agents.energy[i],
                    w.agents.action[i]
                ))
                .collect::<Vec<_>>()
        );
        assert_eq!(w.stats.births, 1);
        assert!(w.stats.rice_harvested >= 256);
        assert_eq!(w.people[64].surname, w.people[0].surname);
        w.check().unwrap();
        w.validate_people().unwrap();
    }
    #[test]
    fn goals_are_bounded_expire_and_reward_actual_progress() {
        let mut w = World::new(2, 42).unwrap();
        w.people[0].goals.clear();
        for k in [
            GoalKind::Save,
            GoalKind::Job,
            GoalKind::Craft,
            GoalKind::Farm,
            GoalKind::Marry,
            GoalKind::Tool,
            GoalKind::Trade,
        ] {
            w.add_goal(0, k, 5);
        }
        assert_eq!(w.people[0].goals.len(), 5);
        w.people[0].goals = vec![Goal {
            kind: GoalKind::Trade,
            target: 1,
            created: 0,
            expires: 100,
        }];
        let happiness = w.people[0].happiness;
        w.people[0].trades = 1;
        w.tick = 10;
        w.life_step();
        assert_eq!(w.people[0].happiness, happiness + 8.0);
        w.people[0].goals = vec![Goal {
            kind: GoalKind::Save,
            target: 1000,
            created: 10,
            expires: 20,
        }];
        w.tick = 20;
        w.life_step();
        assert_eq!(w.people[0].happiness, happiness + 6.0);
    }
    #[test]
    fn only_compatible_willing_adults_can_marry() {
        let mut w = World::new(2, 42).unwrap();
        w.markets[0].housing_blocks = 1;
        for i in 0..2 {
            w.people[i].initial_age_days = 25 * 365;
            w.people[i].gender = i as u8;
            w.people[i].aspiration = Aspiration::Family;
            w.people[i].goals = vec![Goal {
                kind: GoalKind::Marry,
                target: 1,
                created: 0,
                expires: 1000,
            }];
            w.agents.x[i] = w.markets[0].x;
            w.agents.y[i] = w.markets[0].y;
        }
        assert!(w.compatible(0, 1));
        w.people[1].initial_age_days = 17 * 365;
        assert!(!w.compatible(0, 1));
        w.people[1].initial_age_days = 25 * 365;
        w.people[1].gender = 0;
        assert!(!w.compatible(0, 1));
        w.people[1].gender = 1;
        w.agents.action[0] = Action::Court;
        w.agents.target[0] = 1;
        w.agents.timer[0] = 3.0;
        w.commitments.commit(0, Action::Court, 1, 0);
        w.execute_personal(0, Action::Court);
        assert_eq!(w.action_memory_inputs(0, Action::Court, 1).unwrap()[0], 1.0);
        assert_eq!(w.people[0].spouse, 1);
        assert_eq!(w.people[1].spouse, 0);
        assert_eq!(w.stats.marriages, 1);
        w.validate_people().unwrap();
    }
    #[test]
    fn third_trimester_forces_return_without_teleporting() {
        let mut w = World::new(2, 42).unwrap();
        w.prepare_family_demo().unwrap();
        w.people[1].pregnancy = Some(Pregnancy {
            father: 0,
            due: 80 * rest::DAY_TICKS,
        });
        w.agents.action[1] = Action::Gather;
        w.agents.x[1] = 1200.0;
        w.agents.y[1] = 200.0;
        let p = (w.agents.x[1], w.agents.y[1]);
        w.domestic_rules();
        assert_eq!(w.agents.action[1], Action::CareFamily);
        assert_eq!((w.agents.x[1], w.agents.y[1]), p);
        let (offers, n) = w.offer(1);
        assert!(offers[..n]
            .iter()
            .all(|c| matches!(c.action, Action::CareFamily | Action::Rest)));
        w.people[1].pregnancy = Some(Pregnancy {
            father: 0,
            due: 200 * rest::DAY_TICKS,
        });
        assert!(!w.must_stay_home(1));
    }
    #[test]
    fn birth_inherits_fathers_surname_brain_and_traits_without_creating_goods() {
        let mut w = World::new(2, 42).unwrap();
        w.prepare_family_demo().unwrap();
        w.people[1].pregnancy.as_mut().unwrap().due = 10;
        let stocks = w.stocks();
        let money = w.coins();
        w.tick = 10;
        w.life_step();
        let child = 2;
        assert_eq!(w.agents.len(), 3);
        assert_eq!(w.people[child].surname, w.people[0].surname);
        assert_eq!(w.people[child].father, 0);
        assert_eq!(w.people[child].mother, 1);
        assert_eq!(w.people[child].age_days(w.tick), 0);
        assert!(!w.is_adult(child));
        assert_eq!(w.agents.coins[child], 0);
        assert!(w.genomes[child].generation > 0);
        assert!(w.genomes[child].compile().is_ok());
        assert!(w.agents.traits[child] & !(w.agents.traits[0] | w.agents.traits[1]) == 0);
        assert_eq!(w.stocks(), stocks);
        assert_eq!(w.coins(), money);
        w.check().unwrap();
        w.validate_people().unwrap();
        let before = w.ledger.consumed[0];
        w.agents.hunger[child] = 35.0;
        w.feed_dependents(child);
        assert_eq!(w.ledger.consumed[0], before + 1);
        w.check().unwrap();
    }
    #[test]
    fn child_becomes_adult_after_checkpointed_age_boundary() {
        let mut w = World::new(2, 42).unwrap();
        w.prepare_family_demo().unwrap();
        w.birth(1, 0);
        let child = 2;
        assert_eq!(w.people[child].age_days(w.tick), 0);
        w.tick = 18 * 365 * rest::DAY_TICKS - 1;
        assert_eq!(w.people[child].age_days(w.tick), 18 * 365 - 1);
        assert!(!w.is_adult(child));
        let (offers, count) = w.offer(child);
        assert!(offers[..count]
            .iter()
            .all(|candidate| matches!(candidate.action, Action::Rest | Action::CareFamily)));

        let path =
            std::env::temp_dir().join(format!("xianxia-child-age-{}.snapshot", std::process::id()));
        w.save(&path).unwrap();
        let mut restored = World::load(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        assert_eq!(restored.people[child].age_days(restored.tick), 18 * 365 - 1);
        restored.step(false);
        assert_eq!(restored.people[child].age_days(restored.tick), 18 * 365);
        assert!(restored.is_adult(child));
        let (offers, count) = restored.offer(child);
        assert!(offers[..count]
            .iter()
            .any(|candidate| !matches!(candidate.action, Action::Rest | Action::CareFamily)));
    }
    #[test]
    fn relatives_and_children_cannot_take_adult_relationships_or_jobs() {
        let mut w = World::new(2, 42).unwrap();
        w.prepare_family_demo().unwrap();
        w.birth(1, 0);
        let (offers, n) = w.offer(2);
        assert!(offers[..n]
            .iter()
            .all(|c| matches!(c.action, Action::Rest | Action::CareFamily)));
        w.people[2].initial_age_days = 20 * 365;
        w.people[0].spouse = NONE;
        w.people[1].spouse = NONE;
        w.people[2].gender = 1;
        w.people[2].goals = vec![Goal {
            kind: GoalKind::Marry,
            target: 1,
            created: 0,
            expires: 1000,
        }];
        w.people[0].goals = w.people[2].goals.clone();
        assert!(!w.compatible(0, 2));
    }
    #[test]
    fn traits_change_actual_travel_and_coin_preference() {
        let mut w = World::new(1, 42).unwrap();
        w.agents.traits[0] = NIGHT_OWL;
        let day = w.actor_speed(0, w.agents.x[0], w.agents.y[0]);
        w.set_hour(20.0).unwrap();
        assert!((w.actor_speed(0, w.agents.x[0], w.agents.y[0]) - day).abs() < 0.001);
        w.agents.traits[0] = 0;
        assert!((w.actor_speed(0, w.agents.x[0], w.agents.y[0]) / day - 0.65).abs() < 0.001);
        w.agents.traits[0] = COIN_GRABBER;
        let mut a = [0.0; INPUTS];
        w.personal_inputs(0, Action::Barter, 2, &mut a);
        assert_eq!(a[36], -1.0);
        let lower = w.brains[0].evaluate(&a);
        w.agents.traits[0] = 0;
        w.personal_inputs(0, Action::Barter, 2, &mut a);
        assert!(w.brains[0].evaluate(&a) > lower);
        w.agents.traits[0] = CLUMSY;
        assert!((w.work_factor(0, Action::Gather, 1) - 0.8).abs() < 0.001);
    }
    #[test]
    fn aspiration_and_goal_are_contextual_brain_inputs() {
        let mut w = World::new(1, 42).unwrap();
        w.people[0].aspiration = Aspiration::Blacksmith;
        w.people[0].goals = vec![Goal {
            kind: GoalKind::Craft,
            target: 1,
            created: 0,
            expires: 100,
        }];
        let mut craft = [0.0; INPUTS];
        w.personal_inputs(0, Action::CraftTool, 9, &mut craft);
        let mut rest = [0.0; INPUTS];
        w.personal_inputs(0, Action::Rest, 0, &mut rest);
        assert_eq!(craft[29], 1.0);
        assert_eq!(craft[33], 1.0);
        assert_eq!(rest[29], 0.0);
        assert_eq!(rest[33], 0.0);
    }
    #[test]
    fn family_and_goals_replay_with_exact_checkpoint_state() {
        let mut a = World::new(12, 42).unwrap();
        a.prepare_family_demo().unwrap();
        a.people[1].pregnancy.as_mut().unwrap().due = 50;
        let path =
            std::env::temp_dir().join(format!("xianxia-family-{}.snapshot", std::process::id()));
        a.save(&path).unwrap();
        let mut b = World::load(&path).unwrap();
        for _ in 0..100 {
            a.step(false);
            b.step(true);
        }
        assert_eq!(a.people, b.people);
        assert_eq!(a.agents, b.agents);
        assert_eq!(a.farms, b.farms);
        assert_eq!(a.summary(), b.summary());
        a.check().unwrap();
        a.validate_people().unwrap();
        std::fs::remove_file(path).unwrap();
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum GoalKind {
    Save,
    Job,
    Craft,
    Farm,
    Marry,
    Rest,
    Eat,
    Trade,
    Tool,
    Child,
    Harvest,
    Teach,
    OwnHome,
    LeaseShop,
    Produce,
    Transport,
    Hunt,
    BusinessLoan,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Goal {
    pub kind: GoalKind,
    pub target: u64,
    pub expires: u64,
    pub created: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Pregnancy {
    pub father: u32,
    pub due: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Person {
    pub given: u16,
    pub surname: u16,
    pub gender: u8,
    pub initial_age_days: u32,
    pub born: u64,
    pub father: u32,
    pub mother: u32,
    pub spouse: u32,
    pub children: Vec<u32>,
    pub pregnancy: Option<Pregnancy>,
    pub last_birth: u64,
    pub aspiration: Aspiration,
    pub aspiration_done: bool,
    pub goals: Vec<Goal>,
    pub happiness: f32,
    pub meals: u64,
    pub trades: u64,
    pub jobs: u64,
    pub crafts: u64,
    pub farm_shifts: u64,
    pub harvested: u64,
    pub rest_cycles: u64,
}
impl Person {
    pub fn age_days(&self, tick: u64) -> u32 {
        self.initial_age_days
            .saturating_add((tick.saturating_sub(self.born) / rest::DAY_TICKS) as u32)
    }
    fn seeded(id: usize, seed: u64, tick: u64, child: bool) -> Self {
        let mut r = Rng(seed ^ (id as u64 + 1).wrapping_mul(0x517cc1b727220a95));
        Self {
            given: r.index(576) as u16,
            surname: r.index(24) as u16,
            gender: r.index(2) as u8,
            initial_age_days: if child {
                0
            } else {
                (18 + r.index(38)) as u32 * 365 + r.index(365) as u32
            },
            born: tick,
            father: NONE,
            mother: NONE,
            spouse: NONE,
            children: vec![],
            pregnancy: None,
            last_birth: 0,
            aspiration: [
                Aspiration::Wealth,
                Aspiration::Blacksmith,
                Aspiration::Family,
                Aspiration::Farmer,
                Aspiration::Service,
                Aspiration::Teacher,
                Aspiration::Homeowner,
                Aspiration::Shopkeeper,
                Aspiration::Artisan,
                Aspiration::Hunter,
                Aspiration::Haulier,
            ][r.index(11)],
            aspiration_done: false,
            goals: vec![],
            happiness: 55.0 + r.signed() * 10.0,
            meals: 0,
            trades: 0,
            jobs: 0,
            crafts: 0,
            farm_shifts: 0,
            harvested: 0,
            rest_cycles: 0,
        }
    }
    pub fn name(&self) -> String {
        format!(
            "{} {}{}",
            SURNAMES[self.surname as usize % 24],
            GIVEN[self.given as usize % 24],
            GIVEN[self.given as usize / 24 % 24]
        )
    }
}
impl World {
    pub(super) fn init_people(&mut self) {
        while self.people.len() < self.agents.len() {
            let i = self.people.len();
            self.people
                .push(Person::seeded(i, self.seed, self.tick, false));
            let mut r = Rng(self.seed ^ i as u64 ^ 0x53524f4c45);
            self.agents.traits[i] = [NIGHT_OWL, COIN_GRABBER, FLEET, HARDY][r.index(4)];
            if r.index(3) == 0 {
                self.agents.traits[i] |= [CLUMSY, SLOW, GLUTTON][r.index(3)];
            }
            if r.index(2) == 0 {
                self.agents.traits[i] |= [SOCIALITE, GREGARIOUS, COMMUNITY_MINDED][r.index(3)];
            }
            self.matches.push(NONE);
            self.refresh_goals(i);
        }
    }
    pub(super) fn is_adult(&self, i: usize) -> bool {
        self.people
            .get(i)
            .is_none_or(|p| p.age_days(self.tick) >= 18 * 365)
    }
    pub(super) fn must_stay_home(&self, i: usize) -> bool {
        self.people.get(i).is_some_and(|p| {
            p.pregnancy
                .as_ref()
                .is_some_and(|g| g.due.saturating_sub(self.tick) <= 92 * rest::DAY_TICKS)
                || p.last_birth > 0 && self.tick.saturating_sub(p.last_birth) < 2 * rest::DAY_TICKS
        })
    }
    pub(super) fn domestic_rules(&mut self) {
        for i in 0..self.agents.len() {
            if !self.agents.alive[i] || self.is_adult(i) && !self.must_stay_home(i) {
                continue;
            }
            if self.agents.contract[i] != NONE {
                self.cancel_job(self.agents.contract[i] as usize);
            }
            let slot = self.agents.convoy[i] as usize;
            if let Some(Some(c)) = self.convoys.get_mut(slot) {
                c.members.retain(|&j| j as usize != i);
            }
            self.agents.convoy[i] = NONE;
            let resting = self.agents.action[i] == Action::Rest
                && self
                    .shelters
                    .get(self.agents.target[i] as usize)
                    .is_some_and(|s| s.settlement.is_some());
            if !self.is_adult(i) || self.must_stay_home(i) {
                let m = &self.markets[self.agents.home[i] as usize];
                if dist(self.agents.x[i], self.agents.y[i], m.x, m.y)
                    <= rest::settlement_radius(m.housing_blocks)
                {
                    self.feed_dependents(i);
                }
            }
            let schooling = self.agents.action[i] == Action::AttendSchool
                && self.best_school(i, Action::AttendSchool)
                    == Some(self.agents.target[i] as usize);
            if !resting && !schooling {
                self.agents.action[i] = Action::CareFamily;
                self.agents.target[i] = self.agents.home[i];
            }
        }
    }
    pub(super) fn family_offers(&self, i: usize) -> ([Candidate; CANDIDATES], usize) {
        let mut out = [Candidate::default(); CANDIDATES];
        let home = self.agents.home[i];
        let mut n = 0;
        if let Some(site) = self.best_rest_site(i) {
            if self.shelters[site].settlement.is_some() {
                out[n] = Candidate {
                    action: Action::Rest,
                    target: site as u32,
                    kind: 0,
                    inputs: [0.0; INPUTS],
                    score: 1.0,
                };
                n += 1;
            }
        }
        if let Some(id) = self.best_school(i, Action::AttendSchool) {
            let mut inputs = [0.0; INPUTS];
            inputs[0] = 1.0;
            inputs[1] = 0.8;
            inputs[3] = dist(
                self.agents.x[i],
                self.agents.y[i],
                self.education.schools[id].x,
                self.education.schools[id].y,
            ) / 1000.0;
            out[n] = Candidate {
                action: Action::AttendSchool,
                target: id as u32,
                kind: 0,
                inputs,
                score: self.brains[i].evaluate(&inputs),
            };
            n += 1;
        }
        out[n] = Candidate {
            action: Action::CareFamily,
            target: home,
            kind: 0,
            inputs: [0.0; INPUTS],
            score: if n == 0 { 1.0 } else { 0.5 },
        };
        (out, n + 1)
    }
    fn goal_value(&self, i: usize, k: GoalKind) -> u64 {
        let p = &self.people[i];
        match k {
            GoalKind::BusinessLoan => self.banking.approved.get(&(i as u32)).copied().unwrap_or(0),
            GoalKind::OwnHome => u64::from(self.owns_urban_home(i)),
            GoalKind::LeaseShop => u64::from(self.owns_business(i)),
            GoalKind::Produce => self.urban_progress(i).made,
            GoalKind::Transport => self.urban_progress(i).hauled,
            GoalKind::Hunt => self.urban_progress(i).hunted,
            GoalKind::Save => self.agents.coins[i],
            GoalKind::Job => p.jobs,
            GoalKind::Craft => p.crafts,
            GoalKind::Farm => p.farm_shifts,
            GoalKind::Marry => u64::from(p.spouse != NONE),
            GoalKind::Rest => p.rest_cycles,
            GoalKind::Eat => p.meals,
            GoalKind::Trade => p.trades,
            GoalKind::Tool => self.agents.tools[i].iter().map(|&n| n as u64).sum(),
            GoalKind::Child => p.children.len() as u64,
            GoalKind::Harvest => p.harvested,
            GoalKind::Teach => self.education.teaching_shifts.get(i).copied().unwrap_or(0),
        }
    }
    fn add_goal(&mut self, i: usize, kind: GoalKind, days: u64) {
        if self.people[i].goals.len() >= 5
            || self.people[i].goals.iter().any(|g| g.kind == kind)
            || kind == GoalKind::Marry && self.people[i].spouse != NONE
        {
            return;
        }
        let target = self.goal_value(i, kind)
            + match kind {
                GoalKind::Save => 20,
                GoalKind::Harvest => 16,
                GoalKind::Trade => 3,
                _ => 1,
            };
        self.people[i].goals.push(Goal {
            kind,
            target,
            expires: self.tick + days * rest::DAY_TICKS,
            created: self.tick,
        });
    }
    fn refresh_goals(&mut self, i: usize) {
        if !self.is_adult(i) {
            return;
        }
        let aspiration = self.people[i].aspiration;
        let guided = match aspiration {
            Aspiration::Wealth => GoalKind::Save,
            Aspiration::Blacksmith => GoalKind::Craft,
            Aspiration::Family => {
                if self.people[i].spouse == NONE {
                    GoalKind::Marry
                } else {
                    GoalKind::Child
                }
            }
            Aspiration::Farmer => GoalKind::Farm,
            Aspiration::Service => GoalKind::Job,
            Aspiration::Teacher => GoalKind::Teach,
            Aspiration::Homeowner => GoalKind::OwnHome,
            Aspiration::Shopkeeper => {
                if self.can_seek_business_loan(i) {
                    GoalKind::BusinessLoan
                } else {
                    GoalKind::LeaseShop
                }
            }
            Aspiration::Artisan => GoalKind::Produce,
            Aspiration::Hunter => GoalKind::Hunt,
            Aspiration::Haulier => GoalKind::Transport,
        };
        self.add_goal(
            i,
            guided,
            if guided == GoalKind::Child {
                365
            } else if guided == GoalKind::BusinessLoan {
                14
            } else {
                30
            },
        );
        if self.people[i].goals.len() < 3 {
            let choice = ((self.seed ^ i as u64 ^ self.tick / 50) % 7) as usize;
            self.add_goal(
                i,
                [
                    GoalKind::Eat,
                    GoalKind::Rest,
                    GoalKind::Trade,
                    GoalKind::Job,
                    GoalKind::Tool,
                    GoalKind::Marry,
                    GoalKind::Save,
                ][choice],
                if choice < 2 { 2 } else { 10 },
            );
        }
        let home = self.agents.home[i] as usize;
        if self
            .councils
            .get(home)
            .is_some_and(|c| c.vacancies > 0 || c.builder_open)
        {
            self.add_goal(i, GoalKind::Job, 5);
        }
        if self.farms.iter().any(|f| f.market as usize == home) {
            self.add_goal(i, GoalKind::Harvest, 20);
        }
    }
    fn compatible(&self, i: usize, j: usize) -> bool {
        if i == j || !self.agents.alive[j] || !self.is_adult(i) || !self.is_adult(j) {
            return false;
        }
        let (a, b) = (&self.people[i], &self.people[j]);
        if a.gender == b.gender
            || a.spouse != NONE
            || b.spouse != NONE
            || a.happiness < 30.0
            || b.happiness < 30.0
        {
            return false;
        }
        if !a.goals.iter().any(|g| g.kind == GoalKind::Marry)
            || !b.goals.iter().any(|g| g.kind == GoalKind::Marry)
        {
            return false;
        }
        if a.father == j as u32
            || a.mother == j as u32
            || b.father == i as u32
            || b.mother == i as u32
            || a.father != NONE && a.father == b.father
            || a.mother != NONE && a.mother == b.mother
        {
            return false;
        }
        a.aspiration == b.aspiration
            || a.goals
                .iter()
                .any(|g| g.kind != GoalKind::Marry && b.goals.iter().any(|h| h.kind == g.kind))
    }
    pub(super) fn marriage_target(&self, i: usize) -> Option<usize> {
        let j = *self.matches.get(i)? as usize;
        (j < self.people.len() && self.compatible(i, j)).then_some(j)
    }
    fn update_matches(&mut self) {
        let mut bins: BTreeMap<(i32, i32), Vec<usize>> = BTreeMap::new();
        for i in 0..self.people.len() {
            self.matches[i] = NONE;
            if self.agents.alive[i]
                && self.is_adult(i)
                && self.people[i].spouse == NONE
                && self.people[i]
                    .goals
                    .iter()
                    .any(|g| g.kind == GoalKind::Marry)
            {
                bins.entry((
                    (self.agents.x[i] / 128.0) as i32,
                    (self.agents.y[i] / 128.0) as i32,
                ))
                .or_default()
                .push(i);
            }
        }
        for ids in bins.values() {
            for &i in ids {
                let cell = (
                    (self.agents.x[i] / 128.0) as i32,
                    (self.agents.y[i] / 128.0) as i32,
                );
                let mut best = (f32::INFINITY, NONE);
                for dx in -1..=1 {
                    for dy in -1..=1 {
                        for &j in bins
                            .get(&(cell.0 + dx, cell.1 + dy))
                            .into_iter()
                            .flatten()
                            .take(16)
                        {
                            if self.compatible(i, j) {
                                let d = dist(
                                    self.agents.x[i],
                                    self.agents.y[i],
                                    self.agents.x[j],
                                    self.agents.y[j],
                                );
                                if d < 180.0 && d < best.0 {
                                    best = (d, j as u32);
                                }
                            }
                        }
                    }
                }
                self.matches[i] = best.1;
            }
        }
    }
    pub(super) fn personal_inputs(
        &self,
        i: usize,
        action: Action,
        kind: usize,
        x: &mut [f32; INPUTS],
    ) {
        let p = &self.people[i];
        let paid = matches!(
            action,
            Action::Deliver
                | Action::Mint
                | Action::Trade
                | Action::Build
                | Action::Gravedigger
                | Action::BuildGraveyard
                | Action::CraftTool
                | Action::BuildBank
                | Action::BuildSchool
                | Action::Teach
                | Action::BuildTavern
                | Action::Farm
                | Action::UrbanWork
                | Action::OperateBusiness
                | Action::Hunt
        );
        x[28] = if p.aspiration == Aspiration::Wealth && paid {
            1.0
        } else {
            0.0
        };
        x[29] = if p.aspiration == Aspiration::Blacksmith
            && (action == Action::CraftTool
                || action == Action::Gather && kind == 7
                || action == Action::Deliver && kind == 7)
        {
            1.0
        } else {
            0.0
        };
        x[30] = if p.aspiration == Aspiration::Family
            && matches!(action, Action::Court | Action::CareFamily)
        {
            1.0
        } else {
            0.0
        };
        x[31] = if p.aspiration == Aspiration::Farmer
            && matches!(action, Action::Farm | Action::HarvestRice)
        {
            1.0
        } else {
            0.0
        };
        x[32] = if p.aspiration == Aspiration::Teacher && action == Action::Teach {
            1.0
        } else if p.aspiration == Aspiration::Service
            && matches!(
                action,
                Action::Gravedigger
                    | Action::BuildGraveyard
                    | Action::Farm
                    | Action::Build
                    | Action::BuildBank
                    | Action::BuildSchool
                    | Action::Teach
                    | Action::BuildTavern
            )
        {
            1.0
        } else {
            0.0
        };
        for g in &p.goals {
            let aligns = match g.kind {
                GoalKind::BusinessLoan => action == Action::BorrowBusiness,
                GoalKind::Save => paid,
                GoalKind::Job => matches!(
                    action,
                    Action::Gravedigger
                        | Action::BuildGraveyard
                        | Action::Farm
                        | Action::Build
                        | Action::BuildBank
                        | Action::BuildSchool
                        | Action::Teach
                        | Action::BuildTavern
                ),
                GoalKind::Craft => action == Action::CraftTool,
                GoalKind::Farm => action == Action::Farm,
                GoalKind::Marry => action == Action::Court,
                GoalKind::Rest => action == Action::Rest,
                GoalKind::Eat => {
                    matches!(
                        action,
                        Action::BuyFood | Action::HarvestRice | Action::ForageMeal
                    ) || action == Action::Gather && [0, 3, 8].contains(&kind)
                }
                GoalKind::Trade => {
                    matches!(action, Action::Trade | Action::Deliver | Action::Barter)
                }
                GoalKind::Tool => matches!(action, Action::BuyTool | Action::EquipTool),
                GoalKind::Child => action == Action::CareFamily,
                GoalKind::Harvest => action == Action::HarvestRice,
                GoalKind::Teach => action == Action::Teach,
                GoalKind::OwnHome => action == Action::BuyHome,
                GoalKind::LeaseShop => action == Action::LeaseBusiness,
                GoalKind::Produce => action == Action::OperateBusiness,
                GoalKind::Transport => action == Action::UrbanWork,
                GoalKind::Hunt => action == Action::Hunt,
            };
            if aligns {
                x[33] = 1.0;
                x[34] = x[34].max(
                    1.0 - g.expires.saturating_sub(self.tick) as f32
                        / g.expires.saturating_sub(g.created).max(1) as f32,
                );
            }
        }
        if action == Action::Socialize {
            x[0] = (1.0 - self.social[i] / 100.0)
                * if self.agents.traits[i] & (SOCIALITE | GREGARIOUS | COMMUNITY_MINDED) != 0 {
                    0.9
                } else {
                    0.15
                };
            x[1] = 0.25
                + if self.agents.traits[i] & SOCIALITE != 0 {
                    0.5
                } else {
                    0.0
                };
        }
        if self.agents.traits[i] & education::EDUCATED != 0
            && matches!(
                action,
                Action::Build
                    | Action::BuildBank
                    | Action::BuildSchool
                    | Action::BuildTavern
                    | Action::BuildGraveyard
                    | Action::Gravedigger
                    | Action::Farm
                    | Action::Teach
                    | Action::CraftTool
            )
        {
            x[1] *= 1.15;
        }
        if self.urban_aspiration_aligns(i, action) {
            x[0] += 0.65;
            x[1] += 0.25;
        }
        x[35] = p.happiness / 100.0;
        x[36] = if self.agents.traits[i] & COIN_GRABBER != 0 {
            if paid {
                1.0
            } else if matches!(action, Action::Barter | Action::WorkForFood) {
                -1.0
            } else {
                0.0
            }
        } else {
            0.0
        };
        x[37] = if self.agents.traits[i] & NIGHT_OWL != 0 {
            1.0
        } else {
            0.0
        };
        x[38] = (self.work_factor(i, action, kind) - 1.0).clamp(-1.0, 1.0);
        x[39] = if self.is_adult(i) { 1.0 } else { 0.0 };
        x[40] = if kind == 7 { 1.0 } else { 0.0 };
        x[41] = if kind == 8 { 1.0 } else { 0.0 };
    }
    pub(super) fn actor_speed(&self, i: usize, x: f32, y: f32) -> f32 {
        let traits = self.agents.traits[i];
        let mut speed = self.speed_at(x, y);
        if self.is_night() && traits & NIGHT_OWL != 0 {
            speed /= 0.65;
        }
        if traits & FLEET != 0 {
            speed *= 1.15;
        }
        if traits & SLOW != 0 {
            speed *= 0.8;
        }
        if !self.is_adult(i) {
            speed *= 0.55;
        }
        if self.urban.owned.get(&(i as u32, 2)).copied().unwrap_or(0) > 0 {
            speed *= 1.05;
        }
        speed
    }
    pub(super) fn closest_food_patch(&self, i: usize) -> Option<usize> {
        self.patches
            .iter()
            .enumerate()
            .filter(|(_, p)| [0, 3].contains(&p.kind) && p.stock > 0)
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
    pub(super) fn urgent_family_care(&self, i: usize) -> bool {
        if !self.is_adult(i) || self.must_stay_home(i) {
            return false;
        }
        let p = &self.people[i];
        p.spouse != NONE
            && self.agents.alive[p.spouse as usize]
            && self.must_stay_home(p.spouse as usize)
            && self.agents.hunger[p.spouse as usize] >= 65.0
            || p.children.iter().any(|&j| {
                self.agents.alive[j as usize]
                    && !self.is_adult(j as usize)
                    && self.agents.hunger[j as usize] >= 65.0
            })
    }
    fn family_needs_food(&self, i: usize) -> bool {
        let p = &self.people[i];
        let spouse = p.spouse;
        spouse != NONE
            && self.agents.alive[spouse as usize]
            && self.must_stay_home(spouse as usize)
            && self.agents.hunger[spouse as usize] > 25.0
            || p.children.iter().any(|&id| {
                self.agents.alive[id as usize]
                    && !self.is_adult(id as usize)
                    && self.agents.hunger[id as usize] > 25.0
            })
    }
    fn gathering_family_food(&self, i: usize) -> bool {
        self.is_adult(i)
            && !self.must_stay_home(i)
            && self.family_needs_food(i)
            && self.agents.berries[i] + self.agents.fish[i] + self.agents.rice[i] < 3
            && self.agents.room(i) > 0
    }
    pub(super) fn personal_destination(&self, i: usize) -> Option<(f32, f32)> {
        let a = &self.agents;
        if self.is_urban_action(a.action[i]) {
            return self.urban_destination(i);
        }
        match a.action[i] {
            Action::BuildSchool | Action::Teach | Action::AttendSchool => self
                .education
                .schools
                .get(a.target[i] as usize)
                .map(|s| (s.x, s.y)),
            Action::Socialize | Action::BuildTavern => {
                self.taverns.get(a.target[i] as usize).map(|t| (t.x, t.y))
            }
            Action::ForageMeal => self.patches.get(a.target[i] as usize).map(|p| (p.x, p.y)),
            Action::CareFamily if self.gathering_family_food(i) => self
                .closest_food_patch(i)
                .map(|id| (self.patches[id].x, self.patches[id].y)),
            Action::Court => {
                let j = a.target[i] as usize;
                self.agents.x.get(j).map(|&x| (x, self.agents.y[j]))
            }
            Action::Farm | Action::HarvestRice => {
                self.farms.get(a.target[i] as usize).map(|f| (f.x, f.y))
            }
            Action::EquipTool => Some((a.x[i], a.y[i])),
            _ => self.markets.get(a.target[i] as usize).map(|m| (m.x, m.y)),
        }
    }
    pub(super) fn execute_personal(&mut self, i: usize, action: Action) {
        if action == Action::CareFamily && self.urgent_family_care(i) {
            self.pause_urban_job(i);
            if self.agents.contract[i] != NONE {
                self.cancel_job(self.agents.contract[i] as usize);
            }
            self.agents.action[i] = Action::CareFamily;
            self.agents.target[i] = self.agents.home[i];
        }
        if self.is_urban_action(action) {
            self.execute_urban(i, action);
            return;
        }
        let Some((x, y)) = self.personal_destination(i) else {
            return;
        };
        let d = dist(self.agents.x[i], self.agents.y[i], x, y);
        if d > 5.0 {
            let step = (self.actor_speed(i, self.agents.x[i], self.agents.y[i]) * DT).min(d - 4.0);
            self.agents.x[i] += (x - self.agents.x[i]) / d * step;
            self.agents.y[i] += (y - self.agents.y[i]) / d * step;
            self.agents.drain_energy(i, DT * 0.07);
            return;
        }
        match action {
            Action::BuildSchool | Action::Teach | Action::AttendSchool => {
                self.execute_education(i, action)
            }
            Action::Socialize | Action::BuildTavern => self.execute_social(i, action),
            Action::ForageMeal => {
                let id = self.agents.target[i] as usize;
                let Some(patch) = self.patches.get(id) else {
                    self.forage_memory.cancel(i);
                    return;
                };
                let kind = patch.kind;
                if patch.stock == 0 {
                    self.forage_memory
                        .resolve(i, id as u32, kind, self.tick, false);
                    return;
                }
                self.agents.timer[i] += DT;
                if self.agents.timer[i] < 1.0 {
                    return;
                }
                if ![0, 3].contains(&kind) {
                    self.forage_memory.cancel(i);
                    return;
                }
                self.patches[id].stock -= 1;
                self.forage_memory
                    .resolve(i, id as u32, kind, self.tick, true);
                self.ledger.consumed[kind] += 1;
                self.people[i].meals += 1;
                self.agents.hunger[i] = (self.agents.hunger[i] - 40.0).max(0.0);
                self.agents.timer[i] = 0.0;
            }
            Action::StoreCargo => {
                let m = self.agents.target[i] as usize;
                if m < self.markets.len() && self.agents.cargo[i] > 0 {
                    self.markets[m].add(
                        self.agents.cargo_kind[i] as usize,
                        self.agents.cargo[i] as u64,
                        self.agents.origin[i],
                    );
                    self.agents.cargo[i] = 0;
                    self.agents.action[i] = Action::Wait;
                }
            }
            Action::Court => {
                let j = self.agents.target[i] as usize;
                if !self.compatible(i, j) {
                    return;
                }
                self.agents.timer[i] += DT;
                if self.agents.timer[i] < 3.0 {
                    return;
                }
                let home = self.agents.home[i] as usize;
                if self.markets[home].housing_blocks == 0 {
                    return;
                }
                self.people[i].spouse = j as u32;
                self.people[j].spouse = i as u32;
                self.agents.home[j] = home as u32;
                self.people[i].happiness = (self.people[i].happiness + 12.0).min(100.0);
                self.people[j].happiness = (self.people[j].happiness + 12.0).min(100.0);
                self.stats.marriages += 1;
                self.commitments
                    .resolve(i, Action::Court, j as u32, 0, self.tick, true);
                self.agents.action[i] = Action::Wait;
            }
            Action::CareFamily => {
                if self.gathering_family_food(i) {
                    if let Some(id) = self.closest_food_patch(i) {
                        self.agents.timer[i] += DT;
                        if self.agents.timer[i] >= 1.0
                            && self.patches[id].stock > 0
                            && self.agents.room(i) >= WEIGHTS[self.patches[id].kind]
                        {
                            self.patches[id].stock -= 1;
                            if self.patches[id].kind == 0 {
                                self.agents.berries[i] += 1
                            } else {
                                self.agents.fish[i] += 1
                            }
                            self.agents.timer[i] = 0.0;
                            self.commitments.resolve(
                                i,
                                Action::CareFamily,
                                self.agents.target[i],
                                0,
                                self.tick,
                                true,
                            );
                        }
                    }
                } else {
                    let meals = self.people[i].meals;
                    self.feed_dependents(i);
                    if self.people[i].meals > meals {
                        self.commitments.resolve(
                            i,
                            Action::CareFamily,
                            self.agents.target[i],
                            0,
                            self.tick,
                            true,
                        );
                    }
                }
            }
            Action::CraftTool | Action::BuyTool | Action::EquipTool => {
                self.execute_industry(i, action)
            }
            Action::Farm | Action::HarvestRice => self.execute_farm(i, action),
            _ => {}
        }
    }
    fn feed_dependents(&mut self, i: usize) {
        if self.is_adult(i) && !self.must_stay_home(i) {
            return;
        }
        if self.agents.berries[i] + self.agents.fish[i] + self.agents.rice[i] > 0 {
            return;
        }
        if self.agents.hunger[i] < 30.0 {
            return;
        }
        let home = self.agents.home[i] as usize;
        for parent in [
            self.people[i].spouse,
            self.people[i].mother,
            self.people[i].father,
        ] {
            if parent == NONE || !self.agents.alive[parent as usize] {
                continue;
            }
            let p = parent as usize;
            let settlement = &self.markets[home];
            if self.agents.home[p] as usize != home
                || dist(
                    self.agents.x[p],
                    self.agents.y[p],
                    settlement.x,
                    settlement.y,
                ) > rest::settlement_radius(settlement.housing_blocks)
            {
                continue;
            }
            let kind = if self.agents.berries[p] > 0 {
                self.agents.berries[p] -= 1;
                Some(0)
            } else if self.agents.fish[p] > 0 {
                self.agents.fish[p] -= 1;
                Some(3)
            } else if self.agents.rice[p] > 0 {
                self.agents.rice[p] -= 1;
                Some(8)
            } else if self.agents.cargo[p] > 0 && [0, 3, 8].contains(&self.agents.cargo_kind[p]) {
                self.agents.cargo[p] -= 1;
                Some(self.agents.cargo_kind[p] as usize)
            } else {
                None
            };
            if let Some(k) = kind {
                self.ledger.consumed[k] += 1;
                self.agents.hunger[i] = (self.agents.hunger[i] - 40.0).max(0.0);
                self.people[i].meals += 1;
                return;
            }
        }
        for k in [0, 3, 8] {
            if self.markets[home].stock(k) > 0 {
                self.markets[home].take(k, 1);
                self.markets[home].quotes[k].offered = self.markets[home].quotes[k]
                    .offered
                    .min(self.markets[home].stock(k));
                self.ledger.consumed[k] += 1;
                self.agents.hunger[i] = (self.agents.hunger[i] - 40.0).max(0.0);
                self.people[i].meals += 1;
                return;
            }
        }
    }
    pub(super) fn life_step(&mut self) {
        if self.tick % 10 != 0 {
            return;
        }
        let n = self.people.len();
        let mut births = vec![];
        for i in 0..n {
            if !self.agents.alive[i] {
                self.people[i].pregnancy = None;
                let spouse = self.people[i].spouse;
                self.people[i].spouse = NONE;
                if spouse != NONE && self.people[spouse as usize].spouse == i as u32 {
                    self.people[spouse as usize].spouse = NONE;
                    self.people[spouse as usize].happiness =
                        (self.people[spouse as usize].happiness - 15.0).max(0.0);
                }
                continue;
            }
            if self.people[i].spouse != NONE && !self.agents.alive[self.people[i].spouse as usize] {
                self.people[i].spouse = NONE;
                self.people[i].happiness = (self.people[i].happiness - 15.0).max(0.0);
            }
            let age = self.people[i].age_days(self.tick);
            self.agents.capacity[i] = if age < 6 * 365 {
                8
            } else if age < 18 * 365 {
                24
            } else {
                32 + (i % 5) as u32 * 4
            };
            let old = std::mem::take(&mut self.people[i].goals);
            let mut keep = Vec::with_capacity(5);
            for g in old {
                if self.goal_value(i, g.kind) >= g.target {
                    self.people[i].happiness = (self.people[i].happiness + 8.0).min(100.0);
                    self.stats.goals_completed += 1;
                } else if self.tick >= g.expires {
                    self.people[i].happiness = (self.people[i].happiness - 2.0).max(0.0);
                } else {
                    keep.push(g)
                }
            }
            self.people[i].goals = keep;
            let done = match self.people[i].aspiration {
                Aspiration::Wealth => self.agents.coins[i] >= 100_000,
                Aspiration::Blacksmith => self.people[i].crafts >= 20,
                Aspiration::Family => self.people[i].children.len() >= 2,
                Aspiration::Farmer => self.people[i].harvested >= 256,
                Aspiration::Service => self.people[i].jobs >= 20,
                Aspiration::Homeowner => self.owns_urban_home(i),
                Aspiration::Shopkeeper => self.owns_business(i),
                Aspiration::Artisan => self.urban_progress(i).made >= 20,
                Aspiration::Hunter => self.urban_progress(i).hunted >= 10,
                Aspiration::Haulier => self.urban_progress(i).hauled >= 20,
                Aspiration::Teacher => {
                    self.education.teaching_shifts.get(i).copied().unwrap_or(0) > 0
                }
            };
            if done && !self.people[i].aspiration_done {
                self.people[i].aspiration_done = true;
                self.people[i].happiness = (self.people[i].happiness + 20.0).min(100.0);
            }
            if self.tick % 50 == 0 {
                self.refresh_goals(i);
            }
            if let Some(g) = &self.people[i].pregnancy {
                let home = &self.markets[self.agents.home[i] as usize];
                if self.tick >= g.due
                    && dist(self.agents.x[i], self.agents.y[i], home.x, home.y)
                        <= rest::settlement_radius(home.housing_blocks)
                {
                    births.push((i, g.father as usize));
                }
            }
        }
        if self.tick % 50 == 0 {
            self.update_matches();
        }
        if self.tick % rest::DAY_TICKS == 0 {
            for i in 0..n {
                let p = &self.people[i];
                let age = p.age_days(self.tick);
                let j = p.spouse as usize;
                if !self.agents.alive[i]
                    || p.gender != 1
                    || !(18 * 365..46 * 365).contains(&age)
                    || j >= n
                    || !self.agents.alive[j]
                    || p.pregnancy.is_some()
                    || p.last_birth > 0
                        && self.tick.saturating_sub(p.last_birth) < 365 * rest::DAY_TICKS
                {
                    continue;
                }
                let home = self.agents.home[i] as usize;
                if self.agents.home[j] != home as u32
                    || self.markets[home].housing_blocks == 0
                    || self.markets[home].stock(0)
                        + self.markets[home].stock(3)
                        + self.markets[home].stock(8)
                        < 8
                    || dist(
                        self.agents.x[i],
                        self.agents.y[i],
                        self.agents.x[j],
                        self.agents.y[j],
                    ) > 80.0
                {
                    continue;
                }
                let chance = if p.aspiration == Aspiration::Family {
                    0.08
                } else {
                    0.025
                };
                if self.rng.unit() < chance {
                    self.people[i].pregnancy = Some(Pregnancy {
                        father: j as u32,
                        due: self.tick + 270 * rest::DAY_TICKS,
                    });
                }
            }
        }
        for (mother, father) in births.into_iter().take(100) {
            if self.agents.len() < 1_200_000 {
                self.birth(mother, father);
            }
        }
    }
    pub(super) fn birth(&mut self, mother: usize, father: usize) {
        let id = self.agents.len();
        let home = self.agents.home[mother];
        let (x, y) = (self.agents.x[mother], self.agents.y[mother]);
        self.agents.push_child(home, x, y);
        let mut person = Person::seeded(id, self.seed, self.tick, true);
        person.surname = self.people[father].surname;
        person.father = father as u32;
        person.mother = mother as u32;
        if self.rng.index(2) == 0 {
            person.aspiration = if self.rng.index(2) == 0 {
                self.people[mother].aspiration
            } else {
                self.people[father].aspiration
            };
        }
        let union =
            (self.agents.traits[mother] | self.agents.traits[father]) & !education::EDUCATED;
        let shared = self.agents.traits[mother] & self.agents.traits[father] & !education::EDUCATED;
        let mut inherited = shared;
        for bit in 0..10 {
            let flag = 1u16 << bit;
            if union & flag != 0 && self.rng.index(2) == 0 {
                inherited |= flag;
            }
        }
        if inherited == 0 && union != 0 {
            let options: Vec<u16> = (0..10)
                .map(|bit| 1u16 << bit)
                .filter(|flag| union & flag != 0)
                .collect();
            inherited = options[self.rng.index(options.len())];
        }
        self.agents.traits[id] = inherited;
        self.social.push(0.0);
        let mut genome =
            Genome::crossover(&self.genomes[father], &self.genomes[mother], &mut self.rng);
        genome.mutate_weights(&mut self.rng, 0.15);
        if self.rng.index(20) == 0 {
            genome.add_node(&mut self.rng, &mut self.innovations);
        }
        let brain = genome.compile().expect("valid inherited brain");
        self.genomes.push(genome);
        self.brains.push(brain);
        self.people.push(person);
        self.matches.push(NONE);
        self.sync_education();
        self.sync_urban();
        self.people[mother].children.push(id as u32);
        self.people[father].children.push(id as u32);
        self.people[mother].pregnancy = None;
        self.people[mother].last_birth = self.tick;
        self.stats.births += 1;
    }
    pub(super) fn person_trace(&self, i: usize) -> serde_json::Value {
        let p = &self.people[i];
        let traits = [
            (education::EDUCATED, "Educated"),
            (SOCIALITE, "Socialite"),
            (GREGARIOUS, "Gregarious"),
            (COMMUNITY_MINDED, "Community minded"),
            (NIGHT_OWL, "Night owl"),
            (COIN_GRABBER, "Coin grabber"),
            (FLEET, "Fleet-footed"),
            (HARDY, "Hardy"),
            (CLUMSY, "Clumsy"),
            (SLOW, "Slow-footed"),
            (GLUTTON, "Gluttonous"),
        ];
        serde_json::json!({"relationships":self.person_relationships(i),"relationship_limit":50,"education_bonus_pending":self.education.bonus_claims.iter().filter(|((person,_),_)|*person as usize==i).map(|(_,claim)|*claim as f32/100.0).sum::<f32>(),"school_days":self.education.attendance.get(i).copied().unwrap_or(0),"teacher_eligible":self.teacher_eligible(i),"teaching_shifts":self.education.teaching_shifts.get(i).copied().unwrap_or(0),"social_score":self.social[i],"social_benefit":if self.agents.traits[i]&SOCIALITE!=0 {"High social score can add happiness"} else {"Optional bonus; low score has no penalty"},"name":p.name(),"surname":SURNAMES[p.surname as usize%24],"gender":if p.gender==0{"Male"}else{"Female"},"age_years":p.age_days(self.tick) as f32/365.0,"adult":self.is_adult(i),"father":p.father,"mother":p.mother,"spouse":if p.spouse==NONE{None}else{Some(serde_json::json!({"id":p.spouse,"name":self.people[p.spouse as usize].name()}))},"children":p.children,"aspiration":match p.aspiration{Aspiration::Wealth=>"Own 1,000 gold (100,000 copper)",Aspiration::Blacksmith=>"Blacksmith: craft 20 tools",Aspiration::Family=>"Raise a family of two children",Aspiration::Farmer=>"Rice grower: harvest 256 rice",Aspiration::Service=>"Serve the community: complete 20 jobs",Aspiration::Teacher=>"Become a teacher",Aspiration::Homeowner=>"Own a home",Aspiration::Shopkeeper=>"Open a shop or workshop",Aspiration::Artisan=>"Produce 20 finished goods",Aspiration::Hunter=>"Complete 10 hunts",Aspiration::Haulier=>"Complete 20 transport jobs"},"aspiration_achieved":p.aspiration_done,"happiness":p.happiness,"tools":self.agents.tools[i].iter().enumerate().filter(|(_,q)|**q>0).map(|(t,_)|PRODUCTS[t+9]).collect::<Vec<_>>(),"traits":traits.iter().filter(|(bit,_)|self.agents.traits[i]&bit!=0).map(|(_,name)|name).collect::<Vec<_>>(),"goals":p.goals.iter().map(|g|serde_json::json!({"kind":if g.kind==GoalKind::BusinessLoan {"Secure a business loan".to_string()}else{format!("{:?}",g.kind)},"progress":self.goal_value(i,g.kind),"target":g.target,"days_left":g.expires.saturating_sub(self.tick) as f32/rest::DAY_TICKS as f32})).collect::<Vec<_>>(),"pregnancy":p.pregnancy.as_ref().map(|g|{let days=g.due.saturating_sub(self.tick) as f32/rest::DAY_TICKS as f32;serde_json::json!({"trimester":if days>180.0{1}else if days>90.0{2}else{3},"days_until_birth":days,"settlement_required":self.must_stay_home(i)})})})
    }
    pub(super) fn life_summary(&self) -> serde_json::Value {
        serde_json::json!({"children":(0..self.people.len()).filter(|&i|self.agents.alive[i]&&!self.is_adult(i)).count(),"married_people":self.people.iter().enumerate().filter(|(i,p)|self.agents.alive[*i]&&p.spouse!=NONE).count(),"pregnancies":self.people.iter().filter(|p|p.pregnancy.is_some()).count(),"average_happiness":self.people.iter().enumerate().filter(|(i,_)|self.agents.alive[*i]).map(|(_,p)|p.happiness).sum::<f32>()/self.living().max(1) as f32})
    }
    pub fn prepare_family_demo(&mut self) -> Result<(), String> {
        if self.agents.len() < 2 {
            return Err("Family demo needs two adults".into());
        }
        for i in 0..2 {
            self.people[i].initial_age_days = 25 * 365;
            self.people[i].gender = i as u8;
            self.people[i].aspiration = Aspiration::Family;
            self.agents.home[i] = 0;
            self.agents.x[i] = self.markets[0].x;
            self.agents.y[i] = self.markets[0].y;
        }
        self.people[0].spouse = 1;
        self.people[1].spouse = 0;
        self.people[1].pregnancy = Some(Pregnancy {
            father: 0,
            due: self.tick + rest::DAY_TICKS,
        });
        Ok(())
    }
    pub(super) fn validate_people(&self) -> Result<(), String> {
        for (i, p) in self.people.iter().enumerate() {
            if p.gender > 1
                || p.goals.len() > 5
                || !p.happiness.is_finite()
                || !(0.0..=100.0).contains(&p.happiness)
            {
                return Err("Invalid identity/goals".into());
            }
            if p.spouse != NONE {
                let j = p.spouse as usize;
                if j >= self.people.len()
                    || self.people[j].spouse != i as u32
                    || p.gender == self.people[j].gender
                {
                    return Err("Invalid marriage".into());
                }
            }
            for parent in [p.father, p.mother] {
                if parent != NONE && (parent as usize >= self.people.len() || parent as usize == i)
                {
                    return Err("Invalid parent".into());
                }
            }
            if let Some(g) = &p.pregnancy {
                if p.gender != 1 || g.father as usize >= self.people.len() {
                    return Err("Invalid pregnancy".into());
                }
            }
        }
        Ok(())
    }
}
