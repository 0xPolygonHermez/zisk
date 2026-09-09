//! The `RomSM` module implements the ROM State Machine.
//!
//! `RomSM` is the component builder for ROM-related instances and their planner.

use std::sync::{atomic::AtomicU64, Arc, Mutex};

use crate::{RomError, RomInstance, RomResult};
use proofman_fields::PrimeField64;
use zisk_asm_runner::RhCell;
use zisk_common::{create_atomic_vec, ComponentBuilder, Instance, InstanceCtx};
use zisk_core::ZiskRom;
use zisk_pil::RomTrace;

/// The `RomSM` struct represents the ROM State Machine
pub struct RomSM {
    /// Parsed Zisk ROM, set via [`set_rom`](Self::set_rom) before each `build_instance` call.
    /// May be replaced between jobs (a long-lived worker can serve multiple ELFs).
    zisk_rom: Mutex<Option<Arc<ZiskRom>>>,

    /// Shared program instruction counter for monitoring ROM operations.
    inst_count: Arc<Vec<AtomicU64>>,

    /// Handoff cell for the ASM ROM-histogram runner, armed by the executor via
    /// [`rh_cell`](Self::rh_cell) when running in ASM mode. Shared with every instance
    /// this SM builds, which is why it outlives them: see [`RhCell`].
    rh: Arc<RhCell>,
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
            rh: Arc::new(RhCell::new()),
        })
    }

    /// The ASM ROM-histogram handoff cell, shared with every instance this SM builds.
    ///
    /// Parking a runner on it (before `build_instance`) is what selects the ASM path;
    /// the runner is joined later, by whoever first reads the histogram. Callers drive
    /// the cell directly — see [`RhCell`] for the park / read / drain contract.
    pub fn rh_cell(&self) -> Arc<RhCell> {
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
        // Armed from the moment the handle is parked, so the mode does not depend on
        // whether the runner has finished yet. The cell is shared, not consumed: an
        // instance rebuilt within the same execution still gets ASM mode.
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
    use zisk_common::{CheckPoint, InstanceType, Plan};

    type F = Goldilocks;

    fn dummy_ictx() -> InstanceCtx {
        InstanceCtx::new(0, Plan::new(0, 0, None, InstanceType::Instance, CheckPoint::None, None))
    }

    fn asm_runner_rh_empty() -> AsmRunnerRH {
        // Empty histogram — on Linux x86_64 `AsmRunnerRH`'s `Drop` `mem::forget`s its
        // payload, so an empty `Vec` keeps the test leak-free.
        AsmRunnerRH::new(AsmRHData::new(0, vec![], vec![]))
    }

    /// Parks an already-finished runner, the state the executor leaves behind on the
    /// ASM path once the emulation returns.
    fn park_finished_rh(sm: &RomSM) {
        sm.rh_cell().park(std::thread::spawn(|| Ok(asm_runner_rh_empty())));
    }

    /// Builds an instance and reports whether it came out in ASM mode. `skip_collector`
    /// is the observable difference: only the ASM path produces its witness without one.
    fn builds_in_asm_mode(sm: &RomSM) -> bool {
        let instance = <RomSM as ComponentBuilder<F>>::build_instance(sm, dummy_ictx());
        let rom_instance =
            instance.as_any().downcast_ref::<RomInstance>().expect("built a RomInstance");
        rom_instance.skip_collector()
    }

    /// A ROM state machine with its ROM set, ready to build instances.
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
    fn drain_disarms_the_cell() {
        let sm = RomSM::new::<F>();
        park_finished_rh(&sm);
        sm.rh_cell().drain();
        assert!(!sm.rh.is_armed(), "the next execution must not inherit this histogram");
    }

    #[test]
    fn build_instance_picks_rust_mode_when_no_rh_data() {
        assert!(!builds_in_asm_mode(&sm_with_rom()), "Rust mode should not skip the collector");
    }

    #[test]
    fn build_instance_picks_asm_mode_when_a_runner_is_parked() {
        let sm = sm_with_rom();
        park_finished_rh(&sm);
        assert!(builds_in_asm_mode(&sm), "ASM mode should skip the collector");
    }

    #[test]
    fn build_instance_keeps_asm_mode_across_rebuilds() {
        let sm = sm_with_rom();
        park_finished_rh(&sm);
        assert!(builds_in_asm_mode(&sm), "first build is ASM mode");

        // The cell is shared, not consumed: rebuilding inside the same execution must not
        // silently downgrade to Rust mode. See `RhCell::is_armed` for what that would cost.
        assert!(builds_in_asm_mode(&sm), "still ASM mode on the second build");
    }

    #[test]
    fn build_instance_stays_in_asm_mode_after_a_failed_runner() {
        // The failure has to reach `compute_witness` as an error rather than being papered
        // over by the Rust path — see `RhCell::is_armed`.
        let sm = sm_with_rom();
        sm.rh_cell().park(std::thread::spawn(|| panic!("RH runner died")));
        sm.rh_cell().resolve().expect_err("the runner failed");

        assert!(builds_in_asm_mode(&sm), "still ASM mode, so the failure surfaces");
    }

    #[test]
    fn build_instance_picks_asm_mode_before_the_runner_finishes() {
        let sm = sm_with_rom();
        sm.rh_cell().park(std::thread::spawn(|| {
            std::thread::sleep(std::time::Duration::from_millis(50));
            Ok(asm_runner_rh_empty())
        }));

        // The whole point of deferring the join: the mode is known at build time even
        // though the histogram is not ready yet.
        assert!(builds_in_asm_mode(&sm), "ASM mode must not wait for the runner");
    }

    #[test]
    #[should_panic(expected = "build_instance called before set_rom")]
    fn build_instance_panics_before_set_rom() {
        let sm = RomSM::new::<F>();
        let _ = <RomSM as ComponentBuilder<F>>::build_instance(&sm, dummy_ictx());
    }
}
