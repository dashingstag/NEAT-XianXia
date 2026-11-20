use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};

use crate::WORLD_HEIGHT;
use crate::WORLD_WIDTH;

/// Bump when generation rules change. Old saves keep their stored
/// version so terrain never moves under an existing world.
pub const GENERATOR_VERSION: u32 = 2;
/// Edge length of a generated chunk in world units.
pub const CHUNK_SIZE: u32 = 384;
/// Fraction of the homeland radius, from the map centre, that is
/// guaranteed mundane: no qi, no spark sites.
const HOMELAND_FRAC: f32 = 0.22;

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum MapSize {
    /// The historical fixed 3072x1920 layout. Legacy saves and fixtures.
    #[default]
    Classic,
    X4,
    X25,
    X100,
}
impl MapSize {
    /// Side-length multiplier over the classic map (area multiplier squared).
    pub fn scale(self) -> u32 {
        match self {
            MapSize::Classic => 1,
            MapSize::X4 => 2,
            MapSize::X25 => 5,
            MapSize::X100 => 10,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Terrain {
    Water,
    Rock,
    Marsh,
    Forest,
    Fertile,
    Grass,
    /// Wadeable running water: blocks nothing, slows walkers, reads as
    /// a ford for roads. Lakes remain impassable.
    River,
}

/// Generator identity persisted in the save. Chunk data itself is
/// derivable from (seed, version, size, coords) and is not persisted.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MapSpec {
    pub size: MapSize,
    pub seed: u64,
    pub version: u32,
    /// Centre of the mundane homeland in world units.
    pub home_x: f32,
    pub home_y: f32,
}
impl MapSpec {
    pub fn classic(seed: u64) -> Self {
        Self {
            size: MapSize::Classic,
            seed,
            version: GENERATOR_VERSION,
            home_x: WORLD_WIDTH / 2.0,
            home_y: WORLD_HEIGHT / 2.0,
        }
    }
    pub fn width(&self) -> f32 {
        WORLD_WIDTH * self.size.scale() as f32
    }
    pub fn height(&self) -> f32 {
        WORLD_HEIGHT * self.size.scale() as f32
    }
    /// Half-width of the mundane ring around the homeland centre.
    pub fn homeland_radius(&self) -> f32 {
        self.width().min(self.height()) * HOMELAND_FRAC
    }
    /// Open-land bounds (min_x, max_x, min_y, max_y): forests, mines,
    /// resource placement. Classic keeps the historical constants.
    pub fn land_bounds(&self) -> (f32, f32, f32, f32) {
        if self.size == MapSize::Classic {
            (20.0, 2670.0, 20.0, 1900.0)
        } else {
            (20.0, self.width() - 20.0, 20.0, self.height() - 20.0)
        }
    }
    /// Farmable-land bounds. Classic keeps the historical constants.
    pub fn field_bounds(&self) -> (f32, f32, f32, f32) {
        if self.size == MapSize::Classic {
            (30.0, 2650.0, 30.0, 1880.0)
        } else {
            (30.0, self.width() - 30.0, 30.0, self.height() - 30.0)
        }
    }
    /// Wildlife roaming clamps.
    pub fn wild_bounds(&self) -> (f32, f32, f32, f32) {
        if self.size == MapSize::Classic {
            (10.0, 2650.0, 10.0, 1890.0)
        } else {
            (10.0, self.width() - 10.0, 10.0, self.height() - 10.0)
        }
    }
}

/// Per-settlement fog-of-war knowledge: `chunks[market_id]` is the set of
/// generated-map chunk coordinates that market's people have visited.
/// Knowledge only accumulates — it is the record of where anyone from
/// that settlement has physically been.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Exploration {
    pub chunks: Vec<BTreeSet<(u32, u32)>>,
}
impl Exploration {
    /// Chunk coordinates covering a world position.
    pub fn chunk_of(x: f32, y: f32) -> (u32, u32) {
        ((x / CHUNK_SIZE as f32) as u32, (y / CHUNK_SIZE as f32) as u32)
    }
    /// Mark the chunks overlapped by a circle as known to `market`.
    pub fn reveal(&mut self, market: usize, x: f32, y: f32, radius: f32) {
        if market >= self.chunks.len() {
            self.chunks.resize(market + 1, BTreeSet::new());
        }
        let (x0, y0) = Self::chunk_of(x - radius, y - radius);
        let (x1, y1) = Self::chunk_of(x + radius, y + radius);
        for cy in y0..=y1 {
            for cx in x0..=x1 {
                self.chunks[market].insert((cx, cy));
            }
        }
    }
    /// Chunks known to any settlement as a row-major 0/1 grid.
    pub fn known_mask(&self, spec: &MapSpec) -> (usize, usize, Vec<u8>) {
        let cols = (spec.width() / CHUNK_SIZE as f32).ceil() as usize;
        let rows = (spec.height() / CHUNK_SIZE as f32).ceil() as usize;
        let mut mask = vec![0u8; cols * rows];
        for &(x, y) in self.chunks.iter().flatten() {
            if (x as usize) < cols && (y as usize) < rows {
                mask[y as usize * cols + x as usize] = 1;
            }
        }
        (cols, rows, mask)
    }
    /// Total chunks known to a market (0 when it knows nothing).
    pub fn known(&self, market: usize) -> usize {
        self.chunks.get(market).map_or(0, |s| s.len())
    }
}

/// One generated chunk: per-cell scalar layers. Cells are CELLS_PER_SIDE
/// per edge; sampling interpolates inside the chunk.
pub const CELLS_PER_SIDE: usize = 8;
#[derive(Clone, Debug)]
pub struct Chunk {
    pub elevation: [[f32; CELLS_PER_SIDE + 1]; CELLS_PER_SIDE + 1],
    pub moisture: [[f32; CELLS_PER_SIDE + 1]; CELLS_PER_SIDE + 1],
    pub qi: [[f32; CELLS_PER_SIDE + 1]; CELLS_PER_SIDE + 1],
}

/// Row-major samples at generator-cell centres for viewers that draw
/// the map without running the generator. Bytes scale 0..=1 to 0..=255.
/// `regions` carries a territory id per cell (255 for water) and
/// `region_labels` its name and centroid.
#[derive(Clone, Debug)]
pub struct TerrainRaster {
    pub cols: usize,
    pub rows: usize,
    pub cell: f32,
    pub terrain: Vec<u8>,
    pub elevation: Vec<u8>,
    pub qi: Vec<u8>,
    pub regions: Vec<u8>,
    pub region_labels: Vec<(String, f32, f32)>,
}

/// One traced watercourse: polyline in world units and a display width.
#[derive(Clone, Debug, PartialEq)]
pub struct River {
    pub points: Vec<(f32, f32)>,
    pub width: f32,
}
const RIVER_CELL: f32 = 48.0;
/// Ordered key for heap use; elevations are non-negative so bit order
/// matches value order.
fn ordered(v: f32) -> u32 {
    v.to_bits()
}

/// Downhill-flow rivers and the cell set they occupy, computed once.
#[derive(Clone, Debug, Default)]
struct RiverGrid {
    cells: std::collections::HashSet<(u32, u32)>,
    rivers: Vec<River>,
}

/// Generated world terrain. The chunk cache is regenerated on demand
/// and intentionally not serialized.
#[derive(Clone, Debug)]
pub struct WorldMap {
    pub spec: MapSpec,
    cache: HashMap<(u32, u32), Chunk>,
    rivers: Option<Box<RiverGrid>>,
}
impl serde::Serialize for WorldMap {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.spec.serialize(s)
    }
}
impl<'de> serde::Deserialize<'de> for WorldMap {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(Self::new(MapSpec::deserialize(d)?))
    }
}

