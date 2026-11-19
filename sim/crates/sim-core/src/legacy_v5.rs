//! Explicit schema-5 reader; preserves state while upgrading content.
use super::*;
use crate::civic::PolicyChoice;
#[derive(Serialize, Deserialize)]
pub struct LegacyAgents {
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
}
#[derive(Serialize, Deserialize)]
pub struct LegacyMarket {
    pub id: u32,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub coins: u64,
    pub goods: [VecDeque<Lot>; 7],
    pub quotes: [Quote; 7],
    pub nearest: [u32; 7],
    pub population: u32,
    pub housing_blocks: u32,
    pub stable_residents: u32,
    pub village: bool,
    pub project: Option<Project>,
    pub sales: [u64; 7],
    pub local_sales: [u64; 7],
    pub imported_sales: [u64; 7],
    pub revenue_total: u64,
    pub expenses_total: u64,
}
#[derive(Serialize, Deserialize)]
pub struct LegacyLedger {
    pub initial: [u64; 7],
    pub renewed: [u64; 7],
    pub consumed: [u64; 7],
    pub built: [u64; 7],
    pub initial_coins: u64,
    pub minted_coins: u64,
    pub minted_ore: [u64; 7],
}
#[derive(Serialize, Deserialize)]
pub struct LegacyStatistics {
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
}
#[derive(Serialize, Deserialize)]
pub struct LegacyWorld {
    pub format: u32,
    pub seed: u64,
    pub rng: Rng,
    pub tick: u64,
    pub agents: LegacyAgents,
    pub genomes: Vec<Genome>,
    pub innovations: Innovations,
    #[serde(skip)]
    pub brains: Vec<Compiled>,
    pub markets: Vec<LegacyMarket>,
    pub patches: Vec<Patch>,
    pub roads: Vec<Road>,
    pub ledger: LegacyLedger,
    pub stats: LegacyStatistics,
    pub sales_profile_labels: Vec<String>,
    pub convoys: Vec<Option<Convoy>>,
    pub shelters: Vec<Shelter>,
    pub clock_offset: u64,
    pub councils: Vec<LegacyCouncil>,
    pub corpses: Vec<Corpse>,
    pub contracts: Vec<Option<Contract>>,
    pub graveyards: Vec<Graveyard>,
}
#[derive(Serialize, Deserialize)]
pub struct LegacyCouncil {
    pub genome: Genome,
    pub innovations: Innovations,
    #[serde(skip)]
    pub brain: Option<Compiled>,
    pub graveyard: u32,
    pub near_dead: u32,
    pub remote_dead: u32,
    pub vacancies: u32,
    pub reward: u64,
    pub escrow: u64,
    pub builder_open: bool,
    pub builder_escrow: u64,
    pub last_filled: u64,
    pub last_revenue: u64,
    pub last_expenses: u64,
    pub income_rate: f32,
    pub expense_rate: f32,
    pub candidates: Vec<u32>,
    pub last_action: String,
    pub blocked: String,
    pub trace: Vec<LegacyPolicyChoice>,
}
#[derive(Serialize, Deserialize)]
pub struct LegacyPolicyChoice {
    pub action: String,
    pub score: f32,
    pub inputs: [f32; 28],
}

pub(super) fn pad<T: Default, const N: usize>(old: [T; N]) -> [T; GOODS] {
    let mut q: VecDeque<T> = old.into_iter().collect();
    std::array::from_fn(|_| q.pop_front().unwrap_or_default())
}
pub(super) fn nearest(old: [u32; 7]) -> [u32; GOODS] {
    std::array::from_fn(|k| if k < 7 { old[k] } else { NONE })
}

impl From<LegacyAgents> for Agents {
    fn from(old: LegacyAgents) -> Self {
        let n = old.x.len();
        Self {
            x: old.x,
            y: old.y,
            health: old.health,
            hunger: old.hunger,
            energy: old.energy,
            alive: old.alive,
            coins: old.coins,
            berries: old.berries,
            fish: old.fish,
            cargo: old.cargo,
            cargo_kind: old.cargo_kind,
            origin: old.origin,
            home: old.home,
            action: old.action,
            target: old.target,
            timer: old.timer,
            last_decision: old.last_decision,
            convoy: old.convoy,
            solo_until: old.solo_until,
            dwell_cell: old.dwell_cell,
            dwell_ticks: old.dwell_ticks,
            residence: old.residence,
            capacity: old.capacity,
            rested_meter: old.rested_meter,
            rested: old.rested,
            sleeping: old.sleeping,
            rest_site: old.rest_site,
            bed_slot: old.bed_slot,
            contract: old.contract,
            carried_body: old.carried_body,
            corpse: old.corpse,
            rice: vec![0; n],
            urban_weight: vec![0; n],
            traits: vec![0; n],
            tools: vec![[0; 4]; n],
        }
    }
}

