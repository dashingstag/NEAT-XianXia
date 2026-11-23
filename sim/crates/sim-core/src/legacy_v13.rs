//! Explicit schema-13 reader: preserves learned relationships and restores the original hill.
//! Frozen layouts this schema decodes into: LegacyAgents, LegacyAnimal, LegacyAnimalPregnancy,
//!   LegacyAspiration, LegacyBank, LegacyBanking, LegacyBuilding, LegacyContract, LegacyConvoy,
//!   LegacyCorpse, LegacyCouncil, LegacyEcologyLedger, LegacyEducation, LegacyEstate,
//!   LegacyFarm, LegacyFertileRegion, LegacyForest, LegacyGoal, LegacyGoalKind, LegacyGrass,
//!   LegacyGraveyard, LegacyJob, LegacyJobKind, LegacyLedger, LegacyLink, LegacyLoan,
//!   LegacyLot, LegacyMarket, LegacyPatch, LegacyPerson, LegacyPolicyChoice, LegacyPregnancy,
//!   LegacyProgress, LegacyProject, LegacyQuote, LegacyRelationships, LegacyRoad, LegacySchool,
//!   LegacyShelter, LegacyStatistics, LegacyTavern, LegacyTownState, LegacyUrban,
//!   LegacyWildlife, LegacyZone.
//! Live types it still trusts: Action, Gene, Genome, Innovations, Kind, Node, Rng.
//! A serde-layout change to a live type reinterprets this version's saved
//! bytes; legacy_schema::legacy_reader_schemas_are_pinned fails first —
//! freeze a Legacy twin in legacy_types.rs before changing the type (#54).
//! Serialized layouts are frozen here. Changing a live type must not alter these readers.
use super::*;
#[derive(Clone, Debug, Serialize, Deserialize)]
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
    pub rice: Vec<u32>,
    pub traits: Vec<u16>,
    pub tools: Vec<[u8; 4]>,
    pub urban_weight: Vec<u32>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LegacyLot {
    pub quantity: u64,
    pub origin: u32,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct LegacyQuote {
    pub bid: u64,
    pub remaining: u64,
    pub escrow: u64,
    pub ask: u64,
    pub offered: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LegacyProject {
    pub kind: String,
    pub progress: f32,
    pub required: f32,
    pub destination: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LegacyMarket {
    pub id: u32,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub coins: u64,
    pub goods: [VecDeque<LegacyLot>; 13],
    pub quotes: [LegacyQuote; 13],
    pub nearest: [u32; 13],
    pub population: u32,
    pub housing_blocks: u32,
    pub stable_residents: u32,
    pub village: bool,
    pub project: Option<LegacyProject>,
    pub sales: [u64; 13],
    pub local_sales: [u64; 13],
    pub imported_sales: [u64; 13],
    pub revenue_total: u64,
    pub expenses_total: u64,
}
impl From<LegacyAgents> for Agents {
    fn from(old: LegacyAgents) -> Self {
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
            rice: old.rice,
            traits: old.traits,
            tools: old.tools,
            urban_weight: old.urban_weight,
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
            goods: std::array::from_fn(|k| {
                old.goods[k]
                    .iter()
                    .map(|l| Lot {
                        quantity: l.quantity,
                        origin: l.origin,
                    })
                    .collect()
            }),
            quotes: std::array::from_fn(|k| Quote {
                bid: old.quotes[k].bid,
                remaining: old.quotes[k].remaining,
                escrow: old.quotes[k].escrow,
                ask: old.quotes[k].ask,
                offered: old.quotes[k].offered,
            }),
            nearest: old.nearest,
            population: old.population,
            housing_blocks: old.housing_blocks,
            stable_residents: old.stable_residents,
            village: old.village,
            project: old.project.map(|p| Project {
                kind: p.kind,
                progress: p.progress,
                required: p.required,
                destination: p.destination,
            }),
            sales: old.sales,
            local_sales: old.local_sales,
            imported_sales: old.imported_sales,
            revenue_total: old.revenue_total,
            expenses_total: old.expenses_total,
        }
    }
}
impl From<Agents> for LegacyAgents {
    fn from(a: Agents) -> Self {
        Self {
            x: a.x,
            y: a.y,
            health: a.health,
            hunger: a.hunger,
            energy: a.energy,
            alive: a.alive,
            coins: a.coins,
            berries: a.berries,
            fish: a.fish,
            cargo: a.cargo,
            cargo_kind: a.cargo_kind,
            origin: a.origin,
            home: a.home,
            action: a.action,
            target: a.target,
            timer: a.timer,
            last_decision: a.last_decision,
            convoy: a.convoy,
            solo_until: a.solo_until,
            dwell_cell: a.dwell_cell,
            dwell_ticks: a.dwell_ticks,
            residence: a.residence,
            capacity: a.capacity,
            rested_meter: a.rested_meter,
            rested: a.rested,
            sleeping: a.sleeping,
            rest_site: a.rest_site,
            bed_slot: a.bed_slot,
            contract: a.contract,
            carried_body: a.carried_body,
            corpse: a.corpse,
            rice: a.rice,
            traits: a.traits,
            tools: a.tools,
            urban_weight: a.urban_weight,
        }
    }
}
impl From<Market> for LegacyMarket {
    fn from(m: Market) -> Self {
        Self {
            id: m.id,
            name: m.name,
            x: m.x,
            y: m.y,
            coins: m.coins,
            goods: std::array::from_fn(|k| {
                m.goods[k]
                    .iter()
                    .map(|l| LegacyLot {
                        quantity: l.quantity,
                        origin: l.origin,
                    })
                    .collect()
            }),
            quotes: std::array::from_fn(|k| LegacyQuote {
                bid: m.quotes[k].bid,
                remaining: m.quotes[k].remaining,
                escrow: m.quotes[k].escrow,
                ask: m.quotes[k].ask,
                offered: m.quotes[k].offered,
            }),
            nearest: m.nearest,
            population: m.population,
            housing_blocks: m.housing_blocks,
            stable_residents: m.stable_residents,
            village: m.village,
            project: m.project.map(|p| LegacyProject {
                kind: p.kind,
                progress: p.progress,
                required: p.required,
                destination: p.destination,
            }),
            sales: m.sales,
            local_sales: m.local_sales,
            imported_sales: m.imported_sales,
            revenue_total: m.revenue_total,
            expenses_total: m.expenses_total,
        }
    }
}
#[cfg(test)]
pub fn to_legacy_agents(a: &Agents) -> LegacyAgents {
    a.clone().into()
}
#[cfg(test)]
pub fn to_legacy_market(m: &Market) -> LegacyMarket {
    m.clone().into()
}
#[derive(Clone, Debug, Serialize, Deserialize)]
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
    pub patches: Vec<legacy_types::LegacyPatch>,
    pub roads: Vec<legacy_types::LegacyRoad>,
    pub ledger: legacy_types::LegacyLedger,
    pub stats: legacy_types::LegacyStatistics,
    pub sales_profile_labels: Vec<String>,
    pub convoys: Vec<Option<legacy_types::LegacyConvoy>>,
    pub shelters: Vec<legacy_types::LegacyShelter>,
    pub clock_offset: u64,
    pub councils: Vec<legacy_types::LegacyCouncil>,
    pub corpses: Vec<legacy_types::LegacyCorpse>,
    pub contracts: Vec<Option<legacy_types::LegacyContract>>,
    pub graveyards: Vec<legacy_types::LegacyGraveyard>,
    pub people: Vec<legacy_types::LegacyPerson>,
    pub matches: Vec<u32>,
    pub farms: Vec<legacy_types::LegacyFarm>,
    pub fertility: Vec<legacy_types::LegacyFertileRegion>,
    pub social: Vec<f32>,
    pub taverns: Vec<legacy_types::LegacyTavern>,
    pub wildlife: legacy_types::LegacyWildlife,
    pub education: legacy_types::LegacyEducation,
    pub urban: legacy_types::LegacyUrban,
    pub banking: legacy_types::LegacyBanking,
    pub relationships: legacy_types::LegacyRelationships,
}
impl From<LegacyWorld> for World {
    fn from(old: LegacyWorld) -> Self {
        Self {
            format: old.format,
            seed: old.seed,
            rng: old.rng,
            tick: old.tick,
            agents: old.agents.into(),
            genomes: old.genomes,
            innovations: old.innovations,
            brains: old.brains,
            markets: old.markets.into_iter().map(Into::into).collect(),
            patches: old.patches.into_iter().map(Into::into).collect(),
            roads: old.roads.into_iter().map(Into::into).collect(),
            ledger: old.ledger.into(),
            stats: old.stats.into(),
            sales_profile_labels: old.sales_profile_labels,
            convoys: old.convoys.into_iter().map(|o| o.map(Into::into)).collect(),
            shelters: old.shelters.into_iter().map(Into::into).collect(),
            clock_offset: old.clock_offset,
            councils: old.councils.into_iter().map(Into::into).collect(),
            corpses: old.corpses.into_iter().map(Into::into).collect(),
            contracts: old.contracts.into_iter().map(|o| o.map(Into::into)).collect(),
            graveyards: old.graveyards.into_iter().map(Into::into).collect(),
            people: old.people.into_iter().map(Into::into).collect(),
            matches: old.matches,
            farms: old.farms.into_iter().map(Into::into).collect(),
            fertility: old.fertility.into_iter().map(Into::into).collect(),
            social: old.social,
            taverns: old.taverns.into_iter().map(Into::into).collect(),
            wildlife: old.wildlife.into(),
            education: old.education.into(),
            urban: old.urban.into(),
            banking: old.banking.into(),
            relationships: old.relationships.into(),
            watch: Default::default(),
            food_memory: Default::default(),
            forage_memory: Default::default(),
            work_memory: Default::default(),
            commitments: Default::default(),
            hill: Hill::default(),
            map: crate::WorldMap::classic(old.seed),
            explored: crate::Exploration::default(),
            governance: Default::default(),
            camp_rumours: Default::default(),
            carcasses: Default::default(),
            meat_trade: Default::default(),
            market_information: Default::default(),
            road_paths: Vec::new(),
            water: WaterState::default(),
            traffic: TrafficState::default(),
            landmarks: Vec::new(),
        }
    }
}
