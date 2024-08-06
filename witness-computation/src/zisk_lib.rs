use log::debug;
use std::{path::PathBuf, sync::Arc};
use zisk_pil::{Pilout, MAIN_AIR_IDS};

use p3_field::AbstractField;
use p3_goldilocks::Goldilocks;
use proofman::WCManager;
use proofman_common::{ExecutionCtx, ProofCtx, WitnessPilout};
use proofman_util::{timer_start, timer_stop_and_log};
use sm_arith::ArithSM;
use sm_arith_32::Arith32SM;
use sm_arith_3264::Arith3264SM;
use sm_arith_64::Arith64SM;
use sm_main::MainSM;
use sm_mem::MemSM;
use sm_mem_aligned::MemAlignedSM;
use sm_mem_unaligned::MemUnalignedSM;
use wchelpers::WCLibrary;

pub struct ZiskWC<F> {
    pub elf_rom: PathBuf,
    pub proving_key_path: PathBuf,
    pub public_inputs_path: PathBuf,
    pub wcm: WCManager<F>,
    // State machines
    pub main_sm: Arc<MainSM>,
    pub mem_sm: Arc<MemSM>,
    pub mem_aligned_sm: Arc<MemAlignedSM>,
    pub mem_unaligned_sm: Arc<MemUnalignedSM>,
    pub arith_sm: Arc<ArithSM>,
    pub arith_32_sm: Arc<Arith32SM>,
}

impl<F: AbstractField> ZiskWC<F> {
    // TODO The path must be a relative path to the current directory where the library is loaded
    // TODO The alternative is passing the path from outside
    const DEFAULT_ELF_ROM: &'static str = "../zisk/witness-computation/rom/zisk.elf";

    pub fn new(elf_rom: PathBuf, proving_key_path: PathBuf, public_inputs_path: PathBuf) -> Self {
        let mut wcm = WCManager::new();

        // TODO REMOVE THIS WHEN READY IN ZISK_PIL
        pub const MEM_ALIGN_AIR_IDS: &[usize] = &[1, 2];
        pub const MEM_UNALIGNED_AIR_IDS: &[usize] = &[3, 4];
        pub const ARITH32_AIR_IDS: &[usize] = &[5];
        pub const ARITH64_AIR_IDS: &[usize] = &[6];
        pub const ARITH3264_AIR_IDS: &[usize] = &[7];

        let mem_aligned_sm = MemAlignedSM::new(&mut wcm, MEM_ALIGN_AIR_IDS);
        let mem_unaligned_sm = MemUnalignedSM::new(&mut wcm, MEM_UNALIGNED_AIR_IDS);
        let mem_sm = MemSM::new(&mut wcm, mem_aligned_sm.clone(), mem_unaligned_sm.clone());

        let arith_32_sm = Arith32SM::new(&mut wcm, ARITH32_AIR_IDS);
        let arith_64_sm = Arith64SM::new(&mut wcm, ARITH64_AIR_IDS);
        let arith_3264_sm = Arith3264SM::new(&mut wcm, ARITH3264_AIR_IDS);
        let arith_sm =
            ArithSM::new(&mut wcm, arith_32_sm.clone(), arith_64_sm.clone(), arith_3264_sm.clone());

        // Check elf rom file exists
        if !std::path::Path::new(&elf_rom).exists() {
            panic!("File {} does not exist", elf_rom.display());
        }

        let main_sm = MainSM::new(
            &elf_rom,
            &proving_key_path,
            &mut wcm,
            mem_sm.clone(),
            arith_sm.clone(),
            MAIN_AIR_IDS,
        );

        ZiskWC {
            elf_rom,
            proving_key_path,
            public_inputs_path,
            wcm,
            main_sm,
            mem_sm,
            mem_aligned_sm,
            mem_unaligned_sm,
            arith_sm,
            arith_32_sm,
        }
    }
}

impl<F: AbstractField + Send + Sync> WCLibrary<F> for ZiskWC<F> {
    fn start_proof(&mut self, pctx: &mut ProofCtx<F>, ectx: &mut ExecutionCtx) {
        self.wcm.start_proof(pctx, ectx);
    }

    fn end_proof(&mut self) {
        self.wcm.end_proof();
    }
    fn execute(&self, pctx: &mut ProofCtx<F>, ectx: &mut ExecutionCtx) {
        timer_start!(EXECUTE);
        // TODO let mut ectx = self.wcm.createExecutionContext(wneeds);
        // TODO Create the pool of threads to execute the state machines here?
        // elf, inputs i trace_steps
        self.main_sm.execute(&self.public_inputs_path, pctx, ectx);
        // TODO ectx.terminate();
        timer_stop_and_log!(EXECUTE);
    }

    fn calculate_plan(&mut self, ectx: &mut ExecutionCtx) {
        self.wcm.calculate_plan(ectx);
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
    proving_key_path: PathBuf,
    public_inputs_path: PathBuf,
) -> Box<dyn WCLibrary<Goldilocks>> {
    env_logger::builder()
        .format_timestamp(None)
        .format_level(true)
        .format_target(false)
        .filter_level(log::LevelFilter::Trace)
        .init();

    //Capture current path
    let current_path = std::env::current_dir().unwrap();
    let elf_rom = current_path.join(ZiskWC::<Goldilocks>::DEFAULT_ELF_ROM);

    Box::new(ZiskWC::new(elf_rom, proving_key_path, public_inputs_path))
}
