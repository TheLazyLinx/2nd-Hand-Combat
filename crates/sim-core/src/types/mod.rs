mod doctrine;
mod inventory;
mod map;
mod scenario;
mod snapshot;

pub use doctrine::*;
pub use inventory::*;
pub use map::*;
pub use scenario::*;
pub use snapshot::*;

#[cfg(feature = "ts-export")]
pub fn export_typescript_types() -> String {
    use std::collections::BTreeMap;
    use ts_rs::{Config, TypeVisitor, TS};

    struct Collector<'a> {
        cfg: &'a Config,
        decls: BTreeMap<String, String>,
    }

    impl TypeVisitor for Collector<'_> {
        fn visit<T: TS + 'static + ?Sized>(&mut self) {
            if T::output_path().is_some() {
                let name = T::ident(self.cfg);
                if self.decls.contains_key(&name) {
                    return;
                }
                self.decls.insert(name, T::decl(self.cfg));
            }
            T::visit_dependencies(self);
            T::visit_generics(self);
        }
    }

    let cfg = Config::default();
    let mut collector = Collector {
        cfg: &cfg,
        decls: BTreeMap::new(),
    };
    collector.visit::<Catalog>();
    collector.visit::<Scenario>();
    collector.visit::<Snapshot>();
    collector.visit::<Validation>();
    collector.visit::<ScenarioAnalysis>();

    let mut out = String::from(
        "// Generated from crates/sim-core by `npm run gen:types`. Do not edit by hand.\n\n",
    );
    for decl in collector.decls.values() {
        out.push_str("export ");
        out.push_str(decl);
        out.push('\n');
    }
    out
}

