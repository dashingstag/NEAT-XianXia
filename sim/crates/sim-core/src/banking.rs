//! Town-funded banks, annually fixed business loans, and conserved cash transfers.
use super::*;
use std::collections::BTreeMap;
pub(super) const YEAR: u64 = rest::DAY_TICKS * 365;
const INTEREST_DENOMINATOR: u64 = YEAR * 100;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Bank {
    pub market: u32,
    pub x: f32,
    pub y: f32,
    pub ready: bool,
    pub progress: f32,
    pub cash: u64,
    pub builder_escrow: u64,
    pub worker: u32,
    pub rate: u8,
    pub next_review: u64,
    pub sales_at_review: u64,
    pub last_year_sales: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    fn town() -> World {
        let mut w = World::new(1000, 42).unwrap();
        w.prepare_urban_demo().unwrap();
        w.apply_urban_policy(0, "GazetteDistricts");
        for policy in ["CommissionHomes", "CommissionShops", "CommissionWorkshops"] {
            for _ in 0..3 {
                let b = w.urban.buildings.len();
                w.apply_urban_policy(0, policy);
                assert_eq!(w.urban.buildings.len(), b + 1);
                w.urban.buildings[b].needs_clear = false;
                let j = w
                    .urban
                    .jobs
                    .iter()
                    .position(|j| {
                        j.as_ref()
                            .is_some_and(|j| j.kind == 0 && j.target == b as u32)
                    })
                    .unwrap();
                w.agents.x[0] = w.urban.buildings[b].x;
                w.agents.y[0] = w.urban.buildings[b].y;
                w.agents.action[0] = Action::UrbanWork;
                w.agents.target[0] = j as u32;
                for _ in 0..202 {
                    w.execute_urban(0, Action::UrbanWork);
                }
                assert!(w.urban.buildings[b].ready);
            }
        }
        w.apply_urban_policy(0, "AdvertisePremises");
        assert!(w.is_town(0));
        w.check().unwrap();
        w
    }
    fn open_bank(w: &mut World) {
        let coins = w.coins();
        w.apply_bank_policy(0, "FundBank");
        assert_eq!(w.banking.banks.len(), 1);
        w.agents.x[0] = w.banking.banks[0].x;
        w.agents.y[0] = w.banking.banks[0].y;
        w.agents.target[0] = 0;
        w.agents.action[0] = Action::BuildBank;
        w.commitments.commit(0, Action::BuildBank, 0, 1);
        for _ in 0..302 {
            w.execute_banking(0, Action::BuildBank);
        }
        assert!(w.banking.banks[0].ready);
        assert_eq!(
            w.action_memory_inputs(0, Action::BuildBank, 0).unwrap()[0],
            1.0
        );
        assert_eq!(w.coins(), coins);
        w.check().unwrap();
    }
    fn borrow(w: &mut World) {
        w.agents.home[20] = 0;
        w.people[20].aspiration = life::Aspiration::Shopkeeper;
        w.agents.x[20] = w.banking.banks[0].x;
        w.agents.y[20] = w.banking.banks[0].y;
        w.agents.target[20] = 0;
        w.agents.action[20] = Action::BorrowBusiness;
        assert!(w.can_seek_business_loan(20));
        let coins = w.coins();
        w.commitments.commit(20, Action::BorrowBusiness, 0, 1);
        w.execute_banking(20, Action::BorrowBusiness);
        assert_eq!(w.agents.coins[20], 58);
        assert_eq!(w.banking.loans[&20].principal, 58);
        assert_eq!(w.coins(), coins);
        w.check().unwrap();
    }
    #[test]
    fn only_towns_can_fund_a_bank_and_materials_cash_and_labor_are_required() {
        let mut village = World::new(1000, 42).unwrap();
        village.prepare_urban_demo().unwrap();
        village.apply_bank_policy(0, "FundBank");
        assert!(village.banking.banks.is_empty());
        let mut w = town();
        let before = w.stocks();
        open_bank(&mut w);
        assert_eq!(w.stocks()[1] + 20, before[1]);
        assert_eq!(w.stocks()[2] + 12, before[2]);
        w.apply_bank_policy(0, "FundBank");
        assert_eq!(w.banking.banks.len(), 1);
        let b = &w.banking.banks[0];
        assert!(w
            .urban
            .zones
            .iter()
            .any(|z| z.market == 0 && z.kind == 1 && dist(b.x, b.y, z.x, z.y) <= z.radius));
    }
    #[test]
    fn loans_move_existing_cash_block_duplicates_and_can_be_repaid_with_interest() {
        let mut w = town();
        open_bank(&mut w);
        borrow(&mut w);
        let coins = w.coins();
        w.execute_banking(20, Action::BorrowBusiness);
        assert_eq!(
            w.action_memory_inputs(20, Action::BorrowBusiness, 0)
                .unwrap()[0],
            1.0
        );
        assert_eq!(w.banking.approved[&20], 1);
        assert_eq!(w.agents.coins[20], 58);
        w.tick = YEAR;
        w.banking_step();
        assert_eq!(w.banking.loans[&20].interest, 2);
        assert_eq!(w.loan_debt(20), 61);
        assert_eq!(w.coins(), coins);
        w.agents.coins[0] -= 100;
        w.agents.coins[20] += 100;
        w.commitments.commit(20, Action::RepayLoan, 0, 1);
        w.execute_banking(20, Action::RepayLoan);
        assert_eq!(
            w.action_memory_inputs(20, Action::RepayLoan, 0).unwrap()[0],
            1.0
        );
        assert!(!w.banking.loans.contains_key(&20));
        assert_eq!(w.banking.principal_paid, 58);
        assert_eq!(w.banking.interest_paid, 3);
        assert_eq!(w.coins(), coins);
        w.check().unwrap();
    }
    #[test]
    fn annual_trade_reviews_move_exactly_one_point_and_loans_keep_their_yearly_rate() {
        let mut w = town();
        open_bank(&mut w);
        w.banking.banks[0].last_year_sales = 100;
        w.tick = YEAR / 2;
        borrow(&mut w);
        w.markets[0].sales[0] += 200;
        w.tick = YEAR;
        w.banking_step();
        assert_eq!(w.banking.banks[0].rate, 6);
        assert_eq!(w.banking.loans[&20].rate, 5);
        w.tick = YEAR + YEAR / 2;
        w.banking_step();
        assert_eq!(w.banking.loans[&20].rate, 6);
        w.markets[0].sales[0] += 50;
        w.tick = YEAR * 2;
        w.banking_step();
        assert_eq!(w.banking.banks[0].rate, 5);
        assert_eq!(w.banking.loans[&20].rate, 6);
        w.check().unwrap();
    }
    #[test]
    fn shopkeeper_goal_requires_a_funded_bank_and_rewards_real_approval() {
        let mut w = town();
        w.agents.home[20] = 0;
        w.people[20].aspiration = life::Aspiration::Shopkeeper;
        w.people[20].goals.clear();
        w.life_step();
        assert!(!w.people[20]
            .goals
            .iter()
            .any(|g| g.kind == life::GoalKind::BusinessLoan));
        open_bank(&mut w);
        w.people[20].goals.clear();
        w.life_step();
        assert!(w.people[20]
            .goals
            .iter()
            .any(|g| g.kind == life::GoalKind::BusinessLoan));
        let choices = w.trace(20).unwrap().choices;
        assert!(choices
            .iter()
            .any(|c| c.action == "BorrowBusiness" && c.inputs[33] > 0.0));
        let before = w.stats.goals_completed;
        borrow(&mut w);
        w.tick = 10;
        w.life_step();
        assert!(w.stats.goals_completed > before);
        assert!(!w.people[20]
            .goals
            .iter()
            .any(|g| g.kind == life::GoalKind::BusinessLoan));
        w.check().unwrap();
    }
    #[test]
    fn no_remote_loan_or_lending_beyond_cash_reserves() {
        let mut w = town();
        open_bank(&mut w);
        w.agents.home[20] = 0;
        w.agents.target[20] = 0;
        w.agents.x[20] = 10.0;
        w.agents.y[20] = 10.0;
        let coins = w.coins();
        w.execute_banking(20, Action::BorrowBusiness);
        assert!(!w.banking.loans.contains_key(&20));
        assert_eq!(w.coins(), coins);
        let q = w.banking.banks[0].cash - 50;
        w.banking.banks[0].cash -= q;
        w.markets[0].coins += q;
        assert!(!w.can_seek_business_loan(20));
        w.agents.x[20] = w.banking.banks[0].x;
        w.agents.y[20] = w.banking.banks[0].y;
        w.execute_banking(20, Action::BorrowBusiness);
        assert!(!w.banking.loans.contains_key(&20));
        w.check().unwrap();
    }
    #[test]
    fn borrowing_does_not_increase_net_worth_and_active_loans_replay_exactly() {
        let mut w = town();
        open_bank(&mut w);
        borrow(&mut w);
        // Raising this wallet with a conserved transfer makes the borrower visible in the top 30.
        w.agents.coins[0] -= 200;
        w.agents.coins[20] += 200;
        let board = w.wealth_leaderboard();
        let row = board["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == 20)
            .unwrap();
        assert_eq!(row["loan_debt"], 58);
        assert_eq!(
            row["net_worth"].as_i64().unwrap(),
            row["coins"].as_i64().unwrap() + row["assets_value"].as_i64().unwrap() - 58
        );
        let p = std::env::temp_dir().join(format!("xianxia-banks-{}.snapshot", std::process::id()));
        w.save(&p).unwrap();
        let mut restored = World::load(&p).unwrap();
        std::fs::remove_file(p).unwrap();
        for _ in 0..30 {
            w.step(false);
            restored.step(false);
        }
        assert_eq!(w.banking_summary(), restored.banking_summary());
        assert_eq!(w.agents, restored.agents);
        w.check().unwrap();
        restored.check().unwrap();
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Loan {
    pub bank: u32,
    pub principal: u64,
    pub interest: u64,
    pub remainder: u64,
    pub rate: u8,
    pub opened: u64,
    pub last_accrual: u64,
    pub next_reprice: u64,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Banking {
    pub banks: Vec<Bank>,
    pub loans: BTreeMap<u32, Loan>,
    pub approved: BTreeMap<u32, u64>,
    pub interest_paid: u64,
    pub principal_paid: u64,
}
impl World {
    pub(super) fn is_bank_action(&self, a: Action) -> bool {
        matches!(
            a,
            Action::BuildBank | Action::BorrowBusiness | Action::RepayLoan
        )
    }
    pub(super) fn bank_coins(&self) -> u64 {
        self.banking
            .banks
            .iter()
            .map(|b| b.cash + b.builder_escrow)
            .sum()
    }
    pub(super) fn loan_debt(&self, i: usize) -> u64 {
        self.banking
            .loans
            .get(&(i as u32))
            .map(|l| l.principal + l.interest + u64::from(l.remainder > 0))
            .unwrap_or(0)
    }
    fn bank_trade_volume(&self, m: usize) -> u64 {
        self.markets[m].sales.iter().sum::<u64>()
            + self
                .urban
                .retail_sales
                .iter()
                .filter(|((market, _), _)| *market as usize == m)
                .map(|(_, n)| n)
                .sum::<u64>()
    }
    fn bank_site(&self, m: usize) -> Option<(f32, f32)> {
        let zone = self
            .urban
            .zones
            .iter()
            .find(|z| z.market as usize == m && z.kind == 1)?;
        (-3..=3)
            .flat_map(|a| {
                (-3..=3).map(move |b| (zone.x + a as f32 * 35.0, zone.y + b as f32 * 35.0))
            })
            .filter(|&(x, y)| {
                dist(x, y, zone.x, zone.y) <= zone.radius
                    && self.site_available(x, y)
                    && !self
                        .wildlife
                        .forests
                        .iter()
                        .any(|f| dist(x, y, f.x, f.y) < f.radius + 15.0)
            })
            .min_by(|a, b| {
                dist(a.0, a.1, zone.x, zone.y).total_cmp(&dist(b.0, b.1, zone.x, zone.y))
            })
    }
    pub(super) fn bank_council_candidates(&self, m: usize) -> Vec<(&'static str, u64, f32)> {
        if !self.is_town(m) {
            return vec![];
        }
        let existing = self.banking.banks.iter().find(|b| b.market as usize == m);
        if existing.is_none()
            && self.markets[m].coins >= 800
            && self.markets[m].stock(1) >= 20
            && self.markets[m].stock(2) >= 12
            && self.bank_site(m).is_some()
        {
            vec![("FundBank", 530, 0.9)]
        } else if existing.is_some_and(|b| b.ready && b.cash < 100) && self.markets[m].coins >= 700
        {
            vec![("CapitaliseBank", 500, 0.85)]
        } else {
            vec![]
        }
    }
    pub(super) fn apply_bank_policy(&mut self, m: usize, action: &str) {
        if !self
            .bank_council_candidates(m)
            .iter()
            .any(|(a, _, _)| *a == action)
        {
            return;
        }
        if action == "CapitaliseBank" {
            let b = self
                .banking
                .banks
                .iter_mut()
                .find(|b| b.market as usize == m)
                .unwrap();
            self.markets[m].coins -= 500;
            b.cash += 500;
            return;
        }
        let (x, y) = self.bank_site(m).unwrap();
        for (k, q) in [(1, 20), (2, 12)] {
            let mut left = q;
            while left > 0 {
                left -= self.markets[m].take(k, left).unwrap().quantity;
            }
            self.ledger.built[k] += q;
            self.markets[m].quotes[k].offered = self.markets[m].quotes[k]
                .offered
                .min(self.markets[m].stock(k));
        }
        self.markets[m].coins -= 530;
        self.banking.banks.push(Bank {
            market: m as u32,
            x,
            y,
            ready: false,
            progress: 0.0,
            cash: 500,
            builder_escrow: 30,
            worker: NONE,
            rate: 5,
            next_review: self.tick + YEAR,
            sales_at_review: self.bank_trade_volume(m),
            last_year_sales: 0,
        });
    }
    fn loan_amount(&self, i: usize) -> Option<u64> {
        if self.banking.loans.contains_key(&(i as u32))
            || !self.is_adult(i)
            || self.must_stay_home(i)
        {
            return None;
        }
        if let Some(b) = self
            .urban
            .buildings
            .iter()
            .find(|b| b.kind > 0 && b.owner == i as u32 && b.ready)
        {
            if b.cash < 20 && self.agents.coins[i] < 50 {
                return Some(50 - b.cash);
            }
            return None;
        }
        if self.agents.coins[i] >= 58 {
            return None;
        }
        self.urban
            .buildings
            .iter()
            .any(|b| {
                b.market == self.agents.home[i]
                    && b.kind > 0
                    && b.ready
                    && b.advertised
                    && b.owner == NONE
            })
            .then_some(58 - self.agents.coins[i])
    }
    pub(super) fn can_seek_business_loan(&self, i: usize) -> bool {
        let Some(q) = self.loan_amount(i) else {
            return false;
        };
        self.banking
            .banks
            .iter()
            .any(|b| b.ready && b.market == self.agents.home[i] && b.cash >= q + 50)
    }
    // Lending is a second neural decision, using the village's compiled policy brain.
    fn bank_will_lend(&self, i: usize, id: usize, q: u64) -> bool {
        let b = &self.banking.banks[id];
        if !b.ready || b.cash < q + 50 || b.market != self.agents.home[i] {
            return false;
        }
        let Some(brain) = self.councils[b.market as usize].brain.as_ref() else {
            return false;
        };
        let mut lend = [0.0; INPUTS];
        lend[0] = 0.85;
        lend[1] = 0.9;
        lend[2] = 0.15 + b.rate as f32 / 100.0;
        lend[4] = q as f32 / b.cash.max(1) as f32;
        lend[8] = (b.cash as f32 / 500.0).min(1.0);
        lend[9] = 1.0;
        let mut wait = lend;
        wait[0] = 0.1;
        wait[1] = 0.0;
        wait[2] = 0.0;
        wait[4] = 0.0;
        brain.evaluate(&lend) > brain.evaluate(&wait)
    }
    pub(super) fn bank_offers(&self, i: usize) -> Vec<Candidate> {
        let mut out = vec![];
        for (id, b) in self.banking.banks.iter().enumerate() {
            let (action, need, benefit, value) =
                if !b.ready && b.builder_escrow > 0 && (b.worker == NONE || b.worker == i as u32) {
                    (Action::BuildBank, 0.9, 0.95, 30.0)
                } else if b.ready
                    && self
                        .banking
                        .loans
                        .get(&(i as u32))
                        .is_some_and(|l| l.bank as usize == id)
                    && self.repayment_budget(i) > 0
                {
                    (
                        Action::RepayLoan,
                        if self.banking.loans[&(i as u32)].opened + YEAR <= self.tick {
                            1.0
                        } else {
                            0.6
                        },
                        0.9,
                        -1.0,
                    )
                } else if let Some(q) = self
                    .loan_amount(i)
                    .filter(|q| self.bank_will_lend(i, id, *q))
                {
                    (
                        Action::BorrowBusiness,
                        1.0,
                        0.9,
                        -(q as f32 * b.rate as f32 / 100.0).min(8.0),
                    )
                } else {
                    continue;
                };
            let mut x = [0.0; INPUTS];
            x[0] = need;
            x[1] = benefit;
            x[2] = if action == Action::BorrowBusiness {
                0.15 + b.rate as f32 / 100.0
            } else {
                0.01
            };
            x[3] = dist(self.agents.x[i], self.agents.y[i], b.x, b.y) / 1000.0;
            x[4] = value;
            x[11] = 0.8;
            out.push(Candidate {
                action,
                target: id as u32,
                kind: 1,
                inputs: x,
                score: 0.0,
            });
        }
        out
    }
    fn repayment_budget(&self, i: usize) -> u64 {
        self.agents.coins[i].saturating_sub(8)
            + self
                .urban
                .buildings
                .iter()
                .filter(|b| b.kind > 0 && b.owner == i as u32)
                .map(|b| b.cash.saturating_sub(50))
                .sum::<u64>()
    }
    pub(super) fn execute_banking(&mut self, i: usize, action: Action) {
        let id = self.agents.target[i] as usize;
        let Some(b) = self.banking.banks.get(id) else {
            return;
        };
        let (x, y) = (b.x, b.y);
        let d = dist(self.agents.x[i], self.agents.y[i], x, y);
        if d > 5.0 {
            let step = (self.actor_speed(i, self.agents.x[i], self.agents.y[i]) * DT).min(d - 4.0);
            self.agents.x[i] += (x - self.agents.x[i]) / d * step;
            self.agents.y[i] += (y - self.agents.y[i]) / d * step;
            return;
        }
        match action {
            Action::BuildBank => {
                let b = &mut self.banking.banks[id];
                if b.ready || b.worker != NONE && b.worker != i as u32 {
                    return;
                }
                b.worker = i as u32;
                b.progress += DT;
                if b.progress < 30.0 {
                    return;
                }
                let pay = b.builder_escrow;
                let m = b.market as usize;
                b.ready = true;
                b.worker = NONE;
                b.builder_escrow = 0;
                self.commitments
                    .resolve(i, Action::BuildBank, id as u32, 1, self.tick, true);
                self.agents.coins[i] += pay;
                self.people[i].jobs += 1;
                self.markets[m].expenses_total += pay;
                self.credit_education_bonus(i, m, pay);
            }
            Action::BorrowBusiness => {
                let Some(q) = self.loan_amount(i) else {
                    return;
                };
                if !self.bank_will_lend(i, id, q) {
                    self.commitments.resolve(
                        i,
                        Action::BorrowBusiness,
                        id as u32,
                        1,
                        self.tick,
                        false,
                    );
                    return;
                }
                let rate = self.banking.banks[id].rate;
                self.banking.banks[id].cash -= q;
                if let Some(b) = self
                    .urban
                    .buildings
                    .iter_mut()
                    .find(|b| b.kind > 0 && b.owner == i as u32 && b.ready)
                {
                    b.cash += q;
                } else {
                    if self.agents.coins[i] == 0 && self.agents.room(i) == 0 {
                        self.banking.banks[id].cash += q;
                        return;
                    }
                    self.agents.coins[i] += q;
                }
                self.banking.loans.insert(
                    i as u32,
                    Loan {
                        bank: id as u32,
                        principal: q,
                        interest: 0,
                        remainder: 0,
                        rate,
                        opened: self.tick,
                        last_accrual: self.tick,
                        next_reprice: self.tick + YEAR,
                    },
                );
                *self.banking.approved.entry(i as u32).or_default() += 1;
                self.commitments
                    .resolve(i, Action::BorrowBusiness, id as u32, 1, self.tick, true);
            }
            Action::RepayLoan => self.repay_bank_loan(i, id),
            _ => {}
        }
    }
    fn accrue_loan(&mut self, i: u32) {
        let Some(l) = self.banking.loans.get_mut(&i) else {
            return;
        };
        let mut at = l.last_accrual;
        while at < self.tick {
            let end = self.tick.min(l.next_reprice);
            let numerator =
                l.principal as u128 * l.rate as u128 * (end - at) as u128 + l.remainder as u128;
            l.interest += (numerator / INTEREST_DENOMINATOR as u128) as u64;
            l.remainder = (numerator % INTEREST_DENOMINATOR as u128) as u64;
            at = end;
            if at == l.next_reprice {
                l.rate = self.banking.banks[l.bank as usize].rate;
                l.next_reprice += YEAR;
            }
        }
        l.last_accrual = self.tick;
    }
    fn repay_bank_loan(&mut self, i: usize, id: usize) {
        if self
            .banking
            .loans
            .get(&(i as u32))
            .is_none_or(|l| l.bank as usize != id)
        {
            return;
        }
        self.accrue_loan(i as u32);
        let budget = self.repayment_budget(i);
        if budget == 0 {
            return;
        }
        let l = self.banking.loans.get_mut(&(i as u32)).unwrap();
        if l.remainder > 0 && budget >= l.principal + l.interest {
            l.interest += 1;
            l.remainder = 0;
        }
        let pay = budget.min(l.principal + l.interest);
        let interest = pay.min(l.interest);
        l.interest -= interest;
        l.principal -= pay - interest;
        let mut left = pay;
        let from_wallet = left.min(self.agents.coins[i].saturating_sub(8));
        self.agents.coins[i] -= from_wallet;
        left -= from_wallet;
        for b in &mut self.urban.buildings {
            if b.kind > 0 && b.owner == i as u32 {
                let q = left.min(b.cash.saturating_sub(50));
                b.cash -= q;
                left -= q;
            }
        }
        self.banking.banks[id].cash += pay;
        self.banking.interest_paid += interest;
        self.banking.principal_paid += pay - interest;
        if pay > 0 {
            self.commitments
                .resolve(i, Action::RepayLoan, id as u32, 1, self.tick, true);
        }
        if l.principal == 0 && l.interest == 0 {
            self.banking.loans.remove(&(i as u32));
        }
    }
    pub(super) fn banking_step(&mut self) {
        if self.tick % 10 != 0 {
            return;
        }
        for id in 0..self.banking.banks.len() {
            let b = &self.banking.banks[id];
            let worker = b.worker;
            let m = b.market as usize;
            if worker != NONE
                && (!self.agents.alive[worker as usize]
                    || self.agents.action[worker as usize] != Action::BuildBank
                    || self.agents.target[worker as usize] != id as u32
                    || self.must_stay_home(worker as usize))
            {
                self.banking.banks[id].worker = NONE;
            }
            while self.tick >= self.banking.banks[id].next_review {
                let volume = self.bank_trade_volume(m);
                let b = &mut self.banking.banks[id];
                let sales = volume.saturating_sub(b.sales_at_review);
                b.rate = if sales > b.last_year_sales {
                    (b.rate + 1).min(20)
                } else if sales < b.last_year_sales {
                    b.rate.saturating_sub(1).max(1)
                } else {
                    b.rate
                };
                b.sales_at_review = volume;
                b.last_year_sales = sales;
                b.next_review += YEAR;
            }
        }
        let ids = self.banking.loans.keys().copied().collect::<Vec<_>>();
        for i in ids {
            self.accrue_loan(i);
        }
    }
    pub(super) fn banking_summary(&self) -> serde_json::Value {
        serde_json::json!({"year_days":365,"initial_rate_percent":5,"annual_step_percentage_points":1,"banks":self.banking.banks.iter().enumerate().map(|(id,b)|serde_json::json!({"id":id,"market":b.market,"x":b.x,"y":b.y,"ready":b.ready,"cash":b.cash,"builder_reward":b.builder_escrow,"rate_percent":b.rate,"days_to_review":b.next_review.saturating_sub(self.tick) as f64/rest::DAY_TICKS as f64,"active_loans":self.banking.loans.values().filter(|l|l.bank as usize==id).count(),"principal_outstanding":self.banking.loans.values().filter(|l|l.bank as usize==id).map(|l|l.principal).sum::<u64>(),"interest_outstanding":self.banking.loans.values().filter(|l|l.bank as usize==id).map(|l|l.interest+u64::from(l.remainder>0)).sum::<u64>()})).collect::<Vec<_>>(),"loans":self.banking.loans.iter().map(|(&i,l)|serde_json::json!({"person":i,"bank":l.bank,"principal":l.principal,"interest":l.interest+u64::from(l.remainder>0),"debt":self.loan_debt(i as usize),"annual_rate_percent":l.rate,"days_to_reprice":l.next_reprice.saturating_sub(self.tick) as f64/rest::DAY_TICKS as f64,"overdue":self.tick>=l.opened+YEAR})).collect::<Vec<_>>(),"interest_paid":self.banking.interest_paid,"principal_paid":self.banking.principal_paid})
    }
    pub(super) fn validate_banking(&self) -> Result<(), String> {
        for b in &self.banking.banks {
            if b.market as usize >= self.markets.len()
                || !b.x.is_finite()
                || !b.y.is_finite()
                || !(1..=20).contains(&b.rate)
                || b.worker != NONE && b.worker as usize >= self.agents.len()
            {
                return Err("Invalid bank".into());
            }
        }
        for (&i, l) in &self.banking.loans {
            if i as usize >= self.agents.len()
                || l.bank as usize >= self.banking.banks.len()
                || !(1..=20).contains(&l.rate)
                || l.remainder >= INTEREST_DENOMINATOR
                || l.last_accrual > self.tick
                || l.next_reprice <= l.last_accrual
            {
                return Err("Invalid business loan".into());
            }
        }
        Ok(())
    }
}
