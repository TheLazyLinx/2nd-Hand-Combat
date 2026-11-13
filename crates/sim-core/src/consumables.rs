use crate::types::ItemUseProperties;
use std::collections::BTreeMap;
pub(crate) const BLAST_RADIUS: f64 = 1.5;
pub(crate) const GRENADE_DAMAGE: f64 = 40.0;
pub(crate) const GRENADE_COOLDOWN_SECONDS: f64 = 3.0;
pub(crate) const MEDICAL_HEAL: f64 = 25.0;
pub(crate) const NEED_USE_THRESHOLD: f64 = 50.0;
pub(crate) const NEED_RESTORE: f64 = 40.0;
pub(crate) fn catalog() -> BTreeMap<String, ItemUseProperties> {
    let mut result = BTreeMap::new();
    result.insert(
        "frag-grenade".into(),
        ItemUseProperties {
            kind: "grenade".into(),
            blast_radius: Some(BLAST_RADIUS),
            damage: Some(GRENADE_DAMAGE),
            cooldown_seconds: Some(GRENADE_COOLDOWN_SECONDS),
            ..Default::default()
        },
    );
    result.insert(
        "medical-supply".into(),
        ItemUseProperties {
            kind: "medical".into(),
            heal_amount: Some(MEDICAL_HEAL),
            ..Default::default()
        },
    );
    for id in ["ration", "water"] {
        result.insert(
            id.into(),
            ItemUseProperties {
                kind: id.into(),
                restore_amount: Some(NEED_RESTORE),
                use_threshold: Some(NEED_USE_THRESHOLD),
                ..Default::default()
            },
        );
    }
    result
}
