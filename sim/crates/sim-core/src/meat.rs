use super::*;
use std::collections::BTreeMap;

const PACK_LIMIT: u64 = 4;
const RANGE: f32 = 150.0;

fn meat_capacity(market: &Market) -> u64 {
    (12 + 4 * u64::from(market.housing_blocks)).min(64)
}

fn meat_prices(stock: u64, capacity: u64, recent_sales: u64) -> (u64, u64) {
    let stock_bid = if stock * 4 < capacity {
        3
    } else if stock * 3 >= capacity * 2 {
        1
    } else {
        2
    };
    let demand_bid = u64::from(recent_sales >= (capacity / 3).max(1));
    let bid = stock_bid + demand_bid;
    (bid, bid + 1)
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(super) struct DailySales {
    day: u64,
    today: u64,
    yesterday: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(super) struct MeatTrade {
    packs: BTreeMap<u32, u64>,
    markets: BTreeMap<u32, u64>,
    collected: u64,
    sold: u64,
    bought: u64,
    eaten: u64,
    discarded: u64,
    #[serde(skip)]
    pub(super) demand: BTreeMap<u32, DailySales>,
}

impl MeatTrade {
    pub(super) fn pack(&self, i: usize) -> u64 {
        self.packs.get(&(i as u32)).copied().unwrap_or(0)
    }

    fn stock(&self, market: u32) -> u64 {
        self.markets.get(&market).copied().unwrap_or(0)
    }

    fn recent_sales(&self, market: u32, tick: u64) -> u64 {
        let day = tick / rest::DAY_TICKS;
        self.demand
            .get(&market)
            .map_or(0, |sales| match day.saturating_sub(sales.day) {
                0 => sales.today + sales.yesterday / 2,
                1 => sales.today / 2,
                _ => 0,
            })
    }

    fn record_sale(&mut self, market: u32, tick: u64) {
        let day = tick / rest::DAY_TICKS;
        let sales = self.demand.entry(market).or_default();
        if day > sales.day {
            sales.yesterday = if day - sales.day == 1 { sales.today } else { 0 };
            sales.today = 0;
            sales.day = day;
        }
        sales.today += 1;
    }

    pub(super) fn live_quote(&self, market: &Market, tick: u64) -> Quote {
        let offered = self.stock(market.id);
        let capacity = meat_capacity(market);
        let (bid, ask) = meat_prices(offered, capacity, self.recent_sales(market.id, tick));
        Quote {
            offered,
            bid,
            ask,
            remaining: capacity.saturating_sub(offered).min(market.coins / bid),
            escrow: 0,
        }
    }

    pub(super) fn discard_pack(&mut self, i: usize) {
        self.discarded += self.packs.remove(&(i as u32)).unwrap_or(0);
    }

    pub(super) fn summary(&self, markets: &[Market], tick: u64) -> serde_json::Value {
        let quotes: BTreeMap<_, _> = markets
            .iter()
            .map(|market| {
                let q = self.live_quote(market, tick);
                (
                    market.id,
                    serde_json::json!({
                        "bid": q.bid, "ask": q.ask, "capacity": meat_capacity(market),
                        "recent_sales": self.recent_sales(market.id, tick)
                    }),
                )
            })
            .collect();
        serde_json::json!({
            "collected": self.collected, "sold": self.sold, "bought": self.bought,
            "eaten_from_pack": self.eaten, "discarded": self.discarded,
            "carried": self.packs.values().sum::<u64>(), "market_stock": self.markets,
            "quotes": quotes
        })
    }
}

impl World {
    pub(super) fn meat_offers(&self, i: usize) -> Vec<Candidate> {
        let a = &self.agents;
        let distance = |x, y| dist(a.x[i], a.y[i], x, y);
        let candidate = |action, target, need, benefit, travel, price| {
            let mut c = Candidate {
                action,
                target,
                kind: 0,
                ..Default::default()
            };
            c.inputs[0] = need;
            c.inputs[1] = benefit;
            c.inputs[3] = travel;
            c.inputs[4] = price;
            c
        };
        let mut offers = Vec::new();
        let pack = self.meat_trade.pack(i);
        let can_collect = self.can_collect_meat(i) && !self.carcasses.is_empty();
        let evaluate_market = |action, market: &Market, quantity: u64, price| {
            let d = distance(market.x, market.y);
            match action {
                Action::SellMeat => candidate(
                    action,
                    market.id,
                    0.8 + pack as f32 / 4.0,
                    0.55 + quantity as f32 * 0.08,
                    d,
                    price as f32,
                ),
                Action::BuyMeat => candidate(
                    action,
                    market.id,
                    0.7 + a.hunger[i] / 100.0,
                    0.9,
                    d,
                    -(price as f32),
                ),
                _ => unreachable!(),
            }
        };
        let buyers = self.markets.iter().filter_map(|m| {
            let q = self.perceived_meat_quote(i, m);
            (q.remaining > 0).then_some((m, q.remaining, q.bid))
        });
        if pack > 0 {
            let sale = buyers
                .clone()
                .map(|(m, quantity, bid)| (m, quantity.min(pack), bid));
            let nearest = sale
                .clone()
                .min_by(|a, b| distance(a.0.x, a.0.y).total_cmp(&distance(b.0.x, b.0.y)));
            let highest_return = sale.max_by(|a, b| {
                (a.1 * a.2)
                    .cmp(&(b.1 * b.2))
                    .then_with(|| distance(b.0.x, b.0.y).total_cmp(&distance(a.0.x, a.0.y)))
                    .then_with(|| b.0.id.cmp(&a.0.id))
            });
            for (market, quantity, bid) in nearest.into_iter().chain(
                highest_return.filter(|best| nearest.is_none_or(|near| near.0.id != best.0.id)),
            ) {
                offers.push(evaluate_market(Action::SellMeat, market, quantity, bid));
            }
        }
        if can_collect {
            if let Some((_, _, bid)) =
                buyers.min_by(|a, b| distance(a.0.x, a.0.y).total_cmp(&distance(b.0.x, b.0.y)))
            {
                if let Some((id, d)) = self
                    .carcasses
                    .iter()
                    .filter(|(_, stock)| **stock > 1)
                    .map(|(&id, _)| {
                        let animal = &self.wildlife.animals[id as usize];
                        (id, distance(animal.x, animal.y))
                    })
                    .filter(|(_, d)| *d <= RANGE)
                    .min_by(|a, b| a.1.total_cmp(&b.1))
                {
                    let surplus_need = 0.6 * (1.0 - (a.hunger[i] / 55.0).clamp(0.0, 1.0));
                    offers.push(candidate(
                        Action::CollectMeat,
                        id,
                        surplus_need,
                        0.8,
                        d,
                        bid as f32,
                    ));
                }
            }
        }
        if a.hunger[i] >= 20.0 && pack == 0 {
            let sellers = self.markets.iter().filter_map(|m| {
                let q = self.perceived_meat_quote(i, m);
                (q.offered > 0 && a.coins[i] >= q.ask).then_some((m, q.ask))
            });
            let nearest = sellers
                .clone()
                .min_by(|a, b| distance(a.0.x, a.0.y).total_cmp(&distance(b.0.x, b.0.y)));
            let cheapest = sellers.min_by(|a, b| {
                a.1.cmp(&b.1)
                    .then_with(|| distance(a.0.x, a.0.y).total_cmp(&distance(b.0.x, b.0.y)))
                    .then_with(|| a.0.id.cmp(&b.0.id))
            });
            for (market, ask) in nearest
                .into_iter()
                .chain(cheapest.filter(|best| nearest.is_none_or(|near| near.0.id != best.0.id)))
            {
                offers.push(evaluate_market(Action::BuyMeat, market, 1, ask));
            }
        }
        offers
    }

    fn can_collect_meat(&self, i: usize) -> bool {
        self.meat_trade.pack(i) < PACK_LIMIT
            && self.agents.energy[i] >= 12.0
            && self.agents.room(i) > u32::from(self.agents.coins[i] == 0)
    }

    pub(super) fn meat_destination(&self, i: usize) -> Option<(f32, f32)> {
        let target = self.agents.target[i] as usize;
        if self.agents.action[i] == Action::CollectMeat {
            return self.wildlife.animals.get(target).map(|a| (a.x, a.y));
        }
        self.markets.get(target).map(|m| (m.x, m.y))
    }

    pub(super) fn execute_meat(&mut self, i: usize, action: Action) {
        let target = self.agents.target[i];
        let Some((x, y)) = self.meat_destination(i) else {
            return self.finish_hunt_action(i, action, target, "unavailable");
        };
        let (from_x, from_y) = (self.agents.x[i], self.agents.y[i]);
        let d = dist(from_x, from_y, x, y);
        if action == Action::CollectMeat && d > RANGE {
            return self.finish_hunt_action(i, action, target, "out_of_range");
        }
        if d > 7.0 {
            let step = (self.actor_speed(i, from_x, from_y) * DT).min(d - 6.0);
            self.agents.x[i] += (x - from_x) / d * step;
            self.agents.y[i] += (y - from_y) / d * step;
            self.agents.drain_energy(i, DT * 0.07);
            return;
        }
        let quote = self
            .markets
            .get(target as usize)
            .map(|market| self.meat_trade.live_quote(market, self.tick))
            .unwrap_or_default();
        let (stock, bid, ask) = (quote.offered, quote.bid, quote.ask);
        let pack = self.meat_trade.pack(i);
        let valid = self.is_adult(i)
            && !self.must_stay_home(i)
            && match action {
                Action::CollectMeat => {
                    self.can_collect_meat(i)
                        && self.carcasses.get(&target).is_some_and(|&q| q > 1)
                        && self
                            .wildlife
                            .animals
                            .get(target as usize)
                            .is_some_and(|a| !a.alive)
                }
                Action::SellMeat => pack > 0 && quote.remaining > 0,
                Action::BuyMeat => {
                    stock > 0
                        && pack == 0
                        && self.agents.hunger[i] >= 20.0
                        && self.agents.coins[i] >= ask
                }
                _ => false,
            };
        if !valid {
            return self.finish_hunt_action(i, action, target, "unavailable");
        }
        let outcome = match action {
            Action::CollectMeat => {
                let stock = self.carcasses.get_mut(&target).unwrap();
                let room = self.agents.room(i) - u32::from(self.agents.coins[i] == 0);
                let take = (*stock - 1).min(PACK_LIMIT - pack).min(room as u64);
                *stock -= take;
                self.meat_trade.packs.insert(i as u32, pack + take);
                self.meat_trade.collected += take;
                self.update_urban_weight(i);
                "collected_meat"
            }
            Action::SellMeat => {
                let market = &mut self.markets[target as usize];
                let qty = pack.min(quote.remaining);
                market.coins -= qty * bid;
                market.expenses_total += qty * bid;
                self.agents.coins[i] += qty * bid;
                self.meat_trade.markets.insert(target, stock + qty);
                if qty == pack {
                    self.meat_trade.packs.remove(&(i as u32));
                } else {
                    self.meat_trade.packs.insert(i as u32, pack - qty);
                }
                self.meat_trade.sold += qty;
                self.update_urban_weight(i);
                self.stats.trades += 1;
                self.people[i].trades += 1;
                "sold_meat"
            }
            Action::BuyMeat => {
                let market = &mut self.markets[target as usize];
                self.agents.coins[i] -= ask;
                market.coins += ask;
                market.revenue_total += ask;
                if stock == 1 {
                    self.meat_trade.markets.remove(&target);
                } else {
                    self.meat_trade.markets.insert(target, stock - 1);
                }
                self.meat_trade.bought += 1;
                self.meat_trade.record_sale(target, self.tick);
                self.agents.hunger[i] = (self.agents.hunger[i] - 32.0).max(0.0);
                self.restore_meal_energy(i);
                self.people[i].meals += 1;
                self.stats.trades += 1;
                self.people[i].trades += 1;
                "bought_and_ate_meat"
            }
            _ => unreachable!(),
        };
        self.finish_hunt_action(i, action, target, outcome);
    }

    pub(super) fn eat_packed_meat(&mut self, i: usize) {
        if self.agents.hunger[i] <= 40.0 || self.meat_trade.pack(i) == 0 {
            return;
        }
        if self.meat_trade.pack(i) == 1 {
            self.meat_trade.packs.remove(&(i as u32));
        } else {
            *self.meat_trade.packs.get_mut(&(i as u32)).unwrap() -= 1;
        }
        self.meat_trade.eaten += 1;
        self.agents.hunger[i] -= 32.0;
        self.restore_meal_energy(i);
        self.people[i].meals += 1;
        self.update_urban_weight(i);
    }

    pub(super) fn validate_meat_trade(&self) -> Result<(), String> {
        let t = &self.meat_trade;
        if t.packs.iter().any(|(&i, &q)| {
            i as usize >= self.agents.len()
                || q > PACK_LIMIT
                || (q > 0 && !self.agents.alive[i as usize])
        }) || t
            .markets
            .iter()
            .any(|(&i, &q)| i as usize >= self.markets.len() || q > 64)
            || t.demand.iter().any(|(&i, sales)| {
                i as usize >= self.markets.len() || sales.day > self.tick / rest::DAY_TICKS
            })
            || u128::from(t.collected)
                != t.packs.values().map(|&q| u128::from(q)).sum::<u128>()
                    + u128::from(t.sold)
                    + u128::from(t.eaten)
                    + u128::from(t.discarded)
            || u128::from(t.sold)
                != t.markets.values().map(|&q| u128::from(q)).sum::<u128>() + u128::from(t.bought)
        {
            return Err("Meat trade conservation error".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commit(world: &mut World, i: usize, action: Action, target: u32) {
        world.agents.action[i] = action;
        world.agents.target[i] = target;
        world.execute(i);
    }

    #[test]
    fn storage_and_recent_completed_sales_change_live_quotes() {
        let mut w = World::new_with_size(8, 42, Hill::default(), MapSize::X4).unwrap();
        let m = &mut w.markets[0];
        m.coins = 100;
        assert_eq!(meat_capacity(m), 12);
        m.housing_blocks = 2;
        assert_eq!(meat_capacity(m), 20);
        w.meat_trade.markets.insert(0, 14);
        let stocked = w.meat_trade.live_quote(m, 0);
        assert_eq!((stocked.remaining, stocked.ask), (6, 2));
        for _ in 0..6 {
            w.meat_trade.record_sale(0, 0);
        }
        assert_eq!(w.meat_trade.live_quote(m, 0).ask, 3);
        assert_eq!(w.meat_trade.live_quote(m, rest::DAY_TICKS).ask, 2);
        assert_eq!(w.meat_trade.live_quote(m, rest::DAY_TICKS * 2).ask, 2);
        assert_eq!(w.meat_trade.live_quote(&w.markets[1], 0).ask, 4);
    }

    fn observe_then_return(w: &mut World, market: usize) {
        let (x, y) = (w.agents.x[0], w.agents.y[0]);
        w.agents.x[0] = w.markets[market].x;
        w.agents.y[0] = w.markets[market].y;
        w.observe_local_market(0);
        w.agents.x[0] = x;
        w.agents.y[0] = y;
    }

    #[test]
    fn neat_can_compare_nearby_market_with_a_better_quote() {
        let mut w = World::new_with_size(8, 42, Hill::default(), MapSize::X4).unwrap();
        w.people[0].initial_age_days = 25 * 365;
        w.agents.x[0] = w.markets[0].x;
        w.agents.y[0] = w.markets[0].y;
        w.agents.hunger[0] = 30.0;
        w.meat_trade.markets.insert(0, 1);
        w.meat_trade.markets.insert(1, 9);
        w.market_information = Default::default();
        observe_then_return(&mut w, 1);
        let (offers, n) = w.offer(0);
        let buys: Vec<_> = offers[..n]
            .iter()
            .filter(|c| c.action == Action::BuyMeat)
            .collect();
        assert_eq!(buys.iter().map(|c| c.target).collect::<Vec<_>>(), [0, 1]);
        assert!(buys[0].inputs[3] < buys[1].inputs[3]);
        assert!(buys[0].inputs[4] < buys[1].inputs[4]);

        w.meat_trade.markets.insert(0, 9);
        w.meat_trade.markets.insert(1, 1);
        w.market_information = Default::default();
        observe_then_return(&mut w, 1);
        w.meat_trade.packs.insert(0, 2);
        let (offers, n) = w.offer(0);
        let sales: Vec<_> = offers[..n]
            .iter()
            .filter(|c| c.action == Action::SellMeat)
            .collect();
        assert_eq!(sales.iter().map(|c| c.target).collect::<Vec<_>>(), [0, 1]);
        assert!(sales[0].inputs[3] < sales[1].inputs[3]);
        assert!(sales[0].inputs[4] < sales[1].inputs[4]);
    }

    #[test]
    fn stale_meat_report_can_start_a_trip_but_cannot_complete_an_empty_sale() {
        let mut w = World::new_with_size(8, 42, Hill::default(), MapSize::X4).unwrap();
        let (x, y) = (w.markets[0].x, w.markets[0].y);
        w.people[0].initial_age_days = 25 * 365;
        w.agents.x[0] = x + 250.0;
        w.agents.y[0] = y;
        w.agents.hunger[0] = 30.0;
        w.meat_trade.markets.insert(0, 10);
        w.market_information = Default::default();
        observe_then_return(&mut w, 0);
        w.meat_trade.markets.clear();
        let wallet = w.agents.coins[0];
        assert!(w
            .meat_offers(0)
            .iter()
            .any(|c| c.action == Action::BuyMeat && c.target == 0));
        commit(&mut w, 0, Action::BuyMeat, 0);
        assert!(w.agents.x[0] < x + 250.0);
        assert_eq!(w.agents.action[0], Action::BuyMeat);
        w.agents.x[0] = x;
        w.execute(0);
        assert_eq!(w.agents.action[0], Action::Wait);
        assert_eq!(w.agents.coins[0], wallet);
        assert_eq!(w.agents.hunger[0], 30.0);
        w.meat_trade.markets.insert(0, 1);
        commit(&mut w, 0, Action::BuyMeat, 0);
        assert_eq!(w.agents.coins[0], wallet - 4);
        assert_eq!(w.meat_trade.recent_sales(0, w.tick), 1);
    }

    #[test]
    fn finite_meat_moves_from_carcass_to_pack_to_market_to_meal() {
        let mut w = World::new_with_size(8, 42, Hill::default(), MapSize::X4).unwrap();
        let (x, y) = (w.wildlife.animals[0].x, w.wildlife.animals[0].y);
        w.wildlife.animals[0].alive = false;
        w.wildlife.animals[0].pregnancy = None;
        w.wildlife.ledger.deaths += 1;
        w.carcasses.insert(0, 2);
        for i in 0..3 {
            w.people[i].initial_age_days = 25 * 365;
            w.agents.x[i] = x;
            w.agents.y[i] = y;
            w.agents.hunger[i] = 25.0;
        }
        observe_then_return(&mut w, 0);
        let coins = w.coins();
        let weight = w.agents.weight(0);
        let (offers, n) = w.offer(0);
        assert!(offers[..n]
            .iter()
            .any(|c| c.action == Action::CollectMeat && c.score.is_finite()));
        commit(&mut w, 0, Action::CollectMeat, 0);
        commit(&mut w, 1, Action::CollectMeat, 0);
        assert_eq!(w.carcasses[&0], 1);
        assert_eq!(w.meat_trade.pack(0), 1);
        assert!(!w.has_hunting_weapon(0));
        assert_eq!(w.meat_trade.pack(1), 0);
        assert_eq!(w.agents.weight(0), weight + 1);
        w.check().unwrap();

        commit(&mut w, 0, Action::SellMeat, 0);
        assert_eq!(w.meat_trade.stock(0), 0);
        for i in 0..3 {
            w.agents.x[i] = w.markets[0].x;
            w.agents.y[i] = w.markets[0].y;
        }
        commit(&mut w, 0, Action::SellMeat, 0);
        assert_eq!(w.meat_trade.stock(0), 1);
        assert_eq!(w.meat_trade.pack(0), 0);
        let checkpoint =
            std::env::temp_dir().join(format!("neat-meat-{}.snapshot", std::process::id()));
        w.save(&checkpoint).unwrap();
        let mut loaded = World::load(&checkpoint).unwrap();
        std::fs::remove_file(checkpoint).unwrap();
        for world in [&mut w, &mut loaded] {
            commit(world, 1, Action::BuyMeat, 0);
            commit(world, 2, Action::BuyMeat, 0);
            assert_eq!(world.agents.hunger[1], 0.0);
            assert_eq!(world.agents.hunger[2], 25.0);
            assert_eq!(world.meat_trade.bought, 1);
            assert_eq!(world.coins(), coins);
            world.check().unwrap();
        }
        assert_eq!(
            w.checkpoint_payload().unwrap(),
            loaded.checkpoint_payload().unwrap()
        );
    }
}
