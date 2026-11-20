//! Explicit schema-10 reader, also reuses the old agent pool for earlier schemas.
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
    pub markets: Vec<legacy_v13::LegacyMarket>,
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
        }
    }
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
            ledger: old.ledger,
            stats: old.stats,
            sales_profile_labels: old.sales_profile_labels,
            convoys: old.convoys,
            shelters: old.shelters,
            clock_offset: old.clock_offset,
            councils: old.councils,
            corpses: old.corpses,
            contracts: old.contracts,
            graveyards: old.graveyards,
            people: old.people,
            matches: old.matches,
            farms: old.farms,
            fertility: old.fertility,
            social: old.social,
            taverns: old.taverns,
            wildlife: old.wildlife,
            education: old.education,
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
            road_paths: Vec::new(),
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
            rng: w.rng.clone(),
            tick: w.tick.clone(),
            agents: w.agents.clone().into(),
            genomes: w.genomes.clone(),
            innovations: w.innovations.clone(),
            brains: w.brains.clone(),
            markets: w.markets.iter().map(legacy_v13::to_legacy_market).collect(),
            patches: w.patches.clone(),
            roads: w.roads.clone(),
            ledger: w.ledger.clone(),
            stats: w.stats.clone(),
            sales_profile_labels: w.sales_profile_labels.clone(),
            convoys: w.convoys.clone(),
            shelters: w.shelters.clone(),
            clock_offset: w.clock_offset.clone(),
            councils: w.councils.clone(),
            corpses: w.corpses.clone(),
            contracts: w.contracts.clone(),
            graveyards: w.graveyards.clone(),
            people: w.people.clone(),
            matches: w.matches.clone(),
            farms: w.farms.clone(),
            fertility: w.fertility.clone(),
            social: w.social.clone(),
            taverns: w.taverns.clone(),
            wildlife: w.wildlife.clone(),
            education: w.education.clone(),
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
