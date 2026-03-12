use once_cell::sync::Lazy;
use std::{collections::HashMap, sync::RwLock};

pub(crate) static HINTS_METRICS: Lazy<RwLock<HashMap<u32, HintRegisterInfo>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

#[derive(Clone, Debug)]
pub(crate) struct HintRegisterInfo {
    pub hint_name: String,
    pub count: u64,
    pub size: u64,
}

pub(crate) fn register_hint(hint_id: u32, hint_name: String) {
    HINTS_METRICS
        .write()
        .expect("HINTS_METRICS poisoned")
        .insert(hint_id, HintRegisterInfo { hint_name, count: 0, size: 0 });
}

pub(crate) fn inc_hint_count(hint_id: u32, hint_size: u64) {
    if let Ok(mut hints) = HINTS_METRICS.write() {
        if let Some(info) = hints.get_mut(&hint_id) {
            info.count += 1;
            info.size += hint_size;
        }
    }
}

pub(crate) fn print_metrics() {
    let hints = HINTS_METRICS.read().expect("HINTS_METRICS poisoned");
    let mut total_hints = 0;
    let mut total_size = 0;
    println!("Hints usage summary:");
    for (_, info) in hints.iter() {
        total_hints += info.count;
        total_size += info.size;
    }
    for (_, info) in hints.iter() {
        if info.count > 0 {
            let percentage = if total_size == 0 {
                0.0
            } else {
                ((info.size as f64 * 100.0) / total_size as f64 * 10.0).round() / 10.0
            };
            println!(
                "  {}: {}, {} bytes ({:.1}%)",
                info.hint_name, info.count, info.size, percentage
            );
        }
    }
    println!("Total hints: {}", total_hints);
    println!("Total size: {} bytes", total_size);
}

pub(crate) fn reset_metrics() {
    let mut hints = HINTS_METRICS.write().expect("HINTS_METRICS poisoned");
    for (_, info) in hints.iter_mut() {
        info.count = 0;
        info.size = 0;
    }
}
