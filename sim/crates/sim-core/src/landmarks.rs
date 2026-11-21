use super::*;

#[derive(Clone, Debug, Serialize)]
pub(super) struct Landmark {
    pub name: &'static str,
    pub kind: &'static str,
    pub x: f32,
    pub y: f32,
}

impl World {
    pub(super) fn init_landmarks(&mut self) {
        if self.map.spec.size == MapSize::Classic {
            return;
        }
        for market in self.markets.iter().take(3) {
            for (name, kind, terrain, angle) in [
                (
                    "Wayfarer's Pavilion",
                    "pavilion",
                    &[Terrain::Grass, Terrain::Fertile][..],
                    0.4_f32,
                ),
                ("Whispering Grove", "grove", &[Terrain::Forest][..], 2.5),
                (
                    "Cloudwatch Stones",
                    "stones",
                    &[Terrain::Grass, Terrain::Forest][..],
                    4.6,
                ),
            ] {
                let x = market.x + angle.cos() * 135.0;
                let y = market.y + angle.sin() * 135.0;
                let (x, y) = self.map.find_terrain(terrain, x, y, 0.0, 155.0);
                if !terrain.contains(&self.map.terrain_at(x, y))
                    || !self.map.buildable(x, y)
                    || self
                        .markets
                        .iter()
                        .take(3)
                        .any(|m| dist(x, y, m.x, m.y) < 75.0)
                    || self.landmarks.iter().any(|l| dist(x, y, l.x, l.y) < 65.0)
                {
                    continue;
                }
                self.landmarks.push(Landmark { name, kind, x, y });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovered_landmarks_offer_recreation_and_require_arrival() {
        let mut w = World::new_with_size(8, 42, Hill::default(), MapSize::X4).unwrap();
        let site = &w.landmarks[0];
        let (x, y) = (site.x, site.y);
        assert!(w.map.buildable(x, y));
        w.agents.x[0] = x + 15.0;
        w.agents.y[0] = y;
        w.people[0].initial_age_days = 25 * 365;
        w.people[0].happiness = 10.0;
        let (offers, count) = w.offer(0);
        assert!(offers[..count]
            .iter()
            .any(|offer| offer.action == Action::Play && offer.target == 0));
        w.agents.action[0] = Action::Play;
        w.agents.target[0] = 0;
        w.execute(0);
        assert_eq!(w.people[0].happiness, 10.0);
        w.agents.x[0] = x;
        w.agents.y[0] = y;
        w.agents.timer[0] = 3.0;
        w.execute(0);
        assert_eq!(w.people[0].happiness, 22.0);

        let path =
            std::env::temp_dir().join(format!("xianxia-landmarks-{}.snapshot", std::process::id()));
        w.save(&path).unwrap();
        let restored = World::load(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        assert_eq!(
            serde_json::json!(w.landmarks),
            serde_json::json!(restored.landmarks)
        );
    }
}
