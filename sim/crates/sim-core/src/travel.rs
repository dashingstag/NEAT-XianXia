//! Deterministic local travel coordination and material-funded settlement founding.
use super::*;
use std::collections::BTreeMap;

const LONG_TRIP: f32 = 180.0;
const VICINITY: f32 = 160.0;
pub const MAX_CONVOY: usize = 50;
const WAIT: u64 = 30;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Convoy {
    pub members: Vec<u32>,
    pub x: f32,
    pub y: f32,
    pub goal_x: f32,
    pub goal_y: f32,
    pub created: u64,
    pub departed: bool,
}

fn segment_distance_points(x: f32, y: f32, a: (f32, f32), b: (f32, f32)) -> f32 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let l2 = dx * dx + dy * dy;
    if l2 == 0.0 {
        return dist(x, y, a.0, a.1);
    }
    let t = (((x - a.0) * dx + (y - a.1) * dy) / l2).clamp(0.0, 1.0);
    dist(x, y, a.0 + dx * t, a.1 + dy * t)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn walkers_slide_along_shores_without_entering_water() {
        let mut w = World::new_with_size(40, 7, Hill::default(), MapSize::X4).unwrap();
        let r = w.map.raster();
        let mut spot = None;
        'outer: for j in 1..r.rows - 1 {
            for i in 1..r.cols - 1 {
                if r.terrain[j * r.cols + i] == Terrain::Water as u8 {
                    continue;
                }
                for (di, dj) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
                    let n = ((j as i32 + dj) * r.cols as i32 + i as i32 + di) as usize;
                    if r.terrain[n] == Terrain::Water as u8 {
                        spot = Some((
                            (i as f32 + 0.5) * r.cell,
                            (j as f32 + 0.5) * r.cell,
                            di as f32,
                            dj as f32,
                        ));
                        break 'outer;
                    }
                }
            }
        }
        let Some((x, y, dx, dy)) = spot else { return };
        w.agents.x[0] = x;
        w.agents.y[0] = y;
        let (tx, ty) = (x + dx * 200.0, y + dy * 200.0);
        for _ in 0..600 {
            w.walk_toward(0, tx, ty, 4.0);
            assert_ne!(
                w.map.terrain_at(w.agents.x[0], w.agents.y[0]),
                Terrain::Water
            );
        }
    }
    #[test]
    fn roads_detour_around_water_bodies() {
        let mut w = World::new_with_size(40, 7, Hill::default(), MapSize::X4).unwrap();
        // Find a horizontal land-water-land stretch wide enough for the
        // 64-unit routing grid to register the water.
        let r = w.map.raster();
        let w0 = Terrain::Water as u8;
        let mut span = None;
        'outer: for j in 0..r.rows {
            let mut start = None;
            for i in 0..r.cols {
                if r.terrain[j * r.cols + i] == w0 {
                    start = start.or(Some(i));
                } else if let Some(s) = start {
                    if i - s >= 10 && s > 0 && i < r.cols - 1 {
                        span = Some((s - 1, i, j));
                        break 'outer;
                    }
                    start = None;
                }
            }
        }
        let Some((land_a, land_b, row)) = span else { return };
        let (ax, ay) = ((land_a as f32 + 0.5) * r.cell, (row as f32 + 0.5) * r.cell);
        let (bx, by) = ((land_b as f32 + 0.5) * r.cell, (row as f32 + 0.5) * r.cell);
        assert!(!w.clear_shot(ax, ay, bx, by));
        w.markets[0].x = ax;
        w.markets[0].y = ay;
        w.markets[1].x = bx;
        w.markets[1].y = by;
        let path = w.route_around_water(0, 1);
        assert!(path.len() >= 3, "a lake between markets must detour");
        for seg in path.windows(2) {
            let (sx, sy) = seg[0];
            let (ex, ey) = seg[1];
            let d = dist(sx, sy, ex, ey);
            let n = (d / 12.0).ceil() as usize;
            for k in 0..=n {
                let t = k as f32 / n.max(1) as f32;
                assert_ne!(
                    w.map.terrain_at(sx + (ex - sx) * t, sy + (ey - sy) * t),
                    Terrain::Water,
                    "routed road must stay on dry land"
                );
            }
        }
        w.roads[0].built = true;
        w.sync_road_paths();
        let mid = ((ax + bx) * 0.5, (ay + by) * 0.5);
        assert!(w.road_distance(0, mid.0, mid.1) > 22.0,
            "the speed corridor follows the detour, not the straight line");
    }
    fn travelers(n: usize) -> World {
        let mut w = World::new(n, 42).unwrap();
        for i in 0..n {
            w.agents.traits[i] = 0;
            w.agents.x[i] = 180.0;
            w.agents.y[i] = 230.0;
            w.agents.action[i] = Action::Rest;
            w.agents.target[i] = 1;
        }
        w.tick = 10;
        w.travel_step();
        w
    }
    #[test]
    fn local_convoys_are_capped_and_do_not_teleport() {
        let mut w = travelers(105);
        let sizes: Vec<_> = w
            .convoys
            .iter()
            .flatten()
            .map(|c| c.members.len())
            .collect();
        assert_eq!(sizes, vec![50, 50, 5]);
        let before = w.agents.x.clone();
        w.tick += 1;
        w.travel_step();
        for (i, x) in before.iter().enumerate() {
            assert!(dist(*x, 230.0, w.agents.x[i], w.agents.y[i]) <= 4.81);
        }
        assert_eq!(w.agents.convoy.iter().filter(|&&c| c != NONE).count(), 105);
    }
    #[test]
    fn nearby_spot_is_reused_even_for_a_different_route() {
        let mut w = World::new(3, 42).unwrap();
        for i in 0..3 {
            w.agents.traits[i] = 0;
            w.agents.x[i] = 180.0 + i as f32 * 50.0;
            w.agents.y[i] = 230.0;
            w.agents.action[i] = Action::Rest;
            w.agents.target[i] = 1;
        }
        w.agents.target[1] = 2;
        w.tick = 10;
        w.travel_step();
        let groups: Vec<_> = w.convoys.iter().flatten().collect();
        assert_eq!(groups.len(), 2);
        assert!(groups.iter().all(|c| c.x == 180.0 && c.y == 230.0));
        assert_eq!(w.agents.convoy[0], w.agents.convoy[2]);
        assert_ne!(w.agents.convoy[0], w.agents.convoy[1]);
        assert_eq!(w.agents.x[2], 280.0);
    }
    #[test]
    fn nearest_compatible_group_wins_and_settlement_spots_remain_reusable() {
        let mut w = World::new(3, 42).unwrap();
        let (gx, gy) = (w.shelters[1].x, w.shelters[1].y);
        for i in 0..3 {
            w.agents.action[i] = Action::Rest;
            w.agents.target[i] = 1;
            w.agents.x[i] = 100.0 + i as f32 * 60.0;
            w.agents.y[i] = 230.0;
        }
        w.convoys = vec![
            Some(Convoy {
                members: vec![0],
                x: 100.0,
                y: 230.0,
                goal_x: gx,
                goal_y: gy,
                created: 10,
                departed: false,
            }),
            Some(Convoy {
                members: vec![1],
                x: 240.0,
                y: 230.0,
                goal_x: gx,
                goal_y: gy,
                created: 10,
                departed: false,
            }),
        ];
        w.agents.convoy[0] = 0;
        w.agents.convoy[1] = 1;
        w.tick = 10;
        w.travel_step();
        assert_eq!(w.agents.convoy[2], 1);
        let mut solo = World::new(1, 42).unwrap();
        solo.agents.x[0] = solo.markets[0].x + 100.0;
        solo.agents.y[0] = solo.markets[0].y;
        solo.agents.action[0] = Action::Rest;
        solo.agents.target[0] = 1;
        solo.tick = 10;
        solo.travel_step();
        let group = solo.convoys.iter().flatten().next().unwrap();
        assert_eq!(group.x, solo.markets[0].x);
        assert_eq!(group.y, solo.markets[0].y);
    }
    #[test]
    fn isolated_traveler_waits_then_leaves_alone() {
        let mut w = travelers(1);
        for _ in 0..29 {
            w.tick += 1;
            w.travel_step();
            assert_ne!(w.agents.convoy[0], NONE);
        }
        w.tick += 1;
        w.travel_step();
        assert_eq!(w.agents.convoy[0], NONE);
        assert_eq!(w.stats.solo_departures, 1);
    }
    #[test]
    fn distant_or_incompatible_travelers_do_not_join() {
        let mut w = World::new(3, 42).unwrap();
        for i in 0..3 {
            w.agents.traits[i] = 0;
            w.agents.x[i] = 180.0;
            w.agents.y[i] = 230.0;
            w.agents.action[i] = Action::Rest;
            w.agents.target[i] = 1;
        }
        w.agents.x[1] = 380.0;
        w.agents.target[2] = 2;
        w.tick = 10;
        w.travel_step();
        assert_eq!(w.convoys.iter().flatten().count(), 3);
    }
    #[test]
    fn convoy_center_is_faster_than_solo_and_deaths_release() {
        let mut w = travelers(2);
        for _ in 0..30 {
            w.tick += 1;
            w.travel_step();
        }
        let slot = w.agents.convoy[0] as usize;
        let c = w.convoys[slot].as_ref().unwrap();
        assert!(c.departed);
        let (x, y) = (c.x, c.y);
        w.tick += 1;
        w.travel_step();
        let c = w.convoys[slot].as_ref().unwrap();
        assert!((dist(x, y, c.x, c.y) - 4.35).abs() < 0.01);
        w.agents.alive[0] = false;
        w.tick += 1;
        w.travel_step();
        assert_eq!(w.agents.convoy, vec![NONE, NONE]);
    }
    #[test]
    fn passing_crowds_do_not_found_camps_but_local_workers_can() {
        let mut w = World::new(8, 42).unwrap();
        w.patches[0].x = 330.0;
        w.patches[0].y = 180.0;
        for i in 0..8 {
            w.agents.x[i] = 330.0;
            w.agents.y[i] = 180.0;
            w.agents.cargo_kind[i] = 1;
            w.agents.cargo[i] = 2;
            w.agents.action[i] = Action::Gather;
            w.agents.dwell_cell[i] = 3 + 4096;
            w.agents.target[i] = 0;
        }
        // Declare this fixture's transferred inventory as an initial endowment.
        w.ledger.initial = w.stocks();
        w.tick = 10;
        w.settlement_step();
        assert_eq!(w.markets.len(), 3);
        for i in 0..8 {
            w.agents.dwell_ticks[i] = 120;
        }
        w.patches[0].x = 600.0;
        w.tick = 20;
        w.settlement_step();
        assert_eq!(w.markets.len(), 3);
        w.patches[0].x = 330.0;
        w.tick = 20;
        w.settlement_step();
        assert_eq!(w.markets.len(), 4);
        assert_eq!(w.stats.camps_founded, 1);
        assert_eq!(w.markets[3].coins, 16);
        assert!(!w.markets[3].village);
        assert_eq!(w.markets[3].housing_blocks, 0);
        w.check().unwrap();
        w.markets[3].housing_blocks = 1;
        for i in 0..8 {
            w.agents.residence[i] = 121;
        }
        w.tick = 30;
        w.settlement_step();
        assert!(w.markets[3].village);
        w.check().unwrap();
    }
    #[test]
    fn distant_dwellers_do_not_fuse_into_one_founding_group() {
        // Cells (3,1) and (19,0) both hash to 19 under `x/96 + 16*(y/96)`
        // because the x range is wider than the multiplier. Eight gatherers
        // that could found a camp alone must not be diluted by four others
        // 1500 units away in a colliding cell.
        let mut w = World::new(12, 42).unwrap();
        for (i, (px, py)) in [(330.0, 180.0), (1850.0, 45.0)].iter().enumerate() {
            w.patches[i].x = *px;
            w.patches[i].y = *py;
            let n = if i == 0 { 8 } else { 4 };
            for j in 0..n {
                let a = i * 8 + j;
                w.agents.x[a] = px + 2.0;
                w.agents.y[a] = *py;
                w.agents.cargo_kind[a] = 1;
                w.agents.cargo[a] = 2;
                w.agents.action[a] = Action::Gather;
                w.agents.target[a] = i as u32;
            }
        }
        // Declare this fixture's transferred inventory as an initial endowment.
        w.ledger.initial = w.stocks();
        for _ in 0..140 {
            w.tick += 1;
            w.settlement_step();
        }
        assert_eq!(w.markets.len(), 4);
        assert_eq!(w.stats.camps_founded, 1);
        let camp = w.markets.last().unwrap();
        assert!(dist(camp.x, camp.y, 330.0, 180.0) < 50.0);
        w.check().unwrap();
    }
    #[test]
    fn insufficient_materials_cannot_create_a_settlement() {
        let mut w = World::new(8, 42).unwrap();
        w.patches[0].x = 330.0;
        w.patches[0].y = 180.0;
        for i in 0..8 {
            w.agents.x[i] = 330.0;
            w.agents.y[i] = 180.0;
            w.agents.action[i] = Action::Gather;
            w.agents.dwell_cell[i] = 3 + 4096;
            w.agents.target[i] = 0;
            w.agents.dwell_ticks[i] = 200;
        }
        w.tick = 10;
        w.settlement_step();
        assert_eq!(w.markets.len(), 3);
        assert!(w.markets.iter().all(|m| !m.village));
        w.check().unwrap();
    }
}

