use pilout::pilout::Hint;
use proofman_common::{AirInstanceCtx, ExecutionCtx, ProofCtx};
use proofman_setup::SetupCtx;

use crate::Decider;

pub struct StdProd;

impl Decider for StdProd {
    fn decide<F>(
        &self,
        stage: u32,
        air_instance: &AirInstanceCtx<F>,
        pctx: &mut ProofCtx<F>,
        ectx: &ExecutionCtx,
        sctx: &SetupCtx,
    ) {
        if stage != 2 {
            return;
        }

        // Look for hints in the pilout and find if there are product-related ones
        let prod_hints = get_hints_by_name_and_air_id(sctx, ["gprod_col"]);

        if !prod_hints.is_empty() {
            self.calculate_witness(stage, air_instance, pctx, ectx, sctx, &prod_hints);
        }
    }
}


impl StdProd {
    const MY_NAME: &'static str = "STD Prod";

    fn calculate_witness<F>(
        &self,
        stage: u32,
        air_instance: &AirInstanceCtx<F>,
        pctx: &mut ProofCtx<F>,
        ectx: &ExecutionCtx,
        sctx: &SetupCtx,
        hints: &[Hint],
    ) -> Result<u64, Box<dyn std::error::Error>> {
        log::info!("{} ··· Starting witness computation stage {}", Self::MY_NAME, stage);

        // For each air_instance, there is at most one product hint
        if hints.len() > 1 {
            return Err("There should be at most one gprod hint".into());
        }

        let gprod_hint = &hints[0];

        let air_group_id = air_instance.air_group_id;
        let air_id = air_instance.air_id;
        let N = pctx.pilout.get_air(air_group_id, air_id).num_rows();
        let setup = sctx.get_setup(air_group_id, air_id).unwrap();

        // Use the hint to populate the gprod column
        let mut gprod = get_hint_field::<F>(setup, gprod_hint, "reference", true, false); // column to feed
        let num = get_hint_field::<F>(setup, gprod_hint, "numerator", false, false);
        let den = get_hint_field::<F>(setup, gprod_hint, "denominator", false,  false);

        gprod.set(0, num.get(0) / den.get(0));
        for i in 1..N {
            // TODO: We should perform the following division in batch using div_lib
            gprod.set(i, gprod.get(i - 1) * (num.get(i) / den.get(i)));
        }

        // set the computed gprod column and its associated airgroup_val
        set_hint_field(setup, gprod_hint, "reference", &gprod);
        set_hint_field_val(setup, gprod_hint, "result", gprod.get(N-1));

        Ok(0)
    }
}