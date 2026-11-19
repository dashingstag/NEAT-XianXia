//! Food-limited woodland herbivores with species-specific candidate-scoring NEAT policies.
use super::*;
use std::collections::BTreeMap;
const MAX_LIVING: usize = 1024;
const MAX_HISTORY: usize = 4096;
const FOREST_CAP: usize = 40;
const NAMES: [&str; 6] = [
    "Graze grass",
    "Eat berries",
    "Flee",
    "Rest",
    "Explore",
    "Find mate",
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
    fn age(&self, tick: u64) -> u32 {
        self.initial_age
            .saturating_add((tick.saturating_sub(self.born) / rest::DAY_TICKS) as u32)
    }
    fn adult(&self, tick: u64) -> bool {
        self.age(tick) >= if self.species == 0 { 90 } else { 365 }
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
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Wildlife {
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
            forests: vec![],
            grass: vec![],
            animals: vec![],
            innovations: Innovations::default(),
            rng: Rng(0),
            ledger: EcologyLedger::default(),
        }
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
impl Wildlife {
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
                    >= if a.species == 0 { 45 } else { 365 } * rest::DAY_TICKS)
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
                    x: (x + angle.cos() * 45.0).clamp(10.0, 2650.0),
                    y: (y + angle.sin() * 45.0).clamp(10.0, 1890.0),
                    stock: 60,
                    capacity: 60,
                    growth: 0.0,
                });
            }
            for j in 0..10 {
                let species = if j < 6 { 0 } else { 1 };
                let genome = w.seed_genome();
                let angle = w.rng.unit() * std::f32::consts::TAU;
                let radius = w.rng.unit() * 50.0;
                w.animals.push(Animal {
                    species,
                    female: j % 2 == 0,
                    forest: f as u32,
                    x: (x + angle.cos() * radius).clamp(10.0, 2650.0),
                    y: (y + angle.sin() * radius).clamp(10.0, 1890.0),
                    heading: angle,
                    alive: true,
                    health: 100.0,
                    hunger: 15.0 + w.rng.unit() * 20.0,
                    energy: 75.0,
                    born: self.tick,
                    initial_age: if species == 0 { 120 } else { 600 },
                    mother: NONE,
                    father: NONE,
                    pregnancy: None,
                    last_mated: 0,
                    action: 4,
                    target: NONE,
                    goal_x: x,
                    goal_y: y,
                    timer: 0.0,
                    threat: 0.0,
                    threat_x: x,
                    threat_y: y,
                    brain: Some(genome.compile().expect("valid animal seed")),
                    genome,
                });
            }
        }
        w.ledger.initial_grass = w.grass.iter().map(|g| g.stock).sum();
        w.ledger.initial_animals = w.animals.len() as u64;
        self.wildlife = w;
    }
    fn animal_choices(&self, i: usize) -> Vec<Choice> {
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
                v[12] = (w.food(a.forest) as f32 / 300.0).min(1.0);
                v[13] = (w.population(a.forest) as f32 / FOREST_CAP as f32).min(1.0);
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
        let portion = if a.species == 0 { 1 } else { 3 };
        if let Some((id, g)) = w
            .grass
            .iter()
            .enumerate()
            .filter(|(_, g)| {
                g.forest == a.forest && g.stock >= portion && (a.hunger > 20.0 || a.energy < 65.0)
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
        if a.threat > 0.0 {
            let dx = a.x - a.threat_x;
            let dy = a.y - a.threat_y;
            let d = (dx * dx + dy * dy).sqrt().max(0.01);
            add(
                2,
                NONE,
                (a.x + dx / d * 80.0).clamp(10.0, 2650.0),
                (a.y + dy / d * 80.0).clamp(10.0, 1890.0),
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
            if a.hunger < 35.0 { 0.35 } else { 0.05 },
            0.15,
            a.threat,
        );
        let f = &w.forests[a.forest as usize];
        let angle = (i as f32 * 2.399963 + self.tick as f32 / 180.0).sin() * std::f32::consts::PI;
        add(
            4,
            NONE,
            (f.x + angle.cos() * f.radius * 0.7).clamp(10.0, 2650.0),
            (f.y + angle.sin() * f.radius * 0.7).clamp(10.0, 1890.0),
            0.2,
            0.2,
            a.threat * 0.5,
        );
        if w.fertile(i, self.tick)
            && w.food(a.forest) >= 30
            && w.population(a.forest) < FOREST_CAP
            && a.threat < 0.25
            && w.animals.iter().filter(|a| a.alive).count() < MAX_LIVING
        {
            if let Some((j, b)) = w
                .animals
                .iter()
                .enumerate()
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
            for i in 0..self.wildlife.animals.len() {
                if !self.wildlife.animals[i].alive {
                    continue;
                }
                let choices = self.animal_choices(i);
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
        for i in 0..n {
            if !self.wildlife.animals[i].alive {
                continue;
            }
            let a = &mut self.wildlife.animals[i];
            a.hunger = (a.hunger + DT * if a.species == 0 { 0.08 } else { 0.12 }).min(120.0);
            a.energy = (a.energy - DT * if a.action == 3 { 0.005 } else { 0.015 }).max(0.0);
            if a.hunger > 95.0 || a.energy <= 0.0 {
                a.health = (a.health - DT * 0.6).max(0.0);
            } else if a.hunger < 40.0 {
                a.health = (a.health + DT * 0.03).min(100.0);
            }
            if a.health <= 0.0 || a.age(self.tick) > if a.species == 0 { 2500 } else { 5000 } {
                a.alive = false;
                a.pregnancy = None;
                self.wildlife.ledger.deaths += 1;
                continue;
            }
            let mate_target = if a.action == 5 {
                Some(a.target as usize)
            } else {
                None
            };
            if let Some(target) = mate_target {
                if let Some(b) = self.wildlife.animals.get(target) {
                    let (x, y) = (b.x, b.y);
                    let a = &mut self.wildlife.animals[i];
                    a.goal_x = x;
                    a.goal_y = y;
                }
            }
            let a = &mut self.wildlife.animals[i];
            let d = dist(a.x, a.y, a.goal_x, a.goal_y);
            if d > 5.0 {
                let speed = if a.species == 0 { 22.0 } else { 28.0 }
                    * if a.action == 2 { 1.6 } else { 1.0 }
                    * if a.adult(self.tick) { 1.0 } else { 0.6 };
                let step = (speed * DT).min(d - 4.0);
                a.heading = (a.goal_y - a.y).atan2(a.goal_x - a.x);
                a.x += (a.goal_x - a.x) / d * step;
                a.y += (a.goal_y - a.y) / d * step;
                a.energy = (a.energy - step * 0.002).max(0.0);
                continue;
            }
            a.timer += DT;
            let action = a.action;
            let target = a.target as usize;
            if [0, 1].contains(&action) && a.timer >= 2.0 && (a.hunger > 20.0 || a.energy < 65.0) {
                let q = if a.species == 0 { 1 } else { 3 };
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
                let days = if self.wildlife.animals[mother].species == 0 {
                    30
                } else {
                    200
                };
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
    }
    fn animal_birth(&mut self, mother: usize) {
        let w = &mut self.wildlife;
        let Some(p) = w.animals[mother].pregnancy.clone() else {
            return;
        };
        if self.tick < p.due || !w.animals[mother].alive {
            return;
        }
        let count = if w.animals[mother].species == 0 { 2 } else { 1 };
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
                x: (a.x + j as f32 * 3.0).clamp(10.0, 2650.0),
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
    pub fn animal_count(&self) -> usize {
        self.wildlife.animals.len()
    }
    pub(super) fn wildlife_summary(&self) -> serde_json::Value {
        let w = &self.wildlife;
        let stock: u64 = w.grass.iter().map(|g| g.stock).sum();
        serde_json::json!({"forests":w.forests,"grass":w.grass,"rabbits":w.animals.iter().filter(|a|a.alive&&a.species==0).count(),"deer":w.animals.iter().filter(|a|a.alive&&a.species==1).count(),"births":w.ledger.births,"deaths":w.ledger.deaths,"conceptions":w.ledger.conceptions,"grass_account":{"initial":w.ledger.initial_grass,"grown":w.ledger.grown_grass,"eaten":w.ledger.eaten_grass,"stock":stock,"error":w.ledger.initial_grass as i128+w.ledger.grown_grass as i128-w.ledger.eaten_grass as i128-stock as i128},"berries_eaten":w.ledger.eaten_berries,"animals":w.animals.iter().enumerate().filter(|(_,a)|a.alive).map(|(id,a)|serde_json::json!({"id":id,"species":a.species,"x":a.x,"y":a.y,"heading":a.heading,"action":a.action,"young":!a.adult(self.tick)})).collect::<Vec<_>>()})
    }
    pub fn animal_trace(&self, i: usize) -> serde_json::Value {
        let Some(a) = self.wildlife.animals.get(i) else {
            return serde_json::Value::Null;
        };
        serde_json::json!({"x":a.x,"y":a.y,"id":i,"species":if a.species==0{"Rabbit"}else{"Deer"},"gender":if a.female{"Female"}else{"Male"},"age_days":a.age(self.tick),"adult":a.adult(self.tick),"alive":a.alive,"health":a.health,"hunger":a.hunger,"energy":a.energy,"mother":if a.mother==NONE{None}else{Some(a.mother)},"father":if a.father==NONE{None}else{Some(a.father)},"pregnancy_days_left":a.pregnancy.as_ref().map(|p|p.due.saturating_sub(self.tick) as f32/rest::DAY_TICKS as f32),"forest":a.forest,"action":NAMES[a.action as usize],"generation":a.genome.generation,"genome_nodes":a.genome.nodes.len(),"genome_genes":a.genome.genes.len(),"sensors":ANIMAL_SENSORS,"choices":self.animal_choices(i).iter().map(|c|serde_json::json!({"action":NAMES[c.action as usize],"score":c.score,"inputs":c.inputs.to_vec()})).collect::<Vec<_>>()})
    }
    pub(super) fn validate_wildlife(&self) -> Result<(), String> {
        let w = &self.wildlife;
        let grass: u64 = w.grass.iter().map(|g| g.stock).sum();
        if w.ledger.initial_grass + w.ledger.grown_grass != grass + w.ledger.eaten_grass
            || w.ledger.initial_animals + w.ledger.births != w.animals.len() as u64
            || w.animals.iter().filter(|a| !a.alive).count() as u64 != w.ledger.deaths
        {
            return Err("Wildlife conservation error".into());
        }
        for g in &w.grass {
            if g.stock > g.capacity || g.forest as usize >= w.forests.len() {
                return Err("Invalid grass patch".into());
            }
        }
        for (i, a) in w.animals.iter().enumerate() {
            if a.species > 1
                || a.action > 5
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