impl WorldMap {
    pub fn new(spec: MapSpec) -> Self {
        Self {
            spec,
            cache: HashMap::new(),
            rivers: None,
        }
    }
    pub fn classic(seed: u64) -> Self {
        Self::new(MapSpec::classic(seed))
    }

    fn chunk_coords(&self, x: f32, y: f32) -> (u32, u32) {
        (
            (x.max(0.0) as u32 / CHUNK_SIZE).min(self.spec.width() as u32 / CHUNK_SIZE),
            (y.max(0.0) as u32 / CHUNK_SIZE).min(self.spec.height() as u32 / CHUNK_SIZE),
        )
    }

    /// Deterministic per-coordinate hash -> [0,1). Order-independent:
    /// the same (seed, layer, gx, gy) always yields the same value.
    fn lattice(&self, layer: u64, gx: u32, gy: u32) -> f32 {
        let mut h = self
            .spec
            .seed
            .wrapping_mul(0x9E37_79B9_7F4A_7C15)
            .wrapping_add(layer.wrapping_mul(0xC2B2_AE3D_27D4_EB4F))
            .wrapping_add((gx as u64) << 32)
            .wrapping_add(gy as u64);
        h ^= h >> 30;
        h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
        h ^= h >> 27;
        h = h.wrapping_mul(0x94D0_49BB_1331_11EB);
        h ^= h >> 31;
        (h >> 11) as f32 / (1u64 << 53) as f32
    }

