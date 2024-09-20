use std::{cell::OnceCell, error::Error, path::PathBuf, sync::Arc};

use pil_std_lib::{RCAirData, RangeCheckAir, Std};
use proofman::{WitnessLibrary, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx, SetupCtx, WitnessPilout};

use p3_field::PrimeField;
use p3_goldilocks::Goldilocks;
use rand::{distributions::Standard, prelude::Distribution};

use crate::{
    Pilout, RangeCheck1, RangeCheck2, RangeCheck3, RangeCheck4, SPECIFIED_RANGES_AIRGROUP_ID,
    SPECIFIED_RANGES_AIR_IDS, U_16_AIR_AIRGROUP_ID, U_16_AIR_AIR_IDS, U_8_AIR_AIRGROUP_ID,
    U_8_AIR_AIR_IDS,
};

pub struct RangeCheckWitness<F: PrimeField> {
    pub wcm: OnceCell<Arc<WitnessManager<F>>>,
    pub range_check1: OnceCell<Arc<RangeCheck1<F>>>,
    pub range_check2: OnceCell<Arc<RangeCheck2<F>>>,
    pub range_check3: OnceCell<Arc<RangeCheck3<F>>>,
    pub range_check4: OnceCell<Arc<RangeCheck4<F>>>,
    pub std_lib: OnceCell<Arc<Std<F>>>,
}

impl<F: PrimeField> Default for RangeCheckWitness<F>
where
    Standard: Distribution<F>,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<F: PrimeField> RangeCheckWitness<F>
where
    Standard: Distribution<F>,
{
    pub fn new() -> Self {
        RangeCheckWitness {
            wcm: OnceCell::new(),
            range_check1: OnceCell::new(),
            range_check2: OnceCell::new(),
            range_check3: OnceCell::new(),
            range_check4: OnceCell::new(),
            std_lib: OnceCell::new(),
        }
    }

    fn initialize(&self, pctx: Arc<ProofCtx<F>>, ectx: Arc<ExecutionCtx>, sctx: Arc<SetupCtx>) {
        let wcm = Arc::new(WitnessManager::new(pctx, ectx, sctx));

        // TODO: Ad macro data into RCAIRData: SpecifiedRanges0Trace.
        // In fact, I only need to pass the length of mul of Specified...
        // Anyways, this solution would be very very specific
        let mut rc_air_data = Vec::new();

        rc_air_data.push(RCAirData {
            air_name: RangeCheckAir::U8Air,
            airgroup_id: U_8_AIR_AIRGROUP_ID,
            air_id: U_8_AIR_AIR_IDS[0],
        });

        rc_air_data.push(RCAirData {
            air_name: RangeCheckAir::U16Air,
            airgroup_id: U_16_AIR_AIRGROUP_ID,
            air_id: U_16_AIR_AIR_IDS[0],
        });

        rc_air_data.push(RCAirData {
            air_name: RangeCheckAir::SpecifiedRanges,
            airgroup_id: SPECIFIED_RANGES_AIRGROUP_ID,
            air_id: SPECIFIED_RANGES_AIR_IDS[0],
        });

        let std_lib = Std::new(wcm.clone(), Some(rc_air_data));
        let range_check1 = RangeCheck1::new(wcm.clone(), std_lib.clone());
        let range_check2 = RangeCheck2::new(wcm.clone(), std_lib.clone());
        let range_check3 = RangeCheck3::new(wcm.clone(), std_lib.clone());
        let range_check4 = RangeCheck4::new(wcm.clone(), std_lib.clone());

        self.wcm.set(wcm);
        self.range_check1.set(range_check1);
        self.range_check2.set(range_check2);
        self.range_check3.set(range_check3);
        self.range_check4.set(range_check4);
        self.std_lib.set(std_lib);
    }
}

impl<F: PrimeField> WitnessLibrary<F> for RangeCheckWitness<F>
where
    Standard: Distribution<F>,
{
    fn start_proof(
        &mut self,
        pctx: Arc<ProofCtx<F>>,
        ectx: Arc<ExecutionCtx>,
        sctx: Arc<SetupCtx>,
    ) {
        self.initialize(pctx.clone(), ectx.clone(), sctx.clone());

        self.wcm.get().unwrap().start_proof(pctx, ectx, sctx);
    }

    fn end_proof(&mut self) {
        self.wcm.get().unwrap().end_proof();
    }

    fn execute(&self, pctx: Arc<ProofCtx<F>>, ectx: Arc<ExecutionCtx>, sctx: Arc<SetupCtx>) {
        // Execute those components that need to be executed
        self.range_check1
            .get()
            .unwrap()
            .execute(pctx.clone(), ectx.clone(), sctx.clone());
        self.range_check2
            .get()
            .unwrap()
            .execute(pctx.clone(), ectx.clone(), sctx.clone());
        self.range_check3
            .get()
            .unwrap()
            .execute(pctx.clone(), ectx.clone(), sctx.clone());
        self.range_check4.get().unwrap().execute(pctx, ectx, sctx);
    }

    fn calculate_witness(
        &mut self,
        stage: u32,
        pctx: Arc<ProofCtx<F>>,
        ectx: Arc<ExecutionCtx>,
        sctx: Arc<SetupCtx>,
    ) {
        self.wcm
            .get()
            .unwrap()
            .calculate_witness(stage, pctx, ectx, sctx);
    }

    fn pilout(&self) -> WitnessPilout {
        Pilout::pilout()
    }
}

#[no_mangle]
pub extern "Rust" fn init_library(
    _rom_path: Option<PathBuf>,
    _public_inputs_path: Option<PathBuf>,
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
