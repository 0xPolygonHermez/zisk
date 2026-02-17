use once_cell::sync::Lazy;
use std::{collections::HashMap, sync::RwLock};

pub(crate) static HINTS_METRICS: Lazy<RwLock<HashMap<u32, HintRegisterInfo>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

#[derive(Clone, Debug)]
pub(crate) struct HintRegisterInfo {
    pub hint_name: String,
    pub count: u64,
}

pub(crate) fn register_hint(hint_id: u32, hint_name: String) {
    HINTS_METRICS
        .write()
        .expect("HINTS_METRICS poisoned")
        .insert(hint_id, HintRegisterInfo { hint_name, count: 0 });
}

pub(crate) fn inc_hint_count(hint_id: u32) {
    if let Ok(mut hints) = HINTS_METRICS.write() {
        if let Some(info) = hints.get_mut(&hint_id) {
            info.count += 1;
        }
    }
}

pub(crate) fn print_metrics() {
    let hints = HINTS_METRICS.read().expect("HINTS_METRICS poisoned");
    let mut total_hints = 0;
    println!("Hints usage summary:");
    for (_, info) in hints.iter() {
        if info.count > 0 {
            println!("  {}: {}", info.hint_name, info.count);
        }
        total_hints += info.count;
    }
    println!("Total hints: {}", total_hints);
}

pub(crate) fn reset_metrics() {
    let mut hints = HINTS_METRICS.write().expect("HINTS_METRICS poisoned");
    for (_, info) in hints.iter_mut() {
        info.count = 0;
    }
}
