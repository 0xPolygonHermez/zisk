use libloading::{Library, Symbol};
use p3_field::Field;
use std::ffi::CString;
use std::fs::File;
use std::sync::Arc;
use proofman_starks_lib_c::*;
use std::path::{Path, PathBuf};
use std::io::Read;

use proofman_common::{ProofCtx, ProofType, Setup, SetupCtx, SetupsVadcop};

use std::os::raw::{c_void, c_char};

use proofman_util::{create_buffer_fast_u8, timer_start_trace, timer_stop_and_log_trace};

type GetWitnessFunc =
    unsafe extern "C" fn(zkin: *mut c_void, dat_file: *const c_char, witness: *mut c_void, n_mutexes: u64);

type GetSizeWitnessFunc = unsafe extern "C" fn() -> u64;

#[allow(clippy::too_many_arguments)]
pub fn generate_vadcop_recursive1_proof<F: Field>(
    pctx: &ProofCtx<F>,
    setups: Arc<SetupsVadcop>,
    proofs: &[*mut c_void],
    circom_witness: &mut [F],
    publics: &[F],
    trace: &[F],
    prover_buffer: &[F],
    output_dir_path: PathBuf,
) -> Result<Vec<*mut c_void>, Box<dyn std::error::Error>> {
    const MY_NAME: &str = "AggProof";

    //Create setup contexts
    let mut proofs_out: Vec<*mut c_void> = Vec::new();

    let global_info_path = pctx.global_info.get_proving_key_path().join("pilout.globalInfo.json");
    let global_info_file: &str = global_info_path.to_str().unwrap();

    let instances = pctx.dctx_get_instances();
    let my_instances = pctx.dctx_get_my_instances();

    for (idx, instance_id) in my_instances.iter().enumerate() {
        let (airgroup_id, air_id) = instances[*instance_id];

        let air_instance_name = &pctx.global_info.airs[airgroup_id][air_id].name;
        let air_instance_id = pctx.dctx_find_air_instance_id(*instance_id);
        let mut zkin;

        if pctx.global_info.get_air_has_compressor(airgroup_id, air_id) {
            timer_start_trace!(GENERATING_COMPRESSOR_PROOF);

            let setup = setups.sctx_compressor.as_ref().unwrap().get_setup(airgroup_id, air_id);
            let p_setup: *mut c_void = (&setup.p_setup).into();

            let setup_path = pctx.global_info.get_air_setup_path(airgroup_id, air_id, &ProofType::Compressor);

            generate_witness::<F>(circom_witness, trace, publics, &setup_path, setup, proofs[idx], 18)?;

            log::info!(
                "{}: {}",
                MY_NAME,
                format!("··· Generating compressor proof for instance {} of {}", air_instance_id, air_instance_name)
            );

            let output_file_path =
                output_dir_path.join(format!("proofs/compressor_{}_{}.json", air_instance_name, instance_id));

            let proof_file = match pctx.options.debug_info.save_proofs_to_file {
                true => output_file_path.to_string_lossy().into_owned(),
                false => String::from(""),
            };

            zkin = gen_recursive_proof_c(
                p_setup,
                trace.as_ptr() as *mut u8,
                prover_buffer.as_ptr() as *mut u8,
                setup.get_const_ptr(),
                setup.get_const_tree_ptr(),
                publics.as_ptr() as *mut u8,
                &proof_file,
                global_info_file,
                airgroup_id as u64,
                true,
            );

            log::info!("{}: ··· Compressor Proof generated.", MY_NAME);
            timer_stop_and_log_trace!(GENERATING_COMPRESSOR_PROOF);
        } else {
            zkin = proofs[idx];
        }

        timer_start_trace!(GENERATE_RECURSIVE1_PROOF);

        let setup = setups.sctx_recursive1.as_ref().unwrap().get_setup(airgroup_id, air_id);
        let p_setup: *mut c_void = (&setup.p_setup).into();

        let recursive2_verkey =
            pctx.global_info.get_air_setup_path(airgroup_id, air_id, &ProofType::Recursive2).display().to_string()
                + ".verkey.json";

        zkin = add_recursive2_verkey_c(zkin, recursive2_verkey.as_str());

        let setup_path = pctx.global_info.get_air_setup_path(airgroup_id, air_id, &ProofType::Recursive1);

        generate_witness::<F>(circom_witness, trace, publics, &setup_path, setup, zkin, 18)?;

        log::info!(
            "{}: {}",
            MY_NAME,
            format!("··· Generating recursive1 proof for instance {} of {}", air_instance_id, air_instance_name)
        );

        let output_file_path =
            output_dir_path.join(format!("proofs/recursive1_{}_{}.json", air_instance_name, instance_id));

        let proof_file = match pctx.options.debug_info.save_proofs_to_file {
            true => output_file_path.to_string_lossy().into_owned(),
            false => String::from(""),
        };

        let p_prove = gen_recursive_proof_c(
            p_setup,
            trace.as_ptr() as *mut u8,
            prover_buffer.as_ptr() as *mut u8,
            setup.get_const_ptr(),
            setup.get_const_tree_ptr(),
            publics.as_ptr() as *mut u8,
            &proof_file,
            global_info_file,
            airgroup_id as u64,
            true,
        );
        proofs_out.push(p_prove);

        log::info!("{}: ··· Recursive1 Proof generated.", MY_NAME);
        timer_stop_and_log_trace!(GENERATE_RECURSIVE1_PROOF);
    }

    Ok(proofs_out)
}

