use super::*;
use std::collections::BTreeMap;

const LOCAL_MARKET_RANGE: f32 = 120.0;
const OBSERVATION_INTERVAL: u64 = 300;
const REPORT_LIFETIME: u64 = rest::DAY_TICKS * 3;
const REPORT_REFRESH: u64 = rest::DAY_TICKS / 2;
const SALE_PRICE: u64 = 1;
const PURCHASE_PRICE: u64 = 2;
pub(super) const MARKET_SLOTS: u32 = 64;

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Report {
    tick: u64,
    quotes: [Quote; GOODS],
    meat: Quote,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn travelers_supply_reports_that_can_be_bought_replayed_and_expire() {
        let mut w = World::new_with_size(8, 42, Hill::default(), MapSize::X4).unwrap();
        let host = 0;
        let observed = 1;
        let (tx, ty) = (w.markets[host].x, w.markets[host].y);
        w.taverns.push(Tavern {
            market: host as u32,
            x: tx,
            y: ty,
            ready: true,
            progress: 20.0,
            escrow: 0,
            visitors: 0,
            guests: vec![],
        });
        for i in 0..2 {
            w.people[i].initial_age_days = 25 * 365;
            w.agents.x[i] = tx;
            w.agents.y[i] = ty;
        }
        let target = observed as u32;
        let coins = w.coins();
        let wallets = [w.agents.coins[0], w.agents.coins[1]];
        assert_eq!(
            w.perceived_quote(1, &w.markets[observed], 0),
            Quote::default()
        );
        assert!(w.information_offers(1).is_empty());
        (w.agents.x[0], w.agents.y[0]) = (w.markets[observed].x, w.markets[observed].y);
        w.observe_local_market(0);
        let quote = w.perceived_quote(0, &w.markets[observed], 0);
        (w.agents.x[0], w.agents.y[0]) = (tx, ty);
        w.tick = 300;
        let (offers, n) = w.offer(0);
        assert!(offers[..n]
            .iter()
            .any(|c| c.action == Action::SellInformation && c.target == target));
        w.agents.action[0] = Action::SellInformation;
        w.agents.target[0] = target;
        w.execute(0);
        assert_eq!(w.agents.coins[0], wallets[0] + SALE_PRICE);
        assert!(w.information_offers(0).is_empty());
        let (offers, n) = w.offer(1);
        assert!(offers[..n]
            .iter()
            .any(|c| c.action == Action::BuyInformation && c.target == target));
        w.agents.action[1] = Action::BuyInformation;
        w.agents.target[1] = target;
        w.execute(1);
        assert_eq!(w.agents.coins[1], wallets[1] - PURCHASE_PRICE);
        assert_eq!(w.perceived_quote(1, &w.markets[observed], 0).ask, quote.ask);
        assert_eq!(w.remembered_market(1, target).unwrap().tick, 0);
        assert_eq!(w.coins(), coins);
        let checkpoint =
            std::env::temp_dir().join(format!("neat-info-{}.snapshot", std::process::id()));
        w.save(&checkpoint).unwrap();
        let mut replay = World::load(&checkpoint).unwrap();
        std::fs::remove_file(checkpoint).unwrap();
        assert_eq!(
            w.checkpoint_payload().unwrap(),
            replay.checkpoint_payload().unwrap()
        );
        replay.tick = REPORT_LIFETIME;
        assert_eq!(
            replay.perceived_quote(1, &replay.markets[observed], 0),
            Quote::default()
        );
        assert!(replay.tavern_report(0, target).is_none());
        assert!(replay.information_offers(1).is_empty());
        replay.expire_market_reports();
        assert!(replay.market_information.taverns.is_empty());
        replay.check().unwrap();
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(super) struct MarketInformation {
    personal: BTreeMap<u32, BTreeMap<u32, Report>>,
    taverns: BTreeMap<u32, BTreeMap<u32, Report>>,
    bought: u64,
    sold: u64,
}

fn fresh(report: &Report, tick: u64) -> bool {
    tick.saturating_sub(report.tick) < REPORT_LIFETIME
}

impl World {
    pub(super) fn local_market_information(&self, i: usize, m: &Market) -> bool {
        self.map.spec.size == MapSize::Classic
            || dist(self.agents.x[i], self.agents.y[i], m.x, m.y) <= LOCAL_MARKET_RANGE
    }

    fn remembered_market(&self, i: usize, market: u32) -> Option<&Report> {
        self.market_information
            .personal
            .get(&(i as u32))?
            .get(&market)
            .filter(|report| fresh(report, self.tick))
    }

    pub(super) fn observe_local_market(&mut self, i: usize) {
        if self.map.spec.size == MapSize::Classic || !self.agents.alive[i] || !self.is_adult(i) {
            return;
        }
        let a = &self.agents;
        let Some(m) = self
            .markets
            .iter()
            .filter_map(|m| {
                let d = dist(a.x[i], a.y[i], m.x, m.y);
                (d <= LOCAL_MARKET_RANGE).then_some((m, d))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(m, _)| m)
        else {
            return;
        };
        if self
            .remembered_market(i, m.id)
            .is_some_and(|report| self.tick - report.tick < OBSERVATION_INTERVAL)
        {
            return;
        }
        let report = Report {
            tick: self.tick,
            quotes: std::array::from_fn(|k| {
                let mut q = m.quotes[k].clone();
                q.escrow = 0;
                q
            }),
            meat: self.meat_trade.live_quote(m, self.tick),
        };
        self.market_information
            .personal
            .entry(i as u32)
            .or_default()
            .insert(m.id, report);
    }

    pub(super) fn expire_market_reports(&mut self) {
        if self.tick % OBSERVATION_INTERVAL != 0 {
            return;
        }
        self.market_information.personal.retain(|&i, reports| {
            reports.retain(|_, report| fresh(report, self.tick));
            self.agents.alive[i as usize] && !reports.is_empty()
        });
        self.market_information.taverns.retain(|_, reports| {
            reports.retain(|_, report| fresh(report, self.tick));
            !reports.is_empty()
        });
    }

    pub(super) fn perceived_quote(&self, i: usize, m: &Market, kind: usize) -> Quote {
        if self.local_market_information(i, m) {
            return m.quotes[kind].clone();
        }
        self.remembered_market(i, m.id)
            .map(|report| report.quotes[kind].clone())
            .unwrap_or_default()
    }

    pub(super) fn perceived_market_stock(&self, i: usize, m: &Market, kind: usize) -> u64 {
        if self.local_market_information(i, m) {
            m.stock(kind)
        } else {
            self.perceived_quote(i, m, kind).offered
        }
    }

    pub(super) fn perceived_meat_quote(&self, i: usize, m: &Market) -> Quote {
        if self.local_market_information(i, m) {
            return self.meat_trade.live_quote(m, self.tick);
        }
        self.remembered_market(i, m.id)
            .map(|report| report.meat.clone())
            .unwrap_or_default()
    }

    pub(super) fn market_information_risk(&self, i: usize, m: &Market) -> f32 {
        if self.local_market_information(i, m) {
            return 0.0;
        }
        let distance = dist(self.agents.x[i], self.agents.y[i], m.x, m.y);
        let age = self
            .remembered_market(i, m.id)
            .map(|report| self.tick.saturating_sub(report.tick))
            .unwrap_or(REPORT_LIFETIME);
        0.03 + 0.07 * age as f32 / REPORT_LIFETIME as f32
            + 0.06 * ((distance - LOCAL_MARKET_RANGE) / 1200.0).clamp(0.0, 1.0)
    }

    fn tavern_report(&self, tavern: u32, market: u32) -> Option<&Report> {
        self.market_information
            .taverns
            .get(&tavern)?
            .get(&market)
            .filter(|report| fresh(report, self.tick))
    }

    fn information_terms(
        &self,
        i: usize,
        tavern: u32,
        market: u32,
        action: Action,
    ) -> Option<&Report> {
        let site = self.taverns.get(tavern as usize)?;
        if !site.ready
            || site.market == market
            || !self.agents.alive[i]
            || !self.is_adult(i)
            || self.must_stay_home(i)
        {
            return None;
        }
        let personal = self.remembered_market(i, market);
        let posted = self.tavern_report(tavern, market);
        match action {
            Action::BuyInformation
                if self.agents.coins[i] >= PURCHASE_PRICE
                    && !self.local_market_information(i, self.markets.get(market as usize)?) =>
            {
                posted.filter(|report| {
                    personal.is_none_or(|known| {
                        report.tick.saturating_sub(known.tick) >= REPORT_REFRESH
                    })
                })
            }
            Action::SellInformation
                if self.markets[site.market as usize].coins >= SALE_PRICE
                    && (self.agents.coins[i] > 0 || self.agents.room(i) > 0) =>
            {
                personal.filter(|report| posted.is_none_or(|known| known.tick < report.tick))
            }
            _ => None,
        }
    }

    pub(super) fn information_offers(&self, i: usize) -> Vec<Candidate> {
        let mut offers = Vec::new();
        for (id, site) in self.taverns.iter().enumerate() {
            let travel = dist(self.agents.x[i], self.agents.y[i], site.x, site.y);
            if !site.ready || travel > LOCAL_MARKET_RANGE {
                continue;
            }
            let sources = [
                (
                    Action::BuyInformation,
                    self.market_information.taverns.get(&(id as u32)),
                    -(PURCHASE_PRICE as f32),
                ),
                (
                    Action::SellInformation,
                    self.market_information.personal.get(&(i as u32)),
                    SALE_PRICE as f32,
                ),
            ];
            for (action, reports, price) in sources {
                let choice = reports
                    .into_iter()
                    .flat_map(|reports| reports.keys())
                    .filter_map(|&market| {
                        self.information_terms(i, id as u32, market, action)
                            .map(|r| (market, r))
                    })
                    .max_by_key(|&(market, report)| (report.tick, std::cmp::Reverse(market)));
                if let Some((market, report)) = choice {
                    let freshness = 1.0 - (self.tick - report.tick) as f32 / REPORT_LIFETIME as f32;
                    let mut c = Candidate {
                        action,
                        target: id as u32 * MARKET_SLOTS + market,
                        ..Default::default()
                    };
                    c.inputs[..5].copy_from_slice(&[
                        0.55,
                        0.45 + 0.5 * freshness,
                        0.0,
                        travel,
                        price,
                    ]);
                    offers.push(c);
                }
            }
        }
        offers
    }

    pub(super) fn information_destination(&self, i: usize) -> Option<(f32, f32)> {
        self.taverns
            .get((self.agents.target[i] / MARKET_SLOTS) as usize)
            .map(|site| (site.x, site.y))
    }

    pub(super) fn execute_information(&mut self, i: usize, action: Action) {
        let target = self.agents.target[i];
        let Some((x, y)) = self.information_destination(i) else {
            return;
        };
        let (from_x, from_y) = (self.agents.x[i], self.agents.y[i]);
        let distance = dist(from_x, from_y, x, y);
        if distance > 5.0 {
            let step = (self.actor_speed(i, from_x, from_y) * DT).min(distance - 4.0);
            self.walk_toward(i, x, y, step);
            self.agents.drain_energy(i, DT * 0.07);
            return;
        }
        let (tavern, market) = (target / MARKET_SLOTS, target % MARKET_SLOTS);
        let Some(report) = self.information_terms(i, tavern, market, action).cloned() else {
            return self.finish_hunt_action(i, action, target, "report_unavailable");
        };
        let host = &mut self.markets[self.taverns[tavern as usize].market as usize];
        let outcome = if action == Action::BuyInformation {
            self.agents.coins[i] -= PURCHASE_PRICE;
            host.coins += PURCHASE_PRICE;
            host.revenue_total += PURCHASE_PRICE;
            self.market_information
                .personal
                .entry(i as u32)
                .or_default()
                .insert(market, report);
            self.market_information.bought += 1;
            "bought_report"
        } else {
            host.coins -= SALE_PRICE;
            host.expenses_total += SALE_PRICE;
            self.agents.coins[i] += SALE_PRICE;
            self.market_information
                .taverns
                .entry(tavern)
                .or_default()
                .insert(market, report);
            self.market_information.sold += 1;
            "sold_report"
        };
        self.finish_hunt_action(i, action, target, outcome);
    }

    pub(super) fn information_summary(&self) -> serde_json::Value {
        serde_json::json!({
            "bought": self.market_information.bought, "sold": self.market_information.sold,
            "lifetime_days": REPORT_LIFETIME / rest::DAY_TICKS,
            "buy_price": PURCHASE_PRICE, "sell_price": SALE_PRICE,
            "taverns": self.market_information.taverns.iter().map(|(&tavern, reports)|
                serde_json::json!({"id": tavern, "reports": reports.iter().filter(|(_, report)| fresh(report, self.tick))
                    .map(|(&market, report)| serde_json::json!({"market": market, "observed_tick": report.tick})).collect::<Vec<_>>()})
            ).collect::<Vec<_>>()
        })
    }

    pub(super) fn validate_market_information(&self) -> Result<(), String> {
        let valid = |reports: &BTreeMap<u32, Report>| {
            reports
                .iter()
                .all(|(&m, r)| (m as usize) < self.markets.len() && r.tick <= self.tick)
        };
        if self
            .market_information
            .personal
            .iter()
            .any(|(&i, reports)| i as usize >= self.agents.len() || !valid(reports))
            || self
                .market_information
                .taverns
                .iter()
                .any(|(&i, reports)| i as usize >= self.taverns.len() || !valid(reports))
        {
            return Err("Invalid market information record".into());
        }
        Ok(())
    }
}

// Frozen layout of this module's save payloads in formats v15-v33 (#97).
// Generated from the traced schema; never edit a field of a Legacy* type.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct LegacyReport {
    pub tick: u64,
    pub quotes: [crate::legacy_v13::LegacyQuote; 13],
    pub meat: crate::legacy_v13::LegacyQuote,
}
impl From<LegacyReport> for Report {
    fn from(old: LegacyReport) -> Self {
        Self {
            tick: old.tick,
            quotes: old.quotes.map(|x1| x1.into()),
            meat: old.meat.into(),
        }
    }
}
#[cfg(test)]
impl From<Report> for LegacyReport {
    fn from(live: Report) -> Self {
        Self {
            tick: live.tick,
            quotes: live.quotes.map(|x1| x1.into()),
            meat: live.meat.into(),
        }
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct LegacyMarketInformation {
    pub personal: BTreeMap<u32, BTreeMap<u32, LegacyReport>>,
    pub taverns: BTreeMap<u32, BTreeMap<u32, LegacyReport>>,
    pub bought: u64,
    pub sold: u64,
}
impl From<LegacyMarketInformation> for MarketInformation {
    fn from(old: LegacyMarketInformation) -> Self {
        Self {
            personal: old
                .personal
                .into_iter()
                .map(|(k1, x1)| (k1, x1.into_iter().map(|(k2, x2)| (k2, x2.into())).collect()))
                .collect(),
            taverns: old
                .taverns
                .into_iter()
                .map(|(k1, x1)| (k1, x1.into_iter().map(|(k2, x2)| (k2, x2.into())).collect()))
                .collect(),
            bought: old.bought,
            sold: old.sold,
        }
    }
}
#[cfg(test)]
impl From<MarketInformation> for LegacyMarketInformation {
    fn from(live: MarketInformation) -> Self {
        Self {
            personal: live
                .personal
                .into_iter()
                .map(|(k1, x1)| (k1, x1.into_iter().map(|(k2, x2)| (k2, x2.into())).collect()))
                .collect(),
            taverns: live
                .taverns
                .into_iter()
                .map(|(k1, x1)| (k1, x1.into_iter().map(|(k2, x2)| (k2, x2.into())).collect()))
                .collect(),
            bought: live.bought,
            sold: live.sold,
        }
    }
}
