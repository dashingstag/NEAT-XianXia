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

/// A routed land detour a stalled mover follows around water. `path`
/// with one point records a completed route lookup and its destination.
/// An empty path is a legacy lookup without a recorded destination.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Detour {
    pub path: Vec<(f32, f32)>,
    /// Arclength progress along `path`: monotonic so a mover pushed off
    /// the route keeps aiming forward instead of flipping between the
    /// projections of adjacent segments.
    pub progress: f32,
    pub attempt: u64,
}

impl Detour {
    fn has_route(&self) -> bool {
        self.path.len() >= 2
    }

    fn matches_destination(&self, x: f32, y: f32) -> bool {
        self.path.last().is_some_and(|&(tx, ty)| dist(x, y, tx, ty) <= 48.0)
    }
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
    /// Routed detours for stalled movers: agent idx, or a convoy slot
    /// with the high bit set.
    pub detours: BTreeMap<u32, Detour>,
    // Per-tick search budget. The next simulation tick resets it after a load.
    #[serde(skip)]
    retry_budget_tick: u64,
    #[serde(skip)]
    fast_retries: u8,
    /// Convoy slot -> stall probe, same 24-unit rule as `progress`.
    pub convoy_probe: BTreeMap<u32, (f32, f32, u64)>,
    /// (action, target) pairs a mover proved water-blocked, so a starving
    /// walker stops re-selecting the crossing it cannot finish and picks
    /// reachable food instead. Fed walkers keep pressing the shore —
    /// their reports feed the port chain and ferries need the demand.
    /// Entries decay after ~a day; soft state, rebuilt on load.
    #[serde(skip)]
    pub unreachable: BTreeMap<u32, BTreeMap<(u8, u32), u64>>,
    /// (action, target) -> positions from which movers proved it water-
    /// blocked. Shared shore knowledge: one strander's failed route marks
    /// the crossing for every mover stalled on the same bank, so a
    /// starving crowd does not each pay the ~700-tick stall cost to
    /// re-learn it. Entries decay after ~a day; soft state.
    #[serde(skip)]
    pub shared_unreachable: BTreeMap<(u8, u32), Vec<(f32, f32, u64)>>,
}

/// Daytime traffic census. Mover-observations accumulate per unordered
/// market pair during the day; at the day rollover they are published to
/// `Road.traffic` (yesterday's movers) and hot corridors without a road
/// record get one so `start_projects` can put them up for construction.
/// Persisted as a checkpoint tail — a mid-day save keeps its samples.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct TrafficState {
    /// (min market id, max market id) -> mover-observations today.
    pub counts: BTreeMap<(u32, u32), u64>,
}

