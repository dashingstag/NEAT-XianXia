use super::*;

impl Agents {
    pub fn weight(&self, i: usize) -> u32 {
        self.urban_weight[i]
            + self.rice[i]
            + self.tools[i]
                .iter()
                .enumerate()
                .map(|(t, &q)| q as u32 * WEIGHTS[9 + t])
                .sum::<u32>()
            + self.berries[i]
            + self.fish[i] * WEIGHTS[3]
            + self.cargo[i] * WEIGHTS[self.cargo_kind[i] as usize]
            + u32::from(self.coins[i] > 0)
            + if self.carried_body[i] != NONE {
                civic::BODY_WEIGHT
            } else {
                0
            }
    }
    pub fn room(&self, i: usize) -> u32 {
        self.capacity[i].saturating_sub(self.weight(i))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn cargo(w: &mut World, kind: usize, q: u32) {
        let p = w.patches.iter_mut().find(|p| p.kind == kind).unwrap();
        p.stock -= q as u64;
        w.agents.cargo_kind[0] = kind as u8;
        w.agents.cargo[0] = q;
    }
    #[test]
    fn wallet_weight_is_constant_and_items_have_distinct_weights() {
        let mut w = World::new(1, 42).unwrap();
        assert_eq!(w.agents.weight(0), 4);
        w.agents.coins[0] = 1_000_000;
        assert_eq!(w.agents.weight(0), 4);
        w.agents.coins[0] = 0;
        assert_eq!(w.agents.weight(0), 3);
        cargo(&mut w, 4, 2);
        assert_eq!(w.agents.weight(0), 13);
    }
    #[test]
    fn exhausted_capacity_blocks_harvesting_and_buying() {
        let mut w = World::new(1, 42).unwrap();
        w.agents.capacity[0] = 4;
        let p = w.patches.iter().position(|p| p.kind == 1).unwrap();
        w.agents.action[0] = Action::Gather;
        w.agents.target[0] = p as u32;
        w.agents.cargo_kind[0] = 1;
        w.agents.x[0] = w.patches[p].x;
        w.agents.y[0] = w.patches[p].y;
        w.agents.timer[0] = 2.0;
        let before = w.patches[p].stock;
        w.execute(0);
        assert_eq!(w.patches[p].stock, before);
        assert_eq!(w.agents.cargo[0], 0);
        w.agents.action[0] = Action::BuyFood;
        w.agents.target[0] = 0;
        w.agents.cargo_kind[0] = 0;
        w.agents.x[0] = w.markets[0].x;
        w.agents.y[0] = w.markets[0].y;
        let coins = w.agents.coins[0];
        w.execute(0);
        assert_eq!(w.agents.coins[0], coins);
        w.check().unwrap();
    }
    #[test]
    fn each_mine_is_finite_and_minting_is_accounted_once() {
        for (kind, value) in [(4, 100), (5, 10), (6, 1)] {
            let mut w = World::new(1, 42).unwrap();
            cargo(&mut w, kind, 2);
            assert!(w
                .patches
                .iter()
                .filter(|p| p.kind >= 4)
                .all(|p| p.renewal == 0.0));
            let money = w.coins();
            w.execute_exchange(0, 0, Action::Mint);
            assert_eq!(w.coins(), money + 2 * value);
            assert_eq!(w.ledger.minted_ore[kind], 2);
            w.execute_exchange(0, 0, Action::Mint);
            assert_eq!(w.coins(), money + 2 * value);
            w.check().unwrap();
        }
    }
    #[test]
    fn penniless_person_can_barter_existing_cargo_for_food() {
        let mut w = World::new(1, 42).unwrap();
        cargo(&mut w, 1, 2);
        w.markets[0].coins += w.agents.coins[0];
        w.agents.coins[0] = 0;
        w.markets[0].add(0, w.agents.berries[0] as u64, 0);
        w.agents.berries[0] = 0;
        w.markets[0].quotes[1].bid = 2;
        w.markets[0].quotes[0].ask = 3;
        let coins = w.coins();
        w.execute_exchange(0, 0, Action::Barter);
        assert_eq!(w.agents.berries[0], 1);
        assert_eq!(w.agents.cargo[0], 0);
        assert_eq!(w.coins(), coins);
        assert_eq!(w.stats.barter_trades, 1);
        w.check().unwrap();
    }
    #[test]
    fn services_transfer_currency_or_food_without_minting() {
        let mut w = World::new(1, 42).unwrap();
        w.markets[0].cancel_quotes();
        w.markets[0].project = Some(Project {
            kind: "Housing block".into(),
            progress: 0.0,
            required: 10.0,
            destination: 0,
        });
        let coins = w.agents.coins[0];
        w.agents.timer[0] = 1.0;
        w.commitments.commit(0, Action::Build, 0, 1);
        w.execute_exchange(0, 0, Action::Build);
        assert_eq!(w.action_memory_inputs(0, Action::Build, 0).unwrap()[0], 1.0);
        assert_eq!(w.agents.coins[0], coins + 1);
        assert_eq!(w.stats.paid_services, 1);
        let food = w.agents.berries[0];
        w.agents.timer[0] = 4.0;
        w.execute_exchange(0, 0, Action::WorkForFood);
        assert_eq!(w.agents.berries[0], food + 1);
        assert_eq!(w.stats.food_services, 1);
        assert_eq!(w.ledger.minted_coins, 0);
        w.check().unwrap();
    }
    #[test]
    fn new_brain_candidates_include_currency_barter_and_safe_rest() {
        let mut w = World::new(1, 42).unwrap();
        cargo(&mut w, 4, 2);
        w.markets[0].add(0, w.agents.berries[0] as u64, 0);
        w.agents.berries[0] = 0;
        let (choices, n) = w.offer(0);
        assert!(choices[..n].iter().any(|c| c.action == Action::Mint));
        assert!(choices[..n].iter().any(|c| c.action == Action::Barter));
        assert!(choices[..n].iter().any(|c| c.action == Action::Wait));
        let mint = choices[..n]
            .iter()
            .find(|c| c.action == Action::Mint)
            .unwrap();
        assert_eq!(mint.inputs[12], 1.0);
        assert!(mint.inputs[15] > 0.0);
        assert!(mint.inputs[17] > 0.0);
        assert!(choices[..n].iter().all(|c| c.score.is_finite()));
        w.check().unwrap();
    }
}

impl World {
    // Accepted goods-for-food quote: incoming units are rounded up, never free credit.
    pub(super) fn barter_terms(&self, i: usize, market: usize) -> Option<(usize, u32, u32)> {
        self.barter_terms_with_quotes(
            i,
            |kind| self.markets[market].quotes[kind].clone(),
            |kind| self.markets[market].stock(kind),
        )
    }
    pub(super) fn perceived_barter_terms(
        &self,
        i: usize,
        market: &Market,
    ) -> Option<(usize, u32, u32)> {
        if self.local_market_information(i, market) {
            return self.barter_terms(i, market.id as usize);
        }
        self.barter_terms_with_quotes(
            i,
            |kind| self.perceived_quote(i, market, kind),
            |kind| self.perceived_quote(i, market, kind).offered,
        )
    }
    fn barter_terms_with_quotes(
        &self,
        i: usize,
        quote: impl Fn(usize) -> Quote,
        stock: impl Fn(usize) -> u64,
    ) -> Option<(usize, u32, u32)> {
        let a = &self.agents;
        let k = a.cargo_kind[i] as usize;
        let bid = quote(k).bid;
        if a.cargo[i] == 0 || bid == 0 {
            return None;
        }
        for food in [0, 3, 8] {
            if food == k {
                continue;
            }
            let ask = quote(food).ask;
            if ask == 0 {
                continue;
            }
            let max_food =
                (3u32.saturating_sub(a.berries[i] + a.fish[i] + a.rice[i])).min(stock(food) as u32);
            for quantity in (1..=max_food).rev() {
                let offered = (quantity as u64 * ask).div_ceil(bid);
                if offered <= a.cargo[i] as u64
                    && a.weight(i) - offered as u32 * WEIGHTS[k] + quantity * WEIGHTS[food]
                        <= a.capacity[i]
                {
                    return Some((food, offered as u32, quantity));
                }
            }
        }
        None
    }
    pub(super) fn execute_exchange(&mut self, i: usize, t: usize, action: Action) {
        let kind = self.agents.cargo_kind[i] as usize;
        if action == Action::Mint {
            let quantity = self.agents.cargo[i] as u64;
            if !(4..=6).contains(&kind) || quantity == 0 {
                return;
            }
            let minted = quantity * MINT_VALUES[kind];
            self.agents.cargo[i] = 0;
            self.agents.coins[i] += minted;
            self.ledger.minted_ore[kind] += quantity;
            self.ledger.minted_coins += minted;
            self.stats.mintings += 1;
            return;
        }
        if action == Action::Barter {
            let Some((food, offered, received)) = self.barter_terms(i, t) else {
                return;
            };
            let m = &mut self.markets[t];
            let mut remaining = received as u64;
            while remaining > 0 {
                remaining -= m.take(food, remaining).unwrap().quantity;
            }
            m.quotes[food].offered = m.quotes[food].offered.min(m.stock(food));
            m.add(kind, offered as u64, self.agents.origin[i]);
            m.sales[kind] += offered as u64;
            if self.agents.origin[i] == m.id {
                m.local_sales[kind] += offered as u64
            } else {
                m.imported_sales[kind] += offered as u64
            }
            self.agents.cargo[i] -= offered;
            if food == 0 {
                self.agents.berries[i] += received
            } else if food == 3 {
                self.agents.fish[i] += received
            } else {
                self.agents.rice[i] += received
            }
            self.stats.barter_trades += 1;
            self.stats.trades += 1;
            return;
        }
        let work_speed = self.work_factor(i, action, 1);
        let m = &mut self.markets[t];
        let Some(project) = m.project.as_ref() else {
            return;
        };
        if project.progress >= project.required || self.agents.energy[i] < 1.0 {
            return;
        }
        self.agents.timer[i] += DT * work_speed;
        let labor = if action == Action::WorkForFood {
            4.0
        } else {
            1.0
        };
        if self.agents.timer[i] + 0.00001 < labor {
            return;
        }
        if action == Action::WorkForFood {
            let Some(food) = [0, 3, 8]
                .into_iter()
                .find(|&k| m.stock(k) > 0 && self.agents.room(i) >= WEIGHTS[k])
            else {
                return;
            };
            m.take(food, 1).unwrap();
            m.quotes[food].offered = m.quotes[food].offered.min(m.stock(food));
            if food == 0 {
                self.agents.berries[i] += 1
            } else if food == 3 {
                self.agents.fish[i] += 1
            } else {
                self.agents.rice[i] += 1
            }
            self.stats.food_services += 1;
        } else {
            if m.coins == 0 || (self.agents.coins[i] == 0 && self.agents.room(i) == 0) {
                return;
            }
            m.coins -= 1;
            m.expenses_total += 1;
            self.agents.coins[i] += 1;
            self.stats.paid_services += 1;
            self.commitments
                .resolve(i, Action::Build, t as u32, 1, self.tick, true);
        }
        let p = m.project.as_mut().unwrap();
        p.progress = (p.progress + labor).min(p.required);
        self.agents.drain_energy(i, labor * 0.3);
        self.agents.timer[i] = 0.0;
        if action != Action::WorkForFood {
            self.credit_education_bonus(i, t, 1);
        }
    }
}
