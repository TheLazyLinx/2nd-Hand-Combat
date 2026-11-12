use sim_core::{analyze_scenario, catalog, validate_scenario, Battle, Scenario};
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
    serde_json::to_string(&validate_scenario(&scenario, for_start)).map_err(js_error)
}

#[wasm_bindgen]
pub fn analyze_scenario_json(json: &str) -> Result<String, JsValue> {
    let scenario: Scenario = serde_json::from_str(json).map_err(js_error)?;
    serde_json::to_string(&analyze_scenario(&scenario)).map_err(js_error)
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

    pub fn snapshot_json(&self) -> Result<String, JsValue> {
        serde_json::to_string(&self.battle.snapshot()).map_err(js_error)
    }
}
