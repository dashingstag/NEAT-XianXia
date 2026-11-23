//! Exact schema-6 reader.
//! Frozen layouts this schema decodes into: LegacyAgents, LegacyAspiration, LegacyContract,
//!   LegacyConvoy, LegacyCorpse, LegacyCouncil, LegacyFarm, LegacyFertileRegion, LegacyGoal,
//!   LegacyGoalKind, LegacyGraveyard, LegacyJobKind, LegacyLedger, LegacyLot, LegacyMarket,
//!   LegacyPatch, LegacyPerson, LegacyPolicyChoice, LegacyPregnancy, LegacyProject,
//!   LegacyQuote, LegacyRoad, LegacyShelter, LegacyStatistics.
//! Live types it still trusts: Action, Gene, Genome, Innovations, Kind, Node, Rng.
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
    pub agents: legacy_v10::LegacyAgents,
    pub genomes: Vec<Genome>,
    pub innovations: Innovations,
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
}
impl From<LegacyWorld> for World {
    fn from(old: LegacyWorld) -> Self {
        let n = old.agents.x.len();
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
            social: vec![0.0; n],
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
    fn schema_six_migrates_without_moving_people_or_changing_assets() {
        let w = World::new(12, 42).unwrap();
        let old = LegacyWorld {
            format: w.format.clone(),
            seed: w.seed.clone(),
            rng: w.rng.clone(),
            tick: w.tick.clone(),
            agents: w.agents.clone().into(),
            genomes: w.genomes.clone(),
            innovations: w.innovations.clone(),
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
        };
        let raw = postcard::to_allocvec(&old).unwrap();
        let mut bytes = b"XXSAVE06".to_vec();
        bytes.extend((raw.len() as u64).to_le_bytes());
        bytes.extend(zstd::stream::encode_all(raw.as_slice(), 1).unwrap());
        let path = std::env::temp_dir().join(format!(
            "xianxia-social-migration-{}.snapshot",
            std::process::id()
        ));
        std::fs::write(&path, bytes).unwrap();
        let loaded = World::load(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(loaded.agents.x, w.agents.x);
        assert_eq!(loaded.people, w.people);
        let mut expected = w.stocks();
        for p in w
            .patches
            .iter()
            .filter(|p| (4..=7).contains(&p.kind) && p.stock > 0)
        {
            expected[p.kind] += 50_000;
        }
        assert_eq!(loaded.stocks(), expected);
        assert_eq!(loaded.coins(), w.coins());
        assert_eq!(loaded.social, vec![0.0; 12]);
        assert_eq!(loaded.format, 15);
    }
}