#[cfg(all(test, feature = "ts-export"))]
mod ts_export_tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};

    fn ts_declarations() -> BTreeMap<String, String> {
        let mut map = BTreeMap::new();
        for line in export_typescript_types().lines() {
            if let Some(rest) = line.strip_prefix("export type ") {
                if let Some((name, body)) = rest.split_once(" = ") {
                    map.insert(name.to_owned(), body.to_owned());
                }
            }
        }
        map
    }

    fn ts_object_keys(body: &str) -> BTreeSet<String> {
        let inner = body
            .trim()
            .trim_end_matches(';')
            .trim()
            .strip_prefix('{')
            .and_then(|s| s.strip_suffix('}'))
            .expect("expected object type body");
        let mut keys = BTreeSet::new();
        let mut depth = 0usize;
        let mut start = 0usize;
        for (i, ch) in inner.char_indices() {
            match ch {
                '{' | '<' | '[' | '(' => depth += 1,
                '}' | '>' | ']' | ')' => depth = depth.saturating_sub(1),
                ',' if depth == 0 => {
                    let field = inner[start..i].trim();
                    if let Some((k, _)) = field.split_once(':') {
                        keys.insert(k.trim().trim_end_matches('?').to_owned());
                    }
                    start = i + 1;
                }
                _ => {}
            }
        }
        let tail = inner[start..].trim();
        if let Some((k, _)) = tail.split_once(':') {
            keys.insert(k.trim().trim_end_matches('?').to_owned());
        }
        keys
    }

    fn assert_json_keys_match_ts<T: serde::Serialize>(
        decls: &BTreeMap<String, String>,
        ts_name: &str,
        value: &T,
        allow_omitted: bool,
    ) {
        let json = serde_json::to_value(value).expect("serialize to json");
        let obj = json.as_object().expect("expected json object");
        let json_keys: BTreeSet<String> = obj.keys().cloned().collect();
        let body = decls
            .get(ts_name)
            .unwrap_or_else(|| panic!("missing TS declaration for {ts_name}"));
        let ts_keys = ts_object_keys(body);
        if allow_omitted {
            assert!(
                json_keys.is_subset(&ts_keys),
                "{ts_name}: serde keys {json_keys:?} not subset of TS keys {ts_keys:?}"
            );
        } else {
            assert_eq!(
                json_keys, ts_keys,
                "{ts_name}: serde keys must exactly match TS keys"
            );
        }
    }

    fn assert_enum_variants_match_ts<T: serde::Serialize>(
        decls: &BTreeMap<String, String>,
        ts_name: &str,
        variants: &[T],
    ) {
        let body = decls
            .get(ts_name)
            .unwrap_or_else(|| panic!("missing TS declaration for {ts_name}"));
        for variant in variants {
            let token = serde_json::to_string(variant).expect("serialize enum variant");
            assert!(
                body.contains(&token),
                "{ts_name}: serialized variant {token} missing from TS union `{body}`"
            );
        }
    }

    #[test]
    fn serde_json_wire_format_matches_exported_typescript_contract() {
        let decls = ts_declarations();
        let cat = crate::catalog();
        assert_json_keys_match_ts(&decls, "Catalog", &cat, false);
        assert_json_keys_match_ts(&decls, "EditorRules", &cat.editor_rules, false);
        assert_json_keys_match_ts(&decls, "MapPreset", &cat.maps[0], false);
        assert_json_keys_match_ts(&decls, "ArenaMap", &cat.maps[0].map, false);
        assert_json_keys_match_ts(&decls, "Obstacle", &cat.maps[0].map.obstacles[0], false);
        assert_json_keys_match_ts(&decls, "Point", &cat.maps[0].map.patrol[0], false);
        assert_json_keys_match_ts(&decls, "SurfaceDefinition", &cat.surfaces[0], false);
        assert_json_keys_match_ts(&decls, "TerrainCellInfo", &cat.terrain_cells[0][0], false);
        assert_json_keys_match_ts(&decls, "UnitDefinition", &cat.units[0], false);
        assert_json_keys_match_ts(&decls, "WeaponDefinition", &cat.weapons[0], false);
        assert_json_keys_match_ts(&decls, "FactionDefinition", &cat.default_factions[0], false);
        assert_json_keys_match_ts(&decls, "Doctrine", &cat.default_doctrine, false);
        assert_json_keys_match_ts(
            &decls,
            "GrenadeDoctrine",
            &cat.default_doctrine.grenades,
            false,
        );
        assert_json_keys_match_ts(
            &decls,
            "MedicalDoctrine",
            &cat.default_doctrine.medical,
            false,
        );
        assert_json_keys_match_ts(&decls, "DoctrineRules", &cat.doctrine_rules, false);
        assert_json_keys_match_ts(
            &decls,
            "Rule",
            &cat.doctrine_rules.grenade_max_distance,
            false,
        );
        assert_json_keys_match_ts(&decls, "ItemDefinition", &cat.items[0], false);
        assert_json_keys_match_ts(
            &decls,
            "LoadoutEntry",
            &cat.units[0].default_loadout[0],
            false,
        );
        assert_json_keys_match_ts(
            &decls,
            "NamedLoadout",
            &NamedLoadout {
                name: "Marksman".into(),
                entries: cat.units[0].default_loadout.clone(),
            },
            false,
        );
        assert_json_keys_match_ts(&decls, "InventorySnapshot", &cat.default_inventory, false);
        assert_json_keys_match_ts(
            &decls,
            "InventoryEntry",
            &cat.default_inventory.entries[0],
            false,
        );
        for props in cat.item_use_properties.values() {
            assert_json_keys_match_ts(&decls, "ItemUseProperties", props, true);
        }
        assert_json_keys_match_ts(
            &decls,
            "ItemUse",
            &ItemUse {
                item_id: "water".into(),
                count: 1,
            },
            false,
        );

        let scenario = Scenario {
            map: ArenaMap::flat(8, 8),
            seed: 1,
            factions: cat.default_factions.clone(),
            loadouts: vec![NamedLoadout {
                name: "Light".into(),
                entries: vec![LoadoutEntry {
                    item_id: "rifle".into(),
                    quantity: 1,
                }],
            }],
            units: vec![
                UnitPlacement {
                    id: 1,
                    faction: 0,
                    definition_id: cat.units[0].id.clone(),
                    position: Point { x: 2.5, y: 2.5 },
                    doctrine: cat.default_doctrine.clone(),
                    loadout: None,
                },
                UnitPlacement {
                    id: 2,
                    faction: 1,
                    definition_id: cat.units[0].id.clone(),
                    position: Point { x: 5.5, y: 2.5 },
                    doctrine: cat.default_doctrine.clone(),
                    loadout: Some(UnitLoadout::Named {
                        name: "Light".into(),
                    }),
                },
            ],
            time_limit: Some(1.0),
        };
        assert_json_keys_match_ts(&decls, "Scenario", &scenario, false);
        assert_json_keys_match_ts(&decls, "UnitPlacement", &scenario.units[0], true);
        assert_json_keys_match_ts(&decls, "UnitPlacement", &scenario.units[1], false);
        let unit_loadout_decl = decls
            .get("UnitLoadout")
            .expect("missing TS declaration for UnitLoadout");
        for variant in [
            UnitLoadout::Named {
                name: "Light".into(),
            },
            UnitLoadout::Inline {
                entries: vec![LoadoutEntry {
                    item_id: "rifle".into(),
                    quantity: 1,
                }],
            },
        ] {
            let val = serde_json::to_value(&variant).expect("serialize UnitLoadout");
            let kind = val.get("kind").and_then(|v| v.as_str()).expect("kind tag");
            assert!(
                unit_loadout_decl.contains(&format!("\"{kind}\"")),
                "UnitLoadout kind {kind} missing from TS union `{unit_loadout_decl}`"
            );
        }

        let validation = crate::validate_scenario(&scenario, true);
        assert_json_keys_match_ts(&decls, "Validation", &validation, false);
        assert_json_keys_match_ts(
            &decls,
            "ValidationIssue",
            &ValidationIssue {
                message: "example".into(),
                unit_id: Some(1),
            },
            false,
        );

        let analysis = crate::analyze_scenario(&scenario);
        assert_json_keys_match_ts(&decls, "ScenarioAnalysis", &analysis, false);
        assert_json_keys_match_ts(
            &decls,
            "AnalysisIssue",
            &AnalysisIssue {
                severity: "warning".into(),
                message: "example".into(),
                unit_id: None,
                position: Some(Point { x: 0.5, y: 0.5 }),
            },
            false,
        );

        let mut battle = crate::Battle::new(scenario).unwrap();
        for _ in 0..25 {
            battle.step();
        }
        let snap = battle.snapshot();
        assert_json_keys_match_ts(&decls, "Snapshot", &snap, false);
        assert_json_keys_match_ts(&decls, "UnitSnapshot", &snap.units[0], false);
        assert_json_keys_match_ts(&decls, "TerrainStatus", &snap.units[0].terrain, false);
        assert_json_keys_match_ts(&decls, "Sustainment", &snap.units[0].sustainment, false);
        let result = snap.result.as_ref().expect("timeout result");
        assert_json_keys_match_ts(&decls, "BattleResult", result, false);
        assert_json_keys_match_ts(&decls, "FactionResult", &result.factions[0], false);

        assert_enum_variants_match_ts(
            &decls,
            "Surface",
            &[
                Surface::Open,
                Surface::Road,
                Surface::Mud,
                Surface::Brush,
                Surface::Woodland,
            ],
        );
        assert_enum_variants_match_ts(
            &decls,
            "PositionPreference",
            &[
                PositionPreference::Balanced,
                PositionPreference::Cover,
                PositionPreference::Concealment,
            ],
        );
        assert_enum_variants_match_ts(
            &decls,
            "RoutePreference",
            &[RoutePreference::Fastest, RoutePreference::Protected],
        );
        assert_enum_variants_match_ts(
            &decls,
            "GrenadePolicy",
            &[
                GrenadePolicy::Never,
                GrenadePolicy::Clusters,
                GrenadePolicy::AnySafeTarget,
                GrenadePolicy::EmptyRifleOnly,
            ],
        );
        assert_enum_variants_match_ts(
            &decls,
            "MedicalPolicy",
            &[
                MedicalPolicy::Never,
                MedicalPolicy::OutOfContact,
                MedicalPolicy::LowHealth,
            ],
        );
        assert_enum_variants_match_ts(
            &decls,
            "OutOfAmmoBehavior",
            &[
                OutOfAmmoBehavior::Withdraw,
                OutOfAmmoBehavior::ContinueDoctrine,
            ],
        );
        assert_enum_variants_match_ts(
            &decls,
            "PatrolDirection",
            &[PatrolDirection::Forward, PatrolDirection::Reverse],
        );
        assert_enum_variants_match_ts(
            &decls,
            "Behavior",
            &[
                Behavior::Searching,
                Behavior::Investigating,
                Behavior::Advancing,
                Behavior::Engaging,
                Behavior::FallingBack,
                Behavior::Defending,
                Behavior::Dead,
            ],
        );
    }
}
