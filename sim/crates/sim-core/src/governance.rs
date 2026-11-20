use super::*;
use std::collections::BTreeMap;

const YEAR: u64 = 365 * rest::DAY_TICKS;
const TERM: u64 = 5 * YEAR;
const NOMINATION_WINDOW: u64 = 30 * rest::DAY_TICKS;
const LEADERSHIP_BOOST: f32 = 8.0;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub(super) struct Governance {
    offices: BTreeMap<u32, Office>,
    happiness: BTreeMap<u32, Award>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
struct Office {
    leader: Option<u32>,
    next_election: Option<u64>,
    round: u64,
    nomination: Option<Nomination>,
    result: Option<Election>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Nomination {
    fee: u64,
    candidates: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Election {
    tick: u64,
    winner: Option<u32>,
    votes: BTreeMap<u32, u32>,
    turnout: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Award {
    market: u32,
    round: u64,
    expires: u64,
}

impl World {
    fn resident(&self, person: usize, market: usize) -> bool {
        self.agents.alive.get(person) == Some(&true) && self.agents.home[person] as usize == market
    }

    fn election_eligible(&self, person: usize, market: usize) -> bool {
        self.resident(person, market) && self.people[person].age_days(self.tick) >= 11 * 365
    }

    pub(super) fn governance_step(&mut self) {
        for market in 0..self.markets.len() {
            let id = market as u32;
            if self.markets[market].village {
                self.governance
                    .offices
                    .entry(id)
                    .or_default()
                    .next_election
                    .get_or_insert(self.tick + TERM);
            }
            let Some(office) = self.governance.offices.get(&id) else {
                continue;
            };
            let departed = office
                .leader
                .is_some_and(|p| !self.resident(p as usize, market));
            let deadline = office.next_election;
            let needs_window = office.nomination.is_none();
            if departed {
                self.governance.offices.get_mut(&id).unwrap().leader = None;
            }
            let Some(deadline) = deadline else { continue };
            if self.tick >= deadline {
                self.elect(market, deadline);
            } else if needs_window && self.tick >= deadline.saturating_sub(NOMINATION_WINDOW) {
                let fee = ((self.richest_resident_net_worth(market).max(0) + 9) / 10) as u64;
                self.governance.offices.get_mut(&id).unwrap().nomination = Some(Nomination {
                    fee,
                    candidates: Vec::new(),
                });
            }
        }
        if self.tick % rest::DAY_TICKS == 0 {
            self.governance
                .happiness
                .retain(|&i, award| award.expires > self.tick && self.agents.alive[i as usize]);
        }
    }

    pub(super) fn select_camp_leader(&mut self, market: u32, founders: &[usize]) {
        let leader = founders.iter().copied().max_by_key(|&i| {
            let trust = founders
                .iter()
                .filter(|&&j| i != j)
                .map(|&j| i32::from(self.relationship_score(i, j)))
                .sum::<i32>();
            (trust, std::cmp::Reverse(i))
        });
        if let Some(i) = leader {
            self.governance.offices.entry(market).or_default().leader = Some(i as u32);
        }
    }

    fn nomination_fee(&self, i: usize, market: usize) -> Option<u64> {
        let office = self.governance.offices.get(&(market as u32))?;
        let nomination = office.nomination.as_ref()?;
        if self.election_eligible(i, market)
            && office
                .next_election
                .is_some_and(|deadline| self.tick < deadline)
            && self.agents.coins[i] >= nomination.fee
            && !nomination.candidates.contains(&(i as u32))
        {
            Some(nomination.fee)
        } else {
            None
        }
    }

    pub(super) fn nomination_offer(&self, i: usize) -> Option<Candidate> {
        let target = self.agents.home[i];
        let fee = self.nomination_fee(i, target as usize)?;
        let m = &self.markets[target as usize];
        let mut inputs = [0.0; INPUTS];
        let aspiration = self.people[i].aspiration;
        inputs[0] = if aspiration == life::Aspiration::Service {
            1.05
        } else {
            0.55
        };
        inputs[1] = 0.8;
        inputs[3] = (dist(self.agents.x[i], self.agents.y[i], m.x, m.y) / 1000.0).min(1.0);
        inputs[4] = -(fee as f32 / self.agents.coins[i].max(1) as f32);
        inputs[5] = if self.agents.action[i] == Action::NominateSelf {
            0.25
        } else {
            0.0
        };
        Some(Candidate {
            action: Action::NominateSelf,
            target,
            kind: 0,
            score: self.score_action(i, Action::NominateSelf, target, &inputs),
            inputs,
        })
    }

    pub(super) fn execute_nomination(&mut self, i: usize) {
        let market = self.agents.target[i] as usize;
        let Some(fee) = self.nomination_fee(i, market) else {
            self.agents.action[i] = Action::Wait;
            self.agents.target[i] = NONE;
            return;
        };
        let m = &self.markets[market];
        let (x, y) = (self.agents.x[i], self.agents.y[i]);
        let d = dist(x, y, m.x, m.y);
        if d > 7.0 {
            let step = (self.actor_speed(i, x, y) * DT).min(d - 6.0);
            self.agents.x[i] += (m.x - x) / d * step;
            self.agents.y[i] += (m.y - y) / d * step;
            self.agents.drain_energy(i, DT * 0.07);
            return;
        }
        self.agents.coins[i] -= fee;
        let m = &mut self.markets[market];
        m.coins += fee;
        m.revenue_total += fee;
        self.governance
            .offices
            .get_mut(&(market as u32))
            .unwrap()
            .nomination
            .as_mut()
            .unwrap()
            .candidates
            .push(i as u32);
        self.agents.action[i] = Action::Wait;
        self.agents.target[i] = NONE;
    }

    fn elect(&mut self, market: usize, deadline: u64) {
        let id = market as u32;
        let office = self.governance.offices.get_mut(&id).unwrap();
        let nominees = office
            .nomination
            .take()
            .map_or_else(Vec::new, |n| n.candidates);
        office.next_election = Some(deadline + TERM);
        office.round += 1;
        let round = office.round;
        let mut rng = Rng(self.seed ^ deadline.rotate_left(19) ^ u64::from(id));
        let mut votes: BTreeMap<u32, u32> = nominees
            .into_iter()
            .filter(|&i| self.election_eligible(i as usize, market))
            .map(|i| (i, 0))
            .collect();
        let mut turnout = 0;
        if !votes.is_empty() {
            for voter in 0..self.agents.len() {
                if !self.election_eligible(voter, market) {
                    continue;
                }
                let choice = votes
                    .keys()
                    .copied()
                    .max_by_key(|&candidate| {
                        let relationship =
                            i32::from(self.relationship_score(voter, candidate as usize));
                        let match_aspiration = self.people[voter].aspiration
                            == self.people[candidate as usize].aspiration;
                        (
                            relationship + if match_aspiration { 200 } else { 0 },
                            rng.next_u64(),
                        )
                    })
                    .unwrap();
                *votes.get_mut(&choice).unwrap() += 1;
                turnout += 1;
            }
        }
        let winner = votes
            .iter()
            .max_by_key(|(_, &count)| (count, rng.next_u64()))
            .map(|(&i, _)| i);
        let office = self.governance.offices.get_mut(&id).unwrap();
        office.leader = winner;
        office.result = Some(Election {
            tick: self.tick,
            winner,
            votes,
            turnout,
        });
        if let Some(winner) = winner {
            let aspiration = self.people[winner as usize].aspiration;
            for i in 0..self.agents.len() {
                if self.resident(i, market) && self.people[i].aspiration == aspiration {
                    self.governance.happiness.insert(
                        i as u32,
                        Award {
                            market: id,
                            round,
                            expires: self.tick + YEAR,
                        },
                    );
                }
            }
        }
    }

    pub(super) fn leadership_award(&self, i: usize) -> Option<(f32, u64)> {
        self.governance
            .happiness
            .get(&(i as u32))
            .filter(|a| self.tick < a.expires)
            .map(|a| (LEADERSHIP_BOOST, a.expires))
    }

    pub(super) fn effective_happiness(&self, i: usize) -> f32 {
        (self.people[i].happiness + self.leadership_award(i).map_or(0.0, |a| a.0)).min(100.0)
    }

    pub(super) fn leader_job_alignment(&self, m: usize, action: &str) -> f32 {
        let leader = self
            .governance
            .offices
            .get(&(m as u32))
            .and_then(|o| o.leader)
            .filter(|&i| self.resident(i as usize, m));
        match (leader.map(|i| self.people[i as usize].aspiration), action) {
            (Some(life::Aspiration::Farmer), "PostFarmer")
            | (Some(life::Aspiration::Teacher), "PostTeacher")
            | (Some(life::Aspiration::Service), "PostGravedigger") => 1.0,
            _ => 0.0,
        }
    }

    pub(super) fn settlement_stage(&self, m: usize) -> &str {
        let town = self.is_town(m);
        if town
            && self.markets[m].stable_residents >= 500
            && self
                .urban
                .buildings
                .iter()
                .filter(|b| b.market as usize == m && b.ready)
                .count()
                >= 50
        {
            "City"
        } else if town {
            "Town"
        } else if self.markets[m].village {
            "Village"
        } else {
            "Camp"
        }
    }

    pub(super) fn governance_summary(&self) -> serde_json::Value {
        serde_json::json!(self
            .markets
            .iter()
            .enumerate()
            .map(|(id, _)| {
                let stage = self.settlement_stage(id);
                let title = match stage {
                    "City" => "City Mayor",
                    "Town" => "Town Mayor",
                    "Village" => "Village Head",
                    _ => "Camp Leader",
                };
                let office = self.governance.offices.get(&(id as u32));
                let leader = office
                    .and_then(|o| o.leader)
                    .filter(|&i| self.resident(i as usize, id));
                let nominees: Vec<_> = office.and_then(|o| o.nomination.as_ref()).into_iter()
                    .flat_map(|n| n.candidates.iter()).take(16)
                    .map(|&i| serde_json::json!({"id":i, "name":self.people[i as usize].name()})).collect();
                serde_json::json!({"market":id,"title":title,"leader":leader,"nominee_preview":nominees,
                "leader_name":leader.map(|i| self.people[i as usize].name()),
                "aspiration":leader.map(|i| format!("{:?}",self.people[i as usize].aspiration)),
                "next_election":office.and_then(|o| o.next_election),
                "nomination":office.and_then(|o| o.nomination.as_ref()),
                "last_election":office.and_then(|o| o.result.as_ref())})
            })
            .collect::<Vec<_>>())
    }

    pub(super) fn validate_governance(&self) -> Result<(), String> {
        let in_range = |p: u32| (p as usize) < self.agents.len();
        if self.governance.offices.iter().any(|(&m, o)| {
            m as usize >= self.markets.len()
                || o.leader.is_some_and(|p| !in_range(p))
                || o.nomination
                    .as_ref()
                    .is_some_and(|n| n.candidates.iter().any(|&p| !in_range(p)))
                || o.result
                    .as_ref()
                    .is_some_and(|e| e.votes.keys().any(|&p| !in_range(p)))
        }) || self
            .governance
            .happiness
            .iter()
            .any(|(&p, a)| !in_range(p) || a.market as usize >= self.markets.len())
        {
            return Err("Invalid settlement governance state".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn village() -> World {
        let mut w = World::new(6, 42).unwrap();
        w.markets[0].village = true;
        for i in 0..w.agents.len() {
            w.agents.home[i] = 0;
            w.agents.x[i] = w.markets[0].x;
            w.agents.y[i] = w.markets[0].y;
            w.people[i].initial_age_days = 20 * 365;
        }
        w.governance_step();
        w
    }

    #[test]
    fn election_charges_once_and_awards_expiring_happiness_across_save() {
        let mut w = village();
        w.tick = TERM - NOMINATION_WINDOW;
        w.people[0].initial_age_days = 11 * 365;
        w.people[1].initial_age_days = 0;
        w.people[0].aspiration = life::Aspiration::Farmer;
        w.people[2].aspiration = life::Aspiration::Farmer;
        w.people[2].happiness = 96.0;
        w.governance_step();
        let expected_fee = ((w.richest_resident_net_worth(0) + 9) / 10) as u64;
        let wallet = w.agents.coins[0];
        let coins = w.coins();
        assert_eq!(w.nomination_fee(0, 0), Some(expected_fee));
        assert_eq!(w.nomination_fee(1, 0), None);
        let (offers, n) = w.offer(0);
        assert!(offers[..n].iter().any(|c| c.action == Action::NominateSelf));
        w.agents.action[0] = Action::NominateSelf;
        w.agents.target[0] = 0;
        w.execute(0);
        w.agents.target[0] = 0;
        w.execute_nomination(0);
        assert_eq!(w.agents.coins[0], wallet - expected_fee);
        assert_eq!(w.coins(), coins);
        w.tick = TERM;
        w.governance_step();
        assert_eq!(w.governance.offices[&0].leader, Some(0));
        assert_eq!(w.governance.offices[&0].result.as_ref().unwrap().turnout, 5);
        assert_eq!(w.governance.offices[&0].next_election, Some(TERM * 2));
        assert_eq!(w.effective_happiness(2), 100.0);
        assert_eq!(w.leader_job_alignment(0, "PostFarmer"), 1.0);
        assert_eq!(w.leader_job_alignment(0, "FundBank"), 0.0);
        let path = std::env::temp_dir().join(format!("governance-{}.snapshot", std::process::id()));
        w.save(&path).unwrap();
        let mut replay = World::load(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        assert_eq!(w.governance, replay.governance);
        replay.governance_step();
        assert_eq!(w.governance, replay.governance);
        replay.agents.home[0] = 1;
        replay.governance_step();
        assert_eq!(replay.governance.offices[&0].leader, None);
        assert_eq!(replay.leader_job_alignment(0, "PostFarmer"), 0.0);
        replay.tick = TERM + YEAR;
        assert_eq!(replay.effective_happiness(2), 96.0);
        replay.check().unwrap();
    }

    #[test]
    fn ballot_aspirations_can_elect_a_candidate() {
        let mut w = village();
        w.people[0].aspiration = life::Aspiration::Farmer;
        for i in 1..6 {
            w.people[i].aspiration = life::Aspiration::Teacher;
        }
        w.governance.offices.get_mut(&0).unwrap().nomination = Some(Nomination {
            fee: 0,
            candidates: vec![0, 1],
        });
        w.tick = TERM;
        w.governance_step();
        assert_eq!(w.governance.offices[&0].leader, Some(1));
    }
}
