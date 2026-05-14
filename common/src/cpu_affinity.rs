//! Pinning helper for Intel hybrid CPUs (P-cores + E-cores).
//!
//! On a 14900K-class machine, scheduling latency-sensitive threads on E-cores
//! (4.4 GHz, no SMT) causes large run-to-run variance compared to P-cores
//! (up to 6.0 GHz, 2-way SMT). The kernel doesn't reliably keep our threads
//! on P-cores without an explicit hint, so we call `sched_setaffinity`
//! ourselves on threads we control.
//!
//! Detection is done at startup by reading the per-CPU
//! `cpufreq/cpuinfo_max_freq` sysfs files: P-cores have a strictly higher
//! max frequency than E-cores on every Intel hybrid SKU. We cache the
//! resulting mask in a `OnceLock` so detection is one-shot.
//!
//! If the host isn't a hybrid CPU (all max freqs equal, e.g. a server CPU
//! or AMD), or if the sysfs files aren't readable, `pin_current_thread_to_p_cores`
//! is a silent no-op.

use std::sync::OnceLock;

/// Mask of CPU indices that belong to performance cores.
/// `None` means "no hybrid detected — don't pin anything."
static P_CORE_CPUS: OnceLock<Option<Vec<usize>>> = OnceLock::new();

fn detect_p_cores() -> Option<Vec<usize>> {
    // Read max frequencies of all online CPUs.
    let mut freqs: Vec<(usize, u64)> = Vec::new();
    for cpu in 0..256 {
        let path = format!("/sys/devices/system/cpu/cpu{cpu}/cpufreq/cpuinfo_max_freq");
        match std::fs::read_to_string(&path) {
            Ok(s) => {
                if let Ok(hz) = s.trim().parse::<u64>() {
                    freqs.push((cpu, hz));
                }
            }
            Err(_) => break, // first missing CPU ends the scan
        }
    }
    if freqs.is_empty() {
        return None;
    }

    // Intel hybrid SKUs (Alder Lake / Raptor Lake / Meteor Lake / 14th-gen)
    // expose **three** freq buckets:
    //   - "favored" P-cores (e.g. 6.0 GHz Thermal Velocity Boost)
    //   - regular  P-cores (e.g. 5.7 GHz)
    //   - E-cores              (e.g. 4.4 GHz)
    // Picking only the strict-max bucket misses the regular P-cores. We
    // instead detect the *lowest* bucket as the E-core band and treat
    // everything strictly above it as a P-core. The gap between P and E is
    // ~1 GHz on every Intel hybrid SKU, so a simple `>` check is robust.
    let min_freq = freqs.iter().map(|&(_, f)| f).min().unwrap();
    let p_cores: Vec<usize> =
        freqs.iter().filter(|&&(_, f)| f > min_freq).map(|&(c, _)| c).collect();

    // Homogeneous (no E-cores) or all-E-cores — nothing to do.
    if p_cores.is_empty() || p_cores.len() == freqs.len() {
        return None;
    }
    Some(p_cores)
}

fn p_core_cpus() -> Option<&'static [usize]> {
    P_CORE_CPUS.get_or_init(detect_p_cores).as_deref()
}

/// Pin the **calling** thread to the host's performance cores.
/// Silent no-op on non-hybrid hosts or non-Linux targets.
pub fn pin_current_thread_to_p_cores() {
    #[cfg(target_os = "linux")]
    {
        let Some(cpus) = p_core_cpus() else { return };
        unsafe {
            let mut set: libc::cpu_set_t = std::mem::zeroed();
            libc::CPU_ZERO(&mut set);
            for &c in cpus {
                libc::CPU_SET(c, &mut set);
            }
            // pid = 0 → current thread
            let _ = libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &set);
        }
    }
}

/// One-time log line summarising what was detected. Safe to call multiple
/// times — `OnceLock` guarantees detection runs once.
pub fn log_p_core_detection() {
    match p_core_cpus() {
        Some(cpus) => {
            tracing::info!(
                "[affinity] detected {} performance-core CPUs: {:?}",
                cpus.len(),
                cpus
            );
        }
        None => {
            tracing::info!("[affinity] no hybrid CPU detected; thread pinning disabled");
        }
    }
}

/// Install the global rayon thread pool with each worker pinned to a P-core.
/// Must be called before anything else uses rayon — once a global pool is
/// initialised, `build_global` becomes a no-op. Safe to call multiple times;
/// returns whether the install succeeded.
///
/// Pool size: defaults to the number of P-core logical CPUs (e.g. 16 on a
/// 14900K with SMT). Pass `num_threads = Some(n)` to override.
pub fn install_pinned_rayon_global_pool(num_threads: Option<usize>) -> bool {
    let Some(cpus) = p_core_cpus() else {
        // Homogeneous CPU — leave the default rayon pool alone.
        return false;
    };
    let n = num_threads.unwrap_or(cpus.len());

    let result = rayon::ThreadPoolBuilder::new()
        .num_threads(n)
        .spawn_handler(|thread| {
            let mut b = std::thread::Builder::new();
            if let Some(name) = thread.name() {
                b = b.name(name.to_owned());
            }
            if let Some(stack_size) = thread.stack_size() {
                b = b.stack_size(stack_size);
            }
            b.spawn(|| {
                pin_current_thread_to_p_cores();
                thread.run();
            })?;
            Ok(())
        })
        .build_global();
    result.is_ok()
}
