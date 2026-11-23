//! Explicit schema-10 reader, also reuses the old agent pool for earlier schemas.
//! Frozen layouts this schema decodes into: LegacyAction, LegacyAgents, LegacyAnimal,
//!   LegacyAnimalPregnancy, LegacyAspiration, LegacyContract, LegacyConvoy, LegacyCorpse,
//!   LegacyCouncil, LegacyEcologyLedger, LegacyEducation, LegacyFarm, LegacyFertileRegion,
//!   LegacyForest, LegacyGene, LegacyGenome, LegacyGoal, LegacyGoalKind, LegacyGrass,
//!   LegacyGraveyard, LegacyInnovations, LegacyJobKind, LegacyKind, LegacyLedger, LegacyLot,
//!   LegacyMarket, LegacyNode, LegacyPatch, LegacyPerson, LegacyPolicyChoice, LegacyPregnancy,
//!   LegacyProject, LegacyQuote, LegacyRng, LegacyRoad, LegacySchool, LegacyShelter,
//!   LegacyStatistics, LegacyTavern, LegacyWildlife.
//! It decodes into no live types, so live types can change freely;
//! legacy_schema::legacy_reader_schemas_are_pinned fails if that stops
//! being true. Never edit a Legacy* layout: old saves depend on it (#54).
use super::*;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
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
    pub action: Vec<legacy_types::LegacyAction>,
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
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LegacyWorld {
    pub format: u32,
    pub seed: u64,
    pub rng: legacy_types::LegacyRng,
    pub tick: u64,
    pub agents: LegacyAgents,
    pub genomes: Vec<legacy_types::LegacyGenome>,
    pub innovations: legacy_types::LegacyInnovations,
    #[serde(skip)]
    pub brains: Vec<Compiled>,
    pub markets: Vec<legacy_v13::LegacyMarket>,
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
            action: old.action.into_iter().map(Into::into).collect(),
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
            urban_weight: vec![0; n],
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
            action: a.action.into_iter().map(Into::into).collect(),
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
        }
    }
}
impl From<LegacyWorld> for World {
    fn from(old: LegacyWorld) -> Self {
        Self {
            format: old.format,
            seed: old.seed,
            rng: old.rng.into(),
            tick: old.tick,
            agents: old.agents.into(),
            genomes: old.genomes.into_iter().map(Into::into).collect(),
            innovations: old.innovations.into(),
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
            urban: Urban::default(),
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
    fn schema_ten_preserves_people_money_resources_and_schools() {
        let w = World::new(12, 42).unwrap();
        let old = LegacyWorld {
            format: w.format.clone(),
            seed: w.seed.clone(),
            rng: w.rng.clone().into(),
            tick: w.tick.clone(),
            agents: w.agents.clone().into(),
            genomes: w.genomes.iter().cloned().map(Into::into).collect(),
            innovations: w.innovations.clone().into(),
            brains: w.brains.clone(),
            markets: w.markets.iter().map(legacy_v13::to_legacy_market).collect(),
            patches: w.patches.iter().cloned().map(Into::into).collect(),
            roads: w.roads.iter().cloned().map(Into::into).collect(),
            ledger: w.ledger.clone().into(),
            stats: w.stats.clone().into(),
            sales_profile_labels: w.sales_profile_labels.clone(),
            convoys: w.convoys.iter().cloned().map(|o| o.map(Into::into)).collect(),
            shelters: w.shelters.iter().cloned().map(Into::into).collect(),
            clock_offset: w.clock_offset.clone(),
            councils: w.councils.iter().cloned().map(Into::into).collect(),
            corpses: w.corpses.iter().cloned().map(Into::into).collect(),
            contracts: w.contracts.iter().cloned().map(|o| o.map(Into::into)).collect(),
            graveyards: w.graveyards.iter().cloned().map(Into::into).collect(),
            people: w.people.iter().cloned().map(Into::into).collect(),
            matches: w.matches.clone(),
            farms: w.farms.iter().cloned().map(Into::into).collect(),
            fertility: w.fertility.iter().cloned().map(Into::into).collect(),
            social: w.social.clone(),
            taverns: w.taverns.iter().cloned().map(Into::into).collect(),
            wildlife: w.wildlife.clone().into(),
            education: w.education.clone().into(),
        };
        let raw = postcard::to_allocvec(&old).unwrap();
        let mut bytes = b"XXSAVE10".to_vec();
        bytes.extend((raw.len() as u64).to_le_bytes());
        bytes.extend(zstd::stream::encode_all(raw.as_slice(), 1).unwrap());
        let path =
            std::env::temp_dir().join(format!("xianxia-urban-v10-{}.snapshot", std::process::id()));
        std::fs::write(&path, bytes).unwrap();
        let loaded = World::load(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(loaded.people, w.people);
        assert_eq!(loaded.stocks(), w.stocks());
        assert_eq!(loaded.coins(), w.coins());
        assert_eq!(loaded.wildlife.ledger.births, w.wildlife.ledger.births);
        assert_eq!(loaded.format, 15);
        loaded.check().unwrap();
    }
}
