use crate::{
    decisions,
    perception::{self, Memory, ThreatMemory, Unit},
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
            u.movement_multiplier = crate::definitions::surface_definition(crate::map::surface(
                &scenario.map,
                u.position,
            ))
            .movement_multiplier;
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
                    perception::observe_all(&self.units, &self.scenario.map, i, self.tick)
                } else {
                    Vec::new()
                }
            })
            .collect();
        for (i, seen) in observations.iter().enumerate() {
            if self.units[i].hp <= 0.0 {
                continue;
            }
            let previous_target = self.units[i].target;
            // Unit storage is ID-sorted, so the known-threat order remains stable as distances change.
            let contacts: Vec<_> = self
                .units
                .iter()
                .enumerate()
                .filter(|(j, _)| seen.contains(j))
                .map(|(_, u)| (u.placement.id, u.position))
                .collect();
            let me = &mut self.units[i];
            me.threat_memories
                .retain(|memory| self.tick - memory.tick <= 200);
            for (id, position) in contacts {
                if let Some(memory) = me.threat_memories.iter_mut().find(|memory| memory.id == id) {
                    memory.position = position;
                    memory.tick = self.tick;
                } else {
                    me.threat_memories.push(ThreatMemory {
                        id,
                        position,
                        tick: self.tick,
                    });
                }
            }
            me.threat_memories.sort_by_key(|memory| memory.id);
            me.known_threats.clear();
            me.known_threats
                .extend(me.threat_memories.iter().map(|memory| memory.position));
            if let Some(j) = seen.first() {
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
                if previous_target.is_some() {
                    self.units[i].needs_decision = true;
                }
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
                let visible_enemy = observations[i].first().map(|j| self.units[*j].position);
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
                        visible_enemy_revealed: observations[i]
                            .first()
                            .is_some_and(|j| self.tick < self.units[*j].revealed_until),
                        tick: self.tick,
                        allies,
                    },
                    &self.scenario.map,
                    faction_index,
                ))
            })
            .collect();
        for (u, decision) in self.units.iter_mut().zip(decisions) {
            if let Some(decision) = decision {
                let fallback = decision.fallback_goal;
                let old_fallback = u.fallback_goal;
                u.behavior = decision.behavior;
                u.goal = decision.goal;
                u.fallback_goal = fallback;
                u.patrol_index = decision.patrol_index;
                if let Some(route) = decision.route {
                    u.route = route;
                }
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
        crate::movement::execute(&mut self.units, &self.scenario.map, self.tick);
        crate::combat::execute(
            &mut self.units,
            &self.scenario.map,
            &mut self.rng,
            self.tick,
        );
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
                    terrain: TerrainStatus {
                        surface: crate::map::surface(&self.scenario.map, u.position),
                        elevation: crate::map::ground_height(&self.scenario.map, u.position),
                        movement_multiplier: u.movement_multiplier,
                        revealed: self.tick < u.revealed_until,
                        protection: u
                            .target
                            .and_then(|id| {
                                self.units
                                    .iter()
                                    .find(|other| other.placement.id == id && other.hp > 0.0)
                            })
                            .map(|other| {
                                1.0 - crate::map::shot_fraction(
                                    &self.scenario.map,
                                    other.position,
                                    u.position,
                                )
                            }),
                    },
                })
                .collect(),
            result: self.result.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn last_seen_is_individual_and_expires_without_hidden_updates() {
        let mut map = ArenaMap::flat(32, 20);
        map.obstacles.push(Obstacle {
            x: 5,
            y: 5,
            width: 1,
            height: 1,
            height_metres: 2.4,
        });
        let unit = |id, faction, position| UnitPlacement {
            id,
            faction,
            definition_id: "infantry".into(),
            position,
            doctrine: Doctrine {
                engagement_distance: 8.0,
                fallback_hp: 0.0,
                position_preference: PositionPreference::Balanced,
                route_preference: RoutePreference::Fastest,
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
        let mut map = ArenaMap::flat(32, 20);
        map.patrol = vec![Point { x: 12.5, y: 10.5 }];
        let unit = |id, faction, position| UnitPlacement {
            id,
            faction,
            definition_id: "infantry".into(),
            position,
            doctrine: Doctrine {
                engagement_distance: 8.0,
                fallback_hp: 0.0,
                position_preference: PositionPreference::Balanced,
                route_preference: RoutePreference::Fastest,
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
    #[test]
    fn all_seen_threats_retain_individual_last_positions_until_expiry() {
        let mut battle = test_battle();
        battle.units[0].position = Point { x: 2.5, y: 5.5 };
        battle.units[1].position = Point { x: 6.5, y: 5.5 };
        let mut third = battle.units[1].placement.clone();
        third.id = 3;
        third.position = Point { x: 7.5, y: 7.5 };
        battle.units.push(Unit::new(third));
        // Keep units stationary so only observation changes drive this regression.
        for u in &mut battle.units {
            u.placement.doctrine.fallback_hp = 100.0;
        }
        battle.step();
        assert_eq!(battle.units[0].threat_memories.len(), 2);
        let last_seen = battle.units[0]
            .threat_memories
            .iter()
            .find(|m| m.id == 2)
            .unwrap()
            .position;
        battle.scenario.map.obstacles.push(Obstacle {
            x: 8,
            y: 5,
            width: 1,
            height: 1,
            height_metres: 2.4,
        });
        battle.units[1].position = Point { x: 9.5, y: 5.5 };
        battle.units[0].position = Point { x: 2.5, y: 5.5 };
        battle.units[2].position = Point { x: 7.5, y: 7.5 };
        battle.step();
        assert_eq!(battle.units[0].target, Some(3));
        assert_eq!(battle.units[0].known_threats.len(), 2);
        assert_eq!(
            battle.units[0]
                .threat_memories
                .iter()
                .find(|m| m.id == 2)
                .unwrap()
                .position,
            last_seen
        );
        // Advancing time while the enemy remains hidden must expire that contact.
        battle.tick = 202;
        battle.units[0].position = Point { x: 2.5, y: 5.5 };
        battle.step();
        assert!(battle.units[0].threat_memories.iter().all(|m| m.id != 2));
    }
}
