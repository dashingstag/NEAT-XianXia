use super::*;
impl Agents {
    pub(super) fn push_child(&mut self, home: u32, x: f32, y: f32) {
        self.x.push(x);
        self.y.push(y);
        self.health.push(100.0);
        self.hunger.push(0.0);
        self.energy.push(50.0);
        self.alive.push(true);
        self.coins.push(0);
        self.berries.push(0);
        self.fish.push(0);
        self.rice.push(0);
        self.cargo.push(0);
        self.cargo_kind.push(0);
        self.origin.push(home);
        self.home.push(home);
        self.action.push(Action::CareFamily);
        self.target.push(home);
        self.timer.push(0.0);
        self.last_decision.push(0);
        self.convoy.push(NONE);
        self.solo_until.push(0);
        self.dwell_cell.push(NONE);
        self.dwell_ticks.push(0);
        self.residence.push(0);
        self.capacity.push(8);
        self.rested_meter.push(40.0);
        self.rested.push(false);
        self.sleeping.push(false);
        self.rest_site.push(NONE);
        self.bed_slot.push(NONE);
        self.contract.push(NONE);
        self.carried_body.push(NONE);
        self.corpse.push(NONE);
        self.traits.push(0);
        self.tools.push([0; 4]);
        self.urban_weight.push(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn toolmaking_purchase_and_equipment_preserve_materials_and_weight() {
        let mut w = World::new(1, 42).unwrap();
        w.markets[0].housing_blocks = 1;
        for (k, q) in [(7, 2), (1, 1)] {
            let p = w.patches.iter_mut().find(|p| p.kind == k).unwrap();
            p.stock -= q;
            w.markets[0].add(k, q, 0);
        }
        w.agents.traits[0] = 0;
        w.agents.target[0] = 0;
        w.agents.cargo_kind[0] = 9;
        w.agents.timer[0] = 5.0;
        let money = w.coins();
        w.execute_industry(0, Action::CraftTool);
        assert_eq!(w.markets[0].stock(9), 1);
        assert_eq!(w.ledger.crafted_in[7], 2);
        assert_eq!(w.ledger.crafted_in[1], 1);
        assert_eq!(w.ledger.crafted_out[9], 1);
        assert_eq!(w.coins(), money);
        w.check().unwrap();
        w.refresh();
        let weight = w.agents.weight(0);
        w.execute_industry(0, Action::BuyTool);
        assert_eq!(w.agents.tools[0][0], 1);
        assert_eq!(w.agents.weight(0), weight + 4);
        assert_eq!(w.work_factor(0, Action::Gather, 1), 1.5);
        w.check().unwrap();
    }
    #[test]
    fn map_spacing_and_berry_supply_are_expanded_and_bounded() {
        let mut w = World::new(100, 42).unwrap();
        assert_eq!(WORLD_WIDTH, 3072.0);
        assert!(
            dist(
                w.markets[0].x,
                w.markets[0].y,
                w.markets[1].x,
                w.markets[1].y
            ) > 1700.0
        );
        assert!(w
            .patches
            .iter()
            .filter(|p| p.kind == 0)
            .all(|p| p.capacity >= 600 && p.renewal >= 3.0));
        assert_eq!(w.patches.iter().filter(|p| p.kind == 7).count(), 3);
        for _ in 0..100 {
            w.step(false);
        }
        assert!(w.patches.iter().all(|p| p.stock <= p.capacity));
        w.check().unwrap();
    }
}
impl World {
    pub(super) fn work_factor(&self, i: usize, action: Action, kind: usize) -> f32 {
        let tool = match action {
            Action::Gather if kind == 1 => Some(0),
            Action::Gather if [2, 4, 5, 6, 7].contains(&kind) => Some(1),
            Action::Farm | Action::HarvestRice => Some(2),
            Action::Gravedigger | Action::BuildGraveyard | Action::Build => Some(3),
            _ => None,
        };
        let mut factor = if tool.is_some_and(|t| self.agents.tools[i][t] > 0) {
            1.5
        } else {
            1.0
        };
        if self.agents.traits[i] & life::CLUMSY != 0 {
            factor *= 0.8;
        }
        if self.agents.traits[i] & life::COMMUNITY_MINDED != 0
            && self.social.get(i).copied().unwrap_or(0.0) >= 60.0
            && matches!(
                action,
                Action::Build
                    | Action::BuildTavern
                    | Action::BuildGraveyard
                    | Action::Gravedigger
                    | Action::Farm
                    | Action::HarvestRice
            )
        {
            factor *= 1.1;
        }
        factor
    }
    pub(super) fn can_craft(&self, i: usize, m: usize, kind: usize) -> bool {
        let iron = if kind == 10 { 3 } else { 2 };
        let market = &self.markets[m];
        !self.urban.states.get(m).is_some_and(|s| s.zoned)
            && (9..13).contains(&kind)
            && self.is_adult(i)
            && !self.must_stay_home(i)
            && market.housing_blocks > 0
            && market.coins >= 2
            && market.stock(7) >= iron
            && market.stock(1) >= 1
            && market.stock(kind) < (market.population / 20).max(2) as u64
            && self.agents.energy[i] > 10.0
    }
    pub(super) fn execute_industry(&mut self, i: usize, action: Action) {
        let kind = self.agents.cargo_kind[i] as usize;
        if !(9..13).contains(&kind) {
            return;
        }
        if action == Action::EquipTool {
            if self.agents.cargo[i] > 0 && self.agents.tools[i][kind - 9] == 0 {
                self.agents.cargo[i] -= 1;
                self.agents.tools[i][kind - 9] = 1;
            }
            return;
        }
        let m = self.agents.target[i] as usize;
        if m >= self.markets.len() {
            return;
        }
        if action == Action::BuyTool {
            let price = self.markets[m].quotes[kind].ask;
            if self.agents.tools[i][kind - 9] > 0
                || self.agents.room(i) < WEIGHTS[kind]
                || price == 0
                || self.agents.coins[i] < price
                || self.markets[m].stock(kind) == 0
            {
                return;
            }
            self.markets[m].take(kind, 1);
            self.markets[m].quotes[kind].offered =
                self.markets[m].quotes[kind].offered.saturating_sub(1);
            self.agents.coins[i] -= price;
            self.markets[m].coins += price;
            self.markets[m].revenue_total += price;
            self.agents.tools[i][kind - 9] = 1;
            self.people[i].trades += 1;
            self.stats.trades += 1;
            return;
        }
        if !self.can_craft(i, m, kind) {
            return;
        }
        self.agents.timer[i] += DT * self.work_factor(i, action, kind);
        if self.agents.timer[i] < 5.0 {
            return;
        }
        self.agents.timer[i] = 0.0;
        for (k, quantity) in [(7, if kind == 10 { 3 } else { 2 }), (1, 1)] {
            let mut remaining = quantity;
            while remaining > 0 {
                remaining -= self.markets[m].take(k, remaining).unwrap().quantity;
            }
            self.ledger.crafted_in[k] += quantity;
            self.markets[m].quotes[k].offered = self.markets[m].quotes[k]
                .offered
                .min(self.markets[m].stock(k));
        }
        self.markets[m].add(kind, 1, m as u32);
        self.ledger.crafted_out[kind] += 1;
        self.markets[m].coins -= 2;
        self.markets[m].expenses_total += 2;
        self.agents.coins[i] += 2;
        self.credit_education_bonus(i, m, 2);
        self.agents.drain_energy(i, 1.0);
        self.people[i].crafts += 1;
        self.stats.tools_crafted += 1;
    }
    pub(super) fn expand_map_and_food(&mut self) {
        let scale = (self.agents.len() as u64).div_ceil(1000).max(1);
        let initial = self.ledger.initial.iter().all(|&n| n == 0);
        let classic = self.map.spec.size == MapSize::Classic;
        if classic {
            for i in 0..self.agents.len() {
                self.agents.x[i] *= 3.0;
                self.agents.y[i] *= 3.0;
            }
            for m in &mut self.markets {
                m.x *= 3.0;
                m.y *= 3.0;
            }
            for s in &mut self.shelters {
                s.x *= 3.0;
                s.y *= 3.0;
            }
            for c in self.convoys.iter_mut().flatten() {
                c.x *= 3.0;
                c.y *= 3.0;
                c.goal_x *= 3.0;
                c.goal_y *= 3.0;
            }
            for g in &mut self.graveyards {
                g.x *= 3.0;
                g.y *= 3.0;
            }
            for c in &mut self.corpses {
                c.x *= 3.0;
                c.y *= 3.0;
            }
            for p in &mut self.patches {
                p.x *= 3.0;
                p.y *= 3.0;
            }
        }
        for p in &mut self.patches {
            if p.kind == 0 {
                p.capacity = 600 * scale;
                if initial {
                    p.stock = p.capacity;
                }
                p.renewal = 3.0 * scale as f64;
            }
        }
        // Historical caps on the classic layout; generated maps use
        // land bounds instead.
        let cap_x = if classic { 2620.0 } else { self.map.spec.width() - 20.0 };
        let cap_y = if classic { 1750.0 } else { self.map.spec.height() - 20.0 };
        let ring_x = if classic { 2670.0 } else { self.map.spec.width() - 20.0 };
        let ring_y = if classic { 1880.0 } else { self.map.spec.height() - 20.0 };
        for m in 0..self.markets.len().min(3) {
            let x = (self.markets[m].x + 180.0).min(cap_x);
            let y = (self.markets[m].y + 130.0).min(cap_y);
            self.patches.push(Patch {
                id: self.patches.len() as u32,
                kind: 7,
                x,
                y,
                stock: 1000 * scale,
                capacity: 1000 * scale,
                renewal: 0.0,
                growth: 0.0,
                age: 0.0,
            });
            self.ledger.initial[7] += 1000 * scale;
            self.fertility.push(FertileRegion {
                x: (self.markets[m].x + 235.0).min(if classic {
                    2590.0
                } else {
                    self.map.spec.width() - 20.0
                }),
                y: self.markets[m].y + 90.0,
                width: 180.0,
                height: 160.0,
                fertility: 0.9,
                moisture: 0.85,
            });
            if m > 0 {
                for j in 0..6 {
                    let angle = j as f32 * std::f32::consts::TAU / 6.0;
                    let px = (self.markets[m].x + angle.cos() * 230.0).clamp(20.0, ring_x);
                    let py = (self.markets[m].y + angle.sin() * 230.0).clamp(20.0, ring_y);
                    self.patches.push(Patch {
                        id: self.patches.len() as u32,
                        kind: 0,
                        x: px,
                        y: py,
                        stock: 600 * scale,
                        capacity: 600 * scale,
                        renewal: 3.0 * scale as f64,
                        growth: 0.0,
                        age: 0.0,
                    });
                    self.ledger.initial[0] += 600 * scale;
                }
            }
        }
    }
    pub(super) fn upgrade_content(&mut self) -> Result<(), String> {
        fn registry(r: &mut Innovations) {
            let shift = |id: u32| if id >= 28 { id + 14 } else { id };
            r.edges = std::mem::take(&mut r.edges)
                .into_iter()
                .map(|((a, b), id)| ((shift(a), shift(b)), id))
                .collect();
            for id in r.splits.values_mut() {
                *id = shift(*id);
            }
            r.next_node = shift(r.next_node);
        }
        fn genome(g: &mut Genome, r: &mut Innovations) {
            let shift = |id: u32| if id >= 28 { id + 14 } else { id };
            for n in &mut g.nodes {
                n.id = shift(n.id);
            }
            for e in &mut g.genes {
                e.from = shift(e.from);
                e.to = shift(e.to);
            }
            for id in 28..42 {
                g.nodes.push(neat_core::Node {
                    id,
                    kind: neat_core::Kind::Input,
                    bias: 0.0,
                });
                for h in 42..46 {
                    g.genes.push(neat_core::Gene {
                        innovation: r.edge(id, h),
                        from: id,
                        to: h,
                        weight: if id <= 34 { 0.65 } else { 0.1 },
                        enabled: true,
                    });
                }
            }
        }
        registry(&mut self.innovations);
        for g in &mut self.genomes {
            genome(g, &mut self.innovations);
        }
        for c in &mut self.councils {
            registry(&mut c.innovations);
            genome(&mut c.genome, &mut c.innovations);
            c.brain = None;
        }
        self.expand_map_and_food();
        self.init_people();
        self.refresh();
        self.format = 6;
        Ok(())
    }
}
