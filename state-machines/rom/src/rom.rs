//! The `RomSM` module implements the ROM State Machine.
//!
//! `RomSM` is the component builder for ROM-related instances and their planner.

use std::sync::{atomic::AtomicU64, Arc, Mutex};

use crate::{RomError, RomInstance, RomResult};
use proofman_fields::PrimeField64;
use zisk_asm_runner::AsmRunnerRH;
use zisk_common::{create_atomic_vec, ComponentBuilder, Instance, InstanceCtx, LateValue};
use zisk_core::ZiskRom;
use zisk_pil::RomTrace;

/// Names the ROM histogram in [`LateValue`]'s wait log — the only place the
/// runner's residual is visible once it is no longer joined inside `execute`.
pub const RH_LABEL: &str = "ROM histogram";

/// The `RomSM` struct represents the ROM State Machine
pub struct RomSM {
    /// Parsed Zisk ROM, set via [`set_rom`](Self::set_rom) before each `build_instance` call.
    /// May be replaced between jobs (a long-lived worker can serve multiple ELFs).
    zisk_rom: Mutex<Option<Arc<ZiskRom>>>,

    /// Shared program instruction counter for monitoring ROM operations.
    inst_count: Arc<Vec<AtomicU64>>,

    /// This execution's ASM ROM histogram, parked here by the executor and read by the
    /// instances this state machine builds. It lives here rather than in the instance
    /// because the two have different lifetimes: an instance is rebuilt for every job,
    /// and the executor has to reach the runner again at the next job boundary. See
    /// [`LateValue`].
    rh: Arc<LateValue<AsmRunnerRH>>,
}

impl RomSM {
    /// Creates a new instance of the `RomSM` state machine.
    ///
    /// # Returns
    /// An `Arc`-wrapped instance of `RomSM`.
    pub fn new<F: PrimeField64>() -> Arc<Self> {
        Arc::new(Self {
            zisk_rom: Mutex::new(None),
            inst_count: Arc::new(create_atomic_vec(RomTrace::<F>::NUM_ROWS)),
            rh: Arc::new(LateValue::new(RH_LABEL)),
        })
    }

    /// The cell this execution's ROM-histogram runner is parked in.
    ///
    /// Handed out so the executor can park a runner before building instances, and
    /// drain it at a job boundary, without this state machine mediating either — see
    /// [`LateValue`] for the park / read / drain contract.
    pub fn rh(&self) -> Arc<LateValue<AsmRunnerRH>> {
        self.rh.clone()
    }

    /// Provides the parsed Zisk ROM. Must be called before the next `build_instance` call.
    /// May be called multiple times — each call replaces the previous ROM.
    ///
    /// # Errors
    /// Returns [`RomError::ZiskRomPoisoned`] if the internal mutex is poisoned.
    pub fn set_rom(&self, zisk_rom: Arc<ZiskRom>) -> RomResult<()> {
        *self.zisk_rom.lock().map_err(|_| RomError::ZiskRomPoisoned)? = Some(zisk_rom);
        Ok(())
    }
}

