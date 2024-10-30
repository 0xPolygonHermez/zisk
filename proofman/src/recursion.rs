use libloading::{Library, Symbol};
use p3_field::Field;
use std::ffi::CString;
use proofman_starks_lib_c::*;
use std::mem::MaybeUninit;
use std::path::{Path, PathBuf};

use proofman_common::{ProofCtx, ProofType, SetupCtx};

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
    proves: &[*mut c_void],
    proof_type: &ProofType,
    output_dir_path: PathBuf,
    save_proof: bool,
) -> Result<Vec<*mut c_void>, Box<dyn std::error::Error>> {
    const MY_NAME: &str = "AggProof";

    //Create setup contexts
    let mut proves_out: Vec<*mut c_void> = Vec::new();

    let global_info_path = pctx.global_info.get_proving_key_path().join("pilout.globalInfo.json");
    let global_info_file: &str = global_info_path.to_str().unwrap();

    let sctx = SetupCtx::new(&pctx.global_info, proof_type);

    // Run proves
    match *proof_type {
        ProofType::Compressor | ProofType::Recursive1 => {
            for (prover_idx, air_instance) in
                pctx.air_instance_repo.air_instances.write().unwrap().iter_mut().enumerate()
            {
                if *proof_type == ProofType::Compressor
                    && !pctx.global_info.get_air_has_compressor(air_instance.airgroup_id, air_instance.air_id)
                {
                    proves_out.push(proves[prover_idx]);
                } else {
                    let air_instance_name = &pctx.global_info.airs[air_instance.airgroup_id][air_instance.air_id].name;

                    timer_start_trace!(GET_SETUP);
                    let setup = sctx.get_setup(air_instance.airgroup_id, air_instance.air_id).expect("Setup not found");
                    let p_setup: *mut c_void = (&setup.p_setup).into();
                    let p_stark_info: *mut c_void = setup.p_setup.p_stark_info;
                    timer_stop_and_log_trace!(GET_SETUP);

                    let mut zkin = proves[prover_idx];
                    if *proof_type == ProofType::Recursive1 {
                        let recursive2_verkey = pctx
                            .global_info
                            .get_air_setup_path(air_instance.airgroup_id, air_instance.air_id, &ProofType::Recursive2)
                            .display()
                            .to_string()
                            + ".verkey.json";
                        zkin = add_recursive2_verkey_c(zkin, recursive2_verkey.as_str());
                    }

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

                    let mut p_prove = gen_recursive_proof_c(p_setup, p_address, p_publics, &proof_file);
                    p_prove =
                        publics2zkin_c(p_prove, p_publics, global_info_file, air_instance.airgroup_id as u64, false);
                    proves_out.push(p_prove);

                    drop(buffer);
                    log::info!("{}: ··· Proof generated.", MY_NAME);

                    timer_stop_and_log_trace!(GENERATE_PROOF);
                }
            }
        }
        ProofType::Recursive2 => {
            let n_airgroups = pctx.global_info.air_groups.len();
            let mut proves_recursive2: Vec<*mut c_void> = Vec::with_capacity(n_airgroups);

            for airgroup in 0..n_airgroups {
                let setup_path = pctx.global_info.get_air_setup_path(airgroup, 0, proof_type);

                let instances = pctx.air_instance_repo.find_airgroup_instances(airgroup);
                if instances.is_empty() {
                    let zkin_file = setup_path.display().to_string() + ".null_zkin.json";
                    let null_zkin = get_zkin_ptr_c(&zkin_file);
                    proves_recursive2.push(null_zkin);
                } else if instances.len() == 1 {
                    proves_recursive2.insert(airgroup, proves[instances[0]]);
                } else {
                    let mut proves_recursive2_airgroup: Vec<*mut c_void> = Vec::new();

                    for instance in instances.iter() {
                        proves_recursive2_airgroup.push(proves[*instance]);
                    }

                    //create a vector of sice indices length
                    let mut alive = instances.len();
                    while alive > 1 {
                        for i in 0..alive / 2 {
                            let j = i * 2;
                            if j + 1 < alive {
                                timer_start_trace!(GET_RECURSIVE2_SETUP);
                                let setup = sctx.get_setup(airgroup, 0).expect("Setup not found");
                                let p_setup: *mut c_void = (&setup.p_setup).into();
                                let p_stark_info: *mut c_void = setup.p_setup.p_stark_info;
                                timer_stop_and_log_trace!(GET_RECURSIVE2_SETUP);

                                let public_inputs_guard = pctx.public_inputs.inputs.read().unwrap();
                                let challenges_guard = pctx.challenges.challenges.read().unwrap();

                                let public_inputs = (*public_inputs_guard).as_ptr() as *mut c_void;
                                let challenges = (*challenges_guard).as_ptr() as *mut c_void;

                                let mut zkin_recursive2 = join_zkin_recursive2_c(
                                    airgroup as u64,
                                    public_inputs,
                                    challenges,
                                    global_info_file,
                                    proves_recursive2_airgroup[j],
                                    proves_recursive2_airgroup[j + 1],
                                    p_stark_info,
                                );

                                let recursive2_verkey = pctx
                                    .global_info
                                    .get_air_setup_path(airgroup, 0, &ProofType::Recursive2)
                                    .display()
                                    .to_string()
                                    + ".verkey.json";
                                zkin_recursive2 = add_recursive2_verkey_c(zkin_recursive2, recursive2_verkey.as_str());

                                let (buffer, publics) =
                                    generate_witness(pctx, airgroup, 0, p_stark_info, zkin_recursive2, proof_type)?;
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

                                proves_recursive2_airgroup[j] =
                                    gen_recursive_proof_c(p_setup, p_address, p_publics, &proof_file);
                                proves_recursive2_airgroup[j] = publics2zkin_c(
                                    proves_recursive2_airgroup[j],
                                    p_publics,
                                    global_info_file,
                                    airgroup as u64,
                                    false,
                                );
                                drop(buffer);
                                timer_stop_and_log_trace!(GENERATE_RECURSIVE2_PROOF);
                                log::info!("{}: ··· Proof generated.", MY_NAME);
                            }
                        }
                        alive = (alive + 1) / 2;
                        //compact elements
                        for i in 0..alive {
                            proves_recursive2_airgroup[i] = proves_recursive2_airgroup[i * 2];
                        }
                    }
                    proves_recursive2.push(proves_recursive2_airgroup[0]);
                }
            }

            let public_inputs_guard = pctx.public_inputs.inputs.read().unwrap();
            let challenges_guard = pctx.challenges.challenges.read().unwrap();
            let proof_values_guard = pctx.proof_values.values.read().unwrap();

            let public_inputs = (*public_inputs_guard).as_ptr() as *mut c_void;
            let challenges = (*challenges_guard).as_ptr() as *mut c_void;
            let proof_values = (*proof_values_guard).as_ptr() as *mut c_void;

            let mut stark_infos_recursive2 = Vec::new();
            for (idx, _) in pctx.global_info.air_groups.iter().enumerate() {
                stark_infos_recursive2.push(sctx.get_setup(idx, 0).unwrap().p_setup.p_stark_info);
            }

            let proves_recursive2_ptr = proves_recursive2.as_mut_ptr();

            let stark_infos_recursive2_ptr = stark_infos_recursive2.as_mut_ptr();

            let zkin_final = join_zkin_final_c(
                public_inputs,
                proof_values,
                challenges,
                global_info_file,
                proves_recursive2_ptr,
                stark_infos_recursive2_ptr,
            );

            proves_out.push(zkin_final);
        }
        ProofType::Final => {
            timer_start_trace!(GET_FINAL_SETUP);
            let setup = sctx.get_setup(0, 0).expect("Setup not found");
            let p_setup: *mut c_void = (&setup.p_setup).into();
            let p_stark_info: *mut c_void = setup.p_setup.p_stark_info;
            timer_stop_and_log_trace!(GET_FINAL_SETUP);

            let (buffer, publics) = generate_witness(pctx, 0, 0, p_stark_info, proves[0], proof_type)?;
            let p_address = buffer.as_ptr() as *mut c_void;
            let p_publics = publics.as_ptr() as *mut c_void;

            log::info!("{}: ··· Generating final proof", MY_NAME);
            timer_start_trace!(GENERATE_PROOF);
            // prove
            let _p_prove = gen_recursive_proof_c(
                p_setup,
                p_address,
                p_publics,
                output_dir_path.join("proofs/final_proof.json").to_string_lossy().as_ref(),
            );
            log::info!("{}: ··· Proof generated.", MY_NAME);
            drop(buffer);
            timer_stop_and_log_trace!(GENERATE_PROOF);
        }
        ProofType::Basic => {
            panic!("Recursion proof whould not be calles for ProofType::Basic ");
        }
    }

    Ok(proves_out)
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
