//! Packed native agent state and an authoritative, integer-accounted economy.
use neat_core::{Compiled, Genome, Innovations, Rng, INPUTS};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, fs, io::Read, path::Path};
mod agriculture;
mod banking;
mod legacy_v12;
mod legacy_v13;
pub use landscape::Hill;
pub use worldmap::{Exploration, MapSize, MapSpec, Terrain, TerrainRaster, WorldMap, CHUNK_SIZE};
mod camp_rumours;
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
mod hunting;
pub use hunting::HuntingStats;
mod industry;
mod landscape;
mod legacy_v4;
mod legacy_v5;
mod legacy_v6;
mod legacy_v7;
mod wildlife;
mod worldmap;
use wildlife::Wildlife;
mod landmarks;
mod life;
mod market_information;
mod meat;
mod needs;
mod social;
use agriculture::{Farm, FertileRegion};
use civic::{Contract, Corpse, Council, Graveyard};
use life::Person;
use social::Tavern;
mod rest;
use rest::Shelter;
mod travel;
use travel::Convoy;
pub use travel::{Ship, TrafficState, WaterState};

pub const DT: f32 = 0.1;
pub const GOODS: usize = 13;
pub const CANDIDATES: usize = 64;
/// Settlements a checkpoint can hold; the wire encodes home markets in one
/// byte and settlement founding stops below this so saves always reload.
pub const MAX_MARKETS: usize = 64;
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
// Action IDs are append-only. Packed frames map newer IDs to a legacy activity.
// Reserved ranges:
// 39-41 hunting/combat, 42-45 taming and husbandry, 46-49 guild quests,
// 50-53 founding and migration, 54-57 qi and cultivation, 58 bow crafting.
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
    HuntWildlife = 39,
    EatCarcass = 40,
    CraftSpear = 41,
    CraftBow = 58,
    Recover = 59,
    Play = 60,
    CollectMeat = 61,
    SellMeat = 62,
    BuyMeat = 63,
    BuyInformation = 64,
    SellInformation = 65,
    NominateSelf = 66,
    DiscussCamp = 67,
    MigrateToCamp = 68,
    DeliverWaterReport = 69,
}
impl Action {
    /// Activity shown on the wire: frames pack the action into six bits, so
    /// variants at 64+ must be mapped to a legacy activity here — the
    /// `every_action_remapped_below_the_six_bit_wire_budget` test enforces
    /// that this match covers every such variant.
    pub fn wire(self) -> Action {
        match self {
            Action::BuyInformation | Action::SellInformation | Action::DiscussCamp => {
                Action::Socialize
            }
            Action::NominateSelf | Action::MigrateToCamp | Action::DeliverWaterReport => {
                Action::Wait
            }
            action => action,
        }
    }
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
    /// Yesterday's daytime mover census on this corridor: agent
    /// observations plus delivered goods, refreshed at each dawn
    /// rollover. Gates the road project in `start_projects`.
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
/// Authoritative world state. Checkpoint schema: fields through
/// `relationships` serialize inside the base postcard struct; anything added
/// after them MUST be `#[serde(skip)]` and appended to `checkpoint_payload`'s
/// tail tuples instead — adding a normal field here breaks deserialization
/// of every save >= v19.
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
    #[serde(skip)]
    camp_rumours: camp_rumours::CampRumours,
    #[serde(skip)]
    carcasses: std::collections::BTreeMap<u32, u64>,
    #[serde(skip)]
    meat_trade: meat::MeatTrade,
    #[serde(skip)]
    market_information: market_information::MarketInformation,
    /// Road corridors that detour around water on generated maps.
    /// A pure function of the map and road endpoints, so it is rebuilt
    /// after loading instead of being serialized.
    #[serde(skip)]
    road_paths: Vec<Vec<(f32, f32)>>,
    /// Stranded-at-water reports on their way to and held by villages.
    /// Serialized as a checkpoint tail, not in the base struct.
    #[serde(skip)]
    pub water: WaterState,
    /// Daytime mover counts per market corridor, folded into
    /// `Road.traffic` at dawn. Serialized as a checkpoint tail.
    #[serde(skip)]
    pub traffic: TrafficState,
    #[serde(skip)]
    landmarks: Vec<landmarks::Landmark>,
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
    memory_offers: Option<[u16; memory_pilot::ACTIONS]>,
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
    pub action: String,
    pub action_target: u32,
    pub action_label: String,
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

/// A checkpoint's save version with its layout capabilities. `World::load`
/// parses the header once into this and every parse branch and migration
/// gate below reads from it — bumping the save format means extending this
/// table, not eight hand-maintained header lists.
struct SaveVersion(u8);
impl SaveVersion {
    const CURRENT: u8 = 34;
    /// Parses the `XXSAVEnn` magic into a supported version number.
    fn parse(data: &[u8]) -> Result<Self, String> {
        let ok = data.len() >= 16
            && &data[..6] == b"XXSAVE"
            && data[6].is_ascii_digit()
            && data[7].is_ascii_digit()
            && (4..=Self::CURRENT).contains(&(10 * (data[6] - b'0') + (data[7] - b'0')));
        if !ok {
            return Err("Invalid checkpoint header".into());
        }
        Ok(Self(10 * (data[6] - b'0') + (data[7] - b'0')))
    }
    /// Development format: base tuple + version-appended tails (v20-29, v34).
    fn dev_family(&self) -> bool {
        matches!(self.0, 20..=29 | 34)
    }
    /// Release format: older tuple + action remap (v19, v30-33).
    fn release_family(&self) -> bool {
        self.0 == 19 || (30..=33).contains(&self.0)
    }
    /// Versions that don't carry a map and get the classic layout
    /// regenerated (v4-14, 16-18; v15 and v19+ load their own).
    fn needs_classic_map(&self) -> bool {
        self.0 <= 14 || (16..=18).contains(&self.0)
    }
    /// Versions that stored wildlife spawns in a tail — don't reset them
    /// (v21-29, v34). Release-family and earlier saves get a fresh spawn set.
    fn has_tail_spawns(&self) -> bool {
        matches!(self.0, 21..=29 | 34)
    }
    /// Versions that stored terrain spawn points/habitats in a tail
    /// (v22-29, v34); everything else derives them on load.
    fn has_tail_spawn_habitats(&self) -> bool {
        matches!(self.0, 22..=29 | 34)
    }
    /// Versions that stored governance state (v30-34); older saves run one
    /// governance step on load to establish councils.
    fn has_governance(&self) -> bool {
        self.0 >= 30
    }
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
            camp_rumours: Default::default(),
            carcasses: Default::default(),
            meat_trade: Default::default(),
            market_information: Default::default(),
            road_paths: Vec::new(),
            water: WaterState::default(),
            traffic: TrafficState::default(),
            landmarks: Vec::new(),
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
            let mut sx = (m.x + w.rng.signed() * 35.0).clamp(5.0, bx);
            let mut sy = (m.y + w.rng.signed() * 35.0).clamp(5.0, by);
            // Shoreline market circles put part of the +-35 spawn range
            // into water (635 of 10,000 on X4 seed 42): an agent born on
            // unbuildable ground can never take a steering step, so
            // redraw to land and only nudge dry when the edge is all wet.
            if size != MapSize::Classic {
                for _ in 0..16 {
                    if w.map.buildable(sx, sy) {
                        break;
                    }
                    sx = (m.x + w.rng.signed() * 35.0).clamp(5.0, bx);
                    sy = (m.y + w.rng.signed() * 35.0).clamp(5.0, by);
                }
                if !w.map.buildable(sx, sy) {
                    (sx, sy) = w.map.dry_ground(sx, sy);
                }
                if !w.map.buildable(sx, sy) {
                    (sx, sy) = (m.x, m.y);
                }
            }
            a.x.push(sx);
            a.y.push(sy);
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
            genome.connect_need_inputs(&mut w.innovations);
            if i % 7 == 0 {
                genome.add_node(&mut w.rng, &mut w.innovations);
            }
            if i % 11 == 0 {
                genome.add_connection(&mut w.rng, &mut w.innovations);
            }
            genome.add_memory_inputs(&mut w.innovations);
            genome.connect_food_readiness(&mut w.innovations);
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
        w.init_terrain_spawn_points();
        w.sync_wildlife_spawn_habitats();
        w.relocate_fertile_land();
        w.sync_education();
        w.init_landmarks();
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
        self.validate_schema()?;
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
        self.validate_camp_rumours()?;
        self.validate_meat_trade()?;
        self.validate_market_information()?;
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
        if self.in_energy_emergency(i) {
            let recovery = self.recovery_offers(i);
            let count = recovery.len();
            out[..count].copy_from_slice(&recovery);
            return (out, count);
        }
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
        if self.needs_food_trip(i) {
            if self.map.spec.size == MapSize::Classic {
                if let Some(patch) = self.closest_food_patch(i) {
                    out[0] = Candidate {
                        action: Action::ForageMeal,
                        target: patch as u32,
                        kind: self.patches[patch].kind,
                        inputs: [0.0; INPUTS],
                        score: 10.0,
                    };
                    return (out, 1);
                }
            }
            let mut food = self.urgent_food_offers(i);
            food.extend(self.water_report_offer(i));
            if !food.is_empty() {
                let count = food.len();
                out[..count].copy_from_slice(&food);
                return (out, count);
            }
            // No stocked patch this mover can reach — head for the home
            // market: whatever food, family or work waits there beats
            // starving in place on a dead shore. If the way home is itself
            // water-blocked the unreachable filter drops this candidate.
            let home = self.agents.home[i] as usize;
            if home < self.markets.len()
                && dist(
                    self.agents.x[i],
                    self.agents.y[i],
                    self.markets[home].x,
                    self.markets[home].y,
                ) > 48.0
            {
                // Aim at whatever food kind the market actually has on the
                // shelf; with empty shelves the walk home is still better
                // than starving where the mover stands.
                let kind = [0usize, 3, 8]
                    .into_iter()
                    .find(|&k| self.markets[home].stock(k) > 0)
                    .unwrap_or(0);
                out[0] = Candidate {
                    action: Action::BuyFood,
                    target: home as u32,
                    kind,
                    inputs: [0.0; INPUTS],
                    score: 9.0,
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
        for offer in self.camp_rumour_offers(i).into_iter().take(limit - count) {
            out[count] = offer;
            count += 1;
        }
        let a = &self.agents;
        let home = a.home[i] as usize;
        let m = &self.markets[home];
        let home_quotes: [Quote; GOODS] = std::array::from_fn(|k| self.perceived_quote(i, m, k));
        let hunger = (a.hunger[i] / 100.0).clamp(0.0, 1.0);
        let held = a.berries[i] + a.fish[i] + a.rice[i];
        let needs = self.need_priorities(i);
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
            x[2] = risk
                + match action {
                    Action::Trade => {
                        self.market_information_risk(i, m)
                            + self.market_information_risk(i, &self.markets[target as usize])
                    }
                    Action::BuyFood
                    | Action::Deliver
                    | Action::Barter
                    | Action::WorkForFood
                    | Action::BuyTool
                    | Action::BuyMeat
                    | Action::SellMeat => {
                        self.market_information_risk(i, &self.markets[target as usize])
                    }
                    _ => 0.0,
                };
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
                (a.coins[i] as f32 / home_quotes[kind].ask.max(1) as f32).min(1.0)
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
            needs.inputs(action, a.hunger[i], &mut x);
            let food_price = if action == Action::BuyFood {
                home_quotes[kind].ask as f32
            } else {
                -price
            };
            if self.map.spec.size != MapSize::Classic || action == Action::HarvestRice {
                x[neat_core::FOOD_READINESS_INPUT] =
                    Self::food_readiness(action, travel, food_price);
            }
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
                .filter(|m| self.perceived_quote(i, m, kind).remaining > 0)
                .max_by(|x, y| {
                    let sx = self.perceived_quote(i, x, kind).bid as f32
                        - dist(a.x[i], a.y[i], x.x, x.y) / 350.0;
                    let sy = self.perceived_quote(i, y, kind).bid as f32
                        - dist(a.x[i], a.y[i], y.x, y.y) / 350.0;
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
                    self.perceived_quote(i, dst, kind).bid as f32,
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
                    home_quotes[kind].bid as f32,
                    0.0,
                );
            }
        }
        if held < 3 && a.cargo[i] == 0 {
            for kind in [0, 3, 8] {
                let q = &home_quotes[kind];
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
                let q = &home_quotes[kind];
                if q.offered == 0 || a.coins[i] < q.ask || a.room(i) < WEIGHTS[kind] {
                    continue;
                }
                for dst in &self.markets {
                    if dst.id == m.id || self.perceived_quote(i, dst, kind).remaining == 0 {
                        continue;
                    }
                    let gain = self.perceived_quote(i, dst, kind).bid as f32
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
                .filter(|dst| self.perceived_barter_terms(i, dst).is_some())
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
                .any(|&k| self.perceived_market_stock(i, m, k) > 0 && a.room(i) >= WEIGHTS[k])
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
                    && self.perceived_market_stock(i, m, kind) > 0
                    && a.coins[i] >= home_quotes[kind].ask
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
                        -(home_quotes[kind].ask as f32),
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
        if a.hunger[i] > 30.0 || a.energy[i] < 30.0 && a.hunger[i] > 15.0 {
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
        } else if self.social[i] < 55.0 && m.population > 1 && a.cargo[i] == 0 {
            add(
                Action::Socialize,
                NONE,
                0,
                0.05,
                0.15,
                0.0,
                dist(a.x[i], a.y[i], m.x, m.y),
                0.0,
                0.0,
            );
        }
        if self.people[i].happiness < 65.0 {
            add(Action::Play, NONE, 0, 0.0, 0.15, 0.0, 0.0, 0.0, 0.0);
            if let Some((id, distance)) = self
                .landmarks
                .iter()
                .enumerate()
                .map(|(id, site)| (id, dist(a.x[i], a.y[i], site.x, site.y)))
                .filter(|&(_, distance)| distance <= 155.0)
                .min_by(|a, b| a.1.total_cmp(&b.1))
            {
                add(
                    Action::Play,
                    id as u32,
                    0,
                    0.0,
                    0.33,
                    0.0,
                    distance,
                    0.0,
                    0.0,
                );
            }
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
        for c in self.hunting_offers(i) {
            add(
                c.action,
                c.target,
                c.kind,
                c.inputs[0],
                c.inputs[1],
                c.inputs[2],
                c.inputs[3],
                0.0,
                0.0,
            );
        }
        if self.map.spec.size != MapSize::Classic {
            for c in self.water_report_offer(i).into_iter().chain(self.information_offers(i)) {
                add(
                    c.action,
                    c.target,
                    c.kind,
                    c.inputs[0],
                    c.inputs[1],
                    0.0,
                    c.inputs[3],
                    c.inputs[4],
                    0.0,
                );
            }
            for c in self.meat_offers(i) {
                add(
                    c.action,
                    c.target,
                    c.kind,
                    c.inputs[0],
                    c.inputs[1],
                    0.0,
                    c.inputs[3],
                    c.inputs[4],
                    0.0,
                );
            }
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
        } else if a.energy[i] < 18.0 {
            add(Action::Recover, NONE, 0, 0.2, 0.0, 0.0, 0.0, 0.0, 0.0);
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
            let mut tally = [0; memory_pilot::ACTIONS];
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
        let best = out[..count]
            .iter()
            .filter(|c| !self.target_unreachable(i, c.action, c.target))
            .max_by(|a, b| a.score.total_cmp(&b.score))
            .copied();
        // A starving mover offered nothing better than Wait still walks:
        // any move toward a market or a job beats standing on a dead shore
        // until the hunger meter runs out.
        let best = if self.agents.hunger[i] >= 55.0
            && best.is_some_and(|c| c.action == Action::Wait)
        {
            out[..count]
                .iter()
                .filter(|c| {
                    c.action != Action::Wait
                        && !self.target_unreachable(i, c.action, c.target)
                })
                .max_by(|a, b| a.score.total_cmp(&b.score))
                .copied()
                .or(best)
        } else {
            best
        };
        // When every offer is water-blocked the mover has nothing better
        // to try — keep its best anyway rather than freeze the decision.
        best.or_else(|| {
            out[..count]
                .iter()
                .max_by(|a, b| a.score.total_cmp(&b.score))
                .copied()
        })
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
        let mut person = self.person_trace(i);
        person["need_priorities"] = self.need_priorities(i).trace(self.in_energy_emergency(i));
        person["carried_meat"] = self.meat_trade.pack(i).into();
        person["carried_food"] = serde_json::json!({
            "berries": self.agents.berries[i],
            "fish": self.agents.fish[i],
            "rice": self.agents.rice[i],
        });
        if let Some(tools) = person["tools"].as_array_mut() {
            if self.wildlife.spears.contains(&(i as u32)) {
                tools.push("wooden spear".into());
            }
            if self.wildlife.bows.contains(&(i as u32)) {
                tools.push("hunting bow".into());
            }
        }
        let mut choices: Vec<_> = out[..count]
            .iter()
            .map(|c| TraceChoice {
                action: format!("{:?}", c.action),
                target: c.target,
                product: self.choice_label(c),
                score: c.score,
                inputs: c.inputs.to_vec(),
                memory_inputs: self.action_memory_inputs(i, c.action, c.target),
            })
            .collect();
        choices.sort_by(|a, b| b.score.total_cmp(&a.score));
        let a = &self.agents;
        let committed = Candidate {
            action: a.action[i],
            target: a.target[i],
            kind: self
                .patches
                .get(a.target[i] as usize)
                .map(|p| p.kind)
                .unwrap_or(a.cargo_kind[i] as usize),
            ..Default::default()
        };
        Some(AgentTrace {
            x: a.x[i],
            y: a.y[i],
            action: format!("{:?}", a.action[i]),
            action_target: a.target[i],
            action_label: self.choice_label(&committed),
            person,
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
    fn choice_label(&self, c: &Candidate) -> String {
        if c.action == Action::MigrateToCamp {
            self.markets[c.target as usize].name.clone()
        } else if c.action == Action::DiscussCamp {
            format!(
                "{} tavern",
                self.markets[self.taverns[c.target as usize].market as usize].name
            )
        } else if self.is_urban_action(c.action) {
            self.urban_choice_name(c)
        } else if matches!(c.action, Action::HuntWildlife | Action::EatCarcass) {
            self.wildlife
                .animals
                .get(c.target as usize)
                .map(|animal| wildlife::SpeciesDef::of(animal.species).name)
                .unwrap_or("wildlife")
                .into()
        } else if c.action == Action::CraftSpear {
            "wooden spear".into()
        } else if c.action == Action::CraftBow {
            "hunting bow".into()
        } else if matches!(
            c.action,
            Action::CollectMeat | Action::SellMeat | Action::BuyMeat
        ) {
            "meat".into()
        } else if c.action == Action::Play {
            self.landmarks
                .get(c.target as usize)
                .map(|site| site.name)
                .unwrap_or("")
                .into()
        } else if matches!(c.action, Action::Recover | Action::Socialize) {
            String::new()
        } else if matches!(c.action, Action::BuyInformation | Action::SellInformation) {
            self.markets
                .get((c.target % market_information::MARKET_SLOTS) as usize)
                .map(|market| market.name.clone())
                .unwrap_or_default()
        } else {
            PRODUCTS[c.kind].into()
        }
    }
    pub fn step(&mut self, parallel: bool) {
        self.tick += 1;
        self.expire_ferry_routes();
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
            if self.agents.alive[i] {
                self.eat_packed_meat(i);
            }
            let hunger_per_second = self.hunger_per_second(i);
            let a = &mut self.agents;
            if !a.alive[i] {
                continue;
            }
            if !a.sleeping[i] {
                a.rested_meter[i] = (a.rested_meter[i] - DT).max(0.0);
                if a.rested_meter[i] <= 0.0 {
                    a.rested[i] = false;
                }
                a.spend_energy(i, DT * 0.08);
            }
            a.hunger[i] += DT * hunger_per_second;
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
                    a.energy[i] = (a.energy[i] + needs::MEAL_ENERGY).min(100.0);
                    self.ledger.consumed[kind] += 1;
                }
            }
            if a.hunger[i] > 85.0 {
                a.health[i] -= DT * 0.8;
            }
            if a.energy[i] <= 0.0 {
                a.health[i] -= DT * needs::EXHAUSTION_DAMAGE_PER_SECOND;
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
        self.expire_market_reports();
        let start = self.tick as usize % 10;
        for i in (start..self.agents.len()).step_by(10) {
            self.observe_local_market(i);
            if self.agents.alive[i] && self.in_energy_emergency(i) {
                self.pause_urban_job(i);
                if self.agents.contract[i] != NONE {
                    self.cancel_job(self.agents.contract[i] as usize);
                }
            }
        }
        let due = self.agents.len().saturating_sub(start).div_ceil(10);
        self.expire_camp_rumours();
        for i in (start..self.agents.len()).step_by(10) {
            self.observe_camp_rumours(i);
        }
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
        self.update_water_reports();
        self.plan_ports();
        self.ship_step();
        // Traffic census: daytime movers accrue corridor observations;
        // the dawn rollover publishes them to Road.traffic so busy
        // corridors get road projects while quiet ones go dormant.
        if self.tick % rest::DAY_TICKS == 0 {
            self.publish_traffic();
        }
        if self.tick % 120 == 0 && !self.is_night() {
            self.sample_traffic();
        }
        if self.tick % 10 == 0 {
            self.refresh();
        }
    }
    fn execute(&mut self, i: usize) {
        let action = self.agents.action[i];
        if matches!(action, Action::DiscussCamp | Action::MigrateToCamp) {
            return self.execute_camp_rumour_action(i, action);
        }
        if action == Action::NominateSelf {
            return self.execute_nomination(i);
        }
        match action {
            Action::Recover => return self.execute_recover(i),
            Action::Play => return self.execute_play(i),
            Action::CollectMeat | Action::SellMeat | Action::BuyMeat => {
                return self.execute_meat(i, action);
            }
            Action::BuyInformation | Action::SellInformation => {
                return self.execute_information(i, action);
            }
            _ => {}
        }
        if matches!(
            action,
            Action::HuntWildlife | Action::EatCarcass | Action::CraftSpear | Action::CraftBow
        ) {
            self.execute_hunting(i, action);
            return;
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
        if action == Action::DeliverWaterReport {
            if self.water.pending.iter().any(|p| p.agent == i as u32) {
                if let Some(market) = self.markets.get(self.agents.target[i] as usize) {
                    let speed = self.actor_speed(i, self.agents.x[i], self.agents.y[i]);
                    self.walk_toward(i, market.x, market.y, speed * DT);
                }
            }
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
        let reach = if action == Action::Gather {
            self.patch_reach(t, 7.0)
        } else {
            7.0
        };
        let d = dist(self.agents.x[i], self.agents.y[i], x, y);
        if d > reach {
            let step = (movement_speed * DT).min(d - (reach - 1.0));
            self.walk_toward(i, x, y, step);
            self.agents.drain_energy(i, DT * 0.07);
            return;
        }
        let a = &mut self.agents;
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
                    let pair = (m.id.min(a.origin[i]), m.id.max(a.origin[i]));
                    *self.traffic.counts.entry(pair).or_default() += q;
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
            | Action::NominateSelf
            | Action::DiscussCamp
            | Action::MigrateToCamp
            | Action::HuntWildlife
            | Action::EatCarcass
            | Action::CraftSpear
            | Action::CraftBow
            | Action::Recover
            | Action::Play
            | Action::CollectMeat
            | Action::SellMeat
            | Action::BuyMeat
            | Action::BuyInformation
            | Action::DeliverWaterReport
            | Action::SellInformation => {}
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
            // traffic is yesterday's daytime mover census (~16 samples
            // per mover-day at the 120-tick interval): 48 is roughly a
            // small caravan or three commuters on the corridor.
            let road = self
                .roads
                .iter()
                .find(|r| {
                    !r.built
                        && !r.under_construction
                        && r.traffic >= 48
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
    pub fn checkpoint_payload(&self) -> Result<Vec<u8>, String> {
        if self.watch.memory_trial.is_some() {
            return Err("The developer memory pilot cannot be checkpointed. Finish the experiment without saving.".into());
        }
        postcard::to_allocvec(&(
            self,
            &self.food_memory,
            &self.forage_memory,
            &self.work_memory,
            &self.commitments,
            &self.map,
            (
                &self.explored,
                &self.carcasses,
                &self.wildlife.spawns,
                &self.wildlife.spears,
                &self.wildlife.bows,
                &self.wildlife.hunting_stats,
                &self.meat_trade,
                &self.market_information,
                &self.meat_trade.demand,
            ),
            (
                &self.governance,
                &self.camp_rumours,
                &self.water,
                &self.traffic,
            ),
        ))
        .map_err(|e| e.to_string())
    }
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let payload = self.checkpoint_payload()?;
        let compressed =
            zstd::stream::encode_all(payload.as_slice(), 3).map_err(|e| e.to_string())?;
        let tmp = path.with_extension("tmp");
        let mut data = b"XXSAVE34".to_vec();
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
        let version = SaveVersion::parse(&data)?;
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
        let mut world: Self = if version.dev_family() {
            let (
                (mut world, purchases, foraging, work, commitments, map, (explored, carcasses)),
                tail,
            ): (
                (
                    Self,
                    food_memory::FoodMemory,
                    food_memory::FoodMemory,
                    work_memory::WorkMemory,
                    commitment_memory::Commitments,
                    WorldMap,
                    (Exploration, std::collections::BTreeMap<u32, u64>),
                ),
                &[u8],
            ) = postcard::take_from_bytes(&raw).map_err(|e| e.to_string())?;
            world.food_memory = purchases;
            world.forage_memory = foraging;
            world.work_memory = work;
            world.commitments = commitments;
            world.map = map;
            world.explored = explored;
            world.carcasses = carcasses;
            // Dev-family tails are appended version by version: each `>=`
            // gate is the version that introduced that tail field.
            if version.0 >= 24 {
                let ((spawns, spears, bows, stats), remaining) =
                    postcard::take_from_bytes(tail).map_err(|e| e.to_string())?;
                if version.0 >= 26 {
                    let (mut trade, rest): (meat::MeatTrade, _) =
                        postcard::take_from_bytes(remaining).map_err(|e| e.to_string())?;
                    if version.0 >= 28 {
                        let (information, trailing) =
                            postcard::take_from_bytes(rest).map_err(|e| e.to_string())?;
                        if version.0 >= 29 {
                            let (demand, rest) =
                                postcard::take_from_bytes(trailing).map_err(|e| e.to_string())?;
                            trade.demand = demand;
                            if version.0 >= 34 {
                                (
                                    world.governance,
                                    world.camp_rumours,
                                    world.water,
                                    world.traffic,
                                ) = postcard::from_bytes(rest).map_err(|e| e.to_string())?;
                            } else if !rest.is_empty() {
                                return Err("Unexpected meat demand checkpoint data".into());
                            }
                        } else if !trailing.is_empty() {
                            return Err("Unexpected market report checkpoint data".into());
                        }
                        world.market_information = information;
                    } else if version.0 == 27 {
                        type OldReport = (u64, Vec<([Quote; GOODS], Quote)>);
                        let _: (Option<OldReport>, Option<OldReport>) =
                            postcard::from_bytes(rest).map_err(|e| e.to_string())?;
                    } else if !rest.is_empty() {
                        return Err("Unexpected meat trade checkpoint data".into());
                    }
                    world.meat_trade = trade;
                } else if !remaining.is_empty() {
                    return Err("Unexpected hunting checkpoint data".into());
                }
                world.wildlife.spawns = spawns;
                world.wildlife.spears = spears;
                world.wildlife.bows = bows;
                world.wildlife.hunting_stats = stats;
            } else if version.0 == 23 {
                let (spawns, spears) = postcard::from_bytes(tail).map_err(|e| e.to_string())?;
                world.wildlife.spawns = spawns;
                world.wildlife.spears = spears;
            } else if version.0 == 22 {
                world.wildlife.spawns = postcard::from_bytes(tail).map_err(|e| e.to_string())?;
            } else if version.0 == 21 {
                world.wildlife.spawns =
                    postcard::from_bytes::<wildlife::LegacyWildlifeSpawns>(tail)
                        .map_err(|e| e.to_string())?
                        .into();
            } else if !tail.is_empty() {
                return Err("Unexpected checkpoint data".into());
            }
            world
        } else if version.release_family() {
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
            if version.0 >= 30 {
                let (governance, trailing) =
                    postcard::take_from_bytes(tail).map_err(|e| e.to_string())?;
                let trailing = if version.0 >= 31 {
                    let (rumours, rest) =
                        postcard::take_from_bytes(trailing).map_err(|e| e.to_string())?;
                    world.camp_rumours = rumours;
                    rest
                } else {
                    trailing
                };
                let trailing = if version.0 >= 32 {
                    let (water, rest) =
                        postcard::take_from_bytes(trailing).map_err(|e| e.to_string())?;
                    world.water = water;
                    rest
                } else {
                    trailing
                };
                let trailing = if version.0 >= 33 {
                    let (traffic, rest) =
                        postcard::take_from_bytes(trailing).map_err(|e| e.to_string())?;
                    world.traffic = traffic;
                    rest
                } else {
                    trailing
                };
                if !trailing.is_empty() {
                    return Err("Unexpected governance checkpoint data".into());
                }
                world.governance = governance;
                // These release formats assigned variant indices 39–41
                // before the development formats' action list was merged.
                for action in &mut world.agents.action {
                    *action = match *action {
                        Action::HuntWildlife => Action::NominateSelf,
                        Action::EatCarcass => Action::DiscussCamp,
                        Action::CraftSpear => Action::MigrateToCamp,
                        other => other,
                    }
                }
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
        } else if version.0 == 18 {
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
        } else if version.0 == 17 {
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
        } else if version.0 == 16 {
            let (mut world, purchases, foraging): (
                Self,
                food_memory::FoodMemory,
                food_memory::FoodMemory,
            ) = postcard::from_bytes(&raw).map_err(|e| e.to_string())?;
            world.food_memory = purchases;
            world.forage_memory = foraging;
            world
        } else if version.0 == 15 {
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
        } else if version.0 == 4 {
            let old: legacy_v4::LegacyWorld =
                postcard::from_bytes(&raw).map_err(|e| e.to_string())?;
            let mut w: World = old.into();
            for i in 0..w.agents.len() {
                if !w.agents.alive[i] {
                    w.register_corpse(i);
                }
            }
            w
        } else if version.0 == 5 {
            let old: legacy_v5::LegacyWorld =
                postcard::from_bytes(&raw).map_err(|e| e.to_string())?;
            old.into()
        } else if version.0 == 6 {
            let old: legacy_v6::LegacyWorld =
                postcard::from_bytes(&raw).map_err(|e| e.to_string())?;
            old.into()
        } else if version.0 == 7 {
            let old: legacy_v7::LegacyWorld =
                postcard::from_bytes(&raw).map_err(|e| e.to_string())?;
            old.into()
        } else if version.0 == 8 || version.0 == 9 {
            let old: legacy_v9::LegacyWorld =
                postcard::from_bytes(&raw).map_err(|e| e.to_string())?;
            old.into()
        } else if version.0 == 10 {
            let old: legacy_v10::LegacyWorld =
                postcard::from_bytes(&raw).map_err(|e| e.to_string())?;
            old.into()
        } else if version.0 == 11 {
            let old: legacy_v11::LegacyWorld =
                postcard::from_bytes(&raw).map_err(|e| e.to_string())?;
            old.into()
        } else if version.0 == 12 {
            let old: legacy_v12::LegacyWorld =
                postcard::from_bytes(&raw).map_err(|e| e.to_string())?;
            old.into()
        } else if version.0 == 13 {
            let old: legacy_v13::LegacyWorld =
                postcard::from_bytes(&raw).map_err(|e| e.to_string())?;
            old.into()
        } else {
            // v14: bare serialized World, no memories or tails.
            postcard::from_bytes(&raw).map_err(|e| e.to_string())?
        };
        if version.0 <= 5 {
            world.upgrade_content()?;
        }
        if version.0 <= 6 {
            for i in 0..world.agents.len() {
                let mut r = Rng(world.seed ^ (i as u64).wrapping_mul(0x9e3779b97f4a7c15));
                if r.index(3) == 0 {
                    world.agents.traits[i] |=
                        [life::SOCIALITE, life::GREGARIOUS, life::COMMUNITY_MINDED][r.index(3)];
                }
            }
        }
        if version.needs_classic_map() {
            world.map = WorldMap::classic(world.seed);
        }
        world.social.resize(world.agents.len(), 0.0);
        world.format = 15;
        if version.0 <= 7 {
            world.init_wildlife();
        }
        if version.0 <= 8 {
            world.upgrade_landscape(false);
        } else if version.0 == 9 {
            world.redistribute_forests();
        }
        if version.0 <= 9 {
            world.relocate_fertile_land();
        }
        if !version.has_tail_spawns() {
            world.wildlife.spawns = Default::default();
        }
        if !version.has_tail_spawn_habitats() {
            world.init_terrain_spawn_points();
        }
        world.migrate_brain_schemas();
        world.sync_education();
        world.wildlife.rebuild()?;
        world.sync_councils();
        world.sync_urban();
        world.sync_relationships();
        world.validate()?;
        world.init_landmarks();
        if !version.has_governance() {
            world.governance_step();
        }
        if !version.has_tail_spawn_habitats() {
            world.sync_wildlife_spawn_habitats();
        }
        for genome in &mut world.genomes {
            genome.add_memory_inputs(&mut world.innovations);
            genome.connect_food_readiness(&mut world.innovations);
        }
        world.brains = world
            .genomes
            .iter()
            .map(|g| g.compile())
            .collect::<Result<Vec<_>, _>>()?;
        Ok(world)
    }
    fn migrate_brain_schemas(&mut self) {
        fn migrate_registry(registry: &mut Innovations, from: usize) {
            registry.migrate_node_ids(from as u32, (INPUTS - from) as u32);
        }

        if let Some(g) = self.genomes.iter().find(|g| g.needs_input_migration()) {
            migrate_registry(&mut self.innovations, g.input_count());
        }
        for g in &mut self.genomes {
            if g.needs_input_migration() {
                g.migrate_inputs(g.input_count());
                g.connect_need_inputs(&mut self.innovations);
            }
        }
        for c in &mut self.councils {
            if c.genome.needs_input_migration() {
                let from = c.genome.input_count();
                c.genome.migrate_inputs(from);
                migrate_registry(&mut c.innovations, from);
            }
        }
        if let Some(a) = self
            .wildlife
            .animals
            .iter()
            .find(|a| a.genome.needs_input_migration())
        {
            migrate_registry(&mut self.wildlife.innovations, a.genome.input_count());
        }
        for a in &mut self.wildlife.animals {
            if a.genome.needs_input_migration() {
                a.genome.migrate_inputs(a.genome.input_count());
            }
        }
    }
    /// The serialized-schema invariant sweep: pool lengths, capacity and
    /// index bounds a world must satisfy to reload. `validate()` gates load
    /// with it and `check()` applies it at runtime too, so a world that can
    /// be saved can also be reloaded.
    fn validate_schema(&self) -> Result<(), String> {
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
            || !(3..=MAX_MARKETS).contains(&self.markets.len())
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
        Ok(())
    }
    fn validate(&self) -> Result<(), String> {
        self.validate_schema()?;
        self.validate_rest()?;
        self.validate_jobs()?;
        self.validate_people()?;
        self.validate_social()?;
        self.validate_wildlife()?;
        self.validate_governance()?;
        self.validate_camp_rumours()?;
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
        self.snapshot_metadata_with_buildings(true)
    }

    fn snapshot_metadata_with_buildings(&self, include_buildings: bool) -> serde_json::Value {
        // Sim-computed house anchors so the viewer reads positions
        // instead of mirroring house_origin's ring layout.
        let houses: Vec<Vec<[f32; 2]>> = self
            .markets
            .iter()
            .map(|m| {
                (0..m.housing_blocks)
                    .map(|h| {
                        let p = rest::house_site(&self.map, m.x, m.y, h);
                        [p.0, p.1]
                    })
                    .collect()
            })
            .collect();
        let markets: Vec<serde_json::Value> = self
            .markets
            .iter()
            .enumerate()
            .map(|(i, m)| {
                serde_json::json!({"id":m.id,"name":m.name,"x":m.x,"y":m.y,"population":m.population,"housing_blocks":m.housing_blocks,"house_positions":houses[i],"stable_residents":m.stable_residents,"stage":self.settlement_stage(m.id as usize),"coins":m.coins,"stock":std::array::from_fn::<u64,GOODS,_>(|k|m.stock(k)),"quotes":m.quotes,"sales":m.sales,"local_sales":m.local_sales,"imported_sales":m.imported_sales,"profile":self.observed_market_profile(m.id as usize),"project":m.project})
            })
            .collect();
        let mut result = serde_json::json!({"seed":self.seed,"tick":self.tick,"clock":self.clock_summary(),"shelters":self.shelter_summary(),"map_width":self.map.spec.width(),"map_height":self.map.spec.height(),"fertility":self.fertility,"farms":self.farm_summary(),"population_life":self.life_summary(),"taverns":self.taverns,"job_boards":self.job_boards(),"graveyards":self.graveyard_summary(),"seconds":self.tick as f32*DT,"allocated_agents":self.agents.len(),"living_agents":self.living(),"stats":self.stats,"convoy_count":self.convoys.iter().flatten().count(),"convoys":self.convoys.iter().enumerate().filter_map(|(slot,c)|c.as_ref().map(|c|serde_json::json!({"slot":slot,"x":c.x,"y":c.y,"members":c.members.len(),"leader":c.members.first(),"departed":c.departed}))).take(2000).collect::<Vec<_>>(),"accounts":self.accounts(),"products":PRODUCTS,"item_weights":WEIGHTS,"sensors":neat_core::SENSORS.as_slice(),"coins":{"initial":self.ledger.initial_coins,"minted":self.ledger.minted_coins,"current":self.coins()},"patches":self.patches,"roads":self.roads.iter().enumerate().map(|(i,r)|{let mut v=serde_json::to_value(r).unwrap();if let Some(p)=self.road_paths.get(i).filter(|p|p.len()>=2){v["path"]=serde_json::json!(p);}v}).collect::<Vec<_>>()});
        result["markets"] = serde_json::json!(markets);
        result["governance"] = self.governance_summary();
        result["camp_rumours"] = self.camp_rumour_summary();
        result["hill"] = serde_json::json!(self.hill);
        result["map"] = serde_json::json!(self.map.spec);
        result["explored_chunks"] = serde_json::json!(self
            .explored
            .chunks
            .iter()
            .map(|s| s.len())
            .collect::<Vec<_>>());
        if self.map.spec.size != MapSize::Classic {
            result["chunk_size"] = serde_json::json!(CHUNK_SIZE);
        }
        result["wildlife"] = self.wildlife_summary();
        result["landmarks"] = serde_json::json!(self.landmarks);
        result["meat_trade"] = self.meat_trade.summary(&self.markets, self.tick);
        result["information_trade"] = self.information_summary();
        if !self.carcasses.is_empty() {
            result["wildlife"]["carcasses"] = self.carcass_sites();
        }
        if self.map.spec.size != MapSize::Classic {
            result["wildlife"]["living_by_species"] =
                serde_json::json!(self.living_wildlife_by_species());
            result["wildlife"]["carcass_meat"] = self.carcasses.values().sum::<u64>().into();
        }
        result["water_blocks"] = serde_json::json!(self.water.blocks);
        result["ports"] = serde_json::json!(self.water.ports);
        result["ships"] = serde_json::json!(self.water.ships);
        result["schools"] = self.school_summary();
        result["urban"] = if include_buildings { self.urban_summary() } else { self.urban_snapshot(false) };
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
        self.viewer_snapshot_with_buildings(limit, selected, paused, animal, leaderboard, true)
    }

    pub fn viewer_snapshot_with_buildings(
        &self,
        limit: usize,
        selected: Option<usize>,
        paused: bool,
        animal: Option<usize>,
        leaderboard: Option<&serde_json::Value>,
        include_buildings: bool,
    ) -> Vec<u8> {
        // Only a departed convoy absorbs its members into the cart marker;
        // rallying members still gather visibly around it.
        let visible_convoys: std::collections::BTreeSet<usize> = self
            .convoys
            .iter()
            .enumerate()
            .filter(|(_, c)| matches!(c, Some(c) if c.departed))
            .take(2000)
            .map(|(i, _)| i)
            .collect();
        let mut metadata = self.snapshot_metadata_with_buildings(include_buildings);
        if !include_buildings {
            metadata["building_stream"] = true.into();
        }
        // Animals travel as compact records after the people samples; the
        // viewer rebuilds wildlife.animals from them. Full reports keep JSON.
        metadata["cached_footpaths"] = serde_json::json!(self.cached_footpaths().collect::<Vec<_>>());
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
            let wire_action = self.agents.action[i].wire();
            out.push(
                wire_action as u8
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
    fn generated_spawns_never_stand_on_unbuildable_ground() {
        // Shoreline market circles put part of the +-35 spawn range into
        // water: 635 of 10,000 spawned wet on X4 seed 42, 426 too deep to
        // ever steer out — frozen crowds from birth.
        for (agents, seed, size) in [
            (3000usize, 7u64, MapSize::X4),
            (10000, 42, MapSize::X4),
            (10000, 42, MapSize::X25),
        ] {
            let w = World::new_with_size(agents, seed, Hill::default(), size).unwrap();
            let wet = (0..w.agents.len())
                .filter(|&i| !w.map.buildable(w.agents.x[i], w.agents.y[i]))
                .count();
            assert_eq!(
                wet, 0,
                "{size:?} seed {seed} spawned {wet} agents unbuildable"
            );
        }
    }
    #[test]
    #[ignore]
    fn diag_wet_spawns() {
        for (agents, seed, size) in [
            (3000usize, 7u64, MapSize::X4),
            (10000, 42, MapSize::X4),
            (10000, 42, MapSize::X25),
        ] {
            let w = World::new_with_size(agents, seed, Hill::default(), size).unwrap();
            let mut wet = 0;
            let mut deep = 0;
            for i in 0..w.agents.len() {
                let (x, y) = (w.agents.x[i], w.agents.y[i]);
                if w.map.terrain_at(x, y) == Terrain::Water {
                    wet += 1;
                    // "deep" = every 6u probe around the spawn is still water,
                    // so no steering step can ever reach dry land.
                    let dry_near = [(6.0, 0.0), (-6.0, 0.0), (0.0, 6.0), (0.0, -6.0)]
                        .iter()
                        .any(|&(ox, oy)| w.map.terrain_at(x + ox, y + oy) != Terrain::Water);
                    if !dry_near {
                        deep += 1;
                    }
                }
            }
            eprintln!("{size:?} seed={seed} agents={agents}: wet_spawns={wet} deep_stuck={deep}");
        }
    }
    #[test]
    #[ignore]
    fn diag_shore_stall() {
        let mut w = World::new_with_size(3000, 42, Hill::default(), MapSize::X4).unwrap();
        let mut prev: Vec<(f32, f32)> = (0..w.agents.len())
            .map(|i| (w.agents.x[i], w.agents.y[i]))
            .collect();
        const NAMES: [&str; 39] = [
            "Rest",
            "Gather",
            "Deliver",
            "BuyFood",
            "Build",
            "Trade",
            "Mint",
            "Barter",
            "WorkForFood",
            "MakeCamp",
            "Wait",
            "BuildGraveyard",
            "Gravedigger",
            "Court",
            "CareFamily",
            "CraftTool",
            "BuyTool",
            "Farm",
            "HarvestRice",
            "EquipTool",
            "ForageMeal",
            "StoreCargo",
            "Socialize",
            "BuildTavern",
            "BuildSchool",
            "Teach",
            "AttendSchool",
            "UrbanWork",
            "LeaseBusiness",
            "OperateBusiness",
            "OrderGoods",
            "BuyHome",
            "RentHome",
            "ShopGoods",
            "Hunt",
            "CollectEstate",
            "BuildBank",
            "BorrowBusiness",
            "RepayLoan",
        ];
        for t in 1..=4000u64 {
            w.step(true);
            if t % 1000 == 0 {
                let wet_farms = w
                    .farms
                    .iter()
                    .filter(|f| w.map.terrain_at(f.x, f.y) == Terrain::Water)
                    .count();
                eprintln!(
                    "  farms={} wet={} detours={} pending={} blocks={} ports={} ships={} queues={}",
                    w.farms.len(),
                    wet_farms,
                    w.water.detours.len(),
                    w.water.pending.len(),
                    w.water.blocks.len(),
                    w.water.ports.len(),
                    w.water.ships.len(),
                    w.water
                        .ports
                        .iter()
                        .map(|p| p.queue.len() + p.queue_far.len())
                        .sum::<usize>()
                );
                let mut hist = [0usize; 70];
                let (mut unmoved, mut alive, mut wet_t) = (0usize, 0usize, 0usize);
                let mut stalls: Vec<String> = Vec::new();
                for i in 0..w.agents.len() {
                    if !w.agents.alive[i] {
                        continue;
                    }
                    alive += 1;
                    let a = w.agents.action[i] as usize;
                    hist[if a < 70 { a } else { 69 }] += 1;
                    if dist(w.agents.x[i], w.agents.y[i], prev[i].0, prev[i].1) < 4.0 {
                        unmoved += 1;
                        let kind = w.agents.cargo_kind[i] as usize;
                        // Resolve the destination the same way execute does.
                        let Some((dx, dy)) = w.personal_destination(i) else {
                            prev[i] = (w.agents.x[i], w.agents.y[i]);
                            continue;
                        };
                        let wet = w.map.terrain_at(dx, dy) == Terrain::Water;
                        wet_t += wet as usize;
                        let shore = w.map.terrain_at(w.agents.x[i] + 8.0, w.agents.y[i])
                            == Terrain::Water
                            || w.map.terrain_at(w.agents.x[i], w.agents.y[i] + 8.0)
                                == Terrain::Water
                            || w.map.terrain_at(w.agents.x[i] - 8.0, w.agents.y[i])
                                == Terrain::Water
                            || w.map.terrain_at(w.agents.x[i], w.agents.y[i] - 8.0)
                                == Terrain::Water;
                        let d = dist(w.agents.x[i], w.agents.y[i], dx, dy);
                        if d > 8.0 && stalls.len() < 10 {
                            stalls.push(format!(
                                "a{i} act={}@{} d={:.0} wet_t={} shore={} convoy={} detour={}",
                                NAMES
                                    .get(w.agents.action[i] as usize)
                                    .copied()
                                    .unwrap_or("?"),
                                kind,
                                d,
                                wet,
                                shore,
                                w.agents.convoy[i],
                                w.water.detours.contains_key(&(i as u32))
                            ));
                        }
                    }
                    prev[i] = (w.agents.x[i], w.agents.y[i]);
                }
                let top: Vec<String> = hist
                    .iter()
                    .enumerate()
                    .filter(|&(_, &c)| c > 0)
                    .map(|(k, &c)| format!("{}:{}", NAMES.get(k).copied().unwrap_or("?"), c))
                    .collect();
                eprintln!(
                    "tick {t}: alive={alive} unmoved={unmoved} wet_targets={wet_t}\n  {}\n  {}",
                    top.join(" "),
                    stalls.join("\n  ")
                );
            }
        }
    }
    #[test]
    #[ignore]
    fn diag_shore_deaths() {
        let mut w = World::new_with_size(3000, 42, Hill::default(), MapSize::X4).unwrap();
        const NAMES: [&str; 39] = [
            "Rest","Gather","Deliver","BuyFood","Build","Trade","Mint","Barter","WorkForFood",
            "MakeCamp","Wait","BuildGraveyard","Gravedigger","Court","CareFamily","CraftTool",
            "BuyTool","Farm","HarvestRice","EquipTool","ForageMeal","StoreCargo","Socialize",
            "BuildTavern","BuildSchool","Teach","AttendSchool","UrbanWork","LeaseBusiness",
            "OperateBusiness","OrderGoods","BuyHome","RentHome","ShopGoods","Hunt",
            "CollectEstate","BuildBank","BorrowBusiness","RepayLoan",
        ];
        let mut seen: Vec<bool> = vec![false; w.agents.len()];
        let mut water_deaths = 0usize;
        let mut shore_deaths = 0usize;
        let mut inland_deaths = 0usize;
        let mut starved_shore = 0usize;
        let mut hist = [0usize; 70];
        let mut clusters: std::collections::BTreeMap<(i32, i32), usize> =
            std::collections::BTreeMap::new();
        let mut samples: Vec<String> = Vec::new();
        for _ in 1..=9600u64 {
            w.step(true);
            for i in 0..w.agents.len() {
                if seen[i] || w.agents.alive[i] {
                    continue;
                }
                seen[i] = true;
                let (x, y) = (w.agents.x[i], w.agents.y[i]);
                let here = w.map.terrain_at(x, y);
                let wet = here == Terrain::Water || here == Terrain::River;
                let shore = !wet && [(24.0, 0.0), (-24.0, 0.0), (0.0, 24.0), (0.0, -24.0)]
                    .iter()
                    .any(|&(ox, oy)| {
                        w.map.terrain_at(x + ox, y + oy) == Terrain::Water
                    });
                let aboard = w.water.ships.iter().any(|s| s.aboard.contains(&(i as u32)));
                let queued = w
                    .water
                    .ports
                    .iter()
                    .any(|p| p.queue.contains(&(i as u32)) || p.queue_far.contains(&(i as u32)));
                let act = NAMES
                    .get(w.agents.action[i] as usize)
                    .copied()
                    .unwrap_or("?");
                hist[(w.agents.action[i] as usize).min(69)] += 1;
                if wet {
                    water_deaths += 1;
                } else if shore || aboard || queued {
                    shore_deaths += 1;
                    *clusters
                        .entry(((x / 96.0) as i32, (y / 96.0) as i32))
                        .or_default() += 1;
                    if w.agents.hunger[i] > 80.0 {
                        starved_shore += 1;
                    }
                    if samples.len() < 15 {
                        let carrying = w.water.pending.iter().any(|p| p.agent == i as u32);
                        let stall = w.water.progress.get(i).and_then(|p| *p).map(|(_, _, s)| w.tick - s);
                        let (mx, my) = {
                            let m = &w.markets[w.agents.home[i] as usize];
                            (m.x, m.y)
                        };
                        let home_d = dist(x, y, mx, my) as i32;
                        let home_ok = w
                            .route_between(x, y, mx, my)
                            .is_some_and(|p| !p.is_empty() || dist(x, y, mx, my) < 48.0);
                        let dest = w.personal_destination(i);
                        let (td, twet) = dest.map(|(dx, dy)| {
                            (dist(x, y, dx, dy) as i32, w.map.terrain_at(dx, dy) == Terrain::Water)
                        }).unwrap_or((-1, false));
                        let home_wet = w
                            .personal_destination(i)
                            .map(|_| {
                                let m = &w.markets[w.agents.home[i] as usize];
                                w.map.terrain_at(m.x, m.y) == Terrain::Water
                            })
                            .unwrap_or(false);
                        samples.push(format!(
                            "a{i} act={act} hunger={:.0} carry={} stall={:?} dest_d={} dest_wet={} home_d={} home_ok={} home_wet={} detour={} blocks={}",
                            w.agents.hunger[i],
                            carrying,
                            stall,
                            td,
                            twet,
                            home_d,
                            home_ok,
                            home_wet,
                            w.water.detours.contains_key(&(i as u32)),
                            w.water.unreachable.get(&(i as u32)).map(|b| b.len()).unwrap_or(0)
                        ));
                    }
                } else {
                    inland_deaths += 1;
                }
            }
        }
        eprintln!(
            "deaths: water={water_deaths} shore={shore_deaths} (starved={starved_shore}) inland={inland_deaths} of {}",
            w.stats.deaths
        );
        let mut top: Vec<_> = clusters.iter().collect();
        top.sort_by(|a, b| b.1.cmp(a.1));
        eprintln!(
            "top shoreline clusters: {}",
            top.iter()
                .take(8)
                .map(|(k, c)| format!("{k:?}:{c}"))
                .collect::<Vec<_>>()
                .join(" ")
        );
        eprintln!(
            "actions: {}",
            hist.iter()
                .enumerate()
                .filter(|&(_, &c)| c > 0)
                .map(|(k, &c)| format!("{}:{}", NAMES.get(k).copied().unwrap_or("?"), c))
                .collect::<Vec<_>>()
                .join(" ")
        );
        let stocked = w
            .patches
            .iter()
            .filter(|p| [0, 3].contains(&p.kind) && p.stock > 0)
            .count();
        eprintln!(
            "patches: {} total, {} stocked food, berry_stock={} fish_stock={}",
            w.patches.len(),
            stocked,
            w.patches.iter().filter(|p| p.kind == 0).map(|p| p.stock).sum::<u64>(),
            w.patches.iter().filter(|p| p.kind == 3).map(|p| p.stock).sum::<u64>()
        );
        eprintln!(
            "ports={} ships={} blocks={} pending={} unreachable={} detours={} probes={} jobs4={}",
            w.water.ports.len(),
            w.water.ships.len(),
            w.water.blocks.len(),
            w.water.pending.len(),
            w.water.unreachable.len(),
            w.water.detours.len(),
            w.water.progress.iter().flatten().count(),
            w.urban.jobs.iter().flatten().filter(|j| j.kind == 4).count()
        );
        eprintln!("{}", samples.join("\n"));
        // What does the offer board look like for a starving Wait-er?
        let mut offer_lines: Vec<String> = Vec::new();
        for i in 0..w.agents.len() {
            if offer_lines.len() >= 10 {
                break;
            }
            if !(w.agents.alive[i]
                && w.agents.action[i] == Action::Wait
                && w.agents.hunger[i] > 85.0)
            {
                continue;
            }
            let mut counts = Counts::new();
            let (out, n) = w.offer_counted(i, Some(&mut counts));
            let board = out[..n]
                .iter()
                .map(|c| format!("{:?}@{}:{:.2}", c.action, c.target, c.score))
                .collect::<Vec<_>>()
                .join(" ");
            offer_lines.push(format!("a{i} hunger={:.0} offers: {board}", w.agents.hunger[i]));
        }
        eprintln!("{}", offer_lines.join("\n"));
        // Why don't the 64 delivered blocks become ports?
        let (mut poor, mut no_wood, mut no_stone, mut ok) = (0, 0, 0, 0);
        for b in &w.water.blocks {
            let m = b.market as usize;
            if m >= w.markets.len() || w.coffers(m) < 96 {
                poor += 1;
            } else if w.markets[m].stock(1) < 20 {
                no_wood += 1;
            } else if w.markets[m].stock(2) < 4 {
                no_stone += 1;
            } else {
                ok += 1;
            }
        }
        eprintln!("block sponsors: poor={poor} no_wood={no_wood} no_stone={no_stone} ready={ok}");
        let per_market: Vec<String> = w
            .markets
            .iter()
            .enumerate()
            .map(|(i, m)| format!("m{i}:c{}:w{}:s{}", w.coffers(i), m.stock(1), m.stock(2)))
            .collect();
        eprintln!("markets: {}", per_market.join(" "));
    }
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
    /// Every Action variant must have a wire encoding below the six-bit
    /// budget after Action::wire() — variants at repr 64+ with no remap arm
    /// would silently corrupt the rested(64)/grouped(128) flag bits.
    #[test]
    fn every_action_remapped_below_the_six_bit_wire_budget() {
        let mut decoded = 0usize;
        for index in 0..128u8 {
            let Ok(action) = postcard::from_bytes::<Action>(&[index]) else {
                break;
            };
            decoded += 1;
            assert!(
                (action.wire() as u8) < 64,
                "{action:?} (repr {}) has no legacy wire mapping",
                action as u8
            );
        }
        // Bump the count when appending variants — and keep wire() covering
        // any repr 64+.
        assert_eq!(decoded, 54);
    }
    /// Postcard stores the declaration index, not the repr. The release-
    /// format remap keys off indices 39-41 decoding to these variants;
    /// inserting or reordering variants above them silently mis-decodes
    /// every release save's action column.
    #[test]
    fn release_remap_declaration_indices_are_pinned() {
        assert_eq!(
            postcard::to_allocvec(&Action::HuntWildlife).unwrap(),
            vec![39u8]
        );
        assert_eq!(postcard::to_allocvec(&Action::EatCarcass).unwrap(), vec![40u8]);
        assert_eq!(postcard::to_allocvec(&Action::CraftSpear).unwrap(), vec![41u8]);
    }
    #[test]
    fn release_checkpoints_restore_their_original_action_indices() {
        let mut world = World::new(8, 17).unwrap();
        // Postcard encodes the declaration index instead of the repr value.
        // These first three gameplay variants occupy the indices used by
        // nomination and migration in the earlier release checkpoint format.
        world.agents.action[..3].copy_from_slice(&[
            Action::HuntWildlife,
            Action::EatCarcass,
            Action::CraftSpear,
        ]);
        let path = std::env::temp_dir().join(format!(
            "xianxia-release-actions-{}.snapshot",
            std::process::id()
        ));
        for (header, expected) in [
            (b"XXSAVE30", &[Action::NominateSelf][..]),
            (
                b"XXSAVE31",
                &[
                    Action::NominateSelf,
                    Action::DiscussCamp,
                    Action::MigrateToCamp,
                ][..],
            ),
            (
                b"XXSAVE32",
                &[
                    Action::NominateSelf,
                    Action::DiscussCamp,
                    Action::MigrateToCamp,
                ][..],
            ),
            (
                b"XXSAVE33",
                &[
                    Action::NominateSelf,
                    Action::DiscussCamp,
                    Action::MigrateToCamp,
                ][..],
            ),
        ] {
            world.agents.action[1..3].copy_from_slice(if expected.len() == 1 {
                &[Action::Wait, Action::Wait]
            } else {
                &[Action::EatCarcass, Action::CraftSpear]
            });
            let mut raw = postcard::to_allocvec(&(
                &world,
                &world.food_memory,
                &world.forage_memory,
                &world.work_memory,
                &world.commitments,
                &world.map,
                &world.explored,
                &world.governance,
            ))
            .unwrap();
            if matches!(header, b"XXSAVE31" | b"XXSAVE32" | b"XXSAVE33") {
                raw.extend(postcard::to_allocvec(&world.camp_rumours).unwrap());
            }
            if matches!(header, b"XXSAVE32" | b"XXSAVE33") {
                raw.extend(postcard::to_allocvec(&world.water).unwrap());
            }
            if header == b"XXSAVE33" {
                raw.extend(postcard::to_allocvec(&world.traffic).unwrap());
            }
            let mut data = header.to_vec();
            data.extend((raw.len() as u64).to_le_bytes());
            data.extend(zstd::stream::encode_all(raw.as_slice(), 3).unwrap());
            std::fs::write(&path, data).unwrap();
            let restored = World::load(&path).unwrap();
            assert_eq!(&restored.agents.action[..expected.len()], expected);
            restored.save(&path).unwrap();
            let reopened = World::load(&path).unwrap();
            assert_eq!(reopened.agents.action, restored.agents.action);
        }
        std::fs::remove_file(path).unwrap();
    }
    /// Pins the version→capability table edges: every migration gate reads
    /// version.0 against these bounds, so a boundary regression flips a
    /// fixup on the wrong save generation.
    #[test]
    fn save_version_capabilities_are_pinned() {
        let v = |n: u8| SaveVersion(n);
        assert!(v(20).dev_family() && !v(30).dev_family() && v(34).dev_family());
        assert!(v(19).release_family() && v(30).release_family() && v(33).release_family());
        assert!(!v(29).release_family() && !v(34).release_family());
        // Classic-map reset: 4-14 and 16-18 get the classic layout; 15 and
        // 19+ carry their own map.
        assert!(v(4).needs_classic_map() && v(14).needs_classic_map());
        assert!(v(16).needs_classic_map() && v(18).needs_classic_map());
        assert!(!v(15).needs_classic_map() && !v(19).needs_classic_map() && !v(34).needs_classic_map());
        // Wildlife spawn tails arrived at v21; terrain spawn points at v22.
        assert!(!v(20).has_tail_spawns() && v(21).has_tail_spawns() && v(34).has_tail_spawns());
        assert!(!v(30).has_tail_spawns());
        assert!(!v(21).has_tail_spawn_habitats() && v(22).has_tail_spawn_habitats());
        assert!(v(34).has_tail_spawn_habitats() && !v(33).has_tail_spawn_habitats());
        // Governance state shipped in v30+; older saves run one step on load.
        assert!(!v(29).has_governance() && v(30).has_governance() && v(34).has_governance());
        // Header parse accepts exactly the supported window.
        assert!(SaveVersion::parse(b"XXSAVE04................").is_ok());
        assert!(SaveVersion::parse(b"XXSAVE34................").is_ok());
        assert!(SaveVersion::parse(b"XXSAVE03................").is_err());
        assert!(SaveVersion::parse(b"XXSAVE35................").is_err());
        assert!(SaveVersion::parse(b"XXSAVE1x................").is_err());
        assert!(SaveVersion::parse(b"XXSAVE04").is_err());
    }
    /// fixtures/dev-v34.snapshot is real binary output (the release
    /// runner loaded bank-v12, stepped 75 ticks, re-saved as XXSAVE34).
    /// It freezes the tuple+tails layout of the development family as
    /// historical bytes: a World serde change that forgets to bump the
    /// format — or a v34 tail-walk edit — fails here instead of passing
    /// on synthesized same-build bytes.
    #[test]
    fn real_dev_format_fixture_loads_and_checks() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../fixtures/dev-v34.snapshot"
        );
        let mut restored = World::load(Path::new(path)).unwrap();
        restored.check().unwrap();
        for _ in 0..3 {
            restored.step(false);
        }
        restored.check().unwrap();
    }
    /// The development family (v20-29, 34) reads one base tuple then
    /// walks a version-dependent tail chain. No real pre-34 bytes
    /// survive (see fixtures/README.md), so this pins the per-version
    /// tail order synthetically: write exactly the elements each
    /// version's reader consumes, in order, and require the load to
    /// place them. Reordering or dropping a tail element in the loader
    /// — or a World layout change without a format bump — fails here.
    #[test]
    fn dev_format_tail_walk_is_pinned() {
        let w = World::new(8, 17).unwrap();
        let base = || {
            postcard::to_allocvec(&(
                &w,
                &w.food_memory,
                &w.forage_memory,
                &w.work_memory,
                &w.commitments,
                &w.map,
                (&w.explored, &w.carcasses),
            ))
            .unwrap()
        };
        let hunt = || {
            postcard::to_allocvec(&(
                &w.wildlife.spawns,
                &w.wildlife.spears,
                &w.wildlife.bows,
                &w.wildlife.hunting_stats,
            ))
            .unwrap()
        };
        let meat = postcard::to_allocvec(&w.meat_trade).unwrap();
        let info = postcard::to_allocvec(&w.market_information).unwrap();
        let demand = postcard::to_allocvec(&w.meat_trade.demand).unwrap();
        // Mirrors World::load's tail walk per version:
        //   v20: nothing             v21: LegacyWildlifeSpawns
        //   v22: spawns              v23: (spawns, spears)
        //   v24-25: hunt4            v26: + meat_trade
        //   v27: + (Option<OldReport>, Option<OldReport>)
        //   v28: + market_information
        //   v29: + demand after market_information
        type OldReport = (u64, Vec<([Quote; GOODS], Quote)>);
        let tails: Vec<(&[u8; 8], Vec<u8>)> = vec![
            (b"XXSAVE20", vec![]),
            (
                b"XXSAVE21",
                // LegacyWildlifeSpawns{points:[{forest,species,vacant}],spawned}
                // — spawned must stay 0: at genesis every animal counts
                // under the ecology ledger's initial_animals.
                postcard::to_allocvec(&(vec![(0usize, 2u8, None::<u64>)], 0u64))
                    .unwrap(),
            ),
            (b"XXSAVE22", postcard::to_allocvec(&w.wildlife.spawns).unwrap()),
            (
                b"XXSAVE23",
                postcard::to_allocvec(&(&w.wildlife.spawns, &w.wildlife.spears))
                    .unwrap(),
            ),
            (b"XXSAVE24", hunt()),
            (b"XXSAVE25", hunt()),
            (b"XXSAVE26", [hunt(), meat.clone()].concat()),
            (
                b"XXSAVE27",
                [
                    hunt(),
                    meat.clone(),
                    postcard::to_allocvec(&(None::<OldReport>, None::<OldReport>))
                        .unwrap(),
                ]
                .concat(),
            ),
            (b"XXSAVE28", [hunt(), meat.clone(), info.clone()].concat()),
            (
                b"XXSAVE29",
                [hunt(), meat.clone(), info.clone(), demand.clone()].concat(),
            ),
        ];
        for (header, tail) in tails {
            let mut raw = base();
            raw.extend(tail);
            let mut data = header.to_vec();
            data.extend((raw.len() as u64).to_le_bytes());
            data.extend(zstd::stream::encode_all(raw.as_slice(), 3).unwrap());
            let path = std::env::temp_dir().join(format!(
                "xianxia-{}-{}.snapshot",
                std::str::from_utf8(&header[6..8]).unwrap(),
                std::process::id()
            ));
            std::fs::write(&path, data).unwrap();
            let restored = World::load(&path).unwrap();
            std::fs::remove_file(&path).unwrap();
            if matches!(
                header,
                b"XXSAVE22"
                    | b"XXSAVE23"
                    | b"XXSAVE24"
                    | b"XXSAVE25"
                    | b"XXSAVE26"
                    | b"XXSAVE27"
                    | b"XXSAVE28"
                    | b"XXSAVE29"
            ) {
                assert_eq!(restored.wildlife.spawns, w.wildlife.spawns);
            }
            if header == b"XXSAVE21" {
                assert_ne!(restored.wildlife.spawns, Default::default());
            }
            if header == b"XXSAVE29" {
                assert_eq!(
                    postcard::to_allocvec(&restored.meat_trade.demand).unwrap(),
                    postcard::to_allocvec(&w.meat_trade.demand).unwrap()
                );
            }
        }
    }
    #[test]
    fn checkpoint_load_preserves_world_state_exactly() {
        // A current-format checkpoint must deserialize without running the
        // legacy init/migration fixups (init_wildlife, upgrade_landscape,
        // relocate_fertile_land, governance_step, map reset). Version guards
        // that miss the newest header re-run those fixups and corrupt fresh
        // state — animals respawn, patches shift, replay diverges.
        let mut w = World::new(64, 42).unwrap();
        for _ in 0..40 {
            w.step(false);
        }
        let path =
            std::env::temp_dir().join(format!("xianxia-load-{}.snapshot", std::process::id()));
        w.save(&path).unwrap();
        let restored = World::load(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(
            postcard::to_allocvec(&w).unwrap(),
            postcard::to_allocvec(&restored).unwrap(),
            "load mutated world state"
        );
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
    /// The legacy readers (v4-13) deserialize old bytes through ~20 types
    /// that are still shared with the live schema — only Agents and Market
    /// got frozen twins. These pins make a serde-layout change in any of
    /// them fail loudly instead of silently reinterpreting real old saves.
    /// A failure means: freeze a legacy twin for the changed type (pattern:
    /// legacy_v13::LegacyAgents / LegacyMarket) or update the pin only when
    /// the byte change was intentional and understood. The `world` pin also
    /// catches non-#[serde(skip)] fields added to the base struct.
    #[test]
    fn legacy_reader_schema_is_pinned() {
        let w = World::new(4, 7).unwrap();
        let pin = |name: &str, bytes: Vec<u8>, expected: usize| {
            assert_eq!(
                bytes.len(),
                expected,
                "{name}: serde layout changed — this type is embedded in \
                 legacy save readers; freeze a legacy twin or update the pin"
            );
        };
        pin("agents", postcard::to_allocvec(&w.agents).unwrap(), 387);
        pin("genomes", postcard::to_allocvec(&w.genomes).unwrap(), 8505);
        pin("innovations", postcard::to_allocvec(&w.innovations).unwrap(), 725);
        pin("markets", postcard::to_allocvec(&w.markets).unwrap(), 546);
        pin("patches", postcard::to_allocvec(&w.patches).unwrap(), 2117);
        pin("roads", postcard::to_allocvec(&w.roads).unwrap(), 16);
        pin("ledger", postcard::to_allocvec(&w.ledger).unwrap(), 105);
        pin("stats", postcard::to_allocvec(&w.stats).unwrap(), 25);
        pin(
            "sales_profile_labels",
            postcard::to_allocvec(&w.sales_profile_labels).unwrap(),
            4,
        );
        pin("shelters", postcard::to_allocvec(&w.shelters).unwrap(), 37);
        pin("councils", postcard::to_allocvec(&w.councils).unwrap(), 8071);
        pin("people", postcard::to_allocvec(&w.people).unwrap(), 205);
        pin("matches", postcard::to_allocvec(&w.matches).unwrap(), 21);
        pin("fertility", postcard::to_allocvec(&w.fertility).unwrap(), 73);
        pin("social", postcard::to_allocvec(&w.social).unwrap(), 17);
        // Single-element pins: an empty Vec serializes to one byte whatever
        // the element layout, so these types need an instance each.
        pin(
            "convoy",
            postcard::to_allocvec(&vec![Some(travel::Convoy {
                members: vec![7],
                x: 1.0,
                y: 2.0,
                goal_x: 3.0,
                goal_y: 4.0,
                created: 5,
                departed: true,
            })])
            .unwrap(),
            22,
        );
        pin(
            "corpse",
            postcard::to_allocvec(&vec![civic::Corpse {
                agent: 1,
                market: 2,
                x: 1.0,
                y: 2.0,
                death_tick: 3,
                status: 4,
                claim: 5,
            }])
            .unwrap(),
            14,
        );
        pin(
            "contract",
            postcard::to_allocvec(&vec![Some(civic::Contract {
                worker: 1,
                market: 2,
                kind: civic::JobKind::Haul,
                corpse: 3,
                graveyard: 4,
                reward: 5,
                phase: 6,
                timer: 7.0,
            })])
            .unwrap(),
            13,
        );
        pin(
            "graveyard",
            postcard::to_allocvec(&vec![civic::Graveyard {
                market: 1,
                x: 1.0,
                y: 2.0,
                progress: 3.0,
                required: 4.0,
                capacity: 5,
                buried: vec![6],
                ready: true,
            }])
            .unwrap(),
            22,
        );
        pin(
            "farm",
            postcard::to_allocvec(&vec![agriculture::Farm {
                market: 1,
                x: 1.0,
                y: 2.0,
                fertility: 3.0,
                moisture: 4.0,
                planted: 5,
                tended_days: 6,
                last_tended: 7,
                mature: true,
                harvest_remaining: 8,
                yield_per_crop: 9,
                escrow: 10,
            }])
            .unwrap(),
            25,
        );
        pin(
            "tavern",
            postcard::to_allocvec(&vec![social::Tavern {
                market: 1,
                x: 1.0,
                y: 2.0,
                ready: true,
                progress: 3.0,
                escrow: 4,
                visitors: 5,
                guests: vec![6],
            }])
            .unwrap(),
            19,
        );
        pin(
            "wildlife",
            postcard::to_allocvec(&w.wildlife).unwrap(),
            539731,
        );
        pin("education", postcard::to_allocvec(&w.education).unwrap(), 27);
        pin("urban", postcard::to_allocvec(&w.urban).unwrap(), 54);
        pin("banking", postcard::to_allocvec(&w.banking).unwrap(), 5);
        pin(
            "relationships",
            postcard::to_allocvec(&w.relationships).unwrap(),
            18,
        );
        pin("rng", postcard::to_allocvec(&w.rng).unwrap(), 10);
        pin("world", postcard::to_allocvec(&w).unwrap(), 560721);
    }
}

mod memory_pilot;

mod food_memory;

mod work_memory;

mod commitment_memory;