impl<F: PrimeField64> ComponentBuilder<F> for RomSM {
    /// Builds an instance of the ROM state machine.
    ///
    /// # Arguments
    /// * `ictx` - The context of the instance, containing the plan and its associated
    ///
    /// # Returns
    /// A boxed implementation of `RomInstance`.
    fn build_instance(&self, ictx: InstanceCtx) -> Box<dyn Instance<F>> {
        let zisk_rom = self
            .zisk_rom
            .lock()
            .expect("RomSM zisk_rom mutex poisoned")
            .as_ref()
            .expect("RomSM::build_instance called before set_rom")
            .clone();
        // Armed from the moment the runner is parked, so the backend does not depend on
        // whether it has finished yet, and stays armed for a runner that *failed* — that
        // failure must surface at witness time, not turn into a silent Rust downgrade.
        // The cell is shared, not consumed: an instance rebuilt within the same
        // execution still gets the ASM backend.
        if self.rh.is_armed() {
            Box::new(RomInstance::new_asm(zisk_rom, ictx, self.rh.clone()))
        } else {
            Box::new(RomInstance::new_rust(zisk_rom, ictx, self.inst_count.clone()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proofman_fields::Goldilocks;
    use zisk_asm_runner::{AsmRHData, AsmRunnerRH};
    use zisk_common::{CheckPoint, InstanceType, LateValueError, Plan};

    type F = Goldilocks;

    fn dummy_ictx() -> InstanceCtx {
        InstanceCtx::new(0, Plan::new(0, 0, None, InstanceType::Instance, CheckPoint::None, None))
    }

    fn asm_runner_rh_empty() -> AsmRunnerRH {
        // Empty histogram — on Linux x86_64 `AsmRunnerRH`'s `Drop` `mem::forget`s its
        // payload, so an empty `Vec` keeps the test leak-free.
        AsmRunnerRH::new(AsmRHData::new(0, vec![], vec![]))
    }

    /// Parks a runner that has already delivered its histogram, the shape the executor
    /// leaves behind once the emulation returns.
    fn park_finished_rh(sm: &RomSM) {
        sm.rh().park(std::thread::spawn(|| Ok(asm_runner_rh_empty())));
    }

    /// Parks a runner that failed, the shape a dead assembly child leaves behind.
    fn park_failed_rh(sm: &RomSM) {
        sm.rh().park(std::thread::spawn(|| Err(LateValueError::failed("runner died"))));
    }

    /// Whether `build_instance` picked the ASM backend, which is what
    /// `RomInstance::skip_collector` reports.
    fn builds_in_asm_mode(sm: &RomSM) -> bool {
        let instance = <RomSM as ComponentBuilder<F>>::build_instance(sm, dummy_ictx());
        instance.as_any().downcast_ref::<RomInstance>().unwrap().skip_collector()
    }

    /// A state machine with a ROM installed, ready for `build_instance`.
    fn sm_with_rom() -> Arc<RomSM> {
        let sm = RomSM::new::<F>();
        sm.set_rom(Arc::new(ZiskRom::default())).expect("set_rom");
        sm
    }

    #[test]
    fn new_allocates_inst_count_sized_to_rom_trace() {
        let sm = RomSM::new::<F>();
        assert_eq!(sm.inst_count.len(), RomTrace::<F>::NUM_ROWS);
    }

    #[test]
    fn new_starts_with_no_rom_and_a_disarmed_rh_cell() {
        let sm = RomSM::new::<F>();
        assert!(sm.zisk_rom.lock().unwrap().is_none());
        assert!(!sm.rh.is_armed());
    }

    #[test]
    fn set_rom_stores_the_rom() {
        let sm = RomSM::new::<F>();
        let rom = Arc::new(ZiskRom::default());
        sm.set_rom(rom.clone()).expect("set should succeed");
        let guard = sm.zisk_rom.lock().unwrap();
        assert!(Arc::ptr_eq(guard.as_ref().unwrap(), &rom));
    }

    #[test]
    fn set_rom_replaces_previous_rom() {
        let sm = RomSM::new::<F>();
        let first = Arc::new(ZiskRom::default());
        let second = Arc::new(ZiskRom::default());
        sm.set_rom(first.clone()).expect("first set");
        sm.set_rom(second.clone()).expect("second set replaces");
        let guard = sm.zisk_rom.lock().unwrap();
        assert!(Arc::ptr_eq(guard.as_ref().unwrap(), &second), "later set must win");
    }

    #[test]
    fn parking_a_runner_arms_the_cell() {
        let sm = RomSM::new::<F>();
        park_finished_rh(&sm);
        assert!(sm.rh.is_armed());
    }

    #[test]
    fn build_instance_picks_the_rust_backend_when_nothing_is_parked() {
        assert!(!builds_in_asm_mode(&sm_with_rom()), "the Rust backend must keep its collector");
    }

    #[test]
    fn build_instance_picks_the_asm_backend_when_a_runner_is_parked() {
        let sm = sm_with_rom();
        park_finished_rh(&sm);
        assert!(builds_in_asm_mode(&sm), "the ASM backend takes its witness from the histogram");
    }

    #[test]
    fn build_instance_keeps_the_asm_backend_across_rebuilds() {
        let sm = sm_with_rom();
        park_finished_rh(&sm);
        assert!(builds_in_asm_mode(&sm), "first instance");
        // The cell is shared, not consumed. An instance rebuilt within the same
        // execution must not fall back to the Rust backend: its collector is never
        // filled on the ASM path, so its witness would be an all-zero ROM trace.
        assert!(builds_in_asm_mode(&sm), "a rebuilt instance must not downgrade");
    }

    #[test]
    fn build_instance_keeps_the_asm_backend_for_a_failed_runner() {
        let sm = sm_with_rom();
        park_failed_rh(&sm);
        // A failure has to reach the witness as a failure. Picking the Rust backend
        // here would replace it with an all-zero trace and no error at all.
        assert!(builds_in_asm_mode(&sm), "a failed runner must not select the Rust backend");
    }

    #[test]
    fn draining_returns_the_state_machine_to_the_rust_backend() {
        let sm = sm_with_rom();
        park_finished_rh(&sm);
        sm.rh().drain();
        assert!(!sm.rh.is_armed(), "the next execution must not inherit this histogram");
        assert!(!builds_in_asm_mode(&sm));
    }

    #[test]
    #[should_panic(expected = "build_instance called before set_rom")]
    fn build_instance_panics_before_set_rom() {
        let sm = RomSM::new::<F>();
        let _ = <RomSM as ComponentBuilder<F>>::build_instance(&sm, dummy_ictx());
    }
}
