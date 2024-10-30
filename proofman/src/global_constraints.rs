use p3_field::Field;
use proofman_hints::{HintCol, HintFieldInfoValues, HintFieldOutput, HintFieldValue, HintFieldValues, HintFieldValuesVec};
use proofman_starks_lib_c::{
    get_hint_field_global_constraints_c, set_hint_field_global_constraints_c, verify_global_constraints_c,
};

use std::{collections::HashMap, sync::Arc};

use proofman_common::{ExtensionField, ProofCtx, SetupCtx};

use colored::*;

use std::os::raw::c_void;

pub fn aggregate_airgroupvals<F: Field>(pctx: Arc<ProofCtx<F>>) -> Vec<Vec<F>> {
    const FIELD_EXTENSION: usize = 3;

    let mut airgroupvalues: Vec<Vec<F>> = Vec::new();
    for agg_types in pctx.global_info.agg_types.iter() {
        let mut values = vec![F::zero(); agg_types.len() * FIELD_EXTENSION];
        for (idx, agg_type) in agg_types.iter().enumerate() {
            if agg_type.agg_type == 1 {
                values[idx * FIELD_EXTENSION] = F::one();
            }
        }
        airgroupvalues.push(values);
    }

    for air_instance in pctx.air_instance_repo.air_instances.write().unwrap().iter() {
        for (idx, agg_type) in pctx.global_info.agg_types[air_instance.airgroup_id].iter().enumerate() {
            let mut acc = ExtensionField {
                value: [
                    airgroupvalues[air_instance.airgroup_id][idx * FIELD_EXTENSION],
                    airgroupvalues[air_instance.airgroup_id][idx * FIELD_EXTENSION + 1],
                    airgroupvalues[air_instance.airgroup_id][idx * FIELD_EXTENSION + 2],
                ],
            };
            if !air_instance.airgroup_values.is_empty() {
                let instance_airgroup_val = ExtensionField {
                    value: [
                        air_instance.airgroup_values[idx * FIELD_EXTENSION],
                        air_instance.airgroup_values[idx * FIELD_EXTENSION + 1],
                        air_instance.airgroup_values[idx * FIELD_EXTENSION + 2],
                    ],
                };
                if agg_type.agg_type == 0 {
                    acc += instance_airgroup_val;
                } else {
                    acc *= instance_airgroup_val;
                }
                airgroupvalues[air_instance.airgroup_id][idx * FIELD_EXTENSION] = acc.value[0];
                airgroupvalues[air_instance.airgroup_id][idx * FIELD_EXTENSION + 1] = acc.value[1];
                airgroupvalues[air_instance.airgroup_id][idx * FIELD_EXTENSION + 2] = acc.value[2];
            }
        }
    }

    airgroupvalues
}

pub fn verify_global_constraints_proof<F: Field>(pctx: Arc<ProofCtx<F>>, sctx: Arc<SetupCtx>) -> bool {
    const MY_NAME: &str = "GlCstVfy";

    log::info!("{}: --> Checking global constraints", MY_NAME);

    let public_inputs_guard = pctx.public_inputs.inputs.read().unwrap();
    let public_inputs = (*public_inputs_guard).as_ptr() as *mut c_void;

    let proof_values_guard = pctx.proof_values.values.read().unwrap();
    let proof_values = (*proof_values_guard).as_ptr() as *mut c_void;

    let mut airgroupvalues = aggregate_airgroupvals(pctx.clone());

    let mut airgroup_values_ptrs: Vec<*mut F> = airgroupvalues
        .iter_mut() // Iterate mutably over the inner Vecs
        .map(|inner_vec| inner_vec.as_mut_ptr()) // Get a raw pointer to each inner Vec
        .collect();

    let global_constraints_verified = verify_global_constraints_c(
        sctx.get_global_bin(),
        public_inputs,
        proof_values,
        airgroup_values_ptrs.as_mut_ptr() as *mut *mut c_void,
    );

    log::info!("{}: <-- Checking global constraints", MY_NAME);

    if global_constraints_verified {
        log::info!(
            "{}: ··· {}",
            MY_NAME,
            "\u{2713} All global constraints were successfully verified".bright_green().bold()
        );
    } else {
        log::info!("{}: ··· {}", MY_NAME, "\u{2717} Not all global constraints were verified".bright_red().bold());
    }

    global_constraints_verified
}

pub fn get_hint_field_gc<F: Field>(
    pctx: Arc<ProofCtx<F>>,
    sctx: Arc<SetupCtx>,
    hint_id: u64,
    hint_field_name: &str,
    print_expression: bool,
) -> HintFieldValue<F> {
    let public_inputs_ptr = pctx.public_inputs.inputs.read().unwrap().as_ptr() as *mut c_void;

    let mut airgroupvalues = aggregate_airgroupvals(pctx.clone());
    let mut airgroup_values_ptrs: Vec<*mut F> = airgroupvalues
        .iter_mut() // Iterate mutably over the inner Vecs
        .map(|inner_vec| inner_vec.as_mut_ptr()) // Get a raw pointer to each inner Vec
        .collect();

    let proof_values_guard = pctx.proof_values.values.read().unwrap();
    let proof_values = (*proof_values_guard).as_ptr() as *mut c_void;

    let raw_ptr = get_hint_field_global_constraints_c(
        sctx.get_global_bin(),
        public_inputs_ptr,
        proof_values,
        airgroup_values_ptrs.as_mut_ptr() as *mut *mut c_void,
        hint_id,
        hint_field_name,
        print_expression,
    );

    unsafe {
        let hint_field_values = &*(raw_ptr as *mut HintFieldInfoValues<F>);
        let value = &*(hint_field_values.hint_field_values.add(0));
        if value.matrix_size != 0 {
            panic!("get_hint_field can only be called with single expressions, but {} is an array", hint_field_name);
        }
        HintCol::from_hint_field(value)
    }
}