/// Pure-pursuit lookahead over a corridor polyline: project (x, y) onto
/// the nearest segment, then return the point `ahead` units of arclength
/// further along. `rev` traverses the polyline from its last point to
/// its first without copying it. The far endpoint is returned once the
/// lookahead runs off the corridor.
fn path_lookahead(
    path: &[(f32, f32)],
    rev: bool,
    x: f32,
    y: f32,
    ahead: f32,
) -> (f32, f32) {
    let n = path.len();
    let at = |i: usize| path[if rev { n - 1 - i } else { i }];
    let (mut best, mut base, mut progress) = (f32::MAX, 0f32, 0f32);
    for s in 0..n - 1 {
        let (ax, ay) = at(s);
        let (bx, by) = at(s + 1);
        let (dx, dy) = (bx - ax, by - ay);
        let l2 = dx * dx + dy * dy;
        let tt = if l2 > 0.0 {
            (((x - ax) * dx + (y - ay) * dy) / l2).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let (px, py) = (ax + dx * tt, ay + dy * tt);
        let d2 = (x - px) * (x - px) + (y - py) * (y - py);
        if d2 < best {
            best = d2;
            progress = base + l2.sqrt() * tt;
        }
        base += l2.sqrt();
    }
    let mut rem = progress + ahead;
    for s in 0..n - 1 {
        let (ax, ay) = at(s);
        let (bx, by) = at(s + 1);
        let len = dist(ax, ay, bx, by);
        if rem <= len && len > 0.0 {
            let tt = rem / len;
            return (ax + (bx - ax) * tt, ay + (by - ay) * tt);
        }
        rem -= len;
    }
    at(n - 1)
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
    fn stalled_walkers_route_around_lakes() {
        let mut w = World::new_with_size(40, 7, Hill::default(), MapSize::X4).unwrap();
        let r = w.map.raster();
        // Find a land→water→land stripe along a row, probing terrain_at
        // directly — raster cells classify whole cells and can hide land
        // bumps inside a "water" cell that the walker's point samples hit.
        // The band must be a continuous ≥32u of water along the line and
        // the A* must return real waypoints around it.
        let mut span = None;
        let limit = (r.cols - 4) as f32 * r.cell;
        'outer: for row in 4..r.rows - 4 {
            let y = (row as f32 + 0.5) * r.cell;
            let mut x = 4.0 * r.cell;
            while x < limit {
                if w.map.terrain_at(x, y) != Terrain::Water {
                    x += 4.0;
                    continue;
                }
                let start = x;
                while x < limit && w.map.terrain_at(x, y) == Terrain::Water {
                    x += 4.0;
                }
                if x - start >= 32.0 && x < limit {
                    let (ax, bx) = (start - 4.0, x);
                    if matches!(
                        w.route_between(ax, y, bx, y),
                        Some(p) if p.len() >= 3
                    ) {
                        span = Some((ax, y, bx, y));
                        break 'outer;
                    }
                }
            }
        }
        let Some((x, y, tx, ty)) = span else { return };
        w.agents.x[0] = x;
        w.agents.y[0] = y;
        // Pin against the shore, then jump the stall probe past the
        // 240-tick detour threshold.
        for _ in 0..24 {
            w.walk_toward(0, tx, ty, 4.0);
        }
        w.tick += 241;
        for _ in 0..8 {
            w.walk_toward(0, tx, ty, 4.0);
        }
        assert!(
            matches!(w.water.detours.get(&0), Some(d) if d.has_route()),
            "stalled walker should have been routed around the lake"
        );
        // Following the detour must actually reach the far shore.
        let mut reached = false;
        for _ in 0..6000 {
            w.tick += 1;
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
        assert!(
            reached,
            "routed walker never reached ({tx},{ty}) from ({x},{y}); ended at ({},{})",
            w.agents.x[0],
            w.agents.y[0]
        );
    }
    #[test]
    fn detour_failure_backoff_follows_the_destination_through_save() {
        let mut w = World::new_with_size(8, 42, Hill::default(), MapSize::X4).unwrap();
        let (x, y) = (w.agents.x[0], w.agents.y[0]);
        let r = w.map.raster();
        let wet = r.terrain.iter().enumerate()
            .find(|&(at, &terrain)| {
                let wx = (at % r.cols) as f32 * r.cell + r.cell * 0.5;
                let wy = (at / r.cols) as f32 * r.cell + r.cell * 0.5;
                terrain == Terrain::Water as u8 && dist(x, y, wx, wy) > 100.0
            })
            .unwrap().0;
        let wx = (wet % r.cols) as f32 * r.cell + r.cell * 0.5;
        let wy = (wet / r.cols) as f32 * r.cell + r.cell * 0.5;
        w.agents.hunger[0] = 60.0;
        w.agents.action[0] = Action::ForageMeal;
        w.agents.target[0] = 0;
        w.tick = 500;
        w.compute_detour(0, x, y, wx, wy);
        assert!(w.target_unreachable(0, Action::ForageMeal, 0));
        assert!(!w.water.detours[&0].has_route());

        let path = std::env::temp_dir().join(format!(
            "xianxia-detour-destination-{}.snapshot", std::process::id()
        ));
        w.save(&path).unwrap();
        let mut w = World::load(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        w.tick = 580;
        w.compute_detour(0, x, y, wx, wy);
        assert_eq!(w.water.detours[&0].attempt, 500);

        w.agents.target[0] = 1;
        w.compute_detour(0, x, y, x, y);
        assert_eq!(w.water.detours[&0].attempt, 580);
        assert!(!w.target_unreachable(0, Action::ForageMeal, 1));
    }

    #[test]
    fn full_report_queue_still_detours_stranders() {
        // Regression: the 256-entry cap on carried reports used to gate
        // the whole probe, so once the queue saturated no walker ever
        // stalled, filed, or received a detour again.
        let mut w = World::new_with_size(40, 7, Hill::default(), MapSize::X4).unwrap();
        for n in 0..256u32 {
            w.water.pending.push(PendingBlock {
                agent: 1000 + n,
                x: 0.0,
                y: 0.0,
                tx: 0.0,
                ty: 0.0,
                tick: w.tick,
            });
        }
        let r = w.map.raster();
        let mut span = None;
        let limit = (r.cols - 4) as f32 * r.cell;
        'outer: for row in 4..r.rows - 4 {
            let y = (row as f32 + 0.5) * r.cell;
            let mut x = 4.0 * r.cell;
            while x < limit {
                if w.map.terrain_at(x, y) != Terrain::Water {
                    x += 4.0;
                    continue;
                }
                let start = x;
                while x < limit && w.map.terrain_at(x, y) == Terrain::Water {
                    x += 4.0;
                }
                if x - start >= 32.0 && x < limit {
                    let (ax, bx) = (start - 4.0, x);
                    if matches!(w.route_between(ax, y, bx, y), Some(p) if p.len() >= 3) {
                        span = Some((ax, y, bx, y));
                        break 'outer;
                    }
                }
            }
        }
        let Some((x, y, tx, ty)) = span else { return };
        w.agents.x[0] = x;
        w.agents.y[0] = y;
        for _ in 0..24 {
            w.walk_toward(0, tx, ty, 4.0);
        }
        w.tick += 241;
        for _ in 0..8 {
            w.walk_toward(0, tx, ty, 4.0);
        }
        assert!(
            matches!(w.water.detours.get(&0), Some(d) if d.has_route()),
            "a saturated report queue must not disable stall probes or detours"
        );
    }
    #[test]
    fn stale_carried_reports_expire() {
        let mut w = World::new_with_size(40, 7, Hill::default(), MapSize::X4).unwrap();
        // Agent 0 stands far from home carrying a report it can never
        // deliver (it is stranded itself): the entry must not hold a
        // report-queue slot forever.
        let home = w.agents.home[0] as usize;
        w.agents.x[0] = w.markets[home].x + 2000.0;
        w.agents.y[0] = w.markets[home].y;
        w.water.pending.push(PendingBlock {
            agent: 0,
            x: w.agents.x[0],
            y: w.agents.y[0],
            tx: 1.0,
            ty: 1.0,
            tick: w.tick,
        });
        w.tick += 4801;
        w.update_water_reports();
        assert!(w.water.pending.is_empty(), "stale report should expire");
        assert!(w.water.blocks.is_empty(), "expired reports do not deliver");
        // A fresh report from the same spot is still carried.
        w.water.pending.push(PendingBlock {
            agent: 0,
            x: w.agents.x[0],
            y: w.agents.y[0],
            tx: 1.0,
            ty: 1.0,
            tick: w.tick,
        });
        w.update_water_reports();
        assert_eq!(w.water.pending.len(), 1);
    }
    #[test]
    fn report_carriers_walk_home_to_deliver() {
        // A stranded walker carrying a report must physically bring it
        // home: without the redirect it kept pinning against the same
        // unreachable shore, so the report queue filled with entries that
        // could never land and the village never learned of the block.
        let mut w = World::new_with_size(40, 7, Hill::default(), MapSize::X4).unwrap();
        let home = w.agents.home[0] as usize;
        let r = rest::settlement_radius(w.markets[home].housing_blocks).max(160.0);
        // Just outside home ground, aiming at a patch far in the distance.
        w.agents.x[0] = w.markets[home].x + r + 40.0;
        w.agents.y[0] = w.markets[home].y;
        w.agents.action[0] = Action::ForageMeal;
        w.agents.target[0] = 0;
        w.water.pending.push(PendingBlock {
            agent: 0,
            x: w.agents.x[0],
            y: w.agents.y[0],
            tx: w.markets[home].x + 900.0,
            ty: w.markets[home].y,
            tick: w.tick,
        });
        // The report overrides the chosen destination until it is filed.
        let (dx, dy) = w.personal_destination(0).unwrap();
        assert_eq!(
            (dx, dy),
            (w.markets[home].x, w.markets[home].y),
            "a carrier's destination is home, not the blocked target"
        );
        for _ in 0..200 {
            w.execute_personal(0, Action::ForageMeal);
            w.update_water_reports();
            if w.water.pending.is_empty() {
                break;
            }
        }
        assert!(w.water.pending.is_empty(), "carrier delivered the report");
        assert_eq!(w.water.blocks.len(), 1, "the village recorded the block");
    }
    #[test]
    fn walkers_standing_in_water_wade_to_dry_ground() {
        // Legacy saves can hold agents born in water (635/10k on X4 seed
        // 42): too deep for any steering step, so they froze in place.
        // They now wade toward the nearest dry point.
        let mut w = World::new_with_size(40, 7, Hill::default(), MapSize::X4).unwrap();
        let r = w.map.raster();
        let mut deep = None;
        'outer: for row in 4..r.rows - 4 {
            let y = (row as f32 + 0.5) * r.cell;
            let mut x = 4.0 * r.cell;
            let limit = (r.cols - 4) as f32 * r.cell;
            while x < limit {
                if w.map.terrain_at(x, y) == Terrain::Water
                    && [(8.0, 0.0), (-8.0, 0.0), (0.0, 8.0), (0.0, -8.0)]
                        .iter()
                        .all(|&(ox, oy)| {
                            w.map.terrain_at(x + ox, y + oy) == Terrain::Water
                        })
                {
                    deep = Some((x, y));
                    break 'outer;
                }
                x += 4.0;
            }
        }
        let Some((wx, wy)) = deep else { return };
        w.agents.x[0] = wx;
        w.agents.y[0] = wy;
        let (dx, dy) = w.map.dry_ground(wx, wy);
        for _ in 0..2000 {
            w.tick += 1;
            w.walk_toward(0, dx, dy, 4.0);
            if w.map.terrain_at(w.agents.x[0], w.agents.y[0]) != Terrain::Water {
                break;
            }
        }
        assert!(
            w.map.terrain_at(w.agents.x[0], w.agents.y[0]) != Terrain::Water,
            "deep-water walker never reached dry ground"
        );
    }
    #[test]
    fn daytime_traffic_samples_promote_road_records() {
        // Movers bound for a foreign market credit their corridor on each
        // census; a pair over the promotion bar gains a Road record so
        // start_projects can put it up for construction.
        let mut w = World::new(3, 42).unwrap();
        w.roads.clear();
        for i in 0..3 {
            w.agents.action[i] = Action::Deliver;
            w.agents.target[i] = 1;
            w.agents.home[i] = 0;
            w.agents.x[i] = 400.0;
            w.agents.y[i] = 210.0;
            w.agents.cargo[i] = 5;
        }
        for _ in 0..3 {
            w.sample_traffic();
        }
        w.publish_traffic();
        let r = w
            .roads
            .iter()
            .find(|r| (r.a == 0 && r.b == 1) || (r.a == 1 && r.b == 0))
            .expect("a hot corridor promotes to a road record");
        assert_eq!(r.traffic, 9);
    }
    #[test]
    fn published_traffic_posts_a_road_project() {
        // 4 movers x 12 census ticks = 48, the busy-corridor bar; the
        // market answers with a Road project agents can work on.
        let mut w = World::new(4, 42).unwrap();
        for i in 0..4 {
            w.agents.action[i] = Action::Deliver;
            w.agents.target[i] = 1;
            w.agents.home[i] = 0;
            w.agents.x[i] = 400.0;
            w.agents.y[i] = 210.0;
            w.agents.cargo[i] = 5;
        }
        for _ in 0..12 {
            w.sample_traffic();
        }
        w.publish_traffic();
        w.markets[0].add(1, 12, 0);
        w.markets[0].add(2, 6, 0);
        w.start_projects();
        assert_eq!(w.markets[0].project.as_ref().unwrap().kind, "Road");
        assert!(w.roads.iter().any(|r| r.under_construction));
    }
    #[test]
    fn walkers_ride_the_built_road_corridor() {
        // A built road's corridor is not the straight market-to-market
        // line — it detours around water. A market-bound walker rides
        // the corridor instead of tracing a parallel line beside it.
        let mut w = World::new(3, 42).unwrap();
        let a = (w.markets[0].x, w.markets[0].y);
        let b = (w.markets[1].x, w.markets[1].y);
        w.roads[0].built = true;
        let corner = (400.0, 420.0);
        w.road_paths = vec![vec![a, corner, b]];
        w.agents.action[0] = Action::Deliver;
        w.agents.target[0] = 1;
        w.agents.home[0] = 0;
        w.agents.x[0] = a.0;
        w.agents.y[0] = a.1;
        for _ in 0..40 {
            w.tick += 1;
            w.walk_toward(0, b.0, b.1, 4.0);
        }
        assert!(
            segment_distance_points(w.agents.x[0], w.agents.y[0], a, b) > 40.0,
            "walker hugged the straight line instead of the corridor"
        );
        assert!(dist(w.agents.x[0], w.agents.y[0], corner.0, corner.1) < 200.0);
    }
    #[test]
    fn caravans_ride_the_built_road_corridor() {
        let mut w = World::new(3, 42).unwrap();
        let a = (w.markets[0].x, w.markets[0].y);
        let b = (w.markets[1].x, w.markets[1].y);
        w.roads[0].built = true;
        let corner = (400.0, 420.0);
        // road_paths must match roads.len() or travel_step's
        // sync_road_paths rebuilds the real corridors.
        w.road_paths = vec![vec![a, corner, b], vec![], vec![]];
        w.convoys = vec![Some(Convoy {
            members: vec![0, 1],
            x: a.0,
            y: a.1,
            goal_x: b.0,
            goal_y: b.1,
            created: 0,
            departed: true,
        })];
        for i in 0..2 {
            w.agents.home[i] = 0;
            w.agents.convoy[i] = 0;
            w.agents.x[i] = a.0;
            w.agents.y[i] = a.1;
        }
        for _ in 0..60 {
            w.tick += 1;
            w.travel_step();
        }
        let c = w.convoys[0].as_ref().unwrap();
        assert!(
            segment_distance_points(c.x, c.y, a, b) > 30.0,
            "caravan hugged the straight line instead of the corridor"
        );
        assert!(dist(c.x, c.y, corner.0, corner.1) < 220.0);
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
    fn shore_span(w: &mut World) -> Option<(f32, f32, f32)> {
        let r = w.map.raster();
        for j in 1..r.rows - 1 {
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
                    return Some((
                        (i as f32 + 0.5) * r.cell,
                        (j as f32 + 0.5) * r.cell,
                        (k as f32 + 0.5) * r.cell,
                    ));
                }
            }
        }
        None
    }
    #[test]
    fn unzoned_camp_plans_port_with_stone_jetty() {
        // A camp is exactly the settlement whose stranders need rescue: the
        // block must plan even without districts, falling back to a stone
        // jetty when the market cannot stock twenty timber.
        let mut w = World::new_with_size(40, 7, Hill::default(), MapSize::X4).unwrap();
        let Some((bx, by, tx)) = shore_span(&mut w) else { return };
        w.water.blocks.push(WaterBlock {
            market: 0,
            x: bx,
            y: by,
            tx,
            ty: by,
            tick: 0,
            reports: 1,
        });
        w.urban.states[0].zoned = false;
        w.markets[0].coins = 500;
        w.markets[0].add(1, 4, 0);
        w.markets[0].add(2, 24, 0);
        w.plan_ports();
        assert_eq!(
            w.water.ports.len(),
            1,
            "unzoned market with jetty materials should still plan"
        );
        assert!(w
            .urban
            .jobs
            .iter()
            .flatten()
            .any(|j| j.kind == 4 && j.source == 0));
    }
    #[test]
    fn report_files_at_the_market_the_carrier_reaches() {
        let mut w = World::new_with_size(40, 7, Hill::default(), MapSize::X4).unwrap();
        if w.markets.len() < 2 {
            return;
        }
        // Home is market 0 but the carrier is stranded on market 1's side:
        // reaching market 1's ground files the report there.
        w.agents.home[0] = 0;
        w.agents.x[0] = w.markets[1].x;
        w.agents.y[0] = w.markets[1].y;
        w.water.pending.push(PendingBlock {
            agent: 0,
            x: 10.0,
            y: 10.0,
            tx: 90.0,
            ty: 10.0,
            tick: w.tick,
        });
        w.update_water_reports();
        assert!(
            w.water.pending.is_empty(),
            "reaching any market delivers the report"
        );
        assert_eq!(
            w.water.blocks.len(),
            1,
            "delivered report becomes a water block"
        );
        assert_eq!(
            w.water.blocks[0].market, 1,
            "block is keyed to the reached market, not the distant home"
        );
    }
    #[test]
    fn pushed_off_route_mover_rejoins_the_detour() {
        let mut w = World::new_with_size(40, 7, Hill::default(), MapSize::X4).unwrap();
        if w.map.spec.size == MapSize::Classic {
            return;
        }
        // Detour (0,0)->(200,0) with stored progress 160; a mover pushed
        // back to (30,0) must re-project instead of steering at a lookahead
        // it can no longer reach.
        w.water.detours.insert(
            0,
            Detour {
                path: vec![(0.0, 0.0), (200.0, 0.0)],
                progress: 160.0,
                attempt: 0,
            },
        );
        let (gx, _) = w.detour_target(0, 30.0, 0.0, 200.0, 0.0);
        assert!(
            gx < 80.0,
            "lookahead should rejoin near the mover, got {gx}"
        );
        assert!(
            w.water.detours[&0].progress < 60.0,
            "stored progress regressed to the fresh projection"
        );
    }
    #[test]
    fn shore_mates_inherit_a_proven_unreachable_target() {
        // One strander's failed route marks the crossing for every hungry
        // mover stalled on the same bank — the crowd must not each pay the
        // ~700-tick stall cost to re-learn it.
        let mut w = World::new_with_size(40, 7, Hill::default(), MapSize::X4).unwrap();
        if w.map.spec.size == MapSize::Classic {
            return;
        }
        w.agents.hunger[0] = 90.0;
        w.agents.hunger[1] = 90.0;
        w.agents.x[0] = 100.0;
        w.agents.y[0] = 100.0;
        w.agents.x[1] = 400.0;
        w.agents.y[1] = 400.0;
        w.water
            .shared_unreachable
            .insert((Action::ForageMeal as u8, 77), vec![(104.0, 96.0, w.tick)]);
        assert!(w.target_unreachable(0, Action::ForageMeal, 77));
        assert!(
            !w.target_unreachable(1, Action::ForageMeal, 77),
            "a mover on a different shore is not taught by this bank"
        );
        w.agents.hunger[0] = 20.0;
        assert!(
            !w.target_unreachable(0, Action::ForageMeal, 77),
            "a fed mover keeps pressing — its reports feed the port chain"
        );
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
    fn failed_detour_blocks_the_unreachable_target() {
        // A walker whose destination has no dry route gets its current
        // (action, target) pair on the blocked list; hunger then stops
        // the decision layer from re-offering the same crossing.
        let mut w = World::new_with_size(40, 7, Hill::default(), MapSize::X4).unwrap();
        // Find a dry point and a water point with no land path between them.
        let mut pair = None;
        let r = w.map.raster();
        'outer: for row in 4..r.rows - 4 {
            for col in 4..r.cols - 4 {
                let (x, y) = ((col as f32 + 0.5) * r.cell, (row as f32 + 0.5) * r.cell);
                if w.map.terrain_at(x, y) == Terrain::Water
                    && w.route_between(100.0, 100.0, x, y).is_none()
                {
                    pair = Some((x, y));
                    break 'outer;
                }
            }
        }
        let Some((wx, wy)) = pair else { return };
        w.agents.x[0] = 100.0;
        w.agents.y[0] = 100.0;
        w.agents.action[0] = Action::ForageMeal;
        w.agents.target[0] = 77;
        w.compute_detour(0, 100.0, 100.0, wx, wy);
        assert!(w
            .water
            .unreachable
            .get(&0)
            .is_some_and(|b| b.contains_key(&(Action::ForageMeal as u8, 77))));
        // The block only bites when the mover is actually hungry.
        w.agents.hunger[0] = 80.0;
        assert!(w.target_unreachable(0, Action::ForageMeal, 77));
        assert!(!w.target_unreachable(0, Action::BuyFood, 77));
        assert!(!w.target_unreachable(0, Action::ForageMeal, 78));
        w.agents.hunger[0] = 10.0;
        assert!(!w.target_unreachable(0, Action::ForageMeal, 77));
        // Entries decay after ~a day so a retry is still possible.
        w.agents.hunger[0] = 80.0;
        w.tick += 2401;
        assert!(!w.target_unreachable(0, Action::ForageMeal, 77));
    }
    #[test]
    fn starving_riders_step_off_a_docked_ship() {
        // Riders pinned to a moored hull cannot reach food: the starving
        // must disembark at the dock instead of dying aboard while the
        // ferry waits for passengers.
        let mut w = World::new_with_size(40, 7, Hill::default(), MapSize::X4).unwrap();
        w.water.ports.push(Port {
            market: 0,
            x: 200.0,
            y: 200.0,
            lx: 800.0,
            ly: 200.0,
            near_done: true,
            far_done: true,
            ferry: true,
            queue: Vec::new(),
            queue_far: Vec::new(),
        });
        w.water.ships.push(Ship {
            port: 0,
            x: 200.0,
            y: 200.0,
            at_far: false,
            aboard: vec![0, 1],
            departed: false,
            docked_since: w.tick,
            path: Vec::new(),
            leg: 0,
        });
        w.agents.hunger[0] = 80.0;
        w.agents.hunger[1] = 20.0;
        w.ship_step();
        assert_eq!(w.water.ships[0].aboard, vec![1]);
        assert!(
            dist(w.agents.x[0], w.agents.y[0], 200.0, 200.0) < 30.0,
            "released rider should stand on the dock"
        );
    }
    #[test]
    fn shipless_port_queues_release_the_starving() {
        // With no ship commissioned nothing drains a queue — starving
        // waiters must be released so they can walk to food.
        let mut w = World::new_with_size(40, 7, Hill::default(), MapSize::X4).unwrap();
        w.water.ports.push(Port {
            market: 0,
            x: 200.0,
            y: 200.0,
            lx: 800.0,
            ly: 200.0,
            near_done: true,
            far_done: true,
            ferry: true,
            queue: vec![0, 1],
            queue_far: vec![2],
        });
        w.agents.hunger[0] = 80.0;
        w.agents.hunger[1] = 20.0;
        w.agents.health[2] = 10.0;
        w.update_water_reports();
        assert_eq!(w.water.ports[0].queue, vec![1]);
        assert!(w.water.ports[0].queue_far.is_empty());
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
            // Mimic the plan_ports survey: march toward the far shore and
            // qualify only when the dry-ground pair is land-blocked for
            // walkers yet still sailable for a hull.
            let dd = dist(bx, by, lx, by).max(1.0);
            let ux = (lx - bx) / dd;
            let (mut landing, mut crossed) = (None, false);
            for k in 1..=128 {
                let (qx, qy) = (bx + ux * k as f32 * 32.0, by);
                let t = w.map.terrain_at(qx, qy);
                if t == Terrain::Water || t == Terrain::River {
                    crossed = true;
                } else if crossed && w.map.buildable(qx, qy) {
                    landing = Some((qx, qy));
                    break;
                }
            }
            let Some((sx, sy)) = landing else { continue };
            let (px, py) = w.map.dry_ground(bx, by);
            let (sx, sy) = w.map.dry_ground(sx, sy);
            if w.route_between(px, py, sx, sy).is_none()
                && w.route_over_water(px, py, sx, sy).is_some()
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
        let wet_x = ax + r.cell;
        assert_eq!(w.map.terrain_at(wet_x, ay), Terrain::Water);
        assert!(w.route_between(ax, ay, wet_x, ay).is_none());
        w.tick = 120;
        for i in 0..5u32 {
            w.agents.hunger[i as usize] = 60.0;
            w.water.detours.insert(i, Detour {
                path: vec![(ax, ay)],
                progress: 0.0,
                attempt: 0,
            });
            w.compute_detour(i, ax, ay, bx, by);
        }
        assert_eq!(w.water.fast_retries, 4);
        assert_eq!(w.water.detours[&4].attempt, 0);
        w.tick += 1;
        w.compute_detour(4, ax, ay, bx, by);
        assert_eq!(w.water.fast_retries, 1);
        assert_eq!(w.water.detours[&4].attempt, w.tick);
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
    fn frontier_workers_can_found_east_of_the_classic_cutoff() {
        let mut w = World::new_with_size(8, 42, Hill::default(), MapSize::X25).unwrap();
        let home = &w.markets[0];
        let (x, y) = w.map.find_terrain(
            &[Terrain::Grass, Terrain::Forest, Terrain::Fertile],
            home.x, home.y, 250.0, 315.0,
        );
        assert!(x > 2700.0 && w.map.buildable(x, y));
        assert!(w.markets.iter().all(|m| dist(x, y, m.x, m.y) >= 120.0));
        w.patches[0].x = x;
        w.patches[0].y = y;
        for i in 0..8 {
            w.agents.x[i] = x;
            w.agents.y[i] = y;
            w.agents.cargo_kind[i] = 1;
            w.agents.cargo[i] = 2;
            w.agents.berries[i] = w.agents.berries[i].max(2);
            w.agents.action[i] = Action::Gather;
            w.agents.dwell_cell[i] = (x / 96.0) as u32 + 4096 * (y / 96.0) as u32;
            w.agents.dwell_ticks[i] = 120;
            w.agents.target[i] = 0;
        }
        w.ledger.initial = w.stocks();
        w.tick = 10;
        let invalid_x = w.map.spec.width() + 100.0;
        w.patches[0].x = invalid_x;
        w.agents.x.fill(invalid_x);
        w.agents.dwell_cell.fill((invalid_x / 96.0) as u32 + 4096 * (y / 96.0) as u32);
        w.settlement_step();
        assert_eq!(w.stats.camps_founded, 0);
        w.patches[0].x = x;
        w.agents.x.fill(x);
        w.agents.dwell_cell.fill((x / 96.0) as u32 + 4096 * (y / 96.0) as u32);
        w.settlement_step();
        assert_eq!(w.stats.camps_founded, 1);
        assert!(dist(x, y, w.markets[3].x, w.markets[3].y) < 0.1);
        w.check().unwrap();
    }

    #[test]
    fn founding_stops_at_the_checkpoint_market_bound() {
        let mut w = World::new(8, 42).unwrap();
        // Saturate markets below the load gate; the founding branch must
        // refuse new camps so every savable world still passes 3..=64.
        while w.markets.len() < crate::MAX_MARKETS {
            let mut m = w.markets[0].clone();
            m.id = w.markets.len() as u32;
            m.y = 1800.0 - w.markets.len() as f32;
            m.coins = 0;
            m.goods = std::array::from_fn(|_| VecDeque::new());
            m.quotes = std::array::from_fn(|_| Quote::default());
            w.markets.push(m);
        }
        w.patches[0].x = 330.0;
        w.patches[0].y = 180.0;
        for i in 0..8 {
            w.agents.x[i] = 330.0;
            w.agents.y[i] = 180.0;
            w.agents.coins[i] = w.agents.coins[i].max(2);
            w.agents.berries[i] = w.agents.berries[i].max(2);
            w.agents.cargo_kind[i] = 1;
            w.agents.cargo[i] = 2;
            w.agents.action[i] = Action::Gather;
            w.agents.dwell_cell[i] = 3 + 4096;
            w.agents.dwell_ticks[i] = 120;
            w.agents.target[i] = 0;
        }
        w.ledger.initial = w.stocks();
        w.tick = 10;
        w.settlement_step();
        assert_eq!(w.markets.len(), crate::MAX_MARKETS);
        assert_eq!(w.stats.camps_founded, 0);
        w.check().unwrap();
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
        if matches!(
            a.action[i],
            Action::BuyInformation | Action::SellInformation
        ) {
            return self.information_destination(i);
        }
        if matches!(
            a.action[i],
            Action::CollectMeat | Action::SellMeat | Action::BuyMeat
        ) {
            return self.meat_destination(i);
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
        if matches!(
            a.action[i],
            Action::MakeCamp | Action::Wait | Action::Play | Action::Recover
        ) {
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
    /// Daytime traffic census. Every SAMPLE ticks while the sun is up,
    /// each traveler bound for a market that is not its home credits that
    /// unordered market pair once — walkers, convoy members and traders
    /// alike. Riders aboard a ship are water traffic, not road traffic.
    /// Counts fold into `Road.traffic` at the next dawn rollover.
    pub(super) fn sample_traffic(&mut self) {
        for i in 0..self.agents.len() {
            let a = &self.agents;
            if !a.alive[i] {
                continue;
            }
            let home = a.home[i];
            let Some((dx, dy)) = self.destination(i) else {
                continue;
            };
            // Standing at the destination already is not travel.
            if dist(a.x[i], a.y[i], dx, dy) < 40.0 {
                continue;
            }
            let Some(g) = self
                .markets
                .iter()
                .filter(|m| dist(m.x, m.y, dx, dy) <= 96.0)
                .min_by(|u, v| {
                    dist(u.x, u.y, dx, dy).total_cmp(&dist(v.x, v.y, dx, dy))
                })
                .map(|m| m.id)
            else {
                continue;
            };
            if g == home {
                continue;
            }
            if self
                .water
                .ships
                .iter()
                .any(|s| s.aboard.contains(&(i as u32)))
            {
                continue;
            }
            *self
                .traffic
                .counts
                .entry((home.min(g), home.max(g)))
                .or_default() += 1;
        }
    }
    /// Day rollover: yesterday's mover counts become each corridor's
    /// published traffic (roads go quiet again when nobody travels them),
    /// and pairs crossing the promotion bar get a `Road` record so
    /// `start_projects` can put the corridor up for construction.
    pub(super) fn publish_traffic(&mut self) {
        for r in &mut self.roads {
            r.traffic = 0;
        }
        let counts = std::mem::take(&mut self.traffic.counts);
        for ((a, b), count) in counts {
            if let Some(r) = self.roads.iter_mut().find(|r| {
                (r.a == a && r.b == b) || (r.a == b && r.b == a)
            }) {
                r.traffic = count;
            } else if count >= 8 && self.roads.len() < 256 {
                self.roads.push(Road {
                    a,
                    b,
                    built: false,
                    under_construction: false,
                    traffic: count,
                });
            }
        }
    }
    /// Lookahead point along a built road corridor serving `origin ->
    /// goal-market`: pure pursuit 40 units ahead of the mover's closest
    /// projection, so caravans ride the corridor instead of tracing a
    /// parallel line beside it. `None` when the goal is not a market, no
    /// built road links the pair, or the mover is close enough to arrive.
    fn road_corridor_target(
        &self,
        origin: u32,
        x: f32,
        y: f32,
        gx: f32,
        gy: f32,
    ) -> Option<(f32, f32)> {
        if dist(x, y, gx, gy) < 40.0 {
            return None;
        }
        let g = self
            .markets
            .iter()
            .filter(|m| dist(m.x, m.y, gx, gy) <= 72.0)
            .min_by(|u, v| {
                dist(u.x, u.y, gx, gy).total_cmp(&dist(v.x, v.y, gx, gy))
            })?
            .id;
        if g == origin {
            return None;
        }
        let ri = self.roads.iter().position(|r| {
            r.built && ((r.a == origin && r.b == g) || (r.b == origin && r.a == g))
        })?;
        let r = &self.roads[ri];
        let ends = [
            (self.markets[r.a as usize].x, self.markets[r.a as usize].y),
            (self.markets[r.b as usize].x, self.markets[r.b as usize].y),
        ];
        let pts = self
            .road_paths
            .get(ri)
            .filter(|p| p.len() >= 2)
            .map(|p| p.as_slice())
            .unwrap_or(&ends);
        Some(path_lookahead(pts, r.a == g, x, y, 40.0))
    }
    /// Routed corridor for the road the mover should ride, if any. A live
    /// water detour wins over the corridor; the corridor wins over the
    /// raw goal. `origin` is the mover's home market (walkers) or the
    /// convoy majority home (caravans).
    fn corridor_or_goal(
        &mut self,
        key: u32,
        origin: u32,
        x: f32,
        y: f32,
        gx: f32,
        gy: f32,
    ) -> (f32, f32) {
        if self.map.spec.size != MapSize::Classic
            && self
                .water
                .detours
                .get(&key)
                .is_some_and(|d| d.has_route())
        {
            return self.detour_target(key, x, y, gx, gy);
        }
        if let Some(t) = self.road_corridor_target(origin, x, y, gx, gy) {
            return t;
        }
        if self.map.spec.size != MapSize::Classic {
            return self.detour_target(key, x, y, gx, gy);
        }
        (gx, gy)
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
        if !self.passable(ax, ay) || !self.passable(bx, by) {
            return None;
        }
        const CELL: f32 = 64.0;
        let cols = (self.map.spec.width() / CELL).ceil() as usize;
        let rows = (self.map.spec.height() / CELL).ceil() as usize;
        let center = |index: usize| {
            ((index % cols) as f32 * CELL + CELL * 0.5,
             (index / cols) as f32 * CELL + CELL * 0.5)
        };
        let mut pass = vec![None; cols * rows];
        let mut walkable = |index: usize| {
            *pass[index].get_or_insert_with(|| {
                let (x, y) = center(index);
                let q = CELL * 0.25;
                [(x, y), (x - q, y - q), (x + q, y + q), (x - q, y + q), (x + q, y - q)]
                    .into_iter()
                    .all(|(x, y)| self.passable(x, y))
            })
        };
        let cell_of = |x: f32, y: f32| {
            ((x / CELL).clamp(0.0, (cols - 1) as f32) as usize,
             (y / CELL).clamp(0.0, (rows - 1) as f32) as usize)
        };
        // Real positions need a safe connector to the grid. A dry endpoint
        // can sit in a partially flooded cell whose centre is unreachable.
        let mut connectors = |x: f32, y: f32| {
            let (i, j) = cell_of(x, y);
            let mut result = Vec::new();
            for row in j.saturating_sub(2)..=(j + 2).min(rows - 1) {
                for col in i.saturating_sub(2)..=(i + 2).min(cols - 1) {
                    let node = row * cols + col;
                    let (cx, cy) = center(node);
                    if walkable(node) && self.clear_shot(x, y, cx, cy) {
                        result.push((node, (dist(x, y, cx, cy) * 10.0).ceil() as u64));
                    }
                }
            }
            result
        };
        let starts = connectors(ax, ay);
        let goals = connectors(bx, by);
        if starts.is_empty() || goals.is_empty() {
            return None;
        }
        use std::cmp::Reverse;
        use std::collections::BinaryHeap;
        let mut cost_to_goal = vec![None; cols * rows];
        for (node, cost) in goals {
            cost_to_goal[node] = Some(cost);
        }
        let heuristic = |node| {
            let (x, y) = center(node);
            (dist(x, y, bx, by) * 10.0) as u64
        };
        let mut cost = vec![u64::MAX; cols * rows];
        let mut from = vec![usize::MAX; cols * rows];
        let mut heap = BinaryHeap::new();
        for (node, initial) in starts {
            cost[node] = initial;
            heap.push(Reverse((initial + heuristic(node), initial, node)));
        }
        let mut best = None;
        let mut best_cost = u64::MAX;
        while let Some(Reverse((estimate, spent, at))) = heap.pop() {
            if estimate >= best_cost {
                break;
            }
            if spent != cost[at] {
                continue;
            }
            if let Some(exit) = cost_to_goal[at] {
                if spent + exit < best_cost {
                    best_cost = spent + exit;
                    best = Some(at);
                }
            }
            let (i, j) = (at % cols, at / cols);
            let (x, y) = center(at);
            for nj in j.saturating_sub(1)..=(j + 1).min(rows - 1) {
                for ni in i.saturating_sub(1)..=(i + 1).min(cols - 1) {
                    let next = nj * cols + ni;
                    if next == at || !walkable(next) {
                        continue;
                    }
                    if ni != i && nj != j && (!walkable(j * cols + ni) || !walkable(nj * cols + i)) {
                        continue;
                    }
                    let (nx, ny) = center(next);
                    let next_cost = spent + (dist(x, y, nx, ny) * 10.0).ceil() as u64;
                    if next_cost < cost[next] && self.clear_shot(x, y, nx, ny) {
                        cost[next] = next_cost;
                        from[next] = at;
                        heap.push(Reverse((next_cost + heuristic(next), next_cost, next)));
                    }
                }
            }
        }
        let goal = best?;
        let mut cells = vec![goal];
        let mut at = goal;
        while from[at] != usize::MAX {
            at = from[at];
            cells.push(at);
        }
        cells.reverse();
        let mut raw = vec![(ax, ay)];
        raw.extend(cells.into_iter().map(center));
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
    pub(super) fn clear_shot(&self, ax: f32, ay: f32, bx: f32, by: f32) -> bool {
        let d = dist(ax, ay, bx, by);
        // Validate endpoints as well as the interior so an underwater
        // target cannot masquerade as a direct dry route.
        if !self.passable(ax, ay) || !self.passable(bx, by) {
            return false;
        }
        let n = (d / 4.0).ceil().max(1.0) as usize;
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
    pub(super) fn patch_reach(&mut self, patch: usize, land_reach: f32) -> f32 {
        let Some(site) = self.patches.get(patch) else {
            return land_reach;
        };
        if site.kind == 3
            && self.map.spec.size != MapSize::Classic
            && self.map.terrain_at(site.x, site.y) == Terrain::Water
        {
            24.0
        } else {
            land_reach
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
                // Riders are pinned to the hull and cannot walk to food:
                // while moored, disembark the starving and the hurt onto
                // the dock so a long wait for passengers is not a death
                // sentence. They re-queue later if still stranded.
                let mut released: Vec<u32> = Vec::new();
                let aboard: Vec<u32> = self.water.ships[si]
                    .aboard
                    .iter()
                    .copied()
                    .filter(|&id| {
                        let i = id as usize;
                        if !self.agents.alive[i] {
                            return false;
                        }
                        if self.agents.hunger[i] > 60.0 || self.agents.health[i] < 30.0 {
                            released.push(id);
                            return false;
                        }
                        true
                    })
                    .collect();
                self.water.ships[si].aboard = aboard;
                for (rank, id) in released.iter().enumerate() {
                    let i = *id as usize;
                    self.agents.x[i] = dock.0 + (rank % 7) as f32 * 3.0 - 9.0;
                    self.agents.y[i] = dock.1 + (rank / 7) as f32 * 3.0 - 10.5;
                }
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
    const DETOUR: &[f32] = &[
        15.0, 30.0, 45.0, 60.0, 75.0, 90.0, 115.0, 140.0, 165.0, 180.0,
    ];
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
        let step = step.min(d);
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
    /// Next steering point for a mover with a routed detour, or (tx, ty)
    /// when there is none. Drops the detour once the destination is in
    /// dry line-of-sight, the waypoints are consumed, or the detour was
    /// routed for a different destination.
    fn detour_target(&mut self, key: u32, x: f32, y: f32, tx: f32, ty: f32) -> (f32, f32) {
        let path_len = match self.water.detours.get(&key) {
            Some(d) if d.has_route() => d.path.len(),
            _ => return (tx, ty),
        };
        // Stale only when the mover's destination changed; a "clear shot"
        // check here was dropped because coarse sampling can call a thin
        // band open and send the mover back into the stall loop.
        let last = self.water.detours[&key].path[path_len - 1];
        if dist(last.0, last.1, tx, ty) > 48.0 {
            self.water.detours.remove(&key);
            return (tx, ty);
        }
        // Pure pursuit with monotonic arclength: project the mover onto
        // the polyline, clamp progress forward, then steer toward a
        // lookahead measured along the path.
        let path = &self.water.detours[&key].path;
        let (mut total, mut best) = (0f32, f32::MAX);
        let mut base = 0f32;
        let mut progress = 0f32;
        for s in 0..path.len() - 1 {
            let (ax, ay) = path[s];
            let (bx, by) = path[s + 1];
            let (dx, dy) = (bx - ax, by - ay);
            let l2 = dx * dx + dy * dy;
            let tt = if l2 > 0.0 {
                (((x - ax) * dx + (y - ay) * dy) / l2).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let (px, py) = (ax + dx * tt, ay + dy * tt);
            let d2 = (x - px) * (x - px) + (y - py) * (y - py);
            if d2 < best {
                best = d2;
                progress = base + l2.sqrt() * tt;
            }
            base += l2.sqrt();
            total = base;
        }
        let stored = self.water.detours[&key].progress;
        // Monotonic progress stops the lookahead flipping between adjacent
        // segments — but a mover shoved well behind its stored progress is
        // off-route entirely: clamping forward steers it at a point that is
        // only reachable across water, and it pins there forever. Let the
        // projection reset instead so the mover re-joins the path.
        if stored - progress <= 32.0 {
            progress = progress.max(stored);
        }
        self.water.detours.get_mut(&key).unwrap().progress = progress;
        if (total - progress <= 12.0 || dist(x, y, last.0, last.1) < 12.0)
            && self.clear_shot(x, y, tx, ty)
        {
            self.water.detours.remove(&key);
            return (tx, ty);
        }
        let path = &self.water.detours[&key].path;
        let mut ahead = progress + 16.0;
        let mut until_corner = progress;
        let mut corner = None;
        for s in 0..path.len() - 1 {
            let (ax, ay) = path[s];
            let (bx, by) = path[s + 1];
            let len = dist(ax, ay, bx, by);
            if corner.is_none() && until_corner <= len && len > 0.0 {
                corner = Some(((ax, ay), (bx, by)));
            }
            if ahead <= len && len > 0.0 {
                let tt = ahead / len;
                let aim = (ax + (bx - ax) * tt, ay + (by - ay) * tt);
                return self.safe_detour_aim(x, y, aim, corner);
            }
            until_corner -= len;
            ahead -= len;
        }
        self.safe_detour_aim(x, y, last, corner)
    }
    fn safe_detour_aim(
        &self,
        x: f32,
        y: f32,
        aim: (f32, f32),
        corner: Option<((f32, f32), (f32, f32))>,
    ) -> (f32, f32) {
        if let Some((before, after)) = corner {
            if !self.clear_shot(x, y, aim.0, aim.1) {
                return if self.clear_shot(x, y, after.0, after.1) {
                    after
                } else {
                    before
                };
            }
        }
        aim
    }

    /// Route a stalled mover around the water blocking it. `route_between`
    /// answers one of: dry waypoints (stored as the active detour), an
    /// empty path (already a clear shot — steering resolves it), or None
    /// (truly water-blocked — the report/ferry chain handles it). The
    /// empty-path entry doubles as a ~2400-tick backoff so unreachable
    /// movers do not rerun A* every stall tick.
    fn compute_detour(&mut self, key: u32, x: f32, y: f32, tx: f32, ty: f32) {
        // Starving solo movers are the famine case: they must re-decide to
        // a reachable target instead of grinding the crossing. Fed movers
        // keep pressing — their stalls are the reports that feed ports.
        let starving_agent = key & 0x8000_0000 == 0
            && self.agents.convoy[key as usize] == NONE
            && self.agents.hunger[key as usize] > 55.0;
        let mut reroute_live = false;
        let mut fast_retry = false;
        if let Some(d) = self.water.detours.get(&key) {
            // Unreachable marks back off for ~2400 ticks; a live detour that
            // still has the mover stalled reroutes sooner from its new spot.
            let normal_retry = if d.has_route() { 480 } else { 2400 };
            let age = self.tick.saturating_sub(d.attempt);
            fast_retry = starving_agent && !d.matches_destination(tx, ty) && age < normal_retry;
            let retry = if fast_retry { 80 } else { normal_retry };
            if self.tick.saturating_sub(d.attempt) < retry {
                return;
            }
            reroute_live = d.has_route() && d.matches_destination(tx, ty);
        }
        let routing_for_report = key & 0x8000_0000 == 0
            && self.water.pending.iter().any(|report| report.agent == key);
        if starving_agent && !routing_for_report
            && self.target_unreachable(
                key as usize,
                self.agents.action[key as usize],
                self.agents.target[key as usize],
            )
        {
            return;
        }
        if !self.water.detours.contains_key(&key) && self.water.detours.len() >= 512 {
            // Router saturated: a starving strander locked out of the A*
            // would re-pick the same unreachable target until it dies.
            // The advisory block expires, so a later open slot can still
            // prove reachability; fed movers keep pressing meanwhile.
            if starving_agent && !routing_for_report {
                self.record_unreachable(key);
            }
            return;
        }
        if reroute_live && starving_agent && !routing_for_report {
            // One routed shot while starving: a detour that expired its
            // retry with the mover still pinned has already failed. Mark
            // the target so the hunger offer redirects to nearer food.
            self.record_unreachable(key);
            self.water.detours.insert(
                key,
                Detour {
                    path: vec![(tx, ty)],
                    progress: 0.0,
                    attempt: self.tick,
                },
            );
            return;
        }
        if fast_retry && !self.clear_shot(x, y, tx, ty) {
            const MAX_FAST_RETRIES_PER_TICK: u8 = 4;
            if self.water.retry_budget_tick != self.tick {
                self.water.retry_budget_tick = self.tick;
                self.water.fast_retries = 0;
            }
            if self.water.fast_retries >= MAX_FAST_RETRIES_PER_TICK {
                return;
            }
            self.water.fast_retries += 1;
        }
        match self.route_between(x, y, tx, ty) {
            Some(path) if !path.is_empty() => {
                self.water.detours.insert(
                    key,
                    Detour {
                        path,
                        progress: 0.0,
                        attempt: self.tick,
                    },
                );
            }
            route => {
                self.water.detours.insert(
                    key,
                    Detour {
                        path: vec![(tx, ty)],
                        progress: 0.0,
                        attempt: self.tick,
                    },
                );
                // A failed A* is proof this walker's destination cannot be
                // reached dry: record its current (action, target) so the
                // decision layer can steer a starving mover elsewhere.
                if route.is_none() && !routing_for_report
                    && key & 0x8000_0000 == 0 && self.agents.convoy[key as usize] == NONE {
                    self.record_unreachable(key);
                }
            }
        }
    }
    /// Mark this agent's current (action, target) as water-blocked. The
    /// entry is advisory for hungry movers and expires after ~2400 ticks;
    /// each agent keeps at most the 8 freshest marks. The shore position
    /// is also shared so neighbours of the same bank skip the target
    /// without paying the stall cost themselves.
    fn record_unreachable(&mut self, key: u32) {
        let i = key as usize;
        let pair = (self.agents.action[i] as u8, self.agents.target[i]);
        let blocks = self.water.unreachable.entry(key).or_default();
        if blocks.len() >= 8 {
            let oldest = *blocks.iter().min_by_key(|(_, t)| *t).unwrap().0;
            blocks.remove(&oldest);
        }
        blocks.insert(pair, self.tick);
        let (x, y) = (self.agents.x[i], self.agents.y[i]);
        let origins = self.water.shared_unreachable.entry(pair).or_default();
        origins.retain(|&(_, _, t)| self.tick.saturating_sub(t) < 2400);
        let fresh = origins
            .iter()
            .any(|&(ox, oy, _)| dist(ox, oy, x, y) <= 48.0);
        if !fresh {
            if origins.len() >= 16 {
                let oldest = origins
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, &(_, _, t))| t)
                    .unwrap()
                    .0;
                origins.remove(oldest);
            }
            origins.push((x, y, self.tick));
        }
    }
    /// True while `target` sits on the mover's water-blocked list — but only
    /// for a hungry mover. A fed strander keeps pressing the shore (its
    /// reports feed the port chain and a ferry may board it); a starving one
    /// must re-decide to food it can actually reach, or it dies in place.
    pub(super) fn target_unreachable(&self, i: usize, action: Action, target: u32) -> bool {
        if self.agents.hunger[i] <= 55.0 {
            return false;
        }
        if self
            .water
            .unreachable
            .get(&(i as u32))
            .and_then(|blocks| blocks.get(&(action as u8, target)))
            .is_some_and(|&t| self.tick.saturating_sub(t) < 2400)
        {
            return true;
        }
        // Same-bank sharing: another strander already proved this target
        // from within ~96u of where this mover is standing.
        let Some(origins) = self
            .water
            .shared_unreachable
            .get(&(action as u8, target))
        else {
            return false;
        };
        let (x, y) = (self.agents.x[i], self.agents.y[i]);
        origins.iter().any(|&(ox, oy, t)| {
            self.tick.saturating_sub(t) < 2400 && dist(ox, oy, x, y) <= 96.0
        })
    }
    /// Move agent `i` toward (tx, ty) by `step` units, steering around water
    /// on generated maps; on Classic this is the historical straight step.
    pub(super) fn walk_toward(&mut self, i: usize, tx: f32, ty: f32, step: f32) {
        let (x, y) = (self.agents.x[i], self.agents.y[i]);
        let (gx, gy) =
            self.corridor_or_goal(i as u32, self.agents.home[i], x, y, tx, ty);
        if let Some((nx, ny)) = self.steer(x, y, gx, gy, step, i % 2 == 0) {
            // Rivers are forded at half speed: the mover still makes
            // absolute progress each tick, it just crosses more slowly.
            let f = if self.map.terrain_at(nx, ny) == Terrain::River {
                0.5
            } else {
                1.0
            };
            self.agents.x[i] = x + (nx - x) * f;
            self.agents.y[i] = y + (ny - y) * f;
        } else if self.map.spec.size != MapSize::Classic
            && self.map.terrain_at(x, y) == Terrain::Water
        {
            // A mover already standing in open water (legacy spawn, a
            // displacement bug) has no legal candidate at all: let it
            // wade straight toward the nearest dry point instead of
            // freezing in place forever.
            let (dx, dy) = self.map.dry_ground(x, y);
            let dd = dist(x, y, dx, dy);
            if dd > 0.5 {
                let s = step.min(dd - 0.5);
                self.agents.x[i] = x + (dx - x) / dd * s;
                self.agents.y[i] = y + (dy - y) / dd * s;
            }
        }
        if self.map.spec.size != MapSize::Classic {
            self.track_stranded(i, tx, ty);
        }
    }
    /// An agent still en route but unable to make 24 units of progress
    /// for ~600 ticks is stranded at water and starts carrying a report
    /// home (delivered by `update_water_reports` on arrival).
    fn track_stranded(&mut self, i: usize, tx: f32, ty: f32) {
        if !self.agents.alive[i] {
            return;
        }
        if self.water.progress.len() <= i {
            self.water.progress.resize(i + 1, None);
        }
        let (x, y) = (self.agents.x[i], self.agents.y[i]);
        match self.water.progress[i] {
            Some((px, py, since)) if dist(x, y, px, py) >= 24.0 => {
                // The mover covered ground — but sliding along a shore is
                // not progress: keep the stall clock running unless it is
                // measurably closer to the target than the last probe
                // point was. (tx, ty) stands in for the distance measured
                // then, so the serialized probe needs no new field.)
                if dist(x, y, tx, ty) < dist(px, py, tx, ty) - 24.0 {
                    self.water.progress[i] = Some((x, y, self.tick));
                } else {
                    self.water.progress[i] = Some((x, y, since));
                }
            }
            Some((_, _, since)) => {
                let stalled = self.tick.saturating_sub(since);
                // A shore-hugging oscillation resolves nothing on its own:
                // after ~240 stalled ticks route the mover along dry
                // waypoints. If a detour exists the agent walks around the
                // lake, the probe resets, and no report is ever filed.
                if stalled > 240 {
                    self.compute_detour(i as u32, x, y, tx, ty);
                }
                // The pending cap bounds carried reports only — probes and
                // detours must keep working while the report queue is full,
                // or every later strander loses its routed escape.
                if stalled > 600
                    && !self.water.pending.iter().any(|p| p.agent == i as u32)
                {
                    if self.water.pending.len() < 256 {
                        self.water.pending.push(PendingBlock {
                            agent: i as u32,
                            x,
                            y,
                            tx,
                            ty,
                            tick: self.tick,
                        });
                    }
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
        if self.map.spec.size == MapSize::Classic {
            return;
        }
        // Evict detours whose mover is gone: a dead walker or a released
        // convoy slot never consumes its route, and the 512-entry cap on
        // the map would otherwise fill with orphans — after which every
        // new strander silently fails compute_detour.
        self.water.detours.retain(|&key, _| {
            if key & 0x8000_0000 != 0 {
                self.convoys
                    .get((key & 0x7fff_ffff) as usize)
                    .is_some_and(|c| c.is_some())
            } else {
                self.agents.alive.get(key as usize).copied().unwrap_or(false)
            }
        });
        // Blocked-target entries age out after ~a day (a port or a changed
        // destination may make the crossing possible again), and die with
        // their mover.
        let tick = self.tick;
        self.water.unreachable.retain(|&key, blocks| {
            if !self.agents.alive.get(key as usize).copied().unwrap_or(false) {
                return false;
            }
            blocks.retain(|_, t| tick.saturating_sub(*t) < 2400);
            !blocks.is_empty()
        });
        // Port queues are only a waiting list: the dead, starving and hurt
        // must leave it even when no ship has been commissioned yet, or a
        // shipless dock becomes a pile of starving walkers.
        for pi in 0..self.water.ports.len() {
            for far in [false, true] {
                let mut q = std::mem::take(if far {
                    &mut self.water.ports[pi].queue_far
                } else {
                    &mut self.water.ports[pi].queue
                });
                q.retain(|&id| {
                    let i = id as usize;
                    self.agents.alive[i] && self.agents.hunger[i] <= 60.0 && self.agents.health[i] > 30.0
                });
                if far {
                    self.water.ports[pi].queue_far = q;
                } else {
                    self.water.ports[pi].queue = q;
                }
            }
        }
        if self.water.pending.is_empty() {
            return;
        }
        let mut arrived: Vec<(PendingBlock, u32)> = Vec::new();
        self.water.pending.retain(|p| {
            let i = p.agent as usize;
            // Filed at the first settlement the bearer actually reaches —
            // a carrier whose home lies across the water can never get
            // there, and holding the report for home delivery would trap
            // it at the same shore it is trying to cross.
            let reached = if self.agents.alive[i] {
                let (x, y) = (self.agents.x[i], self.agents.y[i]);
                self.markets
                    .iter()
                    .enumerate()
                    .filter(|(_, m)| {
                        let r = rest::settlement_radius(m.housing_blocks).max(160.0);
                        dist(x, y, m.x, m.y) < r
                    })
                    .min_by(|(_, a), (_, b)| {
                        dist(x, y, a.x, a.y).total_cmp(&dist(x, y, b.x, b.y))
                    })
                    .map(|(mi, _)| mi as u32)
            } else {
                None
            };
            // A carried report goes stale after ~2 days: the bearer may be
            // stranded on the wrong side itself, and keeping it forever
            // would hold a report-queue slot for a delivery that can never
            // happen. Expired reports drop silently; reaching home still
            // delivers.
            let expired = self.tick.saturating_sub(p.tick) > 4800;
            if let Some(m) = reached {
                arrived.push((*p, m));
            }
            reached.is_none() && !expired && self.agents.alive[i]
        });
        for (p, market) in arrived {
            if !self.agents.alive[p.agent as usize] {
                continue;
            }
            let cell = ((p.x / 96.0) as i32, (p.y / 96.0) as i32);
            if let Some(b) = self.water.blocks.iter_mut().find(|b| {
                b.market == market
                    && ((b.x / 96.0) as i32, (b.y / 96.0) as i32) == cell
            }) {
                b.reports = b.reports.saturating_add(1);
                b.tick = p.tick;
            } else if self.water.blocks.len() < 64 {
                self.water.blocks.push(WaterBlock {
                    market,
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
    /// Turn delivered water blocks into paired port construction. A
    /// settlement with the coffers and materials surveys a landing across
    /// the water, then posts Shipwright jobs on the urban board: the home
    /// shore always, the far shore only when a dry land route exists —
    /// otherwise the port is flagged `ferry` and waits for a ship to carry
    /// builders.
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
            // forty timber for a proper junk, or twelve for a lashed raft
            // when the market's forests are stripped — a slow ferry still
            // beats no crossing at all.
            let raft = self.markets[m].stock(1) < 40;
            if p.near_done
                && p.ferry
                && !self.water.ships.iter().any(|s| s.port == pi as u32)
                && !self.urban.jobs.iter().flatten().any(|j| {
                    j.kind == 4 && j.target == pi as u32 && j.source == 2
                })
                && self.markets[m].stock(1) >= if raft { 12 } else { 40 }
                && self.markets[m].coins >= WAGE
            {
                let mut left = if raft { 12 } else { 40 };
                while left > 0 {
                    left -= self.markets[m]
                        .take(1, left)
                        .expect("reserved ship timber")
                        .quantity;
                }
                self.ledger.built[1] += if raft { 12 } else { 40 };
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
            // Any settlement that can pay and stock the materials answers a
            // delivered block: the strander is often a camp-dweller, and a
            // zoning gate would deny ferries to exactly the movers the water
            // chain exists to rescue. Timber is preferred, but a fieldstone
            // jetty on a few piles serves when the forests are stripped.
            let materials = if m < self.markets.len()
                && self.markets[m].stock(1) >= WOOD
                && self.markets[m].stock(2) >= STONE
            {
                Some([(1usize, WOOD), (2, STONE)])
            } else if m < self.markets.len()
                && self.markets[m].stock(1) >= 4
                && self.markets[m].stock(2) >= 24
            {
                Some([(1usize, 4), (2, 24)])
            } else {
                None
            };
            if m >= self.markets.len() || self.coffers(m) < 2 * WAGE || materials.is_none() {
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
            for (k, q) in materials.unwrap() {
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
                    || self.in_energy_emergency(i)
                    || self.should_travel_for_food_alone(i)
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
                let key = 0x8000_0000 | slot as u32;
                // The caravan rides a built road serving its home->goal
                // pair instead of tracing a parallel line beside it;
                // a live water detour still wins while active.
                let mut votes: BTreeMap<u32, u32> = BTreeMap::new();
                for &id in &c.members {
                    *votes.entry(self.agents.home[id as usize]).or_default() += 1;
                }
                let home = votes
                    .into_iter()
                    .max_by_key(|(_, n)| *n)
                    .map(|(h, _)| h)
                    .unwrap_or(NONE);
                let (gx, gy) =
                    self.corridor_or_goal(key, home, c.x, c.y, c.goal_x, c.goal_y);
                if let Some((nx, ny)) = self.steer(c.x, c.y, gx, gy, step, slot % 2 == 0)
                {
                    c.x = nx;
                    c.y = ny;
                }
                if self.map.spec.size != MapSize::Classic {
                    match self.water.convoy_probe.get(&(slot as u32)).copied() {
                        Some((px, py, since)) if dist(c.x, c.y, px, py) >= 24.0 => {
                            // Sliding along a shore is not progress — only
                            // a probe point measurably closer to the goal
                            // resets the stall clock.
                            if dist(c.x, c.y, c.goal_x, c.goal_y)
                                < dist(px, py, c.goal_x, c.goal_y) - 24.0
                            {
                                self.water
                                    .convoy_probe
                                    .insert(slot as u32, (c.x, c.y, self.tick));
                            } else {
                                self.water
                                    .convoy_probe
                                    .insert(slot as u32, (c.x, c.y, since));
                            }
                        }
                        Some((_, _, since))
                            if self.tick.saturating_sub(since) > 240 =>
                        {
                            self.compute_detour(key, c.x, c.y, c.goal_x, c.goal_y);
                        }
                        Some(_) => {}
                        _ => {
                            self.water
                                .convoy_probe
                                .insert(slot as u32, (c.x, c.y, self.tick));
                        }
                    }
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
                || self.in_energy_emergency(i)
                || self.should_travel_for_food_alone(i)
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
        let classic = self.map.spec.size == MapSize::Classic;
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
                && (!classic || a.x[i] < 2700.0)
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
            if self.markets.len() >= crate::MAX_MARKETS || ids.len() < 8 {
                continue;
            }
            let x = ids.iter().map(|&i| self.agents.x[i]).sum::<f32>() / ids.len() as f32;
            let y = ids.iter().map(|&i| self.agents.y[i]).sum::<f32>() / ids.len() as f32;
            if !classic {
                let (min_x, max_x, min_y, max_y) = self.map.spec.land_bounds();
                if !(min_x..=max_x).contains(&x)
                    || !(min_y..=max_y).contains(&y)
                    || !self.map.buildable(x, y)
                {
                    continue;
                }
            }
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
