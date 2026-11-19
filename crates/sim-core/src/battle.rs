use crate::{
    decisions,
    definitions::TUNING,
    navigation::SearchScratch,
    perception,
    terrain::TerrainCache,
    types::*,
    unit::{Memory, ThreatMemory, Unit},
    validation::validate_scenario,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StepTimings {
    pub sustainment_ns: u64,
    pub perception_ns: u64,
    pub decisions_ns: u64,
    pub movement_ns: u64,
    pub post_perception_ns: u64,
    pub item_actions_ns: u64,
    pub combat_ns: u64,
    pub results_ns: u64,
}
impl StepTimings {
    pub fn total_ns(&self) -> u64 {
        self.sustainment_ns
            + self.perception_ns
            + self.decisions_ns
            + self.movement_ns
            + self.post_perception_ns
            + self.item_actions_ns
            + self.combat_ns
            + self.results_ns
    }
}

pub(crate) trait StepTimer {
    fn mark_sustainment(&mut self);
    fn mark_perception(&mut self);
    fn mark_decisions(&mut self);
    fn mark_movement(&mut self);
    fn mark_post_perception(&mut self);
    fn mark_item_actions(&mut self);
    fn mark_combat(&mut self);
    fn mark_results(&mut self);
}

pub(crate) struct NoopTimer;
impl StepTimer for NoopTimer {
    #[inline(always)]
    fn mark_sustainment(&mut self) {}
    #[inline(always)]
    fn mark_perception(&mut self) {}
    #[inline(always)]
    fn mark_decisions(&mut self) {}
    #[inline(always)]
    fn mark_movement(&mut self) {}
    #[inline(always)]
    fn mark_post_perception(&mut self) {}
    #[inline(always)]
    fn mark_item_actions(&mut self) {}
    #[inline(always)]
    fn mark_combat(&mut self) {}
    #[inline(always)]
    fn mark_results(&mut self) {}
}

#[cfg(not(target_arch = "wasm32"))]
struct RealTimer {
    last: std::time::Instant,
    timings: StepTimings,
}
#[cfg(not(target_arch = "wasm32"))]
impl RealTimer {
    fn new() -> Self {
        Self {
            last: std::time::Instant::now(),
            timings: StepTimings::default(),
        }
    }
    fn lap(&mut self) -> u64 {
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(self.last).as_nanos() as u64;
        self.last = now;
        elapsed
    }
}
#[cfg(not(target_arch = "wasm32"))]
impl StepTimer for RealTimer {
    fn mark_sustainment(&mut self) {
        self.timings.sustainment_ns = self.lap();
    }
    fn mark_perception(&mut self) {
        self.timings.perception_ns = self.lap();
    }
    fn mark_decisions(&mut self) {
        self.timings.decisions_ns = self.lap();
    }
    fn mark_movement(&mut self) {
        self.timings.movement_ns = self.lap();
    }
    fn mark_post_perception(&mut self) {
        self.timings.post_perception_ns = self.lap();
    }
    fn mark_item_actions(&mut self) {
        self.timings.item_actions_ns = self.lap();
    }
    fn mark_combat(&mut self) {
        self.timings.combat_ns = self.lap();
    }
    fn mark_results(&mut self) {
        self.timings.results_ns = self.lap();
    }
}

pub struct Battle {
    scenario: Scenario,
    terrain: TerrainCache,
    scratch: SearchScratch,
    pre_observations: Vec<Vec<usize>>,
    post_observations: Vec<Vec<usize>>,
    moved: Vec<bool>,
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
        let terrain = TerrainCache::new(&scenario.map);
        let scratch = SearchScratch::new(terrain.width, terrain.height);
        let unit_count = units.len();
        Ok(Self {
            rng: scenario.seed as u64 + 1,
            scenario,
            terrain,
            scratch,
            pre_observations: vec![Vec::new(); unit_count],
            post_observations: vec![Vec::new(); unit_count],
            moved: vec![false; unit_count],
            units,
            tick: 0,
            result: None,
        })
    }
    pub fn is_finished(&self) -> bool {
        self.result.is_some()
    }
    pub fn step(&mut self) {
        self.step_with(&mut NoopTimer);
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub fn step_profiled(&mut self) -> StepTimings {
        let mut timer = RealTimer::new();
        self.step_with(&mut timer);
        timer.timings
    }
    fn step_with<T: StepTimer>(&mut self, timer: &mut T) {
        if self.result.is_some() {
            return;
        }
        #[cfg(test)]
        {
            self.terrain.sync_if_mutated(&self.scenario.map);
            let n = self.units.len();
            if self.pre_observations.len() != n {
                self.pre_observations.resize_with(n, Vec::new);
                self.post_observations.resize_with(n, Vec::new);
                self.moved.resize(n, false);
            }
        }
        self.tick += 1;
        for u in &mut self.units {
            crate::sustainment::advance(u);
        }
        timer.mark_sustainment();
        let terrain = self.terrain.view(&self.scenario.map);
        // Sight is refreshed every tick independently for each living unit.
        for i in 0..self.units.len() {
            perception::observe_all_into(
                &self.units,
                terrain,
                i,
                self.tick,
                &mut self.pre_observations[i],
            );
        }
        let mut contacts = Vec::new();
        for (i, seen) in self.pre_observations.iter().enumerate() {
            if self.units[i].hp <= 0.0 {
                continue;
            }
            if !seen.is_empty() {
                self.units[i].last_contact = self.tick;
            }
            let previous_target = self.units[i].target;
            contacts.clear();
            contacts.extend(seen.iter().map(|&j| {
                let u = &self.units[j];
                (u.placement.id, u.position)
            }));
            let first_seen = seen
                .first()
                .map(|&j| (self.units[j].placement.id, self.units[j].position));
            let me = &mut self.units[i];
            me.threat_memories
                .retain(|memory| self.tick - memory.tick <= TUNING.memory_expiry_ticks);
            for &(id, position) in &contacts {
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
            if let Some((id, position)) = first_seen {
                me.target = Some(id);
                if previous_target != Some(id) {
                    me.needs_decision = true;
                }
                me.memory = Some(Memory {
                    position,
                    tick: self.tick,
                    investigated: false,
                });
            } else {
                me.target = None;
                if previous_target.is_some() {
                    me.needs_decision = true;
                }
                if me
                    .memory
                    .as_ref()
                    .is_some_and(|m| self.tick - m.tick > TUNING.memory_expiry_ticks)
                {
                    me.memory = None;
                }
            }
        }
        timer.mark_perception();
        let mut decisions = Vec::with_capacity(self.units.len());
        for i in 0..self.units.len() {
            let cadence_due = (self.tick + self.units[i].placement.id)
                .is_multiple_of(TUNING.decision_interval_ticks);
            if self.units[i].hp <= 0.0 || (!cadence_due && !self.units[i].needs_decision) {
                decisions.push(None);
                continue;
            }
            let faction_index = self
                .scenario
                .factions
                .iter()
                .position(|f| *f == self.units[i].placement.faction)
                .unwrap_or(0);
            let unit = &self.units[i];
            let visible_enemy = self.pre_observations[i]
                .first()
                .map(|j| self.units[*j].position);
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
            decisions.push(Some(decisions::decide_with(
                &decisions::DecisionInput {
                    unit,
                    visible_enemy,
                    visible_enemy_revealed: self.pre_observations[i]
                        .first()
                        .is_some_and(|j| self.tick < self.units[*j].revealed_until),
                    tick: self.tick,
                    allies,
                },
                terrain,
                &mut self.scratch,
                faction_index,
            )));
        }
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
                if decision.mark_investigated {
                    if let Some(memory) = &mut u.memory {
                        memory.investigated = true;
                    }
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
        timer.mark_decisions();
        crate::movement::execute_with(
            &mut self.units,
            terrain,
            &mut self.scratch,
            self.tick,
            &mut self.moved,
        );
        timer.mark_movement();
        crate::combat::execute_with_context(
            &mut self.units,
            terrain,
            &mut self.rng,
            self.tick,
            &self.pre_observations,
            &self.moved,
            &mut self.post_observations,
            timer,
        );
        self.result = crate::results::evaluate(&self.scenario, &self.units, self.tick);
        timer.mark_results();
    }
    pub fn snapshot(&self) -> Snapshot {
        let terrain = self.terrain.view(&self.scenario.map);
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
                    inventory: u.inventory.snapshot(),
                    sustainment: Sustainment {
                        nutrition: u.nutrition,
                        hydration: u.hydration,
                        needs_factor: u.needs_factor(),
                        nutrition_shortage: crate::sustainment::shortage(u.nutrition),
                        hydration_shortage: crate::sustainment::shortage(u.hydration),
                    },
                    mobility_factor: u.inventory.load_factor() * u.needs_factor(),
                    terrain: TerrainStatus {
                        surface: crate::map::surface(&self.scenario.map, u.position),
                        elevation: self.terrain.ground_height(&self.scenario.map, u.position),
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
                                1.0 - crate::map::shot_fraction_t(
                                    terrain,
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
                grenades: GrenadeDoctrine::default(),
                medical: MedicalDoctrine::default(),
                out_of_ammo_behavior: OutOfAmmoBehavior::default(),
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
                grenades: GrenadeDoctrine::default(),
                medical: MedicalDoctrine::default(),
                out_of_ammo_behavior: OutOfAmmoBehavior::default(),
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
    #[test]
    fn long_battle_consumes_supplies_repeatably_then_terminal_state_freezes() {
        let mut battle = test_battle();
        assert!(battle
            .snapshot()
            .units
            .iter()
            .all(|unit| unit.terrain.movement_multiplier == 0.0));
        battle.scenario.map.patrol.clear();
        battle.scenario.time_limit = 200.0;
        let mut replay = Battle::new(battle.scenario.clone()).unwrap();
        for _ in 0..4000 {
            battle.step();
            replay.step();
        }
        let terminal = battle.snapshot();
        assert_eq!(terminal, replay.snapshot());
        assert_eq!(terminal.result.as_ref().unwrap().reason, "timeout");
        for unit in &terminal.units {
            assert_eq!(unit.hp, 100.0);
            assert_eq!(unit.shots_fired, 0);
            assert_eq!(unit.inventory.weight_grams, 7520);
            assert_eq!(unit.inventory.stowed_volume_ml, 3900);
            assert_eq!(unit.sustainment.hydration, 0.0);
            assert!((unit.sustainment.nutrition - 80.0).abs() < 1e-7);
            assert!(unit.sustainment.hydration_shortage);
            assert!((unit.mobility_factor - 0.65025).abs() < 1e-7);
            for id in ["ration", "water"] {
                assert_eq!(
                    unit.inventory
                        .uses
                        .iter()
                        .find(|used| used.item_id == id)
                        .unwrap()
                        .count,
                    2
                );
            }
        }
        for _ in 0..20 {
            battle.step();
        }
        assert_eq!(battle.snapshot(), terminal);
    }
    #[test]
    fn unreachable_memory_returns_unit_to_patrol_in_battle() {
        let mut battle = test_battle();
        battle.scenario.map.obstacles.push(Obstacle {
            x: 15,
            y: 0,
            width: 1,
            height: 20,
            height_metres: 2.5,
        });
        battle.units[0].memory = Some(Memory {
            position: Point { x: 20.5, y: 10.5 },
            tick: 0,
            investigated: false,
        });
        battle.step();
        assert!(battle.units[0].memory.as_ref().unwrap().investigated);
        assert_eq!(battle.snapshot().units[0].behavior, Behavior::Searching);
        assert!(battle.snapshot().units[0].position.x > 2.5);
    }
}
