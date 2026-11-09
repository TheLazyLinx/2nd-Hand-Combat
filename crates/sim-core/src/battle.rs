use crate::{
    decisions,
    perception::{self, Memory, Unit},
    types::*,
    validation::validate_scenario,
};

pub struct Battle {
    scenario: Scenario,
    units: Vec<Unit>,
    tick: u32,
    rng: u64,
    result: Option<BattleResult>,
}
impl Battle {
    pub fn new(scenario: Scenario) -> Result<Self, String> {
        let validation = validate_scenario(&scenario, true);
        if !validation.valid {
            return Err(validation
                .issues
                .iter()
                .map(|i| i.message.as_str())
                .collect::<Vec<_>>()
                .join(" "));
        }
        let mut units: Vec<_> = scenario.units.iter().cloned().map(Unit::new).collect();
        units.sort_by_key(|u| u.placement.id);
        for u in &mut units {
            if !scenario.map.patrol.is_empty() {
                u.patrol_index = (u.placement.id as usize) % scenario.map.patrol.len();
            }
        }
        Ok(Self {
            rng: scenario.seed as u64 + 1,
            scenario,
            units,
            tick: 0,
            result: None,
        })
    }
    pub fn step(&mut self) {
        if self.result.is_some() {
            return;
        }
        self.tick += 1;
        // Sight is refreshed every tick independently for each living unit.
        let observations: Vec<_> = (0..self.units.len())
            .map(|i| {
                if self.units[i].hp > 0.0 {
                    perception::observe(&self.units, &self.scenario.map, i)
                } else {
                    None
                }
            })
            .collect();
        for (i, seen) in observations.iter().enumerate() {
            if self.units[i].hp <= 0.0 {
                continue;
            }
            let previous_target = self.units[i].target;
            if let Some(j) = seen {
                let position = self.units[*j].position;
                let id = self.units[*j].placement.id;
                self.units[i].target = Some(id);
                if previous_target != Some(id) {
                    self.units[i].needs_decision = true;
                }
                self.units[i].memory = Some(Memory {
                    position,
                    tick: self.tick,
                    investigated: false,
                });
            } else {
                self.units[i].target = None;
                if self.units[i]
                    .memory
                    .as_ref()
                    .is_some_and(|m| self.tick - m.tick > 200)
                {
                    self.units[i].memory = None;
                }
            }
        }
        let decisions: Vec<_> = (0..self.units.len())
            .map(|i| {
                if self.units[i].hp <= 0.0 || (self.tick % 10 != 1 && !self.units[i].needs_decision)
                {
                    return None;
                }
                let faction_index = self
                    .scenario
                    .factions
                    .iter()
                    .position(|f| *f == self.units[i].placement.faction)
                    .unwrap_or(0);
                let unit = &self.units[i];
                let visible_enemy = observations[i].map(|j| self.units[j].position);
                let allies = self
                    .units
                    .iter()
                    .filter(|other| {
                        other.hp > 0.0
                            && other.placement.faction == unit.placement.faction
                            && other.placement.id != unit.placement.id
                    })
                    .map(|other| other.position)
                    .collect();
                Some(decisions::decide(
                    &decisions::DecisionInput {
                        unit,
                        visible_enemy,
                        allies,
                    },
                    &self.scenario.map,
                    faction_index,
                ))
            })
            .collect();
        for (u, decision) in self.units.iter_mut().zip(decisions) {
            if let Some((behavior, goal, fallback, patrol)) = decision {
                let old_fallback = u.fallback_goal;
                u.behavior = behavior;
                u.goal = goal;
                u.fallback_goal = fallback;
                u.patrol_index = patrol;
                u.needs_decision = false;
                u.fallback_threat = if fallback.is_some() && fallback == old_fallback {
                    u.fallback_threat
                } else if fallback.is_some() {
                    u.memory.as_ref().map(|m| m.position)
                } else {
                    None
                };
            }
        }
        crate::movement::execute(&mut self.units, &self.scenario.map);
        crate::combat::execute(&mut self.units, &self.scenario.map, &mut self.rng);
        self.result = crate::results::evaluate(&self.scenario, &self.units, self.tick);
    }
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            tick: self.tick,
            elapsed: self.tick as f64 * crate::definitions::STEP,
            units: self
                .units
                .iter()
                .map(|u| UnitSnapshot {
                    id: u.placement.id,
                    faction: u.placement.faction,
                    position: u.position,
                    hp: u.hp,
                    behavior: u.behavior.clone(),
                    target_id: u.target,
                    shots_fired: u.shots,
                })
                .collect(),
            result: self.result.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog;
    #[test]
    fn last_seen_is_individual_and_expires_without_hidden_updates() {
        let mut map = catalog().map;
        map.walls = vec![Point { x: 5.0, y: 5.0 }];
        map.cover.clear();
        map.patrol.clear();
        let unit = |id, faction, position| UnitPlacement {
            id,
            faction,
            definition_id: "infantry".into(),
            position,
            doctrine: Doctrine {
                engagement_distance: 8.0,
                fallback_hp: 0.0,
            },
        };
        let scenario = Scenario {
            map,
            seed: 1,
            factions: vec![0, 1],
            units: vec![
                unit(1, 0, Point { x: 3.5, y: 5.5 }),
                unit(2, 1, Point { x: 4.5, y: 5.5 }),
                unit(3, 1, Point { x: 30.5, y: 18.5 }),
            ],
            time_limit: 180.0,
        };
        let mut battle = Battle::new(scenario).unwrap();
        battle.step();
        battle.units[1].position = Point { x: 6.5, y: 5.5 };
        battle.units[1].hp = 0.0;
        battle.step();
        assert_eq!(battle.snapshot().units[0].target_id, None);
        assert_eq!(
            battle.units[0].memory.as_ref().unwrap().position,
            Point { x: 4.5, y: 5.5 }
        );
        for _ in 0..201 {
            battle.step();
        }
        assert!(battle.units[0].memory.is_none());
        assert_eq!(battle.snapshot().units[0].behavior, Behavior::Searching);
    }
    fn test_battle() -> Battle {
        let mut map = catalog().map;
        map.walls.clear();
        map.cover.clear();
        map.patrol = vec![Point { x: 12.5, y: 10.5 }];
        let unit = |id, faction, position| UnitPlacement {
            id,
            faction,
            definition_id: "infantry".into(),
            position,
            doctrine: Doctrine {
                engagement_distance: 8.0,
                fallback_hp: 0.0,
            },
        };
        Battle::new(Scenario {
            map,
            seed: 1,
            factions: vec![0, 1],
            units: vec![
                unit(1, 0, Point { x: 2.5, y: 10.5 }),
                unit(2, 1, Point { x: 28.5, y: 10.5 }),
            ],
            time_limit: 180.0,
        })
        .unwrap()
    }
    #[test]
    fn new_sighting_interrupts_patrol_on_the_next_tick() {
        let mut battle = test_battle();
        battle.step();
        assert_eq!(battle.snapshot().units[0].behavior, Behavior::Searching);
        battle.units[1].position = Point { x: 8.5, y: 10.5 };
        battle.step();
        assert_eq!(battle.snapshot().units[0].behavior, Behavior::Engaging);
        assert_eq!(battle.snapshot().units[0].shots_fired, 1);
    }
    #[test]
    fn threshold_crossing_interrupts_engagement() {
        let mut battle = test_battle();
        battle.units[1].position = Point { x: 8.5, y: 10.5 };
        battle.units[0].hp = 40.0;
        battle.units[0].placement.doctrine.fallback_hp = 30.0;
        battle.step();
        assert_eq!(battle.snapshot().units[0].hp, 20.0);
        battle.step();
        assert_eq!(battle.snapshot().units[0].behavior, Behavior::FallingBack);
    }
    #[test]
    fn fallback_reconsiders_when_threat_moves_across_unit() {
        let mut battle = test_battle();
        battle.units[0].position = Point { x: 10.5, y: 10.5 };
        battle.units[1].position = Point { x: 14.5, y: 10.5 };
        battle.units[0].placement.doctrine.fallback_hp = 100.0;
        battle.step();
        assert!(battle.units[0].goal.unwrap().x < 10.5);
        battle.units[1].position = Point { x: 5.5, y: 10.5 };
        for _ in 0..10 {
            battle.step();
        }
        assert!(battle.units[0].goal.unwrap().x > battle.units[0].position.x);
    }
    #[test]
    fn simultaneous_damage_takes_priority_over_final_tick_timeout() {
        let mut battle = test_battle();
        battle.scenario.time_limit = 1.0;
        battle.tick = 19;
        battle.units[0].position = Point { x: 3.5, y: 10.5 };
        battle.units[1].position = Point { x: 7.5, y: 10.5 };
        battle.units[0].hp = 20.0;
        battle.units[1].hp = 20.0;
        battle.step();
        let result = battle.snapshot().result.unwrap();
        assert_eq!(result.reason, "simultaneous_elimination");
        assert_eq!(result.winner, None);
        assert_eq!(result.factions.iter().map(|f| f.casualties).sum::<u32>(), 2);
    }
}
