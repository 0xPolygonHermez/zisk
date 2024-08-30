use std::{fmt::Debug,sync::Arc};

use p3_field::Field;
use proofman_common::{AirInstanceCtx, ExecutionCtx, ProofCtx};
use proofman_hints::{get_hint_field, get_hint_ids_by_name, set_hint_field, set_hint_field_val};
use proofman_setup::SetupCtx;

use crate::Decider;

pub struct StdSum<F> {
    _phantom: std::marker::PhantomData<F>,
}

impl<F: Copy + Debug + Field> Decider<F> for StdSum<F> {
    fn decide(
        &self,
        stage: u32,
        air_instance_idx: usize,
        pctx: &mut ProofCtx<F>,
        ectx: &ExecutionCtx,
        sctx: &SetupCtx,
    ) {
        if stage != 2 {
            return;
        }

        let air_instances = pctx.air_instances.read().unwrap();
        let air_instance = &air_instances[air_instance_idx];

        // Look for hints in the pilout and find if there are sum-related ones
        let setup = sctx
            .get_setup(air_instance.air_group_id, air_instance.air_id)
            .expect("REASON");
        let gsum_hints = get_hint_ids_by_name(setup, "gsum_col");
        let im_hints = get_hint_ids_by_name(setup, "im_col");

        // If the gsum col is found, then start to work
        if !gsum_hints.is_empty() {
            if let Err(e) = self.calculate_witness(
                stage,
                air_instance,
                pctx,
                ectx,
                sctx,
                &gsum_hints,
                &im_hints,
            ) {
                log::error!("Failed to calculate witness: {:?}", e);
                panic!();
            }
        }
    }
}

impl<F: Copy + Debug + Field> StdSum<F> {
    const MY_NAME: &'static str = "STD Sum";

    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            _phantom: std::marker::PhantomData,
        })
    }

    fn calculate_witness(
        &self,
        stage: u32,
        air_instance: &AirInstanceCtx<F>,
        pctx: &ProofCtx<F>,
        _ectx: &ExecutionCtx,
        sctx: &SetupCtx,
        gsum_hints: &Vec<u64>,
        im_hints: &Vec<u64>,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        log::info!(
            "{} ··· Starting witness computation stage {}",
            Self::MY_NAME,
            stage
        );

        let air_group_id = air_instance.air_group_id;
        let air_id = air_instance.air_id;
        let num_rows = pctx.pilout.get_air(air_group_id, air_id).num_rows();
        let setup = sctx.get_setup(air_group_id, air_id).unwrap();

        // Populate the im columns
        for hint in im_hints {
            // HECTOR: Check the correctness of the last flag parameters
            let mut im = get_hint_field::<F>(setup, *hint as usize, "reference", true);
            let num = get_hint_field::<F>(setup, *hint as usize, "numerator", false);
            let den = get_hint_field::<F>(setup, *hint as usize, "denominator", false);

            for i in 0..num_rows {
                // TODO: We should perform the following division in batch using div_lib
                im.set(i, num.get(i) / den.get(i));
            }
            set_hint_field(setup, *hint as u64, "reference", &im);
        }

        let gsum_hint = gsum_hints[0] as usize;

        // Populate the gsum column
        let mut gsum = get_hint_field::<F>(setup, gsum_hint, "reference", true);
        let expr = get_hint_field::<F>(setup, gsum_hint, "expression", false);

        gsum.set(0, expr.get(0));
        for i in 1..num_rows {
            // TODO: We should perform the following division in batch using div_lib
            gsum.set(i, gsum.get(i - 1) + expr.get(i));
        }

        // set the computed gsum column and its associated airgroup_val
        set_hint_field(setup, gsum_hint as u64, "reference", &gsum);
        set_hint_field_val(setup, gsum_hint as u64, "result", gsum.get(num_rows - 1));

        Ok(0)
    }
}
