//! Read-only wealth estimates. No money or inventory is changed by valuation.
use super::*;
use std::{
    cmp::Reverse,
    collections::{BTreeMap, BinaryHeap},
};

/// Per-viewer subscription. Same-tick edits invalidate the cached revision.
#[derive(Default)]
pub struct WealthView {
    enabled: bool,
    cached: Option<(u64, serde_json::Value)>,
    builds: u64,
}

impl WealthView {
    pub fn subscribe(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.invalidate();
        }
    }
    pub fn invalidate(&mut self) {
        self.cached = None;
    }
    pub fn builds(&self) -> u64 {
        self.builds
    }
    pub fn report(&mut self, world: &World) -> Option<&serde_json::Value> {
        if !self.enabled {
            return None;
        }
        if self
            .cached
            .as_ref()
            .is_none_or(|(tick, _)| *tick != world.tick)
        {
            self.cached = Some((world.tick, world.wealth_leaderboard()));
            self.builds += 1;
        }
        self.cached.as_ref().map(|(_, board)| board)
    }
}

// Copper reference prices, independent of fluctuating bids and asks.
const RAW_VALUE: [u64; GOODS] = [2, 4, 3, 3, 100, 10, 1, 6, 3, 18, 18, 18, 18];
const WARE_VALUE: [u64; 5] = [9, 18, 16, 30, 4];
#[derive(Clone, Copy, Default)]
struct Holdings {
    business_cash: u64,
    stored_cash: u64,
    homes: u64,
    stock: u64,
    stored_goods: u64,
    debt: u64,
    home_count: u32,
    business_count: u32,
}
fn inventory(raw: &[u64; GOODS], wares: &[u64; 5]) -> u64 {
    raw.iter().zip(RAW_VALUE).map(|(q, p)| q * p).sum::<u64>()
        + wares
            .iter()
            .zip(WARE_VALUE)
            .map(|(q, p)| q * p)
            .sum::<u64>()
}
impl World {
    fn wealth_holdings(&self) -> BTreeMap<u32, Holdings> {
        let mut holdings: BTreeMap<u32, Holdings> = BTreeMap::new();
        for b in &self.urban.buildings {
            if !b.ready {
                continue;
            }
            if b.kind == 0 {
                if b.owner != NONE {
                    let h = holdings.entry(b.owner).or_default();
                    h.homes += b.price;
                    h.home_count += 1;
                }
                let tenant = if b.owner != NONE { b.owner } else { b.renter };
                if tenant != NONE {
                    let h = holdings.entry(tenant).or_default();
                    h.stored_goods += inventory(&b.raw, &b.wares);
                    h.debt += b.arrears;
                }
            } else if b.owner != NONE {
                let h = holdings.entry(b.owner).or_default();
                h.business_cash += b.cash;
                h.stock += inventory(&b.raw, &b.wares);
                h.debt += b.arrears;
                h.business_count += 1;
            }
        }
        for e in &self.urban.estates {
            if e.owner == NONE {
                continue;
            }
            let h = holdings.entry(e.owner).or_default();
            h.stored_cash += e.cash;
            h.stored_goods += inventory(&e.raw, &e.wares);
        }
        for j in self.urban.jobs.iter().flatten() {
            if j.payer != NONE {
                holdings.entry(j.payer).or_default().stored_goods += inventory(&j.raw, &j.wares);
            }
        }
        for (&(i, k), q) in &self.urban.owned {
            holdings.entry(i).or_default().stored_goods += q * WARE_VALUE[k as usize];
        }
        holdings
    }

    fn possessions_value(&self, i: usize) -> u64 {
        let a = &self.agents;
        a.berries[i] as u64 * RAW_VALUE[0]
            + a.fish[i] as u64 * RAW_VALUE[3]
            + a.rice[i] as u64 * RAW_VALUE[8]
            + a.cargo[i] as u64 * RAW_VALUE[a.cargo_kind[i] as usize]
            + a.tools[i]
                .iter()
                .enumerate()
                .map(|(k, q)| *q as u64 * RAW_VALUE[k + 9])
                .sum::<u64>()
    }

