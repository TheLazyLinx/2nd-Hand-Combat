//! WebAssembly adapter for `sim-core`. Keeps browser bindings out of the native core.

use sim_core::{
    analyze_skirmish_scenario, catalog, validate_skirmish_scenario, Battle, Scenario,
    UnitInventoryUpdate, COMPACT_UNIT_BYTES,
};
use wasm_bindgen::prelude::*;

fn js_error(error: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}

#[wasm_bindgen]
pub fn catalog_json() -> Result<String, JsValue> {
    serde_json::to_string(&catalog()).map_err(js_error)
}

#[wasm_bindgen]
pub fn validate_scenario_json(json: &str, for_start: bool) -> Result<String, JsValue> {
    let scenario: Scenario = serde_json::from_str(json).map_err(js_error)?;
    serde_json::to_string(&validate_skirmish_scenario(&scenario, for_start)).map_err(js_error)
}

#[wasm_bindgen]
pub fn analyze_scenario_json(json: &str) -> Result<String, JsValue> {
    let scenario: Scenario = serde_json::from_str(json).map_err(js_error)?;
    serde_json::to_string(&analyze_skirmish_scenario(&scenario)).map_err(js_error)
}

#[wasm_bindgen]
pub struct WasmBattle {
    battle: Battle,
}

#[wasm_bindgen]
impl WasmBattle {
    #[wasm_bindgen(constructor)]
    pub fn new(json: &str) -> Result<WasmBattle, JsValue> {
        let scenario: Scenario = serde_json::from_str(json).map_err(js_error)?;
        Ok(Self {
            battle: Battle::new(scenario).map_err(js_error)?,
        })
    }

    pub fn advance(&mut self, steps: u32) -> Result<(), JsValue> {
        if steps > 5 {
            return Err(js_error(
                "At most five simulation steps may be advanced per batch.",
            ));
        }
        for _ in 0..steps {
            self.battle.step();
        }
        Ok(())
    }

    pub fn tick(&self) -> u32 {
        self.battle.tick()
    }

    pub fn elapsed(&self) -> f64 {
        self.battle.elapsed()
    }

    pub fn is_finished(&self) -> bool {
        self.battle.is_finished()
    }

    pub fn unit_count(&self) -> u32 {
        self.battle.unit_count() as u32
    }

    pub fn snapshot_json(&self) -> Result<String, JsValue> {
        serde_json::to_string(&self.battle.snapshot()).map_err(js_error)
    }

    pub fn compact_units_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.battle.unit_count() * COMPACT_UNIT_BYTES);
        self.battle.write_compact_units(&mut bytes);
        bytes
    }

    pub fn unit_inventory_if_changed_json(
        &self,
        unit_id: u32,
        last_version: u32,
    ) -> Result<Option<String>, JsValue> {
        let Some((version, inventory)) =
            self.battle.unit_inventory_if_changed(unit_id, last_version)
        else {
            return Ok(None);
        };
        let update = UnitInventoryUpdate {
            unit_id,
            version,
            inventory,
        };
        serde_json::to_string(&update).map(Some).map_err(js_error)
    }

    pub fn result_json(&self) -> Result<Option<String>, JsValue> {
        match self.battle.result() {
            Some(result) => serde_json::to_string(result).map(Some).map_err(js_error),
            None => Ok(None),
        }
    }

    pub fn stop(&mut self) {
        self.battle.stop();
    }
}
