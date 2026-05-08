use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use crate::{BackendProverOpts, UnitTestProver};

/// True if `$HOME/.zisk/provingKey` exists. Tests skip silently when not.
pub fn proving_key_available() -> bool {
    let Ok(home) = std::env::var("HOME") else { return false };
    PathBuf::from(home).join(".zisk").join("provingKey").exists()
}

/// One `UnitTestProver` per process, lazily initialised. `ProofMan::new()`
/// calls `MPI_Init`, which is one-shot — a second instance in the same
/// binary is undefined behaviour, not just expensive.
fn shared() -> &'static Mutex<UnitTestProver> {
    static PROVER: OnceLock<Mutex<UnitTestProver>> = OnceLock::new();
    PROVER.get_or_init(|| {
        Mutex::new(
            UnitTestProver::new(&BackendProverOpts::default())
                .expect("UnitTestProver::new failed; is ~/.zisk/provingKey present?"),
        )
    })
}

/// Borrow the shared [`UnitTestProver`] for the duration of a closure.
///
/// - Skips silently (with an `eprintln!`) if no proving key is present.
/// - Recovers from a poisoned mutex: a previous test panicking inside its
///   closure won't break subsequent tests, since `UnitTestProver` itself
///   holds no per-call state worth fearing.
/// - The closure should not call `with_prover` recursively — that would
///   deadlock on the mutex.
pub fn with_prover<F>(f: F)
where
    F: FnOnce(&UnitTestProver),
{
    if !proving_key_available() {
        eprintln!("skipping: no proving key at $HOME/.zisk/provingKey");
        return;
    }
    let guard = shared().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    f(&guard);
}
