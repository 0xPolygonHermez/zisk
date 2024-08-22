use log::debug;
use sm_binary::{BinaryBasicSM, BinaryExtensionSM, BinarySM};
use std::{error::Error, path::PathBuf, sync::Arc};
use zisk_pil::{Pilout, MAIN_AIR_IDS};

use p3_field::AbstractField;
use p3_goldilocks::Goldilocks;
use proofman::{WitnessLibrary, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx, WitnessPilout};
use proofman_util::{timer_start, timer_stop_and_log};
use sm_arith::{Arith3264SM, Arith32SM, Arith64SM, ArithSM};
use sm_main::MainSM;
use sm_mem::{MemAlignedSM, MemSM, MemUnalignedSM};

pub struct ZiskWitness<F> {
    pub proving_key_path: PathBuf,
    pub public_inputs_path: PathBuf,
    pub wcm: WitnessManager<F>,
    // State machines
    pub main_sm: Arc<MainSM<F>>,
    pub mem_sm: Arc<MemSM>,
    pub mem_aligned_sm: Arc<MemAlignedSM>,
    pub mem_unaligned_sm: Arc<MemUnalignedSM>,
    pub arith_sm: Arc<ArithSM>,
    pub arith_32_sm: Arc<Arith32SM>,
}

impl<F: AbstractField + Copy + Send + Sync + 'static> ZiskWitness<F> {
    const MY_NAME: &'static str = "ZiskLib ";

    pub fn new(
        rom_path: PathBuf,
        public_inputs_path: PathBuf,
        proving_key_path: PathBuf,
    ) -> Result<Self, Box<dyn Error>> {
        // Check rom_path path exists
        if !rom_path.exists() {
            return Err(format!("ROM file not found at path: {:?}", rom_path).into());
        }

        // Check public_inputs_path is a folder
        if !public_inputs_path.exists() {
            return Err(
                format!("Public inputs file not found at path: {:?}", public_inputs_path).into()
            );
        }

        // Check proving_key_path exists
        if !proving_key_path.exists() {
            return Err(
                format!("Proving key folder not found at path: {:?}", proving_key_path).into()
            );
        }

        // Check proving_key_path is a folder
        if !proving_key_path.is_dir() {
            return Err(
                format!("Proving key parameter must be a folder: {:?}", proving_key_path).into()
            );
        }

        let mut wcm = WitnessManager::new();

        // TODO REMOVE THIS WHEN READY IN ZISK_PIL
        pub const MEM_ALIGN_AIR_IDS: &[usize] = &[1, 2];
        pub const MEM_UNALIGNED_AIR_IDS: &[usize] = &[3, 4];
        pub const ARITH32_AIR_IDS: &[usize] = &[5];
        pub const BINARY_BASIC_AIR_IDS: &[usize] = &[6];
        pub const BINARY_EXTENDED_AIR_IDS: &[usize] = &[7];
        pub const ARITH64_AIR_IDS: &[usize] = &[8];
        pub const ARITH3264_AIR_IDS: &[usize] = &[9];

        let mem_aligned_sm = MemAlignedSM::new(&mut wcm, MEM_ALIGN_AIR_IDS);
        let mem_unaligned_sm = MemUnalignedSM::new(&mut wcm, MEM_UNALIGNED_AIR_IDS);
        let mem_sm = MemSM::new(&mut wcm, mem_aligned_sm.clone(), mem_unaligned_sm.clone());

        let binary_sm = BinaryBasicSM::new(&mut wcm, BINARY_BASIC_AIR_IDS);
        let binary_extension_sm = BinaryExtensionSM::new(&mut wcm, BINARY_EXTENDED_AIR_IDS);
        let binary_sm = BinarySM::new(&mut wcm, binary_sm.clone(), binary_extension_sm.clone());

        let arith_32_sm = Arith32SM::new(&mut wcm, ARITH32_AIR_IDS);
        let arith_64_sm = Arith64SM::new(&mut wcm, ARITH64_AIR_IDS);
        let arith_3264_sm = Arith3264SM::new(&mut wcm, ARITH3264_AIR_IDS);
        let arith_sm =
            ArithSM::new(&mut wcm, arith_32_sm.clone(), arith_64_sm.clone(), arith_3264_sm.clone());

        let main_sm = MainSM::new(
            &rom_path,
            &proving_key_path,
            &mut wcm,
            mem_sm.clone(),
            binary_sm.clone(),
            arith_sm.clone(),
            MAIN_AIR_IDS,
        );

        Ok(ZiskWitness {
            proving_key_path,
            public_inputs_path,
            wcm,
            main_sm,
            mem_sm,
            mem_aligned_sm,
            mem_unaligned_sm,
            arith_sm,
            arith_32_sm,
        })
    }
}

impl<F: AbstractField + Copy + Send + Sync + 'static> WitnessLibrary<F> for ZiskWitness<F> {
    fn start_proof(&mut self, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx) {
        log::info!("{}: Starting proof", Self::MY_NAME);

        self.wcm.start_proof(pctx, ectx);
    }

    fn end_proof(&mut self) {
        log::info!("{}: Finalizing proof", Self::MY_NAME);

        self.wcm.end_proof();
    }
    fn execute(&self, pctx: &mut ProofCtx<F>, ectx: &mut ExecutionCtx) {
        log::info!("{}: Executing proof", Self::MY_NAME);

        timer_start!(EXECUTE);
        // TODO let mut ectx = self.wcm.createExecutionContext(wneeds);
        // TODO Create the pool of threads to execute the state machines here?
        // elf, inputs i trace_steps
        self.main_sm.execute(&self.public_inputs_path, pctx, ectx);
        // TODO ectx.terminate();
        timer_stop_and_log!(EXECUTE);
    }

    fn calculate_witness(&mut self, stage: u32, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx) {
        self.wcm.calculate_witness(stage, pctx, ectx);
    }

    fn pilout(&self) -> WitnessPilout {
        Pilout::pilout()
    }
}

#[no_mangle]
pub extern "Rust" fn init_library(
    rom_path: Option<PathBuf>,
    public_inputs_path: PathBuf,
    proving_key_path: PathBuf,
) -> Result<Box<dyn WitnessLibrary<Goldilocks>>, Box<dyn Error>> {
    env_logger::builder()
        .format_timestamp(None)
        .format_level(true)
        .format_target(false)
        .filter_level(log::LevelFilter::Trace)
        .init();

    let rom_path = rom_path.ok_or("ROM path is required")?;

    let zisk_witness = ZiskWitness::new(rom_path, public_inputs_path, proving_key_path)?;
    Ok(Box::new(zisk_witness))
}
