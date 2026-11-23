//! Explicit schema-11 reader: adds empty town banking without changing existing assets.
//! Frozen layouts this schema decodes into: LegacyAgents, LegacyAspiration, LegacyCouncil,
//!   LegacyGoal, LegacyGoalKind, LegacyLedger, LegacyLot, LegacyMarket, LegacyPerson,
//!   LegacyPolicyChoice, LegacyPregnancy, LegacyProject, LegacyQuote, LegacyStatistics.
//! Live types it still trusts: Action, Animal, AnimalPregnancy, Building, Contract, Convoy,
//!   Corpse, EcologyLedger, Education, Estate, Farm, FertileRegion, Forest, Gene, Genome,
//!   Grass, Graveyard, Innovations, Job, JobKind, Kind, Node, Patch, Progress, Rng, Road,
//!   School, Shelter, Tavern, TownState, Urban, Wildlife, Zone.
//! A serde-layout change to a live type reinterprets this version's saved
//! bytes; legacy_schema::legacy_reader_schemas_are_pinned fails first —
//! freeze a Legacy twin in legacy_types.rs before changing the type (#54).
use super::*;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LegacyWorld {
    pub format: u32,
    pub seed: u64,
    pub rng: Rng,
    pub tick: u64,
    pub agents: legacy_v13::LegacyAgents,
    pub genomes: Vec<Genome>,
    pub innovations: Innovations,
    #[serde(skip)]
    pub brains: Vec<Compiled>,
    pub markets: Vec<legacy_v13::LegacyMarket>,
    pub patches: Vec<Patch>,
    pub roads: Vec<Road>,
    pub ledger: legacy_types::LegacyLedger,
    pub stats: legacy_types::LegacyStatistics,
    pub sales_profile_labels: Vec<String>,
    pub convoys: Vec<Option<Convoy>>,
    pub shelters: Vec<Shelter>,
    pub clock_offset: u64,
    pub councils: Vec<legacy_types::LegacyCouncil>,
    pub corpses: Vec<Corpse>,
    pub contracts: Vec<Option<Contract>>,
    pub graveyards: Vec<Graveyard>,
    pub people: Vec<legacy_types::LegacyPerson>,
    pub matches: Vec<u32>,
    pub farms: Vec<Farm>,
    pub fertility: Vec<FertileRegion>,
    pub social: Vec<f32>,
    pub taverns: Vec<Tavern>,
    pub wildlife: Wildlife,
    pub education: Education,
    pub urban: Urban,
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
            people: old.people.into_iter().map(Into::into).collect(),
            matches: old.matches,
            farms: old.farms,
            fertility: old.fertility,
            social: old.social,
            taverns: old.taverns,
            wildlife: old.wildlife,
            education: old.education,
            urban: old.urban,
            banking: Banking::default(),
            relationships: Relationships::default(),
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
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn schema_eleven_preserves_assets_and_adds_no_bank_cash() {
        let w = World::new(64, 42).unwrap();
        let old = LegacyWorld {
            format: w.format.clone(),
            seed: w.seed.clone(),
            rng: w.rng.clone(),
            tick: w.tick.clone(),
            agents: legacy_v13::to_legacy_agents(&w.agents),
            genomes: w.genomes.clone(),
            innovations: w.innovations.clone(),
            brains: w.brains.clone(),
            markets: w.markets.iter().map(legacy_v13::to_legacy_market).collect(),
            patches: w.patches.clone(),
            roads: w.roads.clone(),
            ledger: w.ledger.clone().into(),
            stats: w.stats.clone().into(),
            sales_profile_labels: w.sales_profile_labels.clone(),
            convoys: w.convoys.clone(),
            shelters: w.shelters.clone(),
            clock_offset: w.clock_offset.clone(),
            councils: w.councils.iter().cloned().map(Into::into).collect(),
            corpses: w.corpses.clone(),
            contracts: w.contracts.clone(),
            graveyards: w.graveyards.clone(),
            people: w.people.iter().cloned().map(Into::into).collect(),
            matches: w.matches.clone(),
            farms: w.farms.clone(),
            fertility: w.fertility.clone(),
            social: w.social.clone(),
            taverns: w.taverns.clone(),
            wildlife: w.wildlife.clone(),
            education: w.education.clone(),
            urban: w.urban.clone(),
        };
        let raw = postcard::to_allocvec(&old).unwrap();
        let mut bytes = b"XXSAVE11".to_vec();
        bytes.extend((raw.len() as u64).to_le_bytes());
        bytes.extend(zstd::stream::encode_all(raw.as_slice(), 1).unwrap());
        let p =
            std::env::temp_dir().join(format!("xianxia-bank-v11-{}.snapshot", std::process::id()));
        std::fs::write(&p, bytes).unwrap();
        let loaded = World::load(&p).unwrap();
        std::fs::remove_file(&p).unwrap();
        assert_eq!(loaded.people, w.people);
        assert_eq!(loaded.stocks(), w.stocks());
        assert_eq!(loaded.coins(), w.coins());
        assert!(loaded.banking.banks.is_empty());
        assert!(loaded.banking.loans.is_empty());
        loaded.check().unwrap();
    }
}