    /// Smooth fractal value noise at integer lattice scale `freq`
    /// (cells per chunk edge), evaluated in chunk-local coordinates.
    fn noise(&self, layer: u64, gx: f32, gy: f32, octaves: u32) -> f32 {
        let mut sum = 0.0;
        let mut amp = 1.0;
        let mut norm = 0.0;
        let mut f = 1.0;
        for _ in 0..octaves {
            let x = gx * f;
            let y = gy * f;
            let x0 = x.floor() as i64;
            let y0 = y.floor() as i64;
            let fx = x - x0 as f32;
            let fy = y - y0 as f32;
            let sx = fx * fx * (3.0 - 2.0 * fx);
            let sy = fy * fy * (3.0 - 2.0 * fy);
            let l = |dx: i64, dy: i64| self.lattice(layer, (x0 + dx) as u32, (y0 + dy) as u32);
            let v = l(0, 0) * (1.0 - sx) * (1.0 - sy)
                + l(1, 0) * sx * (1.0 - sy)
                + l(0, 1) * (1.0 - sx) * sy
                + l(1, 1) * sx * sy;
            sum += v * amp;
            norm += amp;
            amp *= 0.5;
            f *= 2.0;
        }
        sum / norm
    }

    fn generate(&self, cx: u32, cy: u32) -> Chunk {
        let n = CELLS_PER_SIDE;
        let mut c = Chunk {
            elevation: [[0.0; 9]; 9],
            moisture: [[0.0; 9]; 9],
            qi: [[0.0; 9]; 9],
        };
        let base_x = cx * CHUNK_SIZE;
        let base_y = cy * CHUNK_SIZE;
        let step = CHUNK_SIZE / n as u32;
        let home_r = self.spec.homeland_radius();
        for j in 0..=n {
            for i in 0..=n {
                let wx = (base_x + i as u32 * step) as f32;
                let wy = (base_y + j as u32 * step) as f32;
                // Lattice coords at 4 cells per chunk for the base octave.
                let gx = wx / CHUNK_SIZE as f32 * 4.0;
                let gy = wy / CHUNK_SIZE as f32 * 4.0;
                c.elevation[j][i] = self.noise(1, gx, gy, 4);
                c.moisture[j][i] = self.noise(2, gx, gy, 3);
                let raw_qi = self.noise(3, gx, gy, 2);
                let dx = wx - self.spec.home_x;
                let dy = wy - self.spec.home_y;
                let dist = (dx * dx + dy * dy).sqrt();
                // Zero qi inside the homeland; fade in over one radius.
                let ramp = ((dist - home_r) / home_r).clamp(0.0, 1.0);
                c.qi[j][i] = raw_qi * ramp;
            }
        }
        c
    }

    fn sample<F: Fn(&Chunk) -> &[[f32; CELLS_PER_SIDE + 1]; CELLS_PER_SIDE + 1]>(
        &mut self,
        x: f32,
        y: f32,
        layer: F,
    ) -> f32 {
        let x = x.clamp(0.0, self.spec.width() - 1.0);
        let y = y.clamp(0.0, self.spec.height() - 1.0);
        let (cx, cy) = self.chunk_coords(x, y);
        let key = (cx, cy);
        if !self.cache.contains_key(&key) {
            let generated = self.generate(cx, cy);
            self.cache.insert(key, generated);
        }
        let chunk = self.cache.get(&key).unwrap();
        let lx = ((x - cx as f32 * CHUNK_SIZE as f32) / CHUNK_SIZE as f32) * CELLS_PER_SIDE as f32;
        let ly = ((y - cy as f32 * CHUNK_SIZE as f32) / CHUNK_SIZE as f32) * CELLS_PER_SIDE as f32;
        let i = (lx.floor() as usize).min(CELLS_PER_SIDE - 1);
        let j = (ly.floor() as usize).min(CELLS_PER_SIDE - 1);
        let fx = lx - i as f32;
        let fy = ly - j as f32;
        let g = layer(chunk);
        g[j][i] * (1.0 - fx) * (1.0 - fy)
            + g[j][i + 1] * fx * (1.0 - fy)
            + g[j + 1][i] * (1.0 - fx) * fy
            + g[j + 1][i + 1] * fx * fy
    }

    pub fn elevation_at(&mut self, x: f32, y: f32) -> f32 {
        self.sample(x, y, |c| &c.elevation)
    }
    pub fn moisture_at(&mut self, x: f32, y: f32) -> f32 {
        self.sample(x, y, |c| &c.moisture)
    }
    pub fn qi_at(&mut self, x: f32, y: f32) -> f32 {
        self.sample(x, y, |c| &c.qi)
    }

