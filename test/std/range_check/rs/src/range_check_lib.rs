use std::{error::Error, path::PathBuf, sync::Arc};

use pil_std_lib::{RCAirData, RangeCheckAir, Std};
use proofman::{WitnessLibrary, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx, SetupCtx, WitnessPilout};

use p3_field::PrimeField;
use p3_goldilocks::Goldilocks;
use rand::{distributions::Standard, prelude::Distribution};

use crate::{
    Pilout, /*RangeCheck1,*/ RangeCheck4, 
    /*U_16_AIR_AIR_IDS, U_16_AIR_SUBPROOF_ID,*/
    U_8_AIR_AIR_IDS, U_8_AIR_SUBPROOF_ID,
};

pub struct RangeCheckWitness<F: PrimeField> {
    pub wcm: WitnessManager<F>,
    // pub range_check1: Arc<RangeCheck1<F>>,
    pub range_check4: Arc<RangeCheck4<F>>,
    pub std_lib: Arc<Std<F>>,
}

impl<F: PrimeField> RangeCheckWitness<F>
where
    Standard: Distribution<F>,
{
    pub fn new() -> Self {
        let mut wcm = WitnessManager::new();

        let mut rc_air_data = Vec::new();

        // TODO: Ad macro data into RCAIRData: SpecifiedRanges0Trace.
        // In fact, I only need to pass the length of mul of Specified...
        // Anyways, this solution would be very very specific

        rc_air_data.push(RCAirData {
            air_name: RangeCheckAir::U8Air,
            air_group_id: U_8_AIR_SUBPROOF_ID[0],
            air_id: U_8_AIR_AIR_IDS[0],
        });
        // rc_air_data.push(RCAirData {
        //     air_name: RangeCheckAir::U16Air,
        //     air_group_id: U_16_AIR_SUBPROOF_ID[0],
        //     air_id: U_16_AIR_AIR_IDS[0],
        // });

        let std_lib = Std::new(&mut wcm, Some(rc_air_data));
        // let range_check1 = RangeCheck::new(&mut wcm, std_lib.clone());
        let range_check4 = RangeCheck4::new(&mut wcm, std_lib.clone());

        RangeCheckWitness {
            wcm,
            // range_check1,
            range_check4,
            std_lib,
        }
    }
}

impl<F: PrimeField> WitnessLibrary<F> for RangeCheckWitness<F>
where
    Standard: Distribution<F>,
{
    fn start_proof(&mut self, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx, sctx: &SetupCtx) {
        self.wcm.start_proof(pctx, ectx, sctx);
    }

    fn end_proof(&mut self) {
        self.wcm.end_proof();
    }

    fn execute(&self, pctx: &mut ProofCtx<F>, ectx: &mut ExecutionCtx, sctx: &SetupCtx) {
        // Execute those components that need to be executed
        // self.range_check1.execute(pctx, ectx, sctx);
        self.range_check4.execute(pctx, ectx, sctx);
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

    fn pilout(&self) -> WitnessPilout {
        Pilout::pilout()
    }
}

#[no_mangle]
pub extern "Rust" fn init_library(
    _rom_path: Option<PathBuf>,
    _public_inputs_path: PathBuf,
) -> Result<Box<dyn WitnessLibrary<Goldilocks>>, Box<dyn Error>> {
    env_logger::builder()
        .format_timestamp(None)
        .format_level(true)
        .format_target(false)
        .filter_level(log::LevelFilter::Trace)
        .init();
    let range_check_witness = RangeCheckWitness::new();
    Ok(Box::new(range_check_witness))
}
