use libloading::{Library, Symbol};
use p3_field::Field;
use std::ffi::CString;
use std::sync::Arc;
use proofman_starks_lib_c::*;
use std::mem::MaybeUninit;
use std::path::{Path, PathBuf};

use proofman_common::{ExecutionCtx, ProofCtx, ProofType, SetupCtx};

use std::os::raw::{c_void, c_char};

use proofman_util::{timer_start_trace, timer_stop_and_log_trace};

type GetCommitedPolsFunc = unsafe extern "C" fn(
    p_address: *mut c_void,
    p_publics: *mut c_void,
    zkin: *mut c_void,
    n: u64,
    n_publics: u64,
    offset_cm1: u64,
    dat_file: *const c_char,
    exec_file: *const c_char,
);

type GenWitnessResult<F> = Result<(Vec<MaybeUninit<F>>, Vec<MaybeUninit<F>>), Box<dyn std::error::Error>>;

pub fn generate_recursion_proof<F: Field>(
    pctx: &ProofCtx<F>,
    ectx: &ExecutionCtx<F>,
    sctx: Arc<SetupCtx<F>>,
    proofs: &[*mut c_void],
    proof_type: &ProofType,
    output_dir_path: PathBuf,
    save_proof: bool,
) -> Result<Vec<*mut c_void>, Box<dyn std::error::Error>> {
    const MY_NAME: &str = "AggProof";

    //Create setup contexts
    let mut proofs_out: Vec<*mut c_void> = Vec::new();

    let global_info_path = pctx.global_info.get_proving_key_path().join("pilout.globalInfo.json");
    let global_info_file: &str = global_info_path.to_str().unwrap();

    // Run proves
    match *proof_type {
        ProofType::Compressor | ProofType::Recursive1 => {
            for (prover_idx, air_instance) in
                pctx.air_instance_repo.air_instances.write().unwrap().iter_mut().enumerate()
            {
                if *proof_type == ProofType::Compressor
                    && !pctx.global_info.get_air_has_compressor(air_instance.airgroup_id, air_instance.air_id)
                {
                    proofs_out.push(proofs[prover_idx]);
                } else {
                    let air_instance_name = &pctx.global_info.airs[air_instance.airgroup_id][air_instance.air_id].name;

                    let setup = sctx.get_setup(air_instance.airgroup_id, air_instance.air_id);
                    let p_setup: *mut c_void = (&setup.p_setup).into();
                    let p_stark_info: *mut c_void = setup.p_setup.p_stark_info;

                    let zkin = if *proof_type == ProofType::Recursive1 {
                        let recursive2_verkey = pctx
                            .global_info
                            .get_air_setup_path(air_instance.airgroup_id, air_instance.air_id, &ProofType::Recursive2)
                            .display()
                            .to_string()
                            + ".verkey.json";
                        add_recursive2_verkey_c(proofs[prover_idx], recursive2_verkey.as_str())
                    } else {
                        proofs[prover_idx]
                    };

                    let (buffer, publics) = generate_witness(
                        pctx,
                        air_instance.airgroup_id,
                        air_instance.air_id,
                        p_stark_info,
                        zkin,
                        proof_type,
                    )?;

                    let p_publics = publics.as_ptr() as *mut c_void;
                    let p_address = buffer.as_ptr() as *mut c_void;

                    timer_start_trace!(GENERATE_PROOF);

                    let proof_type_str = match proof_type {
                        ProofType::Compressor => "compressor",
                        ProofType::Recursive1 => "recursive1",
                        _ => panic!(),
                    };

                    log::info!(
                        "{}: {}",
                        MY_NAME,
                        format!(
                            "··· Generating {} proof for instance {} of {}",
                            proof_type_str,
                            air_instance.air_instance_id.unwrap(),
                            air_instance_name
                        )
                    );

                    let output_file_path = output_dir_path
                        .join(format!("proofs/{}_{}_{}.json", proof_type_str, air_instance_name, prover_idx));

                    let proof_file = match save_proof {
                        true => output_file_path.to_string_lossy().into_owned(),
                        false => String::from(""),
                    };

                    let const_pols_ptr = (*setup.const_pols.values.read().unwrap()).as_ptr() as *mut c_void;
                    let const_tree_ptr = (*setup.const_tree.values.read().unwrap()).as_ptr() as *mut c_void;

                    let p_prove = gen_recursive_proof_c(
                        p_setup,
                        p_address,
                        const_pols_ptr,
                        const_tree_ptr,
                        p_publics,
                        &proof_file,
                        global_info_file,
                        air_instance.airgroup_id as u64,
                    );
                    proofs_out.push(p_prove);

                    drop(buffer);
                    drop(publics);
                    log::info!("{}: ··· Proof generated.", MY_NAME);

                    timer_stop_and_log_trace!(GENERATE_PROOF);
                }
            }
        }
        ProofType::Recursive2 => {
            let mut dctx = ectx.dctx.write().unwrap();
            let n_airgroups = pctx.global_info.air_groups.len();
            let mut alives = Vec::with_capacity(n_airgroups);
            let mut airgroup_proofs: Vec<Vec<Option<*mut c_void>>> = Vec::with_capacity(n_airgroups);
            let mut null_zkin: Option<*mut c_void> = None;

            // Pre-process data before starting recursion loop
            for airgroup in 0..n_airgroups {
                let instances = &dctx.airgroup_instances[airgroup];
                airgroup_proofs.push(Vec::with_capacity(instances.len().max(1)));
                if !instances.is_empty() {
                    for instance in instances.iter() {
                        let local_instance = dctx.glob2loc[*instance];
                        let proof = local_instance.map(|idx| proofs[idx]);
                        airgroup_proofs[airgroup].push(proof);
                    }
                } else {
                    // If there are no instances, we need to add a null proof (only rank 0)
                    if dctx.rank == 0 {
                        if null_zkin.is_none() {
                            let setup_path = pctx.global_info.get_air_setup_path(airgroup, 0, proof_type);
                            let zkin_file = setup_path.display().to_string() + ".null_zkin.json";
                            null_zkin = Some(get_zkin_ptr_c(&zkin_file));
                        }
                        airgroup_proofs[airgroup].push(Some(null_zkin.unwrap()));
                    } else {
                        airgroup_proofs[airgroup].push(None);
                    }
                }
                alives.push(airgroup_proofs[airgroup].len());
            }
            // agregation loop
            loop {
                dctx.barrier();
                dctx.distribute_recursive2_proofs(&alives, &mut airgroup_proofs);
                let mut pending_agregations = false;
                for airgroup in 0..n_airgroups {
                    //create a vector of sice indices length
                    let mut alive = alives[airgroup];
                    if alive > 1 {
                        for i in 0..alive / 2 {
                            let j = i * 2;
                            if airgroup_proofs[airgroup][j].is_none() {
                                continue;
                            }
                            if j + 1 < alive {
                                if airgroup_proofs[airgroup][j + 1].is_none() {
                                    panic!("Recursive2 proof is missing");
                                }
                                let setup = sctx.get_setup(airgroup, 0);
                                let p_setup: *mut c_void = (&setup.p_setup).into();
                                let p_stark_info: *mut c_void = setup.p_setup.p_stark_info;

                                let public_inputs_guard = pctx.public_inputs.inputs.read().unwrap();
                                let challenges_guard = pctx.challenges.challenges.read().unwrap();

                                let public_inputs = (*public_inputs_guard).as_ptr() as *mut c_void;
                                let challenges = (*challenges_guard).as_ptr() as *mut c_void;

                                let zkin_recursive2 = join_zkin_recursive2_c(
                                    airgroup as u64,
                                    public_inputs,
                                    challenges,
                                    global_info_file,
                                    airgroup_proofs[airgroup][j].unwrap(),
                                    airgroup_proofs[airgroup][j + 1].unwrap(),
                                    p_stark_info,
                                );

                                let recursive2_verkey = pctx
                                    .global_info
                                    .get_air_setup_path(airgroup, 0, &ProofType::Recursive2)
                                    .display()
                                    .to_string()
                                    + ".verkey.json";
                                let zkin_recursive2_updated =
                                    add_recursive2_verkey_c(zkin_recursive2, recursive2_verkey.as_str());

                                let (buffer, publics) = generate_witness(
                                    pctx,
                                    airgroup,
                                    0,
                                    p_stark_info,
                                    zkin_recursive2_updated,
                                    proof_type,
                                )?;
                                let p_publics = publics.as_ptr() as *mut c_void;
                                let p_address = buffer.as_ptr() as *mut c_void;

                                timer_start_trace!(GENERATE_RECURSIVE2_PROOF);
                                let proof_file = match save_proof {
                                    true => output_dir_path
                                        .join(format!(
                                            "proofs/recursive2_{}_{}_{}.json",
                                            pctx.global_info.air_groups[airgroup],
                                            j,
                                            j + 1
                                        ))
                                        .to_string_lossy()
                                        .into_owned(),
                                    false => String::from(""),
                                };

                                let air_instance_name = &pctx.global_info.airs[airgroup][0].name;

                                log::info!(
                                    "{}: {}",
                                    MY_NAME,
                                    format!("··· Generating recursive2 proof for instances of {}", air_instance_name)
                                );
                                let const_pols_ptr = (*setup.const_pols.values.read().unwrap()).as_ptr() as *mut c_void;
                                let const_tree_ptr = (*setup.const_tree.values.read().unwrap()).as_ptr() as *mut c_void;

                                let zkin = gen_recursive_proof_c(
                                    p_setup,
                                    p_address,
                                    const_pols_ptr,
                                    const_tree_ptr,
                                    p_publics,
                                    &proof_file,
                                    global_info_file,
                                    airgroup as u64,
                                );

                                airgroup_proofs[airgroup][j] = Some(zkin);

                                drop(buffer);
                                drop(publics);
                                timer_stop_and_log_trace!(GENERATE_RECURSIVE2_PROOF);
                                log::info!("{}: ··· Proof generated.", MY_NAME);
                            }
                        }
                        alive = (alive + 1) / 2;
                        //compact elements
                        for i in 0..alive {
                            airgroup_proofs[airgroup][i] = airgroup_proofs[airgroup][i * 2];
                        }
                        alives[airgroup] = alive;
                        if alive > 1 {
                            pending_agregations = true;
                        }
                    }
                }
                if !pending_agregations {
                    break;
                }
            }
            if dctx.rank == 0 {
                let mut proofs_recursive2: Vec<*mut c_void> = Vec::with_capacity(n_airgroups);
                for proofs in airgroup_proofs {
                    proofs_recursive2.push(proofs[0].unwrap());
                }
                let public_inputs_guard = pctx.public_inputs.inputs.read().unwrap();
                let challenges_guard = pctx.challenges.challenges.read().unwrap();
                let proof_values_guard = pctx.proof_values.values.read().unwrap();

                let public_inputs = (*public_inputs_guard).as_ptr() as *mut c_void;
                let challenges = (*challenges_guard).as_ptr() as *mut c_void;
                let proof_values = (*proof_values_guard).as_ptr() as *mut c_void;

                let mut stark_infos_recursive2 = Vec::new();
                for (idx, _) in pctx.global_info.air_groups.iter().enumerate() {
                    stark_infos_recursive2.push(sctx.get_setup(idx, 0).p_setup.p_stark_info);
                }

                let proofs_recursive2_ptr = proofs_recursive2.as_mut_ptr();

                let stark_infos_recursive2_ptr = stark_infos_recursive2.as_mut_ptr();

                let zkin_final = join_zkin_final_c(
                    public_inputs,
                    proof_values,
                    challenges,
                    global_info_file,
                    proofs_recursive2_ptr,
                    stark_infos_recursive2_ptr,
                );

                proofs_out.push(zkin_final);
            }
        }
        ProofType::Final => {
            let setup = sctx.get_setup(0, 0);
            let p_setup: *mut c_void = (&setup.p_setup).into();
            let p_stark_info: *mut c_void = setup.p_setup.p_stark_info;

            let (buffer, publics) = generate_witness(pctx, 0, 0, p_stark_info, proofs[0], proof_type)?;
            let p_address = buffer.as_ptr() as *mut c_void;
            let p_publics = publics.as_ptr() as *mut c_void;

            log::info!("{}: ··· Generating final proof", MY_NAME);
            timer_start_trace!(GENERATE_PROOF);
            // prove
            let const_pols_ptr = (*setup.const_pols.values.read().unwrap()).as_ptr() as *mut c_void;
            let const_tree_ptr = (*setup.const_tree.values.read().unwrap()).as_ptr() as *mut c_void;
            let _p_prove = gen_recursive_proof_c(
                p_setup,
                p_address,
                const_pols_ptr,
                const_tree_ptr,
                p_publics,
                output_dir_path.join("proofs/final_proof.json").to_string_lossy().as_ref(),
                global_info_file,
                0,
            );
            log::info!("{}: ··· Proof generated.", MY_NAME);
            drop(buffer);
            timer_stop_and_log_trace!(GENERATE_PROOF);
        }
        ProofType::Basic => {
            panic!("Recursion proof whould not be calles for ProofType::Basic ");
        }
    }

    Ok(proofs_out)
}