pub fn get_hint_field_gc_a<F: Field>(
    pctx: Arc<ProofCtx<F>>,
    sctx: Arc<SetupCtx>,
    hint_id: u64,
    hint_field_name: &str,
    print_expression: bool,
) -> HintFieldValuesVec<F> {
    let public_inputs_ptr = pctx.public_inputs.inputs.read().unwrap().as_ptr() as *mut c_void;

    let mut airgroupvalues = aggregate_airgroupvals(pctx.clone());
    let mut airgroup_values_ptrs: Vec<*mut F> = airgroupvalues
        .iter_mut() // Iterate mutably over the inner Vecs
        .map(|inner_vec| inner_vec.as_mut_ptr()) // Get a raw pointer to each inner Vec
        .collect();

    let proof_values_guard = pctx.proof_values.values.read().unwrap();
    let proof_values = (*proof_values_guard).as_ptr() as *mut c_void;

    let raw_ptr = get_hint_field_global_constraints_c(
        sctx.get_global_bin(),
        public_inputs_ptr,
        proof_values,
        airgroup_values_ptrs.as_mut_ptr() as *mut *mut c_void,
        hint_id,
        hint_field_name,
        print_expression,
    );

    unsafe {
        let mut hint_field_values = Vec::new();
        let hint_field = &*(raw_ptr as *mut HintFieldInfoValues<F>);
        for v in 0..hint_field.n_values {
            let h = &*(hint_field.hint_field_values.add(v as usize));
            if v == 0 && h.matrix_size != 1 {
                panic!("get_hint_field_m can only be called with an array of expressions!");
            }
            let hint_value = HintCol::from_hint_field(h);
            hint_field_values.push(hint_value);
        }

        HintFieldValuesVec { values: hint_field_values }
    }
}

pub fn get_hint_field_gc_m<F: Field>(
    pctx: Arc<ProofCtx<F>>,
    sctx: Arc<SetupCtx>,
    hint_id: u64,
    hint_field_name: &str,
    print_expression: bool,
) -> HintFieldValues<F> {
    let public_inputs_ptr = pctx.public_inputs.inputs.read().unwrap().as_ptr() as *mut c_void;

    let mut airgroupvalues = aggregate_airgroupvals(pctx.clone());
    let mut airgroup_values_ptrs: Vec<*mut F> = airgroupvalues
        .iter_mut() // Iterate mutably over the inner Vecs
        .map(|inner_vec| inner_vec.as_mut_ptr()) // Get a raw pointer to each inner Vec
        .collect();

    let proof_values_guard = pctx.proof_values.values.read().unwrap();
    let proof_values = (*proof_values_guard).as_ptr() as *mut c_void;

    let raw_ptr = get_hint_field_global_constraints_c(
        sctx.get_global_bin(),
        public_inputs_ptr,
        proof_values,
        airgroup_values_ptrs.as_mut_ptr() as *mut *mut c_void,
        hint_id,
        hint_field_name,
        print_expression,
    );

    unsafe {
        let hint_field = &*(raw_ptr as *mut HintFieldInfoValues<F>);
        let mut hint_field_values = HashMap::with_capacity(hint_field.n_values as usize);

        for v in 0..hint_field.n_values {
            let h = &*(hint_field.hint_field_values.add(v as usize));
            if v == 0 && h.matrix_size > 2 {
                panic!("get_hint_field_m can only be called with a matrix of expressions!",);
            }
            let hint_value = HintCol::from_hint_field(h);
            let mut pos = Vec::new();
            for p in 0..h.matrix_size {
                pos.push(h.pos.wrapping_add(p as usize) as u64);
            }
            hint_field_values.insert(pos, hint_value);
        }

        HintFieldValues { values: hint_field_values }
    }
}

pub fn set_hint_field<F: Field>(
    pctx: Arc<ProofCtx<F>>,
    sctx: Arc<SetupCtx>,
    hint_id: u64,
    hint_field_name: &str,
    value: HintFieldOutput<F>,
) {
    let proof_values_guard = pctx.proof_values.values.read().unwrap();
    let proof_values = (*proof_values_guard).as_ptr() as *mut c_void;

    let mut value_array: Vec<F> = Vec::new();

    match value {
        HintFieldOutput::Field(val) => {
            value_array.push(val);
        }
        HintFieldOutput::FieldExtended(val) => {
            value_array.push(val.value[0]);
            value_array.push(val.value[1]);
            value_array.push(val.value[2]);
        }
    };

    let values_ptr = value_array.as_ptr() as *mut c_void;

    let id =
        set_hint_field_global_constraints_c(sctx.get_global_bin(), proof_values, values_ptr, hint_id, hint_field_name);

    pctx.set_proof_value_calculated(id as usize);
}
