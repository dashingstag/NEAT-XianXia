//! Explicit world-edit resources and deterministic woodland placement.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Hill {
    pub x: f32,
    pub y: f32,
    pub radius: f32,
}
impl Default for Hill {
    fn default() -> Self {
        Self {
            x: 1500.0,
            y: 1380.0,
            radius: 360.0,
        }
    }
}
impl Hill {
    pub(super) fn validate(self) -> Result<(), String> {
        if self.radius.is_finite()
            && self.radius > 0.0
            && (self.radius..=WORLD_WIDTH - self.radius).contains(&self.x)
            && (self.radius..=WORLD_HEIGHT - self.radius).contains(&self.y)
        {
            Ok(())
        } else {
            Err("Hill must fit inside the map".into())
        }
    }
    pub(super) fn clearance(self, x: f32, y: f32) -> f32 {
        dist(x, y, self.x, self.y) - self.radius
    }
}
impl World {
    fn forest_ground_clear(&self, x: f32, y: f32, r: f32) -> bool {
        x - r >= 20.0
            && x + r <= 2670.0
            && y - r >= 20.0
            && y + r <= 1900.0
            && self.hill.clearance(x, y) >= r + 25.0
            && self
                .patches
                .iter()
                .filter(|p| p.kind == 2)
                .all(|p| dist(x, y, p.x, p.y) >= r + 35.0)
            && self.markets.iter().all(|m| {
                dist(x, y, m.x, m.y) >= rest::settlement_radius(m.housing_blocks) + r + 90.0
            })
    }
    pub(super) fn redistribute_forests(&mut self) {
        let old = self.wildlife.forests.clone();
        let assignments: Vec<_> = self
            .patches
            .iter()
            .map(|p| {
                if [0, 1].contains(&p.kind) {
                    old.iter()
                        .enumerate()
                        .min_by(|(_, a), (_, b)| {
                            dist(p.x, p.y, a.x, a.y).total_cmp(&dist(p.x, p.y, b.x, b.y))
                        })
                        .and_then(|(i, f)| {
                            (dist(p.x, p.y, f.x, f.y) <= f.radius + 15.0).then_some(i)
                        })
                } else {
                    None
                }
            })
            .collect();
        let mut placed: Vec<(f32, f32, f32)> = vec![];
        for (i, f) in old.iter().enumerate() {
            let radius = assignments
                .iter()
                .enumerate()
                .filter(|(_, owner)| **owner == Some(i))
                .map(|(p, _)| dist(self.patches[p].x, self.patches[p].y, f.x, f.y) + 15.0)
                .fold(f.radius, f32::max)
                .min(140.0);
            let mut candidate = None;
            let mut best = f32::INFINITY;
            for gx in 0..19 {
                for gy in 0..13 {
                    let x = 160.0 + gx as f32 * 130.0;
                    let y = 160.0 + gy as f32 * 130.0;
                    if !self.forest_ground_clear(x, y, radius) {
                        continue;
                    }
                    let overlap: f32 = placed
                        .iter()
                        .map(|(px, py, pr)| (radius + pr + 20.0 - dist(x, y, *px, *py)).max(0.0))
                        .sum();
                    let preferred = if i % 10 < 8 {
                        0
                    } else if i % 10 == 8 {
                        1
                    } else {
                        2
                    };
                    let town = &self.markets[preferred.min(self.markets.len() - 1)];
                    let nearest = self
                        .markets
                        .iter()
                        .enumerate()
                        .min_by(|(_, a), (_, b)| {
                            dist(x, y, a.x, a.y).total_cmp(&dist(x, y, b.x, b.y))
                        })
                        .map(|(m, _)| m)
                        .unwrap();
                    let score = dist(x, y, town.x, town.y)
                        + dist(x, y, f.x, f.y) * 0.04
                        + overlap * 8.0
                        + if nearest != preferred { 2500.0 } else { 0.0 };
                    if score < best {
                        best = score;
                        candidate = Some((x, y));
                    }
                }
            }
            if let Some((x, y)) = candidate {
                let dx = x - f.x;
                let dy = y - f.y;
                let site = &mut self.wildlife.forests[i];
                site.x = x;
                site.y = y;
                site.radius = radius;
                for g in self
                    .wildlife
                    .grass
                    .iter_mut()
                    .filter(|g| g.forest as usize == i)
                {
                    g.x += dx;
                    g.y += dy;
                }
                for a in self
                    .wildlife
                    .animals
                    .iter_mut()
                    .filter(|a| a.forest as usize == i)
                {
                    a.x = (a.x + dx).clamp(10.0, 2650.0);
                    a.y = (a.y + dy).clamp(10.0, 1890.0);
                    a.goal_x = (a.goal_x + dx).clamp(10.0, 2650.0);
                    a.goal_y = (a.goal_y + dy).clamp(10.0, 1890.0);
                }
                for (p, owner) in self.patches.iter_mut().zip(&assignments) {
                    if *owner == Some(i) {
                        p.x += dx;
                        p.y += dy;
                    }
                }
                placed.push((x, y, radius));
            }
        }
        self.refresh();
    }
    pub(super) fn upgrade_landscape(&mut self, initial: bool) {
        self.redistribute_forests();
        for p in self
            .patches
            .iter_mut()
            .filter(|p| (4..=7).contains(&p.kind))
        {
            if initial {
                p.stock = 50_000;
                p.capacity = 50_000;
            } else if p.stock > 0 {
                p.stock += 50_000;
                p.capacity += 50_000;
                self.ledger.initial[p.kind] += 50_000;
            }
            p.renewal = 0.0;
            p.growth = 0.0;
        }
        self.refresh();
    }
    pub(super) fn farm_ground(&self, x: f32, y: f32, r: f32) -> bool {
        (30.0 + r..=2650.0 - r).contains(&x)
            && (30.0 + r..=1880.0 - r).contains(&y)
            && self.hill.clearance(x, y) > r
            && self
                .patches
                .iter()
                .filter(|p| p.kind == 2)
                .all(|p| dist(x, y, p.x, p.y) > 35.0 + r)
    }
    pub(super) fn relocate_fertile_land(&mut self) {
        for id in 0..self.fertility.len() {
            let r = &self.fertility[id];
            let radius = (r.width * r.width + r.height * r.height).sqrt() * 0.5;
            if self.farm_ground(r.x, r.y, radius) {
                continue;
            }
            let old = (r.x, r.y);
            let candidate = (0..32)
                .flat_map(|x| {
                    (0..22).map(move |y| (80.0 + x as f32 * 80.0, 80.0 + y as f32 * 80.0))
                })
                .filter(|(x, y)| self.farm_ground(*x, *y, radius))
                .min_by(|a, b| {
                    dist(a.0, a.1, old.0, old.1).total_cmp(&dist(b.0, b.1, old.0, old.1))
                });
            if let Some((x, y)) = candidate {
                self.fertility[id].x = x;
                self.fertility[id].y = y;
            }
        }
        for id in 0..self.farms.len() {
            if self.farm_ground(self.farms[id].x, self.farms[id].y, 26.0) {
                continue;
            }
            let m = self.farms[id].market as usize;
            let town = &self.markets[m];
            if let Some(r) = self
                .fertility
                .iter()
                .filter(|r| self.farm_ground(r.x, r.y, 70.0))
                .min_by(|a, b| {
                    dist(a.x, a.y, town.x, town.y).total_cmp(&dist(b.x, b.y, town.x, town.y))
                })
            {
                let rank = self.farms[..id]
                    .iter()
                    .filter(|f| f.market as usize == m)
                    .count();
                self.farms[id].x = r.x + (rank % 2) as f32 * 50.0 - 25.0;
                self.farms[id].y = r.y + (rank / 2) as f32 * 50.0 - 25.0;
            }
        }
    }
    pub fn add_resource(
        &mut self,
        kind: usize,
        x: f32,
        y: f32,
        quantity: u64,
    ) -> Result<u32, String> {
        if ![0, 4, 5, 6, 7].contains(&kind) || !(1..=1_000_000).contains(&quantity) {
            return Err("Choose berries or a metal mine, with 1..1,000,000 units".into());
        }
        if !x.is_finite()
            || !y.is_finite()
            || !(30.0..=2650.0).contains(&x)
            || !(30.0..=1880.0).contains(&y)
        {
            return Err("Place resources on land inside the map".into());
        }
        if kind != 0
            && self
                .wildlife
                .forests
                .iter()
                .any(|f| dist(x, y, f.x, f.y) < f.radius + 20.0)
        {
            return Err("Place mines on clear ground outside forests".into());
        }
        if self.markets.iter().any(|m| dist(x, y, m.x, m.y) < 65.0)
            || self.taverns.iter().any(|t| dist(x, y, t.x, t.y) < 35.0)
        {
            return Err("Choose open ground away from the town centre".into());
        }
        for m in &self.markets {
            for house in 0..m.housing_blocks {
                let (hx, hy) = rest::house_origin(house);
                if dist(
                    x,
                    y,
                    (m.x + hx).clamp(5.0, 2670.0) + 10.0,
                    (m.y + hy).clamp(5.0, 1900.0) + 7.5,
                ) < 25.0
                {
                    return Err("Choose open ground away from houses".into());
                }
            }
        }
        if self
            .patches
            .iter()
            .any(|p| p.stock > 0 && dist(x, y, p.x, p.y) < 25.0)
            || self
                .education
                .schools
                .iter()
                .any(|s| dist(x, y, s.x, s.y) < 35.0)
            || self.farms.iter().any(|f| dist(x, y, f.x, f.y) < 45.0)
            || self.graveyards.iter().any(|g| dist(x, y, g.x, g.y) < 50.0)
        {
            return Err("Choose clear ground away from existing sites".into());
        }
        if kind == 0
            && (self.hill.clearance(x, y) < 25.0
                || self
                    .patches
                    .iter()
                    .filter(|p| p.kind == 2)
                    .any(|p| dist(x, y, p.x, p.y) < 35.0))
        {
            return Err("Berries need soil away from rocky ground".into());
        }
        if self.patches.len() >= 4096 {
            return Err("World resource-site limit reached".into());
        }
        let id = self.patches.len() as u32;
        self.patches.push(Patch {
            id,
            kind,
            x,
            y,
            stock: quantity,
            capacity: quantity,
            renewal: if kind == 0 { 1.0 } else { 0.0 },
            growth: 0.0,
            age: 0.0,
        });
        // Player world edits are explicit initial-supply additions, never trade/mint income.
        self.ledger.initial[kind] += quantity;
        self.refresh();
        self.check()?;
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn open(w: &World) -> (f32, f32) {
        (0..24)
            .flat_map(|x| (0..16).map(move |y| (80.0 + x as f32 * 100.0, 80.0 + y as f32 * 100.0)))
            .find(|(x, y)| {
                w.wildlife
                    .forests
                    .iter()
                    .all(|f| dist(*x, *y, f.x, f.y) > f.radius + 25.0)
                    && w.markets.iter().all(|m| dist(*x, *y, m.x, m.y) > 70.0)
                    && w.patches.iter().all(|p| dist(*x, *y, p.x, p.y) > 40.0)
                    && dist(*x, *y, 1500.0, 1380.0) > 400.0
            })
            .unwrap()
    }
    #[test]
    fn moved_hill_controls_placement_snapshot_and_checkpoint() {
        let hill: Hill =
            serde_json::from_str(include_str!("../../../../world/shifted-hill.json")).unwrap();
        let w = World::new_with_hill(64, 42, hill).unwrap();
        assert!(!w.farm_ground(hill.x, hill.y, 0.0));
        assert!(!w.forest_ground_clear(hill.x, hill.y, 10.0));
        assert!(w
            .wildlife
            .forests
            .iter()
            .all(|f| hill.clearance(f.x, f.y) >= f.radius));
        assert_eq!(w.snapshot_metadata()["hill"], serde_json::json!(hill));
        let file =
            std::env::temp_dir().join(format!("shifted-hill-{}.snapshot", std::process::id()));
        w.save(&file).unwrap();
        assert_eq!(World::load(&file).unwrap().hill, hill);
        std::fs::remove_file(file).unwrap();
    }
    #[test]
    fn woodland_clears_stone_and_settlements_without_minting_goods() {
        let w = World::new(64, 42).unwrap();
        assert!(w
            .wildlife
            .forests
            .iter()
            .all(|f| w.forest_ground_clear(f.x, f.y, f.radius)));
        assert!(w
            .patches
            .iter()
            .filter(|p| (4..=7).contains(&p.kind))
            .all(|p| p.stock == 50_000 && p.capacity == 50_000 && p.renewal == 0.0));
        assert_eq!(w.stocks(), w.ledger.initial);
        w.check().unwrap();
    }
    #[test]
    fn editor_additions_are_accounted_and_invalid_sites_do_not_mutate_world() {
        let mut w = World::new(2, 42).unwrap();
        let (x, y) = open(&w);
        let old = w.stocks();
        let count = w.patches.len();
        assert!(w.add_resource(4, f32::NAN, y, 10).is_err());
        assert!(w.add_resource(4, 3000.0, y, 10).is_err());
        assert!(w.add_resource(3, x, y, 10).is_err());
        let forest = &w.wildlife.forests[0];
        assert!(w.add_resource(4, forest.x, forest.y, 10).is_err());
        assert_eq!(w.patches.len(), count);
        let id = w.add_resource(4, x, y, 12345).unwrap();
        assert_eq!(id as usize, count);
        assert_eq!(w.stocks()[4], old[4] + 12345);
        assert_eq!(w.patches[id as usize].renewal, 0.0);
        w.check().unwrap();
        let (bx, by) = open(&w);
        let berry = w.add_resource(0, bx, by, 600).unwrap();
        assert_eq!(w.patches[berry as usize].capacity, 600);
        assert_eq!(w.patches[berry as usize].renewal, 1.0);
        w.check().unwrap();
    }
    #[test]
    fn woodland_changes_leave_player_berries_on_open_ground_in_place() {
        let mut w = World::new(2, 42).unwrap();
        let (x, y) = open(&w);
        let id = w.add_resource(0, x, y, 600).unwrap() as usize;
        w.redistribute_forests();
        assert_eq!((w.patches[id].x, w.patches[id].y), (x, y));
        w.check().unwrap();
    }
    #[test]
    fn exhausted_ore_never_renews_and_save_restore_does_not_refill_it() {
        let mut w = World::new(2, 42).unwrap();
        let (x, y) = open(&w);
        let id = w.add_resource(7, x, y, 2).unwrap() as usize;
        w.agents.x[0] = x;
        w.agents.y[0] = y;
        w.agents.action[0] = Action::Gather;
        w.agents.target[0] = id as u32;
        w.agents.cargo_kind[0] = 7;
        w.agents.traits[0] = 0;
        for _ in 0..2 {
            w.agents.timer[0] = 1.0;
            w.execute(0);
        }
        assert_eq!(w.patches[id].stock, 0);
        assert_eq!(w.agents.cargo[0], 2);
        for _ in 0..100 {
            w.step(false);
        }
        assert_eq!(w.patches[id].stock, 0);
        let path =
            std::env::temp_dir().join(format!("xianxia-exhausted-{}.snapshot", std::process::id()));
        w.save(&path).unwrap();
        let restored = World::load(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(restored.patches[id].stock, 0);
        assert_eq!(restored.patches[id].capacity, 2);
        restored.check().unwrap();
    }
}
