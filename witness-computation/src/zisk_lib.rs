use log::trace;
use pil_std_lib::Std;
use sm_binary::{
    BinaryBasicSM, BinaryBasicTableSM, BinaryExtensionSM, BinaryExtensionTableSM, BinarySM,
};
use sm_quick_ops::QuickOpsSM;
use std::{error::Error, path::PathBuf, sync::Arc};
use zisk_pil::*;

use p3_field::PrimeField;
use p3_goldilocks::Goldilocks;
use proofman::{WitnessLibrary, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx, SetupCtx, WitnessPilout};
use proofman_util::{timer_start, timer_stop_and_log};
use sm_arith::ArithSM;
use sm_main::MainSM;
use sm_mem::{MemAlignedSM, MemSM, MemUnalignedSM};

pub struct ZiskWitness<F: PrimeField> {
    pub public_inputs_path: PathBuf,
    pub rom_path: PathBuf,

    // Witness computation manager
    pub wcm: Option<Arc<WitnessManager<F>>>,

    // State machines
    pub arith_sm: Option<Arc<ArithSM<F>>>,
    pub binary_sm: Option<Arc<BinarySM<F>>>,
    pub binary_basic_sm: Option<Arc<BinaryBasicSM<F>>>,
    pub binary_extension_sm: Option<Arc<BinaryExtensionSM<F>>>,
    pub main_sm: Option<Arc<MainSM<F>>>,
    pub mem_sm: Option<Arc<MemSM>>,
    pub mem_aligned_sm: Option<Arc<MemAlignedSM>>,
    pub mem_unaligned_sm: Option<Arc<MemUnalignedSM>>,
    pub quickops_sm: Option<Arc<QuickOpsSM>>,
}

impl<F: PrimeField + Copy + Send + Sync + 'static> ZiskWitness<F> {
    pub fn new(rom_path: PathBuf, public_inputs_path: PathBuf) -> Result<Self, Box<dyn Error>> {
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

        Ok(ZiskWitness {
            public_inputs_path,
            rom_path,
            wcm: None,
            arith_sm: None,
            binary_sm: None,
            binary_basic_sm: None,
            binary_extension_sm: None,
            main_sm: None,
            mem_sm: None,
            mem_aligned_sm: None,
            mem_unaligned_sm: None,
            quickops_sm: None,
        })
    }

    fn initialize(&mut self, pctx: Arc<ProofCtx<F>>, ectx: Arc<ExecutionCtx>, sctx: Arc<SetupCtx>) {
        let wcm = WitnessManager::new(pctx, ectx, sctx);
        let wcm = Arc::new(wcm);

        // TODO REMOVE THIS WHEN READY IN ZISK_PIL
        pub const MEM_AIRGROUP_ID: usize = 100;
        pub const MEM_ALIGN_AIR_IDS: &[usize] = &[1];
        pub const MEM_UNALIGNED_AIR_IDS: &[usize] = &[2, 3];
        // pub const ARITH_AIRGROUP_ID: usize = 101;

        // pub const ARITH_32_AIR_IDS: &[usize] = &[4, 5];
        // pub const ARITH_MUL_64_AIR_IDS: &[usize] = &[6];
        // pub const ARITH_MUL_32_AIR_IDS: &[usize] = &[7];
        // pub const ARITH_FULL_AIR_IDS: &[usize] = &[8];
        // pub const ARITH_TABLE_AIR_IDS: &[usize] = &[9];
        // pub const ARITH_RANGE_TABLE_AIR_IDS: &[usize] = &[11];

        pub const QUICKOPS_AIRGROUP_ID: usize = 102;
        pub const QUICKOPS_AIR_IDS: &[usize] = &[10];

        let mem_aligned_sm = MemAlignedSM::new(wcm.clone(), MEM_AIRGROUP_ID, MEM_ALIGN_AIR_IDS);
        let mem_unaligned_sm =
            MemUnalignedSM::new(wcm.clone(), MEM_AIRGROUP_ID, MEM_UNALIGNED_AIR_IDS);
        let mem_sm = MemSM::new(wcm.clone(), mem_aligned_sm.clone(), mem_unaligned_sm.clone());

        let binary_basic_table_sm =
            BinaryBasicTableSM::new(wcm.clone(), BINARY_TABLE_AIRGROUP_ID, BINARY_TABLE_AIR_IDS);
        let binary_basic_sm = BinaryBasicSM::new(
            wcm.clone(),
            binary_basic_table_sm,
            BINARY_AIRGROUP_ID,
            BINARY_AIR_IDS,
        );

        let binary_extension_table_sm = BinaryExtensionTableSM::new(
            wcm.clone(),
            BINARY_EXTENSION_TABLE_AIRGROUP_ID,
            BINARY_EXTENSION_TABLE_AIR_IDS,
        );
        let binary_extension_sm = BinaryExtensionSM::new(
            wcm.clone(),
            binary_extension_table_sm,
            BINARY_EXTENSION_AIRGROUP_ID,
            BINARY_EXTENSION_AIR_IDS,
        );
        let binary_sm =
            BinarySM::new(wcm.clone(), binary_basic_sm.clone(), binary_extension_sm.clone());

        let arith_sm = ArithSM::new(wcm.clone());

        let quickops_sm = QuickOpsSM::new(wcm.clone(), QUICKOPS_AIRGROUP_ID, QUICKOPS_AIR_IDS);

        let main_sm = MainSM::new(
            &self.rom_path,
            &wcm,
            mem_sm.clone(),
            binary_sm.clone(),
            arith_sm.clone(),
            MAIN_AIRGROUP_ID,
            MAIN_AIR_IDS,
        );

        _ = Std::new(wcm.clone(), None);

        self.wcm = Some(wcm);
        self.arith_sm = Some(arith_sm);
        self.binary_sm = Some(binary_sm);
        self.binary_basic_sm = Some(binary_basic_sm);
        self.binary_extension_sm = Some(binary_extension_sm);
        self.main_sm = Some(main_sm);
        self.mem_sm = Some(mem_sm);
        self.mem_aligned_sm = Some(mem_aligned_sm);
        self.mem_unaligned_sm = Some(mem_unaligned_sm);
        self.quickops_sm = Some(quickops_sm);
    }
}