impl From<LegacyMarket> for Market {
    fn from(old: LegacyMarket) -> Self {
        Self {
            id: old.id,
            name: old.name,
            x: old.x,
            y: old.y,
            coins: old.coins,
            goods: pad(old.goods),
            quotes: pad(old.quotes),
            nearest: nearest(old.nearest),
            population: old.population,
            housing_blocks: old.housing_blocks,
            stable_residents: old.stable_residents,
            village: old.village,
            project: old.project,
            sales: pad(old.sales),
            local_sales: pad(old.local_sales),
            imported_sales: pad(old.imported_sales),
            revenue_total: old.revenue_total,
            expenses_total: old.expenses_total,
        }
    }
}

impl From<LegacyLedger> for Ledger {
    fn from(old: LegacyLedger) -> Self {
        Self {
            initial: pad(old.initial),
            renewed: pad(old.renewed),
            consumed: pad(old.consumed),
            built: pad(old.built),
            initial_coins: old.initial_coins,
            minted_coins: old.minted_coins,
            minted_ore: pad(old.minted_ore),
            crafted_in: [0; GOODS],
            crafted_out: [0; GOODS],
        }
    }
}

impl From<LegacyStatistics> for Statistics {
    fn from(old: LegacyStatistics) -> Self {
        Self {
            decisions: old.decisions,
            trades: old.trades,
            shipments: old.shipments,
            harvested: old.harvested,
            deaths: old.deaths,
            buildings: old.buildings,
            roads: old.roads,
            rejected: old.rejected,
            convoys_departed: old.convoys_departed,
            solo_departures: old.solo_departures,
            camps_founded: old.camps_founded,
            barter_trades: old.barter_trades,
            paid_services: old.paid_services,
            food_services: old.food_services,
            mintings: old.mintings,
            burials: old.burials,
            graveyards_built: old.graveyards_built,
            jobs_completed: old.jobs_completed,
            jobs_cancelled: old.jobs_cancelled,
            births: 0,
            marriages: 0,
            goals_completed: 0,
            tools_crafted: 0,
            farm_shifts: 0,
            rice_harvested: 0,
        }
    }
}

impl From<LegacyWorld> for World {
    fn from(old: LegacyWorld) -> Self {
        Self {
            format: 6,
            seed: old.seed,
            rng: old.rng,
            tick: old.tick,
            agents: old.agents.into(),
            genomes: old.genomes,
            innovations: old.innovations,
            brains: old.brains,
            markets: old.markets.into_iter().map(Into::into).collect(),
            patches: old.patches,
            roads: old.roads,
            ledger: old.ledger.into(),
            stats: old.stats.into(),
            sales_profile_labels: old.sales_profile_labels,
            convoys: old.convoys,
            shelters: old.shelters,
            clock_offset: old.clock_offset,
            councils: old.councils.into_iter().map(Into::into).collect(),
            corpses: old.corpses,
            contracts: old.contracts,
            graveyards: old.graveyards,
            people: vec![],
            matches: vec![],
            farms: vec![],
            fertility: vec![],
            social: vec![],
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
            hill: Hill::default(),
        }
    }
}

impl From<LegacyCouncil> for Council {
    fn from(old: LegacyCouncil) -> Self {
        Self {
            genome: old.genome,
            innovations: old.innovations,
            brain: old.brain,
            graveyard: old.graveyard,
            near_dead: old.near_dead,
            remote_dead: old.remote_dead,
            vacancies: old.vacancies,
            reward: old.reward,
            escrow: old.escrow,
            builder_open: old.builder_open,
            builder_escrow: old.builder_escrow,
            last_filled: old.last_filled,
            last_revenue: old.last_revenue,
            last_expenses: old.last_expenses,
            income_rate: old.income_rate,
            expense_rate: old.expense_rate,
            candidates: old.candidates,
            last_action: old.last_action,
            blocked: old.blocked,
            trace: old.trace.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<LegacyPolicyChoice> for PolicyChoice {
    fn from(old: LegacyPolicyChoice) -> Self {
        Self {
            action: old.action,
            score: old.score,
            inputs: (0..INPUTS)
                .map(|k| if k < 28 { old.inputs[k] } else { 0.0 })
                .collect(),
        }
    }
}
