use std::sync::Arc;

use num_bigint::BigInt;
use p3_field::PrimeField;

use proofman_common::{ProofCtx, SetupCtx};
use witness::WitnessManager;

use crate::{AirComponent, StdProd, StdRangeCheck, RangeCheckAir, StdSum};

pub struct Std<F: PrimeField> {
    pub pctx: Arc<ProofCtx<F>>,
    pub sctx: Arc<SetupCtx>,
    pub range_check: Arc<StdRangeCheck<F>>,
    pub std_prod: Arc<StdProd<F>>,
    pub std_sum: Arc<StdSum<F>>,
}

impl<F: PrimeField> Std<F> {
    const MY_NAME: &'static str = "STD     ";

    pub fn new(wcm: Arc<WitnessManager<F>>) -> Arc<Self> {
        let std_mode = wcm.get_pctx().options.debug_info.std_mode.clone();
        log::info!("{}: ··· The PIL2 STD library has been initialized on mode {}", Self::MY_NAME, std_mode.name);

        // Instantiate the STD components
        let std_prod = StdProd::new(wcm.get_pctx(), wcm.get_sctx(), None, None);
        let std_sum = StdSum::new(wcm.get_pctx(), wcm.get_sctx(), None, None);
        let range_check = StdRangeCheck::new(wcm.get_pctx(), wcm.get_sctx());

        Self::register_std(wcm.clone(), std_prod.clone(), std_sum.clone(), range_check.clone());

        Arc::new(Self { pctx: wcm.get_pctx(), sctx: wcm.get_sctx(), range_check, std_prod, std_sum })
    }

    pub fn register_std(
        wcm: Arc<WitnessManager<F>>,
        std_prod: Arc<StdProd<F>>,
        std_sum: Arc<StdSum<F>>,
        range_check: Arc<StdRangeCheck<F>>,
    ) {
        wcm.register_component(std_prod.clone());
        wcm.register_component(std_sum.clone());

        if range_check.u8air.is_some() {
            wcm.register_component(range_check.u8air.clone().unwrap());
        }

        if range_check.u16air.is_some() {
            wcm.register_component(range_check.u16air.clone().unwrap());
        }

        if range_check.specified_ranges.is_some() {
            wcm.register_component(range_check.specified_ranges.clone().unwrap());
        }

        wcm.register_component(range_check.clone());
    }

    // Gets the range for the range check.
    pub fn get_range(&self, min: BigInt, max: BigInt, predefined: Option<bool>) -> usize {
        self.range_check.get_range(min, max, predefined)
    }

    // Processes the inputs for the range check.
    pub fn range_check(&self, val: F, multiplicity: F, id: usize) {
        self.range_check.assign_values(val, multiplicity, id);
    }

    pub fn get_ranges(&self) -> Vec<(usize, usize, RangeCheckAir)> {
        self.range_check.get_ranges()
    }

    pub fn drain_inputs(&self, rc_type: &RangeCheckAir) {
        match rc_type {
            RangeCheckAir::U8Air => {
                self.range_check.u8air.as_ref().unwrap().drain_inputs(self.pctx.clone(), self.sctx.clone());
            }
            RangeCheckAir::U16Air => {
                self.range_check.u16air.as_ref().unwrap().drain_inputs(self.pctx.clone(), self.sctx.clone());
            }
            RangeCheckAir::SpecifiedRanges => {
                self.range_check.specified_ranges.as_ref().unwrap().drain_inputs(self.pctx.clone(), self.sctx.clone());
            }
        };
    }
}
