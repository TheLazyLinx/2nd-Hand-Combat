//! Metric collection and deterministic FNV-1a outcome fingerprints for headless battles.

use sim_core::{Battle, Behavior, Scenario, Snapshot, Surface};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default)]
pub struct FactionMetrics {
    pub faction: u32,
    pub survivors: u32,
    pub casualties: u32,
    pub shots_fired: u32,
    pub damage_taken: f64,
    pub living_unit_ticks: u32,
    pub behavior_ticks: BTreeMap<String, u32>,
    pub in_contact_ticks: u32,
    pub cover_ticks: u32,
    pub protection_sum: f64,
    pub vegetation_ticks: u32,
    pub exposed_advancing_ticks: u32,
    pub first_shot_x_sum: f64,
    pub first_shot_count: u32,
    pub item_uses: BTreeMap<String, u32>,
}

impl FactionMetrics {
    pub fn mean_protection(&self) -> f64 {
        if self.in_contact_ticks == 0 {
            0.0
        } else {
            self.protection_sum / self.in_contact_ticks as f64
        }
    }
    pub fn cover_share(&self) -> f64 {
        if self.in_contact_ticks == 0 {
            0.0
        } else {
            self.cover_ticks as f64 / self.in_contact_ticks as f64
        }
    }
    pub fn vegetation_share(&self) -> f64 {
        if self.living_unit_ticks == 0 {
            0.0
        } else {
            self.vegetation_ticks as f64 / self.living_unit_ticks as f64
        }
    }
    pub fn behavior_share(&self, label: &str) -> f64 {
        if self.living_unit_ticks == 0 {
            0.0
        } else {
            *self.behavior_ticks.get(label).unwrap_or(&0) as f64 / self.living_unit_ticks as f64
        }
    }
    pub fn mean_first_shot_x(&self) -> Option<f64> {
        (self.first_shot_count > 0).then_some(self.first_shot_x_sum / self.first_shot_count as f64)
    }
    pub fn item_use_count(&self, item_id: &str) -> u32 {
        *self.item_uses.get(item_id).unwrap_or(&0)
    }
}

#[derive(Clone, Debug)]
pub struct RunMetrics {
    pub seed: u32,
    pub ticks: u32,
    pub elapsed: f64,
    pub winner: Option<u32>,
    pub reason: String,
    pub factions: Vec<FactionMetrics>,
    pub fingerprint: u64,
}

impl RunMetrics {
    pub fn faction(&self, id: u32) -> &FactionMetrics {
        self.factions
            .iter()
            .find(|f| f.faction == id)
            .expect("faction present in run metrics")
    }
}

fn behavior_key(behavior: &Behavior) -> &'static str {
    match behavior {
        Behavior::Searching => "searching",
        Behavior::Investigating => "investigating",
        Behavior::Advancing => "advancing",
        Behavior::Engaging => "engaging",
        Behavior::FallingBack => "falling_back",
        Behavior::Defending => "defending",
        Behavior::Dead => "dead",
    }
}

pub fn fnv1a_update(mut hash: u64, bytes: &[u8]) -> u64 {
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

pub fn fingerprint_snapshot(snapshot: &Snapshot) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    h = fnv1a_update(h, &snapshot.tick.to_le_bytes());
    if let Some(res) = &snapshot.result {
        h = fnv1a_update(h, &[1]);
        match res.winner {
            Some(w) => {
                h = fnv1a_update(h, &[1]);
                h = fnv1a_update(h, &w.to_le_bytes());
            }
            None => h = fnv1a_update(h, &[0]),
        }
        h = fnv1a_update(h, res.reason.as_str().as_bytes());
    } else {
        h = fnv1a_update(h, &[0]);
    }
    for u in &snapshot.units {
        h = fnv1a_update(h, &u.id.to_le_bytes());
        h = fnv1a_update(h, &u.faction.to_le_bytes());
        let qx = (u.position.x * 1e6).round() as i64;
        let qy = (u.position.y * 1e6).round() as i64;
        let qhp = (u.hp * 1e6).round() as i64;
        h = fnv1a_update(h, &qx.to_le_bytes());
        h = fnv1a_update(h, &qy.to_le_bytes());
        h = fnv1a_update(h, &qhp.to_le_bytes());
        h = fnv1a_update(h, behavior_key(&u.behavior).as_bytes());
        h = fnv1a_update(h, &u.shots_fired.to_le_bytes());
        let mut uses: Vec<(&str, u32)> = u
            .inventory
            .uses
            .iter()
            .map(|entry| (entry.item_id.as_str(), entry.count))
            .collect();
        uses.sort_by_key(|(id, _)| *id);
        for (id, count) in uses {
            h = fnv1a_update(h, id.as_bytes());
            h = fnv1a_update(h, &count.to_le_bytes());
        }
    }
    h
}