#[allow(clippy::too_many_arguments)]
pub fn generate_vadcop_recursive2_proof<F: Field>(
    pctx: &ProofCtx<F>,
    sctx: Arc<SetupCtx>,
    proofs: &[*mut c_void],
    circom_witness: &mut [F],
    publics: &[F],
    trace: &[F],
    prover_buffer: &[F],
    output_dir_path: PathBuf,
) -> Result<*mut c_void, Box<dyn std::error::Error>> {
    const MY_NAME: &str = "AggProof";

    let global_info_path = pctx.global_info.get_proving_key_path().join("pilout.globalInfo.json");
    let global_info_file: &str = global_info_path.to_str().unwrap();

    let mut dctx = pctx.dctx.write().unwrap();
    let n_airgroups = pctx.global_info.air_groups.len();
    let mut alives = Vec::with_capacity(n_airgroups);
    let mut airgroup_proofs: Vec<Vec<Option<*mut c_void>>> = Vec::with_capacity(n_airgroups);

    let mut null_zkin: Option<*mut c_void> = None;
    let mut zkin_final = std::ptr::null_mut();

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
                    let setup_path = pctx.global_info.get_air_setup_path(airgroup, 0, &ProofType::Recursive2);
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

                        let zkin_recursive2 = join_zkin_recursive2_c(
                            airgroup as u64,
                            pctx.get_publics_ptr(),
                            pctx.get_challenges_ptr(),
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

                        let setup_path = pctx.global_info.get_air_setup_path(airgroup, 0, &ProofType::Recursive2);

                        generate_witness::<F>(
                            circom_witness,
                            trace,
                            publics,
                            &setup_path,
                            setup,
                            zkin_recursive2_updated,
                            18,
                        )?;

                        timer_start_trace!(GENERATE_RECURSIVE2_PROOF);
                        let proof_file = match pctx.options.debug_info.save_proofs_to_file {
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

                        let zkin = gen_recursive_proof_c(
                            p_setup,
                            trace.as_ptr() as *mut u8,
                            prover_buffer.as_ptr() as *mut u8,
                            setup.get_const_ptr(),
                            setup.get_const_tree_ptr(),
                            publics.as_ptr() as *mut u8,
                            &proof_file,
                            global_info_file,
                            airgroup as u64,
                            true,
                        );

                        airgroup_proofs[airgroup][j] = Some(zkin);

                        timer_stop_and_log_trace!(GENERATE_RECURSIVE2_PROOF);
                        log::info!("{}: ··· Recursive 2 Proof generated.", MY_NAME);
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

        let mut stark_infos_recursive2 = Vec::new();
        for (idx, _) in pctx.global_info.air_groups.iter().enumerate() {
            stark_infos_recursive2.push(sctx.get_setup(idx, 0).p_setup.p_stark_info);
        }

        let proofs_recursive2_ptr = proofs_recursive2.as_mut_ptr();

        let stark_infos_recursive2_ptr = stark_infos_recursive2.as_mut_ptr();

        zkin_final = join_zkin_final_c(
            pctx.get_publics_ptr(),
            pctx.get_proof_values_ptr(),
            pctx.get_challenges_ptr(),
            global_info_file,
            proofs_recursive2_ptr,
            stark_infos_recursive2_ptr,
        );
    }

    Ok(zkin_final)
}

#[allow(clippy::too_many_arguments)]
pub fn generate_vadcop_final_proof<F: Field>(
    pctx: &ProofCtx<F>,
    setup: Arc<Setup>,
    proof: *mut c_void,
    circom_witness: &mut [F],
    publics: &[F],
    trace: &[F],
    prover_buffer: &[F],
    output_dir_path: PathBuf,
) -> Result<*mut c_void, Box<dyn std::error::Error>> {
    const MY_NAME: &str = "AggProof";

    let global_info_path = pctx.global_info.get_proving_key_path().join("pilout.globalInfo.json");
    let global_info_file: &str = global_info_path.to_str().unwrap();

    let p_setup: *mut c_void = (&setup.p_setup).into();

    let setup_path = pctx.global_info.get_setup_path("vadcop_final");

    generate_witness::<F>(circom_witness, trace, publics, &setup_path, &setup, proof, 18)?;

    let proof_file = output_dir_path.join("proofs/vadcop_final_proof.json").to_string_lossy().into_owned();

    log::info!("{}: ··· Generating vadcop final proof", MY_NAME);
    timer_start_trace!(GENERATE_VADCOP_FINAL_PROOF);
    // prove
    let p_prove = gen_recursive_proof_c(
        p_setup,
        trace.as_ptr() as *mut u8,
        prover_buffer.as_ptr() as *mut u8,
        setup.get_const_ptr(),
        setup.get_const_tree_ptr(),
        publics.as_ptr() as *mut u8,
        &proof_file,
        global_info_file,
        0,
        false,
    );
    log::info!("{}: ··· Vadcop final Proof generated.", MY_NAME);
    timer_stop_and_log_trace!(GENERATE_VADCOP_FINAL_PROOF);

    Ok(p_prove)
}

#[allow(clippy::too_many_arguments)]
pub fn generate_recursivef_proof<F: Field>(
    pctx: &ProofCtx<F>,
    setup: Arc<Setup>,
    proof: *mut c_void,
    circom_witness: &mut [F],
    publics: &[F],
    trace: &[F],
    prover_buffer: &[F],
    output_dir_path: PathBuf,
) -> Result<*mut c_void, Box<dyn std::error::Error>> {
    const MY_NAME: &str = "RecProof";

    let global_info_path = pctx.global_info.get_proving_key_path().join("pilout.globalInfo.json");
    let global_info_file: &str = global_info_path.to_str().unwrap();

    let p_setup: *mut c_void = (&setup.p_setup).into();

    let setup_path = pctx.global_info.get_setup_path("recursivef");

    generate_witness::<F>(circom_witness, trace, publics, &setup_path, &setup, proof, 12)?;

    let proof_file = match pctx.options.debug_info.save_proofs_to_file {
        true => output_dir_path.join("proofs/recursivef.json").to_string_lossy().into_owned(),
        false => String::from(""),
    };

    log::info!("{}: ··· Generating recursiveF proof", MY_NAME);
    timer_start_trace!(GENERATE_RECURSIVEF_PROOF);
    // prove
    let p_prove = gen_recursive_proof_c(
        p_setup,
        trace.as_ptr() as *mut u8,
        prover_buffer.as_ptr() as *mut u8,
        setup.get_const_ptr(),
        setup.get_const_tree_ptr(),
        publics.as_ptr() as *mut u8,
        &proof_file,
        global_info_file,
        0,
        false,
    );
    log::info!("{}: ··· RecursiveF Proof generated.", MY_NAME);
    timer_stop_and_log_trace!(GENERATE_RECURSIVEF_PROOF);

    Ok(p_prove)
}

pub fn generate_fflonk_snark_proof<F: Field>(
    pctx: &ProofCtx<F>,
    proof: *mut c_void,
    output_dir_path: PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    const MY_NAME: &str = "FinProof";

    let setup_path = pctx.global_info.get_setup_path("final");

    let rust_lib_filename = setup_path.display().to_string() + ".so";
    let rust_lib_path = Path::new(rust_lib_filename.as_str());

    if !rust_lib_path.exists() {
        return Err(format!("Rust lib dynamic library not found at path: {:?}", rust_lib_path).into());
    }
    let library: Library = unsafe { Library::new(rust_lib_path)? };

    let dat_filename = setup_path.display().to_string() + ".dat";
    let dat_filename_str = CString::new(dat_filename.as_str()).unwrap();
    let dat_filename_ptr = dat_filename_str.as_ptr() as *mut std::os::raw::c_char;

    unsafe {
        timer_start_trace!(CALCULATE_FINAL_WITNESS);

        let get_size_witness: Symbol<GetSizeWitnessFunc> = library.get(b"getSizeWitness\0")?;
        let size_witness = get_size_witness();

        let witness = create_buffer_fast_u8((size_witness * 32) as usize);
        let witness_ptr = witness.as_ptr() as *mut u8;

        let get_witness: Symbol<GetWitnessFunc> = library.get(b"getWitness\0")?;

        let nmutex = 128;
        get_witness(proof, dat_filename_ptr, witness_ptr as *mut c_void, nmutex);

        timer_stop_and_log_trace!(CALCULATE_FINAL_WITNESS);

        timer_start_trace!(CALCULATE_FINAL_PROOF);
        let proof_file = output_dir_path.join("proofs").to_string_lossy().into_owned();

        let zkey_filename = setup_path.display().to_string() + ".zkey";
        log::info!("{}: ··· Generating final snark proof", MY_NAME);
        gen_final_snark_proof_c(witness_ptr, zkey_filename.as_str(), &proof_file);
        timer_stop_and_log_trace!(CALCULATE_FINAL_PROOF);
        log::info!("{}: ··· Final Snark Proof generated.", MY_NAME);
    }

    Ok(())
}

fn generate_witness<F: Field>(
    witness: &mut [F],
    buffer: &[F],
    publics: &[F],
    setup_path: &Path,
    setup: &Setup,
    zkin: *mut c_void,
    n_cols: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    // Load the symbol (function) from the library
    let rust_lib_filename = setup_path.display().to_string() + ".so";
    let rust_lib_path = Path::new(rust_lib_filename.as_str());

    if !rust_lib_path.exists() {
        return Err(format!("Rust lib dynamic library not found at path: {:?}", rust_lib_path).into());
    }

    let library: Library = unsafe { Library::new(rust_lib_path)? };

    let dat_filename = setup_path.display().to_string() + ".dat";
    let dat_filename_str = CString::new(dat_filename.as_str()).unwrap();
    let dat_filename_ptr = dat_filename_str.as_ptr() as *mut std::os::raw::c_char;

    let nmutex = 128;

    let exec_filename = setup_path.display().to_string() + ".exec";
    let exec_filename_str = CString::new(exec_filename.as_str()).unwrap();
    let exec_filename_ptr = exec_filename_str.as_ptr() as *mut std::os::raw::c_char;

    let size_witness = unsafe {
        let get_size_witness: Symbol<GetSizeWitnessFunc> = library.get(b"getSizeWitness\0")?;
        get_size_witness()
    };

    timer_start_trace!(CALCULATE_WITNESS);

    unsafe {
        let get_witness: Symbol<GetWitnessFunc> = library.get(b"getWitness\0")?;
        get_witness(zkin, dat_filename_ptr, witness.as_ptr() as *mut c_void, nmutex);
    }

    get_committed_pols_c(
        witness.as_ptr() as *mut u8,
        exec_filename_ptr,
        buffer.as_ptr() as *mut u8,
        publics.as_ptr() as *mut u8,
        size_witness,
        1 << (setup.stark_info.stark_struct.n_bits),
        setup.stark_info.n_publics,
        n_cols as u64,
    );
    timer_stop_and_log_trace!(CALCULATE_WITNESS);

    Ok(())
}

pub fn get_buff_sizes<F: Field>(
    pctx: Arc<ProofCtx<F>>,
    setups: Arc<SetupsVadcop>,
) -> Result<(usize, usize, usize, usize), Box<dyn std::error::Error>> {
    let mut witness_size = 0;
    let mut publics = 0;
    let mut buffer = 0;
    let mut prover_size = 0;

    let instances = pctx.dctx_get_instances();
    let my_instances = pctx.dctx_get_my_instances();

    for instance_id in my_instances.iter() {
        let (airgroup_id, air_id) = instances[*instance_id];

        if pctx.global_info.get_air_has_compressor(airgroup_id, air_id) {
            let setup_compressor = setups.sctx_compressor.as_ref().unwrap().get_setup(airgroup_id, air_id);
            let setup_path = pctx.global_info.get_air_setup_path(airgroup_id, air_id, &ProofType::Compressor);
            let sizes = get_size(&setup_path, setup_compressor, 18)?;
            witness_size = witness_size.max(sizes.0);
            publics = publics.max(sizes.1);
            buffer = buffer.max(sizes.2);
            prover_size = prover_size.max(setup_compressor.prover_buffer_size);
        }

        let setup_recursive1 = setups.sctx_recursive1.as_ref().unwrap().get_setup(airgroup_id, air_id);
        let setup_path = pctx.global_info.get_air_setup_path(airgroup_id, air_id, &ProofType::Recursive1);
        let sizes = get_size(&setup_path, setup_recursive1, 18)?;
        witness_size = witness_size.max(sizes.0);
        publics = publics.max(sizes.1);
        buffer = buffer.max(sizes.2);
        prover_size = prover_size.max(setup_recursive1.prover_buffer_size);
    }

    let n_airgroups = pctx.global_info.air_groups.len();
    for airgroup in 0..n_airgroups {
        let setup = setups.sctx_recursive2.as_ref().unwrap().get_setup(airgroup, 0);
        let setup_path = pctx.global_info.get_air_setup_path(airgroup, 0, &ProofType::Recursive2);
        let sizes = get_size(&setup_path, setup, 18)?;
        witness_size = witness_size.max(sizes.0);
        publics = publics.max(sizes.1);
        buffer = buffer.max(sizes.2);
        prover_size = prover_size.max(setup.prover_buffer_size);
    }

    let setup_final = setups.setup_vadcop_final.as_ref().unwrap();
    let setup_path = pctx.global_info.get_setup_path("vadcop_final");
    let sizes = get_size(&setup_path, setup_final, 18)?;
    witness_size = witness_size.max(sizes.0);
    publics = publics.max(sizes.1);
    buffer = buffer.max(sizes.2);
    prover_size = prover_size.max(setup_final.prover_buffer_size);

    if pctx.options.final_snark {
        let setup_recursivef = setups.setup_recursivef.as_ref().unwrap();
        let setup_path = pctx.global_info.get_setup_path("recursivef");
        let sizes = get_size(&setup_path, setup_recursivef, 12)?;
        witness_size = witness_size.max(sizes.0);
        publics = publics.max(sizes.1);
        buffer = buffer.max(sizes.2);
        prover_size = prover_size.max(setup_recursivef.prover_buffer_size);
    }

    Ok((witness_size, publics, buffer, prover_size as usize))
}

fn get_size(
    setup_path: &Path,
    setup: &Setup,
    n_cols: usize,
) -> Result<(usize, usize, usize), Box<dyn std::error::Error>> {
    // Load the symbol (function) from the library
    let rust_lib_filename = setup_path.display().to_string() + ".so";
    let rust_lib_path = Path::new(rust_lib_filename.as_str());

    if !rust_lib_path.exists() {
        return Err(format!("Rust lib dynamic library not found at path: {:?}", rust_lib_path).into());
    }

    let library: Library = unsafe { Library::new(rust_lib_path)? };

    let exec_filename = setup_path.display().to_string() + ".exec";

    let mut size_witness = unsafe {
        let get_size_witness: Symbol<GetSizeWitnessFunc> = library.get(b"getSizeWitness\0")?;
        get_size_witness()
    };

    let mut file = File::open(exec_filename)?; // Open the file

    let mut n_adds = [0u8; 8]; // Buffer for nAdds (u64 is 8 bytes)
    file.read_exact(&mut n_adds)?;
    let n_adds = u64::from_le_bytes(n_adds);

    size_witness += n_adds;

    let n_publics = setup.stark_info.n_publics as usize;
    let buffer_size = n_cols * (1 << (setup.stark_info.stark_struct.n_bits)) as usize;

    Ok((size_witness as usize, n_publics, buffer_size))
}
