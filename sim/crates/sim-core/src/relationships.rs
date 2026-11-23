//! Sparse, mutual, learned relationships. Separate randomness preserves economic/family replay.
use super::*;
use std::collections::{BTreeMap, BTreeSet};
pub(super) const CONTACT_INTERVAL: u64 = rest::DAY_TICKS / 24;
const DECAY_INTERVAL: u64 = rest::DAY_TICKS * 30;
const LIMIT: usize = 50;
type Pair = (u32, u32);
#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub(super) enum Context {
    Caravan = 1,
    House = 2,
    Shop = 4,
    Tavern = 8,
    Camp = 16,
    Village = 32,
}

#[cfg(test)]
mod tests {
    use super::*;
    fn plain(n: usize) -> World {
        let mut w = World::new(n, 42).unwrap();
        for i in 0..n {
            w.people[i].spouse = NONE;
            w.people[i].father = NONE;
            w.people[i].mother = NONE;
            w.agents.traits[i] = 0;
        }
        w
    }
    fn roll(w: &mut World, range: usize, wanted: usize) {
        let seed = (0..10000u64)
            .find(|&seed| Rng(seed).index(range) == wanted)
            .unwrap();
        w.relationships.rng = Rng(seed);
    }
    #[test]
    fn family_traits_and_both_limits_have_the_requested_probabilities() {
        let mut w = plain(3);
        assert_eq!(w.relationship_chances(0, 1, 0), (50, 50));
        w.people[0].spouse = 1;
        w.people[1].spouse = 0;
        assert_eq!(w.relationship_chances(0, 1, 0), (65, 35));
        w.agents.traits[0] = life::NIGHT_OWL | life::HARDY;
        w.agents.traits[1] = life::NIGHT_OWL | life::HARDY;
        assert_eq!(w.relationship_chances(0, 1, 0), (80, 20));
        assert_eq!(w.relationship_chances(0, 1, 100), (65, 5));
        assert_eq!(w.relationship_chances(0, 1, -100), (65, 5));
        assert_eq!(w.relationship_chances(0, 2, 100), (35, 35));
        w.people[0].spouse = NONE;
        w.people[1].spouse = NONE;
        w.people[0].father = 2;
        w.people[1].father = 2;
        assert!(w.relationship_family(0, 1));
        assert!(w.relationship_family(0, 2));
    }
    #[test]
    fn scores_are_mutual_bounded_and_never_roll_more_than_once_per_hour() {
        let mut w = plain(2);
        let economic_rng = w.rng.0;
        w.relationship_contact(0, 1, Context::Shop);
        let first = w.relationship_score(0, 1);
        assert_eq!(first.abs(), 1);
        w.relationship_contact(1, 0, Context::Tavern);
        assert_eq!(w.relationship_score(0, 1), first);
        assert_eq!(w.relationships.interactions, 1);
        assert_eq!(w.relationship_score(0, 1), w.relationship_score(1, 0));
        for _ in 0..500 {
            w.tick += CONTACT_INTERVAL;
            w.relationship_contact(0, 1, Context::Caravan);
            assert!((-100..=100).contains(&w.relationship_score(0, 1)));
        }
        w.relationships
            .edges
            .entry((0, 1))
            .or_insert(Link {
                score: 100,
                next_decay: w.tick + DECAY_INTERVAL,
                last_contact: 0,
                contexts: 1,
            })
            .score = 100;
        w.sync_relationships();
        w.tick += CONTACT_INTERVAL;
        roll(&mut w, 100, 99);
        w.relationship_contact(0, 1, Context::Caravan);
        assert_eq!(w.relationship_score(0, 1), 100);
        w.tick += CONTACT_INTERVAL;
        roll(&mut w, 100, 0);
        w.relationship_contact(0, 1, Context::Caravan);
        assert_eq!(w.relationship_score(0, 1), 100);
        w.tick += CONTACT_INTERVAL;
        roll(&mut w, 100, 40);
        w.relationship_contact(0, 1, Context::Caravan);
        assert_eq!(w.relationship_score(0, 1), 99);
        w.relationships.edges.get_mut(&(0, 1)).unwrap().score = -100;
        w.tick += CONTACT_INTERVAL;
        roll(&mut w, 100, 40);
        w.relationship_contact(0, 1, Context::Caravan);
        assert_eq!(w.relationship_score(0, 1), -100);
        w.tick += CONTACT_INTERVAL;
        roll(&mut w, 100, 0);
        w.relationship_contact(0, 1, Context::Caravan);
        assert_eq!(w.relationship_score(0, 1), -99);
        assert_eq!(w.rng.0, economic_rng);
        w.check().unwrap();
    }
    #[test]
    fn cap_applies_to_both_people_and_zero_removal_frees_capacity() {
        let mut w = plain(54);
        for j in 1..=50 {
            w.relationship_contact(0, j, Context::Caravan);
        }
        assert_eq!(w.person_relationships(0).len(), 50);
        w.relationship_contact(0, 51, Context::Shop);
        assert_eq!(w.relationship_score(0, 51), 0);
        w.relationships.edges.get_mut(&(0, 1)).unwrap().score = 1;
        w.tick = CONTACT_INTERVAL;
        roll(&mut w, 100, 99);
        w.relationship_contact(0, 1, Context::House);
        assert_eq!(w.relationship_score(0, 1), 0);
        assert_eq!(w.person_relationships(0).len(), 49);
        w.relationship_contact(0, 51, Context::Shop);
        assert_eq!(w.person_relationships(0).len(), 50);
        w.relationship_contact(52, 0, Context::Shop);
        assert_eq!(w.relationship_score(52, 0), 0);
        w.check().unwrap();
    }
    #[test]
    fn monthly_drift_moves_both_signs_toward_zero_with_a_fifty_percent_roll() {
        let mut w = plain(3);
        w.relationship_contact(0, 1, Context::Shop);
        w.relationship_contact(0, 2, Context::Shop);
        w.relationships.edges.get_mut(&(0, 1)).unwrap().score = 1;
        w.relationships.edges.get_mut(&(0, 2)).unwrap().score = -1;
        w.tick = DECAY_INTERVAL - 1;
        w.relationships_step();
        assert_eq!(w.relationships.drift_checks, 0);
        let seed = (0..10000u64)
            .find(|&seed| {
                let mut r = Rng(seed);
                r.index(2) == 0 && r.index(2) == 0
            })
            .unwrap();
        w.relationships.rng = Rng(seed);
        w.tick = DECAY_INTERVAL;
        w.relationships_step();
        assert!(w.relationships.edges.is_empty());
        assert_eq!(w.relationships.drift_steps, 2);
        w.check().unwrap();
        w.tick += CONTACT_INTERVAL;
        w.relationship_contact(0, 1, Context::Shop);
        let before = w.relationship_score(0, 1);
        let due = w.relationships.edges[&(0, 1)].next_decay;
        roll(&mut w, 2, 1);
        w.tick = due;
        w.relationships_step();
        assert_eq!(w.relationship_score(0, 1), before);
        assert_eq!(
            w.relationships.edges[&(0, 1)].next_decay,
            due + DECAY_INTERVAL
        );
    }
    #[test]
    fn sleeping_together_uses_actual_house_chunks_and_not_the_whole_settlement() {
        let mut w = plain(4);
        w.markets[0].housing_blocks = 2;
        w.sync_shelters();
        for (i, slot) in [(0, 0usize), (1, 31), (2, 32), (3, 2)] {
            w.shelters[0].beds[slot] = i as u32;
            w.shelters[0].free.retain(|&s| s != slot as u32);
            w.agents.rest_site[i] = 0;
            w.agents.bed_slot[i] = slot as u32;
            w.agents.action[i] = Action::Rest;
            w.agents.target[i] = 0;
            w.agents.sleeping[i] = i != 3;
        }
        w.relationship_sleep_arrival(0, 0);
        assert_eq!(w.relationship_score(0, 1).abs(), 1);
        assert_eq!(w.relationship_score(0, 2), 0);
        assert_eq!(w.relationship_score(0, 3), 0);
        w.check().unwrap();
    }
    #[test]
    fn caravan_and_tavern_contacts_require_real_shared_presence() {
        let mut w = plain(4);
        w.tick = CONTACT_INTERVAL;
        for i in 0..2 {
            w.agents.x[i] = 100.0;
            w.agents.y[i] = 100.0;
        }
        w.agents.x[2] = 500.0;
        w.agents.y[2] = 500.0;
        w.convoys.push(Some(Convoy {
            members: vec![0, 1, 2],
            x: 100.0,
            y: 100.0,
            goal_x: 1000.0,
            goal_y: 100.0,
            created: 0,
            departed: false,
        }));
        w.relationships_step();
        assert_eq!(w.relationship_score(0, 1).abs(), 1);
        assert_eq!(w.relationship_score(0, 2), 0);
        w.convoys.clear();
        w.relationships = Relationships::default();
        w.sync_relationships();
        w.taverns.push(Tavern {
            market: 0,
            x: 100.0,
            y: 100.0,
            ready: true,
            progress: 20.0,
            escrow: 0,
            visitors: 3,
            guests: vec![0, 1, 2],
        });
        for i in 0..3 {
            w.agents.action[i] = Action::Socialize;
            w.agents.target[i] = 0;
        }
        w.agents.timer[0] = 5.0;
        w.execute_social(0, Action::Socialize);
        assert_eq!(w.relationship_score(0, 1).abs(), 1);
        assert_eq!(w.relationship_score(0, 2), 0);
    }
    #[test]
    fn actual_shop_sales_create_customer_owner_contacts_but_remote_owners_do_not() {
        let mut w = World::new(256, 42).unwrap();
        w.prepare_urban_demo().unwrap();
        w.apply_urban_policy(0, "GazetteDistricts");
        w.apply_urban_policy(0, "CommissionShops");
        let b = &mut w.urban.buildings[0];
        b.ready = true;
        b.needs_clear = false;
        b.owner = 1;
        b.wares[1] = 2;
        w.urban.produced[1] = 2;
        let (x, y) = (b.x, b.y);
        w.agents.x[1] = x;
        w.agents.y[1] = y;
        for i in [0, 2] {
            w.agents.x[i] = x;
            w.agents.y[i] = y;
            w.agents.action[i] = Action::ShopGoods;
            w.agents.target[i] = 14;
        }
        w.execute_urban(0, Action::ShopGoods);
        assert_eq!(w.relationship_score(0, 1).abs(), 1);
        w.agents.x[1] += 100.0;
        w.execute_urban(2, Action::ShopGoods);
        assert_eq!(w.relationship_score(2, 1), 0);
        w.check().unwrap();
    }
    #[test]
    fn death_removes_links_and_checkpoint_rebuilds_indices_and_randomness_exactly() {
        let mut w = plain(8);
        for j in 1..8 {
            w.relationship_contact(0, j, Context::Caravan);
        }
        w.relationship_death(1);
        assert_eq!(w.relationship_score(0, 1), 0);
        assert_eq!(w.person_relationships(0).len(), 6);
        let path =
            std::env::temp_dir().join(format!("xianxia-relations-{}.snapshot", std::process::id()));
        w.save(&path).unwrap();
        let mut loaded = World::load(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        for _ in 0..300 {
            w.step(false);
            loaded.step(true);
        }
        assert_eq!(w.person_relationships(0), loaded.person_relationships(0));
        assert_eq!(w.relationships_summary(), loaded.relationships_summary());
        assert_eq!(w.agents, loaded.agents);
        w.check().unwrap();
        loaded.check().unwrap();
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Link {
    pub score: i16,
    pub next_decay: u64,
    pub last_contact: u64,
    pub contexts: u8,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Relationships {
    pub edges: BTreeMap<Pair, Link>,
    #[serde(skip)]
    pub(super) neighbours: BTreeMap<u32, Vec<u32>>,
    #[serde(skip)]
    pub(super) due: BTreeSet<(u64, u32, u32)>,
    // Brief cooldowns for pairs which just reached zero; these are not relationships.
    pub(super) recent: BTreeMap<Pair, u64>,
    pub(super) rng: Rng,
    pub(super) initialized: bool,
    pub interactions: u64,
    pub positive: u64,
    pub negative: u64,
    pub drift_steps: u64,
    pub drift_checks: u64,
    pub capacity_skips: u64,
}
impl Default for Relationships {
    fn default() -> Self {
        Self {
            edges: BTreeMap::new(),
            neighbours: BTreeMap::new(),
            due: BTreeSet::new(),
            recent: BTreeMap::new(),
            rng: Rng(0),
            initialized: false,
            interactions: 0,
            positive: 0,
            negative: 0,
            drift_steps: 0,
            drift_checks: 0,
            capacity_skips: 0,
        }
    }
}
impl Relationships {
    fn add_neighbour(&mut self, i: u32, j: u32) {
        let list = self.neighbours.entry(i).or_default();
        if let Err(at) = list.binary_search(&j) {
            list.insert(at, j);
        }
    }
    fn remove(&mut self, pair: Pair) {
        if let Some(link) = self.edges.remove(&pair) {
            self.due.remove(&(link.next_decay, pair.0, pair.1));
            for (i, j) in [pair, (pair.1, pair.0)] {
                if let Some(list) = self.neighbours.get_mut(&i) {
                    if let Ok(at) = list.binary_search(&j) {
                        list.remove(at);
                    }
                    if list.is_empty() {
                        self.neighbours.remove(&i);
                    }
                }
            }
        }
    }
}
impl World {
    pub(super) fn sync_relationships(&mut self) {
        let r = &mut self.relationships;
        if !r.initialized {
            r.rng = Rng(self.seed ^ 0x52454c4154494f4e);
            r.initialized = true;
        }
        r.neighbours.clear();
        r.due.clear();
        let rows = r
            .edges
            .iter()
            .map(|(&p, l)| (p, l.next_decay))
            .collect::<Vec<_>>();
        for (p, t) in rows {
            r.add_neighbour(p.0, p.1);
            r.add_neighbour(p.1, p.0);
            r.due.insert((t, p.0, p.1));
        }
    }
    pub(super) fn relationship_score(&self, i: usize, j: usize) -> i16 {
        let pair = if i < j {
            (i as u32, j as u32)
        } else {
            (j as u32, i as u32)
        };
        self.relationships
            .edges
            .get(&pair)
            .map(|l| l.score)
            .unwrap_or(0)
    }
    fn relationship_family(&self, i: usize, j: usize) -> bool {
        let (a, b) = (&self.people[i], &self.people[j]);
        a.spouse == j as u32
            || b.spouse == i as u32
            || a.father == j as u32
            || a.mother == j as u32
            || b.father == i as u32
            || b.mother == i as u32
            || a.father != NONE && a.father == b.father
            || a.mother != NONE && a.mother == b.mother
    }
    fn relationship_chances(&self, i: usize, j: usize, score: i16) -> (usize, usize) {
        let positive =
            50 + if self.relationship_family(i, j) {
                15
            } else {
                0
            } + if self.agents.traits[i] & self.agents.traits[j] != 0 {
                15
            } else {
                0
            };
        let penalty = if score.abs() == 100 { 15 } else { 0 };
        (positive - penalty, 100 - positive - penalty)
    }
    pub(super) fn relationship_contact(&mut self, i: usize, j: usize, context: Context) {
        if i == j
            || i >= self.agents.len()
            || j >= self.agents.len()
            || !self.agents.alive[i]
            || !self.agents.alive[j]
        {
            return;
        }
        let pair = if i < j {
            (i as u32, j as u32)
        } else {
            (j as u32, i as u32)
        };
        if self
            .relationships
            .recent
            .get(&pair)
            .is_some_and(|&until| until > self.tick)
        {
            return;
        }
        if let Some(l) = self.relationships.edges.get_mut(&pair) {
            // Preserve evidence of context even when the same-hour cooldown suppresses a roll.
            l.contexts |= context as u8;
            if self.tick.saturating_sub(l.last_contact) < CONTACT_INTERVAL {
                return;
            }
        } else if [pair.0, pair.1].iter().any(|id| {
            self.relationships
                .neighbours
                .get(id)
                .map(Vec::len)
                .unwrap_or(0)
                >= LIMIT
        }) {
            self.relationships.capacity_skips += 1;
            return;
        }
        let old = self
            .relationships
            .edges
            .get(&pair)
            .map(|l| l.score)
            .unwrap_or(0);
        let (positive, negative) = self.relationship_chances(i, j, old);
        let roll = self.relationships.rng.index(100);
        let delta = if roll < positive {
            1
        } else if roll < positive + negative {
            -1
        } else {
            0
        };
        let next = (old + delta).clamp(-100, 100);
        let r = &mut self.relationships;
        r.interactions += 1;
        if next > old {
            r.positive += 1;
        } else if next < old {
            r.negative += 1;
        }
        if next == 0 {
            r.remove(pair);
            r.recent.insert(pair, self.tick + CONTACT_INTERVAL);
            return;
        }
        if let Some(l) = r.edges.get_mut(&pair) {
            l.score = next;
            l.last_contact = self.tick;
            l.contexts |= context as u8;
        } else {
            let due = self.tick + DECAY_INTERVAL;
            r.edges.insert(
                pair,
                Link {
                    score: next,
                    next_decay: due,
                    last_contact: self.tick,
                    contexts: context as u8,
                },
            );
            r.add_neighbour(pair.0, pair.1);
            r.add_neighbour(pair.1, pair.0);
            r.due.insert((due, pair.0, pair.1));
        }
    }
    pub(super) fn relationship_group(&mut self, people: &[u32], context: Context) {
        let mut ids = people.to_vec();
        ids.sort_unstable();
        ids.dedup();
        for a in 0..ids.len() {
            for b in a + 1..ids.len() {
                self.relationship_contact(ids[a] as usize, ids[b] as usize, context);
            }
        }
    }
    pub(super) fn relationship_sleep_arrival(&mut self, i: usize, site: usize) {
        let s = &self.shelters[site];
        let chunk = self.agents.bed_slot[i] as usize / 32;
        let peers = s
            .beds
            .chunks(32)
            .nth(chunk)
            .unwrap_or(&[])
            .iter()
            .copied()
            .filter(|&j| {
                j != NONE
                    && j != i as u32
                    && self.agents.alive[j as usize]
                    && self.agents.sleeping[j as usize]
                    && self.agents.rest_site[j as usize] == site as u32
            })
            .collect::<Vec<_>>();
        let context = if s.settlement.is_some() {
            Context::House
        } else {
            Context::Camp
        };
        for j in peers {
            self.relationship_contact(i, j as usize, context);
        }
    }
    pub(super) fn relationship_death(&mut self, i: usize) {
        let peers = self
            .relationships
            .neighbours
            .get(&(i as u32))
            .cloned()
            .unwrap_or_default();
        for j in peers {
            let pair = if (i as u32) < j {
                (i as u32, j)
            } else {
                (j, i as u32)
            };
            self.relationships.remove(pair);
        }
    }
    pub(super) fn relationships_step(&mut self) {
        // An ordered due index touches only relationships whose monthly review is due.
        while self
            .relationships
            .due
            .first()
            .is_some_and(|&(due, _, _)| due <= self.tick)
        {
            let (due, i, j) = self.relationships.due.pop_first().unwrap();
            if !self.agents.alive[i as usize] || !self.agents.alive[j as usize] {
                self.relationships.remove((i, j));
                continue;
            }
            self.relationships.drift_checks += 1;
            let change = self.relationships.rng.index(2) == 0;
            let l = self.relationships.edges.get_mut(&(i, j)).unwrap();
            if change {
                l.score -= l.score.signum();
                self.relationships.drift_steps += 1;
            }
            if l.score == 0 {
                let until = l.last_contact + CONTACT_INTERVAL;
                self.relationships.remove((i, j));
                if until > self.tick {
                    self.relationships.recent.insert((i, j), until);
                }
            } else {
                l.next_decay = due + DECAY_INTERVAL;
                self.relationships.due.insert((l.next_decay, i, j));
            }
        }
        let phase = self.tick % CONTACT_INTERVAL;
        if phase == 0 {
            self.relationships
                .recent
                .retain(|_, until| *until > self.tick);
        }
        let mut groups = vec![];
        // Stagger groups across the hour instead of processing every caravan on one frame.
        for (id, c) in self.convoys.iter().enumerate() {
            if id as u64 % CONTACT_INTERVAL != phase {
                continue;
            }
            if let Some(c) = c {
                let ids = c
                    .members
                    .iter()
                    .copied()
                    .filter(|&i| {
                        self.agents.alive[i as usize]
                            && dist(
                                self.agents.x[i as usize],
                                self.agents.y[i as usize],
                                c.x,
                                c.y,
                            ) <= 35.0
                    })
                    .collect::<Vec<_>>();
                if ids.len() > 1 {
                    groups.push((Context::Caravan, ids));
                }
            }
        }
        for (site, s) in self.shelters.iter().enumerate() {
            for (house, beds) in s.beds.chunks(32).enumerate() {
                if (site as u64 * 53 + house as u64) % CONTACT_INTERVAL != phase {
                    continue;
                }
                let ids = beds
                    .iter()
                    .copied()
                    .filter(|&i| {
                        i != NONE
                            && self.agents.alive[i as usize]
                            && self.agents.sleeping[i as usize]
                            && self.agents.rest_site[i as usize] == site as u32
                    })
                    .collect::<Vec<_>>();
                if ids.len() > 1 {
                    groups.push((
                        if s.settlement.is_some() {
                            Context::House
                        } else {
                            Context::Camp
                        },
                        ids,
                    ));
                }
            }
        }
        for (id, t) in self.taverns.iter().enumerate() {
            if !t.ready || id as u64 % CONTACT_INTERVAL != phase {
                continue;
            }
            let ids = t
                .guests
                .iter()
                .copied()
                .filter(|&i| {
                    self.agents.alive[i as usize]
                        && self.agents.action[i as usize] == Action::Socialize
                        && self.agents.target[i as usize] == id as u32
                        && dist(
                            self.agents.x[i as usize],
                            self.agents.y[i as usize],
                            t.x,
                            t.y,
                        ) <= 6.0
                })
                .collect::<Vec<_>>();
            if ids.len() > 1 {
                groups.push((Context::Tavern, ids));
            }
        }
        for (context, ids) in groups {
            self.relationship_group(&ids, context);
        }
    }
    pub(super) fn relationship_affinity(&self, i: usize, action: Action, target: u32) -> f32 {
        match action {
            Action::Court if (target as usize) < self.agents.len() => {
                self.relationship_score(i, target as usize) as f32 / 100.0
            }
            Action::ShopGoods => self
                .urban
                .buildings
                .get(target as usize / 32)
                .filter(|b| b.owner != NONE)
                .map(|b| self.relationship_score(i, b.owner as usize) as f32 / 100.0)
                .unwrap_or(0.0),
            Action::Socialize => self
                .taverns
                .get(target as usize)
                .map(|t| {
                    let peers = t
                        .guests
                        .iter()
                        .copied()
                        .filter(|&j| j != i as u32)
                        .collect::<Vec<_>>();
                    if peers.is_empty() {
                        0.0
                    } else {
                        peers
                            .iter()
                            .map(|&j| self.relationship_score(i, j as usize) as f32)
                            .sum::<f32>()
                            / (100.0 * peers.len() as f32)
                    }
                })
                .unwrap_or(0.0),
            _ => 0.0,
        }
    }
    pub(super) fn person_relationships(&self, i: usize) -> Vec<serde_json::Value> {
        let mut peers = self
            .relationships
            .neighbours
            .get(&(i as u32))
            .cloned()
            .unwrap_or_default();
        peers.sort_unstable_by_key(|&j| {
            (
                std::cmp::Reverse(self.relationship_score(i, j as usize).abs()),
                j,
            )
        });
        peers.into_iter().map(|j| {
            let pair=if (i as u32)<j {(i as u32,j)}else{(j,i as u32)};let l=&self.relationships.edges[&pair];
            let (positive,negative)=self.relationship_chances(i,j as usize,l.score);
            serde_json::json!({"id":j,"name":self.people[j as usize].name(),"score":l.score,"family":self.relationship_family(i,j as usize),"shared_trait":self.agents.traits[i]&self.agents.traits[j as usize]!=0,"positive_chance_percent":positive,"negative_chance_percent":negative,"days_to_drift":l.next_decay.saturating_sub(self.tick) as f64/rest::DAY_TICKS as f64,"contexts":([(1,"Caravan"),(2,"House"),(4,"Shop"),(8,"Tavern"),(16,"Camp"),(32,"Village")].iter().filter(|(bit,_)|l.contexts&bit!=0).map(|(_,name)|*name).collect::<Vec<_>>())})
        }).collect()
    }
    pub(super) fn relationships_summary(&self) -> serde_json::Value {
        serde_json::json!({"pairs":self.relationships.edges.len(),"max_per_person":LIMIT,"minimum":-100,"maximum":100,"contact_interval_hours":1,"drift_interval_days":30,"drift_chance_percent":50,"family_bonus_percentage_points":15,"shared_trait_bonus_percentage_points":15,"limit_penalty_percentage_points":15,"interactions":self.relationships.interactions,"positive_changes":self.relationships.positive,"negative_changes":self.relationships.negative,"drift_steps":self.relationships.drift_steps,"capacity_skips":self.relationships.capacity_skips})
    }
    pub(super) fn validate_relationships(&self) -> Result<(), String> {
        if self.relationships.edges.len() != self.relationships.due.len() {
            return Err("Invalid relationship schedule".into());
        }
        for (&(i, j), l) in &self.relationships.edges {
            if i >= j
                || j as usize >= self.agents.len()
                || l.score == 0
                || !(-100..=100).contains(&l.score)
                || l.last_contact > self.tick
                || !self.relationships.due.contains(&(l.next_decay, i, j))
            {
                return Err("Invalid learned relationship".into());
            }
            for (a, b) in [(i, j), (j, i)] {
                if !self
                    .relationships
                    .neighbours
                    .get(&a)
                    .is_some_and(|peers| peers.len() <= LIMIT && peers.binary_search(&b).is_ok())
                {
                    return Err("Invalid relationship index or cap".into());
                }
            }
        }
        for (&i, peers) in &self.relationships.neighbours {
            if i as usize >= self.agents.len()
                || peers.len() > LIMIT
                || peers.windows(2).any(|p| p[0] >= p[1])
            {
                return Err("Invalid relationship neighbours".into());
            }
            for &j in peers {
                let pair = if i < j { (i, j) } else { (j, i) };
                if !self.relationships.edges.contains_key(&pair) {
                    return Err("Orphan relationship index".into());
                }
            }
        }
        Ok(())
    }
}