    fn net_worth(&self, i: usize, h: Holdings) -> (i128, u64) {
        let cash = self.agents.coins[i] + h.business_cash + h.stored_cash;
        let net = (cash + h.homes + h.stock + h.stored_goods + self.possessions_value(i)) as i128
            - h.debt as i128
            - self.loan_debt(i) as i128;
        (net, cash)
    }

    pub(super) fn richest_resident_net_worth(&self, market: usize) -> i128 {
        let holdings = self.wealth_holdings();
        (0..self.agents.len())
            .filter(|&i| self.agents.alive[i] && self.agents.home[i] as usize == market)
            .map(|i| {
                self.net_worth(i, holdings.get(&(i as u32)).copied().unwrap_or_default())
                    .0
            })
            .max()
            .unwrap_or(0)
    }

    pub(super) fn wealth_leaderboard(&self) -> serde_json::Value {
        let holdings = self.wealth_holdings();
        // Only retain the best 30: O(population log 30), without sorting or copying all people.
        let mut best = BinaryHeap::new();
        for i in 0..self.agents.len() {
            if !self.agents.alive[i] {
                continue;
            }
            let h = holdings.get(&(i as u32)).copied().unwrap_or_default();
            let (net, cash) = self.net_worth(i, h);
            let candidate = (net, cash, Reverse(i));
            if best.len() < 30 {
                best.push(Reverse(candidate));
            } else if best.peek().is_some_and(|Reverse(worst)| candidate > *worst) {
                best.pop();
                best.push(Reverse(candidate));
            }
        }
        let mut ranked = best.into_iter().map(|Reverse(row)| row).collect::<Vec<_>>();
        ranked.sort_unstable_by(|a, b| b.cmp(a));
        let rows=ranked.into_iter().enumerate().map(|(rank,(net,cash,Reverse(i)))| {
            let h=holdings.get(&(i as u32)).copied().unwrap_or_default();
            let belongings=self.possessions_value(i)+h.stored_goods;
            serde_json::json!({"rank":rank+1,"id":i,"name":self.people[i].name(),"village":self.markets[self.agents.home[i] as usize].name,"net_worth":net,"coins":cash,"wallet":self.agents.coins[i],"business_cash":h.business_cash,"stored_cash":h.stored_cash,"homes_value":h.homes,"business_stock":h.stock,"belongings":belongings,"assets_value":h.homes+h.stock+belongings,"arrears":h.debt,"loan_debt":self.loan_debt(i),"debt":h.debt+self.loan_debt(i),"homes":h.home_count,"businesses":h.business_count})
        }).collect::<Vec<_>>();
        serde_json::json!({"scope":"Living individuals / top 30","currency":"copper","valuation":"Estimated net worth = all owned coins + homes + inventory and belongings - rent arrears and bank debt. Leased premises remain village property. Fixed reference prices; not guaranteed sale proceeds. Pending job rewards are excluded.","raw_reference_prices":RAW_VALUE,"ware_reference_prices":WARE_VALUE,"rows":rows})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn subscription_reuses_only_the_current_world_revision() {
        let mut w = World::new(64, 10).unwrap();
        let mut view = WealthView::default();
        assert!(view.report(&w).is_none());
        assert_eq!(view.builds(), 0);
        view.subscribe(true);
        for _ in 0..3 {
            assert_eq!(view.report(&w), Some(&w.wealth_leaderboard()));
        }
        assert_eq!(view.builds(), 1);
        w.step(false);
        assert_eq!(view.report(&w), Some(&w.wealth_leaderboard()));
        assert_eq!(view.builds(), 2);
        w.agents.coins[9] += 1000;
        view.invalidate();
        assert_eq!(view.report(&w), Some(&w.wealth_leaderboard()));
        assert_eq!(view.builds(), 3);
        view.subscribe(false);
        assert!(view.report(&w).is_none());
        view.subscribe(true);
        assert_eq!(view.report(&w), Some(&w.wealth_leaderboard()));
        assert_eq!(view.builds(), 4);
    }
    #[test]
    fn ranking_is_stable_excludes_dead_and_does_not_change_the_world() {
        let mut w = World::new(64, 42).unwrap();
        w.agents.coins[10] += 100;
        w.agents.coins[11] += 100;
        w.agents.coins[12] += 200;
        w.agents.alive[12] = false;
        let before = w.agents.clone();
        let coins = w.coins();
        let stocks = w.stocks();
        let board = w.wealth_leaderboard();
        let rows = board["rows"].as_array().unwrap();
        assert_eq!(rows.len(), 30);
        assert_eq!(rows[0]["id"], 10);
        assert_eq!(rows[1]["id"], 11);
        assert!(rows.iter().all(|r| r["id"] != 12));
        assert_eq!(w.agents, before);
        assert_eq!(w.coins(), coins);
        assert_eq!(w.stocks(), stocks);
    }
    #[test]
    fn stored_cash_and_personal_goods_are_valued_separately() {
        let mut w = World::new(2, 42).unwrap();
        w.urban.estates.push(urban::Estate {
            owner: 0,
            x: 0.0,
            y: 0.0,
            cash: 10,
            raw: [0; GOODS],
            wares: [0, 0, 0, 1, 0],
        });
        w.urban.owned.insert((0, 2), 1);
        let board = w.wealth_leaderboard();
        let row = &board["rows"][0];
        assert_eq!(row["id"], 0);
        assert_eq!(row["stored_cash"], 10);
        assert_eq!(
            row["belongings"].as_u64().unwrap(),
            w.agents.berries[0] as u64 * 2
                + w.agents.fish[0] as u64 * 3
                + w.agents.rice[0] as u64 * 3
                + 46
        );
        assert_eq!(
            row["net_worth"].as_i64().unwrap(),
            row["coins"].as_i64().unwrap() + row["assets_value"].as_i64().unwrap()
                - row["arrears"].as_i64().unwrap()
        );
    }
    #[test]
    fn leased_premises_are_excluded_but_inventory_transit_and_debt_are_included() {
        let mut w = World::new(256, 42).unwrap();
        w.prepare_urban_demo().unwrap();
        w.apply_urban_policy(0, "GazetteDistricts");
        w.apply_urban_policy(0, "CommissionHomes");
        w.apply_urban_policy(0, "CommissionWorkshops");
        assert_eq!(w.urban.buildings.len(), 2);
        w.urban.buildings[0].ready = true;
        w.urban.buildings[0].owner = 0;
        w.urban.buildings[1].ready = true;
        w.urban.buildings[1].owner = 0;
        w.urban.buildings[1].cash = 50;
        w.urban.buildings[1].arrears = 7;
        w.urban.buildings[1].raw[1] = 2;
        let mut raw = [0; GOODS];
        raw[1] = 3;
        w.urban.jobs.push(Some(urban::Job {
            market: 0,
            kind: 1,
            payer: 0,
            firm: 1,
            source: NONE,
            target: 1,
            worker: 1,
            phase: 1,
            x: 0.0,
            y: 0.0,
            raw,
            wares: [0; 5],
            reward: 2,
            progress: 0.0,
        }));
        let board = w.wealth_leaderboard();
        let owner = board["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == 0)
            .unwrap();
        assert_eq!(owner["homes_value"], 120);
        assert_eq!(owner["business_stock"], 8);
        assert_eq!(owner["business_cash"], 50);
        assert_eq!(owner["arrears"], 7);
        let held = w.agents.berries[0] as u64 * 2
            + w.agents.fish[0] as u64 * 3
            + w.agents.rice[0] as u64 * 3;
        assert_eq!(owner["belongings"].as_u64().unwrap(), held + 12);
        assert_eq!(owner["assets_value"].as_u64().unwrap(), 120 + 8 + held + 12);
        assert_eq!(
            owner["net_worth"].as_i64().unwrap(),
            w.agents.coins[0] as i64 + 50 + 120 + 8 + held as i64 + 12 - 7
        );
    }
}