impl World {
    fn destination(&self, i: usize) -> Option<(f32, f32)> {
        let a = &self.agents;
        if matches!(a.action[i], Action::DiscussCamp | Action::MigrateToCamp) {
            return self.camp_rumour_destination(i);
        }
        if matches!(a.action[i], Action::Gravedigger | Action::BuildGraveyard) {
            return self.job_destination(i);
        }
        if matches!(
            a.action[i],
            Action::Court
                | Action::CareFamily
                | Action::CraftTool
                | Action::BuyTool
                | Action::Farm
                | Action::HarvestRice
                | Action::EquipTool
                | Action::ForageMeal
                | Action::Socialize
                | Action::BuildTavern
                | Action::BuildSchool
                | Action::Teach
                | Action::AttendSchool
                | Action::UrbanWork
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
                | Action::StoreCargo
        ) {
            return self.personal_destination(i);
        }
        if a.action[i] == Action::Rest {
            return self
                .shelters
                .get(a.target[i] as usize)
                .map(|_| self.shelter_destination(i, a.target[i] as usize));
        }
        if matches!(a.action[i], Action::MakeCamp | Action::Wait) {
            return Some((a.x[i], a.y[i]));
        }
        if a.action[i] == Action::Gather {
            self.patches.get(a.target[i] as usize).map(|p| (p.x, p.y))
        } else {
            let target = if a.action[i] == Action::Trade {
                a.home[i]
            } else {
                a.target[i]
            };
            self.markets.get(target as usize).map(|m| (m.x, m.y))
        }
    }
    pub(super) fn speed_at(&self, x: f32, y: f32) -> f32 {
        let base = if self.roads.iter().enumerate().any(|(idx, r)| {
            r.built && self.road_distance(idx, x, y) < 22.0
        }) {
            54.0
        } else {
            30.0
        };
        base * self.night_speed()
    }
    /// Distance to a road's corridor: its routed path on generated maps,
    /// or the historical straight segment where no path exists.
    fn road_distance(&self, idx: usize, x: f32, y: f32) -> f32 {
        let r = &self.roads[idx];
        if let Some(path) = self.road_paths.get(idx).filter(|p| p.len() >= 2) {
            path.windows(2)
                .map(|s| segment_distance_points(x, y, s[0], s[1]))
                .fold(f32::INFINITY, f32::min)
        } else {
            segment_distance(
                x,
                y,
                &self.markets[r.a as usize],
                &self.markets[r.b as usize],
            )
        }
    }
    /// Rebuild routed corridors when the road list has changed. The path
    /// is a deterministic function of map terrain and endpoints, so when
    /// it is computed does not matter for replay.
    pub(super) fn sync_road_paths(&mut self) {
        if self.road_paths.len() == self.roads.len() {
            return;
        }
        let ends: Vec<(usize, usize)> = self
            .roads
            .iter()
            .map(|r| (r.a as usize, r.b as usize))
            .collect();
        self.road_paths = ends
            .into_iter()
            .map(|(a, b)| self.route_around_water(a, b))
            .collect();
    }
    /// A* over a coarse water-impassable grid between two markets, then a
    /// greedy line-of-sight smoothing pass. Falls back to the straight
    /// segment when no dry route exists.
    fn route_around_water(&mut self, a: usize, b: usize) -> Vec<(f32, f32)> {
        let (ax, ay) = (self.markets[a].x, self.markets[a].y);
        let (bx, by) = (self.markets[b].x, self.markets[b].y);
        if self.map.spec.size == MapSize::Classic || self.clear_shot(ax, ay, bx, by) {
            return Vec::new();
        }
        const CELL: f32 = 64.0;
        let cols = (self.map.spec.width() / CELL).ceil() as usize;
        let rows = (self.map.spec.height() / CELL).ceil() as usize;
        let mut pass = vec![false; cols * rows];
        // A cell is walkable only when its centre and quarter-offset samples
        // are all dry, so routed segments cannot shave through water cells.
        for j in 0..rows {
            for i in 0..cols {
                let cx = (i as f32 + 0.5) * CELL;
                let cy = (j as f32 + 0.5) * CELL;
                let q = CELL * 0.25;
                pass[j * cols + i] = [(cx, cy), (cx - q, cy - q), (cx + q, cy + q), (cx - q, cy + q), (cx + q, cy - q)]
                    .into_iter()
                    .all(|(x, y)| self.map.terrain_at(x, y) != Terrain::Water);
            }
        }
        let cell_of = |x: f32, y: f32| {
            (
                (x / CELL).clamp(0.0, (cols - 1) as f32) as usize,
                (y / CELL).clamp(0.0, (rows - 1) as f32) as usize,
            )
        };
        let (si, sj) = cell_of(ax, ay);
        let (gi, gj) = cell_of(bx, by);
        // A* with deciunit costs: orthogonal 10, diagonal 14, octile heuristic.
        use std::collections::BinaryHeap;
        let mut g = vec![u64::MAX; cols * rows];
        let mut from = vec![usize::MAX; cols * rows];
        let mut heap = BinaryHeap::new();
        let h = |i: usize, j: usize| {
            let di = i.abs_diff(gi);
            let dj = j.abs_diff(gj);
            (di.max(dj) * 10 + di.min(dj) * 4) as u64
        };
        let start = sj * cols + si;
        g[start] = 0;
        heap.push((std::cmp::Reverse(h(si, sj)), start));
        let goal = gj * cols + gi;
        let mut found = false;
        while let Some((std::cmp::Reverse(_), at)) = heap.pop() {
            if at == goal {
                found = true;
                break;
            }
            let (i, j) = (at % cols, at / cols);
            for dj in -1i32..=1 {
                for di in -1i32..=1 {
                    if di == 0 && dj == 0 {
                        continue;
                    }
                    let (ni, nj) = (i as i32 + di, j as i32 + dj);
                    if ni < 0 || nj < 0 || ni >= cols as i32 || nj >= rows as i32 {
                        continue;
                    }
                    let n = nj as usize * cols + ni as usize;
                    // No corner cutting through water on diagonals.
                    if !pass[n]
                        || (di != 0 && dj != 0
                            && (!pass[j * cols + ni as usize]
                                || !pass[nj as usize * cols + i]))
                    {
                        continue;
                    }
                    let cost = g[at] + if di == 0 || dj == 0 { 10 } else { 14 };
                    if cost < g[n] {
                        g[n] = cost;
                        from[n] = at;
                        heap.push((
                            std::cmp::Reverse(cost + h(ni as usize, nj as usize)),
                            n,
                        ));
                    }
                }
            }
        }
        if !found {
            return vec![(ax, ay), (bx, by)];
        }
        let mut cells = vec![goal];
        let mut at = goal;
        while at != start {
            at = from[at];
            cells.push(at);
        }
        cells.reverse();
        let mut raw: Vec<(f32, f32)> = vec![(ax, ay)];
        raw.extend(cells.iter().map(|&c| {
            (
                (c % cols) as f32 * CELL + CELL * 0.5,
                (c / cols) as f32 * CELL + CELL * 0.5,
            )
        }));
        raw.push((bx, by));
        // Greedy line-of-sight smoothing over the water boundary.
        let mut path = vec![raw[0]];
        let mut cursor = 0;
        while cursor < raw.len() - 1 {
            let mut far = cursor + 1;
            for k in (cursor + 2)..raw.len() {
                if self.clear_shot(raw[cursor].0, raw[cursor].1, raw[k].0, raw[k].1) {
                    far = k;
                }
            }
            path.push(raw[far]);
            cursor = far;
        }
        path
    }
    fn clear_shot(&mut self, ax: f32, ay: f32, bx: f32, by: f32) -> bool {
        let d = dist(ax, ay, bx, by);
        let n = (d / 16.0).ceil().max(1.0) as usize;
        for k in 1..n {
            let t = k as f32 / n as f32;
            if self.map.terrain_at(ax + (bx - ax) * t, ay + (by - ay) * t)
                == Terrain::Water
            {
                return false;
            }
        }
        true
    }
    fn release(&mut self, members: &[u32]) {
        for &id in members {
            self.agents.convoy[id as usize] = NONE;
            self.agents.solo_until[id as usize] = self.tick + 120;
        }
    }
    pub(super) fn travel_description(&self, i: usize) -> String {
        if let Some(Some(c)) = self.convoys.get(self.agents.convoy[i] as usize) {
            format!(
                "{}  /  {} travelers  /  trip committed (emergencies may leave)",
                if c.departed {
                    "Convoy"
                } else {
                    "Rallying convoy"
                },
                c.members.len()
            )
        } else if self.tick < self.agents.solo_until[i] {
            "Individual travel / final approach".into()
        } else {
            "Local activity".into()
        }
    }
    /// Whether a walker may stand at (x, y). Classic maps keep the
    /// historical everything-walkable rule; generated maps treat water
    /// as impassable so shores shape real routes.
    pub(super) fn passable(&mut self, x: f32, y: f32) -> bool {
        self.map.spec.size == MapSize::Classic
            || self.map.terrain_at(x, y) != Terrain::Water
    }
    /// Steer `step` units from (x, y) toward (tx, ty). When the direct step
    /// lands in water, try progressively larger rotations either way so the
    /// mover follows the shoreline around the obstacle; deterministic order
    /// keeps replays exact.
    const DETOUR: &[f32] = &[15.0, 30.0, 45.0, 60.0, 75.0, 90.0];
    fn steer(&mut self, x: f32, y: f32, tx: f32, ty: f32, step: f32) -> Option<(f32, f32)> {
        let d = dist(x, y, tx, ty);
        if d <= 0.0 || step <= 0.0 {
            return None;
        }
        let ux = (tx - x) / d;
        let uy = (ty - y) / d;
        if self.passable(x + ux * step, y + uy * step) {
            return Some((x + ux * step, y + uy * step));
        }
        for &deg in Self::DETOUR {
            let (sin, cos) = deg.to_radians().sin_cos();
            for &s in [1.0f32, -1.0].iter() {
                let rx = ux * cos - uy * sin * s;
                let ry = ux * sin * s + uy * cos;
                let (nx, ny) = (x + rx * step, y + ry * step);
                if self.passable(nx, ny) {
                    return Some((nx, ny));
                }
            }
        }
        None
    }
    /// Move agent `i` toward (tx, ty) by `step` units, steering around water
    /// on generated maps; on Classic this is the historical straight step.
    pub(super) fn walk_toward(&mut self, i: usize, tx: f32, ty: f32, step: f32) {
        let (x, y) = (self.agents.x[i], self.agents.y[i]);
        if let Some((nx, ny)) = self.steer(x, y, tx, ty, step) {
            // Rivers are forded at half speed: the mover still makes
            // absolute progress each tick, it just crosses more slowly.
            let f = if self.map.terrain_at(nx, ny) == Terrain::River {
                0.5
            } else {
                1.0
            };
            self.agents.x[i] = x + (nx - x) * f;
            self.agents.y[i] = y + (ny - y) * f;
        }
    }
    pub(super) fn travel_step(&mut self) {
        self.sync_road_paths();
        // Pool slots are reused; inactive convoys do not accumulate in saves.
        for slot in 0..self.convoys.len() {
            let Some(mut c) = self.convoys[slot].take() else {
                continue;
            };
            c.members.retain(|&id| {
                let i = id as usize;
                if !self.agents.alive[i]
                    || self.agents.health[i] < 25.0
                    || self.agents.hunger[i] > 70.0
                {
                    self.agents.convoy[i] = NONE;
                    self.agents.solo_until[i] = self.tick + 120;
                    false
                } else {
                    true
                }
            });
            if c.members.is_empty() {
                continue;
            }
            if c.departed && c.members.len() < 2 {
                self.release(&c.members);
                continue;
            }
            let ready = c.members.iter().all(|&id| {
                dist(
                    self.agents.x[id as usize],
                    self.agents.y[id as usize],
                    c.x,
                    c.y,
                ) < 20.0
            });
            if !c.departed
                && ((self.tick - c.created >= WAIT && ready)
                    || (c.members.len() == MAX_CONVOY && ready))
            {
                if c.members.len() == 1 {
                    self.stats.solo_departures += 1;
                    self.release(&c.members);
                    continue;
                }
                c.departed = true;
                self.relationship_group(&c.members, relationships::Context::Caravan);
                self.stats.convoys_departed += 1;
            }
            // A straggler cannot hold a rally indefinitely.
            if !c.departed && self.tick - c.created > 90 {
                self.release(&c.members);
                continue;
            }
            if c.departed {
                let d = dist(c.x, c.y, c.goal_x, c.goal_y);
                if d < 10.0 {
                    self.release(&c.members);
                    continue;
                }
                let step = (c
                    .members
                    .iter()
                    .map(|&id| self.actor_speed(id as usize, c.x, c.y))
                    .fold(f32::INFINITY, f32::min)
                    * 1.45
                    * DT)
                    .min(d);
                if let Some((nx, ny)) = self.steer(c.x, c.y, c.goal_x, c.goal_y, step) {
                    c.x = nx;
                    c.y = ny;
                }
            }
            for (rank, &id) in c.members.iter().enumerate() {
                let i = id as usize;
                let x = c.x + (rank % 7) as f32 * 3.0 - 9.0;
                let y = c.y + (rank / 7) as f32 * 3.0 - 10.5;
                let d = dist(self.agents.x[i], self.agents.y[i], x, y);
                if d > 0.01 {
                    let speed = self.actor_speed(i, self.agents.x[i], self.agents.y[i]);
                    let step = (speed * DT * if c.departed { 1.6 } else { 1.0 }).min(d);
                    self.walk_toward(i, x, y, step);
                    self.agents.drain_energy(i, DT * 0.07);
                }
            }
            self.convoys[slot] = Some(c);
        }
        if self.tick % 10 != 0 {
            return;
        }
        let mut buckets: BTreeMap<(i32, i32), Vec<usize>> = BTreeMap::new();
        for (slot, c) in self.convoys.iter().enumerate() {
            if let Some(c) = c {
                if !c.departed {
                    buckets
                        .entry(((c.x / 64.0) as i32, (c.y / 64.0) as i32))
                        .or_default()
                        .push(slot);
                }
            }
        }
        let mut free: Vec<usize> = self
            .convoys
            .iter()
            .enumerate()
            .filter_map(|(i, c)| c.is_none().then_some(i))
            .collect();
        for i in 0..self.agents.len() {
            let a = &self.agents;
            if !a.alive[i]
                || a.convoy[i] != NONE
                || self.tick < a.solo_until[i]
                || a.health[i] < 25.0
                || a.hunger[i] > 70.0
            {
                continue;
            }
            let Some((gx, gy)) = self.destination(i) else {
                continue;
            };
            let (x, y) = (a.x[i], a.y[i]);
            if dist(x, y, gx, gy) < LONG_TRIP {
                continue;
            }
            let cell = ((x / 64.0) as i32, (y / 64.0) as i32);
            let mut found = None;
            let mut nearest_group = f32::INFINITY;
            let mut rally = None;
            let mut nearest_spot = f32::INFINITY;
            // Search existing gathering spots before allocating another group or spot.
            for dx in -3..=3 {
                for dy in -3..=3 {
                    if let Some(slots) = buckets.get(&(cell.0 + dx, cell.1 + dy)) {
                        for &slot in slots {
                            let c = self.convoys[slot].as_ref().unwrap();
                            let d = dist(x, y, c.x, c.y);
                            if d <= VICINITY {
                                if d < nearest_spot {
                                    nearest_spot = d;
                                    rally = Some((c.x, c.y));
                                }
                                if c.members.len() < MAX_CONVOY
                                    && dist(gx, gy, c.goal_x, c.goal_y) < 64.0
                                    && d < nearest_group
                                {
                                    found = Some(slot);
                                    nearest_group = d;
                                }
                            }
                        }
                    }
                }
            }
            // Settlement centres remain reusable meeting spots after convoys depart.
            if rally.is_none() {
                for m in &self.markets {
                    let d = dist(x, y, m.x, m.y);
                    if d <= VICINITY && d < nearest_spot {
                        nearest_spot = d;
                        rally = Some((m.x, m.y));
                    }
                }
            }
            let slot = if let Some(slot) = found {
                slot
            } else {
                let slot = free.pop().unwrap_or(self.convoys.len());
                let (rx, ry) = rally.unwrap_or((x, y));
                let c = Some(Convoy {
                    members: vec![],
                    x: rx,
                    y: ry,
                    goal_x: gx,
                    goal_y: gy,
                    created: self.tick,
                    departed: false,
                });
                if slot == self.convoys.len() {
                    self.convoys.push(c)
                } else {
                    self.convoys[slot] = c
                }
                buckets
                    .entry(((rx / 64.0) as i32, (ry / 64.0) as i32))
                    .or_default()
                    .push(slot);
                slot
            };
            let c = self.convoys[slot].as_mut().unwrap();
            c.members.push(i as u32);
            self.agents.convoy[i] = slot as u32;
        }
    }