fn generate_witness<F: Field>(
    pctx: &ProofCtx<F>,
    airgroup_id: usize,
    air_id: usize,
    p_stark_info: *mut c_void,
    zkin: *mut c_void,
    proof_type: &ProofType,
) -> GenWitnessResult<F> {
    // Load the symbol (function) from the library
    timer_start_trace!(CALCULATE_WITNESS);

    let total_n = get_map_totaln_c(p_stark_info) as usize;
    let buffer: Vec<MaybeUninit<F>> = Vec::with_capacity(total_n);
    let p_address = buffer.as_ptr() as *mut c_void;

    let n = get_stark_info_n_c(p_stark_info);
    let offset_cm1 = get_map_offsets_c(p_stark_info, "cm1", false);

    let n_publics = get_stark_info_n_publics_c(p_stark_info) as usize;
    let publics: Vec<MaybeUninit<F>> = Vec::with_capacity(n_publics);
    let p_publics = publics.as_ptr() as *mut c_void;

    let setup_path = match proof_type {
        ProofType::Final => pctx.global_info.get_final_setup_path(),
        _ => pctx.global_info.get_air_setup_path(airgroup_id, air_id, proof_type),
    };

    let rust_lib_filename = setup_path.display().to_string() + ".so";
    let rust_lib_path = Path::new(rust_lib_filename.as_str());

    if !rust_lib_path.exists() {
        return Err(format!("Rust lib dynamic library not found at path: {:?}", rust_lib_path).into());
    }

    let library: Library = unsafe { Library::new(rust_lib_path)? };
    unsafe {
        let get_commited_pols: Symbol<GetCommitedPolsFunc> = library.get(b"getCommitedPols\0")?;

        // Call the function
        let dat_filename = setup_path.display().to_string() + ".dat";
        let dat_filename_str = CString::new(dat_filename.as_str()).unwrap();
        let dat_filename_ptr = dat_filename_str.as_ptr() as *mut std::os::raw::c_char;

        let exec_filename = setup_path.display().to_string() + ".exec";
        let exec_filename_str = CString::new(exec_filename.as_str()).unwrap();
        let exec_filename_ptr = exec_filename_str.as_ptr() as *mut std::os::raw::c_char;

        get_commited_pols(
            p_address,
            p_publics,
            zkin,
            n,
            n_publics as u64,
            offset_cm1,
            dat_filename_ptr,
            exec_filename_ptr,
        );
    }
    timer_stop_and_log_trace!(CALCULATE_WITNESS);

    Ok((buffer, publics))
}
