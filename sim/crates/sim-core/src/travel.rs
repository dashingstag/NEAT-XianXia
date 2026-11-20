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

/// A blockage a stranded traveler could not get around.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct PendingBlock {
    pub agent: u32,
    pub x: f32,
    pub y: f32,
    pub tx: f32,
    pub ty: f32,
    pub tick: u64,
}

/// A blockage report physically delivered to a settlement. Villages
/// read these when planning ports (M8-P2); `reports` counts how many
/// travelers hit the same stretch of water.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct WaterBlock {
    pub market: u32,
    pub x: f32,
    pub y: f32,
    pub tx: f32,
    pub ty: f32,
    pub tick: u64,
    pub reports: u16,
}

/// A paired ferry landing planned by a village in response to delivered
/// WaterBlocks. The near side sits on home shore; `lx, ly` is the first
/// buildable ground marching from the block toward what the walker was
/// trying to reach. `ferry` marks landings no dry route can reach — the
/// far side then waits for a ship to carry builders over (M8-P3).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Port {
    pub market: u32,
    pub x: f32,
    pub y: f32,
    pub lx: f32,
    pub ly: f32,
    pub near_done: bool,
    pub far_done: bool,
    pub ferry: bool,
    /// Agents waiting at the home-shore dock for the ship.
    pub queue: Vec<u32>,
    /// Agents waiting at the landing dock for the return trip.
    pub queue_far: Vec<u32>,
}

/// A ferry shuttling between a port's two docks. It sails once eight
/// passengers board (or one has waited a long while), carries up to a
/// hundred, and returns whenever a queue forms on the far side.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Ship {
    pub port: u32,
    pub x: f32,
    pub y: f32,
    /// Currently moored at the landing dock rather than the home dock.
    pub at_far: bool,
    /// Passengers aboard; their positions pin to the ship like a convoy.
    pub aboard: Vec<u32>,
    pub departed: bool,
    pub docked_since: u64,
    /// Waypoints for the current leg, computed once per crossing.
    pub path: Vec<(f32, f32)>,
    pub leg: u16,
}

