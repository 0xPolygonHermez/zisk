//! The `RomSM` module implements the ROM State Machine.
//!
//! `RomSM` is the for ROM-related instances and their planner.

use std::sync::{atomic::AtomicU64, Arc, Mutex, OnceLock};

use crate::{RomError, RomInstance, RomPlanner, RomResult};
use asm_runner::AsmRunnerRH;
use fields::PrimeField64;
use zisk_common::{create_atomic_vec, ComponentBuilder, Instance, InstanceCtx, Planner};
use zisk_core::ZiskRom;
use zisk_pil::RomTrace;

/// The `RomSM` struct represents the ROM State Machine
pub struct RomSM {
    /// Zisk Rom, set once via [`set_rom`](Self::set_rom) before the first `build_instance` call.
    zisk_rom: OnceLock<Arc<ZiskRom>>,

    /// Shared program instruction counter for monitoring ROM operations.
    inst_count: Arc<Vec<AtomicU64>>,

    /// ASM-runner ROM histogram, set via [`set_rh_data`](Self::set_rh_data) when running in ASM mode.
    rh_data: Mutex<Option<AsmRunnerRH>>,
}

impl RomSM {
    /// Creates a new instance of the `RomSM` state machine.
    ///
    /// # Returns
    /// An `Arc`-wrapped instance of `RomSM`.
    pub fn new<F: PrimeField64>() -> Arc<Self> {
        Arc::new(Self {
            zisk_rom: OnceLock::new(),
            inst_count: Arc::new(create_atomic_vec(RomTrace::<F>::NUM_ROWS)),
            rh_data: Mutex::new(None),
        })
    }

    /// Provides the ASM-runner ROM histogram. Must be called before `build_instance`
    /// when running in ASM mode; in Rust mode it is not called at all.
    ///
    /// # Errors
    /// Returns [`RomError::RhDataPoisoned`] if the internal mutex is poisoned.
    pub fn set_rh_data(&self, handler: AsmRunnerRH) -> RomResult<()> {
        *self.rh_data.lock().map_err(|_| RomError::RhDataPoisoned)? = Some(handler);
        Ok(())
    }

    /// Provides the parsed Zisk ROM. Must be called exactly once before `build_instance`.
    ///
    /// # Errors
    /// Returns [`RomError::RomAlreadySet`] if called more than once.
    pub fn set_rom(&self, zisk_rom: Arc<ZiskRom>) -> RomResult<()> {
        self.zisk_rom.set(zisk_rom).map_err(|_| RomError::RomAlreadySet)?;
        Ok(())
    }
}

impl<F: PrimeField64> ComponentBuilder<F> for RomSM {
    /// Builds a planner for ROM-related instances.
    ///
    /// # Returns
    /// A boxed implementation of `RomPlanner`.
    fn build_planner(&self) -> Box<dyn Planner> {
        Box::new(RomPlanner)
    }

