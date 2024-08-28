use std::fmt::Debug;

use p3_field::Field;
use proofman_common::{AirInstanceCtx, ExecutionCtx, ProofCtx};
use proofman_hints::{get_hint_field, get_hint_ids_by_name, set_hint_field, set_hint_field_val};
use proofman_setup::SetupCtx;

use crate::Decider;

pub struct StdProd<F> {
    _phantom: std::marker::PhantomData<F>,
}

impl<F: Copy + Debug + Field> Decider<F> for StdProd<F> {
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

        // Look for hints in the pilout and find if there are product-related ones
        let setup = sctx
            .get_setup(air_instance.air_group_id, air_instance.air_id)
            .expect("REASON");
        let prod_hints = get_hint_ids_by_name(setup, "gprod_col");

        if !prod_hints.is_empty() {
            if let Err(e) =
                self.calculate_witness(stage, air_instance, pctx, ectx, sctx, &prod_hints)
            {
                log::error!("Failed to calculate witness: {:?}", e);
                panic!();
            }
        }
    }
}

impl<F: Copy + Debug + Field> StdProd<F> {
    const MY_NAME: &'static str = "STD Prod";

    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }

    fn calculate_witness(
        &self,
        stage: u32,
        air_instance: &AirInstanceCtx<F>,
        pctx: &ProofCtx<F>,
        _ectx: &ExecutionCtx,
        sctx: &SetupCtx,
        hints: &Vec<u64>,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        log::info!(
            "{} ··· Starting witness computation stage {}",
            Self::MY_NAME,
            stage
        );

        let gprod_hint = hints[0] as usize;

        let air_group_id = air_instance.air_group_id;
        let air_id = air_instance.air_id;
        let nun_rows = pctx.pilout.get_air(air_group_id, air_id).num_rows();
        let setup = sctx.get_setup(air_group_id, air_id).unwrap();

        // Use the hint to populate the gprod column
        let mut gprod = get_hint_field::<F>(setup, gprod_hint, "reference", true);
        let num = get_hint_field::<F>(setup, gprod_hint, "numerator", false);
        let den = get_hint_field::<F>(setup, gprod_hint, "denominator", false);

        gprod.set(0, num.get(0) / den.get(0));
        for i in 1..nun_rows {
            // TODO: We should perform the following division in batch using div_lib
            gprod.set(i, gprod.get(i - 1) * (num.get(i) / den.get(i)));
        }

        // set the computed gprod column and its associated airgroup_val
        set_hint_field(setup, gprod_hint as u64, "reference", &gprod);
        set_hint_field_val(setup, gprod_hint as u64, "result", gprod.get(nun_rows - 1));

        Ok(0)
    }
}
