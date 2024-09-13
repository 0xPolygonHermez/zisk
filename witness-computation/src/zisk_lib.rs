use log::debug;
use sm_binary::{BasicTableSM, BinaryBasicSM, BinaryExtensionSM, BinarySM, ExtensionTableSM};
use sm_quick_ops::QuickOpsSM;
use std::{error::Error, path::PathBuf, sync::Arc};
use zisk_pil::{
    Pilout, BINARY_AIR_IDS, BINARY_EXTENSION_AIR_IDS, BINARY_EXTENSION_SUBPROOF_ID,
    BINARY_EXTENSION_TABLE_SUBPROOF_ID, BINARY_SUBPROOF_ID, BINARY_TABLE_AIR_IDS,
    BINARY_TABLE_SUBPROOF_ID, MAIN_AIR_IDS, MAIN_SUBPROOF_ID,
};

use p3_field::AbstractField;
use p3_goldilocks::Goldilocks;
use proofman::{WitnessLibrary, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx, SetupCtx, WitnessPilout};
use proofman_util::{timer_start, timer_stop_and_log};
use sm_arith::{Arith3264SM, Arith32SM, Arith64SM, ArithSM};
use sm_main::MainSM;
use sm_mem::{MemAlignedSM, MemSM, MemUnalignedSM};

pub struct ZiskWitness<F> {
    pub public_inputs_path: PathBuf,
    pub wcm: WitnessManager<F>,
    // State machines
    pub arith_sm: Arc<ArithSM>,
    pub arith_32_sm: Arc<Arith32SM>,
    pub arith_64_sm: Arc<Arith64SM>,
    pub arith_3264_sm: Arc<Arith3264SM>,
    pub binary_sm: Arc<BinarySM>,
    pub binary_basic_sm: Arc<BinaryBasicSM>,
    pub basic_table_sm: Arc<BasicTableSM>,
    pub binary_extension_sm: Arc<BinaryExtensionSM>,
    pub extension_table_sm: Arc<ExtensionTableSM>,
    pub main_sm: Arc<MainSM<F>>,
    pub mem_sm: Arc<MemSM>,
    pub mem_aligned_sm: Arc<MemAlignedSM>,
    pub mem_unaligned_sm: Arc<MemUnalignedSM>,
    pub quickops_sm: Arc<QuickOpsSM>,
}