/// Water-route reporting state. Persisted so pending reports survive
/// checkpoints; `progress` is a per-agent movement probe rebuilt the
/// same way on load.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct WaterState {
    /// idx -> (x, y, since): last position from which the agent failed
    /// to make 24 units of progress while still having a destination.
    pub progress: Vec<Option<(f32, f32, u64)>>,
    /// Reports carried by stranded agents until they reach home.
    pub pending: Vec<PendingBlock>,
    /// Reports delivered to markets, deduped by 96-unit cell.
    pub blocks: Vec<WaterBlock>,
    /// Paired landings planned from delivered blocks.
    pub ports: Vec<Port>,
    /// Ferries built at finished ports.
    pub ships: Vec<Ship>,
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
        // March the probe along (dx, dy) through the water until land
        // reappears: that far shore is a genuinely reachable target.
        let mut target = None;
        for k in (8..=80).step_by(4) {
            let (px, py) = (x + dx * k as f32 * 8.0, y + dy * k as f32 * 8.0);
            let t = w.map.terrain_at(px, py);
            if t != Terrain::Water && t != Terrain::River {
                target = Some((px, py));
                break;
            }
        }
        let Some((tx, ty)) = target else { return };
        let mut reached = false;
        for _ in 0..1200 {
            w.walk_toward(0, tx, ty, 4.0);
            assert_ne!(
                w.map.terrain_at(w.agents.x[0], w.agents.y[0]),
                Terrain::Water
            );
            if dist(w.agents.x[0], w.agents.y[0], tx, ty) < 10.0 {
                reached = true;
                break;
            }
        }
        // Sticky-side steering must round the shore and arrive instead of
        // oscillating in a shoreline pocket.
        assert!(
            reached,
            "walker never reached ({tx},{ty}) from ({x},{y}); ended at ({},{})",
            w.agents.x[0],
            w.agents.y[0]
        );
        // A target that is itself water resolves to the nearest dry shore
        // rather than stranding the mover circling the bank.
        w.agents.x[0] = x;
        w.agents.y[0] = y;
        let wet = (x + dx * 120.0, y + dy * 120.0);
        let expect = w.map.dry_ground(wet.0, wet.1);
        for _ in 0..600 {
            w.walk_toward(0, wet.0, wet.1, 4.0);
            assert_ne!(
                w.map.terrain_at(w.agents.x[0], w.agents.y[0]),
                Terrain::Water
            );
        }
        assert!(
            dist(w.agents.x[0], w.agents.y[0], expect.0, expect.1) < 48.0,
            "water target should settle the walker near the dry shore {:?}, not ({},{})",
            expect,
            w.agents.x[0],
            w.agents.y[0]
        );
    }
    #[test]
    fn stranded_walkers_report_blocks_home() {
        let mut w = World::new_with_size(24, 7, Hill::default(), MapSize::X4).unwrap();
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
        let (tx, ty) = (x + dx * 400.0, y + dy * 400.0);
        w.agents.x[0] = x;
        w.agents.y[0] = y;
        // Establish the progress probe, then let 600+ ticks pass with
        // the walker pinned against the water.
        for _ in 0..8 {
            w.walk_toward(0, tx, ty, 4.0);
        }
        w.tick += 601;
        for _ in 0..4 {
            w.walk_toward(0, tx, ty, 4.0);
        }
        assert_eq!(w.water.pending.len(), 1, "stranded walker carries a report");
        // Reaching home ground delivers the report to the market.
        let home = w.agents.home[0] as usize;
        w.agents.x[0] = w.markets[home].x;
        w.agents.y[0] = w.markets[home].y;
        w.update_water_reports();
        assert!(w.water.pending.is_empty());
        assert_eq!(w.water.blocks.len(), 1);
        assert_eq!(w.water.blocks[0].market, home as u32);
        assert_eq!(w.water.blocks[0].reports, 1);
        // A second report at the same shore dedupes into `reports`.
        w.water.pending.push(PendingBlock {
            agent: 1,
            x: w.water.blocks[0].x + 8.0,
            y: w.water.blocks[0].y,
            tx,
            ty,
            tick: w.tick,
        });
        w.agents.home[1] = home as u32;
        w.agents.x[1] = w.markets[home].x;
        w.agents.y[1] = w.markets[home].y;
        w.update_water_reports();
        assert_eq!(w.water.blocks.len(), 1);
        assert_eq!(w.water.blocks[0].reports, 2);
    }
    #[test]
    fn zoned_village_plans_and_builds_port() {
        let mut w = World::new_with_size(40, 7, Hill::default(), MapSize::X4).unwrap();
        let r = w.map.raster();
        // Find a land→water→land stripe along a raster row: the block sits
        // on the near shore and the survey must land on the far shore.
        let mut span = None;
        'outer: for j in 1..r.rows - 1 {
            for i in 1..r.cols - 2 {
                if r.terrain[j * r.cols + i] == Terrain::Water as u8 {
                    continue;
                }
                let mut k = i + 1;
                while k < r.cols - 1 && r.terrain[j * r.cols + k] == Terrain::Water as u8 {
                    k += 1;
                }
                if k > i + 1
                    && k < r.cols - 1
                    && r.terrain[j * r.cols + k] != Terrain::Water as u8
                {
                    span = Some((
                        (i as f32 + 0.5) * r.cell,
                        (j as f32 + 0.5) * r.cell,
                        (k as f32 + 0.5) * r.cell,
                    ));
                    break 'outer;
                }
            }
        }
        let Some((bx, by, tx)) = span else { return };
        w.water.blocks.push(WaterBlock {
            market: 0,
            x: bx,
            y: by,
            tx,
            ty: by,
            tick: 0,
            reports: 1,
        });
        w.urban.states[0].zoned = true;
        w.markets[0].coins = 500;
        w.markets[0].add(1, 20, 0);
        w.markets[0].add(2, 4, 0);
        w.plan_ports();
        assert_eq!(w.water.ports.len(), 1, "block should plan one port");
        let p = w.water.ports[0].clone();
        assert!(w.map.buildable(p.x, p.y), "home port must be dry");
        assert!(w.map.buildable(p.lx, p.ly), "landing port must be dry");
        let near = w
            .urban
            .jobs
            .iter()
            .enumerate()
            .find_map(|(id, j)| j.as_ref().filter(|j| j.kind == 4 && j.source == 0).map(|_| id))
            .expect("near-side shipwright job posted");
        // A worker standing on the site completes the near side.
        w.agents.action[0] = Action::UrbanWork;
        w.agents.target[0] = near as u32;
        w.agents.x[0] = p.x;
        w.agents.y[0] = p.y;
        for _ in 0..200 {
            w.execute_urban_job(0);
            if w.water.ports[0].near_done {
                break;
            }
        }
        assert!(w.water.ports[0].near_done, "shipwright finishes the port");
    }
    #[test]
    fn ship_ferries_queued_walkers_across_water() {
        let mut w = World::new_with_size(40, 7, Hill::default(), MapSize::X4).unwrap();
        let r = w.map.raster();
        let mut span = None;
        'outer: for j in 1..r.rows - 1 {
            for i in 1..r.cols - 2 {
                if r.terrain[j * r.cols + i] == Terrain::Water as u8 {
                    continue;
                }
                let mut k = i + 1;
                while k < r.cols - 1 && r.terrain[j * r.cols + k] == Terrain::Water as u8 {
                    k += 1;
                }
                if k > i + 1
                    && k < r.cols - 1
                    && r.terrain[j * r.cols + k] != Terrain::Water as u8
                {
                    span = Some((
                        (i as f32 + 0.5) * r.cell,
                        (j as f32 + 0.5) * r.cell,
                        (k as f32 + 0.5) * r.cell,
                    ));
                    break 'outer;
                }
            }
        }
        let Some((bx, by, lx)) = span else { return };
        w.water.ports.push(Port {
            market: 0,
            x: bx,
            y: by,
            lx,
            ly: by,
            near_done: true,
            far_done: true,
            ferry: true,
            queue: Vec::new(),
            queue_far: Vec::new(),
        });
        w.water.ships.push(Ship {
            port: 0,
            x: bx,
            y: by,
            at_far: false,
            aboard: Vec::new(),
            departed: false,
            docked_since: w.tick,
            path: Vec::new(),
            leg: 0,
        });
        for i in 0..8u32 {
            w.water.ports[0].queue.push(i);
            w.agents.x[i as usize] = bx;
            w.agents.y[i as usize] = by;
        }
        for _ in 0..6000 {
            w.ship_step();
            if w.water.ships[0].at_far {
                break;
            }
        }
        assert!(w.water.ships[0].at_far, "ship should cross to the landing");
        assert!(w.water.ships[0].aboard.is_empty(), "riders disembark");
        for i in 0..8usize {
            let d = dist(w.agents.x[i], w.agents.y[i], lx, by);
            assert!(d < 40.0, "rider {i} left {d} from the far dock");
        }
    }
    #[test]
    fn end_to_end_strand_report_port_ship() {
        // The whole M8 chain on one seed: a walker strands at a shore,
        // carries the blockage home, the zoned village plans and escrows a
        // port, a builder finishes the dock, a crew launches the ferry and
        // a queued assembly crosses the water aboard it.
        let mut w = World::new_with_size(40, 7, Hill::default(), MapSize::X4).unwrap();
        // The span must be a real ferry gap: land-blocked for walkers but
        // still sailable. Collect stripes from the raster, then keep the
        // first the routers agree on.
        let r = w.map.raster();
        let mut candidates = Vec::new();
        for j in 1..r.rows - 1 {
            for i in 1..r.cols - 2 {
                if r.terrain[j * r.cols + i] == Terrain::Water as u8 {
                    continue;
                }
                let mut k = i + 1;
                while k < r.cols - 1 && r.terrain[j * r.cols + k] == Terrain::Water as u8 {
                    k += 1;
                }
                if k > i + 1 && k < r.cols - 1 {
                    let t = r.terrain[j * r.cols + k];
                    if t != Terrain::Water as u8 {
                        candidates.push((
                            (i as f32 + 0.5) * r.cell,
                            (j as f32 + 0.5) * r.cell,
                            (k as f32 + 0.5) * r.cell,
                        ));
                    }
                }
            }
        }
        let mut span = None;
        for (bx, by, lx) in candidates {
            if w.route_between(bx, by, lx, by).is_none()
                && w.route_over_water(bx, by, lx, by).is_some()
            {
                span = Some((bx, by, lx));
                break;
            }
        }
        let Some((bx, by, lx)) = span else { return };
        // Stage 1 — agent 0 strands: two probes 700 ticks apart with no
        // progress push a pending report.
        w.agents.home[0] = 0;
        w.agents.x[0] = bx;
        w.agents.y[0] = by;
        w.walk_toward(0, lx, by, 1.0);
        w.tick += 700;
        w.walk_toward(0, lx, by, 1.0);
        assert!(
            w.water.pending.iter().any(|p| p.agent == 0),
            "stranded walker carries a report"
        );
        // Stage 2 — the bearer reaches home ground and delivers it.
        w.agents.x[0] = w.markets[0].x;
        w.agents.y[0] = w.markets[0].y;
        w.update_water_reports();
        assert_eq!(w.water.blocks.len(), 1, "report delivered to the village");
        // Stage 3 — the zoned village escrows materials and posts the
        // near-side shipwright job.
        w.urban.states[0].zoned = true;
        w.markets[0].coins = 500;
        w.markets[0].add(1, 60, 0);
        w.markets[0].add(2, 4, 0);
        w.plan_ports();
        assert_eq!(w.water.ports.len(), 1);
        let port = w.water.ports[0].clone();
        let near = w
            .urban
            .jobs
            .iter()
            .enumerate()
            .find_map(|(id, j)| j.as_ref().filter(|j| j.kind == 4 && j.source == 0).map(|_| id))
            .expect("near-side build job posted");
        // Stage 4 — a builder stands on the site and finishes the dock.
        w.agents.action[1] = Action::UrbanWork;
        w.agents.target[1] = near as u32;
        w.agents.x[1] = port.x;
        w.agents.y[1] = port.y;
        for _ in 0..200 {
            w.execute_urban_job(1);
            if w.water.ports[0].near_done {
                break;
            }
        }
        assert!(w.water.ports[0].near_done, "dock built");
        // Stage 5 — the gap is land-blocked, so the port is a ferry and
        // the village commissions a ship on the next planning pass.
        assert!(w.water.ports[0].ferry, "land-blocked gap needs a ferry");
        w.plan_ports();
        let ship_job = w
            .urban
            .jobs
            .iter()
            .enumerate()
            .find_map(|(id, j)| j.as_ref().filter(|j| j.kind == 4 && j.source == 2).map(|_| id))
            .expect("ship commissioned once the dock stands");
        w.agents.action[1] = Action::UrbanWork;
        w.agents.target[1] = ship_job as u32;
        for _ in 0..200 {
            w.execute_urban_job(1);
            if !w.water.ships.is_empty() {
                break;
            }
        }
        assert_eq!(w.water.ships.len(), 1, "ferry launched");
        // Stage 6 — stranded walkers queue at the dock when their target
        // is genuinely unreachable over land; the join predicate is
        // asserted directly, then eight riders board and cross.
        let (ax, ay) = (port.x, port.y);
        w.agents.x[0] = ax;
        w.agents.y[0] = ay;
        let expect_join = w.route_between(ax, ay, port.lx, port.ly).is_none()
            && w.route_between(port.lx, port.ly, port.lx, port.ly + 8.0).is_some();
        assert_eq!(w.try_board(0, port.lx, port.ly + 8.0), expect_join);
        for i in 0..8u32 {
            w.water.ports[0].queue.push(i);
            w.agents.x[i as usize] = ax;
            w.agents.y[i as usize] = ay;
        }
        for _ in 0..6000 {
            w.ship_step();
            if w.water.ships[0].at_far {
                break;
            }
        }
        assert!(w.water.ships[0].at_far, "ferry crossed the gap");
        assert!(w.water.ships[0].aboard.is_empty(), "riders disembarked");
        for i in 0..8usize {
            assert!(
                dist(w.agents.x[i], w.agents.y[i], port.lx, port.ly) < 40.0,
                "rider {i} stands at the landing"
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
    fn only_departed_convoys_hide_members_in_viewer_records() {
        let mut w = travelers(3);
        w.convoys = vec![
            Some(Convoy {
                members: vec![0],
                x: 180.0,
                y: 230.0,
                goal_x: 0.0,
                goal_y: 0.0,
                created: 0,
                departed: true,
            }),
            Some(Convoy {
                members: vec![1],
                x: 240.0,
                y: 230.0,
                goal_x: 0.0,
                goal_y: 0.0,
                created: 0,
                departed: false,
            }),
        ];
        for membership in w.agents.convoy.iter_mut() {
            *membership = NONE;
        }
        w.agents.convoy[0] = 0;
        w.agents.convoy[1] = 1;
        let bytes = w.snapshot(3, None, true);
        let jsonlen = u32::from_le_bytes(bytes[24..28].try_into().unwrap()) as usize;
        let flags = |id: u32| {
            bytes[32 + jsonlen..32 + jsonlen + 48]
                .chunks_exact(16)
                .find(|r| u32::from_le_bytes(r[..4].try_into().unwrap()) == id)
                .map(|r| r[12] & 128 != 0)
        };
        assert_eq!(flags(0), Some(true), "departed member hidden");
        assert_eq!(flags(1), Some(false), "rallying member visible");
        assert_eq!(flags(2), Some(false), "independent agent visible");
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
        self.route_between(ax, ay, bx, by)
            .unwrap_or_else(|| vec![(ax, ay), (bx, by)])
    }
    /// Dry-land route between two points: `None` when water truly blocks
    /// every path, `Some(empty)` when the direct shot is already dry,
    /// otherwise the routed waypoint list.
    pub(super) fn route_between(
        &mut self,
        ax: f32,
        ay: f32,
        bx: f32,
        by: f32,
    ) -> Option<Vec<(f32, f32)>> {
        if self.map.spec.size == MapSize::Classic || self.clear_shot(ax, ay, bx, by) {
            return Some(Vec::new());
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
            return None;
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
        Some(path)
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
    pub(super) fn passable(&self, x: f32, y: f32) -> bool {
        self.map.spec.size == MapSize::Classic
            || self.map.terrain_at(x, y) != Terrain::Water
    }
    /// Water-only A* between two docks on the same coarse grid the road
    /// router uses, pass cells inverted: every sample must be water or
    /// river, except inside a 32-unit mooring zone at either dock. The
    /// crossing a port actually needs is then precomputed once per leg,
    /// which lets ferries round headlands that stall greedy steering.
    fn route_over_water(
        &mut self,
        ax: f32,
        ay: f32,
        bx: f32,
        by: f32,
    ) -> Option<Vec<(f32, f32)>> {
        const CELL: f32 = 64.0;
        let cols = (self.map.spec.width() / CELL).ceil() as usize;
        let rows = (self.map.spec.height() / CELL).ceil() as usize;
        let wet_at = |x: f32, y: f32| {
            matches!(
                self.map.terrain_at(x, y),
                Terrain::Water | Terrain::River
            )
        };
        let mooring = |x: f32, y: f32| dist(x, y, ax, ay) < 32.0 || dist(x, y, bx, by) < 32.0;
        let mut pass = vec![false; cols * rows];
        for j in 0..rows {
            for i in 0..cols {
                let cx = (i as f32 + 0.5) * CELL;
                let cy = (j as f32 + 0.5) * CELL;
                let q = CELL * 0.25;
                pass[j * cols + i] = mooring(cx, cy)
                    || [(cx, cy), (cx - q, cy - q), (cx + q, cy + q), (cx - q, cy + q), (cx + q, cy - q)]
                        .into_iter()
                        .all(|(x, y)| wet_at(x, y));
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
        pass[start] = true;
        g[start] = 0;
        heap.push((std::cmp::Reverse(h(si, sj)), start));
        let goal = gj * cols + gi;
        pass[goal] = true;
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
            return None;
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
        // Smooth with a wet line-of-sight pass (mooring zones stay open).
        let wet_shot = |ax: f32, ay: f32, bx: f32, by: f32| {
            let d = dist(ax, ay, bx, by);
            let n = (d / 16.0).ceil().max(1.0) as usize;
            (1..n).all(|k| {
                let t = k as f32 / n as f32;
                let (x, y) = (ax + (bx - ax) * t, ay + (by - ay) * t);
                wet_at(x, y) || mooring(x, y)
            })
        };
        let mut path = vec![raw[0]];
        let mut cursor = 0;
        while cursor < raw.len() - 1 {
            let mut far = cursor + 1;
            for k in (cursor + 2)..raw.len() {
                if wet_shot(raw[cursor].0, raw[cursor].1, raw[k].0, raw[k].1) {
                    far = k;
                }
            }
            path.push(raw[far]);
            cursor = far;
        }
        Some(path)
    }
    /// Water-only mirror of `steer` for ferries: the direct step wins
    /// when it stays on water or river, otherwise the smallest rotation
    /// that does. Ships may nose onto the dock shore to board.
    fn sail_steer(
        &self,
        x: f32,
        y: f32,
        tx: f32,
        ty: f32,
        step: f32,
        side: bool,
        home: (f32, f32),
    ) -> Option<(f32, f32)> {
        let d = dist(x, y, tx, ty);
        let (ux, uy) = ((tx - x) / d, (ty - y) / d);
        let wet = |nx: f32, ny: f32| {
            self.map.spec.size == MapSize::Classic
                || matches!(
                    self.map.terrain_at(nx, ny),
                    Terrain::Water | Terrain::River
                )
                // May nose up to either dock's shore to moor or push off.
                || dist(nx, ny, tx, ty) < 18.0
                || dist(nx, ny, home.0, home.1) < 24.0
        };
        if wet(x + ux * step, y + uy * step) {
            return Some((x + ux * step, y + uy * step));
        }
        for deg in [15.0f32, 30.0, 45.0, 60.0, 75.0, 90.0, 120.0, 150.0, 165.0] {
            for s in if side { [1.0f32, -1.0] } else { [-1.0f32, 1.0] } {
                let a = (s * deg).to_radians();
                let (sn, cs) = a.sin_cos();
                let (rx, ry) = (ux * cs - uy * sn, ux * sn + uy * cs);
                let (nx, ny) = (x + rx * step, y + ry * step);
                if wet(nx, ny) {
                    return Some((nx, ny));
                }
            }
        }
        None
    }
    /// A stranded agent joins the nearest port dock that can ferry it
    /// toward its blocked target: only when no dry route to the target
    /// exists but the partner shore can reach it. Returns true when the
    /// agent joined a queue.
    fn try_board(&mut self, i: usize, tx: f32, ty: f32) -> bool {
        let (x, y) = (self.agents.x[i], self.agents.y[i]);
        for pi in 0..self.water.ports.len() {
            if !self.water.ports[pi].near_done {
                continue;
            }
            for side in 0..2 {
                let (dock, partner) = {
                    let p = &self.water.ports[pi];
                    if side == 0 {
                        ((p.x, p.y), (p.lx, p.ly))
                    } else {
                        ((p.lx, p.ly), (p.x, p.y))
                    }
                };
                if dist(x, y, dock.0, dock.1) >= 192.0 {
                    continue;
                }
                let aboard_ship = self
                    .water
                    .ships
                    .iter()
                    .any(|s| s.aboard.contains(&(i as u32)));
                let queued = {
                    let p = &self.water.ports[pi];
                    p.queue.contains(&(i as u32)) || p.queue_far.contains(&(i as u32))
                };
                if aboard_ship || queued {
                    continue;
                }
                if self.route_between(x, y, tx, ty).is_some()
                    || self.route_between(partner.0, partner.1, tx, ty).is_none()
                {
                    continue;
                }
                let q = if side == 0 {
                    &mut self.water.ports[pi].queue
                } else {
                    &mut self.water.ports[pi].queue_far
                };
                if q.len() < 100 {
                    q.push(i as u32);
                    return true;
                }
            }
        }
        false
    }
    /// Ferries board queues, cross when enough passengers wait, pin
    /// riders to the hull like convoy members, and disembark them at
    /// the far dock where their own targets resume.
    pub(super) fn ship_step(&mut self) {
        if self.water.ships.is_empty() {
            return;
        }
        for si in 0..self.water.ships.len() {
            let pi = self.water.ships[si].port as usize;
            let (at_far, departed) = {
                let s = &self.water.ships[si];
                (s.at_far, s.departed)
            };
            let p = match self.water.ports.get(pi) {
                Some(p) => p.clone(),
                None => continue,
            };
            let (dock, other) = if at_far {
                ((p.lx, p.ly), (p.x, p.y))
            } else {
                ((p.x, p.y), (p.lx, p.ly))
            };
            if !departed {
                // Board waiting passengers standing near the dock.
                for qi in (0..self.port_queue_mut(pi, at_far).len()).rev() {
                    if self.water.ships[si].aboard.len() >= 100 {
                        break;
                    }
                    let id = self.port_queue_mut(pi, at_far)[qi];
                    let i = id as usize;
                    if !self.agents.alive[i]
                        || self.agents.health[i] < 25.0
                        || self.agents.hunger[i] > 70.0
                    {
                        self.port_queue_mut(pi, at_far).remove(qi);
                        continue;
                    }
                    // The dock sits inside the same dedup cell the block
                    // was reported from, so a strand pocket counts as
                    // waiting at the pier.
                    if dist(self.agents.x[i], self.agents.y[i], dock.0, dock.1) < 96.0 {
                        self.port_queue_mut(pi, at_far).remove(qi);
                        self.water.ships[si].aboard.push(id);
                    }
                }
                let aboard: Vec<u32> = self.water.ships[si]
                    .aboard
                    .iter()
                    .copied()
                    .filter(|&id| self.agents.alive[id as usize])
                    .collect();
                self.water.ships[si].aboard = aboard;
                let aboard = self.water.ships[si].aboard.len();
                let waited = self
                    .tick
                    .saturating_sub(self.water.ships[si].docked_since);
                if aboard >= 8 || (aboard >= 1 && waited > 2400) {
                    self.water.ships[si].departed = true;
                }
                // Riders hold position on the moored hull.
                self.pin_riders(si);
                continue;
            }
            let (sx, sy) = (self.water.ships[si].x, self.water.ships[si].y);
            let d = dist(sx, sy, other.0, other.1);
            if self.water.ships[si].path.is_empty() {
                // Route the crossing once; a lake with no wet path at all
                // falls back to the direct line and stalls at the shore.
                self.water.ships[si].path = self
                    .route_over_water(sx, sy, other.0, other.1)
                    .unwrap_or_else(|| vec![(sx, sy), (other.0, other.1)]);
                self.water.ships[si].leg = 1;
            }
            if d < 14.0 {
                // Disembark everyone at the new dock; each resumes its own
                // action with the far shore now underfoot.
                let riders = std::mem::take(&mut self.water.ships[si].aboard);
                for (rank, id) in riders.iter().enumerate() {
                    let i = *id as usize;
                    if !self.agents.alive[i] {
                        continue;
                    }
                    self.agents.x[i] = other.0 + (rank % 7) as f32 * 3.0 - 9.0;
                    self.agents.y[i] = other.1 + (rank / 7) as f32 * 3.0 - 10.5;
                }
                let s = &mut self.water.ships[si];
                s.at_far = !s.at_far;
                s.departed = false;
                s.docked_since = self.tick;
                s.path = Vec::new();
                s.leg = 0;
                let span = dist(dock.0, dock.1, other.0, other.1).max(1.0);
                s.x = other.0 + (other.0 - dock.0) / span * 10.0;
                s.y = other.1 + (other.1 - dock.1) / span * 10.0;
                continue;
            }
            // Follow the routed leg; each waypoint hands off to the next.
            let (wx, wy) = {
                let s = &self.water.ships[si];
                s.path[(s.leg as usize).min(s.path.len() - 1)]
            };
            if dist(sx, sy, wx, wy) < 8.0
                && (self.water.ships[si].leg as usize) < self.water.ships[si].path.len() - 1
            {
                self.water.ships[si].leg += 1;
            }
            let step = (5.0 * DT).min(d);
            if let Some((nx, ny)) =
                self.sail_steer(sx, sy, wx, wy, step, si % 2 == 0, dock)
            {
                self.water.ships[si].x = nx;
                self.water.ships[si].y = ny;
            }
            self.pin_riders(si);
        }
    }
    fn port_queue_mut(&mut self, port: usize, far: bool) -> &mut Vec<u32> {
        if far {
            &mut self.water.ports[port].queue_far
        } else {
            &mut self.water.ports[port].queue
        }
    }
    /// Riders ride the hull in convoy-style rank offsets.
    fn pin_riders(&mut self, si: usize) {
        let (sx, sy) = (self.water.ships[si].x, self.water.ships[si].y);
        for (rank, id) in self.water.ships[si].aboard.iter().enumerate() {
            let i = *id as usize;
            if !self.agents.alive[i] {
                continue;
            }
            self.agents.x[i] = sx + (rank % 7) as f32 * 3.0 - 9.0;
            self.agents.y[i] = sy + (rank / 7) as f32 * 3.0 - 10.5;
        }
    }
    /// Steer `step` units from (x, y) toward (tx, ty). When the direct step
    /// lands in water, try progressively larger rotations so the mover
    /// follows the shoreline around the obstacle. Each mover prefers one
    /// side by id parity, which stops the left/right flip-flopping that
    /// strands movers in shoreline pockets; angles past 90° let a mover
    /// back out of a cove instead of waiting forever.
    const DETOUR: &[f32] = &[15.0, 30.0, 45.0, 60.0, 75.0, 90.0, 115.0, 140.0, 165.0];
    fn steer(
        &mut self,
        x: f32,
        y: f32,
        tx: f32,
        ty: f32,
        step: f32,
        prefer_ccw: bool,
    ) -> Option<(f32, f32)> {
        // A target that is itself underwater can never be arrived at; walk
        // to the dry ground closest to it instead.
        let (tx, ty) = if self.passable(tx, ty) {
            (tx, ty)
        } else {
            self.map.dry_ground(tx, ty)
        };
        let d = dist(x, y, tx, ty);
        if d <= 0.0 || step <= 0.0 {
            return None;
        }
        let ux = (tx - x) / d;
        let uy = (ty - y) / d;
        if self.passable(x + ux * step, y + uy * step) {
            return Some((x + ux * step, y + uy * step));
        }
        let sides: [f32; 2] = if prefer_ccw { [1.0, -1.0] } else { [-1.0, 1.0] };
        for &deg in Self::DETOUR {
            let (sin, cos) = deg.to_radians().sin_cos();
            for &s in sides.iter() {
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
        if let Some((nx, ny)) = self.steer(x, y, tx, ty, step, i % 2 == 0) {
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
        if self.map.spec.size != MapSize::Classic {
            self.track_stranded(i, tx, ty);
        }
    }
    /// An agent still en route but unable to make 24 units of progress
    /// for ~600 ticks is stranded at water and starts carrying a report
    /// home (delivered by `update_water_reports` on arrival).
    fn track_stranded(&mut self, i: usize, tx: f32, ty: f32) {
        if !self.agents.alive[i] || self.water.pending.len() >= 256 {
            return;
        }
        if self.water.progress.len() <= i {
            self.water.progress.resize(i + 1, None);
        }
        let (x, y) = (self.agents.x[i], self.agents.y[i]);
        match self.water.progress[i] {
            Some((px, py, since)) if dist(x, y, px, py) < 24.0 => {
                if self.tick.saturating_sub(since) > 600
                    && !self.water.pending.iter().any(|p| p.agent == i as u32)
                {
                    self.water.pending.push(PendingBlock {
                        agent: i as u32,
                        x,
                        y,
                        tx,
                        ty,
                        tick: self.tick,
                    });
                    // A working ferry nearby beats walking the report home:
                    // queue at the dock and let the ship do the crossing.
                    self.try_board(i, tx, ty);
                }
            }
            _ => self.water.progress[i] = Some((x, y, self.tick)),
        }
    }
    /// Deliver carried reports once their bearer reaches home ground.
    pub(super) fn update_water_reports(&mut self) {
        if self.map.spec.size == MapSize::Classic || self.water.pending.is_empty() {
            return;
        }
        let mut arrived: Vec<PendingBlock> = Vec::new();
        self.water.pending.retain(|p| {
            let i = p.agent as usize;
            let home = self.agents.home[i] as usize;
            let keep = self.agents.alive[i]
                && home < self.markets.len()
                && {
                    let m = &self.markets[home];
                    let r = rest::settlement_radius(m.housing_blocks).max(160.0);
                    dist(self.agents.x[i], self.agents.y[i], m.x, m.y) >= r
                };
            if !keep {
                arrived.push(*p);
            }
            keep
        });
        for p in arrived {
            if !self.agents.alive[p.agent as usize] {
                continue;
            }
            let market = self.agents.home[p.agent as usize];
            let cell = ((p.x / 96.0) as i32, (p.y / 96.0) as i32);
            if let Some(b) = self.water.blocks.iter_mut().find(|b| {
                b.market == market as u32
                    && ((b.x / 96.0) as i32, (b.y / 96.0) as i32) == cell
            }) {
                b.reports = b.reports.saturating_add(1);
                b.tick = p.tick;
            } else if self.water.blocks.len() < 64 {
                self.water.blocks.push(WaterBlock {
                    market: market as u32,
                    x: p.x,
                    y: p.y,
                    tx: p.tx,
                    ty: p.ty,
                    tick: p.tick,
                    reports: 1,
                });
            }
        }
    }
    /// Turn delivered water blocks into paired port construction. A zoned
    /// village with the coffers and materials surveys a landing across the
    /// water, then posts Shipwright jobs on the urban board: the home shore
    /// always, the far shore only when a dry land route exists — otherwise
    /// the port is flagged `ferry` and waits for a ship to carry builders.
    pub(super) fn plan_ports(&mut self) {
        if self.map.spec.size == MapSize::Classic
            || (self.water.blocks.is_empty() && self.water.ports.is_empty())
        {
            return;
        }
        const WAGE: u64 = 48;
        const WOOD: u64 = 20;
        const STONE: u64 = 4;
        // Repost sides whose job was refunded or abandoned — the escrowed
        // materials stay committed to the port, only the wage is re-paid.
        for pi in 0..self.water.ports.len() {
            let p = self.water.ports[pi].clone();
            let m = p.market as usize;
            if m >= self.markets.len() {
                continue;
            }
            for side in 0..2u32 {
                let unfinished = if side == 0 { !p.near_done } else { !p.far_done };
                if !unfinished || (side == 1 && p.ferry) {
                    continue;
                }
                let posted = self.urban.jobs.iter().flatten().any(|j| {
                    j.kind == 4 && j.target == pi as u32 && j.source == side
                });
                if !posted && self.markets[m].coins >= WAGE {
                    self.markets[m].coins -= WAGE;
                    let (x, y) = if side == 0 { (p.x, p.y) } else { (p.lx, p.ly) };
                    let j = self.new_job(m, 4, NONE, NONE, side, pi as u32, x, y, WAGE);
                    self.post_urban_job(j);
                }
            }
            // A finished home dock on a ferry port commissions its ship:
            // forty timber and a shipwright crew, posted like a build job.
            if p.near_done
                && p.ferry
                && !self.water.ships.iter().any(|s| s.port == pi as u32)
                && !self.urban.jobs.iter().flatten().any(|j| {
                    j.kind == 4 && j.target == pi as u32 && j.source == 2
                })
                && self.markets[m].stock(1) >= 40
                && self.markets[m].coins >= WAGE
            {
                let mut left = 40;
                while left > 0 {
                    left -= self.markets[m]
                        .take(1, left)
                        .expect("reserved ship timber")
                        .quantity;
                }
                self.ledger.built[1] += 40;
                self.markets[m].coins -= WAGE;
                let j = self.new_job(m, 4, NONE, NONE, 2, pi as u32, p.x, p.y, WAGE);
                self.post_urban_job(j);
            }
        }
        let mut done: Vec<usize> = Vec::new();
        for bi in 0..self.water.blocks.len() {
            if self.water.ports.len() >= 32 {
                break;
            }
            let b = self.water.blocks[bi];
            let m = b.market as usize;
            if m >= self.markets.len()
                || !self.urban.states.get(m).map(|s| s.zoned).unwrap_or(false)
                || self.coffers(m) < 2 * WAGE
                || self.markets[m].stock(1) < WOOD
                || self.markets[m].stock(2) < STONE
            {
                continue;
            }
            if self.water.ports.iter().any(|p| {
                dist(p.x, p.y, b.x, b.y) < 96.0 || dist(p.lx, p.ly, b.x, b.y) < 96.0
            }) {
                done.push(bi);
                continue;
            }
            // March from the stranded position toward the blocked target;
            // the first buildable ground past water is the landing site.
            let dd = dist(b.x, b.y, b.tx, b.ty).max(1.0);
            let (ux, uy) = ((b.tx - b.x) / dd, (b.ty - b.y) / dd);
            let (mut lx, mut ly, mut landed, mut crossed) = (b.x, b.y, false, false);
            for k in 1..=128 {
                let t = k as f32 * 32.0;
                let (px, py) = (b.x + ux * t, b.y + uy * t);
                let here = self.map.terrain_at(px, py);
                if here == Terrain::Water || here == Terrain::River {
                    crossed = true;
                } else if crossed && self.map.buildable(px, py) {
                    (lx, ly, landed) = (px, py, true);
                    break;
                }
            }
            if !landed {
                // Open water all the way: an island nothing can serve.
                done.push(bi);
                continue;
            }
            let (px, py) = self.map.dry_ground(b.x, b.y);
            (lx, ly) = self.map.dry_ground(lx, ly);
            let id = self.water.ports.len() as u32;
            let land_link = self.route_between(px, py, lx, ly).is_some();
            // A ferry only makes sense when water actually carries a ship
            // across; a landlocked dead-end gets no port at all.
            let sea_link = !land_link && self.route_over_water(px, py, lx, ly).is_some();
            if !land_link && !sea_link {
                done.push(bi);
                continue;
            }
            self.water.ports.push(Port {
                market: b.market,
                x: px,
                y: py,
                lx,
                ly,
                near_done: false,
                far_done: false,
                ferry: !land_link,
                queue: Vec::new(),
                queue_far: Vec::new(),
            });
            for (k, q) in [(1usize, WOOD), (2, STONE)] {
                let mut left = q;
                while left > 0 {
                    left -= self.markets[m]
                        .take(k, left)
                        .expect("reserved port materials")
                        .quantity;
                }
                self.ledger.built[k] += q;
            }
            self.markets[m].coins -= WAGE;
            let j = self.new_job(m, 4, NONE, NONE, 0, id, px, py, WAGE);
            self.post_urban_job(j);
            if land_link {
                self.markets[m].coins -= WAGE;
                let j = self.new_job(m, 4, NONE, NONE, 1, id, lx, ly, WAGE);
                self.post_urban_job(j);
            }
            done.push(bi);
        }
        for bi in done.into_iter().rev() {
            self.water.blocks.remove(bi);
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
                if let Some((nx, ny)) = self.steer(c.x, c.y, c.goal_x, c.goal_y, step, slot % 2 == 0)
                {
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
            let (x, y) = self.map.dry_ground(x, y);
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