impl<F: PrimeField + Copy + Send + Sync + 'static> WitnessLibrary<F> for ZiskWitness<F> {
    fn start_proof(
        &mut self,
        pctx: Arc<ProofCtx<F>>,
        ectx: Arc<ExecutionCtx>,
        sctx: Arc<SetupCtx>,
    ) {
        self.initialize(pctx.clone(), ectx.clone(), sctx.clone());

        self.wcm.as_ref().unwrap().start_proof(pctx, ectx, sctx);
    }

    fn end_proof(&mut self) {
        self.wcm.as_ref().unwrap().end_proof();
    }
    fn execute(&self, pctx: Arc<ProofCtx<F>>, ectx: Arc<ExecutionCtx>, sctx: Arc<SetupCtx>) {
        timer_start!(EXECUTE);
        // TODO let mut ectx = self.wcm.createExecutionContext(wneeds);
        // TODO Create the pool of threads to execute the state machines here?
        // elf, inputs i trace_steps
        self.main_sm.as_ref().unwrap().execute(&self.public_inputs_path, pctx, ectx, sctx);
        // TODO ectx.terminate();
        timer_stop_and_log!(EXECUTE);
    }

    fn calculate_witness(
        &mut self,
        stage: u32,
        pctx: Arc<ProofCtx<F>>,
        ectx: Arc<ExecutionCtx>,
        sctx: Arc<SetupCtx>,
    ) {
        self.wcm.as_ref().unwrap().calculate_witness(stage, pctx, ectx, sctx);
    }

    fn debug(&mut self, _pctx: Arc<ProofCtx<F>>, _ectx: Arc<ExecutionCtx>, _sctx: Arc<SetupCtx>) {}
    fn pilout(&self) -> WitnessPilout {
        Pilout::pilout()
    }
}

#[no_mangle]
pub extern "Rust" fn init_library(
    rom_path: Option<PathBuf>,
    public_inputs_path: PathBuf,
) -> Result<Box<dyn WitnessLibrary<Goldilocks>>, Box<dyn Error>> {
    env_logger::builder()
        .format_timestamp(None)
        .format_level(true)
        .format_target(false)
        .filter_level(log::LevelFilter::Trace)
        .init();

    let rom_path = rom_path.ok_or("ROM path is required")?;

    let zisk_witness = ZiskWitness::new(rom_path, public_inputs_path)?;
    Ok(Box::new(zisk_witness))
}
