//! Food-limited woodland wildlife with species-specific candidate-scoring NEAT policies.
use super::*;
use std::collections::BTreeMap;
const MAX_LIVING: usize = 1024;
const MAX_HISTORY: usize = 4096;
const SPAWN_CHECK_TICKS: u64 = 10;
const PREY_VACANCY_DAYS: u64 = 15;
const PREDATOR_VACANCY_DAYS: u64 = 90;
const PHEASANT: u8 = 4;
const FOREST_CAP: usize = 40;
/// Data-driven species traits. New species (predators etc.) append a
/// row here instead of adding `species ==` branches. Order matters:
/// saved worlds and the wire store `species` as this table's index.
#[derive(Clone, Copy, Debug)]
pub struct SpeciesDef {
    pub name: &'static str,
    /// Age (days) at which the animal can breed.
    pub adult_days: u32,
    /// Days between matings.
    pub cooldown_days: u32,
    /// Gestation length in days.
    pub gestation_days: u32,
    /// Units eaten per feeding.
    pub portion: u64,
    /// Hunger gained per second.
    pub hunger_rate: f32,
    /// Maximum age in days.
    pub lifespan_days: u32,
    /// Ground speed units/second.
    pub speed: f32,
    /// Offspring per birth.
    pub litter: u32,
    /// Age assigned to founding population animals.
    pub founding_age: u32,
    pub predator: bool,
}
pub const SPECIES: [SpeciesDef; 5] = [
    SpeciesDef {
        name: "Rabbit",
        adult_days: 90,
        cooldown_days: 45,
        gestation_days: 30,
        portion: 1,
        hunger_rate: 0.08,
        lifespan_days: 2500,
        speed: 22.0,
        litter: 2,
        founding_age: 120,
        predator: false,
    },
    SpeciesDef {
        name: "Deer",
        adult_days: 365,
        cooldown_days: 365,
        gestation_days: 200,
        portion: 3,
        hunger_rate: 0.12,
        lifespan_days: 5000,
        speed: 28.0,
        litter: 1,
        founding_age: 600,
        predator: false,
    },
    SpeciesDef {
        name: "Fox",
        adult_days: 240,
        cooldown_days: 180,
        gestation_days: 55,
        portion: 1,
        hunger_rate: 0.002,
        lifespan_days: 2600,
        speed: 32.0,
        litter: 2,
        founding_age: 300,
        predator: true,
    },
    SpeciesDef {
        name: "Wolf",
        adult_days: 650,
        cooldown_days: 365,
        gestation_days: 65,
        portion: 2,
        hunger_rate: 0.002,
        lifespan_days: 4400,
        speed: 36.0,
        litter: 2,
        founding_age: 800,
        predator: true,
    },
    SpeciesDef {
        name: "Pheasant",
        adult_days: 180,
        cooldown_days: 90,
        gestation_days: 25,
        portion: 1,
        hunger_rate: 0.07,
        lifespan_days: 2200,
        speed: 24.0,
        litter: 2,
        founding_age: 240,
        predator: false,
    },
];
impl SpeciesDef {
    /// Traits for a species index. Unknown indices (e.g. a newer save on
    /// an older build) degrade to the last known row rather than panic.
    pub fn of(species: u8) -> &'static SpeciesDef {
        SPECIES.get(species as usize).unwrap_or(&SPECIES[0])
    }
}
const NAMES: [&str; 9] = [
    "Graze grass",
    "Eat berries",
    "Flee",
    "Rest",
    "Explore",
    "Find mate",
    "Hunt prey",
    "Feed on carcass",
    "Move to new den",
];
pub const ANIMAL_SENSORS: [&str; 16] = [
    "need",
    "benefit",
    "risk",
    "distance",
    "fertility",
    "continuity",
    "rabbit",
    "deer",
    "hunger",
    "energy",
    "health",
    "human_proximity",
    "grass_cover",
    "local_density",
    "night",
    "adult",
];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Forest {
    pub x: f32,
    pub y: f32,
    pub radius: f32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Grass {
    pub forest: u32,
    pub x: f32,
    pub y: f32,
    pub stock: u64,
    pub capacity: u64,
    pub growth: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnimalPregnancy {
    pub father: u32,
    pub due: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Animal {
    pub species: u8,
    pub female: bool,
    pub forest: u32,
    pub x: f32,
    pub y: f32,
    pub heading: f32,
    pub alive: bool,
    pub health: f32,
    pub hunger: f32,
    pub energy: f32,
    pub born: u64,
    pub initial_age: u32,
    pub mother: u32,
    pub father: u32,
    pub pregnancy: Option<AnimalPregnancy>,
    pub last_mated: u64,
    pub action: u8,
    pub target: u32,
    pub goal_x: f32,
    pub goal_y: f32,
    pub timer: f32,
    pub threat: f32,
    pub threat_x: f32,
    pub threat_y: f32,
    pub genome: Genome,
    #[serde(skip)]
    pub brain: Option<Compiled>,
}
impl Animal {
    pub(super) fn age(&self, tick: u64) -> u32 {
        self.initial_age
            .saturating_add((tick.saturating_sub(self.born) / rest::DAY_TICKS) as u32)
    }
    pub(super) fn adult(&self, tick: u64) -> bool {
        self.age(tick) >= SpeciesDef::of(self.species).adult_days
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EcologyLedger {
    pub initial_grass: u64,
    pub grown_grass: u64,
    pub eaten_grass: u64,
    pub eaten_berries: u64,
    pub initial_animals: u64,
    pub births: u64,
    pub deaths: u64,
    pub conceptions: u64,
    pub lost_pregnancies: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
enum SpawnAnchor {
    Grove,
    Mountain { x: f32, y: f32 },
    Shore { x: f32, y: f32 },
}

impl SpawnAnchor {
    fn area(self, forest: &Forest) -> (f32, f32, f32) {
        match self {
            Self::Grove => (forest.x, forest.y, forest.radius),
            Self::Mountain { x, y } | Self::Shore { x, y } => (x, y, 120.0),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct SpawnPoint {
    anchor: SpawnAnchor,
    forest: usize,
    species: u8,
    vacant_since: Option<u64>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub(super) struct WildlifeSpawns {
    points: Vec<SpawnPoint>,
    spawned: u64,
}

#[derive(Deserialize)]
pub(super) struct LegacyWildlifeSpawns {
    points: Vec<LegacySpawnPoint>,
    spawned: u64,
}

#[derive(Deserialize)]
struct LegacySpawnPoint {
    forest: usize,
    species: u8,
    vacant_since: Option<u64>,
}

impl From<LegacyWildlifeSpawns> for WildlifeSpawns {
    fn from(old: LegacyWildlifeSpawns) -> Self {
        Self {
            points: old
                .points
                .into_iter()
                .map(|p| SpawnPoint {
                    anchor: SpawnAnchor::Grove,
                    forest: p.forest,
                    species: p.species,
                    vacant_since: p.vacant_since,
                })
                .collect(),
            spawned: old.spawned,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Wildlife {
    #[serde(skip)]
    pub(super) spawns: WildlifeSpawns,
    #[serde(skip)]
    pub(super) spears: std::collections::BTreeSet<u32>,
    #[serde(skip)]
    pub(super) bows: std::collections::BTreeSet<u32>,
    #[serde(skip)]
    pub(super) hunting_stats: HuntingStats,
    pub forests: Vec<Forest>,
    pub grass: Vec<Grass>,
    pub animals: Vec<Animal>,
    pub innovations: Innovations,
    pub rng: Rng,
    pub ledger: EcologyLedger,
}
impl Default for Wildlife {
    fn default() -> Self {
        Self {
            spawns: WildlifeSpawns::default(),
            spears: Default::default(),
            bows: Default::default(),
            hunting_stats: Default::default(),
            forests: vec![],
            grass: vec![],
            animals: vec![],
            innovations: Innovations::default(),
            rng: Rng(0),
            ledger: EcologyLedger::default(),
        }
    }
}
/// Per-decision-round caches so animal choices don't rescan the whole
/// animal/grass list for every individual (was O(n^2) at 1024 animals).
/// Buckets are by forest because mating requires the same forest anyway,
/// so restricting scans is semantically identical.
struct ForestStats {
    members: Vec<Vec<usize>>,
    grass_idx: Vec<Vec<usize>>,
    food: Vec<u64>,
    pop: Vec<usize>,
    species_pop: Vec<[usize; SPECIES.len()]>,
    carrion: Vec<u64>,
    living: usize,
}
impl ForestStats {
    fn build(w: &Wildlife, carcasses: &BTreeMap<u32, u64>) -> Self {
        let n = w.forests.len();
        let mut stats = Self {
            members: vec![vec![]; n],
            grass_idx: vec![vec![]; n],
            food: vec![0; n],
            pop: vec![0; n],
            species_pop: vec![[0; SPECIES.len()]; n],
            carrion: vec![0; n],
            living: 0,
        };
        for (i, a) in w.animals.iter().enumerate() {
            if !a.alive {
                continue;
            }
            stats.living += 1;
            if let Some(m) = stats.members.get_mut(a.forest as usize) {
                m.push(i);
                stats.pop[a.forest as usize] += 1;
                stats.species_pop[a.forest as usize][a.species as usize] += 1;
            }
        }
        for (i, g) in w.grass.iter().enumerate() {
            if let Some(v) = stats.grass_idx.get_mut(g.forest as usize) {
                v.push(i);
                stats.food[g.forest as usize] += g.stock;
            }
        }
        for (&id, &stock) in carcasses {
            let a = &w.animals[id as usize];
            let den = &w.forests[a.forest as usize];
            if dist(a.x, a.y, den.x, den.y) <= den.radius * 2.5 {
                stats.carrion[a.forest as usize] += stock;
            }
        }
        stats
    }
}

#[derive(Clone)]
struct Choice {
    action: u8,
    target: u32,
    x: f32,
    y: f32,
    inputs: [f32; INPUTS],
    score: f32,
}
fn founder_species(frontier: bool, forest: usize) -> &'static [u8] {
    if !frontier {
        &[0, 0, 0, 0, 0, 0, 1, 1, 1, 1]
    } else if forest % 4 == 0 {
        &[0, 0, 0, 0, 1, 1, 4, 4, 2, 2]
    } else if forest % 4 == 2 {
        &[0, 0, 0, 0, 1, 1, 4, 4, 3, 3]
    } else {
        &[0, 0, 0, 0, 0, 0, 1, 1, 4, 4]
    }
}

impl Wildlife {
    fn add_founder(
        &mut self,
        species: u8,
        female: bool,
        forest: usize,
        position: (f32, f32),
        heading: f32,
        tick: u64,
        genome: Genome,
    ) {
        let (x, y) = position;
        let den = &self.forests[forest];
        self.animals.push(Animal {
            species,
            female,
            forest: forest as u32,
            x,
            y,
            heading,
            alive: true,
            health: 100.0,
            hunger: 15.0 + self.rng.unit() * 20.0,
            energy: 75.0,
            born: tick,
            initial_age: SpeciesDef::of(species).founding_age,
            mother: NONE,
            father: NONE,
            pregnancy: None,
            last_mated: 0,
            action: 4,
            target: NONE,
            goal_x: den.x,
            goal_y: den.y,
            timer: 0.0,
            threat: 0.0,
            threat_x: den.x,
            threat_y: den.y,
            brain: Some(genome.compile().expect("valid animal seed")),
            genome,
        });
    }

    pub(super) fn rebuild(&mut self) -> Result<(), String> {
        for a in &mut self.animals {
            a.brain = Some(a.genome.compile()?);
        }
        Ok(())
    }
    fn food(&self, forest: u32) -> u64 {
        self.grass
            .iter()
            .filter(|g| g.forest == forest)
            .map(|g| g.stock)
            .sum()
    }
    fn population(&self, forest: u32) -> usize {
        self.animals
            .iter()
            .filter(|a| a.alive && a.forest == forest)
            .count()
    }
    fn fertile(&self, i: usize, tick: u64) -> bool {
        let a = &self.animals[i];
        a.alive
            && a.adult(tick)
            && a.health >= 70.0
            && a.hunger < 45.0
            && a.energy >= 50.0
            && a.pregnancy.is_none()
            && (a.last_mated == 0
                || tick.saturating_sub(a.last_mated)
                    >= SpeciesDef::of(a.species).cooldown_days as u64 * rest::DAY_TICKS)
    }
    fn compatible(&self, i: usize, j: usize, tick: u64) -> bool {
        if i == j || !self.fertile(i, tick) || !self.fertile(j, tick) {
            return false;
        }
        let (a, b) = (&self.animals[i], &self.animals[j]);
        a.species == b.species
            && a.female != b.female
            && a.forest == b.forest
            && a.mother != j as u32
            && a.father != j as u32
            && b.mother != i as u32
            && b.father != i as u32
            && !(a.mother != NONE && a.mother == b.mother)
            && !(a.father != NONE && a.father == b.father)
    }
    fn seed_genome(&mut self) -> Genome {
        let mut g = Genome::seeded(&mut self.rng, &mut self.innovations);
        for gene in &mut g.genes {
            if gene.from < INPUTS as u32 {
                let base = match gene.from {
                    0 => 2.4,
                    1 => 1.4,
                    2 => -2.8,
                    3 => -0.7,
                    4 => 0.7,
                    5 => 0.15,
                    _ => 0.0,
                };
                gene.weight = base + self.rng.signed() * 0.12;
            }
        }
        if self.rng.index(4) == 0 {
            g.add_node(&mut self.rng, &mut self.innovations);
        }
        g
    }
}
impl World {
    pub(super) fn init_wildlife(&mut self) {
        let mut w = Wildlife {
            rng: Rng(self.seed ^ 0x57494c444c494645),
            ..Wildlife::default()
        };
        let (bx, by) = {
            let (min_x, max_x, min_y, max_y) = self.map.spec.wild_bounds();
            ((min_x, max_x), (min_y, max_y))
        };
        // Group actual berry/timber resources into small groves; stocks and positions stay intact.
        for p in self.patches.iter().filter(|p| [0, 1].contains(&p.kind)) {
            if let Some(f) = w
                .forests
                .iter_mut()
                .find(|f| dist(f.x, f.y, p.x, p.y) < 125.0)
            {
                f.radius = f.radius.max(dist(f.x, f.y, p.x, p.y) + 12.0).min(140.0);
            } else {
                w.forests.push(Forest {
                    x: p.x,
                    y: p.y,
                    radius: 90.0,
                });
            }
        }
        for f in 0..w.forests.len() {
            let (x, y) = (w.forests[f].x, w.forests[f].y);
            for j in 0..5 {
                let angle = j as f32 * std::f32::consts::TAU / 5.0;
                w.grass.push(Grass {
                    forest: f as u32,
                    x: (x + angle.cos() * 45.0).clamp(bx.0, bx.1),
                    y: (y + angle.sin() * 45.0).clamp(by.0, by.1),
                    stock: 60,
                    capacity: 60,
                    growth: 0.0,
                });
            }
            let frontier = self.map.spec.size != MapSize::Classic;
            for (j, &species) in founder_species(frontier, f).iter().enumerate() {
                let genome = w.seed_genome();
                let angle = w.rng.unit() * std::f32::consts::TAU;
                let radius = w.rng.unit() * 50.0;
                let position = (
                    (x + angle.cos() * radius).clamp(bx.0, bx.1),
                    (y + angle.sin() * radius).clamp(by.0, by.1),
                );
                w.add_founder(species, j % 2 == 0, f, position, angle, self.tick, genome);
            }
        }
        w.ledger.initial_grass = w.grass.iter().map(|g| g.stock).sum();
        w.ledger.initial_animals = w.animals.len() as u64;
        self.wildlife = w;
        self.sync_wildlife_spawn_habitats();
    }

    pub(super) fn plant_wildlife_grove(&mut self, x: f32, y: f32) {
        if self
            .wildlife
            .forests
            .iter()
            .any(|f| dist(f.x, f.y, x, y) <= f.radius)
        {
            self.sync_wildlife_spawn_habitats();
            return;
        }
        let forest = self.wildlife.forests.len() as u32;
        self.wildlife.forests.push(Forest { x, y, radius: 90.0 });
        for j in 0..5 {
            let angle = j as f32 * std::f32::consts::TAU / 5.0;
            self.wildlife.grass.push(Grass {
                forest,
                x: x + angle.cos() * 45.0,
                y: y + angle.sin() * 45.0,
                stock: 60,
                capacity: 60,
                growth: 0.0,
            });
            self.wildlife.ledger.initial_grass += 60;
        }
        self.sync_wildlife_spawn_habitats();
    }

    pub(super) fn sync_wildlife_spawn_habitats(&mut self) {
        let forests = &self.wildlife.forests;
        let clearings = &self.urban.cleared;
        let cleared = |x, y| {
            clearings
                .iter()
                .any(|&(px, py, radius)| dist(x, y, px, py) <= radius)
        };
        let timber: Vec<_> = self.patches.iter().filter(|p| p.kind == 1).collect();
        let viable: Vec<_> = forests
            .iter()
            .map(|forest| {
                if cleared(forest.x, forest.y) {
                    return false;
                }
                let mut has_timber_site = false;
                for p in timber
                    .iter()
                    .filter(|p| dist(p.x, p.y, forest.x, forest.y) <= forest.radius)
                {
                    has_timber_site = true;
                    if p.stock > 0 && !cleared(p.x, p.y) {
                        return true;
                    }
                }
                // Some original groves are berry woodland without a timber site.
                !has_timber_site
            })
            .collect();
        let points = &mut self.wildlife.spawns.points;
        points.retain(|p| match p.anchor {
            SpawnAnchor::Grove => viable.get(p.forest).copied().unwrap_or(false),
            SpawnAnchor::Mountain { x, y } | SpawnAnchor::Shore { x, y } => !cleared(x, y),
        });
        let mut active = vec![[false; SPECIES.len()]; forests.len()];
        for p in points.iter().filter(|p| p.anchor == SpawnAnchor::Grove) {
            if let Some(species) = active[p.forest].get_mut(p.species as usize) {
                *species = true;
            }
        }
        let frontier = self.map.spec.size != MapSize::Classic;
        for (forest, &alive) in viable.iter().enumerate() {
            if !alive {
                continue;
            }
            let species = founder_species(frontier, forest);
            for id in 0..SPECIES.len() as u8 {
                if species.contains(&id) && !active[forest][id as usize] {
                    points.push(SpawnPoint {
                        anchor: SpawnAnchor::Grove,
                        forest,
                        species: id,
                        vacant_since: None,
                    });
                }
            }
        }
    }

    pub(super) fn init_terrain_spawn_points(&mut self) {
        if self.wildlife.forests.is_empty() {
            return;
        }
        let (min_x, max_x, min_y, max_y) = self.map.spec.wild_bounds();
        let frontier = self.map.spec.size != MapSize::Classic;
        let mut mountains = Vec::<(f32, f32)>::new();
        let mut shores = Vec::<(f32, f32)>::new();

        if frontier {
            const DIRECTIONS: [(f32, f32); 4] = [(1.0, 0.0), (0.0, 1.0), (-1.0, 0.0), (0.0, -1.0)];
            let local = self.wildlife.forests.iter().flat_map(|grove| {
                [0.0, 96.0, 192.0, 288.0]
                    .into_iter()
                    .flat_map(move |distance| {
                        DIRECTIONS.into_iter().map(move |(dx, dy)| {
                            (
                                (grove.x + dx * distance).clamp(min_x, max_x),
                                (grove.y + dy * distance).clamp(min_y, max_y),
                                false,
                            )
                        })
                    })
            });
            // If the groves are far from a terrain feature, look along the
            // generator's terrain lattice for one representative site.
            let fallback = (48..max_y as u32).step_by(48).flat_map(|y| {
                (48..max_x as u32)
                    .step_by(48)
                    .map(move |x| (x as f32, y as f32, true))
            });
            let width = self.map.spec.width();
            let height = self.map.spec.height();
            for (x, y, fallback) in local.chain(fallback) {
                if fallback && !mountains.is_empty() && !shores.is_empty() {
                    break;
                }
                let terrain = self.map.terrain_at(x, y);
                if terrain == Terrain::Rock
                    && (!fallback || mountains.is_empty())
                    && mountains
                        .iter()
                        .all(|&(px, py)| dist(x, y, px, py) >= 300.0)
                {
                    mountains.push((x, y));
                }
                if terrain != Terrain::Water
                    && (!fallback || shores.is_empty())
                    && shores.iter().all(|&(px, py)| dist(x, y, px, py) >= 300.0)
                    && DIRECTIONS.into_iter().any(|(dx, dy)| {
                        let px = x + dx * 64.0;
                        let py = y + dy * 64.0;
                        (0.0..width).contains(&px)
                            && (0.0..height).contains(&py)
                            && self.map.terrain_at(px, py) == Terrain::Water
                    })
                {
                    shores.push((x, y));
                }
            }
        } else {
            let hill = self.hill;
            for (dx, dy) in [(1.0, 0.0), (0.0, 1.0), (-1.0, 0.0), (0.0, -1.0)] {
                mountains.push((
                    (hill.x + dx * hill.radius * 0.8).clamp(min_x, max_x),
                    (hill.y + dy * hill.radius * 0.8).clamp(min_y, max_y),
                ));
            }
            for fraction in [0.2, 0.5, 0.8] {
                shores.push((max_x - 30.0, self.map.spec.height() * fraction));
            }
        }
        let mountain_species: &[u8] = if frontier { &[1, 3] } else { &[1] };
        let shore_species: &[u8] = if frontier { &[0, PHEASANT] } else { &[0] };
        let w = &mut self.wildlife;
        for ((x, y), anchor, species) in mountains
            .into_iter()
            .map(|(x, y)| ((x, y), SpawnAnchor::Mountain { x, y }, mountain_species))
            .chain(
                shores
                    .into_iter()
                    .map(|(x, y)| ((x, y), SpawnAnchor::Shore { x, y }, shore_species)),
            )
        {
            let (forest, _) = w
                .forests
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| dist(x, y, a.x, a.y).total_cmp(&dist(x, y, b.x, b.y)))
                .expect("a terrain spawn needs an existing grove");
            for &species in species {
                w.spawns.points.push(SpawnPoint {
                    anchor,
                    forest,
                    species,
                    vacant_since: None,
                });
            }
        }
    }

    fn replenish_wildlife(&mut self) {
        let w = &mut self.wildlife;
        if w.spawns.points.is_empty() || w.animals.len() + 2 > MAX_HISTORY {
            return;
        }

        let mut by_species: [Vec<(f32, f32)>; SPECIES.len()] = std::array::from_fn(|_| Vec::new());
        let mut by_forest = vec![0; w.forests.len()];
        let mut living = 0;
        for a in w.animals.iter().filter(|a| a.alive) {
            by_species[a.species as usize].push((a.x, a.y));
            by_forest[a.forest as usize] += 1;
            living += 1;
        }
        let (min_x, max_x, min_y, max_y) = self.map.spec.wild_bounds();
        for index in 0..w.spawns.points.len() {
            let point = &mut w.spawns.points[index];
            let den = &w.forests[point.forest];
            let (x, y, radius) = point.anchor.area(den);
            let occupied = by_species[point.species as usize]
                .iter()
                .any(|&(px, py)| dist(x, y, px, py) <= radius);
            if occupied {
                point.vacant_since = None;
                continue;
            }
            let since = *point.vacant_since.get_or_insert(self.tick);
            let delay_days = if SpeciesDef::of(point.species).predator {
                PREDATOR_VACANCY_DAYS
            } else {
                PREY_VACANCY_DAYS
            };
            if self.tick.saturating_sub(since) < delay_days * rest::DAY_TICKS
                || living + 2 > MAX_LIVING
                || w.animals.len() + 2 > MAX_HISTORY
                || by_forest[point.forest] + 2 > FOREST_CAP
            {
                continue;
            }

            // The square root distributes births uniformly over the disk.
            let mut positions = (0..32).filter_map(|_| {
                let angle = w.rng.unit() * std::f32::consts::TAU;
                let r = radius * w.rng.unit().sqrt();
                let px = x + r * angle.cos();
                let py = y + r * angle.sin();
                ((min_x..=max_x).contains(&px)
                    && (min_y..=max_y).contains(&py)
                    && !self
                        .urban
                        .cleared
                        .iter()
                        .any(|&(x, y, radius)| dist(px, py, x, y) <= radius)
                    && (self.map.spec.size == MapSize::Classic
                        || self.map.terrain_at(px, py) != Terrain::Water))
                    .then_some((px, py, angle))
            });
            let (Some(first), Some(second)) = (positions.next(), positions.next()) else {
                continue;
            };
            let (forest, species) = (point.forest, point.species);
            point.vacant_since = None;
            // Append immediately so overlapping points see this pair.
            let make = |w: &mut Wildlife, female, (px, py, angle)| {
                let genome = w.seed_genome();
                w.add_founder(species, female, forest, (px, py), angle, self.tick, genome);
            };
            make(w, true, first);
            make(w, false, second);
            by_species[species as usize].extend([(first.0, first.1), (second.0, second.1)]);
            by_forest[forest] += 2;
            w.spawns.spawned += 2;
            living += 2;
        }
    }

    fn animal_choices(&self, i: usize, stats: &ForestStats) -> Vec<Choice> {
        let w = &self.wildlife;
        let a = &w.animals[i];
        if !a.alive {
            return vec![];
        }
        let mut out = vec![];
        let mut add =
            |action: u8, target: u32, x: f32, y: f32, need: f32, benefit: f32, risk: f32| {
                let mut v = [0.0; INPUTS];
                v[0] = need;
                v[1] = benefit;
                v[2] = risk;
                v[3] = (dist(a.x, a.y, x, y) / 250.0).min(1.0);
                v[4] = if action == 5 { 1.0 } else { 0.0 };
                v[5] = if a.action == action { 1.0 } else { 0.0 };
                v[6] = if a.species == 0 { 1.0 } else { 0.0 };
                v[7] = a.species as f32;
                v[8] = a.hunger / 100.0;
                v[9] = a.energy / 100.0;
                v[10] = a.health / 100.0;
                v[11] = a.threat;
                v[12] = (stats.food.get(a.forest as usize).copied().unwrap_or(0) as f32 / 300.0)
                    .min(1.0);
                v[13] = (stats.pop.get(a.forest as usize).copied().unwrap_or(0) as f32
                    / FOREST_CAP as f32)
                    .min(1.0);
                v[14] = if self.is_night() { 1.0 } else { 0.0 };
                v[15] = if a.adult(self.tick) { 1.0 } else { 0.0 };
                let score = a.brain.as_ref().expect("rebuilt animal brain").evaluate(&v);
                out.push(Choice {
                    action,
                    target,
                    x,
                    y,
                    inputs: v,
                    score,
                });
            };
        let species = SpeciesDef::of(a.species);
        let portion = species.portion;
        let (min_x, max_x, min_y, max_y) = self.map.spec.wild_bounds();
        let den = &w.forests[a.forest as usize];
        if species.predator && a.hunger > 42.0 {
            let range = den.radius * 2.5;
            let is_prey = |b: &Animal| {
                !SpeciesDef::of(b.species).predator && (a.species == 3 || b.species != 1)
            };
            let nearby = |b: &Animal| dist(den.x, den.y, b.x, b.y) <= range;
            let exposed = |b: &Animal| {
                let grove = &w.forests[b.forest as usize];
                // Dense cover conceals small prey groups. Larger groups draw
                // attention, and foragers outside the grove are exposed.
                stats.species_pop[b.forest as usize][b.species as usize] > 2
                    || dist(grove.x, grove.y, b.x, b.y) > grove.radius
            };
            if let Some((id, b)) = self
                .carcasses
                .iter()
                .filter(|(_, meat)| **meat >= portion)
                .map(|(&id, _)| (id, &w.animals[id as usize]))
                .filter(|(_, b)| nearby(b))
                .min_by(|(_, b), (_, c)| {
                    dist(a.x, a.y, b.x, b.y).total_cmp(&dist(a.x, a.y, c.x, c.y))
                })
            {
                add(7, id, b.x, b.y, a.hunger / 60.0, 1.0, 0.0);
            } else if let Some((id, b)) = stats.members[a.forest as usize]
                .iter()
                .copied()
                .map(|id| (id, &w.animals[id]))
                .filter(|(_, b)| is_prey(b) && nearby(b) && exposed(b))
                .min_by(|(_, b), (_, c)| {
                    dist(a.x, a.y, b.x, b.y).total_cmp(&dist(a.x, a.y, c.x, c.y))
                })
            {
                add(6, id as u32, b.x, b.y, a.hunger / 55.0, 0.9, 0.06);
            } else if let Some((next, f)) = w
                .forests
                .iter()
                .enumerate()
                .filter(|(id, _)| *id != a.forest as usize && stats.pop[*id] < FOREST_CAP)
                .filter(|(id, _)| {
                    stats.species_pop[*id][a.species as usize] < 3
                        && (stats.carrion[*id] >= portion * 2
                            || stats.members[*id]
                                .iter()
                                .map(|&j| &w.animals[j])
                                .filter(|b| is_prey(b) && exposed(b))
                                .take(2)
                                .count()
                                == 2)
                })
                .min_by(|(_, f), (_, g)| {
                    dist(a.x, a.y, f.x, f.y).total_cmp(&dist(a.x, a.y, g.x, g.y))
                })
            {
                add(8, next as u32, f.x, f.y, a.hunger / 46.0, 0.85, 0.0);
            }
        }
        if !species.predator {
            if let Some((id, g)) = stats
                .grass_idx
                .get(a.forest as usize)
                .into_iter()
                .flatten()
                .filter_map(|&id| {
                    let g = &w.grass[id];
                    (g.stock >= portion && (a.hunger > 20.0 || a.energy < 65.0)).then_some((id, g))
                })
                .min_by(|(_, a1), (_, b)| {
                    dist(a.x, a.y, a1.x, a1.y).total_cmp(&dist(a.x, a.y, b.x, b.y))
                })
            {
                add(
                    0,
                    id as u32,
                    g.x,
                    g.y,
                    a.hunger / 65.0 + (100.0 - a.energy) / 150.0,
                    0.65,
                    a.threat * 0.5,
                );
            }
            if let Some(p) = self
                .patches
                .iter()
                .filter(|p| {
                    p.kind == 0
                        && (a.hunger > 20.0 || a.energy < 65.0)
                        && p.stock >= portion
                        && dist(
                            p.x,
                            p.y,
                            w.forests[a.forest as usize].x,
                            w.forests[a.forest as usize].y,
                        ) < 160.0
                })
                .min_by(|p, q| dist(a.x, a.y, p.x, p.y).total_cmp(&dist(a.x, a.y, q.x, q.y)))
            {
                add(1, p.id, p.x, p.y, a.hunger / 75.0, 0.55, a.threat * 0.6);
            }
        }
        if a.threat > 0.0 {
            let dx = a.x - a.threat_x;
            let dy = a.y - a.threat_y;
            let d = (dx * dx + dy * dy).sqrt().max(0.01);
            add(
                2,
                NONE,
                (a.x + dx / d * 80.0).clamp(min_x, max_x),
                (a.y + dy / d * 80.0).clamp(min_y, max_y),
                a.threat * 1.7,
                0.75,
                0.0,
            );
        }
        add(
            3,
            NONE,
            a.x,
            a.y,
            if species.predator {
                (100.0 - a.energy) / 75.0
            } else if a.hunger < 35.0 {
                0.35
            } else {
                0.05
            },
            0.15,
            a.threat,
        );
        let f = &w.forests[a.forest as usize];
        let angle = (i as f32 * 2.399963 + self.tick as f32 / 180.0).sin() * std::f32::consts::PI;
        let roam = f.radius * if species.predator { 1.9 } else { 0.7 };
        add(
            4,
            NONE,
            (f.x + angle.cos() * roam).clamp(min_x, max_x),
            (f.y + angle.sin() * roam).clamp(min_y, max_y),
            0.2,
            0.2,
            a.threat * 0.5,
        );
        if w.fertile(i, self.tick)
            && stats.food.get(a.forest as usize).copied().unwrap_or(0) >= 30
            && stats.pop.get(a.forest as usize).copied().unwrap_or(0) < FOREST_CAP
            && a.threat < 0.25
            && stats.living < MAX_LIVING
        {
            if let Some((j, b)) = stats
                .members
                .get(a.forest as usize)
                .into_iter()
                .flatten()
                .map(|&j| (j, &w.animals[j]))
                .filter(|(j, _)| w.compatible(i, *j, self.tick))
                .min_by(|(_, b), (_, c)| {
                    dist(a.x, a.y, b.x, b.y).total_cmp(&dist(a.x, a.y, c.x, c.y))
                })
            {
                add(5, j as u32, b.x, b.y, 0.65, 0.55, a.threat);
            }
        }
        out
    }
    pub(super) fn wildlife_step(&mut self) {
        if self.wildlife.forests.is_empty() {
            return;
        }
        let daylight = if self.is_night() { 0.3 } else { 1.0 };
        for g in &mut self.wildlife.grass {
            if g.stock < g.capacity {
                g.growth += 0.02 * DT as f64 * daylight;
                if g.growth >= 1.0 {
                    let q = (g.growth as u64).min(g.capacity - g.stock);
                    g.stock += q;
                    g.growth -= q as f64;
                    self.wildlife.ledger.grown_grass += q;
                }
            }
        }
        if self.tick % 10 == 0 {
            let mut humans: BTreeMap<(i32, i32), Vec<(f32, f32)>> = BTreeMap::new();
            for i in 0..self.agents.len() {
                if self.agents.alive[i] {
                    humans
                        .entry((
                            (self.agents.x[i] / 96.0) as i32,
                            (self.agents.y[i] / 96.0) as i32,
                        ))
                        .or_default()
                        .push((self.agents.x[i], self.agents.y[i]));
                }
            }
            for a in &mut self.wildlife.animals {
                if !a.alive {
                    continue;
                }
                a.threat = 0.0;
                let cell = ((a.x / 96.0) as i32, (a.y / 96.0) as i32);
                let mut nearest = 70.0;
                for dx in -1..=1 {
                    for dy in -1..=1 {
                        if let Some(points) = humans.get(&(cell.0 + dx, cell.1 + dy)) {
                            for &(x, y) in points {
                                let d = dist(a.x, a.y, x, y);
                                if d < nearest {
                                    nearest = d;
                                    a.threat_x = x;
                                    a.threat_y = y;
                                }
                            }
                        }
                    }
                }
                a.threat = (1.0 - nearest / 70.0).max(0.0);
            }
            let pursuers: Vec<_> = self
                .wildlife
                .animals
                .iter()
                .filter(|a| a.alive && a.action == 6)
                .map(|a| (a.target as usize, a.x, a.y))
                .collect();
            for (target, x, y) in pursuers {
                if let Some(prey) = self
                    .wildlife
                    .animals
                    .get_mut(target)
                    .filter(|a| a.alive && a.species == PHEASANT)
                {
                    let threat = (1.0 - dist(x, y, prey.x, prey.y) / 18.0).max(0.0);
                    if threat > prey.threat {
                        prey.threat = threat;
                        prey.threat_x = x;
                        prey.threat_y = y;
                    }
                }
            }
            let stats = ForestStats::build(&self.wildlife, &self.carcasses);
            for i in 0..self.wildlife.animals.len() {
                if !self.wildlife.animals[i].alive {
                    continue;
                }
                let choices = self.animal_choices(i, &stats);
                if let Some(c) = choices.iter().max_by(|a, b| a.score.total_cmp(&b.score)) {
                    let a = &mut self.wildlife.animals[i];
                    if a.action != c.action || a.target != c.target {
                        a.timer = 0.0;
                    }
                    a.action = c.action;
                    a.target = c.target;
                    a.goal_x = c.x;
                    a.goal_y = c.y;
                }
            }
        }
        let n = self.wildlife.animals.len();
        let generated = self.map.spec.size != MapSize::Classic;
        for i in 0..n {
            if !self.wildlife.animals[i].alive {
                continue;
            }
            let a = &mut self.wildlife.animals[i];
            a.hunger = (a.hunger + DT * SpeciesDef::of(a.species).hunger_rate).min(120.0);
            a.energy = (a.energy - DT * if a.action == 3 { 0.005 } else { 0.015 }).max(0.0);
            if a.hunger > 95.0 || a.energy <= 0.0 {
                a.health = (a.health - DT * 0.6).max(0.0);
            } else if a.hunger < 40.0 {
                a.health = (a.health + DT * 0.03).min(100.0);
            }
            if a.health <= 0.0 || a.age(self.tick) > SpeciesDef::of(a.species).lifespan_days {
                a.alive = false;
                a.pregnancy = None;
                self.wildlife.ledger.deaths += 1;
                let species = SpeciesDef::of(a.species);
                if generated && !species.predator {
                    self.carcasses.insert(i as u32, species.portion * 4);
                }
                continue;
            }
            let moving_target = if [5, 6].contains(&a.action) {
                Some(a.target as usize)
            } else {
                None
            };
            if let Some(target) = moving_target {
                if let Some(b) = self.wildlife.animals.get(target) {
                    let den = &self.wildlife.forests[self.wildlife.animals[i].forest as usize];
                    let (x, y) = if self.wildlife.animals[i].action == 6
                        && (!b.alive || dist(den.x, den.y, b.x, b.y) > den.radius * 2.5)
                    {
                        (den.x, den.y)
                    } else {
                        (b.x, b.y)
                    };
                    let a = &mut self.wildlife.animals[i];
                    a.goal_x = x;
                    a.goal_y = y;
                }
            }
            let (ax, ay, gx, gy, step) = {
                let a = &self.wildlife.animals[i];
                let d = dist(a.x, a.y, a.goal_x, a.goal_y);
                if d <= 5.0 {
                    (a.x, a.y, a.goal_x, a.goal_y, 0.0)
                } else {
                    let speed = SpeciesDef::of(a.species).speed
                        * match (a.action, a.species) {
                            (6, 2 | 3) => 1.35,
                            (2, _) => 1.6,
                            _ => 1.0,
                        }
                        * if a.adult(self.tick) { 1.0 } else { 0.6 };
                    (a.x, a.y, a.goal_x, a.goal_y, (speed * DT).min(d - 4.0))
                }
            };
            if step > 0.0 {
                let d = dist(ax, ay, gx, gy);
                let dx = (gx - ax) / d * step;
                let dy = (gy - ay) / d * step;
                // Animals refuse open water (rivers stay fordable). If the
                // direct step is wet, slide along whichever shore axis is dry.
                let (nx, ny) = if self.passable(ax + dx, ay + dy) {
                    (ax + dx, ay + dy)
                } else if self.passable(ax + dx, ay) {
                    (ax + dx, ay)
                } else if self.passable(ax, ay + dy) {
                    (ax, ay + dy)
                } else {
                    (ax, ay)
                };
                let a = &mut self.wildlife.animals[i];
                a.heading = (gy - a.y).atan2(gx - a.x);
                a.x = nx;
                a.y = ny;
                a.energy = (a.energy - step * 0.002).max(0.0);
                continue;
            }
            let a = &mut self.wildlife.animals[i];
            a.timer += DT;
            let action = a.action;
            let target = a.target as usize;
            if action == 8 && target < self.wildlife.forests.len() {
                a.forest = target as u32;
                a.action = 3;
                a.target = NONE;
                a.timer = 0.0;
            } else if action == 3 && SpeciesDef::of(a.species).predator {
                a.energy = (a.energy + DT * 0.4).min(100.0);
            } else if action == 6 && a.timer >= 1.0 && target < n {
                let species = SpeciesDef::of(a.species);
                let maturity =
                    (a.age(self.tick) as f32 / species.adult_days as f32).clamp(0.1, 1.0);
                let damage = 36.0 * maturity * (a.health / 100.0) * (a.energy / 100.0);
                a.timer = 0.0;
                let a_position = (a.x, a.y);
                let prey = &mut self.wildlife.animals[target];
                if prey.alive
                    && !SpeciesDef::of(prey.species).predator
                    && dist(a_position.0, a_position.1, prey.x, prey.y) <= 5.0
                {
                    prey.health = (prey.health - damage).max(0.0);
                    if prey.health == 0.0 {
                        prey.alive = false;
                        prey.pregnancy = None;
                        self.wildlife.ledger.deaths += 1;
                        self.carcasses
                            .insert(target as u32, SpeciesDef::of(prey.species).portion * 4);
                    }
                }
            } else if action == 7 && a.timer >= 2.0 {
                let portion = SpeciesDef::of(a.species).portion;
                if let Some(stock) = self
                    .carcasses
                    .get_mut(&(target as u32))
                    .filter(|q| **q >= portion)
                {
                    *stock -= portion;
                    a.hunger = (a.hunger - 32.0).max(0.0);
                    a.energy = (a.energy + 18.0).min(100.0);
                }
                a.timer = 0.0;
            } else if [0, 1].contains(&action)
                && a.timer >= 2.0
                && (a.hunger > 20.0 || a.energy < 65.0)
            {
                let q = SpeciesDef::of(a.species).portion;
                let eaten = if action == 0 {
                    if let Some(g) = self.wildlife.grass.get_mut(target) {
                        if g.stock >= q {
                            g.stock -= q;
                            self.wildlife.ledger.eaten_grass += q;
                            true
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else if let Some(p) = self.patches.get_mut(target) {
                    if p.kind == 0 && p.stock >= q {
                        p.stock -= q;
                        self.ledger.consumed[0] += q;
                        self.wildlife.ledger.eaten_berries += q;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                };
                if eaten {
                    a.hunger = (a.hunger - 32.0).max(0.0);
                    a.energy = (a.energy + 18.0).min(100.0);
                }
                a.timer = 0.0;
            } else if action == 5
                && a.timer >= 5.0
                && target < n
                && self.wildlife.compatible(i, target, self.tick)
                && self.wildlife.food(self.wildlife.animals[i].forest) >= 30
                && self.wildlife.population(self.wildlife.animals[i].forest) < FOREST_CAP
                && dist(
                    self.wildlife.animals[i].x,
                    self.wildlife.animals[i].y,
                    self.wildlife.animals[target].x,
                    self.wildlife.animals[target].y,
                ) < 8.0
            {
                let (mother, father) = if self.wildlife.animals[i].female {
                    (i, target)
                } else {
                    (target, i)
                };
                let days =
                    SpeciesDef::of(self.wildlife.animals[mother].species).gestation_days as u64;
                self.wildlife.animals[mother].pregnancy = Some(AnimalPregnancy {
                    father: father as u32,
                    due: self.tick + days * rest::DAY_TICKS,
                });
                for id in [mother, father] {
                    let a = &mut self.wildlife.animals[id];
                    a.energy -= 8.0;
                    a.last_mated = self.tick.max(1);
                    a.timer = 0.0;
                }
                self.wildlife.ledger.conceptions += 1;
            }
        }
        for mother in 0..n {
            if self.wildlife.animals[mother].alive
                && self.wildlife.animals[mother]
                    .pregnancy
                    .as_ref()
                    .is_some_and(|p| self.tick >= p.due)
            {
                self.animal_birth(mother);
            }
        }
        if self.tick % SPAWN_CHECK_TICKS == 0 {
            self.sync_wildlife_spawn_habitats();
            self.replenish_wildlife();
        }
    }
    fn animal_birth(&mut self, mother: usize) {
        let (min_x, max_x, _, _) = self.map.spec.wild_bounds();
        let w = &mut self.wildlife;
        let Some(p) = w.animals[mother].pregnancy.clone() else {
            return;
        };
        if self.tick < p.due || !w.animals[mother].alive {
            return;
        }
        let count = SpeciesDef::of(w.animals[mother].species).litter as usize;
        let forest = w.animals[mother].forest;
        if w.animals[mother].hunger >= 50.0
            || w.animals[mother].energy < 30.0
            || w.food(forest) < count as u64 * 6
            || w.population(forest) + count > FOREST_CAP
            || w.animals.iter().filter(|a| a.alive).count() + count > MAX_LIVING
            || w.animals.len() + count > MAX_HISTORY
        {
            if self.tick > p.due + 7 * rest::DAY_TICKS {
                w.animals[mother].pregnancy = None;
                w.ledger.lost_pregnancies += 1;
            }
            return;
        }
        for j in 0..count {
            let mut genome = Genome::crossover(
                &w.animals[mother].genome,
                &w.animals[p.father as usize].genome,
                &mut w.rng,
            );
            genome.mutate_weights(&mut w.rng, 0.15);
            if w.rng.index(10) == 0 {
                genome.add_node(&mut w.rng, &mut w.innovations);
            }
            let a = &w.animals[mother];
            let child = Animal {
                species: a.species,
                female: w.rng.index(2) == 0,
                forest,
                x: (a.x + j as f32 * 3.0).clamp(min_x, max_x),
                y: a.y,
                heading: a.heading,
                alive: true,
                health: 100.0,
                hunger: 20.0,
                energy: 5.0,
                born: self.tick,
                initial_age: 0,
                mother: mother as u32,
                father: p.father,
                pregnancy: None,
                last_mated: 0,
                action: 3,
                target: NONE,
                goal_x: a.x,
                goal_y: a.y,
                timer: 0.0,
                threat: 0.0,
                threat_x: a.x,
                threat_y: a.y,
                brain: Some(genome.compile().expect("valid offspring brain")),
                genome,
            };
            w.animals.push(child);
            w.ledger.births += 1;
        }
        w.animals[mother].energy -= count as f32 * 15.0;
        w.animals[mother].pregnancy = None;
        w.animals[mother].last_mated = self.tick;
    }
    pub fn living_wildlife_by_species(&self) -> BTreeMap<&'static str, usize> {
        let mut counts: BTreeMap<_, _> = SPECIES.iter().map(|species| (species.name, 0)).collect();
        for a in self.wildlife.animals.iter().filter(|a| a.alive) {
            *counts.entry(SpeciesDef::of(a.species).name).or_default() += 1;
        }
        counts
    }

    pub fn animal_count(&self) -> usize {
        self.wildlife.animals.len()
    }
    pub(super) fn wildlife_summary(&self) -> serde_json::Value {
        let w = &self.wildlife;
        let stock: u64 = w.grass.iter().map(|g| g.stock).sum();
        serde_json::json!({"forests":w.forests,"grass":w.grass,"rabbits":w.animals.iter().filter(|a|a.alive&&a.species==0).count(),"deer":w.animals.iter().filter(|a|a.alive&&a.species==1).count(),"births":w.ledger.births,"deaths":w.ledger.deaths,"conceptions":w.ledger.conceptions,"grass_account":{"initial":w.ledger.initial_grass,"grown":w.ledger.grown_grass,"eaten":w.ledger.eaten_grass,"stock":stock,"error":w.ledger.initial_grass as i128+w.ledger.grown_grass as i128-w.ledger.eaten_grass as i128-stock as i128},"berries_eaten":w.ledger.eaten_berries,"animals":w.animals.iter().enumerate().filter(|(_,a)|a.alive).map(|(id,a)|serde_json::json!({"id":id,"species":a.species,"x":a.x,"y":a.y,"heading":a.heading,"action":a.action,"young":!a.adult(self.tick)})).collect::<Vec<_>>()})
    }
    pub(super) fn carcass_sites(&self) -> serde_json::Value {
        let sites: Vec<_> = self
            .carcasses
            .iter()
            .filter(|(_, portions)| **portions > 0)
            .filter_map(|(&id, &portions)| {
                self.wildlife.animals.get(id as usize).map(|animal| {
                    serde_json::json!({
                        "id": id,
                        "x": animal.x,
                        "y": animal.y,
                        "species": SpeciesDef::of(animal.species).name,
                        "portions": portions,
                    })
                })
            })
            .collect();
        serde_json::json!(sites)
    }

    pub fn animal_trace(&self, i: usize) -> serde_json::Value {
        let Some(a) = self.wildlife.animals.get(i) else {
            return serde_json::Value::Null;
        };
        serde_json::json!({"x":a.x,"y":a.y,"id":i,"species":SpeciesDef::of(a.species).name,"predator":SpeciesDef::of(a.species).predator,"gender":if a.female{"Female"}else{"Male"},"age_days":a.age(self.tick),"adult":a.adult(self.tick),"alive":a.alive,"health":a.health,"hunger":a.hunger,"energy":a.energy,"mother":if a.mother==NONE{None}else{Some(a.mother)},"father":if a.father==NONE{None}else{Some(a.father)},"pregnancy_days_left":a.pregnancy.as_ref().map(|p|p.due.saturating_sub(self.tick) as f32/rest::DAY_TICKS as f32),"forest":a.forest,"action":NAMES[a.action as usize],"generation":a.genome.generation,"genome_nodes":a.genome.nodes.len(),"genome_genes":a.genome.genes.len(),"sensors":ANIMAL_SENSORS,"choices":self.animal_choices(i,&ForestStats::build(&self.wildlife, &self.carcasses)).iter().map(|c|serde_json::json!({"action":NAMES[c.action as usize],"score":c.score,"inputs":c.inputs.to_vec()})).collect::<Vec<_>>()})
    }
    pub(super) fn validate_wildlife(&self) -> Result<(), String> {
        let w = &self.wildlife;
        if w.spears
            .iter()
            .chain(&w.bows)
            .any(|&i| i as usize >= self.agents.len())
        {
            return Err("Invalid hunting weapon owner".into());
        }
        let grass: u64 = w.grass.iter().map(|g| g.stock).sum();
        if w.ledger.initial_grass + w.ledger.grown_grass != grass + w.ledger.eaten_grass
            || w.ledger.initial_animals + w.ledger.births + w.spawns.spawned
                != w.animals.len() as u64
            || w.animals.iter().filter(|a| !a.alive).count() as u64 != w.ledger.deaths
        {
            return Err("Wildlife conservation error".into());
        }
        for point in &w.spawns.points {
            if point.forest >= w.forests.len()
                || point.species as usize >= SPECIES.len()
                || match point.anchor {
                    SpawnAnchor::Grove => false,
                    SpawnAnchor::Mountain { x, y } | SpawnAnchor::Shore { x, y } => {
                        !(0.0..=self.map.spec.width()).contains(&x)
                            || !(0.0..=self.map.spec.height()).contains(&y)
                    }
                }
                || point.vacant_since.is_some_and(|tick| tick > self.tick)
            {
                return Err("Invalid wildlife spawn point".into());
            }
        }
        for (&id, &meat) in &self.carcasses {
            if !w
                .animals
                .get(id as usize)
                .is_some_and(|a| !a.alive && meat <= SpeciesDef::of(a.species).portion * 4)
            {
                return Err("Invalid wildlife carcass".into());
            }
        }
        for g in &w.grass {
            if g.stock > g.capacity || g.forest as usize >= w.forests.len() {
                return Err("Invalid grass patch".into());
            }
        }
        for (i, a) in w.animals.iter().enumerate() {
            if a.species as usize >= SPECIES.len()
                || a.action >= NAMES.len() as u8
                || a.forest as usize >= w.forests.len()
                || ![a.x, a.y, a.health, a.hunger, a.energy]
                    .iter()
                    .all(|x| x.is_finite())
                || a.mother != NONE && a.mother as usize >= i
                || a.father != NONE && a.father as usize >= i
                || a.pregnancy
                    .as_ref()
                    .is_some_and(|p| !a.female || p.father as usize >= w.animals.len())
            {
                return Err("Invalid animal".into());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn planting_depletion_regrowth_and_clearance_control_spawn_points() {
        let mut w = World::new(32, 42).unwrap();
        let forest = w.wildlife.forests.len();
        let patch = (200..1700)
            .step_by(100)
            .find_map(|y| {
                (200..2500).step_by(100).find_map(|x| {
                    let (x, y) = (x as f32, y as f32);
                    if w.wildlife
                        .forests
                        .iter()
                        .any(|f| dist(x, y, f.x, f.y) <= f.radius + 90.0)
                    {
                        return None;
                    }
                    w.add_resource(1, x, y, 30).ok().map(|id| id as usize)
                })
            })
            .expect("the fixture has open land for a new grove");
        assert_eq!(w.wildlife.forests.len(), forest + 1);
        let at_grove = |p: &&SpawnPoint| p.anchor == SpawnAnchor::Grove && p.forest == forest;
        assert_eq!(w.wildlife.spawns.points.iter().filter(at_grove).count(), 2);

        for p in w
            .wildlife
            .spawns
            .points
            .iter_mut()
            .filter(|p| p.forest == forest)
        {
            p.vacant_since = Some(0);
        }
        w.ledger.consumed[1] += w.patches[patch].stock;
        w.patches[patch].stock = 0;
        w.patches[patch].age = 0.0;
        w.tick = SPAWN_CHECK_TICKS - 1;
        w.step(false);
        assert_eq!(w.wildlife.spawns.points.iter().filter(at_grove).count(), 0);

        w.patches[patch].age = 60.0;
        w.patches[patch].growth = 0.999;
        w.tick += SPAWN_CHECK_TICKS - 1;
        w.step(false);
        assert_eq!(w.patches[patch].stock, 1);
        let active: Vec<_> = w.wildlife.spawns.points.iter().filter(at_grove).collect();
        assert_eq!(active.len(), 2);
        assert!(active
            .iter()
            .all(|point| point.vacant_since == Some(w.tick)));

        let (x, y) = (w.patches[patch].x, w.patches[patch].y);
        w.urban.cleared.push((x, y, 45.0));
        w.sync_wildlife_spawn_habitats();
        assert_eq!(w.wildlife.spawns.points.iter().filter(at_grove).count(), 0);
        assert!(w.add_resource(1, x, y, 30).is_err());
        w.check().unwrap();
    }

    #[test]
    fn terrain_spawn_points_follow_the_hill_and_stay_on_land() {
        let hill = Hill {
            x: 1300.0,
            y: 1250.0,
            radius: 210.0,
        };
        let mut world = World::new_with_hill(32, 42, hill).unwrap();
        world
            .wildlife
            .spawns
            .points
            .retain(|p| p.anchor != SpawnAnchor::Grove);
        assert_eq!(world.wildlife.spawns.points.len(), 7);
        for p in &world.wildlife.spawns.points {
            match p.anchor {
                SpawnAnchor::Mountain { x, y } => assert!(hill.clearance(x, y) < 0.0),
                SpawnAnchor::Shore { x, .. } => {
                    assert_eq!(x, world.map.spec.wild_bounds().1 - 30.0)
                }
                SpawnAnchor::Grove => unreachable!(),
            }
        }
        for a in &mut world.wildlife.animals {
            a.alive = false;
        }
        let initial = world.wildlife.animals.len();
        world.wildlife.ledger.deaths = initial as u64;
        world.replenish_wildlife();
        world.tick = PREY_VACANCY_DAYS * rest::DAY_TICKS;
        world.replenish_wildlife();
        assert_eq!(world.wildlife.animals.len(), initial + 14);
        for (pair, p) in world.wildlife.animals[initial..]
            .chunks_exact(2)
            .zip(&world.wildlife.spawns.points)
        {
            let (x, y, radius) = p.anchor.area(&world.wildlife.forests[p.forest]);
            for a in pair {
                assert_eq!(a.species, p.species);
                assert!(dist(x, y, a.x, a.y) <= radius);
                assert!(a.x <= world.map.spec.wild_bounds().1);
            }
        }
        world.check().unwrap();
    }

    #[test]
    fn generated_spawn_anchors_match_rock_and_water_edges() {
        let mut world = World::new_with_size(32, 42, Hill::default(), MapSize::X4).unwrap();
        let mut mountain = false;
        let mut shore = false;
        for point in &world.wildlife.spawns.points {
            match point.anchor {
                SpawnAnchor::Mountain { x, y } => {
                    mountain = true;
                    assert_eq!(world.map.terrain_at(x, y), Terrain::Rock);
                }
                SpawnAnchor::Shore { x, y } => {
                    shore = true;
                    assert_ne!(world.map.terrain_at(x, y), Terrain::Water);
                    assert!([(64.0, 0.0), (-64.0, 0.0), (0.0, 64.0), (0.0, -64.0)]
                        .iter()
                        .any(|(dx, dy)| world.map.terrain_at(x + dx, y + dy) == Terrain::Water));
                }
                SpawnAnchor::Grove => {}
            }
        }
        assert!(mountain && shore);
    }

    #[test]
    fn vacant_spawn_points_reset_and_replay_with_species_delays() {
        let mut world = World::new_with_size(32, 42, Hill::default(), MapSize::X4).unwrap();
        world.wildlife.spawns.points.retain(|p| {
            p.anchor == SpawnAnchor::Grove && p.forest == 0 && [0, 2].contains(&p.species)
        });
        assert_eq!(world.wildlife.spawns.points.len(), 2);
        let initial = world.wildlife.animals.len();
        for a in &mut world.wildlife.animals {
            a.alive = false;
        }
        world.wildlife.ledger.deaths = initial as u64;
        world.tick = 10;
        world.replenish_wildlife();

        let den = &world.wildlife.forests[0];
        let rabbit = &mut world.wildlife.animals[0];
        rabbit.alive = true;
        rabbit.x = den.x;
        rabbit.y = den.y;
        world.wildlife.ledger.deaths -= 1;
        world.tick = rest::DAY_TICKS;
        world.replenish_wildlife();
        assert_eq!(world.wildlife.spawns.points[0].vacant_since, None);
        assert_eq!(world.wildlife.spawns.points[1].vacant_since, Some(10));
        world.wildlife.animals[0].alive = false;
        world.wildlife.ledger.deaths += 1;
        world.replenish_wildlife();

        let path =
            std::env::temp_dir().join(format!("xianxia-spawn-{}.snapshot", std::process::id()));
        world.save(&path).unwrap();
        let mut restored = World::load(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        assert_eq!(world.wildlife.spawns, restored.wildlife.spawns);

        for world in [&mut world, &mut restored] {
            let prey_due = (1 + PREY_VACANCY_DAYS) * rest::DAY_TICKS;
            let predator_due = 10 + PREDATOR_VACANCY_DAYS * rest::DAY_TICKS;
            world.tick = prey_due - 1;
            world.replenish_wildlife();
            assert_eq!(world.wildlife.animals.len(), initial);
            world.tick = prey_due;
            world.replenish_wildlife();
            assert_eq!(world.wildlife.animals.len(), initial + 2);
            world.tick = predator_due - 1;
            world.replenish_wildlife();
            assert_eq!(world.wildlife.animals.len(), initial + 2);
            world.tick = predator_due;
            world.replenish_wildlife();
            assert_eq!(world.wildlife.animals.len(), initial + 4);

            let den = &world.wildlife.forests[0];
            let spawned = &world.wildlife.animals[initial..];
            for (a, species) in spawned.iter().zip([0, 0, 2, 2]) {
                assert_eq!(a.species, species);
                assert!(dist(a.x, a.y, den.x, den.y) <= den.radius);
            }
            assert_ne!((spawned[0].x, spawned[0].y), (spawned[1].x, spawned[1].y));
            assert_eq!(world.wildlife.spawns.spawned, 4);
            assert_eq!(world.wildlife.ledger.births, 0);
            world.check().unwrap();
        }
        assert_eq!(
            world.checkpoint_payload().unwrap(),
            restored.checkpoint_payload().unwrap()
        );
    }

    #[test]
    fn animals_do_not_enter_open_water() {
        let mut w = World::new_with_size(8, 7, Hill::default(), MapSize::X4).unwrap();
        if w.wildlife.animals.is_empty() {
            return;
        }
        let r = w.map.raster();
        let mut shore = None;
        'outer: for j in 1..r.rows - 1 {
            for i in 1..r.cols - 1 {
                if r.terrain[j * r.cols + i] == Terrain::Water as u8 {
                    continue;
                }
                for (di, dj) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
                    let n = ((j as i32 + dj) * r.cols as i32 + i as i32 + di) as usize;
                    if r.terrain[n] == Terrain::Water as u8 {
                        shore = Some((
                            (i as f32 + 0.5) * r.cell,
                            (j as f32 + 0.5) * r.cell,
                            ((i as f32 + 0.5) + di as f32 * 3.0) * r.cell,
                            ((j as f32 + 0.5) + dj as f32 * 3.0) * r.cell,
                        ));
                        break 'outer;
                    }
                }
            }
        }
        let Some((lx, ly, wx, wy)) = shore else { return };
        w.wildlife.animals[0].x = lx;
        w.wildlife.animals[0].y = ly;
        for _ in 0..120 {
            // Keep the goal pinned in the water; the animal may still be
            // redirected by its own choices, but it must never stand wet.
            w.wildlife.animals[0].goal_x = wx;
            w.wildlife.animals[0].goal_y = wy;
            w.step(false);
            assert_ne!(
                w.map.terrain_at(w.wildlife.animals[0].x, w.wildlife.animals[0].y),
                Terrain::Water,
                "animal entered water at ({},{})",
                w.wildlife.animals[0].x,
                w.wildlife.animals[0].y
            );
        }
    }
    #[test]
    fn groves_are_small_and_each_animal_has_an_independent_brain() {
        let w = World::new(12, 42).unwrap();
        assert!(w.wildlife.forests.len() > 3);
        assert!(w.wildlife.forests.iter().all(|f| f.radius <= 140.0));
        assert_eq!(w.wildlife.animals.len(), w.wildlife.forests.len() * 10);
        assert_ne!(w.wildlife.animals[0].genome, w.wildlife.animals[1].genome);
        assert!(w
            .wildlife
            .animals
            .iter()
            .all(|a| a.genome.compile().is_ok()));
        assert_eq!(w.stocks(), w.ledger.initial);
        w.check().unwrap();
    }
    #[test]
    fn feeding_uses_actual_grass_or_shared_berries_after_physical_arrival() {
        let mut w = World::new(2, 42).unwrap();
        let (x, y) = (w.wildlife.grass[0].x, w.wildlife.grass[0].y);
        let a = &mut w.wildlife.animals[0];
        a.x = x + 100.0;
        a.y = y;
        a.goal_x = x;
        a.goal_y = y;
        a.action = 0;
        a.target = 0;
        a.hunger = 80.0;
        w.tick = 1;
        let before = w.wildlife.grass[0].stock;
        w.wildlife_step();
        assert!((w.wildlife.animals[0].x - (x + 100.0)).abs() <= 2.21);
        assert_eq!(w.wildlife.grass[0].stock, before);
        let a = &mut w.wildlife.animals[0];
        a.x = x;
        a.y = y;
        a.timer = 2.0;
        w.tick = 2;
        w.wildlife_step();
        assert_eq!(w.wildlife.grass[0].stock, before - 1);
        let berries = w.patches.iter().position(|p| p.kind == 0).unwrap();
        let q = w.patches[berries].stock;
        let consumed = w.ledger.consumed[0];
        let a = &mut w.wildlife.animals[0];
        a.action = 1;
        a.target = berries as u32;
        a.x = w.patches[berries].x;
        a.y = w.patches[berries].y;
        a.goal_x = a.x;
        a.goal_y = a.y;
        a.timer = 2.0;
        a.hunger = 80.0;
        w.tick = 3;
        w.wildlife_step();
        assert_eq!(w.patches[berries].stock, q - 1);
        assert_eq!(w.ledger.consumed[0], consumed + 1);
        w.check().unwrap();
    }
    #[test]
    fn breeding_rejects_young_relatives_and_different_species() {
        let mut w = World::new(2, 42).unwrap();
        assert!(w.wildlife.compatible(0, 1, 1));
        w.wildlife.animals[1].initial_age = 0;
        assert!(!w.wildlife.compatible(0, 1, 1));
        w.wildlife.animals[1].initial_age = 120;
        w.wildlife.animals[1].species = 1;
        assert!(!w.wildlife.compatible(0, 1, 1));
        w.wildlife.animals[1].species = 0;
        w.wildlife.animals[1].mother = 0;
        assert!(!w.wildlife.compatible(0, 1, 1));
    }
    #[test]
    fn birth_waits_for_gestation_and_inherits_brains_without_creating_food() {
        let mut w = World::new(2, 42).unwrap();
        let n = w.wildlife.animals.len();
        let stocks = w.stocks();
        let grass = w.wildlife.grass.iter().map(|g| g.stock).sum::<u64>();
        w.wildlife.animals[0].pregnancy = Some(AnimalPregnancy {
            father: 1,
            due: 100,
        });
        w.tick = 99;
        w.animal_birth(0);
        assert_eq!(w.wildlife.animals.len(), n);
        w.tick = 100;
        w.animal_birth(0);
        assert_eq!(w.wildlife.animals.len(), n + 2);
        assert_eq!(w.wildlife.animals[0].energy, 45.0);
        for child in &w.wildlife.animals[n..] {
            assert_eq!(child.mother, 0);
            assert_eq!(child.father, 1);
            assert!(child.genome.generation > 0);
            assert!(child.genome.compile().is_ok());
            assert!(!child.adult(w.tick));
        }
        assert_eq!(w.stocks(), stocks);
        assert_eq!(w.wildlife.grass.iter().map(|g| g.stock).sum::<u64>(), grass);
        w.check().unwrap();
    }
    #[test]
    fn no_food_blocks_births_and_starvation_is_possible() {
        let mut w = World::new(2, 42).unwrap();
        w.wildlife.animals.truncate(2);
        w.wildlife.ledger.initial_animals = 2;
        for g in &mut w.wildlife.grass {
            w.wildlife.ledger.eaten_grass += g.stock;
            g.stock = 0;
            g.capacity = 0;
        }
        for p in w.patches.iter_mut().filter(|p| p.kind == 0) {
            w.ledger.consumed[0] += p.stock;
            p.stock = 0;
        }
        for a in &mut w.wildlife.animals {
            a.hunger = 100.0;
            a.energy = 0.0;
        }
        w.wildlife.animals[0].pregnancy = Some(AnimalPregnancy { father: 1, due: 1 });
        w.tick = 1;
        w.animal_birth(0);
        assert_eq!(w.wildlife.ledger.births, 0);
        for tick in 1..2000 {
            w.tick = tick;
            w.wildlife_step();
        }
        assert_eq!(w.wildlife.ledger.deaths, 2);
        assert_eq!(w.wildlife.ledger.births, 0);
        w.check().unwrap();
    }
    #[test]
    fn deer_have_longer_gestation_one_fawn_and_larger_meals() {
        let mut w = World::new(2, 42).unwrap();
        let mother = 6;
        let father = 7;
        assert!(w.wildlife.compatible(mother, father, 1));
        let (x, y) = (w.wildlife.animals[father].x, w.wildlife.animals[father].y);
        let a = &mut w.wildlife.animals[mother];
        a.x = x;
        a.y = y;
        a.goal_x = x;
        a.goal_y = y;
        a.action = 5;
        a.target = father as u32;
        a.timer = 5.0;
        w.tick = 1;
        w.wildlife_step();
        let due = w.wildlife.animals[mother].pregnancy.as_ref().unwrap().due;
        assert_eq!(due, 1 + 200 * rest::DAY_TICKS);
        let n = w.animal_count();
        w.tick = due;
        w.wildlife.animals[mother].energy = 75.0;
        w.animal_birth(mother);
        assert_eq!(w.animal_count(), n + 1);
        assert_eq!(w.wildlife.animals[n].species, 1);
        let (gx, gy) = (w.wildlife.grass[0].x, w.wildlife.grass[0].y);
        let stock = w.wildlife.grass[0].stock;
        let a = &mut w.wildlife.animals[father];
        a.x = gx;
        a.y = gy;
        a.goal_x = gx;
        a.goal_y = gy;
        a.action = 0;
        a.target = 0;
        a.timer = 2.0;
        a.hunger = 80.0;
        w.tick = due + 1;
        w.wildlife_step();
        assert_eq!(w.wildlife.grass[0].stock, stock - 3);
        w.check().unwrap();
    }
    #[test]
    fn a_hunt_creates_one_finite_carcass_and_preserves_it_in_a_checkpoint() {
        let mut world = World::new_with_size(32, 42, Hill::default(), MapSize::X4).unwrap();
        let predator = world
            .wildlife
            .animals
            .iter()
            .position(|a| a.species == 2)
            .unwrap();
        let prey = world
            .wildlife
            .animals
            .iter()
            .position(|a| a.species == 0)
            .unwrap();
        let den = &world.wildlife.forests[world.wildlife.animals[predator].forest as usize];
        let (x, y) = (den.x, den.y);
        let target = &mut world.wildlife.animals[prey];
        target.x = x;
        target.y = y;
        target.goal_x = x;
        target.goal_y = y;
        target.action = 3;
        target.health = 1.0;
        let hunter = &mut world.wildlife.animals[predator];
        hunter.x = x;
        hunter.y = y;
        hunter.action = 6;
        hunter.target = prey as u32;
        hunter.timer = 2.0;
        world.tick = 1;
        world.wildlife_step();
        assert!(!world.wildlife.animals[prey].alive);
        assert_eq!(world.carcasses.get(&(prey as u32)), Some(&4));

        let hunter = &mut world.wildlife.animals[predator];
        hunter.action = 7;
        hunter.timer = 2.0;
        world.wildlife_step();
        assert_eq!(world.carcasses.get(&(prey as u32)), Some(&3));
        world.check().unwrap();

        let path =
            std::env::temp_dir().join(format!("xianxia-carcass-{}.snapshot", std::process::id()));
        world.save(&path).unwrap();
        let mut loaded = World::load(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        assert_eq!(world.carcasses, loaded.carcasses);
        for _ in 0..20 {
            world.step(false);
            loaded.step(true);
        }
        assert_eq!(world.carcasses, loaded.carcasses);
        assert_eq!(
            postcard::to_allocvec(&world).unwrap(),
            postcard::to_allocvec(&loaded).unwrap()
        );
    }

    #[test]
    fn wildlife_and_pending_pregnancy_replay_exactly() {
        let mut w = World::new(2, 42).unwrap();
        w.wildlife.animals[0].pregnancy = Some(AnimalPregnancy { father: 1, due: 25 });
        let path =
            std::env::temp_dir().join(format!("xianxia-wildlife-{}.snapshot", std::process::id()));
        w.save(&path).unwrap();
        let mut loaded = World::load(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        for _ in 0..100 {
            w.step(false);
            loaded.step(true);
        }
        assert_eq!(
            postcard::to_allocvec(&w).unwrap(),
            postcard::to_allocvec(&loaded).unwrap()
        );
        assert!(loaded.wildlife.ledger.births >= 2);
        w.check().unwrap();
    }
}
