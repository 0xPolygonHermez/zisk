use std::fmt::Debug;

use p3_field::Field;
use pilout::pilout::Hint;
use proofman_common::{AirInstanceCtx, ExecutionCtx, ProofCtx};
use proofman_hints::{get_hint_field, set_hint_field, set_hint_field_val};
use proofman_setup::SetupCtx;

use crate::Decider;

pub struct StdSum<F> {
    _phantom: std::marker::PhantomData<F>,
}

impl<F: Copy + Debug + Field> Decider<F> for StdSum<F> {
    fn decide(
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

        // Look for hints in the pilout and find if there are sum-related ones
        let sum_hints = get_hints_by_name_and_air_id(sctx, ["gsum_col", "im_col"]);

        if !sum_hints.is_empty() {
            self.calculate_witness(stage, air_instance, pctx, ectx, sctx, &sum_hints);
        }
    }
}

impl<F: Copy + Debug + Field> StdSum<F> {
    const MY_NAME: &'static str = "STD Sum";

    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }

    fn calculate_witness(
        &self,
        stage: u32,
        air_instance: &AirInstanceCtx<F>,
        pctx: &mut ProofCtx<F>,
        ectx: &ExecutionCtx,
        sctx: &SetupCtx,
        hints: &[Hint],
    ) -> Result<u64, Box<dyn std::error::Error>> {
        log::info!(
            "{} ··· Starting witness computation stage {}",
            Self::MY_NAME,
            stage
        );

        let mut im_hints = Vec::new();
        for hint in hints {
            if hint.name == "im_col" {
                im_hints.push(hint);
            }
        }

        let mut gsum_hint = Vec::new();
        for hint in hints {
            if hint.name == "gsum_col" {
                gsum_hint.push(hint);
            }
        }

        if gsum_hint.len() > 1 {
            return Err("There should be at most one gsum hint".into());
        }

        let air_group_id = air_instance.air_group_id;
        let air_id = air_instance.air_id;
        let N = pctx.pilout.get_air(air_group_id, air_id).num_rows();
        let setup = sctx.get_setup(air_group_id, air_id).unwrap();

        // Populate the im columns
        for hint in im_hints {
            // HECTOR: Check the correctness of the last flag parameters
            let im = get_hint_field::<F>(setup, hint, "reference", false);
            let num = get_hint_field::<F>(setup, hint, "numerator", false);
            let den = get_hint_field::<F>(setup, hint, "denominator", false);

            for i in 0..N {
                // TODO: We should perform the following division in batch using div_lib
                im.set(i, num.get(i) / den.get(i));
            }
            set_hint_field(setup, hint, "reference", &im);
        }

        // Populate the gsum column
        let mut gsum = get_hint_field::<F>(setup, gsum_hint, "reference", true); // column to feed
        let expr = get_hint_field::<F>(setup, gsum_hint, "expression", false);

        gsum.set(0, expr.get(0));
        for i in 1..N {
            // TODO: We should perform the following division in batch using div_lib
            gsum.set(i, gsum.get(i - 1) + expr.get(i));
        }

        // set the computed gsum column and its associated airgroup_val
        set_hint_field(setup, gsum_hint, "reference", &gsum);
        set_hint_field_val(setup, gsum_hint, "result", gsum.get(N - 1));

        Ok(0)
    }
}
