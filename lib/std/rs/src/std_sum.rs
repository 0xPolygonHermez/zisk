use std::{
    mem,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex, RwLock,
    },
};

use std::{sync::mpsc, thread};

use pilout::{pilout::{Hint, PilOut}, pilout_proxy::PilOutProxy};
use proofman::WitnessManager;
use proofman_common::{AirInstance, ExecutionCtx, ProofCtx};
use rayon::Scope;
use witness_helpers::WitnessComponent;
use crate::Decider;

// trace!(StdSumRow, StdSumCol<F> { val });

pub struct StdSum;

impl StdSum {
    fn calculate_witness<F>(
        &self,
        hints: &[Hint],
        air_instance: &AirInstance,
        pctx: &mut ProofCtx<F>,
        ectx: &ExecutionCtx,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        // log::info!("Starting witness computation stage {}", stage);

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

        if gsum_hint.len() != 1 {
            return Err("gsum_col hint must have exactly one element".into());
        } else if im_hints.len() == 0 {
            return Err("im_col hint must have at least one element".into());
        }

        // 1] Logic to populate ims columns
        let num_rows = pctx.pilout.get_air(air_instance.air_group_id, air_instance.air_id).num_rows();

        let buffer = &pctx.air_instances.read().unwrap()[0].buffer;

        for (idx, hint) in im_hints.iter().enumerate() {
            let offset = get_offset_map_c(format!("reference{}", idx), buffer);
            let dest_ref = StdSumCol::map_from_buffer(buffer, offset, num_rows);

            let buff_ref = get_hint_field_c(buffer, air_id, hint_id, "reference"); // column to feed

            // len will be 1, extension, num_rows or num_rows * extension
            let (buffer_num, type_num) = get_hint_field_c(buffer, air_id, hint_id, "numerator"); // array of field elements
            let vec_num = match type_num {
                FIELD_ELEMENT => unsafe { Vec::<F>::from_raw_parts(buffer_num, 1, 1) },
                EXTENDED_FIELD_ELEMENT => unsafe { Vec::<EF>::from_raw_parts(buffer_num, EF.degree(), EF.degree()) },
                COLUMN => unsafe { Vec::<F>::from_raw_parts(buffer_num, num_rows, num_rows) },
                EXTENDED_COLUMN => unsafe { Vec::from_raw_parts(buffer_num, num_rows*EF.degree(), num_rows*EF.degree()) },
            };

            let (buffer_den, type_den) = get_hint_field_c(buffer, air_id, hint_id, "denominator"); // array of field elements
            let vec_num = match type_num {
                FIELD_ELEMENT => unsafe { Vec::<F>::from_raw_parts(buffer_den, 1, 1) },
                EXTENDED_FIELD_ELEMENT => unsafe { Vec::<EF>::from_raw_parts(buffer_den, EF.degree(), EF.degree()) },
                COLUMN => unsafe { Vec::<F>::from_raw_parts(buffer_den, num_rows, num_rows) },
                EXTENDED_COLUMN => unsafe { Vec::from_raw_parts(buffer_den, num_rows*EF.degree(), num_rows*EF.degree()) },
            };

            for i in 0..num_rows {
                dest_ref[i] = vec_num[i] / vec_den[i];
            }
        }

        // 2] Logic to populate gsum column
        let offset = get_offset_map_c(format!("reference{}", gsum_hint), buffer);
        let dest_ref = StdSumCol::map_from_buffer(buffer, offset, num_rows);

        // let buffer_expr = Vec::<F>::with_capacity(num_rows);

        get_hint_field_c(buffer, dest_ref, hint_id, "reference"); // column to feed

        // len will be 1, extension, num_rows or num_rows * extension
        let (len, buffer_expr) = get_hint_field_c(pk, air_id, step_params, hint_id, "expression"); // array of field elements
        let vec_num = unsafe { Vec::from_raw_parts(buffer_num, len, len) };

        dest_ref[0] = buffer_expr[0];
        for i in 1..num_rows {
            dest_ref[i] = dest_ref[i-1] + buffer_expr[i];
        }

        // set airgroup value as the last element of the gsum column
        let air_group_value = get_hint_field_c(buffer, buffer_result, hint_id, "result"); // array of field elements
        ait_group_value = dest_ref[num_rows - 1]; // TODO: Set airgroup value

        Ok(0)
    }
}

impl Decider for StdSum {
    fn decide<F>(
        &self,
        stage: u32,
        pilout: &PilOutProxy,
        air_instance: &AirInstance,
        pctx: &mut ProofCtx<F>,
        ectx: &ExecutionCtx,
    ) {
        if stage != 2 {
            return;
        }
        // Look for hints in the pilout and find if there are sum-related ones
        let sum_hints = pilout.get_hints_by_name_and_air_id(air_instance.air_id, ["im_col",  "gsum_col"]);

        if !sum_hints.is_empty() {
            self.calculate_witness(&sum_hints, air_instance, pctx, ectx);
        }
    }
}