    /// Rivers, computed once from downhill flow accumulation on a coarse
    /// grid. Deterministic: the result depends only on the map spec.
    fn river_grid(&mut self) -> &RiverGrid {
        if self.rivers.is_none() {
            self.rivers = Some(Box::new(self.build_rivers()));
        }
        self.rivers.as_ref().unwrap()
    }
    /// Traced watercourses for the viewer; empty on Classic or v1 maps.
    pub fn rivers(&mut self) -> &[River] {
        if self.spec.version < 2 || self.spec.size == MapSize::Classic {
            return &[];
        }
        &self.river_grid().rivers
    }
    fn build_rivers(&mut self) -> RiverGrid {
        let cols = (self.spec.width() / RIVER_CELL).ceil() as usize;
        let rows = (self.spec.height() / RIVER_CELL).ceil() as usize;
        let mut elev = vec![0.0f32; cols * rows];
        for j in 0..rows {
            for i in 0..cols {
                elev[j * cols + i] = self
                    .elevation_at((i as f32 + 0.5) * RIVER_CELL, (j as f32 + 0.5) * RIVER_CELL);
            }
        }
        // Priority-flood from the borders inward so every land cell drains
        // to the map edge: filled[c] is the spill elevation and down[c] the
        // neighbour it drains through. Raw fbm has too many local pits for
        // naive steepest-descent tracing to produce real watercourses.
        let mut filled = vec![f32::INFINITY; cols * rows];
        let mut down = vec![usize::MAX; cols * rows];
        let mut heap = std::collections::BinaryHeap::new();
        for j in 0..rows {
            for i in 0..cols {
                if i == 0 || j == 0 || i == cols - 1 || j == rows - 1 {
                    let at = j * cols + i;
                    filled[at] = elev[at];
                    heap.push(std::cmp::Reverse((ordered(elev[at]), at)));
                }
            }
        }
        while let Some(std::cmp::Reverse((eb, at))) = heap.pop() {
            let e = f32::from_bits(eb);
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
                    if filled[n] == f32::INFINITY {
                        filled[n] = elev[n].max(e);
                        down[n] = at;
                        heap.push(std::cmp::Reverse((ordered(filled[n]), n)));
                    }
                }
            }
        }
        // Accumulate flow over the filled surface in spill order.
        let mut order: Vec<usize> = (0..cols * rows).collect();
        order.sort_by(|&a, &b| filled[b].total_cmp(&filled[a]).then(a.cmp(&b)));
        let mut flow = vec![1u32; cols * rows];
        let water = |e: f32| e < 0.34;
        for &c in &order {
            if water(filled[c]) || down[c] == usize::MAX {
                continue;
            }
            flow[down[c]] += flow[c];
        }
        // Sources: modestly elevated cells whose catchment has already
        // collected several upstream cells, spaced away from rivers that
        // were traced earlier so channels don't braid into duplicates.
        let mut grid = RiverGrid::default();
        for &c in &order {
            if water(elev[c]) || flow[c] < 16 {
                continue;
            }
            let (ci, cj) = ((c % cols) as i32, (c / cols) as i32);
            let crowded = (-6..=6).any(|dj| {
                (-6..=6).any(|di| {
                    grid.cells
                        .contains(&((ci + di) as u32, (cj + dj) as u32))
                })
            });
            if crowded {
                continue;
            }
            let mut trace = Vec::new();
            let mut at = c;
            for _ in 0..4 * (cols + rows) {
                let key = ((at % cols) as u32, (at / cols) as u32);
                trace.push(at);
                if grid.cells.contains(&key) || water(elev[at]) {
                    break;
                }
                let n = down[at];
                if n == usize::MAX {
                    break;
                }
                at = n;
            }
            if trace.len() < 4 {
                continue;
            }
            for &at in &trace {
                grid.cells.insert(((at % cols) as u32, (at / cols) as u32));
            }
            grid.rivers.push(River {
                points: trace
                    .iter()
                    .map(|&at| {
                        (
                            (at % cols) as f32 * RIVER_CELL + RIVER_CELL * 0.5,
                            (at / cols) as f32 * RIVER_CELL + RIVER_CELL * 0.5,
                        )
                    })
                    .collect(),
                width: (flow[c] as f32 / 48.0).sqrt().clamp(1.0, 4.0),
            });
        }
        grid
    }
    /// Whether a world position falls in a traced river cell.
    fn is_river(&mut self, x: f32, y: f32) -> bool {
        if self.spec.version < 2 || self.spec.size == MapSize::Classic {
            return false;
        }
        let key = ((x / RIVER_CELL) as u32, (y / RIVER_CELL) as u32);
        self.river_grid().cells.contains(&key)
    }
    /// Terrain classification read by settlement, resource and wildlife
    /// placement. Thresholds are part of GENERATOR_VERSION.
    pub fn terrain_at(&mut self, x: f32, y: f32) -> Terrain {
        let e = self.elevation_at(x, y);
        let m = self.moisture_at(x, y);
        if e < 0.34 {
            return Terrain::Water;
        }
        if self.is_river(x, y) {
            return Terrain::River;
        }
        if e > 0.78 {
            Terrain::Rock
        } else if m > 0.72 && e < 0.45 {
            Terrain::Marsh
        } else if m > 0.58 {
            Terrain::Forest
        } else if m > 0.42 && e < 0.55 {
            Terrain::Fertile
        } else {
            Terrain::Grass
        }
    }

    /// Deterministic spiral scan for a point whose terrain matches
    /// `want`, searching a ring around (cx, cy). Returns the first hit;
    /// falls back to the last candidate when nothing matches.
    pub fn find_terrain(
        &mut self,
        want: &[Terrain],
        cx: f32,
        cy: f32,
        r_min: f32,
        r_max: f32,
    ) -> (f32, f32) {
        const STEPS: usize = 512;
        const GOLDEN: f32 = 2.399_963_1;
        let (min_x, max_x, min_y, max_y) = self.spec.land_bounds();
        let mut last = (cx.clamp(min_x, max_x), cy.clamp(min_y, max_y));
        for s in 0..STEPS {
            let t = s as f32 / STEPS as f32;
            let r = r_min + (r_max - r_min) * t;
            let a = s as f32 * GOLDEN;
            let x = (cx + a.cos() * r).clamp(min_x, max_x);
            let y = (cy + a.sin() * r).clamp(min_y, max_y);
            last = (x, y);
            if want.contains(&self.terrain_at(x, y)) {
                return (x, y);
            }
        }
        last
    }

    /// Samples roughly 768 cells across so the wire payload stays bounded
    /// while zoomed-in viewers get finer cells on smaller maps.
    pub fn raster(&mut self) -> TerrainRaster {
        let cell = self.spec.width() / 768.0;
        let cols = (self.spec.width() / cell).ceil() as usize;
        let rows = (self.spec.height() / cell).ceil() as usize;
        let unit = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        let mut out = TerrainRaster {
            cols,
            rows,
            cell,
            terrain: Vec::with_capacity(cols * rows),
            elevation: Vec::with_capacity(cols * rows),
            qi: Vec::with_capacity(cols * rows),
            regions: Vec::new(),
            region_labels: Vec::new(),
        };
        for j in 0..rows {
            for i in 0..cols {
                let x = (i as f32 + 0.5) * cell;
                let y = (j as f32 + 0.5) * cell;
                out.terrain.push(self.terrain_at(x, y) as u8);
                out.elevation.push(unit(self.elevation_at(x, y)));
                out.qi.push(unit(self.qi_at(x, y)));
            }
        }
        if self.spec.size != MapSize::Classic {
            let (regions, region_labels) = self.grow_regions(cols, rows, cell);
            out.regions = regions;
            out.region_labels = region_labels;
        }
        out
    }

    /// Region-grow named territories over the raster grid. The mundane
    /// homeland is always region 0, "Heartlands"; frontier seeds are
    /// greedy-spaced land cells ordered by lattice hash, then flood-filled.
    fn grow_regions(
        &mut self,
        cols: usize,
        rows: usize,
        cell: f32,
    ) -> (Vec<u8>, Vec<(String, f32, f32)>) {
        const EPITHETS: [&str; 16] = [
            "Azure", "Jade", "Verdant", "Silent", "Crimson", "Ashen", "Misty",
            "Howling", "Golden", "Umbral", "Emerald", "Scarlet", "Ivory",
            "Obsidian", "Whispering", "Sacred",
        ];
        const NOUNS: [&str; 16] = [
            "Reaches", "Marches", "Wilds", "Vale", "Steppes", "Basin",
            "Highlands", "Shore", "Expanse", "Delta", "Glade", "Frontier",
            "Barrens", "Hollow", "Plateau", "Thickets",
        ];
        let mut region = vec![u8::MAX; cols * rows];
        let home_r = self.spec.homeland_radius() * 0.85;
        // Land cells outside the homeland are seed candidates, ordered by a
        // lattice hash so the pick is order-independent and replay-safe.
        let mut order: Vec<(u64, usize)> = Vec::new();
        for j in (0..rows).step_by(4) {
            for i in (0..cols).step_by(4) {
                let x = (i as f32 + 0.5) * cell;
                let y = (j as f32 + 0.5) * cell;
                let at = j * cols + i;
                if self.terrain_at(x, y) == Terrain::Water {
                    continue;
                }
                if (x - self.spec.home_x).hypot(y - self.spec.home_y) < home_r {
                    continue;
                }
                let h = (self.lattice(5, i as u32, j as u32) * 1e9) as u64;
                order.push((h, at));
            }
        }
        order.sort();
        let order: Vec<usize> = order.into_iter().map(|(_, at)| at).collect();
        let area = self.spec.width() * self.spec.height();
        let classic_area = WORLD_WIDTH * WORLD_HEIGHT;
        let target = ((area / classic_area) * 4.0) as usize;
        let k = target.clamp(6, 48);
        let spacing = (area / k as f32).sqrt() * 0.5 / cell;
        let mut seeds: Vec<usize> = Vec::new();
        for &c in &order {
            if seeds.len() + 1 >= k {
                break;
            }
            let (ci, cj) = (c % cols, c / cols);
            if seeds.iter().all(|&s| {
                let (si, sj) = (s % cols, s / cols);
                ((ci as f32 - si as f32).hypot(cj as f32 - sj as f32)) > spacing
            }) {
                seeds.push(c);
            }
        }
        // Multi-source BFS: homeland cells take region 0, seeds 1..=n.
        let mut queue: std::collections::VecDeque<(usize, u8)> =
            std::collections::VecDeque::new();
        for j in 0..rows {
            for i in 0..cols {
                let x = (i as f32 + 0.5) * cell;
                let y = (j as f32 + 0.5) * cell;
                let at = j * cols + i;
                if self.terrain_at(x, y) == Terrain::Water {
                    region[at] = u8::MAX;
                } else if (x - self.spec.home_x).hypot(y - self.spec.home_y) < home_r {
                    region[at] = 0;
                    queue.push_back((at, 0));
                }
            }
        }
        for (id, &s) in seeds.iter().enumerate() {
            if region[s] == u8::MAX {
                region[s] = (id + 1) as u8;
                queue.push_back((s, (id + 1) as u8));
            }
        }
        while let Some((at, id)) = queue.pop_front() {
            let (i, j) = (at % cols, at / cols);
            for (di, dj) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
                let (ni, nj) = (i as i32 + di, j as i32 + dj);
                if ni < 0 || nj < 0 || ni >= cols as i32 || nj >= rows as i32 {
                    continue;
                }
                let n = nj as usize * cols + ni as usize;
                if region[n] == u8::MAX
                    && self.terrain_at(
                        (ni as f32 + 0.5) * cell,
                        (nj as f32 + 0.5) * cell,
                    ) != Terrain::Water
                {
                    region[n] = id;
                    queue.push_back((n, id));
                }
            }
        }
        // Centroids become label anchors; names come from the seed's hash.
        let mut sum = vec![(0f32, 0f32, 0u32); seeds.len() + 1];
        for j in 0..rows {
            for i in 0..cols {
                let id = region[j * cols + i] as usize;
                if id < sum.len() {
                    sum[id].0 += (i as f32 + 0.5) * cell;
                    sum[id].1 += (j as f32 + 0.5) * cell;
                    sum[id].2 += 1;
                }
            }
        }
        let mut labels = Vec::with_capacity(seeds.len() + 1);
        for id in 0..=seeds.len() {
            let (sx, sy, n) = sum[id];
            if n == 0 {
                continue;
            }
            let name = if id == 0 {
                "Heartlands".to_string()
            } else {
                let (i, j) = (seeds[id - 1] % cols, seeds[id - 1] / cols);
                let h = self.lattice(6, i as u32, j as u32);
                let a = (h * EPITHETS.len() as f32) as usize % EPITHETS.len();
                let b = ((h * NOUNS.len() as f32 * EPITHETS.len() as f32) as usize)
                    % NOUNS.len();
                format!("{} {}", EPITHETS[a], NOUNS[b])
            };
            labels.push((name, sx / n as f32, sy / n as f32));
        }
        (region, labels)
    }
}

