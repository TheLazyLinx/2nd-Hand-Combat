use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(rename_all = "camelCase"))]
#[serde(rename_all = "camelCase")]
pub struct Doctrine {
    pub engagement_distance: f64,
    pub fallback_hp: f64,
    pub position_preference: PositionPreference,
    pub route_preference: RoutePreference,
    #[serde(default)]
    pub grenades: GrenadeDoctrine,
    #[serde(default)]
    pub medical: MedicalDoctrine,
    #[serde(default)]
    pub out_of_ammo_behavior: OutOfAmmoBehavior,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(rename_all = "camelCase"))]
#[serde(rename_all = "camelCase")]
pub enum GrenadePolicy {
    Never,
    Clusters,
    AnySafeTarget,
    EmptyRifleOnly,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(rename_all = "camelCase"))]
#[serde(rename_all = "camelCase")]
pub struct GrenadeDoctrine {
    pub policy: GrenadePolicy,
    pub max_distance: f64,
    pub min_enemies: u32,
    pub safety_margin: f64,
}
impl Default for GrenadeDoctrine {
    fn default() -> Self {
        Self {
            policy: GrenadePolicy::Clusters,
            max_distance: 6.0,
            min_enemies: 2,
            safety_margin: 0.25,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(rename_all = "camelCase"))]
#[serde(rename_all = "camelCase")]
pub enum MedicalPolicy {
    Never,
    OutOfContact,
    LowHealth,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(rename_all = "camelCase"))]
#[serde(rename_all = "camelCase")]
pub struct MedicalDoctrine {
    pub policy: MedicalPolicy,
    pub hp_threshold: f64,
    pub safe_seconds: f64,
    pub cooldown_seconds: f64,
}
impl Default for MedicalDoctrine {
    fn default() -> Self {
        Self {
            policy: MedicalPolicy::OutOfContact,
            hp_threshold: 50.0,
            safe_seconds: 5.0,
            cooldown_seconds: 10.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(rename_all = "camelCase"))]
#[serde(rename_all = "camelCase")]
pub enum OutOfAmmoBehavior {
    #[default]
    Withdraw,
    ContinueDoctrine,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(rename_all = "camelCase"))]
#[serde(rename_all = "camelCase")]
pub enum PositionPreference {
    Balanced,
    Cover,
    Concealment,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(rename_all = "camelCase"))]
#[serde(rename_all = "camelCase")]
pub enum RoutePreference {
    Fastest,
    Protected,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
pub struct Rule {
    pub min: f64,
    pub max: f64,
    pub step: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(rename_all = "camelCase"))]
#[serde(rename_all = "camelCase")]
pub struct DoctrineRules {
    pub grenade_max_distance: Rule,
    pub grenade_min_enemies: Rule,
    pub grenade_safety_margin: Rule,
    pub medical_hp_threshold: Rule,
    pub medical_safe_seconds: Rule,
    pub medical_cooldown_seconds: Rule,
}

#[cfg(test)]
mod doctrine_serde_tests {
    use super::*;
    #[test]
    fn absent_new_fields_receive_defaults_but_partial_groups_are_rejected() {
        let legacy = r#"{"engagementDistance":8,"fallbackHp":30,"positionPreference":"balanced","routePreference":"fastest"}"#;
        let d: Doctrine = serde_json::from_str(legacy).unwrap();
        assert_eq!(d.grenades.policy, GrenadePolicy::Clusters);
        assert_eq!(d.medical.policy, MedicalPolicy::OutOfContact);
        assert_eq!(d.out_of_ammo_behavior, OutOfAmmoBehavior::Withdraw);
        let partial = r#"{"engagementDistance":8,"fallbackHp":30,"positionPreference":"balanced","routePreference":"fastest","medical":{"policy":"never"}}"#;
        assert!(serde_json::from_str::<Doctrine>(partial).is_err());
    }
}
