//! Packed native agent state and an authoritative, integer-accounted economy.
use neat_core::{Compiled, Genome, Innovations, Rng, INPUTS, LEGACY_INPUTS};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, fs, io::Read, path::Path};
mod agriculture;
mod banking;
mod legacy_v12;
mod legacy_v13;
pub use landscape::Hill;
pub use worldmap::{Exploration, MapSize, MapSpec, Terrain, TerrainRaster, WorldMap, CHUNK_SIZE};
mod governance;
mod relationships;
use relationships::Relationships;
mod legacy_v10;
mod legacy_v11;
mod urban;
use banking::Banking;
mod wealth;
use urban::Urban;
pub use wealth::WealthView;
mod civic;
mod economy;
mod education;
mod legacy_v9;
use education::Education;
mod industry;
mod landscape;
mod legacy_v4;
mod legacy_v5;
mod legacy_v6;
mod legacy_v7;
mod wildlife;
mod worldmap;
use wildlife::Wildlife;
mod life;
mod social;
use agriculture::{Farm, FertileRegion};
use civic::{Contract, Corpse, Council, Graveyard};
use life::Person;
use social::Tavern;
mod rest;
use rest::Shelter;
mod travel;
use travel::Convoy;

pub const DT: f32 = 0.1;
pub const GOODS: usize = 13;
pub const CANDIDATES: usize = 64;
pub const WEIGHTS: [u32; GOODS] = [1, 4, 3, 2, 5, 4, 3, 4, 1, 4, 5, 3, 4];
pub const MINT_VALUES: [u64; GOODS] = [0, 0, 0, 0, 100, 10, 1, 0, 0, 0, 0, 0, 0];
pub const PRODUCTS: [&str; GOODS] = [
    "berries",
    "timber",
    "stone",
    "fish",
    "gold ore",
    "silver ore",
    "copper ore",
    "iron",
    "rice",
    "wood axe",
    "pickaxe",
    "farm hoe",
    "shovel",
];
pub const WORLD_WIDTH: f32 = 3072.0;
pub const WORLD_HEIGHT: f32 = 1920.0;
const NONE: u32 = u32::MAX;
// Wire frames pack action into 6 bits: IDs must stay below 64 and be
// append-only (old saves and replays store the index). Reserved ranges:
// 39-41 hunting/combat, 42-45 taming and husbandry, 46-49 guild quests,
// 50-53 founding and migration, 54-57 qi and cultivation, 58-63 unassigned.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[repr(u8)]
pub enum Action {
    Rest = 0,
    Gather = 1,
    Deliver = 2,
    BuyFood = 3,
    Build = 4,
    Trade = 5,
    Mint = 6,
    Barter = 7,
    WorkForFood = 8,
    MakeCamp = 9,
    Wait = 10,
    BuildGraveyard = 11,
    Gravedigger = 12,
    Court = 13,
    CareFamily = 14,
    CraftTool = 15,
    BuyTool = 16,
    Farm = 17,
    HarvestRice = 18,
    EquipTool = 19,
    ForageMeal = 20,
    StoreCargo = 21,
    Socialize = 22,
    BuildTavern = 23,
    BuildSchool = 24,
    Teach = 25,
    AttendSchool = 26,
    UrbanWork = 27,
    LeaseBusiness = 28,
    OperateBusiness = 29,
    OrderGoods = 30,
    BuyHome = 31,
    RentHome = 32,
    ShopGoods = 33,
    Hunt = 34,
    CollectEstate = 35,
    BuildBank = 36,
    BorrowBusiness = 37,
    RepayLoan = 38,
    NominateSelf = 66,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Agents {
    pub x: Vec<f32>,
    pub y: Vec<f32>,
    pub health: Vec<f32>,
    pub hunger: Vec<f32>,
    pub energy: Vec<f32>,
    pub alive: Vec<bool>,
    pub coins: Vec<u64>,
    pub berries: Vec<u32>,
    pub fish: Vec<u32>,
    pub cargo: Vec<u32>,
    pub cargo_kind: Vec<u8>,
    pub origin: Vec<u32>,
    pub home: Vec<u32>,
    pub action: Vec<Action>,
    pub target: Vec<u32>,
    pub timer: Vec<f32>,
    pub last_decision: Vec<u64>,
    pub convoy: Vec<u32>,
    pub solo_until: Vec<u64>,
    pub dwell_cell: Vec<u32>,
    pub dwell_ticks: Vec<u32>,
    pub residence: Vec<u32>,
    pub capacity: Vec<u32>,
    pub rested_meter: Vec<f32>,
    pub rested: Vec<bool>,
    pub sleeping: Vec<bool>,
    pub rest_site: Vec<u32>,
    pub bed_slot: Vec<u32>,
    pub contract: Vec<u32>,
    pub carried_body: Vec<u32>,
    pub corpse: Vec<u32>,
    pub rice: Vec<u32>,
    pub traits: Vec<u16>,
    pub tools: Vec<[u8; 4]>,
    pub urban_weight: Vec<u32>,
}
impl Agents {
    pub fn len(&self) -> usize {
        self.x.len()
    }
    pub fn is_empty(&self) -> bool {
        self.x.is_empty()
    }
    fn empty() -> Self {
        Self {
            x: vec![],
            y: vec![],
            health: vec![],
            hunger: vec![],
            energy: vec![],
            alive: vec![],
            coins: vec![],
            berries: vec![],
            fish: vec![],
            cargo: vec![],
            cargo_kind: vec![],
            origin: vec![],
            home: vec![],
            action: vec![],
            target: vec![],
            timer: vec![],
            last_decision: vec![],
            convoy: vec![],
            solo_until: vec![],
            dwell_cell: vec![],
            dwell_ticks: vec![],
            residence: vec![],
            capacity: vec![],
            rested_meter: vec![],
            rested: vec![],
            sleeping: vec![],
            rest_site: vec![],
            bed_slot: vec![],
            contract: vec![],
            carried_body: vec![],
            corpse: vec![],
            rice: vec![],
            traits: vec![],
            tools: vec![],
            urban_weight: vec![],
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Patch {
    pub id: u32,
    pub kind: usize,
    pub x: f32,
    pub y: f32,
    pub stock: u64,
    pub capacity: u64,
    pub renewal: f64,
    pub growth: f64,
    pub age: f32,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Lot {
    pub quantity: u64,
    pub origin: u32,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Quote {
    pub bid: u64,
    pub remaining: u64,
    pub escrow: u64,
    pub ask: u64,
    pub offered: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Project {
    pub kind: String,
    pub progress: f32,
    pub required: f32,
    pub destination: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Market {
    pub id: u32,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub coins: u64,
    pub goods: [VecDeque<Lot>; GOODS],
    pub quotes: [Quote; GOODS],
    pub nearest: [u32; GOODS],
    pub population: u32,
    pub housing_blocks: u32,
    pub stable_residents: u32,
    pub village: bool,
    pub project: Option<Project>,
    pub sales: [u64; GOODS],
    pub local_sales: [u64; GOODS],
    pub imported_sales: [u64; GOODS],
    pub revenue_total: u64,
    pub expenses_total: u64,
}
impl Market {
    pub fn stock(&self, k: usize) -> u64 {
        self.goods[k].iter().map(|l| l.quantity).sum()
    }
    fn add(&mut self, k: usize, q: u64, origin: u32) {
        if q == 0 {
            return;
        }
        if let Some(l) = self.goods[k].back_mut() {
            if l.origin == origin {
                l.quantity += q;
                return;
            }
        }
        self.goods[k].push_back(Lot {
            quantity: q,
            origin,
        });
    }
    fn take(&mut self, k: usize, max: u64) -> Option<Lot> {
        let l = self.goods[k].front_mut()?;
        let q = l.quantity.min(max);
        let out = Lot {
            quantity: q,
            origin: l.origin,
        };
        l.quantity -= q;
        if l.quantity == 0 {
            self.goods[k].pop_front();
        }
        Some(out)
    }
    fn cancel_quotes(&mut self) {
        for q in &mut self.quotes {
            self.coins += q.escrow;
            *q = Quote::default();
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Road {
    pub a: u32,
    pub b: u32,
    pub built: bool,
    pub under_construction: bool,
    pub traffic: u64,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Ledger {
    pub initial: [u64; GOODS],
    pub renewed: [u64; GOODS],
    pub consumed: [u64; GOODS],
    pub built: [u64; GOODS],
    pub initial_coins: u64,
    pub minted_coins: u64,
    pub minted_ore: [u64; GOODS],
    pub crafted_in: [u64; GOODS],
    pub crafted_out: [u64; GOODS],
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Statistics {
    pub decisions: u64,
    pub trades: u64,
    pub shipments: u64,
    pub harvested: u64,
    pub deaths: u64,
    pub buildings: u64,
    pub roads: u64,
    pub rejected: u64,
    pub convoys_departed: u64,
    pub solo_departures: u64,
    pub camps_founded: u64,
    pub barter_trades: u64,
    pub paid_services: u64,
    pub food_services: u64,
    pub mintings: u64,
    pub burials: u64,
    pub graveyards_built: u64,
    pub jobs_completed: u64,
    pub jobs_cancelled: u64,
    pub births: u64,
    pub marriages: u64,
    pub goals_completed: u64,
    pub tools_crafted: u64,
    pub farm_shifts: u64,
    pub rice_harvested: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct World {
    pub format: u32,
    pub seed: u64,
    pub rng: Rng,
    pub tick: u64,
    pub agents: Agents,
    pub genomes: Vec<Genome>,
    pub innovations: Innovations,
    #[serde(skip)]
    pub brains: Vec<Compiled>,
    pub markets: Vec<Market>,
    pub patches: Vec<Patch>,
    pub roads: Vec<Road>,
    pub ledger: Ledger,
    pub stats: Statistics,
    pub sales_profile_labels: Vec<String>,
    pub convoys: Vec<Option<Convoy>>,
    pub shelters: Vec<Shelter>,
    pub clock_offset: u64,
    pub councils: Vec<Council>,
    pub corpses: Vec<Corpse>,
    pub contracts: Vec<Option<Contract>>,
    pub graveyards: Vec<Graveyard>,
    pub people: Vec<Person>,
    pub matches: Vec<u32>,
    pub farms: Vec<Farm>,
    pub fertility: Vec<FertileRegion>,
    pub social: Vec<f32>,
    pub taverns: Vec<Tavern>,
    pub wildlife: Wildlife,
    pub education: Education,
    pub urban: Urban,
    pub banking: Banking,
    pub relationships: Relationships,
    #[serde(skip)]
    watch: Watch,
    pub hill: Hill,
    #[serde(skip)]
    food_memory: food_memory::FoodMemory,
    #[serde(skip)]
    forage_memory: food_memory::FoodMemory,
    #[serde(skip)]
    work_memory: work_memory::WorkMemory,
    #[serde(skip)]
    commitments: commitment_memory::Commitments,
    #[serde(skip, default = "default_checkpoint_map")]
    pub map: WorldMap,
    #[serde(skip)]
    pub explored: Exploration,
    #[serde(skip)]
    governance: governance::Governance,
}
fn default_checkpoint_map() -> WorldMap {
    WorldMap::classic(0)
}

#[derive(Clone, Debug, Default)]
struct Watch {
    memory_trial: Option<memory_pilot::Trial>,
    retail_flow: [u64; 4],
    person: Option<usize>,
    last: Option<serde_json::Value>,
    purchase: Option<serde_json::Value>,
}
type Counts = std::collections::BTreeMap<String, [usize; 2]>;
#[derive(Clone, Copy, Debug)]
struct Candidate {
    action: Action,
    target: u32,
    kind: usize,
    inputs: [f32; INPUTS],
    score: f32,
}
impl Default for Candidate {
    fn default() -> Self {
        Self {
            action: Action::Rest,
            target: 0,
            kind: 0,
            inputs: [0.0; INPUTS],
            score: f32::NEG_INFINITY,
        }
    }
}
struct Request {
    memory_offers: Option<[u16; Action::RepayLoan as usize + 1]>,
    counts: Option<Box<Counts>>,
    agent: usize,
    candidate: Candidate,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Account {
    pub minted: u64,
    pub transformed_in: u64,
    pub transformed_out: u64,
    pub product: String,
    pub initial: u64,
    pub renewed: u64,
    pub stock: u64,
    pub consumed: u64,
    pub construction: u64,
    pub error: i128,
}
#[derive(Clone, Debug, Serialize)]
pub struct AgentTrace {
    pub x: f32,
    pub y: f32,
    pub person: serde_json::Value,
    pub job: serde_json::Value,
    pub rested_meter: f32,
    pub rested: bool,
    pub sleeping: bool,
    pub rest_location: String,
    pub carried_weight: u32,
    pub capacity: u32,
    pub travel: String,
    pub id: usize,
    pub genome_nodes: usize,
    pub genome_genes: usize,
    pub home: u32,
    pub alive: bool,
    pub health: f32,
    pub energy: f32,
    pub hunger: f32,
    pub coins: u64,
    pub cargo: u32,
    pub product: String,
    pub choices: Vec<TraceChoice>,
    pub last_decision: Option<serde_json::Value>,
    pub last_purchase: Option<serde_json::Value>,
    pub candidate_counts: Counts,
}
#[derive(Clone, Debug, Serialize)]
pub struct TraceChoice {
    pub action: String,
    pub target: u32,
    pub product: String,
    pub score: f32,
    pub inputs: Vec<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_inputs: Option<[f32; 4]>,
}

impl World {
    pub fn new(count: usize, seed: u64) -> Result<Self, String> {
        Self::new_with_hill(count, seed, Hill::default())
    }
    pub fn new_with_hill(count: usize, seed: u64, hill: Hill) -> Result<Self, String> {
        Self::new_with_size(count, seed, hill, MapSize::Classic)
    }
    /// `size` selects the generated-map preset. Classic keeps the
    /// historical fixed layout; other sizes procedurally place
    /// settlements and resources on generated terrain.
    pub fn new_with_size(
        count: usize,
        seed: u64,
        hill: Hill,
        size: MapSize,
    ) -> Result<Self, String> {
        let spec = if size == MapSize::Classic {
            MapSpec::classic(seed)
        } else {
            MapSpec {
                size,
                seed,
                version: worldmap::GENERATOR_VERSION,
                home_x: WORLD_WIDTH * size.scale() as f32 / 2.0,
                home_y: WORLD_HEIGHT * size.scale() as f32 / 2.0,
            }
        };
        let hill = if size == MapSize::Classic {
            hill
        } else {
            let s = size.scale() as f32;
            Hill {
                x: hill.x * s,
                y: hill.y * s,
                radius: hill.radius * s,
            }
        };
        hill.validate(spec.width(), spec.height())?;
        if count == 0 || count > 1_200_000 {
            return Err("Population must be 1..1,200,000".into());
        }
        let mut w = Self {
            format: 15,
            seed,
            rng: Rng(seed),
            tick: 0,
            agents: Agents::empty(),
            genomes: Vec::with_capacity(count),
            innovations: Innovations::default(),
            brains: Vec::with_capacity(count),
            markets: vec![],
            patches: vec![],
            roads: vec![
                Road {
                    a: 0,
                    b: 1,
                    built: false,
                    under_construction: false,
                    traffic: 0,
                },
                Road {
                    a: 1,
                    b: 2,
                    built: false,
                    under_construction: false,
                    traffic: 0,
                },
                Road {
                    a: 0,
                    b: 2,
                    built: false,
                    under_construction: false,
                    traffic: 0,
                },
            ],
            ledger: Ledger::default(),
            stats: Statistics::default(),
            sales_profile_labels: vec![String::new(); 3],
            convoys: vec![],
            shelters: vec![],
            clock_offset: 0,
            councils: vec![],
            corpses: vec![],
            contracts: vec![],
            graveyards: vec![],
            people: vec![],
            matches: vec![],
            farms: vec![],
            fertility: vec![],
            social: vec![0.0; count],
            taverns: vec![],
            wildlife: Wildlife::default(),
            education: Education::default(),
            urban: Urban::default(),
            banking: Banking::default(),
            relationships: Relationships::default(),
            watch: Default::default(),
            food_memory: Default::default(),
            forage_memory: Default::default(),
            work_memory: Default::default(),
            commitments: Default::default(),
            hill,
            map: WorldMap::new(spec),
            explored: Exploration::default(),
            governance: governance::Governance::default(),
        };
        let market_sites: Vec<(f32, f32)> = if size == MapSize::Classic {
            vec![(180.0, 230.0), (780.0, 190.0), (510.0, 490.0)]
        } else {
            let r = w.map.spec.homeland_radius();
            (0..3)
                .map(|m| {
                    let angle = m as f32 * std::f32::consts::TAU / 3.0;
                    let cx = w.map.spec.home_x + angle.cos() * r * 0.4;
                    let cy = w.map.spec.home_y + angle.sin() * r * 0.4;
                    w.map
                        .find_terrain(&[Terrain::Fertile, Terrain::Grass], cx, cy, 0.0, r * 0.5)
                })
                .collect()
        };
        for (id, (name, (x, y))) in ["Willow Reach", "Tidewater", "Stone Crossing"]
            .iter()
            .zip(market_sites)
            .enumerate()
        {
            w.markets.push(Market {
                id: id as u32,
                name: name.to_string(),
                x,
                y,
                coins: 0,
                goods: std::array::from_fn(|_| VecDeque::new()),
                quotes: std::array::from_fn(|_| Quote::default()),
                nearest: [NONE; GOODS],
                population: 0,
                housing_blocks: 0,
                stable_residents: 0,
                village: false,
                project: None,
                sales: [0; GOODS],
                local_sales: [0; GOODS],
                imported_sales: [0; GOODS],
                revenue_total: 0,
                expenses_total: 0,
            });
        }
        for i in 0..count {
            let home = i % 3;
            let m = &mut w.markets[home];
            m.population += 1;
            m.coins += 20;
            let a = &mut w.agents;
            let (bx, by) = if size == MapSize::Classic {
                (890.0, 635.0)
            } else {
                (w.map.spec.width() - 5.0, w.map.spec.height() - 5.0)
            };
            a.x.push((m.x + w.rng.signed() * 35.0).clamp(5.0, bx));
            a.y.push((m.y + w.rng.signed() * 35.0).clamp(5.0, by));
            a.health.push(100.0);
            a.hunger.push(w.rng.unit() * 12.0);
            a.energy.push(80.0 + w.rng.unit() * 20.0);
            a.alive.push(true);
            a.coins.push(8);
            a.berries.push(3);
            a.fish.push(0);
            a.cargo.push(0);
            a.cargo_kind.push(0);
            a.origin.push(home as u32);
            a.home.push(home as u32);
            a.action.push(Action::Rest);
            a.target.push(home as u32);
            a.timer.push(0.0);
            a.last_decision.push(0);
            a.convoy.push(NONE);
            a.solo_until.push(0);
            a.dwell_cell.push(NONE);
            a.dwell_ticks.push(0);
            a.residence.push(0);
            a.capacity.push(32 + (i % 5) as u32 * 4);
            a.rested_meter.push(40.0);
            a.rested.push(false);
            a.sleeping.push(false);
            a.rest_site.push(NONE);
            a.bed_slot.push(NONE);
            a.contract.push(NONE);
            a.carried_body.push(NONE);
            a.corpse.push(NONE);
            a.rice.push(0);
            a.traits.push(0);
            a.urban_weight.push(0);
            a.tools.push([0; 4]);
            let mut genome = Genome::seeded(&mut w.rng, &mut w.innovations);
            if i % 7 == 0 {
                genome.add_node(&mut w.rng, &mut w.innovations);
            }
            if i % 11 == 0 {
                genome.add_connection(&mut w.rng, &mut w.innovations);
            }
            genome.add_memory_inputs(&mut w.innovations);
            w.brains.push(genome.compile()?);
            w.genomes.push(genome);
        }
        let scale = (count as u64).div_ceil(1000).max(1);
        for kind in 0..4 {
            let number = if kind == 1 { 16 } else { 10 };
            for j in 0..number {
                let (x, y) = if size != MapSize::Classic {
                    let home = &w.markets[j % w.markets.len()];
                    let (cx, cy, r) = (home.x, home.y, w.map.spec.homeland_radius());
                    match kind {
                        // Berries and timber stay near settlements on
                        // growing land; stone sits anywhere off water;
                        // fish need the shoreline.
                        0 => {
                            w.map
                                .find_terrain(&[Terrain::Fertile, Terrain::Grass], cx, cy, 60.0, r)
                        }
                        1 => w.map.find_terrain(
                            &[Terrain::Forest, Terrain::Fertile, Terrain::Grass],
                            cx,
                            cy,
                            60.0,
                            r,
                        ),
                        2 => w.map.find_terrain(
                            &[Terrain::Grass, Terrain::Rock, Terrain::Fertile],
                            cx,
                            cy,
                            60.0,
                            r,
                        ),
                        _ => w.map.find_terrain(&[Terrain::Water], cx, cy, 0.0, r * 1.5),
                    }
                } else {
                    match kind {
                        0 => (90.0 + w.rng.unit() * 300.0, 100.0 + w.rng.unit() * 220.0),
                        1 => (60.0 + w.rng.unit() * 260.0, 100.0 + w.rng.unit() * 260.0),
                        2 => (410.0 + w.rng.unit() * 180.0, 350.0 + w.rng.unit() * 190.0),
                        _ => (870.0 + w.rng.unit() * 18.0, 75.0 + j as f32 * 48.0),
                    }
                };
                let cap = if kind == 1 { 100 * scale } else { 80 * scale };
                w.patches.push(Patch {
                    id: w.patches.len() as u32,
                    kind,
                    x,
                    y,
                    stock: cap,
                    capacity: cap,
                    renewal: if kind == 2 {
                        0.0
                    } else if kind == 1 {
                        0.015 * scale as f64
                    } else {
                        0.06 * scale as f64
                    },
                    growth: 0.0,
                    age: 0.0,
                });
            }
        }
        let metal_sites: Vec<(f32, f32)> = if size == MapSize::Classic {
            vec![(570.0, 365.0), (615.0, 430.0), (410.0, 540.0)]
        } else {
            let r = w.map.spec.homeland_radius();
            (0..3)
                .map(|j| {
                    let home = &w.markets[j % w.markets.len()];
                    w.map
                        .find_terrain(&[Terrain::Rock], home.x, home.y, 0.0, r * 1.5)
                })
                .collect()
        };
        for (kind, (x, y)) in [4usize, 5, 6].into_iter().zip(metal_sites) {
            w.patches.push(Patch {
                id: w.patches.len() as u32,
                kind,
                x,
                y,
                stock: 120 * scale,
                capacity: 120 * scale,
                renewal: 0.0,
                growth: 0.0,
                age: 0.0,
            });
        }
        for m in &mut w.markets {
            m.add(0, m.population as u64 * 2, m.id);
        }
        w.expand_map_and_food();
        if size == MapSize::Classic {
            w.site_stone_crossing();
        }
        w.init_people();
        w.init_wildlife();
        w.upgrade_landscape(true);
        w.relocate_fertile_land();
        w.sync_education();
        w.ledger.initial = w.stocks();
        w.ledger.initial_coins = w.coins();
        w.sync_councils();
        w.sync_urban();
        w.sync_relationships();
        w.sync_shelters();
        // Each settlement starts knowing its immediate surroundings.
        for m in 0..w.markets.len() {
            let (x, y) = (w.markets[m].x, w.markets[m].y);
            w.explored.reveal(m, x, y, 500.0);
        }
        w.refresh();
        Ok(w)
    }
    pub fn living(&self) -> usize {
        self.agents.alive.iter().filter(|a| **a).count()
    }
    pub fn coins(&self) -> u64 {
        self.bank_coins()
            + self.urban_coins()
            + self.job_escrow()
            + self.farms.iter().map(|f| f.escrow).sum::<u64>()
            + self.taverns.iter().map(|t| t.escrow).sum::<u64>()
            + self
                .education
                .schools
                .iter()
                .map(|s| s.build_escrow + s.wage_escrow)
                .sum::<u64>()
            + self.agents.coins.iter().sum::<u64>()
            + self
                .markets
                .iter()
                .map(|m| m.coins + m.quotes.iter().map(|q| q.escrow).sum::<u64>())
                .sum::<u64>()
    }
    pub fn stocks(&self) -> [u64; GOODS] {
        let mut s = self.urban_raw_stocks();
        for p in &self.patches {
            s[p.kind] += p.stock;
        }
        for m in &self.markets {
            for (k, v) in s.iter_mut().enumerate() {
                *v += m.stock(k);
            }
        }
        for i in 0..self.agents.len() {
            s[0] += self.agents.berries[i] as u64;
            s[3] += self.agents.fish[i] as u64;
            s[8] += self.agents.rice[i] as u64;
            for t in 0..4 {
                s[9 + t] += self.agents.tools[i][t] as u64;
            }
            s[self.agents.cargo_kind[i] as usize] += self.agents.cargo[i] as u64;
        }
        s[8] += self.farms.iter().map(|f| f.harvest_remaining).sum::<u64>();
        s
    }
    pub fn accounts(&self) -> Vec<Account> {
        let stock = self.stocks();
        (0..GOODS)
            .map(|k| Account {
                product: PRODUCTS[k].into(),
                minted: self.ledger.minted_ore[k],
                transformed_in: self.ledger.crafted_in[k],
                transformed_out: self.ledger.crafted_out[k],
                initial: self.ledger.initial[k],
                renewed: self.ledger.renewed[k],
                stock: stock[k],
                consumed: self.ledger.consumed[k],
                construction: self.ledger.built[k],
                error: self.ledger.initial[k] as i128
                    + self.ledger.renewed[k] as i128
                    + self.ledger.crafted_out[k] as i128
                    - stock[k] as i128
                    - self.ledger.consumed[k] as i128
                    - self.ledger.built[k] as i128
                    - self.ledger.minted_ore[k] as i128
                    - self.ledger.crafted_in[k] as i128,
            })
            .collect()
    }
    pub fn check(&self) -> Result<(), String> {
        for a in self.accounts() {
            if a.error != 0 {
                return Err(format!("{} conservation error {}", a.product, a.error));
            }
        }
        if self.coins() != self.ledger.initial_coins + self.ledger.minted_coins {
            return Err("Coin conservation error".into());
        }
        for i in 0..self.agents.len() {
            if self.agents.weight(i) > self.agents.capacity[i] {
                return Err(format!("Agent {i} exceeds carrying capacity"));
            }
        }
        self.validate_wildlife()?;
        self.validate_governance()?;
        self.validate_education()?;
        self.validate_urban()?;
        self.validate_banking()?;
        self.validate_relationships()?;
        Ok(())
    }
    fn refresh(&mut self) {
        for mi in 0..self.markets.len() {
            let m = &self.markets[mi];
            let mut nearest = [NONE; GOODS];
            let mut costs = [f32::INFINITY; GOODS];
            for p in &self.patches {
                if p.stock == 0 {
                    continue;
                }
                let d = dist(m.x, m.y, p.x, p.y);
                if d < costs[p.kind] {
                    costs[p.kind] = d;
                    nearest[p.kind] = p.id;
                }
            }
            self.markets[mi].nearest = nearest;
        }
        for m in &mut self.markets {
            m.cancel_quotes();
            let pop = m.population as u64;
            for k in 0..GOODS {
                let stock = m.stock(k);
                let target = match k {
                    0 | 3 | 8 => (pop / 3).max(12),
                    1 => (pop / 10).max(24),
                    _ => (pop / 16).max(12),
                };
                let deficit = target.saturating_sub(stock);
                let bid = (1 + deficit * 6 / target.max(1)).min(8);
                let qty = deficit
                    .min((pop / 2).max(16))
                    .min(m.coins.saturating_sub((pop / 8).max(8)) / bid);
                let escrow = qty * bid;
                m.coins -= escrow;
                m.quotes[k] = Quote {
                    bid,
                    remaining: qty,
                    escrow,
                    ask: bid + 1,
                    offered: stock,
                };
            }
        }
    }
    pub fn watch_person(&mut self, person: Option<usize>) {
        if self.watch.person != person {
            self.watch = Watch {
                person,
                retail_flow: self.watch.retail_flow,
                memory_trial: self.watch.memory_trial.take(),
                ..Default::default()
            };
        }
    }
    /// Successful shop orders, wholesale spend including freight, pickups, and deliveries.
    pub fn retail_flow(&self) -> [u64; 4] {
        self.watch.retail_flow
    }
    pub fn last_watched_decision(&self) -> Option<&serde_json::Value> {
        self.watch.last.as_ref()
    }
    #[cfg(test)]
    fn offer(&self, i: usize) -> ([Candidate; CANDIDATES], usize) {
        self.offer_counted(i, None)
    }
    fn offer_counted(
        &self,
        i: usize,
        omitted: Option<&mut Counts>,
    ) -> ([Candidate; CANDIDATES], usize) {
        self.offer_with_limit(i, omitted, CANDIDATES)
    }
    fn offer_with_limit(
        &self,
        i: usize,
        mut omitted: Option<&mut Counts>,
        limit: usize,
    ) -> ([Candidate; CANDIDATES], usize) {
        let mut out = [Candidate::default(); CANDIDATES];
        if !self.is_adult(i) || self.must_stay_home(i) {
            let (mut family, mut count) = self.family_offers(i);
            if count < limit {
                if let Some(offer) = self.nomination_offer(i) {
                    family[count] = offer;
                    count += 1;
                }
            }
            return (family, count);
        }
        if self.agents.hunger[i] >= 55.0 {
            if let Some(p) = self.closest_food_patch(i) {
                out[0] = Candidate {
                    action: Action::ForageMeal,
                    target: p as u32,
                    kind: self.patches[p].kind,
                    inputs: [0.0; INPUTS],
                    score: 10.0,
                };
                return (out, 1);
            }
        }
        if self.urgent_family_care(i) {
            out[0] = Candidate {
                action: Action::CareFamily,
                target: self.agents.home[i],
                kind: 0,
                inputs: [0.0; INPUTS],
                score: 10.0,
            };
            return (out, 1);
        }
        if let Some(c) = self.urban_committed_offer(i) {
            out[0] = c;
            return (out, 1);
        }
        let mut count = 0;
        if limit > 0 {
            if let Some(offer) = self.nomination_offer(i) {
                out[0] = offer;
                count = 1;
            }
        }
        let a = &self.agents;
        let home = a.home[i] as usize;
        let m = &self.markets[home];
        let hunger = (a.hunger[i] / 100.0).clamp(0.0, 1.0);
        let held = a.berries[i] + a.fish[i] + a.rice[i];
        let mut add = |action: Action,
                       target: u32,
                       kind: usize,
                       need: f32,
                       benefit: f32,
                       risk: f32,
                       travel: f32,
                       price: f32,
                       infrastructure: f32| {
            if count == limit {
                if let Some(counts) = omitted.as_deref_mut() {
                    counts.entry(format!("{action:?}")).or_default()[1] += 1;
                }
                return;
            }
            let mut x = [0.0; INPUTS];
            x[0] = need
                + if a.hunger[i] > 65.0
                    && (matches!(
                        action,
                        Action::BuyFood
                            | Action::Barter
                            | Action::WorkForFood
                            | Action::HarvestRice
                    ) || action == Action::Gather && [0, 3, 8].contains(&kind))
                {
                    1.0
                } else {
                    0.0
                };
            x[1] = benefit;
            x[2] = risk;
            x[3] = (travel / 1000.0).min(1.0);
            x[4] = (price / 8.0).clamp(-1.0, 1.0);
            x[5] = if a.action[i] == action && a.target[i] == target {
                0.25
            } else {
                0.0
            };
            if !matches!(action, Action::Gravedigger | Action::BuildGraveyard) && kind < 4 {
                x[6 + kind] = 1.0;
            } else if (4..=6).contains(&kind) {
                x[12 + kind - 4] = 1.0;
            }
            x[15] = (a.coins[i] as f32 / 100.0).min(1.0);
            x[16] = 1.0 / (1.0 + a.coins[i] as f32 / 20.0);
            x[17] = a.weight(i) as f32 / a.capacity[i] as f32;
            x[18] = a.room(i) as f32 / a.capacity[i] as f32;
            x[19] = if matches!(action, Action::BuyFood | Action::Trade) {
                (a.coins[i] as f32 / m.quotes[kind].ask.max(1) as f32).min(1.0)
            } else {
                0.0
            };
            x[20] = if matches!(action, Action::Barter | Action::WorkForFood) {
                1.0
            } else {
                0.0
            };
            x[21] = if action == Action::WorkForFood {
                -1.0
            } else if matches!(
                action,
                Action::Mint
                    | Action::Deliver
                    | Action::Build
                    | Action::Gravedigger
                    | Action::BuildGraveyard
                    | Action::UrbanWork
                    | Action::OperateBusiness
            ) {
                1.0
            } else {
                0.0
            };
            x[22] = if self.is_night() { 1.0 } else { 0.0 };
            x[23] = a.rested_meter[i] / 100.0;
            x[24] = if a.rested[i] { 1.0 } else { 0.0 };
            x[25] = if action == Action::Rest { 1.0 } else { 0.0 };
            x[26] = if action == Action::Rest {
                x[22] * (1.0 - a.hunger[i] / 70.0).max(0.0)
            } else {
                0.0
            };
            x[27] = if action == Action::Rest {
                1.0
            } else if action == Action::MakeCamp {
                0.5
            } else {
                0.0
            };
            self.personal_inputs(i, action, kind, &mut x);
            x[1] += self.relationship_affinity(i, action, target) * 0.1;
            x[10] = a.energy[i] / 100.0;
            x[11] = infrastructure;
            let score = self.score_action(i, action, target, &x);
            out[count] = Candidate {
                action,
                target,
                kind,
                inputs: x,
                score,
            };
            count += 1;
        };
        if a.cargo[i] > 0 {
            let kind = a.cargo_kind[i] as usize;
            let best = self
                .markets
                .iter()
                .filter(|m| m.quotes[kind].remaining > 0)
                .max_by(|x, y| {
                    let sx = x.quotes[kind].bid as f32 - dist(a.x[i], a.y[i], x.x, x.y) / 350.0;
                    let sy = y.quotes[kind].bid as f32 - dist(a.x[i], a.y[i], y.x, y.y) / 350.0;
                    sx.total_cmp(&sy)
                });
            if let Some(dst) = best {
                add(
                    Action::Deliver,
                    dst.id,
                    kind,
                    a.cargo[i] as f32 / 5.0,
                    0.8,
                    0.0,
                    dist(a.x[i], a.y[i], dst.x, dst.y),
                    dst.quotes[kind].bid as f32,
                    0.0,
                );
            }
        }
        if a.room(i) > 0 && a.energy[i] > 8.0 {
            for kind in 0..GOODS {
                if a.cargo[i] > 0 && a.cargo_kind[i] as usize != kind {
                    continue;
                }
                if a.room(i) < WEIGHTS[kind] {
                    continue;
                }
                let id = m.nearest[kind];
                if id == NONE {
                    continue;
                }
                let p = &self.patches[id as usize];
                if p.stock == 0 {
                    continue;
                }
                let edible = kind == 0 || kind == 3 || kind == 8;
                let need = if edible {
                    hunger + (if held < 2 { 0.6 } else { 0.05 })
                } else {
                    0.1
                };
                add(
                    Action::Gather,
                    id,
                    kind,
                    need,
                    0.5,
                    if kind == 3 { 0.12 } else { 0.01 },
                    dist(a.x[i], a.y[i], p.x, p.y),
                    m.quotes[kind].bid as f32,
                    0.0,
                );
            }
        }
        if held < 3 && a.cargo[i] == 0 {
            for kind in [0, 3, 8] {
                let q = &m.quotes[kind];
                if q.offered > 0 && a.coins[i] >= q.ask && a.room(i) >= WEIGHTS[kind] {
                    add(
                        Action::BuyFood,
                        m.id,
                        kind,
                        hunger + if held == 0 { 1.2 } else { 0.0 },
                        0.9,
                        0.0,
                        dist(a.x[i], a.y[i], m.x, m.y),
                        0.0,
                        0.0,
                    );
                    break;
                }
            }
        }
        if a.cargo[i] == 0 && held > 0 && a.energy[i] > 20.0 {
            let mut best = None;
            let mut profit = 0.0;
            for kind in 0..GOODS {
                let q = &m.quotes[kind];
                if q.offered == 0 || a.coins[i] < q.ask || a.room(i) < WEIGHTS[kind] {
                    continue;
                }
                for dst in &self.markets {
                    if dst.id == m.id || dst.quotes[kind].remaining == 0 {
                        continue;
                    }
                    let gain = dst.quotes[kind].bid as f32
                        - q.ask as f32
                        - dist(m.x, m.y, dst.x, dst.y) / 800.0;
                    if gain > profit {
                        profit = gain;
                        best = Some((kind, dst.id));
                    }
                }
            }
            if let Some((kind, dst)) = best {
                add(
                    Action::Trade,
                    dst,
                    kind,
                    0.2,
                    profit.min(3.0) / 3.0,
                    0.08,
                    dist(a.x[i], a.y[i], m.x, m.y),
                    profit,
                    0.0,
                );
            }
        }
        if m.project.is_some() && m.coins > 0 && a.energy[i] > 12.0 && a.cargo[i] == 0 {
            add(
                Action::Build,
                m.id,
                1,
                0.2,
                0.65,
                0.0,
                dist(a.x[i], a.y[i], m.x, m.y),
                0.0,
                0.7,
            );
        }
        if a.cargo[i] > 0 && (4..=6).contains(&a.cargo_kind[i]) {
            add(
                Action::Mint,
                m.id,
                a.cargo_kind[i] as usize,
                0.3 + x_currency_need(a.coins[i]),
                0.9,
                0.0,
                dist(a.x[i], a.y[i], m.x, m.y),
                0.0,
                0.0,
            );
        }
        if a.cargo[i] > 0 && held < 3 {
            if let Some(dst) = self
                .markets
                .iter()
                .filter(|dst| self.barter_terms(i, dst.id as usize).is_some())
                .min_by(|u, v| {
                    dist(a.x[i], a.y[i], u.x, u.y).total_cmp(&dist(a.x[i], a.y[i], v.x, v.y))
                })
            {
                add(
                    Action::Barter,
                    dst.id,
                    a.cargo_kind[i] as usize,
                    hunger + if held == 0 { 1.4 } else { 0.4 },
                    0.9,
                    0.0,
                    dist(a.x[i], a.y[i], dst.x, dst.y),
                    0.0,
                    0.0,
                );
            }
        }
        if m.project.is_some()
            && a.cargo[i] == 0
            && a.energy[i] > 12.0
            && held < 3
            && [0, 3, 8]
                .iter()
                .any(|&k| m.stock(k) > 0 && a.room(i) >= WEIGHTS[k])
        {
            add(
                Action::WorkForFood,
                m.id,
                0,
                hunger + if held == 0 { 1.2 } else { 0.1 },
                0.8,
                0.0,
                dist(a.x[i], a.y[i], m.x, m.y),
                0.0,
                0.7,
            );
        }
        if a.contract[i] == NONE && a.cargo[i] == 0 && a.energy[i] > 8.0 && a.hunger[i] < 65.0 {
            if let Some((market, kind, reward, risk, travel, pressure)) = self.job_opportunity(i) {
                add(
                    kind,
                    market as u32,
                    2,
                    0.25 + pressure * 0.7 + x_currency_need(a.coins[i]) * 0.35,
                    (reward as f32 / 12.0).min(1.4),
                    risk,
                    travel,
                    reward as f32,
                    0.8,
                );
            }
        }
        if self.people[i].spouse != NONE {
            add(
                Action::CareFamily,
                m.id,
                0,
                0.25,
                0.3,
                0.0,
                dist(a.x[i], a.y[i], m.x, m.y),
                0.0,
                0.2,
            );
        }
        if let Some(partner) = self.marriage_target(i) {
            add(
                Action::Court,
                partner as u32,
                0,
                0.35,
                0.6,
                0.0,
                dist(a.x[i], a.y[i], a.x[partner], a.y[partner]),
                0.0,
                0.0,
            );
        }
        if a.cargo[i] == 0 {
            for kind in 9..13 {
                if self.can_craft(i, home, kind) {
                    add(
                        Action::CraftTool,
                        m.id,
                        kind,
                        0.3,
                        0.7,
                        0.02,
                        dist(a.x[i], a.y[i], m.x, m.y),
                        2.0,
                        0.8,
                    );
                }
                if a.tools[i][kind - 9] == 0
                    && m.stock(kind) > 0
                    && a.coins[i] >= m.quotes[kind].ask
                    && a.room(i) >= WEIGHTS[kind]
                {
                    add(
                        Action::BuyTool,
                        m.id,
                        kind,
                        0.15,
                        0.5,
                        0.0,
                        dist(a.x[i], a.y[i], m.x, m.y),
                        -(m.quotes[kind].ask as f32),
                        0.4,
                    );
                }
            }
        }
        if (9..13).contains(&(a.cargo_kind[i] as usize))
            && a.cargo[i] > 0
            && a.tools[i][a.cargo_kind[i] as usize - 9] == 0
        {
            add(
                Action::EquipTool,
                m.id,
                a.cargo_kind[i] as usize,
                0.2,
                0.7,
                0.0,
                0.0,
                0.0,
                0.0,
            );
        }
        if let Some(f) = self.best_farm(i, false) {
            add(
                Action::Farm,
                f as u32,
                8,
                0.3,
                0.7,
                0.02,
                dist(a.x[i], a.y[i], self.farms[f].x, self.farms[f].y),
                2.0,
                0.6,
            );
        }
        if let Some(f) = self.best_farm(i, true) {
            add(
                Action::HarvestRice,
                f as u32,
                8,
                hunger + if held < 2 { 0.6 } else { 0.05 },
                0.7,
                0.01,
                dist(a.x[i], a.y[i], self.farms[f].x, self.farms[f].y),
                2.0,
                0.5,
            );
        }
        if a.hunger[i] > 30.0 {
            if let Some(p) = self.closest_food_patch(i) {
                add(
                    Action::ForageMeal,
                    p as u32,
                    self.patches[p].kind,
                    hunger + 0.6,
                    0.8,
                    0.0,
                    dist(a.x[i], a.y[i], self.patches[p].x, self.patches[p].y),
                    0.0,
                    0.0,
                );
            }
        }
        if let Some(t) = self.best_tavern(i, false) {
            let site = &self.taverns[t];
            add(
                Action::Socialize,
                t as u32,
                0,
                if a.traits[i] & (life::SOCIALITE | life::GREGARIOUS | life::COMMUNITY_MINDED) != 0
                {
                    0.85
                } else {
                    0.1
                },
                0.35,
                0.0,
                dist(a.x[i], a.y[i], site.x, site.y),
                0.0,
                0.6,
            );
        }
        if let Some(t) = self.best_tavern(i, true) {
            let site = &self.taverns[t];
            add(
                Action::BuildTavern,
                t as u32,
                1,
                0.65,
                0.8,
                0.0,
                dist(a.x[i], a.y[i], site.x, site.y),
                0.0,
                0.5,
            );
        }
        for c in self.urban_offers(i) {
            add(
                c.action,
                c.target,
                c.kind,
                c.inputs[0],
                c.inputs[1],
                c.inputs[2],
                c.inputs[3] * 1000.0,
                c.inputs[4],
                c.inputs[11],
            );
        }
        for action in [Action::BuildSchool, Action::Teach] {
            if let Some(id) = self.best_school(i, action) {
                let s = &self.education.schools[id];
                add(
                    action,
                    id as u32,
                    1,
                    if self.people[i].aspiration == life::Aspiration::Teacher {
                        1.2
                    } else {
                        0.7
                    },
                    0.9,
                    0.0,
                    dist(a.x[i], a.y[i], s.x, s.y),
                    10.0,
                    0.8,
                );
            }
        }
        if a.cargo[i] > 0 {
            add(
                Action::StoreCargo,
                m.id,
                a.cargo_kind[i] as usize,
                if a.room(i) < 4 { 0.8 } else { 0.1 },
                0.2,
                0.0,
                dist(a.x[i], a.y[i], m.x, m.y),
                0.0,
                0.0,
            );
        }
        let rest_need = (100.0 - a.energy[i]) / 70.0
            + (100.0 - a.rested_meter[i]) / 100.0
            + if self.is_night() { 0.8 } else { 0.0 };
        if let Some(site) = self.best_rest_site(i) {
            let (x, y) = self.shelter_destination(i, site);
            add(
                Action::Rest,
                site as u32,
                0,
                rest_need,
                0.3,
                0.0,
                dist(a.x[i], a.y[i], x, y),
                0.0,
                0.0,
            );
        }
        if self.can_make_camp(i) {
            add(
                Action::MakeCamp,
                NONE,
                1,
                rest_need,
                0.25,
                0.08,
                0.0,
                0.0,
                0.0,
            );
        }
        add(Action::Wait, NONE, 0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        (out, count)
    }
    fn request(&self, i: usize) -> Option<Request> {
        if !self.agents.alive[i]
            || self.agents.contract[i] != NONE
            || i as u64 % 10 != self.tick % 10
        {
            return None;
        }
        let mut counts = (self.watch.person == Some(i)).then(Box::<Counts>::default);
        let (out, count) = self.offer_counted(i, counts.as_deref_mut());
        if let Some(totals) = counts.as_mut() {
            for c in &out[..count] {
                totals.entry(format!("{:?}", c.action)).or_default()[0] += 1;
            }
        }
        let mut out = out;
        let memory_offers = self.watch.memory_trial.as_ref().map(|trial| {
            let mut tally = [0; Action::RepayLoan as usize + 1];
            for candidate in &mut out[..count] {
                tally[candidate.action as usize] += 1;
                candidate.score += trial.adjust(
                    (i, candidate.action as u8, candidate.target, candidate.kind),
                    self.tick,
                    &candidate.inputs,
                );
            }
            tally
        });
        out[..count]
            .iter()
            .max_by(|a, b| a.score.total_cmp(&b.score))
            .copied()
            .map(|candidate| Request {
                memory_offers,
                counts,
                agent: i,
                candidate,
            })
    }
    fn capture_decision(&mut self, r: &Request) {
        if self.watch.person == Some(r.agent) && self.agents.convoy[r.agent] == NONE {
            let c = r.candidate;
            let mut decision = serde_json::json!({"actor_id":r.agent,"decision_tick":self.tick,"action":format!("{:?}",c.action),"target":c.target,"candidate_counts":r.counts,"score":c.score,"resolution_tick":null,"outcome":null});
            if c.action == Action::ShopGoods {
                let shop = c.target as usize / 32;
                decision["shop_id"] = shop.into();
                decision["product_id"] = (c.target % 32).into();
                decision["owner_id"] =
                    serde_json::json!(self.urban.buildings.get(shop).map(|b| b.owner));
            }
            self.watch.last = Some(decision);
        }
    }
    pub fn trace(&self, i: usize) -> Option<AgentTrace> {
        if i >= self.agents.len() {
            return None;
        }
        let mut candidate_counts = Counts::new();
        let (out, count) = self.offer_counted(i, Some(&mut candidate_counts));
        for c in &out[..count] {
            candidate_counts
                .entry(format!("{:?}", c.action))
                .or_default()[0] += 1;
        }
        let watched = self.watch.person == Some(i);
        let mut choices: Vec<_> = out[..count]
            .iter()
            .map(|c| TraceChoice {
                action: format!("{:?}", c.action),
                target: c.target,
                product: if self.is_urban_action(c.action) {
                    self.urban_choice_name(c)
                } else {
                    PRODUCTS[c.kind].into()
                },
                score: c.score,
                inputs: c.inputs.to_vec(),
                memory_inputs: self.action_memory_inputs(i, c.action, c.target),
            })
            .collect();
        choices.sort_by(|a, b| b.score.total_cmp(&a.score));
        let a = &self.agents;
        Some(AgentTrace {
            x: a.x[i],
            y: a.y[i],
            person: self.person_trace(i),
            job: self.job_trace(i),
            rested_meter: a.rested_meter[i],
            rested: a.rested[i],
            sleeping: a.sleeping[i],
            rest_location: self.rest_description(i),
            carried_weight: a.weight(i),
            capacity: a.capacity[i],
            travel: self.travel_description(i),
            id: i,
            genome_nodes: self.genomes[i].nodes.len(),
            genome_genes: self.genomes[i].genes.len(),
            home: a.home[i],
            alive: a.alive[i],
            health: a.health[i],
            energy: a.energy[i],
            hunger: a.hunger[i],
            coins: a.coins[i],
            cargo: a.cargo[i],
            product: PRODUCTS[a.cargo_kind[i] as usize].into(),
            candidate_counts,
            last_decision: if watched {
                self.watch.last.clone()
            } else {
                None
            },
            last_purchase: if watched {
                self.watch.purchase.clone()
            } else {
                None
            },
            choices,
        })
    }
    pub fn step(&mut self, parallel: bool) {
        self.tick += 1;
        self.sync_shelters();
        self.domestic_rules();
        for p in &mut self.patches {
            if p.stock >= p.capacity || p.renewal == 0.0 {
                continue;
            }
            p.age += DT;
            if p.kind == 1 && p.stock == 0 && p.age < 60.0 {
                continue;
            }
            let recruitment = if p.kind == 3 {
                (p.stock as f64 / p.capacity as f64).clamp(0.02, 1.0)
            } else {
                1.0
            };
            p.growth += p.renewal * DT as f64 * recruitment;
            if p.growth >= 1.0 {
                let q = (p.growth as u64).min(p.capacity - p.stock);
                p.stock += q;
                p.growth -= q as f64;
                self.ledger.renewed[p.kind] += q;
            }
        }
        let mut new_deaths = Vec::new();
        for i in 0..self.agents.len() {
            let a = &mut self.agents;
            if !a.alive[i] {
                continue;
            }
            if !a.sleeping[i] {
                a.rested_meter[i] = (a.rested_meter[i] - DT).max(0.0);
                if a.rested_meter[i] <= 0.0 {
                    a.rested[i] = false;
                }
                a.drain_energy(i, DT * 0.08);
            }
            a.hunger[i] += DT
                * if self.people[i].age_days(self.tick) < 18 * 365 {
                    0.18
                } else if a.traits[i] & life::GLUTTON != 0 {
                    1.05
                } else {
                    0.9
                };
            if a.hunger[i] > 40.0 {
                let eaten = if a.berries[i] > 0 {
                    a.berries[i] -= 1;
                    Some(0)
                } else if a.fish[i] > 0 {
                    a.fish[i] -= 1;
                    Some(3)
                } else if a.rice[i] > 0 {
                    a.rice[i] -= 1;
                    Some(8)
                } else if a.cargo[i] > 0 && [0, 3, 8].contains(&a.cargo_kind[i]) {
                    a.cargo[i] -= 1;
                    Some(a.cargo_kind[i] as usize)
                } else {
                    None
                };
                if let Some(kind) = eaten {
                    self.people[i].meals += 1;
                    a.hunger[i] -= 40.0;
                    self.ledger.consumed[kind] += 1;
                }
            }
            if a.hunger[i] > 85.0 {
                a.health[i] -= DT * 0.8;
            }
            if a.health[i] <= 0.0 {
                a.health[i] = 0.0;
                a.alive[i] = false;
                a.sleeping[i] = false;
                a.rested[i] = false;
                self.stats.deaths += 1;
                new_deaths.push(i);
            }
        }
        for i in new_deaths {
            self.food_memory.forget(i);
            self.forage_memory.forget(i);
            self.commitments.forget(i);
            for memory in &mut self.work_memory {
                memory.forget(i);
            }
            self.register_corpse(i);
        }
        self.governance_step();
        self.civic_step();
        let start = self.tick as usize % 10;
        let due = self.agents.len().saturating_sub(start).div_ceil(10);
        let requests: Vec<Request> = if parallel {
            (0..due)
                .into_par_iter()
                .filter_map(|j| self.request(start + j * 10))
                .collect()
        } else {
            (0..due)
                .filter_map(|j| self.request(start + j * 10))
                .collect()
        };
        for r in requests {
            if let (Some(trial), Some(offers)) = (&mut self.watch.memory_trial, r.memory_offers) {
                trial.commit(
                    r.agent,
                    r.candidate,
                    offers,
                    self.agents.convoy[r.agent] == NONE,
                );
            }
            self.capture_decision(&r);
            if self.agents.convoy[r.agent] == NONE {
                let c = r.candidate;
                if self.agents.cargo[r.agent] == 0
                    || matches!(c.action, Action::StoreCargo | Action::Barter)
                {
                    self.commitments.cancel_trade(r.agent);
                }
                self.commitments.commit(r.agent, c.action, c.target, c.kind);
                let food = [
                    (Action::BuyFood, &mut self.food_memory),
                    (Action::ForageMeal, &mut self.forage_memory),
                ];
                let work = work_memory::WorkKind::ACTIONS
                    .into_iter()
                    .zip(&mut self.work_memory);
                for (action, memory) in food.into_iter().chain(work) {
                    if c.action == action
                        && (action != Action::Rest || !self.agents.rested[r.agent])
                    {
                        memory.commit(r.agent, c.target, c.kind);
                    } else {
                        memory.cancel(r.agent);
                    }
                }
            }
            if matches!(
                r.candidate.action,
                Action::Gravedigger | Action::BuildGraveyard
            ) {
                if self.agents.convoy[r.agent] == NONE {
                    if !self.accept_job(r.agent, r.candidate.target as usize, r.candidate.action) {
                        self.commitments.resolve(
                            r.agent,
                            r.candidate.action,
                            r.candidate.target,
                            r.candidate.kind,
                            self.tick,
                            false,
                        );
                    }
                    self.agents.last_decision[r.agent] = self.tick;
                    self.stats.decisions += 1;
                }
                continue;
            }
            let a = &mut self.agents;
            let i = r.agent;
            if a.convoy[i] != NONE {
                continue;
            }
            if a.action[i] != r.candidate.action || a.target[i] != r.candidate.target {
                a.timer[i] = 0.0;
            }
            a.action[i] = r.candidate.action;
            a.target[i] = r.candidate.target;
            if a.cargo[i] == 0 {
                a.cargo_kind[i] = r.candidate.kind as u8;
            }
            a.last_decision[i] = self.tick;
            self.stats.decisions += 1;
        }
        self.sync_rest_reservations();
        self.travel_step();
        let len = self.agents.len();
        let offset = self.tick as usize % len;
        for j in 0..len {
            let i = (j + offset) % len;
            if self.agents.alive[i] && self.agents.convoy[i] == NONE {
                self.agents.sleeping[i] = false;
                if self.watch.memory_trial.is_some() {
                    self.execute_with_memory_pilot(i);
                } else {
                    self.execute(i);
                }
            }
        }
        self.start_projects();
        self.settlement_step();
        self.life_step();
        self.farm_step();
        self.social_step();
        self.education_step();
        self.urban_step();
        self.banking_step();
        self.relationships_step();
        self.wildlife_step();
        self.update_explored();
        if self.tick % 10 == 0 {
            self.refresh();
        }
    }
    fn execute(&mut self, i: usize) {
        let action = self.agents.action[i];
        if action == Action::NominateSelf {
            return self.execute_nomination(i);
        }
        if matches!(
            action,
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
            self.execute_personal(i, action);
            return;
        }
        if matches!(action, Action::Gravedigger | Action::BuildGraveyard) {
            self.execute_job(i);
            return;
        }
        if action == Action::Rest {
            self.execute_rest(i);
            return;
        }
        if action == Action::MakeCamp {
            self.execute_make_camp(i);
            return;
        }
        if action == Action::Wait {
            return;
        }
        let movement_speed = self.actor_speed(i, self.agents.x[i], self.agents.y[i]);
        let labor_speed = self.work_factor(i, action, self.agents.cargo_kind[i] as usize);
        let t = self.agents.target[i] as usize;
        let kind = self.agents.cargo_kind[i] as usize;
        let home = self.agents.home[i] as usize;
        let (x, y) = if action == Action::Gather {
            let Some(p) = self.patches.get(t) else { return };
            (p.x, p.y)
        } else if action == Action::Trade {
            (self.markets[home].x, self.markets[home].y)
        } else {
            let Some(m) = self.markets.get(t) else { return };
            (m.x, m.y)
        };
        if dist(self.agents.x[i], self.agents.y[i], x, y) <= 7.0
            && matches!(
                action,
                Action::Mint | Action::Barter | Action::Build | Action::WorkForFood
            )
        {
            self.execute_exchange(i, t, action);
            return;
        }
        let a = &mut self.agents;
        let d = dist(a.x[i], a.y[i], x, y);
        if d > 7.0 {
            let speed = movement_speed;
            let step = (speed * DT).min(d - 6.0);
            a.x[i] += (x - a.x[i]) / d * step;
            a.y[i] += (y - a.y[i]) / d * step;
            a.drain_energy(i, DT * 0.07);
            return;
        }
        a.timer[i] += DT * labor_speed;
        match action {
            Action::Rest
            | Action::MakeCamp
            | Action::Wait
            | Action::Gravedigger
            | Action::BuildGraveyard => {}
            Action::Gather => {
                let p = &mut self.patches[t];
                let memory = &mut self.work_memory[work_memory::WorkKind::Gather as usize];
                if p.kind != kind || a.room(i) < WEIGHTS[kind] || a.energy[i] < 1.0 {
                    memory.cancel(i);
                    return;
                }
                if p.stock == 0 {
                    memory.resolve(i, t as u32, kind, self.tick, false);
                    return;
                }
                if a.timer[i] >= 1.0 {
                    p.stock -= 1;
                    p.age = 0.0;
                    a.cargo[i] += 1;
                    a.origin[i] = home as u32;
                    a.drain_energy(i, 0.8);
                    a.timer[i] = 0.0;
                    self.stats.harvested += 1;
                    memory.resolve(i, t as u32, kind, self.tick, true);
                }
            }
            Action::Deliver => {
                if a.cargo[i] == 0 {
                    return;
                }
                let m = &mut self.markets[t];
                let q = a.cargo[i] as u64;
                let q = q.min(m.quotes[kind].remaining);
                if q == 0 {
                    self.stats.rejected += 1;
                    self.commitments
                        .resolve(i, Action::Deliver, t as u32, kind, self.tick, false);
                    self.commitments
                        .trade_sale(i, t as u32, kind, self.tick, false, false);
                    return;
                }
                let price = m.quotes[kind].bid;
                let payment = q * price;
                assert!(m.quotes[kind].escrow >= payment);
                m.quotes[kind].escrow -= payment;
                m.quotes[kind].remaining -= q;
                a.coins[i] += payment;
                a.cargo[i] -= q as u32;
                self.commitments
                    .resolve(i, Action::Deliver, t as u32, kind, self.tick, true);
                self.commitments
                    .trade_sale(i, t as u32, kind, self.tick, true, a.cargo[i] == 0);
                m.add(kind, q, a.origin[i]);
                m.sales[kind] += q;
                if a.origin[i] == m.id {
                    m.local_sales[kind] += q;
                } else {
                    m.imported_sales[kind] += q;
                    self.stats.shipments += 1;
                    for r in &mut self.roads {
                        if (r.a == m.id && r.b == a.origin[i])
                            || (r.b == m.id && r.a == a.origin[i])
                        {
                            r.traffic += q;
                        }
                    }
                }
                self.stats.trades += 1;
                self.people[i].trades += 1;
            }
            Action::BuyFood => {
                if ![0, 3, 8].contains(&kind)
                    || a.berries[i] + a.fish[i] + a.rice[i] >= 4
                    || a.room(i) < WEIGHTS[kind]
                {
                    self.food_memory.cancel(i);
                    return;
                }
                let m = &mut self.markets[t];
                let price = m.quotes[kind].ask;
                if a.coins[i] < price {
                    self.food_memory.cancel(i);
                    return;
                }
                if m.quotes[kind].offered == 0 || price == 0 {
                    self.food_memory
                        .resolve(i, t as u32, kind, self.tick, false);
                    return;
                }
                if let Some(l) = m.take(kind, 1) {
                    self.food_memory.resolve(i, t as u32, kind, self.tick, true);
                    a.coins[i] -= price;
                    m.coins += price;
                    m.revenue_total += price;
                    m.quotes[kind].offered -= 1;
                    if kind == 0 {
                        a.berries[i] += l.quantity as u32;
                    } else if kind == 3 {
                        a.fish[i] += l.quantity as u32;
                    } else {
                        a.rice[i] += l.quantity as u32;
                    }
                    self.stats.trades += 1;
                    self.people[i].trades += 1;
                } else {
                    self.food_memory
                        .resolve(i, t as u32, kind, self.tick, false);
                }
            }
            Action::Trade => {
                if a.cargo[i] > 0 {
                    return;
                }
                let m = &mut self.markets[home];
                let price = m.quotes[kind].ask;
                if price == 0 || m.quotes[kind].offered == 0 {
                    self.commitments
                        .resolve(i, Action::Trade, t as u32, kind, self.tick, false);
                    return;
                }
                if a.coins[i] < price {
                    return;
                }
                let qty = (a.room(i) / WEIGHTS[kind]) as u64;
                let qty = qty.min(m.quotes[kind].offered).min(a.coins[i] / price);
                if qty == 0 {
                    return;
                }
                if let Some(l) = m.take(kind, qty) {
                    let cost = l.quantity * price;
                    a.coins[i] -= cost;
                    m.coins += cost;
                    m.revenue_total += cost;
                    m.quotes[kind].offered -= l.quantity;
                    a.cargo[i] = l.quantity as u32;
                    a.origin[i] = l.origin;
                    a.action[i] = Action::Deliver;
                    self.commitments.trade_pickup(i, t as u32, kind);
                    self.stats.trades += 1;
                    self.people[i].trades += 1;
                } else {
                    self.commitments
                        .resolve(i, Action::Trade, t as u32, kind, self.tick, false);
                }
            }
            Action::Build
            | Action::Mint
            | Action::Barter
            | Action::WorkForFood
            | Action::Court
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
            | Action::NominateSelf => {}
        }
    }
    fn start_projects(&mut self) {
        for id in 0..self.markets.len() {
            let done = self.markets[id]
                .project
                .as_ref()
                .is_some_and(|p| p.progress >= p.required);
            if done {
                let p = self.markets[id].project.take().unwrap();
                if p.kind == "Housing block" {
                    self.markets[id].housing_blocks += 1;
                    self.stats.buildings += 1;
                } else {
                    if let Some(r) = self.roads.iter_mut().find(|r| {
                        (r.a == id as u32 && r.b == p.destination)
                            || (r.b == id as u32 && r.a == p.destination)
                    }) {
                        r.built = true;
                        r.under_construction = false;
                        self.stats.roads += 1;
                    }
                }
            }
            if self.markets[id].project.is_some() {
                continue;
            }
            let road = self
                .roads
                .iter()
                .find(|r| {
                    !r.built
                        && !r.under_construction
                        && r.traffic >= 12
                        && (r.a == id as u32 || r.b == id as u32)
                })
                .map(|r| if r.a == id as u32 { r.b } else { r.a });
            let housing = self.markets[id].housing_blocks * 32 < self.markets[id].population;
            if !housing && road.is_none() {
                continue;
            }
            let wood = if road.is_some() { 12 } else { 16 };
            let stone = if road.is_some() { 6 } else { 4 };
            let m = &mut self.markets[id];
            if m.stock(1) < wood || m.stock(2) < stone {
                continue;
            }
            for (k, quantity) in [(1, wood), (2, stone)] {
                let mut remaining = quantity;
                while remaining > 0 {
                    let lot = m.take(k, remaining).unwrap();
                    remaining -= lot.quantity;
                }
                self.ledger.built[k] += quantity;
                m.quotes[k].offered = m.quotes[k].offered.min(m.stock(k));
            }
            if let Some(destination) = road {
                if let Some(r) = self.roads.iter_mut().find(|r| {
                    (r.a == id as u32 && r.b == destination)
                        || (r.b == id as u32 && r.a == destination)
                }) {
                    r.under_construction = true;
                }
            }
            m.project = Some(Project {
                kind: if road.is_some() {
                    "Road".into()
                } else {
                    "Housing block".into()
                },
                progress: 0.0,
                required: if road.is_some() { 24.0 } else { 10.0 },
                destination: road.unwrap_or(id as u32),
            });
        }
    }
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let payload = postcard::to_allocvec(&(
            self,
            &self.food_memory,
            &self.forage_memory,
            &self.work_memory,
            &self.commitments,
            &self.map,
            &self.explored,
            &self.governance,
        ))
        .map_err(|e| e.to_string())?;
        let compressed =
            zstd::stream::encode_all(payload.as_slice(), 3).map_err(|e| e.to_string())?;
        let tmp = path.with_extension("tmp");
        let mut data = b"XXSAVE30".to_vec();
        data.extend((payload.len() as u64).to_le_bytes());
        data.extend(compressed);
        fs::write(&tmp, data).map_err(|e| e.to_string())?;
        fs::rename(&tmp, path).map_err(|e| e.to_string())
    }
    pub fn load(path: &Path) -> Result<Self, String> {
        let data = fs::read(path).map_err(|e| e.to_string())?;
        if data.starts_with(b"XXSAVE01")
            || data.starts_with(b"XXSAVE02")
            || data.starts_with(b"XXSAVE03")
        {
            return Err("This save predates day/night, reserved beds and rested-aware brains. Start a new world; the original save is preserved.".into());
        }
        if data.len() < 16
            || (!matches!(&data[..8], b"XXSAVE19" | b"XXSAVE30")
                && &data[..8] != b"XXSAVE18"
                && &data[..8] != b"XXSAVE17"
                && &data[..8] != b"XXSAVE16"
                && &data[..8] != b"XXSAVE15"
                && &data[..8] != b"XXSAVE14"
                && &data[..8] != b"XXSAVE13"
                && &data[..8] != b"XXSAVE12"
                && &data[..8] != b"XXSAVE11"
                && &data[..8] != b"XXSAVE10"
                && &data[..8] != b"XXSAVE09"
                && &data[..8] != b"XXSAVE08"
                && &data[..8] != b"XXSAVE07"
                && &data[..8] != b"XXSAVE06"
                && &data[..8] != b"XXSAVE04"
                && &data[..8] != b"XXSAVE05")
        {
            return Err("Invalid checkpoint header".into());
        }
        let expected = u64::from_le_bytes(data[8..16].try_into().unwrap());
        if expected > 8_000_000_000 {
            return Err("Checkpoint exceeds load budget".into());
        }
        let decoder = zstd::stream::read::Decoder::new(&data[16..]).map_err(|e| e.to_string())?;
        let mut raw = Vec::new();
        decoder
            .take(expected + 1)
            .read_to_end(&mut raw)
            .map_err(|e| e.to_string())?;
        if raw.len() as u64 != expected {
            return Err("Checkpoint length mismatch".into());
        }
        let mut world: Self = if matches!(&data[..8], b"XXSAVE19" | b"XXSAVE30") {
            let ((mut world, purchases, foraging, work, commitments, map, explored), tail): (
                (
                    Self,
                    food_memory::FoodMemory,
                    food_memory::FoodMemory,
                    work_memory::WorkMemory,
                    commitment_memory::Commitments,
                    WorldMap,
                    Exploration,
                ),
                &[u8],
            ) = postcard::take_from_bytes(&raw).map_err(|e| e.to_string())?;
            if &data[..8] == b"XXSAVE30" {
                let (governance, trailing) =
                    postcard::take_from_bytes(tail).map_err(|e| e.to_string())?;
                if !trailing.is_empty() {
                    return Err("Unexpected governance checkpoint data".into());
                }
                world.governance = governance;
            } else if !tail.is_empty() {
                return Err("Unexpected checkpoint data".into());
            }
            world.food_memory = purchases;
            world.forage_memory = foraging;
            world.work_memory = work;
            world.commitments = commitments;
            world.map = map;
            world.explored = explored;
            world
        } else if &data[..8] == b"XXSAVE18" {
            let (mut world, purchases, foraging, work, commitments): (
                Self,
                food_memory::FoodMemory,
                food_memory::FoodMemory,
                work_memory::WorkMemory,
                commitment_memory::Commitments,
            ) = postcard::from_bytes(&raw).map_err(|e| e.to_string())?;
            world.food_memory = purchases;
            world.forage_memory = foraging;
            world.work_memory = work;
            world.commitments = commitments;
            world
        } else if &data[..8] == b"XXSAVE17" {
            let (mut world, purchases, foraging, work): (
                Self,
                food_memory::FoodMemory,
                food_memory::FoodMemory,
                work_memory::WorkMemory,
            ) = postcard::from_bytes(&raw).map_err(|e| e.to_string())?;
            world.food_memory = purchases;
            world.forage_memory = foraging;
            world.work_memory = work;
            world
        } else if &data[..8] == b"XXSAVE16" {
            let (mut world, purchases, foraging): (
                Self,
                food_memory::FoodMemory,
                food_memory::FoodMemory,
            ) = postcard::from_bytes(&raw).map_err(|e| e.to_string())?;
            world.food_memory = purchases;
            world.forage_memory = foraging;
            world
        } else if &data[..8] == b"XXSAVE15" {
            let (mut world, memory) =
                postcard::take_from_bytes::<Self>(&raw).map_err(|e| e.to_string())?;
            // Main used v15 for the widened genome. Early memory builds also used v15
            // and appended a purchase-memory payload to the same serialized world.
            world.map = WorldMap::classic(world.seed);
            if let Ok((spec, remaining)) = postcard::take_from_bytes::<MapSpec>(memory) {
                if spec.seed == world.seed
                    && spec.version == worldmap::GENERATOR_VERSION
                    && (0.0..=spec.width()).contains(&spec.home_x)
                    && (0.0..=spec.height()).contains(&spec.home_y)
                {
                    world.map = WorldMap::new(spec);
                    if !remaining.is_empty() {
                        world.explored =
                            postcard::from_bytes(remaining).map_err(|e| e.to_string())?;
                    }
                } else {
                    world.food_memory = postcard::from_bytes(memory).map_err(|e| e.to_string())?;
                }
            } else if !memory.is_empty() {
                world.food_memory = postcard::from_bytes(memory).map_err(|e| e.to_string())?;
            }
            world
        } else if &data[..8] == b"XXSAVE04" {
            let old: legacy_v4::LegacyWorld =
                postcard::from_bytes(&raw).map_err(|e| e.to_string())?;
            let mut w: World = old.into();
            for i in 0..w.agents.len() {
                if !w.agents.alive[i] {
                    w.register_corpse(i);
                }
            }
            w
        } else if &data[..8] == b"XXSAVE05" {
            let old: legacy_v5::LegacyWorld =
                postcard::from_bytes(&raw).map_err(|e| e.to_string())?;
            old.into()
        } else if &data[..8] == b"XXSAVE06" {
            let old: legacy_v6::LegacyWorld =
                postcard::from_bytes(&raw).map_err(|e| e.to_string())?;
            old.into()
        } else if &data[..8] == b"XXSAVE07" {
            let old: legacy_v7::LegacyWorld =
                postcard::from_bytes(&raw).map_err(|e| e.to_string())?;
            old.into()
        } else if &data[..8] == b"XXSAVE08" || &data[..8] == b"XXSAVE09" {
            let old: legacy_v9::LegacyWorld =
                postcard::from_bytes(&raw).map_err(|e| e.to_string())?;
            old.into()
        } else if &data[..8] == b"XXSAVE10" {
            let old: legacy_v10::LegacyWorld =
                postcard::from_bytes(&raw).map_err(|e| e.to_string())?;
            old.into()
        } else if &data[..8] == b"XXSAVE11" {
            let old: legacy_v11::LegacyWorld =
                postcard::from_bytes(&raw).map_err(|e| e.to_string())?;
            old.into()
        } else if &data[..8] == b"XXSAVE12" {
            let old: legacy_v12::LegacyWorld =
                postcard::from_bytes(&raw).map_err(|e| e.to_string())?;
            old.into()
        } else if &data[..8] == b"XXSAVE13" {
            let old: legacy_v13::LegacyWorld =
                postcard::from_bytes(&raw).map_err(|e| e.to_string())?;
            old.into()
        } else {
            postcard::from_bytes(&raw).map_err(|e| e.to_string())?
        };
        if &data[..8] == b"XXSAVE04" || &data[..8] == b"XXSAVE05" {
            world.upgrade_content()?;
        }
        if &data[..8] == b"XXSAVE04" || &data[..8] == b"XXSAVE05" || &data[..8] == b"XXSAVE06" {
            for i in 0..world.agents.len() {
                let mut r = Rng(world.seed ^ (i as u64).wrapping_mul(0x9e3779b97f4a7c15));
                if r.index(3) == 0 {
                    world.agents.traits[i] |=
                        [life::SOCIALITE, life::GREGARIOUS, life::COMMUNITY_MINDED][r.index(3)];
                }
            }
        }
        if !matches!(&data[..8], b"XXSAVE19" | b"XXSAVE30") && &data[..8] != b"XXSAVE15" {
            world.map = WorldMap::classic(world.seed);
        }
        world.social.resize(world.agents.len(), 0.0);
        world.format = 15;
        if &data[..8] != b"XXSAVE08"
            && &data[..8] != b"XXSAVE09"
            && &data[..8] != b"XXSAVE10"
            && &data[..8] != b"XXSAVE11"
            && &data[..8] != b"XXSAVE12"
            && &data[..8] != b"XXSAVE13"
            && &data[..8] != b"XXSAVE14"
            && &data[..8] != b"XXSAVE15"
            && &data[..8] != b"XXSAVE16"
            && &data[..8] != b"XXSAVE17"
            && &data[..8] != b"XXSAVE18"
            && !matches!(&data[..8], b"XXSAVE19" | b"XXSAVE30")
        {
            world.init_wildlife();
        }
        if &data[..8] != b"XXSAVE09"
            && &data[..8] != b"XXSAVE10"
            && &data[..8] != b"XXSAVE11"
            && &data[..8] != b"XXSAVE12"
            && &data[..8] != b"XXSAVE13"
            && &data[..8] != b"XXSAVE14"
            && &data[..8] != b"XXSAVE15"
            && &data[..8] != b"XXSAVE16"
            && &data[..8] != b"XXSAVE17"
            && &data[..8] != b"XXSAVE18"
            && !matches!(&data[..8], b"XXSAVE19" | b"XXSAVE30")
        {
            world.upgrade_landscape(false);
        } else if &data[..8] == b"XXSAVE09" {
            world.redistribute_forests();
        }
        if &data[..8] != b"XXSAVE10"
            && &data[..8] != b"XXSAVE11"
            && &data[..8] != b"XXSAVE12"
            && &data[..8] != b"XXSAVE13"
            && &data[..8] != b"XXSAVE14"
            && &data[..8] != b"XXSAVE15"
            && &data[..8] != b"XXSAVE16"
            && &data[..8] != b"XXSAVE17"
            && &data[..8] != b"XXSAVE18"
            && !matches!(&data[..8], b"XXSAVE19" | b"XXSAVE30")
        {
            world.relocate_fertile_land();
        }
        world.migrate_brain_schemas();
        world.sync_education();
        world.wildlife.rebuild()?;
        world.sync_councils();
        world.sync_urban();
        world.sync_relationships();
        world.validate()?;
        for genome in &mut world.genomes {
            genome.add_memory_inputs(&mut world.innovations);
        }
        world.brains = world
            .genomes
            .iter()
            .map(|g| g.compile())
            .collect::<Result<Vec<_>, _>>()?;
        Ok(world)
    }
    /// Widens pre-format-15 brains from the 42-sensor schema to the reserved
    /// 48-sensor layout, shifting hidden and output node IDs consistently.
    fn migrate_brain_schemas(&mut self) {
        let delta = (INPUTS - LEGACY_INPUTS) as u32;
        if delta == 0 {
            return;
        }
        let mut migrated = false;
        for g in &mut self.genomes {
            if g.needs_input_migration() {
                g.migrate_inputs(LEGACY_INPUTS);
                migrated = true;
            }
        }
        if migrated {
            self.innovations
                .migrate_node_ids(LEGACY_INPUTS as u32, delta);
        }
        for c in &mut self.councils {
            if c.genome.needs_input_migration() {
                c.genome.migrate_inputs(LEGACY_INPUTS);
                c.innovations.migrate_node_ids(LEGACY_INPUTS as u32, delta);
            }
        }
        let mut wild = false;
        for a in &mut self.wildlife.animals {
            if a.genome.needs_input_migration() {
                a.genome.migrate_inputs(LEGACY_INPUTS);
                wild = true;
            }
        }
        if wild {
            self.wildlife
                .innovations
                .migrate_node_ids(LEGACY_INPUTS as u32, delta);
        }
    }
    fn validate(&self) -> Result<(), String> {
        let n = self.agents.len();
        let a = &self.agents;
        let lengths = [
            a.y.len(),
            a.health.len(),
            a.hunger.len(),
            a.energy.len(),
            a.alive.len(),
            a.coins.len(),
            a.berries.len(),
            a.fish.len(),
            a.cargo.len(),
            a.cargo_kind.len(),
            a.origin.len(),
            a.home.len(),
            a.action.len(),
            a.target.len(),
            a.timer.len(),
            a.last_decision.len(),
            a.convoy.len(),
            a.solo_until.len(),
            a.dwell_cell.len(),
            a.dwell_ticks.len(),
            a.residence.len(),
            a.capacity.len(),
            a.rested_meter.len(),
            a.rested.len(),
            a.sleeping.len(),
            a.rest_site.len(),
            a.bed_slot.len(),
            a.contract.len(),
            a.carried_body.len(),
            a.corpse.len(),
            a.rice.len(),
            a.traits.len(),
            a.tools.len(),
            a.urban_weight.len(),
            self.people.len(),
            self.matches.len(),
            self.genomes.len(),
        ];
        self.hill
            .validate(self.map.spec.width(), self.map.spec.height())?;
        if self.format != 15
            || n == 0
            || n > 1_200_000
            || lengths.iter().any(|l| *l != n)
            || !(3..=64).contains(&self.markets.len())
        {
            return Err("Invalid checkpoint schema or pool lengths".into());
        }
        for i in 0..n {
            if a.home[i] as usize >= self.markets.len()
                || a.cargo_kind[i] as usize >= GOODS
                || !a.x[i].is_finite()
                || !a.y[i].is_finite()
            {
                return Err("Invalid agent state".into());
            }
        }
        for p in &self.patches {
            if p.kind >= GOODS || p.stock > p.capacity {
                return Err("Invalid resource patch".into());
            }
        }
        let mut membership = vec![NONE; n];
        for (slot, c) in self.convoys.iter().enumerate() {
            if let Some(c) = c {
                if c.members.is_empty()
                    || c.members.len() > travel::MAX_CONVOY
                    || ![c.x, c.y, c.goal_x, c.goal_y].iter().all(|v| v.is_finite())
                {
                    return Err("Invalid convoy".into());
                }
                for &id in &c.members {
                    let i = id as usize;
                    if i >= n || membership[i] != NONE || a.convoy[i] != slot as u32 {
                        return Err("Invalid convoy membership".into());
                    }
                    membership[i] = slot as u32;
                }
            }
        }
        if membership != a.convoy {
            return Err("Orphan convoy membership".into());
        }
        self.validate_rest()?;
        self.validate_jobs()?;
        self.validate_people()?;
        self.validate_social()?;
        self.validate_wildlife()?;
        self.validate_governance()?;
        self.validate_education()?;
        self.validate_urban()?;
        self.validate_banking()?;
        self.validate_relationships()?;
        self.check()
    }
    pub fn summary(&self) -> serde_json::Value {
        let mut report = self.snapshot_metadata();
        report["leaderboard"] = self.wealth_leaderboard();
        report
    }

    fn snapshot_metadata(&self) -> serde_json::Value {
        let mut result = serde_json::json!({"seed":self.seed,"tick":self.tick,"clock":self.clock_summary(),"shelters":self.shelter_summary(),"map_width":self.map.spec.width(),"map_height":self.map.spec.height(),"fertility":self.fertility,"farms":self.farm_summary(),"population_life":self.life_summary(),"taverns":self.taverns,"job_boards":self.job_boards(),"graveyards":self.graveyard_summary(),"seconds":self.tick as f32*DT,"allocated_agents":self.agents.len(),"living_agents":self.living(),"stats":self.stats,"convoy_count":self.convoys.iter().flatten().count(),"convoys":self.convoys.iter().enumerate().filter_map(|(slot,c)|c.as_ref().map(|c|serde_json::json!({"slot":slot,"x":c.x,"y":c.y,"members":c.members.len(),"leader":c.members.first(),"departed":c.departed}))).take(2000).collect::<Vec<_>>(),"accounts":self.accounts(),"products":PRODUCTS,"item_weights":WEIGHTS,"sensors":neat_core::SENSORS.as_slice(),"coins":{"initial":self.ledger.initial_coins,"minted":self.ledger.minted_coins,"current":self.coins()},"markets":self.markets.iter().map(|m|{serde_json::json!({"id":m.id,"name":m.name,"x":m.x,"y":m.y,"population":m.population,"housing_blocks":m.housing_blocks,"stable_residents":m.stable_residents,"stage":self.settlement_stage(m.id as usize),"coins":m.coins,"stock":std::array::from_fn::<u64,GOODS,_>(|k|m.stock(k)),"quotes":m.quotes,"sales":m.sales,"local_sales":m.local_sales,"imported_sales":m.imported_sales,"profile":self.observed_market_profile(m.id as usize),"project":m.project})}).collect::<Vec<_>>(),"patches":self.patches,"roads":self.roads});
        result["governance"] = self.governance_summary();
        result["hill"] = serde_json::json!(self.hill);
        result["map"] = serde_json::json!(self.map.spec);
        result["explored_chunks"] = serde_json::json!(self
            .explored
            .chunks
            .iter()
            .map(|s| s.len())
            .collect::<Vec<_>>());
        result["wildlife"] = self.wildlife_summary();
        result["schools"] = self.school_summary();
        result["urban"] = self.urban_summary();
        result["banking"] = self.banking_summary();
        result["relationships"] = self.relationships_summary();
        result["banks"] = result["banking"]["banks"].clone();
        result["education"] = serde_json::json!({"graduates":self.education.graduated,"lessons":self.education.lessons,"bonuses_paid":self.education.bonuses_paid,"educated_people":self.agents.traits.iter().filter(|t|**t&education::EDUCATED!=0).count()});
        result
    }

    pub fn snapshot(&self, limit: usize, selected: Option<usize>, paused: bool) -> Vec<u8> {
        self.snapshot_with_wildlife(limit, selected, paused, None)
    }
    pub fn snapshot_with_wildlife(
        &self,
        limit: usize,
        selected: Option<usize>,
        paused: bool,
        animal: Option<usize>,
    ) -> Vec<u8> {
        self.viewer_snapshot(
            limit,
            selected,
            paused,
            animal,
            Some(&self.wealth_leaderboard()),
        )
    }

    pub fn viewer_snapshot(
        &self,
        limit: usize,
        selected: Option<usize>,
        paused: bool,
        animal: Option<usize>,
        leaderboard: Option<&serde_json::Value>,
    ) -> Vec<u8> {
        let visible_convoys: std::collections::BTreeSet<usize> = self
            .convoys
            .iter()
            .enumerate()
            .filter(|(_, c)| c.is_some())
            .take(2000)
            .map(|(i, _)| i)
            .collect();
        let mut metadata = self.snapshot_metadata();
        // Animals travel as compact records after the people samples; the
        // viewer rebuilds wildlife.animals from them. Full reports keep JSON.
        metadata["wildlife"]
            .as_object_mut()
            .map(|w| w.remove("animals"));
        if let Some(board) = leaderboard {
            metadata["leaderboard"] = board.clone();
        }
        if let Some(id) = selected {
            metadata["selected"] = serde_json::to_value(self.trace(id)).unwrap_or_default();
        }
        if let Some(id) = animal {
            metadata["selected_animal"] = self.animal_trace(id);
        }
        let json = serde_json::to_vec(&metadata).unwrap();
        let count = self.agents.len().min(limit);
        let animal_count = self.wildlife.animals.iter().filter(|a| a.alive).count();
        let mut out = Vec::with_capacity(32 + json.len() + (count + animal_count) * 16);
        out.extend(b"XNS1");
        out.extend(2u16.to_le_bytes());
        out.extend((paused as u16).to_le_bytes());
        out.extend(self.tick.to_le_bytes());
        out.extend((self.agents.len() as u32).to_le_bytes());
        out.extend((count as u32).to_le_bytes());
        out.extend((json.len() as u32).to_le_bytes());
        out.extend((animal_count as u32).to_le_bytes());
        out.extend(json);
        let mut stride = (self.agents.len() / count.max(1)).max(1);
        while gcd(stride, self.agents.len()) != 1 {
            stride += 1;
        }
        for j in 0..count {
            let i = j * stride % self.agents.len();
            out.extend((i as u32).to_le_bytes());
            out.extend(self.agents.x[i].to_le_bytes());
            out.extend(self.agents.y[i].to_le_bytes());
            out.push(
                (if self.agents.action[i] == Action::NominateSelf {
                    Action::Wait
                } else {
                    self.agents.action[i]
                }) as u8
                    | if self.agents.rested[i] { 64 } else { 0 }
                    | if self.agents.alive[i]
                        && visible_convoys.contains(&(self.agents.convoy[i] as usize))
                    {
                        128
                    } else {
                        0
                    },
            );
            out.push(
                self.agents.cargo_kind[i]
                    | if self.agents.sleeping[i] { 128 } else { 0 }
                    | if self.agents.carried_body[i] != NONE {
                        32
                    } else {
                        0
                    }
                    | if self.corpse_hidden(i) { 64 } else { 0 },
            );
            out.push(
                self.agents.health[i].clamp(0.0, 100.0) as u8
                    | if !self.is_adult(i) { 128 } else { 0 },
            );
            out.push(self.agents.home[i] as u8);
        }
        for (id, a) in self.wildlife.animals.iter().enumerate() {
            if !a.alive {
                continue;
            }
            out.extend((id as u32).to_le_bytes());
            out.extend(a.x.to_le_bytes());
            out.extend(a.y.to_le_bytes());
            out.push(a.species);
            out.push(a.action);
            out.push(a.health.clamp(0.0, 100.0) as u8 | if a.adult(self.tick) { 0 } else { 128 });
            out.push(
                (a.heading.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU * 255.0)
                    .round() as u8,
            );
        }
        out
    }
}
fn x_currency_need(coins: u64) -> f32 {
    1.0 / (1.0 + coins as f32 / 20.0)
}
fn gcd(mut a: usize, mut b: usize) -> usize {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a
}
fn dist(ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    (ax - bx).hypot(ay - by)
}
fn segment_distance(x: f32, y: f32, a: &Market, b: &Market) -> f32 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let t = (((x - a.x) * dx + (y - a.y) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
    dist(x, y, a.x + t * dx, a.y + t * dy)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn conservation_survives_parallel_competition() {
        let mut w = World::new(200, 42).unwrap();
        for _ in 0..500 {
            w.step(true);
            w.check().unwrap();
        }
        assert!(w.stats.harvested > 0);
        assert!(w.stats.trades > 0);
    }
    #[test]
    fn serial_and_parallel_results_match() {
        let mut a = World::new(150, 4).unwrap();
        let mut b = a.clone();
        for _ in 0..180 {
            a.step(false);
            b.step(true);
        }
        assert_eq!(a.agents, b.agents);
        assert_eq!(a.markets, b.markets);
        assert_eq!(a.stats, b.stats);
    }
    #[test]
    fn classification_is_observational() {
        let mut a = World::new(90, 5).unwrap();
        let mut b = a.clone();
        b.sales_profile_labels = vec!["port".into(), "timber".into(), "kingdom".into()];
        for _ in 0..100 {
            a.step(false);
            b.step(false);
        }
        assert_eq!(a.agents, b.agents);
        assert_eq!(a.markets, b.markets);
    }
    #[test]
    fn checkpoint_replays_exactly() {
        let mut a = World::new(96, 9).unwrap();
        for _ in 0..55 {
            a.step(false);
        }
        let path =
            std::env::temp_dir().join(format!("xianxia-{}-{}.snapshot", std::process::id(), 9));
        a.save(&path).unwrap();
        let mut b = World::load(&path).unwrap();
        for _ in 0..120 {
            a.step(false);
            b.step(true);
        }
        assert_eq!(a.agents, b.agents);
        assert_eq!(a.markets, b.markets);
        assert_eq!(a.rng, b.rng);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn snapshot_is_bounded_and_versioned() {
        let w = World::new(300, 2).unwrap();
        let bytes = w.snapshot(25, Some(0), true);
        assert_eq!(&bytes[..4], b"XNS1");
        assert_eq!(u16::from_le_bytes(bytes[4..6].try_into().unwrap()), 2);
        assert_eq!(u32::from_le_bytes(bytes[20..24].try_into().unwrap()), 25);
        let jsonlen = u32::from_le_bytes(bytes[24..28].try_into().unwrap()) as usize;
        let animals = u32::from_le_bytes(bytes[28..32].try_into().unwrap()) as usize;
        assert_eq!(animals, w.wildlife.animals.len());
        assert_eq!(bytes.len(), 32 + jsonlen + (25 + animals) * 16);
    }
    #[test]
    fn each_agent_has_distinct_weights_and_valid_brain() {
        let w = World::new(100, 3).unwrap();
        assert_ne!(w.genomes[1], w.genomes[2]);
        assert_eq!(w.brains.len(), w.agents.len());
        assert!(w.genomes.iter().any(|g| g.nodes.len() > 17));
    }
    #[test]
    fn render_sampling_does_not_select_only_one_home_region() {
        let w = World::new(3000, 6).unwrap();
        let bytes = w.snapshot(50, None, true);
        let metadata = u32::from_le_bytes(bytes[24..28].try_into().unwrap()) as usize;
        let mut counts = [0usize; 3];
        let mut seen = std::collections::BTreeSet::new();
        for record in bytes[32 + metadata..32 + metadata + 50 * 16].chunks_exact(16) {
            counts[record[15] as usize] += 1;
            seen.insert(u32::from_le_bytes(record[..4].try_into().unwrap()));
        }
        assert!(counts.iter().all(|n| *n > 0));
        assert_eq!(seen.len(), 50);
    }
    #[test]
    fn competing_settlements_cannot_fund_the_same_road_twice() {
        let mut w = World::new(12, 6).unwrap();
        for m in &mut w.markets {
            m.cancel_quotes();
            m.add(1, 100, m.id);
            m.add(2, 100, m.id);
            w.ledger.initial[1] += 100;
            w.ledger.initial[2] += 100;
        }
        for road in &mut w.roads {
            road.traffic = 50;
        }
        w.start_projects();
        let mut edges = std::collections::BTreeSet::new();
        for m in &w.markets {
            let p = m.project.as_ref().unwrap();
            edges.insert((m.id.min(p.destination), m.id.max(p.destination)));
        }
        assert_eq!(edges.len(), 3);
        assert_eq!(w.ledger.built[1], 36);
        assert_eq!(w.ledger.built[2], 18);
        w.check().unwrap();
    }
    #[test]
    fn unsold_materials_can_fund_housing_without_creating_resources() {
        let mut w = World::new(12, 6).unwrap();
        w.markets[0].add(1, 16, 0);
        w.markets[0].add(2, 4, 0);
        w.ledger.initial[1] += 16;
        w.ledger.initial[2] += 4;
        w.refresh();
        assert_eq!(w.markets[0].quotes[1].offered, 16);
        w.start_projects();
        assert_eq!(w.markets[0].project.as_ref().unwrap().kind, "Housing block");
        assert_eq!(w.markets[0].quotes[1].offered, 0);
        w.check().unwrap();
    }
}

mod memory_pilot;

mod food_memory;

mod work_memory;

mod commitment_memory;