impl crate::World {
    /// Fog of war: credit each living agent's current chunk to their
    /// home settlement. Knowledge accumulates over time; positions are
    /// cheap to insert (one set insert per agent per tick).
    pub(super) fn update_explored(&mut self) {
        self.explored
            .chunks
            .resize(self.markets.len(), BTreeSet::new());
        for i in 0..self.agents.len() {
            if !self.agents.alive[i] {
                continue;
            }
            let home = self.agents.home[i] as usize;
            if home >= self.explored.chunks.len() {
                continue;
            }
            let chunk = Exploration::chunk_of(self.agents.x[i], self.agents.y[i]);
            self.explored.chunks[home].insert(chunk);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_is_order_independent() {
        let mut a = WorldMap::new(MapSpec {
            size: MapSize::X4,
            seed: 7,
            version: GENERATOR_VERSION,
            home_x: 3000.0,
            home_y: 2000.0,
        });
        let mut b = WorldMap::new(MapSpec {
            size: MapSize::X4,
            seed: 7,
            version: GENERATOR_VERSION,
            home_x: 3000.0,
            home_y: 2000.0,
        });
        // Query far points in opposite orders; caches differ, values must not.
        let a1 = a.elevation_at(5000.0, 1000.0);
        let a2 = a.elevation_at(100.0, 3500.0);
        let b2 = b.elevation_at(100.0, 3500.0);
        let b1 = b.elevation_at(5000.0, 1000.0);
        assert_eq!(a1, b1);
        assert_eq!(a2, b2);
    }

    #[test]
    fn homeland_has_no_qi() {
        let spec = MapSpec {
            size: MapSize::X4,
            seed: 11,
            version: GENERATOR_VERSION,
            home_x: 3072.0,
            home_y: 1920.0,
        };
        let mut m = WorldMap::new(spec.clone());
        let r = spec.homeland_radius();
        for k in 0..8 {
            let ang = k as f32 * std::f32::consts::TAU / 8.0;
            let x = spec.home_x + ang.cos() * r * 0.5;
            let y = spec.home_y + ang.sin() * r * 0.5;
            assert_eq!(m.qi_at(x, y), 0.0, "qi inside homeland at {x},{y}");
        }
        // Far points eventually show nonzero qi on most seeds.
        let edge = m.qi_at(spec.home_x + r * 3.0, spec.home_y + r * 3.0);
        assert!(edge > 0.0);
    }

    #[test]
    fn different_seeds_generate_different_terrain() {
        let spec = |seed| MapSpec {
            size: MapSize::X4,
            seed,
            version: GENERATOR_VERSION,
            home_x: 3000.0,
            home_y: 2000.0,
        };
        let mut a = WorldMap::new(spec(1));
        let mut b = WorldMap::new(spec(2));
        let mut differ = 0;
        for k in 0..20 {
            let x = 200.0 + k as f32 * 250.0;
            if a.elevation_at(x, 500.0) != b.elevation_at(x, 500.0) {
                differ += 1;
            }
        }
        assert!(differ > 15);
    }

    #[test]
    fn generated_world_places_on_terrain_and_steps() {
        let mut w = crate::World::new_with_size(
            150,
            7,
            crate::Hill::default(),
            MapSize::X4,
        )
        .unwrap();
        let (min_x, max_x, min_y, max_y) = w.map.spec.land_bounds();
        for m in &w.markets {
            assert!(
                (min_x..=max_x).contains(&m.x) && (min_y..=max_y).contains(&m.y),
                "market {} at {},{}",
                m.id,
                m.x,
                m.y
            );
            // Homeland is mundane: no qi under a starting settlement.
            assert_eq!(w.map.qi_at(m.x, m.y), 0.0);
        }
        for p in &w.patches {
            assert!(
                p.x.is_finite()
                    && p.y.is_finite()
                    && p.x >= 0.0
                    && p.y >= 0.0
                    && p.x <= w.map.spec.width()
                    && p.y <= w.map.spec.height(),
                "patch {:?} out of map",
                p
            );
        }
        // Farms sit on workable ground, not rock or water.
        for f in &w.fertility {
            assert!(w.farm_ground(f.x, f.y, 1.0), "fertile region {:?}", f);
        }
        for _ in 0..30 {
            w.step(false);
        }
        w.check().unwrap();
        // A different seed lays out a different homeland.
        let other = crate::World::new_with_size(
            150,
            8,
            crate::Hill::default(),
            MapSize::X4,
        )
        .unwrap();
        let same = w
            .markets
            .iter()
            .zip(&other.markets)
            .filter(|(a, b)| a.x == b.x && a.y == b.y)
            .count();
        assert!(same < 2, "seeds produced near-identical market sites");
    }

    #[test]
    fn exploration_accumulates_and_survives_save() {
        let mut w = crate::World::new(20, 42).unwrap();
        // Settlements start knowing their home area.
        assert!(w.explored.known(0) > 0);
        let before = w.explored.known(1);
        // Walk an agent of market 1 far away; their chunk becomes known.
        let i = (0..w.agents.len())
            .find(|&i| w.agents.alive[i] && w.agents.home[i] == 1)
            .unwrap();
        w.agents.x[i] = 2600.0;
        w.agents.y[i] = 1500.0;
        let target = Exploration::chunk_of(2600.0, 1500.0);
        w.step(false);
        assert!(w.explored.known(1) > before || w.explored.chunks[1].contains(&target));
        // Persisted through save/load.
        let path = std::env::temp_dir().join(format!(
            "xianxia-explored-{}.snapshot",
            std::process::id()
        ));
        w.save(&path).unwrap();
        let restored = crate::World::load(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(restored.explored, w.explored);
    }

    #[test]
    fn spec_dimensions_scale_by_side() {
        assert_eq!(MapSpec::classic(0).width(), WORLD_WIDTH);
        let x4 = MapSpec {
            size: MapSize::X100,
            seed: 0,
            version: 1,
            home_x: 0.0,
            home_y: 0.0,
        };
        assert_eq!(x4.width(), WORLD_WIDTH * 10.0);
    }
    #[test]
    fn landscape_raster_matches_terrain_at() {
        let mut m = WorldMap::new(MapSpec {
            size: MapSize::X4,
            seed: 5,
            version: GENERATOR_VERSION,
            home_x: 3072.0,
            home_y: 1920.0,
        });
        let r = m.raster();
        assert_eq!((r.cols, r.rows, r.cell), (768, 480, 8.0));
        assert_eq!(r.terrain.len(), r.cols * r.rows);
        assert_eq!(r.qi.len(), r.elevation.len());
        for (i, j) in [(0, 0), (37, 11), (767, 479), (384, 240)] {
            let (x, y) = ((i as f32 + 0.5) * r.cell, (j as f32 + 0.5) * r.cell);
            assert_eq!(r.terrain[j * r.cols + i], m.terrain_at(x, y) as u8);
        }
        let kinds: BTreeSet<u8> = r.terrain.iter().copied().collect();
        assert!(kinds.len() >= 3, "a frontier map shows varied terrain");
    }

    #[test]
    fn rivers_are_deterministic_and_classified() {
        let spec = MapSpec {
            size: MapSize::X4,
            seed: 5,
            version: GENERATOR_VERSION,
            home_x: 3072.0,
            home_y: 1920.0,
        };
        let mut a = WorldMap::new(spec.clone());
        let mut b = WorldMap::new(spec.clone());
        assert_eq!(a.rivers(), b.rivers());
        assert!(!a.rivers().is_empty(), "a frontier map spawns rivers");
        let rivers = a.rivers().to_vec();
        for river in rivers {
            for &(x, y) in river.points.iter().skip(1) {
                assert!(
                    matches!(a.terrain_at(x, y), Terrain::River | Terrain::Water),
                    "traced river cells must classify as running or open water"
                );
            }
        }
        // Version 1 maps keep the old water-free classification.
        let mut legacy = WorldMap::new(MapSpec { version: 1, ..spec });
        assert!(legacy.rivers().is_empty());
    }

    #[test]
    fn regions_cover_the_continent_with_named_territories() {
        let mut m = WorldMap::new(MapSpec {
            size: MapSize::X4,
            seed: 5,
            version: GENERATOR_VERSION,
            home_x: 3072.0,
            home_y: 1920.0,
        });
        let r = m.raster();
        assert_eq!(r.regions.len(), r.cols * r.rows);
        assert!(r.region_labels.iter().any(|(n, _, _)| n == "Heartlands"));
        let land = r.terrain.iter().filter(|&&t| t != Terrain::Water as u8).count();
        let named = r.regions.iter().filter(|&&id| id != u8::MAX).count();
        assert!(
            named as f32 > land as f32 * 0.5,
            "most land belongs to a named territory (islands may stay wild)"
        );
        assert!(r.region_labels.len() > 3);
    }

    #[test]
    fn landscape_known_mask_unions_settlements() {
        let spec = MapSpec::classic(0);
        let mut e = Exploration::default();
        e.reveal(0, 100.0, 100.0, 10.0);
        e.reveal(2, 3000.0, 1900.0, 10.0);
        let (cols, rows, mask) = e.known_mask(&spec);
        assert_eq!((cols, rows), (8, 5));
        assert_eq!(mask.iter().filter(|&&k| k == 1).count(), 2);
        assert_eq!((mask[0], mask[4 * 8 + 7]), (1, 1));
    }
}