    /// Builds an instance of the ROM state machine.
    ///
    /// # Arguments
    /// * `ictx` - The context of the instance, containing the plan and its associated
    ///
    /// # Returns
    /// A boxed implementation of `RomInstance`.
    fn build_instance(&self, ictx: InstanceCtx) -> Box<dyn Instance<F>> {
        let zisk_rom =
            self.zisk_rom.get().expect("RomSM::build_instance called before set_rom").clone();
        if let Some(rh_data) = self.rh_data.lock().expect("RomSM rh_data mutex poisoned").take() {
            Box::new(RomInstance::new_asm(zisk_rom, ictx, rh_data))
        } else {
            Box::new(RomInstance::new_rust(zisk_rom, ictx, self.inst_count.clone()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use asm_runner::{AsmRHData, AsmRunnerRH};
    use fields::Goldilocks;
    use zisk_common::{CheckPoint, InstanceType, Plan};

    type F = Goldilocks;

    fn dummy_ictx() -> InstanceCtx {
        InstanceCtx::new(0, Plan::new(0, 0, None, InstanceType::Instance, CheckPoint::None, None))
    }

    fn asm_runner_rh_empty() -> AsmRunnerRH {
        // Empty histogram — `AsmRunnerRH`'s `Drop` `mem::forget`s its payload, so an empty
        // `Vec` keeps the test leak-free.
        AsmRunnerRH::new(AsmRHData::new(0, vec![]))
    }

    #[test]
    fn new_allocates_inst_count_sized_to_rom_trace() {
        let sm = RomSM::new::<F>();
        assert_eq!(sm.inst_count.len(), RomTrace::<F>::NUM_ROWS);
    }

    #[test]
    fn new_starts_with_no_rom_and_no_rh_data() {
        let sm = RomSM::new::<F>();
        assert!(sm.zisk_rom.get().is_none());
        assert!(sm.rh_data.lock().unwrap().is_none());
    }

    #[test]
    fn set_rom_stores_the_rom() {
        let sm = RomSM::new::<F>();
        let rom = Arc::new(ZiskRom::default());
        sm.set_rom(rom.clone()).expect("first set should succeed");
        assert!(Arc::ptr_eq(sm.zisk_rom.get().unwrap(), &rom));
    }

    #[test]
    fn set_rom_returns_already_set_on_double_set() {
        let sm = RomSM::new::<F>();
        sm.set_rom(Arc::new(ZiskRom::default())).expect("first set should succeed");
        let err = sm.set_rom(Arc::new(ZiskRom::default())).expect_err("second set must fail");
        assert!(matches!(err, RomError::RomAlreadySet), "got {err:?}");
    }

    #[test]
    fn set_rh_data_stores_the_handler() {
        let sm = RomSM::new::<F>();
        sm.set_rh_data(asm_runner_rh_empty()).expect("set should succeed");
        assert!(sm.rh_data.lock().unwrap().is_some());
    }

    #[test]
    fn build_instance_picks_rust_mode_when_no_rh_data() {
        let sm = RomSM::new::<F>();
        sm.set_rom(Arc::new(ZiskRom::default())).expect("set_rom");
        let instance = <RomSM as ComponentBuilder<F>>::build_instance(&sm, dummy_ictx());
        let rom_instance = instance.as_any().downcast_ref::<RomInstance>().unwrap();
        assert!(!rom_instance.skip_collector(), "Rust mode should not skip the collector");
    }

    #[test]
    fn build_instance_picks_asm_mode_when_rh_data_present() {
        let sm = RomSM::new::<F>();
        sm.set_rom(Arc::new(ZiskRom::default())).expect("set_rom");
        sm.set_rh_data(asm_runner_rh_empty()).expect("set_rh_data");
        let instance = <RomSM as ComponentBuilder<F>>::build_instance(&sm, dummy_ictx());
        let rom_instance = instance.as_any().downcast_ref::<RomInstance>().unwrap();
        assert!(rom_instance.skip_collector(), "ASM mode should skip the collector");
    }

    #[test]
    fn build_instance_consumes_rh_data_after_first_call() {
        let sm = RomSM::new::<F>();
        sm.set_rom(Arc::new(ZiskRom::default())).expect("set_rom");
        sm.set_rh_data(asm_runner_rh_empty()).expect("set_rh_data");
        let _first = <RomSM as ComponentBuilder<F>>::build_instance(&sm, dummy_ictx());
        // rh_data is consumed by the first build_instance; the next call falls back to Rust.
        let second = <RomSM as ComponentBuilder<F>>::build_instance(&sm, dummy_ictx());
        let rom_instance = second.as_any().downcast_ref::<RomInstance>().unwrap();
        assert!(!rom_instance.skip_collector(), "rh_data consumed → next instance is Rust mode");
    }

    #[test]
    #[should_panic(expected = "build_instance called before set_rom")]
    fn build_instance_panics_before_set_rom() {
        let sm = RomSM::new::<F>();
        let _ = <RomSM as ComponentBuilder<F>>::build_instance(&sm, dummy_ictx());
    }
}