impl<F: AbstractField + Copy + Send + Sync + 'static> ZiskWitness<F> {
    const MY_NAME: &'static str = "ZiskLib ";

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

        let mut wcm = WitnessManager::new();

        // TODO REMOVE THIS WHEN READY IN ZISK_PIL
        pub const MEM_SUBPROOF_ID: usize = 100;
        pub const MEM_ALIGN_AIR_IDS: &[usize] = &[1];
        pub const MEM_UNALIGNED_AIR_IDS: &[usize] = &[2, 3];
        pub const ARITH_SUBPROOF_ID: usize = 101;
        pub const ARITH32_AIR_IDS: &[usize] = &[4, 5];
        pub const ARITH64_AIR_IDS: &[usize] = &[6];
        pub const ARITH3264_AIR_IDS: &[usize] = &[7];
        pub const QUICKOPS_SUBPROOF_ID: usize = 103;
        pub const QUICKOPS_AIR_IDS: &[usize] = &[10];

        let mem_aligned_sm = MemAlignedSM::new(&mut wcm, MEM_SUBPROOF_ID, MEM_ALIGN_AIR_IDS);
        let mem_unaligned_sm =
            MemUnalignedSM::new(&mut wcm, MEM_SUBPROOF_ID, MEM_UNALIGNED_AIR_IDS);
        let mem_sm = MemSM::new(&mut wcm, mem_aligned_sm.clone(), mem_unaligned_sm.clone());

        let basic_table_sm =
            BasicTableSM::new(&mut wcm, BINARY_TABLE_SUBPROOF_ID[0], BINARY_TABLE_AIR_IDS);
        let binary_basic_sm = BinaryBasicSM::new(
            &mut wcm,
            basic_table_sm.clone(),
            BINARY_SUBPROOF_ID[0],
            BINARY_AIR_IDS,
        );
        let extension_table_sm = ExtensionTableSM::new(
            &mut wcm,
            BINARY_EXTENSION_TABLE_SUBPROOF_ID[0],
            BINARY_EXTENSION_AIR_IDS,
        );
        let binary_extension_sm = BinaryExtensionSM::new(
            &mut wcm,
            extension_table_sm.clone(),
            BINARY_EXTENSION_SUBPROOF_ID[0],
            BINARY_EXTENSION_AIR_IDS,
        );
        let binary_sm =
            BinarySM::new(&mut wcm, binary_basic_sm.clone(), binary_extension_sm.clone());

        let arith_32_sm = Arith32SM::new(&mut wcm, ARITH_SUBPROOF_ID, ARITH32_AIR_IDS);
        let arith_64_sm = Arith64SM::new(&mut wcm, ARITH_SUBPROOF_ID, ARITH64_AIR_IDS);
        let arith_3264_sm = Arith3264SM::new(&mut wcm, ARITH_SUBPROOF_ID, ARITH3264_AIR_IDS);
        let arith_sm =
            ArithSM::new(&mut wcm, arith_32_sm.clone(), arith_64_sm.clone(), arith_3264_sm.clone());

        let quickops_sm = QuickOpsSM::new(&mut wcm, QUICKOPS_SUBPROOF_ID, QUICKOPS_AIR_IDS);

        let main_sm = MainSM::new(
            &rom_path,
            &mut wcm,
            mem_sm.clone(),
            binary_sm.clone(),
            arith_sm.clone(),
            MAIN_SUBPROOF_ID[0],
            MAIN_AIR_IDS,
        );

        Ok(ZiskWitness {
            public_inputs_path,
            wcm,
            arith_sm,
            arith_32_sm,
            arith_64_sm,
            arith_3264_sm,
            binary_sm,
            binary_basic_sm,
            basic_table_sm,
            binary_extension_sm,
            extension_table_sm,
            main_sm,
            mem_sm,
            mem_aligned_sm,
            mem_unaligned_sm,
            quickops_sm,
        })
    }
}

impl<F: AbstractField + Copy + Send + Sync + 'static> WitnessLibrary<F> for ZiskWitness<F> {
    fn start_proof(&mut self, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx, sctx: &SetupCtx) {
        self.wcm.start_proof(pctx, ectx, sctx);
    }

    fn end_proof(&mut self) {
        self.wcm.end_proof();
    }
    fn execute(&self, pctx: &mut ProofCtx<F>, ectx: &mut ExecutionCtx, sctx: &SetupCtx) {
        log::info!("{}: --> Executing proof", Self::MY_NAME);

        timer_start!(EXECUTE);
        // TODO let mut ectx = self.wcm.createExecutionContext(wneeds);
        // TODO Create the pool of threads to execute the state machines here?
        // elf, inputs i trace_steps
        self.main_sm.execute(&self.public_inputs_path, pctx, ectx, sctx);
        // TODO ectx.terminate();
        timer_stop_and_log!(EXECUTE);
    }

    fn calculate_witness(
        &mut self,
        stage: u32,
        pctx: &mut ProofCtx<F>,
        ectx: &ExecutionCtx,
        sctx: &SetupCtx,
    ) {
        self.wcm.calculate_witness(stage, pctx, ectx, sctx);
    }

    fn debug(&mut self, _pctx: &ProofCtx<F>, _ectx: &ExecutionCtx, _sctx: &SetupCtx) {
        // let mut air_instances = pctx.air_instances.write().unwrap();

        // for (_air_instance_id, air_instance_ctx) in air_instances.iter_mut().enumerate() {
        //     _ = print_by_name(sctx, air_instance_ctx, "Main.a_src_imm", None, 51, 52);
        //     _ = print_by_name(sctx, air_instance_ctx, "Main.a", Some(vec![0]), 51, 52);
        //     _ = print_by_name(sctx, air_instance_ctx, "Main.a", Some(vec![1]), 51, 52);
        //     _ = print_by_name(sctx, air_instance_ctx, "a_use_sp_imm1", None, 51, 52);
        // }
    }
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
