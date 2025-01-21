use p3_field::Field;
use crate::{HintCol, HintFieldInfoC, HintFieldInfo, HintFieldOutput, HintFieldValue, HintFieldValues, HintFieldValuesVec};
use proofman_starks_lib_c::{
    get_hint_field_global_constraints_values_c, get_hint_field_global_constraints_sizes_c,
    get_hint_field_global_constraints_c, set_hint_field_global_constraints_c,
};
use std::ffi::c_void;

use std::{collections::HashMap, sync::Arc};

use proofman_common::{ExtensionField, ProofCtx, SetupCtx};

pub fn aggregate_airgroupvals<F: Field>(pctx: Arc<ProofCtx<F>>) -> Vec<Vec<u64>> {
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

    for (_, air_instance) in pctx.air_instance_repo.air_instances.write().unwrap().iter() {
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

    let mut airgroupvalues_u64: Vec<Vec<u64>> = Vec::new();
    for (id, agg_types) in pctx.global_info.agg_types.iter().enumerate() {
        let mut values = vec![0; agg_types.len() * FIELD_EXTENSION];
        for idx in 0..agg_types.len() {
            values[idx * FIELD_EXTENSION] =
                airgroupvalues[id][idx * FIELD_EXTENSION].to_string().parse::<u64>().unwrap();
            values[idx * FIELD_EXTENSION + 1] =
                airgroupvalues[id][idx * FIELD_EXTENSION + 1].to_string().parse::<u64>().unwrap();
            values[idx * FIELD_EXTENSION + 2] =
                airgroupvalues[id][idx * FIELD_EXTENSION + 2].to_string().parse::<u64>().unwrap();
        }
        airgroupvalues_u64.push(values);
    }

    airgroupvalues_u64
}

fn get_global_hint_f<F: Field>(
    pctx: Option<Arc<ProofCtx<F>>>,
    sctx: Arc<SetupCtx>,
    hint_id: u64,
    hint_field_name: &str,
    print_expression: bool,
) -> Vec<HintFieldInfo<F>> {
    let n_hints_values = get_hint_field_global_constraints_values_c(sctx.get_global_bin(), hint_id, hint_field_name);

    let mut hint_field_values: Vec<HintFieldInfo<F>> = vec![HintFieldInfo::default(); n_hints_values as usize];

    let mut hint_field_values_c = HintFieldInfoC::<F>::from_hint_field_info_vec(&mut hint_field_values);
    let mut hint_field_values_c_ptr = hint_field_values_c.as_mut_ptr() as *mut c_void;

    get_hint_field_global_constraints_sizes_c(
        sctx.get_global_info_file().as_str(),
        sctx.get_global_bin(),
        hint_field_values_c_ptr,
        hint_id,
        hint_field_name,
        print_expression,
    );

    HintFieldInfoC::<F>::sync_to_hint_field_info(&mut hint_field_values, &hint_field_values_c);

    for hint_field_value in hint_field_values.iter_mut() {
        hint_field_value.init_buffers(true);
    }

    hint_field_values_c = HintFieldInfoC::<F>::from_hint_field_info_vec(&mut hint_field_values);
    hint_field_values_c_ptr = hint_field_values_c.as_mut_ptr() as *mut c_void;

    let publics = if let Some(ref pctx) = pctx { pctx.get_publics_ptr() } else { std::ptr::null_mut() };
    let challenges = if let Some(ref pctx) = pctx { pctx.get_challenges_ptr() } else { std::ptr::null_mut() };
    let proof_values = if let Some(ref pctx) = pctx { pctx.get_proof_values_ptr() } else { std::ptr::null_mut() };
    let airgroup_values = if let Some(ref pctx) = pctx {
        let mut airgroupvals = aggregate_airgroupvals(pctx.clone());
        let mut airgroup_values_ptrs: Vec<*mut u64> = airgroupvals
            .iter_mut() // Iterate mutably over the inner Vecs
            .map(|inner_vec| inner_vec.as_mut_ptr()) // Get a raw pointer to each inner Vec
            .collect();
        airgroup_values_ptrs.as_mut_ptr() as *mut *mut u8
    } else {
        std::ptr::null_mut()
    };

    get_hint_field_global_constraints_c(
        sctx.get_global_info_file().as_str(),
        sctx.get_global_bin(),
        hint_field_values_c_ptr,
        publics,
        challenges,
        proof_values,
        airgroup_values,
        hint_id,
        hint_field_name,
        print_expression,
    );

    hint_field_values
}
pub fn get_hint_field_constant_gc<F: Field>(
    sctx: Arc<SetupCtx>,
    hint_id: u64,
    hint_field_name: &str,
    print_expression: bool,
) -> HintFieldValue<F> {
    let hint_info = get_global_hint_f(None, sctx, hint_id, hint_field_name, print_expression);

    if hint_info[0].matrix_size != 0 {
        panic!("get_hint_field can only be called with single expressions, but {} is an array", hint_field_name);
    }

    if print_expression {
        log::info!("HintsInf: {}", std::str::from_utf8(&hint_info[0].expression_line).unwrap());
    }

    HintCol::from_hint_field(&hint_info[0])
}

pub fn get_hint_field_gc_constant_a<F: Field>(
    sctx: Arc<SetupCtx>,
    hint_id: u64,
    hint_field_name: &str,
    print_expression: bool,
) -> HintFieldValuesVec<F> {
    let hint_infos: Vec<HintFieldInfo<F>> = get_global_hint_f(None, sctx, hint_id, hint_field_name, print_expression);

    let mut hint_field_values = Vec::new();
    for (v, hint_info) in hint_infos.iter().enumerate() {
        if v == 0 && hint_info.matrix_size != 1 {
            panic!("get_hint_field_m can only be called with an array of expressions!");
        }
        if print_expression {
            log::info!("HintsInf: {}", std::str::from_utf8(&hint_info.expression_line).unwrap());
        }
        let hint_value = HintCol::from_hint_field(hint_info);
        hint_field_values.push(hint_value);
    }

    HintFieldValuesVec { values: hint_field_values }
}

pub fn get_hint_field_constant_gc_m<F: Field>(
    sctx: Arc<SetupCtx>,
    hint_id: u64,
    hint_field_name: &str,
    print_expression: bool,
) -> HintFieldValues<F> {
    let hint_infos = get_global_hint_f(None, sctx, hint_id, hint_field_name, print_expression);

    let mut hint_field_values = HashMap::with_capacity(hint_infos.len() as usize);

    for (v, hint_info) in hint_infos.iter().enumerate() {
        if v == 0 && hint_info.matrix_size > 2 {
            panic!("get_hint_field_m can only be called with a matrix of expressions!",);
        }
        let hint_value = HintCol::from_hint_field(hint_info);
        let mut pos = Vec::new();
        for p in 0..hint_info.matrix_size {
            pos.push(hint_info.pos[p as usize]);
        }
        if print_expression {
            log::info!("HintsInf: {}", std::str::from_utf8(&hint_info.expression_line).unwrap());
        }
        hint_field_values.insert(pos, hint_value);
    }

    HintFieldValues { values: hint_field_values }
}

pub fn get_hint_field_gc<F: Field>(
    pctx: Arc<ProofCtx<F>>,
    sctx: Arc<SetupCtx>,
    hint_id: u64,
    hint_field_name: &str,
    print_expression: bool,
) -> HintFieldValue<F> {
    let hint_info = get_global_hint_f(Some(pctx), sctx, hint_id, hint_field_name, print_expression);

    if hint_info[0].matrix_size != 0 {
        panic!("get_hint_field can only be called with single expressions, but {} is an array", hint_field_name);
    }

    if print_expression {
        log::info!("HintsInf: {}", std::str::from_utf8(&hint_info[0].expression_line).unwrap());
    }

    HintCol::from_hint_field(&hint_info[0])
}

pub fn get_hint_field_gc_a<F: Field>(
    pctx: Arc<ProofCtx<F>>,
    sctx: Arc<SetupCtx>,
    hint_id: u64,
    hint_field_name: &str,
    print_expression: bool,
) -> HintFieldValuesVec<F> {
    let hint_infos: Vec<HintFieldInfo<F>> =
        get_global_hint_f(Some(pctx), sctx, hint_id, hint_field_name, print_expression);

    let mut hint_field_values = Vec::new();
    for (v, hint_info) in hint_infos.iter().enumerate() {
        if v == 0 && hint_info.matrix_size != 1 {
            panic!("get_hint_field_m can only be called with an array of expressions!");
        }
        if print_expression {
            log::info!("HintsInf: {}", std::str::from_utf8(&hint_info.expression_line).unwrap());
        }
        let hint_value = HintCol::from_hint_field(hint_info);
        hint_field_values.push(hint_value);
    }
    HintFieldValuesVec { values: hint_field_values }
}

pub fn get_hint_field_gc_m<F: Field>(
    pctx: Arc<ProofCtx<F>>,
    sctx: Arc<SetupCtx>,
    hint_id: u64,
    hint_field_name: &str,
    print_expression: bool,
) -> HintFieldValues<F> {
    let hint_infos = get_global_hint_f(Some(pctx), sctx, hint_id, hint_field_name, print_expression);

    let mut hint_field_values = HashMap::with_capacity(hint_infos.len() as usize);

    for (v, hint_info) in hint_infos.iter().enumerate() {
        if v == 0 && hint_info.matrix_size > 2 {
            panic!("get_hint_field_m can only be called with a matrix of expressions!",);
        }
        let hint_value = HintCol::from_hint_field(hint_info);
        let mut pos = Vec::new();
        for p in 0..hint_info.matrix_size {
            pos.push(hint_info.pos[p as usize]);
        }
        if print_expression {
            log::info!("HintsInf: {}", std::str::from_utf8(&hint_info.expression_line).unwrap());
        }
        hint_field_values.insert(pos, hint_value);
    }

    HintFieldValues { values: hint_field_values }
}

pub fn set_hint_field_gc<F: Field>(
    pctx: Arc<ProofCtx<F>>,
    sctx: Arc<SetupCtx>,
    hint_id: u64,
    hint_field_name: &str,
    value: HintFieldOutput<F>,
) {
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

    set_hint_field_global_constraints_c(
        sctx.get_global_info_file().as_str(),
        sctx.get_global_bin(),
        pctx.get_proof_values_ptr(),
        value_array.as_ptr() as *mut u8,
        hint_id,
        hint_field_name,
    );
}
