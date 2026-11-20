//! Zoned property, private firms and physically carried inter-building contracts.
use super::*;
use std::collections::BTreeMap;
pub const WARES: usize = 5;
pub const WARE_NAMES: [&str; WARES] = ["hides", "leather clothes", "shoes", "furniture", "meat"];
const WARE_WEIGHT: [u32; WARES] = [3, 2, 2, 8, 1];
const RETAIL: [u64; WARES] = [9, 18, 16, 30, 4];
const WHOLESALE: [u64; WARES] = [6, 11, 10, 19, 2];
pub const PERSONA_BIAS: [[f32; 6]; 4] = [
    [0.0; 6],
    [0.03, 0.04, 0.0, 0.0, 0.06, 0.0],
    [0.02, 0.02, -0.02, 0.04, 0.0, 0.05],
    [0.02, 0.03, -0.03, 0.0, 0.06, 0.0],
];
const PERSONA_INPUTS: [u32; 6] = [0, 1, 2, 8, 10, 9];
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Zone {
    pub market: u32,
    pub kind: u8,
    pub x: f32,
    pub y: f32,
    pub radius: f32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TownState {
    pub zoned: bool,
    pub persona: u8,
    pub last_eviction: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Building {
    pub market: u32,
    pub kind: u8,
    pub line: u8,
    pub x: f32,
    pub y: f32,
    pub ready: bool,
    pub progress: f32,
    pub needs_clear: bool,
    pub advertised: bool,
    pub owner: u32,
    pub renter: u32,
    pub price: u64,
    pub rent: u64,
    pub leased: u64,
    pub arrears: u64,
    pub last_rent: u64,
    pub shelter: u32,
    pub cash: u64,
    pub raw: [u64; GOODS],
    pub wares: [u64; WARES],
    pub made: u64,
    pub sold: u64,
    pub last_service: u64,
    pub last_open: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Job {
    pub market: u32,
    pub kind: u8,
    pub payer: u32,
    pub firm: u32,
    pub source: u32,
    pub target: u32,
    pub worker: u32,
    pub phase: u8,
    pub x: f32,
    pub y: f32,
    pub raw: [u64; GOODS],
    pub wares: [u64; WARES],
    pub reward: u64,
    pub progress: f32,
}
impl Job {
    fn weight(&self) -> u32 {
        (0..GOODS)
            .map(|k| self.raw[k] as u32 * WEIGHTS[k])
            .sum::<u32>()
            + (0..WARES)
                .map(|k| self.wares[k] as u32 * WARE_WEIGHT[k])
                .sum::<u32>()
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Progress {
    pub made: u64,
    pub hauled: u64,
    pub hunted: u64,
    pub sales: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Estate {
    pub owner: u32,
    pub x: f32,
    pub y: f32,
    pub cash: u64,
    pub raw: [u64; GOODS],
    pub wares: [u64; WARES],
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Urban {
    pub states: Vec<TownState>,
    pub zones: Vec<Zone>,
    pub buildings: Vec<Building>,
    pub jobs: Vec<Option<Job>>,
    pub assigned: Vec<u32>,
    pub owned: BTreeMap<(u32, u8), u64>,
    pub progress: BTreeMap<u32, Progress>,
    pub estates: Vec<Estate>,
    pub produced: [u64; WARES],
    pub consumed: [u64; WARES],
    pub cleared: Vec<(f32, f32, f32)>,
    pub evicted_until: BTreeMap<u32, u64>,
    pub posted_public: u64,
    pub posted_private: u64,
    pub completed: u64,
    pub evictions: u64,
    pub retail_sales: BTreeMap<(u32, u8), u64>,
}
impl World {
    fn coffers(&self, m: usize) -> u64 {
        self.markets[m].coins + self.markets[m].quotes.iter().map(|q| q.escrow).sum::<u64>()
    }
    pub(super) fn sync_urban(&mut self) {
        self.urban.assigned.resize(self.agents.len(), NONE);
        self.agents.urban_weight.resize(self.agents.len(), 0);
        while self.urban.states.len() < self.markets.len() {
            let m = self.urban.states.len();
            let t = &self.markets[m];
            let persona = if t.x > 2200.0 {
                2
            } else if self.hill.clearance(t.x, t.y) < 190.0 {
                3
            } else {
                1
            };
            self.urban.states.push(TownState {
                zoned: false,
                persona,
                last_eviction: 0,
            });
            self.apply_persona_delta(m, 0, persona);
        }
    }
    fn apply_persona_delta(&mut self, m: usize, old: u8, new: u8) {
        if let Some(c) = self.councils.get_mut(m) {
            for gene in &mut c.genome.genes {
                for (p, id) in PERSONA_INPUTS.iter().enumerate() {
                    if gene.from == *id {
                        gene.weight +=
                            PERSONA_BIAS[new as usize][p] - PERSONA_BIAS[old as usize][p];
                    }
                }
            }
            c.brain = c.genome.compile().ok();
        }
    }
    pub fn set_village_persona(&mut self, m: usize, persona: u8) -> Result<(), String> {
        if m >= self.markets.len() || persona > 3 {
            return Err("Choose a village and profile 0..3".into());
        }
        let old = self.urban.states[m].persona;
        self.apply_persona_delta(m, old, persona);
        self.urban.states[m].persona = persona;
        Ok(())
    }
    pub(super) fn urban_progress(&self, i: usize) -> Progress {
        self.urban
            .progress
            .get(&(i as u32))
            .cloned()
            .unwrap_or_default()
    }
    pub(super) fn owns_urban_home(&self, i: usize) -> bool {
        self.urban
            .buildings
            .iter()
            .any(|b| b.kind == 0 && b.owner == i as u32)
    }
    pub(super) fn owns_business(&self, i: usize) -> bool {
        self.urban
            .buildings
            .iter()
            .any(|b| b.kind > 0 && b.owner == i as u32)
    }
    fn business(&self, i: usize) -> Option<usize> {
        self.urban
            .buildings
            .iter()
            .position(|b| b.kind > 0 && b.owner == i as u32 && b.ready)
    }
    fn home_property(&self, i: usize) -> Option<usize> {
        self.urban
            .buildings
            .iter()
            .position(|b| b.kind == 0 && (b.owner == i as u32 || b.renter == i as u32))
    }
    pub(super) fn is_town(&self, m: usize) -> bool {
        (0..3).all(|k| {
            self.urban
                .buildings
                .iter()
                .filter(|b| b.market as usize == m && b.kind == k && b.ready)
                .count()
                >= 3
        })
    }
    pub(super) fn urban_coins(&self) -> u64 {
        self.urban.buildings.iter().map(|b| b.cash).sum::<u64>()
            + self
                .urban
                .jobs
                .iter()
                .flatten()
                .map(|j| j.reward)
                .sum::<u64>()
            + self.urban.estates.iter().map(|e| e.cash).sum::<u64>()
    }
    pub(super) fn urban_raw_stocks(&self) -> [u64; GOODS] {
        let mut s = [0; GOODS];
        for b in &self.urban.buildings {
            for k in 0..GOODS {
                s[k] += b.raw[k];
            }
        }
        for j in self.urban.jobs.iter().flatten() {
            for k in 0..GOODS {
                s[k] += j.raw[k];
            }
        }
        for e in &self.urban.estates {
            for k in 0..GOODS {
                s[k] += e.raw[k];
            }
        }
        s
    }
    fn ware_stocks(&self) -> [u64; WARES] {
        let mut s = [0; WARES];
        for b in &self.urban.buildings {
            for k in 0..WARES {
                s[k] += b.wares[k];
            }
        }
        for j in self.urban.jobs.iter().flatten() {
            for k in 0..WARES {
                s[k] += j.wares[k];
            }
        }
        for e in &self.urban.estates {
            for k in 0..WARES {
                s[k] += e.wares[k];
            }
        }
        for (&(_, k), q) in &self.urban.owned {
            s[k as usize] += q;
        }
        s
    }
    pub(super) fn can_use_urban_shelter(&self, i: usize, site: usize) -> bool {
        if let Some(b) = self
            .urban
            .buildings
            .iter()
            .find(|b| b.kind == 0 && b.shelter == site as u32)
        {
            let tenant = if b.owner != NONE { b.owner } else { b.renter };
            if tenant == NONE {
                return false;
            }
            let p = &self.people[i];
            return tenant == i as u32
                || p.spouse == tenant
                || !self.is_adult(i) && (p.father == tenant || p.mother == tenant);
        }
        true
    }
    pub(super) fn urban_aspiration_aligns(&self, i: usize, a: Action) -> bool {
        match self.people[i].aspiration {
            life::Aspiration::Homeowner => a == Action::BuyHome,
            life::Aspiration::Shopkeeper => matches!(
                a,
                Action::LeaseBusiness
                    | Action::OperateBusiness
                    | Action::OrderGoods
                    | Action::BorrowBusiness
            ),
            life::Aspiration::Artisan => {
                matches!(a, Action::LeaseBusiness | Action::OperateBusiness)
            }
            life::Aspiration::Hunter => a == Action::Hunt,
            life::Aspiration::Haulier => a == Action::UrbanWork,
            _ => false,
        }
    }
    pub(super) fn is_urban_action(&self, a: Action) -> bool {
        matches!(
            a,
            Action::UrbanWork
                | Action::LeaseBusiness
                | Action::OperateBusiness
                | Action::OrderGoods
                | Action::BuyHome
                | Action::RentHome
                | Action::ShopGoods
                | Action::Hunt
                | Action::CollectEstate
                | Action::BuildBank
                | Action::BorrowBusiness
                | Action::RepayLoan
        )
    }
    pub(super) fn urban_destination(&self, i: usize) -> Option<(f32, f32)> {
        let a = &self.agents;
        let target = a.target[i] as usize;
        if self.is_bank_action(a.action[i]) {
            return self.banking.banks.get(target).map(|b| (b.x, b.y));
        }
        if a.action[i] == Action::CollectEstate {
            return self.urban.estates.get(target).map(|e| (e.x, e.y));
        }
        if a.action[i] == Action::UrbanWork {
            let j = self.urban.jobs.get(target)?.as_ref()?;
            if j.kind == 1 || j.kind == 2 || j.kind == 3 {
                if j.phase == 0 {
                    return Some((j.x, j.y));
                }
                if j.target == NONE {
                    let m = &self.markets[j.market as usize];
                    return Some((m.x, m.y));
                }
                let b = self.urban.buildings.get(j.target as usize)?;
                return Some((b.x, b.y));
            }
            let b = self.urban.buildings.get(j.target as usize)?;
            return Some((b.x, b.y));
        }
        if a.action[i] == Action::Hunt {
            return self.wildlife.animals.get(target).map(|a| (a.x, a.y));
        }
        let id = if a.action[i] == Action::ShopGoods {
            target / 32
        } else {
            target
        };
        self.urban.buildings.get(id).map(|b| (b.x, b.y))
    }
    fn post_urban_job(&mut self, j: Job) -> usize {
        let private = j.payer != NONE;
        if private {
            self.urban.posted_private += 1
        } else {
            self.urban.posted_public += 1
        }
        if let Some(id) = self.urban.jobs.iter().position(Option::is_none) {
            self.urban.jobs[id] = Some(j);
            id
        } else {
            self.urban.jobs.push(Some(j));
            self.urban.jobs.len() - 1
        }
    }
    fn new_job(
        &self,
        m: usize,
        kind: u8,
        payer: u32,
        firm: u32,
        source: u32,
        target: u32,
        x: f32,
        y: f32,
        reward: u64,
    ) -> Job {
        Job {
            market: m as u32,
            kind,
            payer,
            firm,
            source,
            target,
            worker: NONE,
            phase: 0,
            x,
            y,
            raw: [0; GOODS],
            wares: [0; WARES],
            reward,
            progress: 0.0,
        }
    }
    fn has_job(&self, id: usize) -> bool {
        self.urban
            .jobs
            .iter()
            .flatten()
            .any(|j| j.firm == id as u32 || j.target == id as u32 || j.source == id as u32)
    }
    pub(super) fn operator_vacancy_pressure(&self, m: usize) -> f32 {
        let premises = self
            .urban
            .buildings
            .iter()
            .filter(|b| b.market as usize == m && b.ready && b.kind > 0)
            .collect::<Vec<_>>();
        if premises.is_empty() {
            return 0.0;
        }
        premises.iter().filter(|b| b.owner == NONE).count() as f32 / premises.len() as f32
    }
    fn operator_role(b: &Building) -> &'static str {
        if b.kind == 1 {
            [
                "Clothes shopkeeper",
                "Tool shopkeeper",
                "Furniture shopkeeper",
                "Butcher shopkeeper",
            ][b.line as usize]
        } else {
            ["Hunter", "Cobbler", "Blacksmith", "Carpenter"][b.line as usize]
        }
    }
    fn operator_job_postings(&self) -> Vec<serde_json::Value> {
        self.urban.buildings.iter().enumerate().filter(|(_,b)|b.kind>0 && b.ready && b.owner==NONE && b.advertised)
            .map(|(id,b)|serde_json::json!({"building":id,"market":b.market,"title":Self::operator_role(b),"zone":if b.kind==1{"Commercial"}else{"Industrial"},"x":b.x,"y":b.y,"rent_per_day":b.rent,"entry_fee":8,"working_capital":50,"required_coins":58,"marketplace_pay_per_day":if b.kind==1{2}else{0},"income":"Sales income; marketplace shift pay only when funded","action":"LeaseBusiness"})).collect()
    }
    pub(super) fn urban_council_candidates(&self, m: usize) -> Vec<(&'static str, u64, f32)> {
        let mut v = vec![];
        let s = &self.urban.states[m];
        let t = &self.markets[m];
        if !t.village {
            return v;
        }
        if !s.zoned && self.coffers(m) >= 500 {
            v.push(("GazetteDistricts", 0, 0.9));
        }
        if s.zoned {
            for (k, name, wood, stone, wage) in [
                (0, "CommissionHomes", 12, 4, 16),
                (1, "CommissionShops", 14, 6, 20),
                (2, "CommissionWorkshops", 18, 8, 24),
            ] {
                if self
                    .urban
                    .buildings
                    .iter()
                    .filter(|b| b.market as usize == m && b.kind == k)
                    .count()
                    < 8
                    && t.stock(1) >= wood
                    && t.stock(2) >= stone
                    && t.coins >= wage + 100
                    && !self
                        .urban
                        .buildings
                        .iter()
                        .any(|b| b.market as usize == m && b.kind == k && !b.ready)
                {
                    v.push((
                        name,
                        wage,
                        if self
                            .urban
                            .buildings
                            .iter()
                            .filter(|b| b.market as usize == m && b.kind == k && b.ready)
                            .count()
                            < 3
                        {
                            1.0
                        } else {
                            0.5
                        },
                    ));
                }
            }
            if self.urban.buildings.iter().any(|b| {
                b.market as usize == m && b.ready && b.kind > 0 && b.owner == NONE && !b.advertised
            }) {
                v.push((
                    "AdvertisePremises",
                    0,
                    0.75 + 0.25 * self.operator_vacancy_pressure(m),
                ));
            }
            if t.coins >= 8
                && self.urban.buildings.iter().enumerate().any(|(id, b)| {
                    b.market as usize == m
                        && b.needs_clear
                        && !self.urban.jobs.iter().flatten().any(|j| {
                            j.kind == 2 && (j.target == id as u32 || j.source == id as u32)
                        })
                })
            {
                v.push(("ClearForDistrict", 8, 0.75));
            }
            if self.coffers(m) < 100
                && self.tick.saturating_sub(s.last_eviction) > rest::DAY_TICKS
                && self.urban.buildings.iter().enumerate().any(|(id, b)| {
                    b.market as usize == m
                        && b.kind == 1
                        && b.owner != NONE
                        && self.tick.saturating_sub(b.leased) > rest::DAY_TICKS
                        && !self.has_job(id)
                })
            {
                v.push(("ReviewShopLeases", 0, 0.5));
            }
            if t.coins >= 8 && self.unused_camp(m).is_some() {
                v.push(("ClearUnusedShelter", 8, 0.45));
            }
        }
        v
    }
    pub(super) fn apply_urban_policy(&mut self, m: usize, action: &str) {
        match action {
            "GazetteDistricts" => {
                if self.coffers(m) < 500 || self.urban.states[m].zoned {
                    return;
                }
                self.urban.states[m].zoned = true;
                let t = &self.markets[m];
                for (kind, dx, dy, radius) in [
                    (0, -180.0, 130.0, 110.0),
                    (1, 140.0, 35.0, 90.0),
                    (2, 40.0, -210.0, 120.0),
                ] {
                    self.urban.zones.push(Zone {
                        market: m as u32,
                        kind,
                        x: (t.x + dx).clamp(140.0, 2520.0),
                        y: (t.y + dy).clamp(140.0, 1760.0),
                        radius,
                    });
                }
            }
            "CommissionHomes" => self.commission_urban(m, 0),
            "CommissionShops" => self.commission_urban(m, 1),
            "CommissionWorkshops" => self.commission_urban(m, 2),
            "AdvertisePremises" => {
                for b in &mut self.urban.buildings {
                    if b.market as usize == m && b.ready && b.kind > 0 && b.owner == NONE {
                        b.advertised = true;
                    }
                }
            }
            "ClearForDistrict" => {
                if self.markets[m].coins < 8 {
                    return;
                }
                if let Some((id, b)) = self.urban.buildings.iter().enumerate().find(|(id, b)| {
                    b.market as usize == m
                        && b.needs_clear
                        && !self.urban.jobs.iter().flatten().any(|j| {
                            j.kind == 2 && (j.target == *id as u32 || j.source == *id as u32)
                        })
                }) {
                    let (x, y) = (b.x, b.y);
                    let j = self.new_job(m, 2, NONE, NONE, NONE, id as u32, x, y, 8);
                    self.markets[m].coins -= 8;
                    self.post_urban_job(j);
                }
            }
            "ReviewShopLeases" => {
                if self.coffers(m) < 100 {
                    if let Some(id) = self
                        .urban
                        .buildings
                        .iter()
                        .enumerate()
                        .filter(|(id, b)| {
                            b.market as usize == m
                                && b.kind == 1
                                && b.owner != NONE
                                && self.tick.saturating_sub(b.leased) > rest::DAY_TICKS
                                && !self.has_job(*id)
                        })
                        .max_by_key(|(_, b)| b.arrears)
                        .map(|(id, _)| id)
                    {
                        self.vacate_business(id);
                        self.urban.states[m].last_eviction = self.tick;
                        self.urban.evictions += 1;
                    }
                }
            }
            "ClearUnusedShelter" => {
                if self.markets[m].coins < 8 {
                    return;
                }
                if let Some(site) = self.unused_camp(m) {
                    let s = &self.shelters[site];
                    let j = self.new_job(m, 3, NONE, NONE, NONE, site as u32, s.x, s.y, 8);
                    self.markets[m].coins -= 8;
                    self.post_urban_job(j);
                }
            }
            _ => {}
        }
    }
    fn unused_camp(&self, m: usize) -> Option<usize> {
        let t = &self.markets[m];
        self.shelters
            .iter()
            .enumerate()
            .find(|(id, s)| {
                s.settlement.is_none()
                    && !s.beds.is_empty()
                    && s.beds.iter().all(|&i| i == NONE)
                    && dist(s.x, s.y, t.x, t.y) < 500.0
                    && !self
                        .agents
                        .target
                        .iter()
                        .zip(&self.agents.action)
                        .any(|(&target, &a)| a == Action::Rest && target == *id as u32)
                    && !self
                        .urban
                        .jobs
                        .iter()
                        .flatten()
                        .any(|j| j.kind == 3 && j.target == *id as u32)
            })
            .map(|(id, _)| id)
    }
    pub(super) fn site_available(&self, x: f32, y: f32) -> bool {
        if !(30.0..=2650.0).contains(&x)
            || !(30.0..=1870.0).contains(&y)
            || self
                .urban
                .buildings
                .iter()
                .any(|b| dist(x, y, b.x, b.y) < 33.0)
            || self
                .banking
                .banks
                .iter()
                .any(|b| dist(x, y, b.x, b.y) < 38.0)
            || self.taverns.iter().any(|b| dist(x, y, b.x, b.y) < 38.0)
            || self
                .education
                .schools
                .iter()
                .any(|b| dist(x, y, b.x, b.y) < 38.0)
            || self.farms.iter().any(|b| dist(x, y, b.x, b.y) < 42.0)
            || self.graveyards.iter().any(|b| dist(x, y, b.x, b.y) < 55.0)
            || self.markets.iter().any(|m| dist(x, y, m.x, m.y) < 55.0)
        {
            return false;
        }
        for m in &self.markets {
            for house in 0..m.housing_blocks.max(m.population.div_ceil(32)) + 4 {
                let (hx, hy) = rest::house_origin(house);
                if dist(
                    x,
                    y,
                    (m.x + hx).clamp(5.0, 2670.0) + 10.0,
                    (m.y + hy).clamp(5.0, 1900.0) + 7.5,
                ) < 27.0
                {
                    return false;
                }
            }
        }
        true
    }
    fn commission_urban(&mut self, m: usize, kind: u8) {
        let (wood, stone, wage) = match kind {
            0 => (12, 4, 16),
            1 => (14, 6, 20),
            _ => (18, 8, 24),
        };
        if !self.urban.states[m].zoned
            || self.markets[m].stock(1) < wood
            || self.markets[m].stock(2) < stone
            || self.markets[m].coins < wage + 100
        {
            return;
        }
        let count = self
            .urban
            .buildings
            .iter()
            .filter(|b| b.market as usize == m && b.kind == kind)
            .count();
        if count >= 8
            || self
                .urban
                .buildings
                .iter()
                .any(|b| b.market as usize == m && b.kind == kind && !b.ready)
        {
            return;
        }
        let Some(z) = self
            .urban
            .zones
            .iter()
            .find(|z| z.market as usize == m && z.kind == kind)
            .cloned()
        else {
            return;
        };
        let pos = (-3..=3)
            .flat_map(|a| (-3..=3).map(move |b| (z.x + a as f32 * 35.0, z.y + b as f32 * 35.0)))
            .filter(|(x, y)| dist(*x, *y, z.x, z.y) <= z.radius && self.site_available(*x, *y))
            .min_by(|a, b| {
                let cost = |(x, y): (f32, f32)| {
                    dist(x, y, z.x, z.y)
                        + if self
                            .wildlife
                            .forests
                            .iter()
                            .any(|f| dist(x, y, f.x, f.y) < f.radius + 20.0)
                        {
                            500.0
                        } else {
                            0.0
                        }
                };
                cost(*a).total_cmp(&cost(*b))
            });
        let Some((x, y)) = pos else {
            return;
        };
        for (k, q) in [(1, wood), (2, stone)] {
            let mut left = q;
            while left > 0 {
                left -= self.markets[m]
                    .take(k, left)
                    .expect("reserved building materials")
                    .quantity;
            }
            self.markets[m].quotes[k].offered = self.markets[m].quotes[k]
                .offered
                .min(self.markets[m].stock(k));
            self.ledger.built[k] += q;
        }
        let cleared = self
            .urban
            .cleared
            .iter()
            .any(|(cx, cy, r)| dist(x, y, *cx, *cy) < *r);
        let needs_clear = !cleared
            && self
                .wildlife
                .forests
                .iter()
                .any(|f| dist(x, y, f.x, f.y) < f.radius + 15.0);
        let id = self.urban.buildings.len();
        self.urban.buildings.push(Building {
            market: m as u32,
            kind,
            line: if kind == 2 {
                count as u8 % 4
            } else {
                count as u8 % 4
            },
            x,
            y,
            ready: false,
            progress: 0.0,
            needs_clear,
            advertised: false,
            owner: NONE,
            renter: NONE,
            price: 120,
            rent: if kind == 0 {
                3
            } else if kind == 1 {
                3
            } else {
                2
            },
            leased: 0,
            arrears: 0,
            last_rent: self.tick / rest::DAY_TICKS,
            shelter: NONE,
            cash: 0,
            raw: [0; GOODS],
            wares: [0; WARES],
            made: 0,
            sold: 0,
            last_service: u64::MAX,
            last_open: 0,
        });
        self.markets[m].coins -= wage;
        let j = self.new_job(m, 0, NONE, NONE, NONE, id as u32, x, y, wage);
        self.post_urban_job(j);
    }
    fn urban_candidate(
        &self,
        i: usize,
        action: Action,
        target: usize,
        kind: usize,
        need: f32,
        benefit: f32,
        price: f32,
        x: f32,
        y: f32,
    ) -> Candidate {
        let mut inputs = [0.0; INPUTS];
        inputs[0] = need;
        inputs[1] = benefit;
        inputs[2] = if action == Action::Hunt { 0.22 } else { 0.03 };
        inputs[3] = dist(self.agents.x[i], self.agents.y[i], x, y) / 1000.0;
        inputs[4] = if action == Action::UrbanWork {
            price.min(8.0)
        } else {
            -price.min(8.0)
        };
        inputs[11] = 0.8;
        Candidate {
            action,
            target: target as u32,
            kind,
            score: self.brains[i].evaluate(&inputs),
            inputs,
        }
    }
    pub(super) fn urban_committed_offer(&self, i: usize) -> Option<Candidate> {
        if !self.is_adult(i) || self.must_stay_home(i) || self.agents.energy[i] < 1.0 {
            return None;
        }
        let id = *self.urban.assigned.get(i)?;
        let j = self.urban.jobs.get(id as usize)?.as_ref()?;
        if j.worker != i as u32 {
            return None;
        }
        let mut c = self.urban_candidate(
            i,
            Action::UrbanWork,
            id as usize,
            1,
            1.0,
            1.0,
            j.reward as f32,
            j.x,
            j.y,
        );
        c.inputs[4] /= 8.0;
        self.personal_inputs(i, c.action, c.kind, &mut c.inputs);
        c.score = self.score_action(i, c.action, c.target, &c.inputs);
        Some(c)
    }
    pub(super) fn urban_offers(&self, i: usize) -> Vec<Candidate> {
        let mut out = vec![];
        if !self.is_adult(i)
            || self.must_stay_home(i)
            || self.agents.cargo[i] > 0
            || self.agents.carried_body[i] != NONE
            || self.agents.contract[i] != NONE
            || self.urban.assigned.get(i).copied().unwrap_or(NONE) != NONE
        {
            return out;
        }
        out.extend(self.bank_offers(i));
        let a = &self.agents;
        let m = a.home[i] as usize;
        if let Some((id, e)) = self.urban.estates.iter().enumerate().find(|(_, e)| {
            e.owner == i as u32
                && (e.cash > 0
                    || e.raw.iter().sum::<u64>() > 0
                    || e.wares.iter().sum::<u64>() > 0 && self.business(i).is_some())
        }) {
            out.push(self.urban_candidate(
                i,
                Action::CollectEstate,
                id,
                1,
                0.9,
                0.95,
                0.0,
                e.x,
                e.y,
            ));
        }
        if let Some((id, j)) = self
            .urban
            .jobs
            .iter()
            .enumerate()
            .filter_map(|(id, j)| j.as_ref().map(|j| (id, j)))
            .filter(|(_, j)| {
                j.worker == NONE
                    && j.weight().max(if j.kind == 2 { 8 } else { 0 }) <= a.room(i)
                    && !(j.kind == 0 && self.urban.buildings[j.target as usize].needs_clear)
            })
            .min_by(|(_, u), (_, v)| {
                dist(a.x[i], a.y[i], u.x, u.y).total_cmp(&dist(a.x[i], a.y[i], v.x, v.y))
            })
        {
            out.push(self.urban_candidate(
                i,
                Action::UrbanWork,
                id,
                1,
                0.8,
                0.9,
                j.reward as f32,
                j.x,
                j.y,
            ));
        }
        if !self.owns_urban_home(i) && self.home_property(i).is_none() {
            if let Some((id, b)) = self
                .urban
                .buildings
                .iter()
                .enumerate()
                .filter(|(_, b)| {
                    b.market as usize == m
                        && b.kind == 0
                        && b.ready
                        && b.owner == NONE
                        && b.renter == NONE
                })
                .min_by(|(_, u), (_, v)| {
                    dist(a.x[i], a.y[i], u.x, u.y).total_cmp(&dist(a.x[i], a.y[i], v.x, v.y))
                })
            {
                if a.coins[i] >= b.price {
                    out.push(self.urban_candidate(
                        i,
                        Action::BuyHome,
                        id,
                        0,
                        0.65,
                        0.8,
                        b.price as f32,
                        b.x,
                        b.y,
                    ));
                }
                if a.coins[i] >= b.rent * 3 {
                    out.push(self.urban_candidate(
                        i,
                        Action::RentHome,
                        id,
                        0,
                        0.45,
                        0.6,
                        b.rent as f32,
                        b.x,
                        b.y,
                    ));
                }
            }
        }
        if !self.owns_business(i)
            && a.coins[i] >= 58
            && self
                .urban
                .evicted_until
                .get(&(i as u32))
                .copied()
                .unwrap_or(0)
                <= self.tick
        {
            for (id, b) in self
                .urban
                .buildings
                .iter()
                .enumerate()
                .filter(|(_, b)| {
                    b.market as usize == m
                        && b.kind > 0
                        && b.ready
                        && b.advertised
                        && b.owner == NONE
                })
                .take(5)
            {
                let fit = match self.people[i].aspiration {
                    life::Aspiration::Hunter => b.kind == 2 && b.line == 0,
                    life::Aspiration::Artisan => b.kind == 2 && b.line != 0,
                    life::Aspiration::Shopkeeper => b.kind == 1,
                    _ => false,
                };
                out.push(self.urban_candidate(
                    i,
                    Action::LeaseBusiness,
                    id,
                    1,
                    if fit { 0.95 } else { 0.3 },
                    0.65,
                    58.0,
                    b.x,
                    b.y,
                ));
            }
        }
        if let Some(id) = self.business(i) {
            let b = &self.urban.buildings[id];
            let order_available = self.procurement(id).is_some();
            let empty = cfg!(feature = "experimental-restocking")
                && order_available
                && b.kind == 1
                && Self::catalog(b.line)
                    .into_iter()
                    .all(|sku| Self::inventory(b, sku) == 0);
            if b.kind == 1 || self.recipe(id).is_some() {
                out.push(self.urban_candidate(
                    i,
                    Action::OperateBusiness,
                    id,
                    1,
                    0.85,
                    if empty { 0.1 } else { 0.9 },
                    0.0,
                    b.x,
                    b.y,
                ));
            }
            if order_available {
                out.push(self.urban_candidate(
                    i,
                    Action::OrderGoods,
                    id,
                    1,
                    0.9,
                    if empty { 1.0 } else { 0.85 },
                    2.0,
                    b.x,
                    b.y,
                ));
            }
            if b.kind == 2 && b.line == 0 && b.wares[0] < 5 && b.cash >= 2 && a.room(i) >= 5 {
                if let Some(prey) = self.hunt_target(i) {
                    let p = &self.wildlife.animals[prey];
                    out.push(self.urban_candidate(
                        i,
                        Action::Hunt,
                        prey,
                        0,
                        0.65,
                        0.8,
                        0.0,
                        p.x,
                        p.y,
                    ));
                }
            }
        }
        for (id, b) in self.urban.buildings.iter().enumerate().filter(|(_, b)| {
            b.kind == 1 && b.ready && b.owner != NONE && self.tick.saturating_sub(b.last_open) < 30
        }) {
            for sku in Self::catalog(b.line) {
                let available = Self::inventory(b, sku);
                if available == 0 {
                    continue;
                }
                let price = Self::retail_price(sku);
                if a.coins[i] < price {
                    continue;
                }
                let need = if sku < 13 {
                    if a.tools[i][sku - 9] > 0 {
                        continue;
                    }
                    0.4
                } else {
                    let k = sku - 13;
                    match k {
                        1 | 2 => {
                            if self
                                .urban
                                .owned
                                .get(&(i as u32, k as u8))
                                .copied()
                                .unwrap_or(0)
                                > 0
                                || a.room(i) < WARE_WEIGHT[k]
                            {
                                continue;
                            }
                            0.45
                        }
                        3 => {
                            let Some(home) = self.home_property(i) else {
                                continue;
                            };
                            if self.urban.buildings[home].wares[3] > 0
                                || self
                                    .urban
                                    .jobs
                                    .iter()
                                    .flatten()
                                    .any(|j| j.target == home as u32 && j.wares[3] > 0)
                                || a.coins[i] < price + 2
                            {
                                continue;
                            }
                            0.5
                        }
                        4 => {
                            if a.hunger[i] < 30.0 {
                                continue;
                            }
                            a.hunger[i] / 70.0
                        }
                        _ => continue,
                    }
                };
                if sku < 13 && a.room(i) < WEIGHTS[sku] {
                    continue;
                }
                out.push(self.urban_candidate(
                    i,
                    Action::ShopGoods,
                    id * 32 + sku,
                    if sku < 13 { sku } else { 0 },
                    need,
                    0.55,
                    price as f32,
                    b.x,
                    b.y,
                ));
                if out.len() > 12 {
                    break;
                }
            }
            if out.len() > 12 {
                break;
            }
        }
        out
    }
    /// Read-only pre-tick availability probe. Shop counters count each premises once,
    /// even when several catalog products pass. urban_candidates counts action/target pairs.
    pub fn retail_supply_probe(&self, actor: Option<usize>) -> [(&'static str, u64); 13] {
        let mut ready_count = 0u64;
        let mut occupied_count = 0u64;
        let mut open_count = 0u64;
        let mut stocked_count = 0u64;
        let mut open_stocked_count = 0u64;
        let mut affordable_count = 0u64;
        let mut fits_count = 0u64;
        let mut urban_candidates_count = 0u64;
        let mut can_fund_order_count = 0u64;
        let mut matching_supplier_count = 0u64;
        let mut funded_supplier_count = 0u64;
        let mut pending_firm_job_count = 0u64;
        let mut order_available_count = 0u64;
        for (id, b) in self
            .urban
            .buildings
            .iter()
            .enumerate()
            .filter(|(_, b)| b.kind == 1 && b.ready)
        {
            ready_count += 1;
            if b.owner == NONE {
                continue;
            }
            occupied_count += 1;
            let open = self.tick.saturating_sub(b.last_open) < 30;
            let mut stocked = false;
            let mut affordable = false;
            let mut fits = false;
            for sku in Self::catalog(b.line) {
                if Self::inventory(b, sku) == 0 {
                    continue;
                }
                stocked = true;
                if let Some(i) = actor {
                    let delivery = if sku == 16 { 2 } else { 0 };
                    if self.agents.coins[i] >= Self::retail_price(sku) + delivery {
                        affordable = true;
                        fits |= match sku {
                            9..=12 => self.agents.room(i) >= WEIGHTS[sku],
                            14 | 15 => self.agents.room(i) >= WARE_WEIGHT[sku - 13],
                            _ => true, // furniture is delivered, meat is consumed
                        };
                    }
                }
            }
            let skus = Self::catalog(b.line);
            let can_fund = |sku| b.cash >= Self::wholesale_price(sku) + 2;
            let supplied = |sku| {
                self.urban.buildings.iter().any(|f| {
                    f.kind == 2 && f.ready && f.owner != NONE && Self::inventory(f, sku) > 0
                })
            };
            can_fund_order_count += u64::from(skus.iter().any(|&sku| can_fund(sku)));
            matching_supplier_count += u64::from(skus.iter().any(|&sku| supplied(sku)));
            funded_supplier_count +=
                u64::from(skus.iter().any(|&sku| can_fund(sku) && supplied(sku)));
            pending_firm_job_count += u64::from(
                self.urban
                    .jobs
                    .iter()
                    .flatten()
                    .any(|j| j.firm == id as u32),
            );
            order_available_count += u64::from(self.procurement(id).is_some());
            open_count += u64::from(open);
            stocked_count += u64::from(stocked);
            open_stocked_count += u64::from(open && stocked);
            affordable_count += u64::from(open && affordable);
            fits_count += u64::from(open && fits);
        }
        if let Some(i) = actor.filter(|_| fits_count > 0) {
            urban_candidates_count = self
                .urban_offers(i)
                .iter()
                .filter(|c| c.action == Action::ShopGoods)
                .count() as u64;
        }
        [
            ("ready", ready_count),
            ("occupied", occupied_count),
            ("open", open_count),
            ("stocked", stocked_count),
            ("open_stocked", open_stocked_count),
            ("affordable", affordable_count),
            ("fits", fits_count),
            ("urban_candidates", urban_candidates_count),
            ("can_fund_order", can_fund_order_count),
            ("matching_supplier", matching_supplier_count),
            ("funded_supplier", funded_supplier_count),
            ("pending_firm_job", pending_firm_job_count),
            ("order_available", order_available_count),
        ]
    }
    fn catalog(line: u8) -> Vec<usize> {
        match line {
            0 => vec![14, 15],
            1 => vec![9, 10, 11, 12],
            2 => vec![16],
            _ => vec![17],
        }
    }
    fn inventory(b: &Building, sku: usize) -> u64 {
        if sku < 13 {
            b.raw[sku]
        } else {
            b.wares[sku - 13]
        }
    }
    fn retail_price(sku: usize) -> u64 {
        if sku < 13 {
            18
        } else {
            RETAIL[sku - 13]
        }
    }
    fn wholesale_price(sku: usize) -> u64 {
        if sku < 13 {
            10
        } else {
            WHOLESALE[sku - 13]
        }
    }
    fn recipe(&self, id: usize) -> Option<(usize, u64, usize)> {
        let b = &self.urban.buildings[id];
        if b.kind != 2 || b.owner == NONE || !b.ready {
            return None;
        }
        match b.line {
            1 => {
                let output = if b.wares[1] < 3 { 1 } else { 2 };
                (b.wares[0] > 0 && b.wares[output] < 4).then_some((13, 1, 13 + output))
            }
            2 => {
                let output = 9 + (b.made as usize % 4);
                let iron = if output == 10 { 3 } else { 2 };
                (b.raw[7] >= iron && b.raw[1] > 0 && b.raw[9..13].iter().sum::<u64>() < 8)
                    .then_some((7, iron, output))
            }
            3 => (b.raw[1] >= 3 && b.wares[3] < 4).then_some((1, 3, 16)),
            _ => None,
        }
    }
    // Supplier None is the actual central market; otherwise the parcel starts at another firm.
    fn procurement(&self, id: usize) -> Option<(Option<usize>, usize, u64)> {
        let b = &self.urban.buildings[id];
        if b.owner == NONE
            || self
                .urban
                .jobs
                .iter()
                .flatten()
                .any(|j| j.firm == id as u32)
        {
            return None;
        }
        let raw = if b.kind == 2 {
            match b.line {
                2 => {
                    if b.raw[7] < 3 {
                        Some((7, 3))
                    } else if b.raw[1] < 2 {
                        Some((1, 2))
                    } else {
                        None
                    }
                }
                3 => (b.raw[1] < 3).then_some((1, 3)),
                _ => None,
            }
        } else {
            None
        };
        if let Some((k, q)) = raw {
            let m = &self.markets[b.market as usize];
            if m.stock(k) >= q && b.cash >= m.quotes[k].ask.max(1) * q + 2 {
                return Some((None, k, q));
            }
        }
        let skus = if b.kind == 1 {
            Self::catalog(b.line)
        } else if b.line == 1 {
            vec![13]
        } else {
            vec![]
        };
        for sku in skus {
            if Self::inventory(b, sku) >= 2 {
                continue;
            }
            let price = Self::wholesale_price(sku);
            if b.cash < price + 2 {
                continue;
            }
            if let Some((seller, _)) = self
                .urban
                .buildings
                .iter()
                .enumerate()
                .filter(|(other, s)| {
                    *other != id
                        && s.kind == 2
                        && s.ready
                        && s.owner != NONE
                        && Self::inventory(s, sku) > 0
                })
                .min_by(|(_, a), (_, c)| {
                    dist(a.x, a.y, b.x, b.y).total_cmp(&dist(c.x, c.y, b.x, b.y))
                })
            {
                return Some((Some(seller), sku, 1));
            }
        }
        None
    }
    fn place_procurement(&mut self, i: usize, id: usize) {
        if self
            .urban
            .buildings
            .get(id)
            .is_none_or(|b| b.owner != i as u32)
        {
            return;
        }
        let Some((source, sku, q)) = self.procurement(id) else {
            self.commitments
                .resolve(i, Action::OrderGoods, id as u32, 1, self.tick, false);
            return;
        };
        let m = self.urban.buildings[id].market as usize;
        let price = if source.is_none() {
            self.markets[m].quotes[sku].ask.max(1)
        } else {
            Self::wholesale_price(sku)
        };
        let cost = price * q;
        if self.urban.buildings[id].cash < cost + 2 {
            return;
        }
        let (x, y) = source
            .map(|s| (self.urban.buildings[s].x, self.urban.buildings[s].y))
            .unwrap_or((self.markets[m].x, self.markets[m].y));
        let mut job = self.new_job(
            m,
            1,
            i as u32,
            id as u32,
            source.map(|s| s as u32).unwrap_or(NONE),
            id as u32,
            x,
            y,
            2,
        );
        if let Some(s) = source {
            if sku < 13 {
                self.urban.buildings[s].raw[sku] -= q;
            } else {
                self.urban.buildings[s].wares[sku - 13] -= q;
            }
            self.urban.buildings[s].cash += cost;
            self.urban.buildings[s].sold += q;
            let seller = self.urban.buildings[s].owner;
            self.urban.progress.entry(seller).or_default().sales += q;
        } else {
            let mut left = q;
            while left > 0 {
                let lot = self.markets[m]
                    .take(sku, left)
                    .expect("available wholesale stock");
                left -= lot.quantity;
                if lot.origin == m as u32 {
                    self.markets[m].local_sales[sku] += lot.quantity
                } else {
                    self.markets[m].imported_sales[sku] += lot.quantity
                }
            }
            self.markets[m].coins += cost;
            self.markets[m].revenue_total += cost;
            self.markets[m].sales[sku] += q;
            self.markets[m].quotes[sku].offered = self.markets[m].quotes[sku]
                .offered
                .min(self.markets[m].stock(sku));
        }
        if self.urban.buildings[id].kind == 1 {
            self.watch.retail_flow[0] += 1;
            self.watch.retail_flow[1] += cost + 2;
        }
        self.urban.buildings[id].cash -= cost + 2;
        if sku < 13 {
            job.raw[sku] = q
        } else {
            job.wares[sku - 13] = q
        }
        let job_id = self.post_urban_job(job);
        self.commitments
            .dispatch(i, Action::OrderGoods, id as u32, job_id);
    }
    fn hunt_target(&self, i: usize) -> Option<usize> {
        self.wildlife
            .animals
            .iter()
            .enumerate()
            .filter(|(_, a)| {
                a.alive
                    && a.initial_age > 0
                    && self
                        .wildlife
                        .animals
                        .iter()
                        .filter(|b| b.alive && b.forest == a.forest)
                        .count()
                        > 6
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
    fn update_urban_weight(&mut self, i: usize) {
        let worn = [1u8, 2u8]
            .into_iter()
            .map(|k| {
                self.urban.owned.get(&(i as u32, k)).copied().unwrap_or(0) as u32
                    * WARE_WEIGHT[k as usize]
            })
            .sum::<u32>();
        let load = self
            .urban
            .assigned
            .get(i)
            .and_then(|&id| self.urban.jobs.get(id as usize))
            .and_then(Option::as_ref)
            .filter(|j| j.phase == 1 && j.worker == i as u32)
            .map(Job::weight)
            .unwrap_or(0);
        self.agents.urban_weight[i] = worn + load;
    }
    pub(super) fn execute_urban(&mut self, i: usize, action: Action) {
        if !self.is_adult(i) || self.must_stay_home(i) {
            return;
        }
        if self.agents.convoy[i] != NONE {
            return;
        }
        if self.is_bank_action(action) {
            self.execute_banking(i, action);
            return;
        }
        if action == Action::UrbanWork {
            self.execute_urban_job(i);
            return;
        }
        let Some((x, y)) = self.urban_destination(i) else {
            return;
        };
        let d = dist(self.agents.x[i], self.agents.y[i], x, y);
        if d > 5.0 {
            let step = (self.actor_speed(i, self.agents.x[i], self.agents.y[i]) * DT).min(d - 4.0);
            self.agents.x[i] += (x - self.agents.x[i]) / d * step;
            self.agents.y[i] += (y - self.agents.y[i]) / d * step;
            self.agents.drain_energy(i, DT * 0.08);
            return;
        }
        let id = self.agents.target[i] as usize;
        if action == Action::CollectEstate {
            if self
                .urban
                .estates
                .get(id)
                .is_none_or(|e| e.owner != i as u32)
            {
                return;
            }
            if self.agents.coins[i] > 0 || self.agents.room(i) > 0 {
                self.agents.coins[i] += self.urban.estates[id].cash;
                if self.urban.estates[id].cash > 0 {
                    self.commitments.resolve(
                        i,
                        Action::CollectEstate,
                        id as u32,
                        1,
                        self.tick,
                        true,
                    );
                }
                self.urban.estates[id].cash = 0;
            }
            let firm = self.business(i);
            let has_stock = self.urban.estates[id].raw.iter().sum::<u64>() > 0
                || firm.is_some() && self.urban.estates[id].wares.iter().sum::<u64>() > 0;
            if has_stock && self.agents.coins[i] >= 2 {
                let m = self.agents.home[i] as usize;
                let mut j = self.new_job(
                    m,
                    1,
                    i as u32,
                    NONE,
                    NONE,
                    firm.map(|b| b as u32).unwrap_or(NONE),
                    x,
                    y,
                    2,
                );
                let mut room = 24u64;
                for k in 0..GOODS {
                    let q = self.urban.estates[id].raw[k].min(room / WEIGHTS[k] as u64);
                    j.raw[k] = q;
                    self.urban.estates[id].raw[k] -= q;
                    room -= q * WEIGHTS[k] as u64;
                }
                if firm.is_some() {
                    for k in 0..WARES {
                        let q = self.urban.estates[id].wares[k].min(room / WARE_WEIGHT[k] as u64);
                        j.wares[k] = q;
                        self.urban.estates[id].wares[k] -= q;
                        room -= q * WARE_WEIGHT[k] as u64;
                    }
                }
                if room < 24 {
                    self.agents.coins[i] -= 2;
                    let job_id = self.post_urban_job(j);
                    self.commitments
                        .dispatch(i, Action::CollectEstate, id as u32, job_id);
                }
            }
            self.agents.action[i] = Action::Wait;
            return;
        }
        match action {
            Action::BuyHome | Action::RentHome => {
                if self.home_property(i).is_some() {
                    return;
                }
                let Some(b) = self.urban.buildings.get_mut(id) else {
                    return;
                };
                if b.kind != 0 || !b.ready || b.owner != NONE || b.renter != NONE {
                    return;
                }
                let pay = if action == Action::BuyHome {
                    b.price
                } else {
                    b.rent
                };
                if self.agents.coins[i] < pay {
                    return;
                }
                self.agents.coins[i] -= pay;
                self.markets[b.market as usize].coins += pay;
                self.markets[b.market as usize].revenue_total += pay;
                if action == Action::BuyHome {
                    b.owner = i as u32
                } else {
                    b.renter = i as u32
                }
                b.leased = self.tick;
                b.last_rent = self.tick / rest::DAY_TICKS;
                self.agents.home[i] = b.market;
                self.people[i].happiness = (self.people[i].happiness + 8.0).min(100.0);
            }
            Action::LeaseBusiness => {
                if self.owns_business(i)
                    || self.agents.coins[i] < 58
                    || self
                        .urban
                        .evicted_until
                        .get(&(i as u32))
                        .copied()
                        .unwrap_or(0)
                        > self.tick
                {
                    return;
                }
                let Some(b) = self.urban.buildings.get_mut(id) else {
                    return;
                };
                if !b.ready || b.kind == 0 || !b.advertised || b.owner != NONE {
                    return;
                }
                self.agents.coins[i] -= 58;
                self.markets[b.market as usize].coins += 8;
                self.markets[b.market as usize].revenue_total += 8;
                b.cash += 50;
                b.owner = i as u32;
                b.leased = self.tick;
                b.last_rent = self.tick / rest::DAY_TICKS;
                b.advertised = false;
            }
            Action::OrderGoods => {
                if self
                    .urban
                    .buildings
                    .get(id)
                    .is_some_and(|b| b.owner == i as u32)
                {
                    self.place_procurement(i, id);
                }
            }
            Action::OperateBusiness => self.operate_business(i, id),
            Action::ShopGoods => self.buy_urban_goods(i, id / 32, id % 32),
            Action::Hunt => {
                let Some(firm) = self.business(i) else {
                    return;
                };
                let b = &self.urban.buildings[firm];
                if b.kind != 2
                    || b.line != 0
                    || b.cash < 2
                    || self.agents.room(i) < 5
                    || self.urban.assigned[i] != NONE
                {
                    return;
                }
                let Some(a) = self.wildlife.animals.get(id) else {
                    return;
                };
                if !a.alive
                    || self
                        .wildlife
                        .animals
                        .iter()
                        .filter(|b| b.alive && b.forest == a.forest)
                        .count()
                        <= 6
                {
                    return;
                }
                self.agents.timer[i] += DT;
                if self.agents.timer[i] < 3.0 {
                    return;
                }
                let (x, y) = (a.x, a.y);
                let m = b.market as usize;
                let a = &mut self.wildlife.animals[id];
                a.alive = false;
                a.pregnancy = None;
                self.wildlife.ledger.deaths += 1;
                self.urban.produced[0] += 1;
                self.urban.produced[4] += 2;
                let mut j = self.new_job(m, 1, i as u32, firm as u32, NONE, firm as u32, x, y, 2);
                j.worker = i as u32;
                j.phase = 1;
                j.wares[0] = 1;
                j.wares[4] = 2;
                self.urban.buildings[firm].cash -= 2;
                let job = self.post_urban_job(j);
                self.urban.assigned[i] = job as u32;
                self.agents.action[i] = Action::UrbanWork;
                self.agents.target[i] = job as u32;
                self.work_memory[work_memory::WorkKind::Urban as usize].commit(i, job as u32, 1);
                self.agents.timer[i] = 0.0;
                self.urban.progress.entry(i as u32).or_default().hunted += 1;
                self.update_urban_weight(i);
            }
            _ => {}
        }
    }
    fn operate_business(&mut self, i: usize, id: usize) {
        if self
            .urban
            .buildings
            .get(id)
            .is_none_or(|b| b.owner != i as u32 || !b.ready)
        {
            return;
        }
        self.agents.timer[i] += DT;
        self.urban.buildings[id].last_open = self.tick;
        if self.agents.timer[i] < 5.0 {
            return;
        }
        self.agents.timer[i] = 0.0;
        let m = self.urban.buildings[id].market as usize;
        if self.urban.buildings[id].kind == 1 {
            let day = self.tick / rest::DAY_TICKS;
            if self.urban.buildings[id].last_service != day
                && self.markets[m].coins >= 2
                && (self.agents.coins[i] > 0 || self.agents.room(i) > 0)
            {
                self.markets[m].coins -= 2;
                self.markets[m].expenses_total += 2;
                self.agents.coins[i] += 2;
                self.credit_education_bonus(i, m, 2);
                self.urban.buildings[id].last_service = day;
            }
        } else if let Some((input, q, output)) = self.recipe(id) {
            let b = &mut self.urban.buildings[id];
            if input < 13 {
                b.raw[input] -= q;
                self.ledger.crafted_in[input] += q;
                if input == 7 {
                    b.raw[1] -= 1;
                    self.ledger.crafted_in[1] += 1;
                }
            } else {
                b.wares[input - 13] -= q;
                self.urban.consumed[input - 13] += q;
            }
            if output < 13 {
                b.raw[output] += 1;
                self.ledger.crafted_out[output] += 1;
            } else {
                b.wares[output - 13] += 1;
                self.urban.produced[output - 13] += 1;
            }
            b.made += 1;
            self.urban.progress.entry(i as u32).or_default().made += 1;
        }
        let b = &mut self.urban.buildings[id];
        if b.cash > 50 && (self.agents.coins[i] > 0 || self.agents.room(i) > 0) {
            let profit = (b.cash - 50).min(5);
            b.cash -= profit;
            self.agents.coins[i] += profit;
        }
        self.agents.drain_energy(i, 0.5);
    }
    fn buy_urban_goods(&mut self, i: usize, id: usize, sku: usize) {
        let before = self.agents.coins[i];
        let outcome = self.settle_purchase(i, id, sku);
        if self.watch.person == Some(i) {
            if let Some(d) = self.watch.last.as_mut() {
                if d["shop_id"].as_u64() == Some(id as u64)
                    && d["product_id"].as_u64() == Some(sku as u64)
                    && d["outcome"].is_null()
                {
                    let purchased = outcome == "purchased";
                    d["resolution_tick"] = self.tick.into();
                    d["outcome"] = outcome.into();
                    d["quantity"] = u64::from(purchased).into();
                    d["buyer_charge"] = (before - self.agents.coins[i]).into();
                    d["market_levy"] = u64::from(purchased).into();
                    d["shop_proceeds"] = (if purchased {
                        Self::retail_price(sku) - 1
                    } else {
                        0
                    })
                    .into();
                    self.watch.purchase = Some(d.clone());
                }
            }
        }
    }
    fn settle_purchase(&mut self, i: usize, id: usize, sku: usize) -> &'static str {
        let Some(b) = self.urban.buildings.get(id) else {
            return "unavailable";
        };
        if b.kind != 1
            || b.owner == NONE
            || !b.ready
            || self.tick.saturating_sub(b.last_open) >= 30
            || !Self::catalog(b.line).contains(&sku)
        {
            return "unavailable";
        }
        if Self::inventory(b, sku) == 0 {
            return "out_of_stock";
        }
        let price = Self::retail_price(sku);
        let home = if sku == 16 {
            self.home_property(i)
        } else {
            None
        };
        if self.agents.coins[i] < price + if sku == 16 { 2 } else { 0 } {
            return "unavailable";
        }
        if sku == 16
            && (home.is_none()
                || home.is_some_and(|h| {
                    self.urban.buildings[h].wares[3] > 0
                        || self
                            .urban
                            .jobs
                            .iter()
                            .flatten()
                            .any(|j| j.target == h as u32 && j.wares[3] > 0)
                }))
        {
            return "unavailable";
        }
        if sku < 13 && (self.agents.tools[i][sku - 9] > 0 || self.agents.room(i) < WEIGHTS[sku]) {
            return "unavailable";
        }
        if [14, 15].contains(&sku)
            && (self
                .urban
                .owned
                .get(&(i as u32, (sku - 13) as u8))
                .copied()
                .unwrap_or(0)
                > 0
                || self.agents.room(i) < WARE_WEIGHT[sku - 13])
        {
            return "unavailable";
        }
        let m = b.market as usize;
        let owner = b.owner;
        let (x, y) = (b.x, b.y);
        self.agents.coins[i] -= price;
        self.urban.buildings[id].cash += price - 1;
        self.markets[m].coins += 1;
        self.markets[m].revenue_total += 1;
        self.urban.buildings[id].sold += 1;
        self.urban.progress.entry(owner).or_default().sales += 1;
        if sku < 13 {
            self.urban.buildings[id].raw[sku] -= 1;
            self.agents.tools[i][sku - 9] += 1;
            self.markets[m].sales[sku] += 1;
            self.markets[m].local_sales[sku] += 1;
        } else {
            let k = sku - 13;
            self.urban.buildings[id].wares[k] -= 1;
            *self
                .urban
                .retail_sales
                .entry((m as u32, k as u8))
                .or_default() += 1;
            if k == 4 {
                self.urban.consumed[k] += 1;
                self.agents.hunger[i] = (self.agents.hunger[i] - 40.0).max(0.0);
                self.people[i].meals += 1;
            } else if k == 3 {
                self.agents.coins[i] -= 2;
                let mut job = self.new_job(
                    m,
                    1,
                    i as u32,
                    NONE,
                    id as u32,
                    home.unwrap() as u32,
                    x,
                    y,
                    2,
                );
                job.wares[k] = 1;
                self.post_urban_job(job);
            } else {
                *self.urban.owned.entry((i as u32, k as u8)).or_default() += 1;
                self.update_urban_weight(i);
            }
        }
        self.people[i].happiness = (self.people[i].happiness + 2.0).min(100.0);
        if dist(
            self.agents.x[i],
            self.agents.y[i],
            self.agents.x[owner as usize],
            self.agents.y[owner as usize],
        ) <= 20.0
        {
            self.relationship_contact(i, owner as usize, relationships::Context::Shop);
        }
        "purchased"
    }
    fn execute_urban_job(&mut self, i: usize) {
        let id = self.agents.target[i] as usize;
        let Some(j) = self.urban.jobs.get(id).and_then(Option::as_ref) else {
            self.work_memory[work_memory::WorkKind::Urban as usize].cancel(i);
            self.urban.assigned[i] = NONE;
            self.update_urban_weight(i);
            return;
        };
        if j.worker != NONE && j.worker != i as u32 {
            self.work_memory[work_memory::WorkKind::Urban as usize].resolve_at(
                i,
                (id as u32, 1),
                work_memory::urban_memory_target(j.market, j.kind),
                self.tick,
                false,
            );
            return;
        }
        if j.kind == 0 && self.urban.buildings[j.target as usize].needs_clear {
            return;
        }
        if self.urban.assigned[i] != NONE && self.urban.assigned[i] != id as u32 {
            return;
        }
        self.urban.jobs[id].as_mut().unwrap().worker = i as u32;
        self.urban.assigned[i] = id as u32;
        let Some((x, y)) = self.urban_destination(i) else {
            return;
        };
        let d = dist(self.agents.x[i], self.agents.y[i], x, y);
        if d > 5.0 {
            let step = (self.actor_speed(i, self.agents.x[i], self.agents.y[i]) * DT).min(d - 4.0);
            self.agents.x[i] += (x - self.agents.x[i]) / d * step;
            self.agents.y[i] += (y - self.agents.y[i]) / d * step;
            self.agents.drain_energy(i, DT * 0.08);
            return;
        }
        let j = self.urban.jobs[id].as_mut().unwrap();
        j.progress += DT;
        if j.kind == 0 {
            if j.progress < 20.0 {
                return;
            }
            let b = &mut self.urban.buildings[j.target as usize];
            b.ready = true;
            b.progress = 20.0;
            if b.kind == 0 {
                b.shelter = self.shelters.len() as u32;
                self.shelters
                    .push(Shelter::new(b.x, b.y, Some(b.market), 4));
            }
            self.finish_urban_job(i, id);
            return;
        }
        if j.phase == 0 && [2, 3].contains(&j.kind) {
            if j.progress < 5.0 {
                return;
            }
            let kind = j.kind;
            let target = j.target;
            let m = j.market;
            if kind == 3 {
                let s = &self.shelters[target as usize];
                if s.beds.iter().any(|&owner| owner != NONE) {
                    self.refund_urban_job(id);
                    return;
                }
                let salvage = self.ledger.built[1].min(2);
                self.ledger.built[1] -= salvage;
                self.shelters[target as usize].beds.clear();
                self.shelters[target as usize].free.clear();
                let j = self.urban.jobs[id].as_mut().unwrap();
                j.raw[1] = salvage;
                j.target = NONE;
                j.source = target;
                j.phase = 1;
                j.progress = 0.0;
            } else {
                let b = &self.urban.buildings[target as usize];
                let timber = self
                    .patches
                    .iter()
                    .position(|p| p.kind == 1 && p.stock > 0 && dist(p.x, p.y, b.x, b.y) < 45.0);
                if let Some(p) = timber {
                    let q = self.patches[p]
                        .stock
                        .min(2)
                        .min((self.agents.room(i) / WEIGHTS[1]) as u64);
                    if q == 0 {
                        return;
                    }
                    self.patches[p].stock -= q;
                    let j = self.urban.jobs[id].as_mut().unwrap();
                    j.raw[1] = q;
                    j.source = target;
                    j.target = NONE;
                    j.phase = 1;
                    j.progress = 0.0;
                } else {
                    let cleared = (b.x, b.y, 45.0);
                    self.urban.buildings[target as usize].needs_clear = false;
                    self.urban.cleared.push(cleared);
                    self.retire_cleared_timber(cleared);
                    self.finish_urban_job(i, id);
                    return;
                }
            }
            let _ = m;
            self.update_urban_weight(i);
            return;
        }
        let j = self.urban.jobs[id].as_ref().unwrap();
        if j.phase == 0 {
            if j.progress < 1.0 || self.agents.room(i) < j.weight() {
                return;
            }
            if j.kind == 1
                && self
                    .urban
                    .buildings
                    .get(j.target as usize)
                    .is_some_and(|b| b.kind == 1)
            {
                self.watch.retail_flow[2] += 1;
            }
            let j = self.urban.jobs[id].as_mut().unwrap();
            j.phase = 1;
            j.progress = 0.0;
            self.update_urban_weight(i);
            return;
        }
        if j.progress < 1.0 {
            return;
        }
        self.finish_urban_job(i, id);
    }
    fn finish_urban_job(&mut self, i: usize, id: usize) {
        let Some(j) = self.urban.jobs[id].take() else {
            return;
        };
        if j.kind != 0 {
            if j.target == NONE {
                for k in 0..GOODS {
                    if j.raw[k] > 0 {
                        self.markets[j.market as usize].add(k, j.raw[k], j.market);
                    }
                }
            } else {
                let b = &mut self.urban.buildings[j.target as usize];
                if j.kind == 1 && b.kind == 1 {
                    self.watch.retail_flow[3] += 1;
                }
                for k in 0..GOODS {
                    b.raw[k] += j.raw[k];
                }
                for k in 0..WARES {
                    b.wares[k] += j.wares[k];
                }
            }
            if j.kind == 2 && j.phase == 1 {
                let b = &self.urban.buildings[j.source as usize];
                let remains = self
                    .patches
                    .iter()
                    .any(|p| p.kind == 1 && p.stock > 0 && dist(p.x, p.y, b.x, b.y) < 45.0);
                if !remains {
                    let pos = (b.x, b.y, 45.0);
                    self.urban.buildings[j.source as usize].needs_clear = false;
                    self.urban.cleared.push(pos);
                    self.retire_cleared_timber(pos);
                }
            }
            self.urban.progress.entry(i as u32).or_default().hauled += 1;
        }
        self.urban.assigned[i] = NONE;
        self.update_urban_weight(i);
        self.agents.coins[i] += j.reward;
        self.people[i].jobs += 1;
        self.urban.completed += 1;
        self.commitments.delivery_finished(id, self.tick);
        self.work_memory[work_memory::WorkKind::Urban as usize].resolve_at(
            i,
            (id as u32, 1),
            work_memory::urban_memory_target(j.market, j.kind),
            self.tick,
            true,
        );
        if j.payer == NONE {
            self.markets[j.market as usize].expenses_total += j.reward;
            self.credit_education_bonus(i, j.market as usize, j.reward);
        }
        self.agents.action[i] = Action::Wait;
    }
    fn retire_cleared_timber(&mut self, (x, y, r): (f32, f32, f32)) {
        for p in &mut self.patches {
            if p.kind == 1 && p.stock == 0 && dist(p.x, p.y, x, y) < r {
                p.capacity = 0;
                p.growth = 0.0;
            }
        }
    }
    fn refund_urban_job(&mut self, id: usize) {
        self.commitments.delivery_cancelled(id);
        let Some(j) = self.urban.jobs[id].take() else {
            return;
        };
        if j.raw.iter().sum::<u64>() + j.wares.iter().sum::<u64>() > 0 {
            self.urban.estates.push(Estate {
                owner: j.payer,
                x: j.x,
                y: j.y,
                cash: 0,
                raw: j.raw,
                wares: j.wares,
            });
        }
        if j.payer == NONE {
            self.markets[j.market as usize].coins += j.reward;
        } else if let Some(b) = self.urban.buildings.get_mut(j.firm as usize) {
            b.cash += j.reward;
        } else {
            self.agents.coins[j.payer as usize] += j.reward;
        }
        if j.worker != NONE {
            self.work_memory[work_memory::WorkKind::Urban as usize].cancel(j.worker as usize);
            self.urban.assigned[j.worker as usize] = NONE;
            self.update_urban_weight(j.worker as usize);
        }
    }
    fn vacate_business(&mut self, id: usize) {
        let b = &mut self.urban.buildings[id];
        let owner = b.owner;
        if owner == NONE {
            return;
        }
        self.urban.estates.push(Estate {
            owner,
            x: b.x,
            y: b.y,
            cash: b.cash,
            raw: b.raw,
            wares: b.wares,
        });
        b.cash = 0;
        b.raw = [0; GOODS];
        b.wares = [0; WARES];
        b.owner = NONE;
        b.arrears = 0;
        b.advertised = false;
        b.last_open = 0;
        self.urban
            .evicted_until
            .insert(owner, self.tick + rest::DAY_TICKS);
    }
    pub(super) fn pause_urban_job(&mut self, i: usize) {
        self.work_memory[work_memory::WorkKind::Urban as usize].cancel(i);
        let Some(&id) = self.urban.assigned.get(i) else {
            return;
        };
        if let Some(Some(j)) = self.urban.jobs.get_mut(id as usize) {
            if j.worker == i as u32 {
                if j.phase == 1 {
                    j.x = self.agents.x[i];
                    j.y = self.agents.y[i];
                    j.phase = 0;
                    j.progress = 0.0;
                }
                j.worker = NONE;
            }
        }
        self.urban.assigned[i] = NONE;
        self.update_urban_weight(i);
    }
    pub(super) fn urban_step(&mut self) {
        self.sync_urban();
        for id in 0..self.urban.jobs.len() {
            let Some(j) = self.urban.jobs[id].as_ref() else {
                continue;
            };
            let worker = j.worker;
            if worker == NONE {
                continue;
            }
            let i = worker as usize;
            if !self.agents.alive[i] || !self.is_adult(i) || self.must_stay_home(i) {
                let j = self.urban.jobs[id].as_mut().unwrap();
                if j.phase == 1 {
                    j.x = self.agents.x[i];
                    j.y = self.agents.y[i];
                    j.phase = 0;
                    j.progress = 0.0;
                }
                j.worker = NONE;
                self.urban.assigned[i] = NONE;
                self.update_urban_weight(i);
            }
        }
        // Detect lost operators each tick; do not leave a dead tenant blocking a premises for a day.
        for id in 0..self.urban.buildings.len() {
            let b = &self.urban.buildings[id];
            if b.kind > 0
                && b.owner != NONE
                && !self.agents.alive[b.owner as usize]
                && !self.has_job(id)
            {
                self.vacate_business(id);
            }
        }
        if self.tick % rest::DAY_TICKS != 0 {
            return;
        }
        let day = self.tick / rest::DAY_TICKS;
        for id in 0..self.urban.buildings.len() {
            let b = &self.urban.buildings[id];
            if !b.ready {
                continue;
            }
            let tenant = if b.kind == 0 { b.renter } else { b.owner };
            if tenant == NONE {
                continue;
            }
            if !self.agents.alive[tenant as usize] {
                if b.kind == 0 {
                    self.urban.buildings[id].renter = NONE;
                } else if !self.has_job(id) {
                    self.vacate_business(id);
                }
                continue;
            }
            if b.last_rent >= day {
                continue;
            }
            let m = b.market as usize;
            let rent = b.rent;
            let pay = if b.kind == 0 {
                let q = self.agents.coins[tenant as usize].min(rent);
                self.agents.coins[tenant as usize] -= q;
                q
            } else {
                let q = b.cash.min(rent);
                self.urban.buildings[id].cash -= q;
                q
            };
            self.markets[m].coins += pay;
            self.markets[m].revenue_total += pay;
            self.urban.buildings[id].arrears += rent - pay;
            self.urban.buildings[id].last_rent = day;
        }
        for id in 0..self.urban.buildings.len() {
            let b = &self.urban.buildings[id];
            if b.kind != 0 || b.owner == NONE {
                continue;
            }
            let owner = b.owner as usize;
            if !self.agents.alive[owner] {
                let heir = self.people[owner]
                    .children
                    .iter()
                    .copied()
                    .find(|&j| self.agents.alive[j as usize])
                    .unwrap_or(NONE);
                self.urban.buildings[id].owner = heir;
            } else if b.wares[3] > 0 {
                self.people[owner].happiness = (self.people[owner].happiness + 2.0).min(100.0);
            }
        }
    }
    pub(super) fn urban_choice_name(&self, c: &Candidate) -> String {
        match c.action {
            Action::UrbanWork => self
                .urban
                .jobs
                .get(c.target as usize)
                .and_then(Option::as_ref)
                .map(|j| {
                    [
                        "Builder contract",
                        "Goods transport",
                        "Forest clearance",
                        "Shelter clearance",
                    ][j.kind as usize]
                        .into()
                })
                .unwrap_or("Completed contract".into()),
            Action::ShopGoods => {
                let sku = c.target as usize % 32;
                if sku < 13 {
                    PRODUCTS[sku].into()
                } else {
                    WARE_NAMES[sku - 13].into()
                }
            }
            Action::LeaseBusiness => self
                .urban
                .buildings
                .get(c.target as usize)
                .map(|b| format!("{} opening", Self::operator_role(b)))
                .unwrap_or("Operator opening".into()),
            Action::BuildBank => "Build town bank".into(),
            Action::BorrowBusiness => "Business loan".into(),
            Action::RepayLoan => "Repay business debt".into(),
            Action::CollectEstate => "Recover stored business assets".into(),
            Action::BuyHome | Action::RentHome => "Residential property".into(),
            Action::Hunt => "Wild game".into(),
            _ => "Business premises".into(),
        }
    }
    pub(super) fn observed_market_profile(&self, m: usize) -> String {
        let mut sales: Vec<(String, u64)> = (0..GOODS)
            .map(|k| (PRODUCTS[k].to_string(), self.markets[m].sales[k]))
            .collect();
        sales.extend((0..WARES).map(|k| {
            (
                WARE_NAMES[k].to_string(),
                self.urban
                    .retail_sales
                    .get(&(m as u32, k as u8))
                    .copied()
                    .unwrap_or(0),
            )
        }));
        let total = sales.iter().map(|(_, n)| n).sum::<u64>();
        if total < 10 {
            return "Insufficient sales".into();
        }
        let (name, n) = sales.into_iter().max_by_key(|(_, n)| *n).unwrap();
        format!(
            "{} {:.0}% of completed purchases",
            name,
            n as f64 / total as f64 * 100.0
        )
    }
    pub(super) fn urban_summary(&self) -> serde_json::Value {
        let wares = self.ware_stocks();
        serde_json::json!({"operator_jobs":self.operator_job_postings(),"wealth_threshold":500,"reserve_threshold":100,"zones":self.urban.zones,"buildings":self.urban.buildings.iter().enumerate().map(|(id,b)|serde_json::json!({"id":id,"market":b.market,"kind":b.kind,"line":b.line,"x":b.x,"y":b.y,"ready":b.ready,"needs_clear":b.needs_clear,"advertised":b.advertised,"owner":if b.owner==NONE{None}else{Some(b.owner)},"renter":if b.renter==NONE{None}else{Some(b.renter)},"price":b.price,"rent":b.rent,"arrears":b.arrears,"cash":b.cash,"raw":b.raw,"wares":b.wares,"made":b.made,"sold":b.sold,"shelter":if b.shelter==NONE{None}else{Some(b.shelter)}})).collect::<Vec<_>>(),"villages":self.urban.states.iter().enumerate().map(|(m,s)|serde_json::json!({"market":m,"zoned":s.zoned,"persona":(["Balanced","Forest","Coastal","Rocky"][s.persona as usize]),"persona_id":s.persona,"coffers":self.coffers(m),"free_cash":self.markets[m].coins,"biases":PERSONA_BIAS[s.persona as usize],"town":self.is_town(m),"completed":(0..3).map(|kind|self.urban.buildings.iter().filter(|b|b.market as usize==m&&b.kind==kind&&b.ready).count()).collect::<Vec<_>>(),"vacant_commercial":self.urban.buildings.iter().filter(|b|b.market as usize==m&&b.ready&&b.kind==1&&b.owner==NONE).count(),"vacant_industrial":self.urban.buildings.iter().filter(|b|b.market as usize==m&&b.ready&&b.kind==2&&b.owner==NONE).count(),"operator_vacancy_pressure":self.operator_vacancy_pressure(m),"lease_openings":self.urban.buildings.iter().filter(|b|b.market as usize==m&&b.owner==NONE&&b.advertised).count()})).collect::<Vec<_>>(),"jobs":self.urban.jobs.iter().enumerate().filter_map(|(id,j)|j.as_ref().map(|j|serde_json::json!({"id":id,"market":j.market,"kind":(["Builder","Haulier","Forester","Shelter removal"][j.kind as usize]),"reward":j.reward,"private":j.payer!=NONE,"worker":if j.worker==NONE{None}else{Some(j.worker)},"x":j.x,"y":j.y,"phase":j.phase,"raw":j.raw,"wares":j.wares}))).take(1024).collect::<Vec<_>>(),"ware_names":WARE_NAMES,"ware_weights":WARE_WEIGHT,"ware_accounts":(0..WARES).map(|k|serde_json::json!({"name":WARE_NAMES[k],"produced":self.urban.produced[k],"consumed":self.urban.consumed[k],"stock":wares[k],"error":self.urban.produced[k] as i128-self.urban.consumed[k] as i128-wares[k] as i128})).collect::<Vec<_>>(),"posted_private":self.urban.posted_private,"posted_public":self.urban.posted_public,"completed_jobs":self.urban.completed,"evictions":self.urban.evictions,"cleared":self.urban.cleared,"estates":self.urban.estates})
    }
    pub(super) fn validate_urban(&self) -> Result<(), String> {
        if self.urban.assigned.len() != self.agents.len()
            || self.urban.states.len() != self.markets.len()
        {
            return Err("Invalid urban pools".into());
        }
        let stock = self.ware_stocks();
        for k in 0..WARES {
            if self.urban.produced[k] != stock[k] + self.urban.consumed[k] {
                return Err(format!("{} conservation error", WARE_NAMES[k]));
            }
        }
        let mut carried = vec![0u32; self.agents.len()];
        for j in self.urban.jobs.iter().flatten() {
            if j.market as usize >= self.markets.len()
                || j.worker != NONE && j.worker as usize >= self.agents.len()
            {
                return Err("Invalid urban contract".into());
            }
            if j.phase == 1 && j.worker != NONE {
                carried[j.worker as usize] += j.weight();
            }
        }
        for (&(i, k), q) in &self.urban.owned {
            if i as usize >= self.agents.len() || k as usize >= WARES {
                return Err("Invalid personal possessions".into());
            }
            if [1, 2].contains(&k) {
                carried[i as usize] += *q as u32 * WARE_WEIGHT[k as usize];
            }
        }
        if carried != self.agents.urban_weight {
            return Err("Urban carrying weight mismatch".into());
        }
        for b in &self.urban.buildings {
            if b.market as usize >= self.markets.len()
                || b.kind > 2
                || b.line > 3
                || b.owner != NONE && b.owner as usize >= self.agents.len()
                || b.renter != NONE && b.renter as usize >= self.agents.len()
            {
                return Err("Invalid property".into());
            }
        }
        Ok(())
    }
}

impl World {
    /// A funded hamlet fixture. All money and materials are transferred from existing agents/land.
    /// Districts, private buildings, tenants and deliveries still require neural decisions.
    pub fn prepare_urban_demo(&mut self) -> Result<(), String> {
        if self.agents.len() < 256 {
            return Err("Urban demo needs at least 256 inhabitants".into());
        }
        for i in 0..self.agents.len() {
            let q = self.agents.coins[i].min(18);
            self.agents.coins[i] -= q;
            self.markets[0].coins += q;
        }
        for (kind, amount) in [(1, 1400u64), (2, 600), (7, 150)] {
            let mut remaining = amount;
            for p in &mut self.patches {
                if p.kind != kind {
                    continue;
                }
                let q = p.stock.min(remaining);
                p.stock -= q;
                remaining -= q;
                self.markets[0].add(kind, q, 0);
                if remaining == 0 {
                    break;
                }
            }
        }
        for i in 0..12 {
            let q = 100.min(self.markets[0].coins);
            self.markets[0].coins -= q;
            self.agents.coins[i] += q;
            self.agents.home[i] = 0;
            self.agents.x[i] = self.markets[0].x + (i % 4) as f32 * 6.0;
            self.agents.y[i] = self.markets[0].y + (i / 4) as f32 * 6.0;
            self.people[i].aspiration = match i % 5 {
                0 => life::Aspiration::Artisan,
                1 => life::Aspiration::Shopkeeper,
                2 => life::Aspiration::Haulier,
                3 => life::Aspiration::Homeowner,
                _ => life::Aspiration::Hunter,
            };
        }
        self.markets[0].village = true;
        self.sync_councils();
        self.sync_urban();
        self.refresh();
        self.check()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn funded() -> World {
        let mut w = World::new(256, 42).unwrap();
        w.prepare_urban_demo().unwrap();
        w
    }
    fn construct(w: &mut World, kind: u8) -> usize {
        w.apply_urban_policy(0, "GazetteDistricts");
        let id = w.urban.buildings.len();
        w.commission_urban(0, kind);
        assert_eq!(w.urban.buildings.len(), id + 1);
        // This fixture selects a cleared open site; clearance has separate coverage.
        w.urban.buildings[id].needs_clear = false;
        let j = w
            .urban
            .jobs
            .iter()
            .position(|j| {
                j.as_ref()
                    .is_some_and(|j| j.kind == 0 && j.target == id as u32)
            })
            .unwrap();
        w.agents.action[0] = Action::UrbanWork;
        w.agents.target[0] = j as u32;
        w.agents.x[0] = w.urban.buildings[id].x;
        w.agents.y[0] = w.urban.buildings[id].y;
        for _ in 0..202 {
            w.execute_urban_job(0);
        }
        assert!(w.urban.buildings[id].ready);
        w.check().unwrap();
        id
    }
    fn at(w: &mut World, i: usize, b: usize, a: Action) {
        w.agents.action[i] = a;
        w.agents.target[i] = b as u32;
        w.agents.x[i] = w.urban.buildings[b].x;
        w.agents.y[i] = w.urban.buildings[b].y;
    }
    #[test]
    fn town_requires_nine_completed_buildings_and_builders_are_paid_once() {
        let mut w = funded();
        let coins = w.coins();
        for kind in 0..3 {
            for _ in 0..3 {
                construct(&mut w, kind);
            }
        }
        assert!(w.is_town(0));
        assert_eq!(w.urban.completed, 9);
        assert_eq!(w.coins(), coins);
        assert_eq!(w.urban.zones.len(), 3);
        assert_eq!(
            w.urban
                .buildings
                .iter()
                .filter(|b| b.kind == 0)
                .map(|b| w.shelters[b.shelter as usize].beds.len())
                .collect::<Vec<_>>(),
            vec![4, 4, 4]
        );
        w.urban.buildings[8].ready = false;
        assert!(!w.is_town(0));
        w.check().unwrap();
    }

    #[test]
    fn city_title_requires_both_confirmed_thresholds() {
        let mut w = funded();
        for kind in 0..3 {
            for _ in 0..3 {
                construct(&mut w, kind);
            }
        }
        w.select_camp_leader(0, &[0]);
        w.governance_step();
        let next = w.governance_summary()[0]["next_election"].clone();
        let building = w.urban.buildings[0].clone();
        w.urban.buildings.resize(49, building.clone());
        w.markets[0].stable_residents = 500;
        assert_eq!(w.governance_summary()[0]["title"], "Town Mayor");
        w.urban.buildings.push(building);
        w.markets[0].stable_residents = 499;
        assert_eq!(w.governance_summary()[0]["title"], "Town Mayor");
        w.markets[0].stable_residents = 500;
        assert_eq!(w.governance_summary()[0]["title"], "City Mayor");
        assert_eq!(w.governance_summary()[0]["leader"], 0);
        assert_eq!(w.governance_summary()[0]["next_election"], next);
    }
    #[test]
    fn home_purchase_and_rent_restrict_private_beds_without_minting_cash() {
        let mut w = funded();
        let b = construct(&mut w, 0);
        let coins = w.coins();
        at(&mut w, 1, b, Action::RentHome);
        w.execute_urban(1, Action::RentHome);
        assert_eq!(w.urban.buildings[b].renter, 1);
        let site = w.urban.buildings[b].shelter as usize;
        assert!(w.can_use_urban_shelter(1, site));
        assert!(!w.can_use_urban_shelter(2, site));
        at(&mut w, 2, b, Action::BuyHome);
        w.execute_urban(2, Action::BuyHome);
        assert_eq!(w.urban.buildings[b].owner, NONE);
        assert_eq!(w.coins(), coins);
        w.check().unwrap();
    }
    #[test]
    fn delivery_needs_pickup_arrival_and_preserves_a_dropped_parcel() {
        let mut w = funded();
        let b = construct(&mut w, 2);
        let mut j = w.new_job(
            0,
            1,
            1,
            NONE,
            NONE,
            b as u32,
            w.markets[0].x,
            w.markets[0].y,
            2,
        );
        w.agents.coins[1] -= 2;
        j.raw[1] = 2;
        w.markets[0].take(1, 2).unwrap();
        let id = w.post_urban_job(j);
        w.agents.action[2] = Action::UrbanWork;
        w.agents.target[2] = id as u32;
        let memory_index = work_memory::WorkKind::Urban as usize;
        w.work_memory[memory_index].commit(2, id as u32, 1);
        w.agents.x[2] = w.markets[0].x;
        w.agents.y[2] = w.markets[0].y;
        for _ in 0..12 {
            w.execute_urban_job(2);
        }
        assert_eq!(w.agents.urban_weight[2], 8);
        assert_eq!(
            w.action_memory_inputs(2, Action::UrbanWork, id as u32),
            Some([0.0; 4])
        );
        assert_eq!(w.urban.buildings[b].raw[1], 0);
        w.agents.x[2] += 10.0;
        let dropped = w.agents.x[2];
        w.pause_urban_job(2);
        assert_eq!(w.urban.jobs[id].as_ref().unwrap().x, dropped);
        assert_eq!(w.agents.urban_weight[2], 0);
        w.agents.action[3] = Action::UrbanWork;
        w.agents.target[3] = id as u32;
        w.work_memory[memory_index].commit(3, id as u32, 1);
        w.agents.x[3] = dropped;
        w.agents.y[3] = w.agents.y[2];
        for _ in 0..12 {
            w.execute_urban_job(3);
        }
        at(&mut w, 3, b, Action::UrbanWork);
        w.agents.target[3] = id as u32;
        for _ in 0..12 {
            w.execute_urban_job(3);
        }
        assert_eq!(w.urban.buildings[b].raw[1], 2);
        assert!(w.urban.jobs[id].is_none());
        let key = work_memory::urban_memory_target(0, 1);
        assert_eq!(w.work_memory[memory_index].inputs(2, key, w.tick), [0.0; 4]);
        assert_eq!(w.work_memory[memory_index].inputs(3, key, w.tick)[0], 1.0);
        w.check().unwrap();
    }
    #[test]
    fn crowded_town_counts_each_discarded_offer_without_reordering() {
        let mut w = funded();
        let shop = construct(&mut w, 1);
        w.urban.buildings[shop].line = 3;
        w.urban.buildings[shop].owner = 2;
        w.urban.buildings[shop].wares[4] = 1;
        w.agents.hunger[3] = 35.0;
        let template = w.urban.buildings[shop].clone();
        w.urban.buildings.extend(std::iter::repeat_n(template, 24));
        let (all, n) = w.offer(3);
        assert!(
            all[..n]
                .iter()
                .filter(|c| c.action == Action::ShopGoods)
                .count()
                > 1
        );
        // Exercise the production admission path under saturation even if domain
        // preselection keeps the current scenario below the normal 64-slot cap.
        let mut omitted = Counts::new();
        let (limited, kept) = w.offer_with_limit(3, Some(&mut omitted), 4);
        assert_eq!(kept, 4);
        for (actual, expected) in limited[..kept].iter().zip(&all) {
            assert_eq!(
                (actual.action, actual.target, actual.score),
                (expected.action, expected.target, expected.score)
            );
        }
        let mut expected = Counts::new();
        for c in &all[kept..n] {
            expected.entry(format!("{:?}", c.action)).or_default()[1] += 1;
        }
        assert_eq!(omitted, expected);
        assert!(omitted["ShopGoods"][1] > 0);
    }

    #[test]
    fn watching_decisions_preserves_serialized_world() {
        let mut w = funded();
        let mut control = w.clone();
        w.watch_person(Some(3));
        for _ in 0..20 {
            w.step(true);
            control.step(false);
        }
        assert!(w.trace(3).unwrap().last_decision.is_some());
        assert_eq!(
            postcard::to_allocvec(&w).unwrap(),
            postcard::to_allocvec(&control).unwrap()
        );
        w.watch_person(Some(4));
        assert!(w.trace(4).unwrap().last_decision.is_none());
    }

    #[test]
    fn retail_probe_separates_stock_open_hours_and_affordability() {
        let mut w = funded();
        let shop = construct(&mut w, 1);
        w.tick += 40;
        w.urban.buildings[shop].line = 3;
        w.urban.buildings[shop].owner = 2;
        let probe = |w: &World| {
            w.retail_supply_probe(Some(3))
                .into_iter()
                .collect::<BTreeMap<_, _>>()
        };
        let empty = probe(&w);
        w.urban.buildings[shop].wares[4] = 1;
        assert_eq!(probe(&w)["stocked"], empty["stocked"] + 1);
        assert_eq!(probe(&w)["open_stocked"], empty["open_stocked"]);
        w.urban.buildings[shop].last_open = w.tick;
        w.agents.coins[3] = 0;
        assert_eq!(probe(&w)["open_stocked"], empty["open_stocked"] + 1);
        assert_eq!(probe(&w)["affordable"], 0);
        w.agents.coins[3] = World::retail_price(17);
        assert!(probe(&w)["fits"] > 0);
        w.tick += 30;
        assert_eq!(probe(&w)["open_stocked"], 0);
    }
    #[test]
    fn two_buyers_compete_for_the_last_shop_meal() {
        let mut w = funded();
        let shop = construct(&mut w, 1);
        w.urban.buildings[shop].line = 3;
        w.apply_urban_policy(0, "AdvertisePremises");
        at(&mut w, 2, shop, Action::LeaseBusiness);
        w.execute_urban(2, Action::LeaseBusiness);
        assert_eq!(w.urban.buildings[shop].owner, 2);
        w.urban.produced[4] += 1;
        w.urban.buildings[shop].wares[4] = 1;
        w.urban.buildings[shop].last_open = w.tick;
        let price = World::retail_price(17);
        let cash = w.urban.buildings[shop].cash;
        let treasury = w.markets[0].coins;
        for i in [3, 4] {
            at(&mut w, i, shop, Action::ShopGoods);
            w.agents.target[i] = (shop * 32 + 17) as u32;
            assert!(w.agents.coins[i] >= price);
        }
        let wallets = [w.agents.coins[3], w.agents.coins[4]];
        let meals = [w.people[3].meals, w.people[4].meals];
        for i in [3, 4] {
            w.watch_person(Some(i));
            w.capture_decision(&Request {
                memory_offers: None,
                agent: i,
                counts: None,
                candidate: Candidate {
                    action: Action::ShopGoods,
                    target: w.agents.target[i],
                    ..Candidate::default()
                },
            });
            w.execute_urban(i, Action::ShopGoods);
            let record = w.trace(i).unwrap().last_purchase.unwrap();
            assert_eq!(
                record["outcome"],
                if i == 3 { "purchased" } else { "out_of_stock" }
            );
            assert_eq!(record["buyer_charge"], if i == 3 { price } else { 0 });
            assert_eq!(record["shop_id"], shop);
            assert_eq!(record["owner_id"], 2);
        }
        assert_eq!(w.agents.coins[3], wallets[0] - price);
        assert_eq!(w.agents.coins[4], wallets[1]);
        assert_eq!(w.people[3].meals, meals[0] + 1);
        assert_eq!(w.people[4].meals, meals[1]);
        assert_eq!(w.urban.buildings[shop].wares[4], 0);
        assert_eq!(w.urban.buildings[shop].cash, cash + price - 1);
        assert_eq!(w.markets[0].coins, treasury + 1);
        w.check().unwrap();
    }
    #[test]
    fn manufacture_wholesale_and_retail_use_real_stock_and_cash() {
        let mut w = funded();
        let factory = construct(&mut w, 2);
        let shop = construct(&mut w, 1);
        w.urban.buildings[factory].line = 3;
        w.urban.buildings[shop].line = 2;
        w.apply_urban_policy(0, "AdvertisePremises");
        for (i, b) in [(1, factory), (2, shop)] {
            at(&mut w, i, b, Action::LeaseBusiness);
            w.execute_urban(i, Action::LeaseBusiness);
            assert_eq!(w.urban.buildings[b].owner, i as u32);
        }
        w.markets[0].take(1, 3).unwrap();
        w.urban.buildings[factory].raw[1] = 3;
        at(&mut w, 1, factory, Action::OperateBusiness);
        w.agents.timer[1] = 5.0;
        w.operate_business(1, factory);
        assert_eq!(w.urban.buildings[factory].wares[3], 1);
        assert_eq!(w.urban.buildings[factory].raw[1], 0);
        w.commitments.commit(2, Action::OrderGoods, shop as u32, 1);
        w.place_procurement(2, shop);
        let neutral = Some([0.0; 4]);
        assert_eq!(
            w.action_memory_inputs(2, Action::OrderGoods, shop as u32),
            neutral
        );
        let path = std::env::temp_dir().join(format!(
            "xianxia-pending-order-{}.snapshot",
            std::process::id()
        ));
        w.save(&path).unwrap();
        let replay = World::load(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        assert_eq!(w.commitments, replay.commitments);
        w = replay;
        let id = w
            .urban
            .jobs
            .iter()
            .position(|j| {
                j.as_ref()
                    .is_some_and(|j| j.target == shop as u32 && j.kind == 1)
            })
            .unwrap();
        assert_eq!(w.urban.buildings[factory].wares[3], 0);
        assert_eq!(w.urban.buildings[shop].wares[3], 0);
        at(&mut w, 3, factory, Action::UrbanWork);
        w.agents.target[3] = id as u32;
        for _ in 0..12 {
            w.execute_urban_job(3);
        }
        at(&mut w, 3, shop, Action::UrbanWork);
        assert_eq!(
            w.action_memory_inputs(2, Action::OrderGoods, shop as u32),
            neutral
        );
        w.agents.target[3] = id as u32;
        for _ in 0..12 {
            w.execute_urban_job(3);
        }
        assert_eq!(w.urban.buildings[shop].wares[3], 1);
        assert_eq!(
            w.action_memory_inputs(2, Action::OrderGoods, shop as u32)
                .unwrap()[0],
            1.0
        );
        assert_eq!(
            w.action_memory_inputs(3, Action::OrderGoods, shop as u32),
            neutral
        );
        let home = construct(&mut w, 0);
        at(&mut w, 4, home, Action::RentHome);
        w.execute_urban(4, Action::RentHome);
        at(&mut w, 2, shop, Action::OperateBusiness);
        w.operate_business(2, shop);
        let before = w.agents.coins[4];
        w.buy_urban_goods(4, shop, 16);
        assert_eq!(w.agents.coins[4], before - 32);
        assert_eq!(w.urban.buildings[home].wares[3], 0);
        let private = w
            .urban
            .jobs
            .iter()
            .flatten()
            .find(|j| j.payer == 4)
            .unwrap();
        assert_eq!(private.target, home as u32);
        assert_eq!(private.wares[3], 1);
        w.buy_urban_goods(4, shop, 16);
        assert_eq!(w.agents.coins[4], before - 32);
        w.check().unwrap();
    }
    #[test]
    fn eviction_keeps_private_assets_and_checkpoint_continues_exactly() {
        let mut w = funded();
        let shop = construct(&mut w, 1);
        w.apply_urban_policy(0, "AdvertisePremises");
        at(&mut w, 1, shop, Action::LeaseBusiness);
        w.execute_urban(1, Action::LeaseBusiness);
        let coins = w.coins();
        w.vacate_business(shop);
        assert_eq!(w.urban.estates[0].cash, 50);
        assert_eq!(w.coins(), coins);
        w.agents.action[1] = Action::CollectEstate;
        w.agents.target[1] = 0;
        w.commitments.commit(1, Action::CollectEstate, 0, 1);
        w.execute_urban(1, Action::CollectEstate);
        assert_eq!(
            w.action_memory_inputs(1, Action::CollectEstate, 0).unwrap()[0],
            1.0
        );
        assert_eq!(w.urban.estates[0].cash, 0);
        assert_eq!(w.coins(), coins);
        w.check().unwrap();
        let path =
            std::env::temp_dir().join(format!("xianxia-urban-{}.snapshot", std::process::id()));
        w.save(&path).unwrap();
        let mut restored = World::load(&path).unwrap();
        for _ in 0..30 {
            w.step(false);
            restored.step(false);
        }
        assert_eq!(w.agents, restored.agents);
        assert_eq!(w.summary()["urban"], restored.summary()["urban"]);
        w.check().unwrap();
        restored.check().unwrap();
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn vacant_premises_post_specific_jobs_and_reopen_after_operator_leaves() {
        let mut w = funded();
        let shop = construct(&mut w, 1);
        let workshop = construct(&mut w, 2);
        let home = construct(&mut w, 0);
        assert!(w.operator_job_postings().is_empty());
        assert_eq!(w.operator_vacancy_pressure(0), 1.0);
        assert!(w
            .urban_council_candidates(0)
            .iter()
            .any(|(a, _, _)| *a == "AdvertisePremises"));
        w.apply_urban_policy(0, "AdvertisePremises");
        w.apply_urban_policy(0, "AdvertisePremises");
        let posts = w.operator_job_postings();
        assert_eq!(posts.len(), 2);
        assert!(posts.iter().all(|p| p["building"] != home));
        assert_eq!(posts[0]["title"], "Clothes shopkeeper");
        assert_eq!(posts[1]["title"], "Hunter");
        let coins = w.coins();
        at(&mut w, 1, shop, Action::LeaseBusiness);
        w.execute_urban(1, Action::LeaseBusiness);
        assert_eq!(w.operator_job_postings().len(), 1);
        assert_eq!(w.operator_vacancy_pressure(0), 0.5);
        at(&mut w, 2, shop, Action::LeaseBusiness);
        let cash = w.agents.coins[2];
        w.execute_urban(2, Action::LeaseBusiness);
        assert_eq!(w.agents.coins[2], cash);
        assert_eq!(w.urban.buildings[shop].owner, 1);
        w.vacate_business(shop);
        assert_eq!(w.operator_job_postings().len(), 1);
        w.apply_urban_policy(0, "AdvertisePremises");
        assert_eq!(w.operator_job_postings().len(), 2);
        w.urban.buildings[workshop].ready = false;
        assert_eq!(w.operator_job_postings().len(), 1);
        assert_eq!(w.coins(), coins);
        w.check().unwrap();
    }
    #[test]
    fn dead_operator_is_detected_before_the_next_rent_day() {
        let mut w = funded();
        let shop = construct(&mut w, 1);
        w.apply_urban_policy(0, "AdvertisePremises");
        at(&mut w, 1, shop, Action::LeaseBusiness);
        w.execute_urban(1, Action::LeaseBusiness);
        w.agents.alive[1] = false;
        w.tick = 1;
        w.urban_step();
        assert_eq!(w.urban.buildings[shop].owner, NONE);
        assert!(w
            .urban_council_candidates(0)
            .iter()
            .any(|(a, _, _)| *a == "AdvertisePremises"));
        w.apply_urban_policy(0, "AdvertisePremises");
        assert_eq!(w.operator_job_postings().len(), 1);
    }
    #[test]
    fn wealth_and_finite_materials_gate_districts_and_construction() {
        let mut w = World::new(256, 42).unwrap();
        let cash = w.markets[0].coins;
        w.markets[0].coins = 0;
        w.agents.coins[127] += cash;
        for k in 0..GOODS {
            let escrow = w.markets[0].quotes[k].escrow;
            w.markets[0].quotes[k].escrow = 0;
            w.markets[0].quotes[k].remaining = 0;
            w.agents.coins[127] += escrow;
        }
        w.markets[0].village = true;
        w.apply_urban_policy(0, "GazetteDistricts");
        assert!(!w.urban.states[0].zoned);
        w.prepare_urban_demo().unwrap();
        w.apply_urban_policy(0, "GazetteDistricts");
        assert!(w.urban.states[0].zoned);
        while w.markets[0].stock(1) > 0 {
            let q = w.markets[0].stock(1);
            let lot = w.markets[0].take(1, q).unwrap();
            w.ledger.consumed[1] += lot.quantity;
        }
        w.commission_urban(0, 0);
        assert!(w.urban.buildings.is_empty());
        w.check().unwrap();
    }
}