    pub(super) fn settlement_step(&mut self) {
        let mut groups: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
        for i in 0..self.agents.len() {
            if !self.agents.alive[i] {
                continue;
            }
            let a = &mut self.agents;
            let m = &self.markets[a.home[i] as usize];
            let at_home =
                dist(a.x[i], a.y[i], m.x, m.y) < rest::settlement_radius(m.housing_blocks);
            if at_home
                && a.convoy[i] == NONE
                && matches!(
                    a.action[i],
                    Action::Rest | Action::Build | Action::WorkForFood
                )
            {
                a.residence[i] = a.residence[i].saturating_add(1);
            }
            // Only sustained local workers, never passing or waiting convoys, can found a camp.
            // Column stride must exceed the widest possible map so distant cells never share a key.
            let cell = (a.x[i] / 96.0) as u32 + 4096 * (a.y[i] / 96.0) as u32;
            let eligible = a.convoy[i] == NONE
                && a.action[i] == Action::Gather
                && a.energy[i] > 20.0
                && a.hunger[i] < 60.0
                && a.x[i] < 2700.0
                && self
                    .patches
                    .get(a.target[i] as usize)
                    .is_some_and(|p| dist(a.x[i], a.y[i], p.x, p.y) < 12.0);
            if eligible && a.dwell_cell[i] == cell {
                a.dwell_ticks[i] = a.dwell_ticks[i].saturating_add(1)
            } else if eligible {
                a.dwell_cell[i] = cell;
                a.dwell_ticks[i] = 0;
            }
            if self.tick % 10 == 0 && eligible && a.dwell_ticks[i] >= 120 {
                groups.entry(cell).or_default().push(i);
            }
        }
        if self.tick % 10 != 0 {
            return;
        }
        for ids in groups.values() {
            if self.markets.len() >= 64 || ids.len() < 8 {
                continue;
            }
            let x = ids.iter().map(|&i| self.agents.x[i]).sum::<f32>() / ids.len() as f32;
            let y = ids.iter().map(|&i| self.agents.y[i]).sum::<f32>() / ids.len() as f32;
            if self.markets.iter().any(|m| dist(x, y, m.x, m.y) < 120.0) {
                continue;
            }
            let founders: Vec<usize> = ids
                .iter()
                .copied()
                .filter(|&i| {
                    self.agents.coins[i] >= 2
                        && self.agents.berries[i] > 1
                        && dist(x, y, self.agents.x[i], self.agents.y[i]) < 65.0
                })
                .take(32)
                .collect();
            let timber: u32 = founders
                .iter()
                .filter(|&&i| self.agents.cargo_kind[i] == 1)
                .map(|&i| self.agents.cargo[i])
                .sum();
            if founders.len() < 8 || timber < 12 {
                continue;
            }
            let id = self.markets.len() as u32;
            let mut m = self.markets[0].clone();
            m.id = id;
            m.name = format!("Waycamp {}", id - 2);
            m.x = x;
            m.y = y;
            m.coins = 0;
            m.goods = std::array::from_fn(|_| VecDeque::new());
            m.quotes = std::array::from_fn(|_| Quote::default());
            m.nearest = [NONE; GOODS];
            m.population = founders.len() as u32;
            m.housing_blocks = 0;
            m.stable_residents = 0;
            m.village = false;
            m.project = None;
            m.sales = [0; GOODS];
            m.local_sales = [0; GOODS];
            m.imported_sales = [0; GOODS];
            m.revenue_total = 0;
            m.expenses_total = 0;
            let mut needed = 12;
            for &i in &founders {
                let a = &mut self.agents;
                a.coins[i] -= 2;
                m.coins += 2;
                a.berries[i] -= 1;
                m.add(0, 1, a.home[i]);
                if a.cargo_kind[i] == 1 {
                    let q = a.cargo[i].min(needed);
                    a.cargo[i] -= q;
                    needed -= q;
                }
                a.home[i] = id;
                a.residence[i] = 0;
                a.dwell_ticks[i] = 0;
            }
            self.ledger.built[1] += 12; // communal timber shelters, not a free permanent housing block
            let nearest = self
                .markets
                .iter()
                .min_by(|a, b| dist(x, y, a.x, a.y).total_cmp(&dist(x, y, b.x, b.y)))
                .unwrap()
                .id;
            self.roads.push(Road {
                a: nearest,
                b: id,
                built: false,
                under_construction: false,
                traffic: 0,
            });
            self.markets.push(m);
            self.select_camp_leader(id, &founders);
            self.announce_new_camp(id, &founders);
            self.sales_profile_labels.push(String::new());
            self.stats.camps_founded += 1;
        }
        for m in &mut self.markets {
            m.population = 0;
            m.stable_residents = 0;
        }
        for i in 0..self.agents.len() {
            if self.agents.alive[i] {
                let m = &mut self.markets[self.agents.home[i] as usize];
                m.population += 1;
                if self.agents.residence[i] >= 120 {
                    m.stable_residents += 1;
                }
            }
        }
        for m in &mut self.markets {
            if !m.village
                && m.stable_residents >= 8
                && m.housing_blocks > 0
                && m.stock(0) + m.stock(3) >= 8
            {
                m.village = true;
            }
            if m.population < 4 {
                m.village = false;
            }
        }
        self.sync_councils();
        self.sync_urban();
        self.sync_education();
    }
}