pub fn run_battle_metrics(scenario: Scenario, max_ticks: u32) -> RunMetrics {
    let seed = scenario.seed;
    let faction_ids: Vec<u32> = scenario.factions.iter().map(|f| f.id).collect();
    let mut battle = Battle::new(scenario).expect("valid standard scenario");
    let initial = battle.snapshot();

    let mut factions: Vec<FactionMetrics> = faction_ids
        .iter()
        .map(|&faction| FactionMetrics {
            faction,
            ..Default::default()
        })
        .collect();
    let mut prev_hp: BTreeMap<u32, f64> = initial.units.iter().map(|u| (u.id, u.hp)).collect();
    let mut prev_shots: BTreeMap<u32, u32> = initial
        .units
        .iter()
        .map(|u| (u.id, u.shots_fired))
        .collect();
    let mut recorded_first_shot: BTreeMap<u32, bool> = BTreeMap::new();

    for _ in 0..max_ticks {
        if battle.is_finished() {
            break;
        }
        battle.step();
        let snap = battle.snapshot();
        for u in &snap.units {
            let fm = factions
                .iter_mut()
                .find(|f| f.faction == u.faction)
                .expect("known faction");
            let before_hp = *prev_hp.get(&u.id).unwrap_or(&u.hp);
            if u.hp < before_hp {
                fm.damage_taken += before_hp - u.hp;
            }
            prev_hp.insert(u.id, u.hp);

            let before_shots = *prev_shots.get(&u.id).unwrap_or(&0);
            if u.shots_fired > before_shots && !recorded_first_shot.contains_key(&u.id) {
                recorded_first_shot.insert(u.id, true);
                fm.first_shot_x_sum += u.position.x;
                fm.first_shot_count += 1;
            }
            prev_shots.insert(u.id, u.shots_fired);

            if u.hp > 0.0 {
                fm.living_unit_ticks += 1;
                *fm.behavior_ticks
                    .entry(behavior_key(&u.behavior).into())
                    .or_default() += 1;
                if matches!(u.terrain.surface, Surface::Brush | Surface::Woodland) {
                    fm.vegetation_ticks += 1;
                }
                if let Some(prot) = u.terrain.protection {
                    fm.in_contact_ticks += 1;
                    fm.protection_sum += prot;
                    if prot > 0.01 {
                        fm.cover_ticks += 1;
                    }
                    if u.behavior == Behavior::Advancing && prot < 0.01 {
                        fm.exposed_advancing_ticks += 1;
                    }
                }
            }
        }
    }

    let final_snap = battle.snapshot();
    for fm in &mut factions {
        for u in final_snap.units.iter().filter(|u| u.faction == fm.faction) {
            if u.hp > 0.0 {
                fm.survivors += 1;
            } else {
                fm.casualties += 1;
            }
            fm.shots_fired += u.shots_fired;
            for item_use in &u.inventory.uses {
                *fm.item_uses.entry(item_use.item_id.clone()).or_default() += item_use.count;
            }
        }
    }

    let (winner, reason) = match &final_snap.result {
        Some(res) => (res.winner, res.reason.as_str().to_owned()),
        None => (None, "unfinished".into()),
    };
    RunMetrics {
        seed,
        ticks: final_snap.tick,
        elapsed: final_snap.elapsed,
        winner,
        reason,
        factions,
        fingerprint: fingerprint_snapshot(&final_snap),
    }
}
