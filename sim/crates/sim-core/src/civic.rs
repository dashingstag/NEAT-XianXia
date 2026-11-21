//! Settlement NEAT policy, escrowed completion contracts and physical corpse handling.
use super::*;
pub const BODY_WEIGHT: u32 = 20;
pub const COUNCIL_SENSORS: [&str; neat_core::FRONTIER_INPUTS] = [
    "need",
    "public_benefit",
    "fiscal_risk",
    "retrieval_distance",
    "cost_fraction",
    "continuity",
    "income_rate",
    "expense_rate",
    "cash_reserve",
    "food_cover",
    "materials_ready",
    "graveyard_ready",
    "unfilled_pressure",
    "corpse_age",
    "night",
    "free_cash",
    "worker_supply",
    "grave_space",
    "nearby_corpses",
    "remote_corpses",
    "active_contracts",
    "income_balance",
    "fund_plot",
    "post_job",
    "raise_reward",
    "wait",
    "housing_cover",
    "health_pressure",
    "fertile_land",
    "rice_demand",
    "farm_count",
    "seed_supply",
    "iron_supply",
    "tool_supply",
    "population_growth",
    "family_housing",
    "children",
    "farm_wages",
    "crop_age",
    "harvest_ready",
    "food_pressure",
    "workforce",
    "threat_level",
    "quest_backlog",
    "guild_ready",
    "frontier_value",
    "mystic_reports",
    "leader_job_alignment",
];
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Corpse {
    pub agent: u32,
    pub market: u32,
    pub x: f32,
    pub y: f32,
    pub death_tick: u64,
    pub status: u8,
    pub claim: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    fn scenario(n: usize) -> World {
        let mut w = World::new(n, 42).unwrap();
        for m in &mut w.markets {
            m.cancel_quotes();
            m.add(1, 100, m.id);
            m.add(2, 100, m.id);
            w.ledger.initial[1] += 100;
            w.ledger.initial[2] += 100;
        }
        w.mortality_test(0, 5).unwrap();
        w
    }
    fn ready(w: &mut World) {
        if w.councils[0].graveyard == NONE {
            w.fund_graveyard(0, true);
        }
        let c = &mut w.councils[0];
        w.markets[0].coins += c.builder_escrow;
        c.builder_escrow = 0;
        c.builder_open = false;
        let g = &mut w.graveyards[c.graveyard as usize];
        g.ready = true;
        g.progress = g.required;
        w.tick += 10;
        w.plan_civic(0);
    }
    #[test]
    fn graveyards_move_beyond_housing_and_preserve_buried_remains() {
        let mut w = scenario(60);
        ready(&mut w);
        let id = w.councils[0].graveyard as usize;
        let town = &w.markets[0];
        assert!(
            dist(w.graveyards[id].x, w.graveyards[id].y, town.x, town.y)
                >= rest::settlement_radius(town.housing_blocks) + 190.0
        );
        let body = w.corpses.iter().position(|c| c.status == 0).unwrap();
        w.corpses[body].status = 2;
        w.corpses[body].claim = NONE;
        w.graveyards[id].buried.push(body as u32);
        w.graveyards[id].x = w.markets[0].x;
        w.graveyards[id].y = w.markets[0].y;
        let stocks = w.stocks();
        let coins = w.coins();
        w.move_graveyards_outward();
        let g = &w.graveyards[id];
        let actor = w.corpses[body].agent as usize;
        assert_eq!(w.corpses[body].x, g.x);
        assert_eq!(w.agents.x[actor], g.x);
        assert_eq!(g.buried, vec![body as u32]);
        assert_eq!(w.stocks(), stocks);
        assert_eq!(w.coins(), coins);
        w.check().unwrap();
        w.markets[0].housing_blocks = 100;
        w.move_graveyards_outward();
        assert!(
            dist(
                w.graveyards[id].x,
                w.graveyards[id].y,
                w.markets[0].x,
                w.markets[0].y
            ) >= rest::settlement_radius(100) + 150.0
        );
        assert!((30.0..=2630.0).contains(&w.graveyards[id].x));
        assert!((30.0..=1870.0).contains(&w.graveyards[id].y));
    }
    #[test]
    fn villages_have_distinct_neat_policies_and_real_income_sensors() {
        let mut w = scenario(60);
        assert_ne!(w.councils[0].genome, w.councils[1].genome);
        assert!(w.councils[0].near_dead > 0);
        assert!(!w.councils[0].trace.is_empty());
        w.markets[0].revenue_total = 25;
        w.tick += 10;
        w.plan_civic(0);
        assert!(w.councils[0].income_rate > 0.0);
        assert!(w.councils[0].trace.iter().all(|c| c.score.is_finite()));
        w.check().unwrap();
    }
    #[test]
    fn full_neural_job_flow_builds_retrieves_and_buries() {
        let mut w = scenario(60);
        let targets: Vec<_> = w.corpses.iter().map(|c| c.agent).collect();
        let targets_buried = |w: &World| {
            targets
                .iter()
                .all(|&i| w.corpses[w.agents.corpse[i as usize] as usize].status == 2)
        };
        for _ in 0..(2 * rest::DAY_TICKS) {
            w.step(true);
            w.check().unwrap();
            if targets_buried(&w) {
                break;
            }
        }
        assert!(w.stats.graveyards_built > 0, "{}", w.summary());
        assert!(targets_buried(&w), "{}", w.summary());
        assert!(w.stats.jobs_completed >= 6);
        w.validate_jobs().unwrap();
    }
    #[test]
    fn one_body_has_one_owner_and_rewards_are_escrowed() {
        let mut w = scenario(60);
        ready(&mut w);
        let corpse = w.councils[0].candidates[0];
        w.councils[0].candidates = vec![corpse];
        let worker = (0..w.agents.len()).find(|&i| w.agents.alive[i]).unwrap();
        let other = worker + 1;
        let reward = w.councils[0].reward;
        let money = w.agents.coins[worker];
        assert!(w.accept_job(worker, 0, Action::Gravedigger));
        assert!(!w.accept_job(other, 0, Action::Gravedigger));
        assert_eq!(w.agents.coins[worker], money);
        assert_eq!(w.corpses[corpse as usize].claim, worker as u32);
        let slot = w.agents.contract[worker] as usize;
        assert_eq!(w.contracts[slot].as_ref().unwrap().reward, reward);
        w.check().unwrap();
        w.validate_jobs().unwrap();
    }
    #[test]
    fn death_drops_a_carried_body_and_refunds_the_unpaid_reward() {
        let mut w = scenario(60);
        ready(&mut w);
        let i = (0..w.agents.len()).find(|&i| w.agents.alive[i]).unwrap();
        assert!(w.accept_job(i, 0, Action::Gravedigger));
        let slot = w.agents.contract[i] as usize;
        let body = w.contracts[slot].as_ref().unwrap().corpse;
        w.contracts[slot].as_mut().unwrap().phase = 2;
        w.agents.carried_body[i] = body;
        w.corpses[body as usize].status = 1;
        assert!(w.agents.weight(i) >= BODY_WEIGHT);
        let reward = w.contracts[slot].as_ref().unwrap().reward;
        let treasury = w.markets[0].coins;
        w.agents.alive[i] = false;
        w.register_corpse(i);
        w.tick += 1;
        w.civic_step();
        assert_eq!(w.agents.carried_body[i], NONE);
        assert_eq!(w.corpses[body as usize].status, 0);
        assert_eq!(w.corpses[body as usize].claim, NONE);
        assert_eq!(w.markets[0].coins, treasury + reward);
        w.check().unwrap();
        w.validate_jobs().unwrap();
    }
    #[test]
    fn unfilled_work_can_raise_rewards_without_creating_money() {
        let mut w = scenario(60);
        ready(&mut w);
        assert!(w.councils[0].vacancies > 0);
        let old = w.councils[0].reward;
        let money = w.coins();
        w.tick += 110;
        w.plan_civic(0);
        assert!(w.councils[0].reward > old, "{}", w.summary());
        assert_eq!(w.coins(), money);
        w.check().unwrap();
    }
    #[test]
    fn graveyard_materials_and_treasury_are_hard_constraints() {
        let mut w = World::new(60, 42).unwrap();
        w.mortality_test(0, 5).unwrap();
        assert_eq!(w.councils[0].graveyard, NONE);
        assert!(!w.councils[0].builder_open);
        assert!(w.councils[0].blocked.contains("timber"));
        w.check().unwrap();
    }
    #[test]
    fn active_contracts_and_corpses_replay_after_save() {
        let mut a = scenario(60);
        ready(&mut a);
        let i = (0..a.agents.len()).find(|&i| a.agents.alive[i]).unwrap();
        assert!(a.accept_job(i, 0, Action::Gravedigger));
        let path =
            std::env::temp_dir().join(format!("xianxia-civic-{}.snapshot", std::process::id()));
        a.save(&path).unwrap();
        let mut b = World::load(&path).unwrap();
        for _ in 0..400 {
            a.step(false);
            b.step(true);
        }
        assert_eq!(a.agents, b.agents);
        assert_eq!(a.contracts, b.contracts);
        assert_eq!(a.corpses, b.corpses);
        assert_eq!(a.graveyards, b.graveyards);
        assert_eq!(a.summary(), b.summary());
        a.validate_jobs().unwrap();
        a.check().unwrap();
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn carry_capacity_and_buried_rendering_are_enforced() {
        let mut w = scenario(60);
        ready(&mut w);
        let i = (0..w.agents.len()).find(|&i| w.agents.alive[i]).unwrap();
        w.agents.capacity[i] = w.agents.weight(i);
        assert!(!w.accept_job(i, 0, Action::Gravedigger));
        let body = w.councils[0].candidates[0] as usize;
        w.corpses[body].status = 2;
        w.graveyards[w.councils[0].graveyard as usize]
            .buried
            .push(body as u32);
        assert!(w.corpse_hidden(w.corpses[body].agent as usize));
        let meta = w.graveyard_summary();
        assert!(meta[0].get("buried").is_none());
        assert_eq!(meta[0]["buried_count"], 1);
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Graveyard {
    pub market: u32,
    pub x: f32,
    pub y: f32,
    pub progress: f32,
    pub required: f32,
    pub capacity: u32,
    pub buried: Vec<u32>,
    pub ready: bool,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum JobKind {
    Build,
    Haul,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Contract {
    pub worker: u32,
    pub market: u32,
    pub kind: JobKind,
    pub corpse: u32,
    pub graveyard: u32,
    pub reward: u64,
    pub phase: u8,
    pub timer: f32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Council {
    pub genome: Genome,
    pub innovations: Innovations,
    #[serde(skip)]
    pub brain: Option<Compiled>,
    pub graveyard: u32,
    pub near_dead: u32,
    pub remote_dead: u32,
    pub vacancies: u32,
    pub reward: u64,
    pub escrow: u64,
    pub builder_open: bool,
    pub builder_escrow: u64,
    pub last_filled: u64,
    pub last_revenue: u64,
    pub last_expenses: u64,
    pub income_rate: f32,
    pub expense_rate: f32,
    pub candidates: Vec<u32>,
    pub last_action: String,
    pub blocked: String,
    pub trace: Vec<PolicyChoice>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PolicyChoice {
    pub action: String,
    pub score: f32,
    pub inputs: Vec<f32>,
}

impl Council {
    fn connect_leadership(&mut self) {
        Self::connect_leadership_gene(&mut self.genome, &mut self.innovations);
    }

    fn connect_leadership_gene(genome: &mut Genome, innovations: &mut Innovations) {
        let Some(to) = genome
            .nodes
            .iter()
            .find(|n| n.kind == neat_core::Kind::Output)
            .map(|n| n.id)
        else {
            return;
        };
        let from = 47;
        if !genome
            .genes
            .iter()
            .any(|gene| (gene.from, gene.to) == (from, to))
        {
            genome.genes.push(neat_core::Gene {
                innovation: innovations.edge(from, to),
                from,
                to,
                weight: 0.35,
                enabled: true,
            });
        }
    }
    fn new(seed: u64, id: usize) -> Self {
        let mut rng = Rng(seed ^ (id as u64 + 1).wrapping_mul(0x9e3779b97f4a7c15));
        let mut innovations = Innovations::default();
        let mut genome = Genome::seeded(&mut rng, &mut innovations);
        // Separate municipal sensor semantics, independently seeded genomes.
        for gene in &mut genome.genes {
            if gene.from < (INPUTS as u32) {
                let base = match gene.from {
                    0 => 2.2,
                    1 => 1.2,
                    2 => -2.0,
                    3 => -0.6,
                    4 => -0.7,
                    6 => 0.35,
                    7 => -0.35,
                    8 => 0.4,
                    10 => 0.3,
                    11 => 0.2,
                    12 => 0.8,
                    13 => 0.3,
                    14 => -0.1,
                    21 => 0.25,
                    _ => 0.0,
                };
                gene.weight = base + rng.signed() * 0.12;
            }
        }
        Self::connect_leadership_gene(&mut genome, &mut innovations);
        let brain = Some(genome.compile().expect("valid council seed"));
        Self {
            genome,
            innovations,
            brain,
            graveyard: NONE,
            near_dead: 0,
            remote_dead: 0,
            vacancies: 0,
            reward: 2,
            escrow: 0,
            builder_open: false,
            builder_escrow: 0,
            last_filled: 0,
            last_revenue: 0,
            last_expenses: 0,
            income_rate: 0.0,
            expense_rate: 0.0,
            candidates: vec![],
            last_action: "Wait".into(),
            blocked: String::new(),
            trace: vec![],
        }
    }
}
impl World {
    pub(super) fn sync_councils(&mut self) {
        while self.councils.len() < self.markets.len() {
            self.councils
                .push(Council::new(self.seed, self.councils.len()));
        }
        for c in &mut self.councils {
            if c.brain.is_none() {
                c.connect_leadership();
                c.brain = c.genome.compile().ok();
            }
        }
    }
    pub(super) fn job_escrow(&self) -> u64 {
        self.councils
            .iter()
            .map(|c| c.escrow + c.builder_escrow)
            .sum::<u64>()
            + self
                .contracts
                .iter()
                .flatten()
                .map(|c| c.reward)
                .sum::<u64>()
    }
    pub(super) fn register_corpse(&mut self, i: usize) {
        if self.meat_trade.pack(i) > 0 {
            self.meat_trade.discard_pack(i);
            self.update_urban_weight(i);
        }
        self.relationship_death(i);
        if self.agents.corpse[i] != NONE {
            return;
        }
        let (x, y) = (self.agents.x[i], self.agents.y[i]);
        let market = self
            .markets
            .iter()
            .min_by(|a, b| dist(x, y, a.x, a.y).total_cmp(&dist(x, y, b.x, b.y)))
            .unwrap()
            .id;
        self.agents.corpse[i] = self.corpses.len() as u32;
        self.corpses.push(Corpse {
            agent: i as u32,
            market,
            x,
            y,
            death_tick: self.tick,
            status: 0,
            claim: NONE,
        });
    }
    pub fn mortality_test(&mut self, market: usize, count: usize) -> Result<usize, String> {
        if market >= self.markets.len() || count == 0 || count > 20 {
            return Err("Mortality test: choose a market and 1..20 nearby people".into());
        }
        let m = &self.markets[market];
        let radius = rest::settlement_radius(m.housing_blocks) + 70.0;
        let victims: Vec<_> = (0..self.agents.len())
            .filter(|&i| {
                self.agents.alive[i] && dist(self.agents.x[i], self.agents.y[i], m.x, m.y) < radius
            })
            .take(count)
            .collect();
        for &i in &victims {
            self.agents.alive[i] = false;
            self.agents.health[i] = 0.0;
            self.agents.sleeping[i] = false;
            self.agents.rested[i] = false;
            self.stats.deaths += 1;
            self.register_corpse(i);
        }
        self.sync_rest_reservations();
        self.civic_step();
        Ok(victims.len())
    }
    fn active_jobs(&self, m: usize, kind: JobKind) -> usize {
        self.contracts
            .iter()
            .flatten()
            .filter(|c| c.market as usize == m && c.kind == kind)
            .count()
    }
    pub(super) fn civic_step(&mut self) {
        self.sync_councils();
        for slot in 0..self.contracts.len() {
            let Some(c) = self.contracts[slot].as_ref() else {
                continue;
            };
            let i = c.worker as usize;
            if !self.agents.alive[i]
                || self.agents.health[i] < 25.0
                || self.agents.hunger[i] > 70.0
                || (!self.agents.rested[i] && self.agents.energy[i] < 2.0)
                || self.must_stay_home(i)
                || !self.is_adult(i)
            {
                self.cancel_job(slot);
                continue;
            }
            if c.kind == JobKind::Haul && c.phase == 2 {
                let body = c.corpse as usize;
                let dead = self.corpses[body].agent as usize;
                self.corpses[body].x = self.agents.x[i];
                self.corpses[body].y = self.agents.y[i];
                self.agents.x[dead] = self.agents.x[i];
                self.agents.y[dead] = self.agents.y[i];
            }
        }
        if self.tick % 10 != 0 {
            return;
        }
        self.move_graveyards_outward();
        for m in 0..self.markets.len() {
            self.plan_civic(m);
        }
    }
    fn plan_civic(&mut self, m: usize) {
        use std::collections::BinaryHeap;
        let market = &self.markets[m];
        let radius = rest::settlement_radius(market.housing_blocks) + 70.0;
        let mut near = 0;
        let mut remote = 0;
        let mut oldest = 0;
        let mut nearest = BinaryHeap::new();
        for (id, body) in self.corpses.iter().enumerate() {
            if body.market as usize != m || body.status == 2 {
                continue;
            }
            let d = dist(body.x, body.y, market.x, market.y);
            if d <= radius {
                near += 1;
            } else {
                remote += 1;
            }
            oldest = oldest.max(self.tick.saturating_sub(body.death_tick));
            if body.status == 0 && body.claim == NONE {
                nearest.push(((d * 100.0) as u32, id as u32));
                if nearest.len() > 64 {
                    nearest.pop();
                }
            }
        }
        let active = self.active_jobs(m, JobKind::Haul);
        let building = self.active_jobs(m, JobKind::Build);
        let c = &mut self.councils[m];
        c.near_dead = near;
        c.remote_dead = remote;
        c.candidates = nearest
            .into_sorted_vec()
            .into_iter()
            .map(|(_, id)| id)
            .collect();
        c.income_rate =
            c.income_rate * 0.8 + market.revenue_total.saturating_sub(c.last_revenue) as f32 * 0.2;
        c.expense_rate = c.expense_rate * 0.8
            + market.expenses_total.saturating_sub(c.last_expenses) as f32 * 0.2;
        c.last_revenue = market.revenue_total;
        c.last_expenses = market.expenses_total;
        let pressure =
            (near as f32 + remote as f32 * 0.15) / ((market.population / 50).max(4) as f32);
        let pressure = pressure.min(1.0);
        let desired = ((near + remote).div_ceil(4) as usize)
            .min(24)
            .saturating_sub(active)
            .min(c.candidates.len());
        if c.vacancies as usize > desired {
            let excess = c.vacancies - desired as u32;
            let refund = excess as u64 * c.reward;
            c.escrow -= refund;
            c.vacancies -= excess;
            self.markets[m].coins += refund;
        }
        let yard = self.councils[m].graveyard;
        let needs_plot = yard == NONE
            || (self.graveyards[yard as usize].ready
                && self.graveyards[yard as usize].buried.len()
                    >= self.graveyards[yard as usize].capacity as usize);
        let unfinished = yard != NONE && !self.graveyards[yard as usize].ready;
        let ready = yard != NONE && self.graveyards[yard as usize].ready;
        let available = if ready {
            self.graveyards[yard as usize].capacity as usize
                - self.graveyards[yard as usize].buried.len()
        } else {
            0
        };
        let desired = desired.min(available.saturating_sub(active));
        let market = &self.markets[m];
        let c = &self.councils[m];
        let material_ok = market.stock(1) >= 8 && market.stock(2) >= 4;
        let treasury = market.coins;
        let next_reward = (c.reward + 1)
            .max(2 + (near / 3).min(18) as u64 + (oldest / 300).min(8))
            .min(32);
        let mut candidates: Vec<(&str, u64, f32)> = vec![("Wait", 0, 0.0)];
        candidates.extend(self.urban_council_candidates(m));
        candidates.extend(self.bank_council_candidates(m));
        if self.can_fund_school(m) {
            candidates.push(("FundSchool", 30, 1.0));
        }
        if self.can_post_teacher(m) {
            candidates.push(("PostTeacher", 12, 0.9));
        }
        if self.can_fund_tavern(m) {
            candidates.push(("FundTavern", 20, 0.7));
        }
        if self.can_post_farm(m) {
            candidates.push(("PostFarmer", 2, 1.0));
        }
        if near + remote > 0
            && !c.builder_open
            && building == 0
            && (needs_plot || unfinished)
            && treasury >= 8
            && (!needs_plot || material_ok)
        {
            candidates.push(("FundGraveyard", 8, 1.0));
        }
        if ready && desired > c.vacancies as usize && treasury >= c.reward {
            candidates.push(("PostGravedigger", c.reward, 0.9));
        }
        if ready
            && c.vacancies > 0
            && self.tick.saturating_sub(c.last_filled) >= 100
            && next_reward > c.reward
            && treasury >= (next_reward - c.reward) * c.vacancies as u64
        {
            candidates.push((
                "RaiseReward",
                (next_reward - c.reward) * c.vacancies as u64,
                1.0,
            ));
        }
        let mut trace = Vec::new();
        for (action, cost, benefit) in candidates {
            let mut x = [0.0; INPUTS];
            x[0] = if action == "FundBank" || action == "CapitaliseBank" {
                0.85
            } else if action == "AdvertisePremises" {
                0.65 + 0.35 * self.operator_vacancy_pressure(m)
            } else if [
                "GazetteDistricts",
                "CommissionHomes",
                "CommissionShops",
                "CommissionWorkshops",
                "AdvertisePremises",
                "ClearForDistrict",
            ]
            .contains(&action)
            {
                0.85
            } else if action == "ReviewShopLeases" {
                0.7
            } else if action == "ClearUnusedShelter" {
                0.35
            } else if action == "FundSchool" || action == "PostTeacher" {
                0.9
            } else if action == "FundTavern" {
                0.5
            } else if action == "PostFarmer" {
                self.farm_food_pressure(m)
            } else if action == "Wait" {
                (1.0 - pressure) * 0.2
            } else {
                pressure
            };
            x[1] = benefit;
            x[2] = if cost > 0 {
                cost as f32 / treasury.max(1) as f32 * 0.45
            } else {
                0.0
            };
            x[4] = cost as f32 / treasury.max(1) as f32;
            x[5] = if c.last_action == action { 0.1 } else { 0.0 };
            x[6] = (c.income_rate / 10.0).min(1.0);
            x[7] = (c.expense_rate / 10.0).min(1.0);
            x[8] = (treasury as f32 / (market.population.max(1) as f32 * 2.0)).min(1.0);
            x[9] = ((market.stock(0) + market.stock(3)) as f32 / market.population.max(1) as f32)
                .min(1.0);
            x[10] = if material_ok { 1.0 } else { 0.0 };
            x[11] = if ready { 1.0 } else { 0.0 };
            x[12] = if action == "RaiseReward" {
                (self.tick.saturating_sub(c.last_filled) as f32 / 100.0).min(1.0)
            } else {
                0.0
            };
            x[13] = (oldest as f32 / 600.0).min(1.0);
            x[14] = if self.is_night() { 1.0 } else { 0.0 };
            x[15] = (treasury as f32 / 100.0).min(1.0);
            x[16] = (active as f32 / 24.0).min(1.0);
            x[17] = (available as f32 / 64.0).min(1.0);
            x[18] = (near as f32 / 20.0).min(1.0);
            x[19] = (remote as f32 / 20.0).min(1.0);
            x[20] = (active as f32 / 24.0).min(1.0);
            x[21] = ((c.income_rate - c.expense_rate) / 10.0).clamp(-1.0, 1.0);
            x[22] = if action == "FundGraveyard" { 1.0 } else { 0.0 };
            x[23] = if action == "PostGravedigger" {
                1.0
            } else {
                0.0
            };
            x[24] = if action == "RaiseReward" { 1.0 } else { 0.0 };
            x[25] = if action == "Wait" { 1.0 } else { 0.0 };
            x[26] =
                (market.housing_blocks as f32 * 32.0 / market.population.max(1) as f32).min(1.0);
            x[27] = pressure;
            x[28] = if self
                .fertility
                .iter()
                .any(|r| r.fertility >= 0.75 && dist(r.x, r.y, market.x, market.y) < 800.0)
            {
                1.0
            } else {
                0.0
            };
            x[29] = self.farm_food_pressure(m);
            x[30] = self.farms.iter().filter(|f| f.market as usize == m).count() as f32 / 4.0;
            x[31] = if market.stock(0) + market.stock(8) >= 2 {
                1.0
            } else {
                0.0
            };
            x[32] = (market.stock(7) as f32 / 20.0).min(1.0);
            x[33] = ((9..13).map(|k| market.stock(k)).sum::<u64>() as f32 / 20.0).min(1.0);
            x[34] = market.population as f32 / self.agents.len().max(1) as f32;
            x[35] = x[26];
            x[36] = self.stats.births as f32 / self.agents.len().max(1) as f32;
            x[40] = x[29];
            x[41] = 1.0 - x[36];
            x[47] = self.leader_job_alignment(m, action);
            trace.push(PolicyChoice {
                action: action.into(),
                score: c.brain.as_ref().unwrap().evaluate(&x),
                inputs: x.to_vec(),
            });
        }
        let action = trace
            .iter()
            .max_by(|a, b| a.score.total_cmp(&b.score))
            .unwrap()
            .action
            .clone();
        self.councils[m].trace = trace;
        self.councils[m].last_action = action.clone();
        self.councils[m].blocked = if near + remote == 0 {
            "No unburied bodies"
        } else if needs_plot && !material_ok {
            "Need 8 timber and 4 stone for a graveyard"
        } else if treasury < 2 {
            "Insufficient treasury cash"
        } else {
            ""
        }
        .into();
        match action.as_str() {
            "GazetteDistricts"
            | "CommissionHomes"
            | "CommissionShops"
            | "CommissionWorkshops"
            | "AdvertisePremises"
            | "ClearForDistrict"
            | "ReviewShopLeases"
            | "ClearUnusedShelter" => self.apply_urban_policy(m, &action),
            "FundBank" | "CapitaliseBank" => self.apply_bank_policy(m, &action),
            "FundSchool" => self.fund_school(m),
            "PostTeacher" => self.post_teacher(m),
            "FundTavern" => self.fund_tavern(m),
            "PostFarmer" => self.post_farm(m),
            "FundGraveyard" => self.fund_graveyard(m, needs_plot),
            "PostGravedigger" => {
                let reward = self.councils[m].reward;
                let add = (desired - self.councils[m].vacancies as usize)
                    .min((self.markets[m].coins / reward) as usize);
                let cost = add as u64 * reward;
                self.markets[m].coins -= cost;
                self.councils[m].escrow += cost;
                if self.councils[m].vacancies == 0 {
                    self.councils[m].last_filled = self.tick;
                }
                self.councils[m].vacancies += add as u32;
            }
            "RaiseReward" => {
                let cost =
                    (next_reward - self.councils[m].reward) * self.councils[m].vacancies as u64;
                self.markets[m].coins -= cost;
                self.councils[m].escrow += cost;
                self.councils[m].reward = next_reward;
                self.councils[m].last_filled = self.tick;
            }
            _ => {}
        }
    }
    fn outlying_graveyard_site(&self, m: usize) -> (f32, f32) {
        let town = &self.markets[m];
        let radius = rest::settlement_radius(town.housing_blocks) + 200.0;
        (0..32)
            .map(|j| {
                let angle = j as f32 * std::f32::consts::TAU / 32.0 - std::f32::consts::FRAC_PI_4;
                let x = (town.x + radius * angle.cos()).clamp(30.0, 2630.0);
                let y = (town.y + radius * angle.sin()).clamp(30.0, 1870.0);
                let clearance = self
                    .markets
                    .iter()
                    .map(|t| dist(x, y, t.x, t.y) - rest::settlement_radius(t.housing_blocks))
                    .fold(f32::INFINITY, f32::min);
                (x, y, clearance)
            })
            .max_by(|a, b| a.2.total_cmp(&b.2))
            .map(|(x, y, _)| (x, y))
            .unwrap()
    }
    pub(super) fn move_graveyards_outward(&mut self) {
        for id in 0..self.graveyards.len() {
            let g = &self.graveyards[id];
            let m = g.market as usize;
            let town = &self.markets[m];
            if dist(g.x, g.y, town.x, town.y)
                >= rest::settlement_radius(town.housing_blocks) + 150.0
            {
                continue;
            }
            let (x, y) = self.outlying_graveyard_site(m);
            let g = &mut self.graveyards[id];
            g.x = x;
            g.y = y;
            for &body in &g.buried {
                let c = &mut self.corpses[body as usize];
                c.x = x;
                c.y = y;
                self.agents.x[c.agent as usize] = x;
                self.agents.y[c.agent as usize] = y;
            }
        }
    }
    fn fund_graveyard(&mut self, m: usize, materials: bool) {
        if materials {
            for (k, qty) in [(1, 8), (2, 4)] {
                let mut left = qty;
                while left > 0 {
                    left -= self.markets[m].take(k, left).unwrap().quantity;
                }
                self.ledger.built[k] += qty;
                self.markets[m].quotes[k].offered = self.markets[m].quotes[k]
                    .offered
                    .min(self.markets[m].stock(k));
            }
            let id = self.councils[m].graveyard;
            if id == NONE {
                let (x, y) = self.outlying_graveyard_site(m);
                self.councils[m].graveyard = self.graveyards.len() as u32;
                self.graveyards.push(Graveyard {
                    market: m as u32,
                    x,
                    y,
                    progress: 0.0,
                    required: 20.0,
                    capacity: 64,
                    buried: vec![],
                    ready: false,
                });
            } else {
                let g = &mut self.graveyards[id as usize];
                g.capacity += 64;
                g.progress = 0.0;
                g.ready = false;
            }
        }
        self.markets[m].coins -= 8;
        self.councils[m].builder_open = true;
        self.councils[m].builder_escrow = 8;
    }
    pub(super) fn job_opportunity(&self, i: usize) -> Option<(usize, Action, u64, f32, f32, f32)> {
        let a = &self.agents;
        let mut best = None;
        let mut utility = f32::NEG_INFINITY;
        for (m, c) in self.councils.iter().enumerate() {
            if !c.builder_open && c.vacancies == 0 {
                continue;
            }
            let distance = dist(a.x[i], a.y[i], self.markets[m].x, self.markets[m].y);
            if distance > 350.0 && a.home[i] as usize != m {
                continue;
            }
            let Some(g) = self.graveyards.get(c.graveyard as usize) else {
                continue;
            };
            let pressure = ((c.near_dead as f32 + c.remote_dead as f32 * 0.15) / 8.0).min(1.0);
            for (action, reward, travel, risk) in [
                (
                    Action::BuildGraveyard,
                    if c.builder_open { 8 } else { 0 },
                    distance + dist(self.markets[m].x, self.markets[m].y, g.x, g.y),
                    0.06,
                ),
                (
                    Action::Gravedigger,
                    if c.vacancies > 0 && a.room(i) >= BODY_WEIGHT {
                        c.reward
                    } else {
                        0
                    },
                    distance
                        + self
                            .nearest_body(i, m)
                            .map(|id| {
                                dist(a.x[i], a.y[i], self.corpses[id].x, self.corpses[id].y)
                                    + dist(self.corpses[id].x, self.corpses[id].y, g.x, g.y)
                            })
                            .unwrap_or(1000.0),
                    0.12 + if self.is_night() { 0.15 } else { 0.0 },
                ),
            ] {
                if reward == 0
                    || (action == Action::Gravedigger
                        && (!g.ready || self.nearest_body(i, m).is_none()))
                {
                    continue;
                }
                let risk =
                    (risk + travel / 2500.0 + ((40.0 - a.health[i]).max(0.0) / 100.0)).min(1.0);
                let score = reward as f32 / (1.0 + travel / 100.0) - risk;
                if score > utility {
                    utility = score;
                    best = Some((m, action, reward, risk, travel, pressure));
                }
            }
        }
        best
    }
    fn nearest_body(&self, i: usize, m: usize) -> Option<usize> {
        self.councils[m]
            .candidates
            .iter()
            .filter_map(|&id| {
                self.corpses
                    .get(id as usize)
                    .filter(|c| c.status == 0 && c.claim == NONE)
                    .map(|c| {
                        (
                            id as usize,
                            dist(self.agents.x[i], self.agents.y[i], c.x, c.y),
                        )
                    })
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(id, _)| id)
    }
    pub(super) fn accept_job(&mut self, i: usize, m: usize, action: Action) -> bool {
        if m >= self.councils.len() || self.agents.contract[i] != NONE || !self.agents.alive[i] {
            return false;
        }
        let yard = self.councils[m].graveyard;
        if yard == NONE {
            return false;
        }
        let (kind, corpse, reward) = if action == Action::BuildGraveyard {
            if !self.councils[m].builder_open {
                return false;
            }
            self.councils[m].builder_open = false;
            let reward = self.councils[m].builder_escrow;
            self.councils[m].builder_escrow = 0;
            (JobKind::Build, NONE, reward)
        } else {
            if self.councils[m].vacancies == 0
                || self.agents.room(i) < BODY_WEIGHT
                || !self.graveyards[yard as usize].ready
            {
                return false;
            }
            let Some(corpse) = self.nearest_body(i, m) else {
                return false;
            };
            let g = &self.graveyards[yard as usize];
            if g.buried.len() + self.active_jobs(m, JobKind::Haul) >= g.capacity as usize {
                return false;
            }
            let reward = self.councils[m].reward;
            self.councils[m].vacancies -= 1;
            self.councils[m].escrow -= reward;
            self.corpses[corpse].claim = i as u32;
            self.councils[m].last_filled = self.tick;
            (JobKind::Haul, corpse as u32, reward)
        };
        let slot = self
            .contracts
            .iter()
            .position(Option::is_none)
            .unwrap_or(self.contracts.len());
        let contract = Some(Contract {
            worker: i as u32,
            market: m as u32,
            kind,
            corpse,
            graveyard: yard,
            reward,
            phase: 0,
            timer: 0.0,
        });
        if slot == self.contracts.len() {
            self.contracts.push(contract)
        } else {
            self.contracts[slot] = contract
        }
        self.agents.contract[i] = slot as u32;
        self.agents.action[i] = action;
        self.agents.target[i] = m as u32;
        self.agents.timer[i] = 0.0;
        true
    }
    pub(super) fn cancel_job(&mut self, slot: usize) {
        let Some(c) = self.contracts[slot].take() else {
            return;
        };
        let i = c.worker as usize;
        self.commitments.cancel(i, Action::BuildGraveyard);
        if c.kind == JobKind::Haul {
            let body = &mut self.corpses[c.corpse as usize];
            if body.status == 1 {
                body.x = self.agents.x[i];
                body.y = self.agents.y[i];
                body.status = 0;
                let dead = body.agent as usize;
                self.agents.x[dead] = body.x;
                self.agents.y[dead] = body.y;
            }
            body.claim = NONE;
        }
        self.markets[c.market as usize].coins += c.reward;
        self.agents.contract[i] = NONE;
        self.agents.carried_body[i] = NONE;
        self.agents.action[i] = Action::Wait;
        self.stats.jobs_cancelled += 1;
    }
    pub(super) fn job_destination(&self, i: usize) -> Option<(f32, f32)> {
        let c = self
            .contracts
            .get(self.agents.contract[i] as usize)?
            .as_ref()?;
        if c.phase == 0 {
            let m = &self.markets[c.market as usize];
            Some((m.x + 22.0, m.y + 5.0))
        } else if c.kind == JobKind::Haul && c.phase == 1 {
            let b = &self.corpses[c.corpse as usize];
            Some((b.x, b.y))
        } else {
            let g = &self.graveyards[c.graveyard as usize];
            Some((g.x, g.y))
        }
    }
    pub(super) fn execute_job(&mut self, i: usize) {
        let work_speed = self.work_factor(i, self.agents.action[i], 2);
        let slot = self.agents.contract[i] as usize;
        let Some((x, y)) = self.job_destination(i) else {
            self.agents.action[i] = Action::Wait;
            return;
        };
        let d = dist(self.agents.x[i], self.agents.y[i], x, y);
        if d > 4.0 {
            let speed = self.actor_speed(i, self.agents.x[i], self.agents.y[i]);
            let step = (speed * DT).min(d - 3.0);
            self.walk_toward(i, x, y, step);
            self.agents.drain_energy(i, DT * 0.1);
            return;
        }
        let c = self.contracts[slot].as_mut().unwrap();
        if c.phase == 0 {
            c.phase = 1;
            return;
        }
        if c.kind == JobKind::Build {
            let g = &mut self.graveyards[c.graveyard as usize];
            g.progress = (g.progress + DT * work_speed).min(g.required);
            self.agents.drain_energy(i, DT * 0.3);
            if g.progress < g.required {
                return;
            }
            g.ready = true;
            self.stats.graveyards_built += 1;
            self.commitments
                .resolve(i, Action::BuildGraveyard, c.market, 2, self.tick, true);
        } else if c.phase == 1 {
            c.timer += DT * work_speed;
            if c.timer < 1.0 {
                return;
            }
            if self.agents.room(i) < BODY_WEIGHT {
                self.cancel_job(slot);
                return;
            }
            let c = self.contracts[slot].as_mut().unwrap();
            self.corpses[c.corpse as usize].status = 1;
            self.agents.carried_body[i] = c.corpse;
            c.phase = 2;
            c.timer = 0.0;
            return;
        } else {
            c.timer += DT * work_speed;
            if c.timer < 3.0 {
                return;
            }
            let g = &mut self.graveyards[c.graveyard as usize];
            if !g.ready || g.buried.len() >= g.capacity as usize {
                self.cancel_job(slot);
                return;
            }
            let body = &mut self.corpses[c.corpse as usize];
            body.status = 2;
            body.claim = NONE;
            body.x = g.x;
            body.y = g.y;
            self.agents.x[body.agent as usize] = g.x;
            self.agents.y[body.agent as usize] = g.y;
            g.buried.push(c.corpse);
            self.agents.carried_body[i] = NONE;
            self.stats.burials += 1;
        }
        let c = self.contracts[slot].take().unwrap();
        self.agents.coins[i] += c.reward;
        self.markets[c.market as usize].expenses_total += c.reward;
        self.credit_education_bonus(i, c.market as usize, c.reward);
        self.agents.contract[i] = NONE;
        self.agents.action[i] = Action::Wait;
        self.stats.jobs_completed += 1;
        self.people[i].jobs += 1;
    }
    pub(super) fn graveyard_summary(&self) -> Vec<serde_json::Value> {
        self.graveyards.iter().enumerate().map(|(id,g)|serde_json::json!({"id":id,"market":g.market,"x":g.x,"y":g.y,"progress":g.progress,"required":g.required,"capacity":g.capacity,"buried_count":g.buried.len(),"ready":g.ready})).collect()
    }
    pub(super) fn corpse_hidden(&self, i: usize) -> bool {
        self.corpses
            .get(self.agents.corpse[i] as usize)
            .is_some_and(|c| c.status != 0)
    }
    pub(super) fn job_trace(&self, i: usize) -> serde_json::Value {
        let Some(Some(c)) = self.contracts.get(self.agents.contract[i] as usize) else {
            return serde_json::Value::Null;
        };
        serde_json::json!({"job":if c.kind==JobKind::Build{"Gravedigger / build graveyard"}else{"Gravedigger"},"market":c.market,"reward":c.reward,"corpse":if c.corpse==NONE {None}else{Some(self.corpses[c.corpse as usize].agent)},"phase":if c.phase==0{"Collecting contract at board"}else if c.kind==JobKind::Build{"Constructing graveyard"}else if c.phase==1{"Retrieving body"}else{"Carrying body to graveyard"}})
    }
    pub(super) fn job_boards(&self) -> Vec<serde_json::Value> {
        self.councils.iter().enumerate().map(|(m,c)|serde_json::json!({"market":m,"name":self.markets[m].name,"x":self.markets[m].x+22.0,"y":self.markets[m].y+5.0,"farmer_openings":self.farms.iter().filter(|f|f.market as usize==m&&f.escrow>0).count(),"farmer_reward":2,"farmer_escrow":self.farms.iter().filter(|f|f.market as usize==m).map(|f|f.escrow).sum::<u64>(),"farm_count":self.farms.iter().filter(|f|f.market as usize==m).count(),"food_pressure":self.farm_food_pressure(m),"nearby_bodies":c.near_dead,"remote_bodies":c.remote_dead,"vacancies":c.vacancies,"reward":c.reward,"escrow":c.escrow,"builder_open":c.builder_open,"builder_reward":c.builder_escrow,"active_haulers":self.active_jobs(m,JobKind::Haul),"active_builders":self.active_jobs(m,JobKind::Build),"treasury":self.markets[m].coins,"income_per_second":c.income_rate,"expenses_per_second":c.expense_rate,"revenue_total":self.markets[m].revenue_total,"expenses_total":self.markets[m].expenses_total,"policy":c.last_action,"blocked":c.blocked,"genome_nodes":c.genome.nodes.len(),"genome_genes":c.genome.genes.len(),"policy_choices":c.trace,"sensors":COUNCIL_SENSORS.as_slice()})).collect()
    }
    pub(super) fn validate_jobs(&self) -> Result<(), String> {
        if self.councils.len() != self.markets.len() {
            return Err("Missing village brains".into());
        }
        let n = self.agents.len();
        let mut claimed = vec![false; self.corpses.len()];
        for (m, c) in self.councils.iter().enumerate() {
            c.genome.compile()?;
            if c.escrow != c.reward * c.vacancies as u64
                || (c.builder_open != (c.builder_escrow > 0))
                || (c.graveyard != NONE
                    && self
                        .graveyards
                        .get(c.graveyard as usize)
                        .is_none_or(|g| g.market as usize != m))
            {
                return Err("Invalid job board escrow".into());
            }
        }
        for (slot, c) in self.contracts.iter().enumerate() {
            if let Some(c) = c {
                let i = c.worker as usize;
                if i >= n
                    || c.market as usize >= self.markets.len()
                    || c.graveyard as usize >= self.graveyards.len()
                    || self.agents.contract[i] != slot as u32
                {
                    return Err("Invalid job contract".into());
                }
                if c.kind == JobKind::Haul {
                    let id = c.corpse as usize;
                    if id >= claimed.len()
                        || claimed[id]
                        || self.corpses[id].claim != c.worker
                        || self.corpses[id].status == 2
                    {
                        return Err("Duplicate corpse claim".into());
                    }
                    claimed[id] = true;
                }
            }
        }
        let mut buried = vec![false; self.corpses.len()];
        for g in &self.graveyards {
            if g.buried.len() > g.capacity as usize || g.market as usize >= self.markets.len() {
                return Err("Invalid graveyard".into());
            }
            for &id in &g.buried {
                if id as usize >= buried.len()
                    || buried[id as usize]
                    || self.corpses[id as usize].status != 2
                {
                    return Err("Invalid burial registry".into());
                }
                buried[id as usize] = true;
            }
        }
        for (id, b) in self.corpses.iter().enumerate() {
            if b.agent as usize >= n
                || self.agents.alive[b.agent as usize]
                || self.agents.corpse[b.agent as usize] != id as u32
                || b.market as usize >= self.markets.len()
                || b.status > 2
                || !b.x.is_finite()
                || !b.y.is_finite()
            {
                return Err("Invalid corpse record".into());
            }
            if (b.status == 2) != buried[id] || (b.claim != NONE) != claimed[id] {
                return Err("Orphan corpse state".into());
            }
        }
        for i in 0..n {
            if self.agents.contract[i] != NONE
                && self
                    .contracts
                    .get(self.agents.contract[i] as usize)
                    .and_then(Option::as_ref)
                    .is_none_or(|c| c.worker as usize != i)
            {
                return Err("Orphan worker contract".into());
            }
            if self.agents.carried_body[i] != NONE
                && self
                    .corpses
                    .get(self.agents.carried_body[i] as usize)
                    .is_none_or(|b| b.claim as usize != i || b.status != 1)
            {
                return Err("Orphan carried body".into());
            }
        }
        Ok(())
    }
}
