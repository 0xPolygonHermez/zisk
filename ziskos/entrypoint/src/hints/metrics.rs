use std::{collections::HashMap, sync::RwLock};
use once_cell::sync::Lazy;

pub (crate) static HINTS_METRICS: Lazy<RwLock<HashMap<u32, HintRegisterInfo>>> = Lazy::new(|| RwLock::new(HashMap::new()));

#[derive(Clone, Debug)]
pub(crate) struct HintRegisterInfo {
    pub hint_name: String,
    pub count: u64,
}

pub(crate) fn register_hint(hint_id: u32, hint_name: String) {
    HINTS_METRICS.write().expect("HINTS poisoned").insert(hint_id, HintRegisterInfo { hint_name, count: 0 });
}

pub(crate) fn inc_hint_count(hint_id: u32) {
    if let Ok(mut hints) = HINTS_METRICS.write() {
        if let Some(info) = hints.get_mut(&hint_id) {
            info.count += 1;
        }
    }
}