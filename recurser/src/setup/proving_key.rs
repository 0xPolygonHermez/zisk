use std::fs;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use serde_json::Value;

use pil2_stark_setup::io::fixed_cols;
use pil2_stark_setup::output::witness_gen::WitnessTracker;
use pil2_stark_setup::proving_key::{bctree, recursive::compile_pil};
use pilout::pilout_proxy::PilOutProxy;
use stark_recurser::plonk2pil::r1cs_types::PlonkOptions;
use stark_recurser::plonk2pil::{self, PlonkResult};
use stark_recurser::stark2circom::stark_inputs::{
    assign_stark_inputs, define_stark_inputs, EnableInput, StarkInputOptions,
};
use stark_recurser::stark2circom::{gen_stark_verifier, StarkVerifierOptions};

use crate::artifacts::RecurserArtifacts;
use crate::templates::StarkInputBlocks;
use crate::{gen_recurser, CircomTemplates};

pub struct RecurserConfig<'a> {
    /// Where artifacts land (must differ from setup_dir).
    pub output_dir: &'a str,
    /// Content-addressed setup id. Artifacts land under
    /// `<output_dir>/provingKey/recurser/<recurser_id>/`.
    pub recurser_id: &'a str,

    /// Hash family the recurser recurses over (e.g. `"Poseidon1"` /
    /// `"Poseidon2"`). Must match vadcop_final's family; read from the proving
    /// key's `globalInfo.json`.
    pub hash: &'a str,

    /// Inner ZisK verifier's verkey (4 Goldilocks limbs). Baked into the
    /// recurser as `rootCVadcopFinalZisk`.
    pub zisk_vk: &'a [String; 4],
    pub stark_info: &'a Value,
    pub verifier_info: &'a Value,

    pub program_vks: &'a [[String; 4]],

    pub circom_templates: &'a CircomTemplates,

    pub circom_exec: &'a str,
    pub circuits_gl_path: &'a str,
    pub recurser_circuits_path: &'a str,
    pub circom_helpers_dir: &'a str,

    /// `pil2-components/lib/std/pil` (or env override). Passed to `pil2com`.
    pub std_pil_path: &'a str,
    /// PIL include path for recurser-side helpers. Passed to `pil2com`.
    pub recurser_pil_path: &'a str,
    /// vadcop_final's starkinfo.json — borrowed by `compute_const_tree`
    /// (recurser shares vadcop_final's STARK shape).
    pub vadcop_final_starkinfo_path: &'a str,
}

pub fn gen_recurser_setup(
    config: &RecurserConfig<'_>,
    witness_tracker: &WitnessTracker,
) -> Result<()> {
    let template = "recurser_aggregator";
    let verifier_name = "vadcop_final_stark.verifier.circom";
    let output_dir = PathBuf::from(config.output_dir);

    let artifacts = RecurserArtifacts::new(config.output_dir, config.recurser_id);
    let files_dir = artifacts.dir().to_path_buf();
    fs::create_dir_all(&files_dir)?;

    let circom_dir = output_dir.join("circom");
    let build_path = output_dir.join("build");
    let pil_dir = output_dir.join("pil");
    fs::create_dir_all(&circom_dir)?;
    fs::create_dir_all(&build_path)?;
    fs::create_dir_all(&pil_dir)?;

    // verkey_input=true so rootC is a signal driven by the recurser mux.
    {
        let rust_opts = StarkVerifierOptions {
            skip_main: true,
            verkey_input: true,
            enable_input: false,
            input_challenges: false,
            fri_queries_batch_size: None,
            multi_fri: false,
            hash: config.hash.to_string(),
        };
        let circom_src = gen_stark_verifier(
            Some(config.zisk_vk),
            config.stark_info,
            config.verifier_info,
            &rust_opts,
        )
        .context("gen_stark_verifier failed in recurser setup")?;
        fs::write(circom_dir.join(verifier_name), &circom_src)
            .context("Failed to write inner verifier circom")?;
    }

    fs::write(
        circom_dir.join(crate::templates::PUBLICS_HELPERS_FILENAME),
        crate::templates::PUBLICS_HELPERS_CIRCOM,
    )
    .context("Failed to write publics_helpers circom")?;

    let io_opts = StarkInputOptions { add_publics: true, is_final: false, parallel: false };
    let define_a = define_stark_inputs(config.stark_info, "a_sv", &io_opts);
    let define_b = define_stark_inputs(config.stark_info, "b_sv", &io_opts);
    let assign_a =
        assign_stark_inputs("vA", "a_sv", config.stark_info, &io_opts, &EnableInput::None);
    let assign_b =
        assign_stark_inputs("vB", "b_sv", config.stark_info, &io_opts, &EnableInput::None);

    let stark_inputs = StarkInputBlocks {
        define_a: define_a.as_str(),
        define_b: define_b.as_str(),
        assign_a: assign_a.as_str(),
        assign_b: assign_b.as_str(),
    };

    let circom_out = circom_dir.join(format!("{}.circom", template));
    {
        let circom_src = gen_recurser(
            verifier_name,
            &config.zisk_vk[..],
            config.program_vks,
            &stark_inputs,
            config.circom_templates,
        )
        .map_err(|e| anyhow::anyhow!("gen_recurser failed: {e}"))?;
        fs::write(&circom_out, &circom_src).context("Failed to write recurser circom")?;
    }

    tracing::info!("Compiling {}...", template);
    let compile_output = std::process::Command::new(config.circom_exec)
        .args([
            "--O1",
            "--r1cs",
            "--prime",
            "goldilocks",
            "--c",
            "--verbose",
            "-l",
            config.recurser_circuits_path,
            "-l",
            config.circuits_gl_path,
        ])
        .arg(circom_out.to_str().unwrap())
        .arg("-o")
        .arg(build_path.to_str().unwrap())
        .output()
        .context("Failed to execute circom for recurser setup")?;

    if !compile_output.status.success() {
        let stderr = String::from_utf8_lossy(&compile_output.stderr);
        bail!("Circom compilation failed for {}: {}", template, stderr);
    }

    tracing::info!("Copying circom files...");
    let dat_src = build_path.join(format!("{}_cpp", template)).join(format!("{}.dat", template));
    let dat_dst = files_dir.join(format!("{}.dat", template));
    if dat_src.exists() {
        fs::copy(&dat_src, &dat_dst)?;
    }

    witness_tracker.run_witness_library_generation(
        config.output_dir,
        files_dir.to_str().unwrap_or(""),
        template,
        template,
        config.circom_helpers_dir,
    );

    let r1cs_path = build_path.join(format!("{}.r1cs", template));
    let r1cs_data = fs::read(&r1cs_path)
        .with_context(|| format!("Failed to read R1CS: {}", r1cs_path.display()))?;

    let plonk_opts = PlonkOptions {
        airgroup_name: Some("RecurserAggregator".to_string()),
        max_constraint_degree: None,
        hash_id: config.hash.to_string(),
    };
    let plonk_result: PlonkResult = plonk2pil::plonk2pil(&r1cs_data, "aggregation", &plonk_opts)
        .context("plonk2pil failed in recurser setup")?;

    // The recurser's output proof must share vadcop_final's STARK domain so
    // it can be fed back into the next fold level (same circuit).
    let vadcop_n_bits = config
        .stark_info
        .get("starkStruct")
        .and_then(|s| s.get("nBits"))
        .and_then(|v| v.as_u64())
        .map(|n| n as usize)
        .context("vadcop_final stark_info is missing starkStruct.nBits")?;
    if plonk_result.n_bits != vadcop_n_bits {
        bail!(
            "Recurser n_bits ({}) does not match vadcop_final starkStruct.nBits ({}). \
             Reduce circuit size (fewer publics ops, smaller templates) or rebuild \
             vadcop_final with a larger nBits.",
            plonk_result.n_bits,
            vadcop_n_bits,
        );
    }

    let fixed_bin_path = build_path.join(format!("{}.fixed.bin", template));
    let fixed_info: Vec<(String, Vec<u32>, Vec<u64>)> = plonk_result
        .fixed_pols
        .iter()
        .map(|fp| (fp.name.clone(), vec![fp.index as u32], fp.values.clone()))
        .collect();
    fixed_cols::write_fixed_pols_bin(
        fixed_bin_path.to_str().unwrap(),
        &plonk_result.airgroup_name,
        &plonk_result.air_name,
        1u64 << plonk_result.n_bits,
        &fixed_info,
    )?;

    let pil_path = pil_dir.join(format!("{}.pil", template));
    fs::write(&pil_path, &plonk_result.pil_str)?;

    let exec_path = artifacts.exec_path();
    let exec_bytes: Vec<u8> = plonk_result.exec.iter().flat_map(|v| v.to_le_bytes()).collect();
    fs::write(&exec_path, &exec_bytes)?;

    // .starkinfo.json / .bin are borrowed from vadcop_final at register time;
    // only .const + .verkey.{json,bin} need to land at the recurser stem.
    let pilout_path = build_path.join(format!("{}.pilout", template));
    compile_pil(
        pil_path.to_str().unwrap(),
        pilout_path.to_str().unwrap(),
        config.std_pil_path,
        config.recurser_pil_path,
    )
    .context("compile_pil failed for recurser")?;

    let pilout_proxy = PilOutProxy::new(pilout_path.to_str().unwrap())
        .map_err(|e| anyhow::anyhow!("Failed to load recurser pilout: {e}"))?;
    let pilout = &pilout_proxy.pilout;
    if pilout.air_groups.is_empty() || pilout.air_groups[0].airs.is_empty() {
        bail!("recurser pilout has no AIR groups: {:?}", pilout_path);
    }
    let air = &pilout.air_groups[0].airs[0];

    let const_path = artifacts.const_path();
    let plonk_values =
        fixed_cols::reorder_plonk_pols_for_pilout(&plonk_result.fixed_pols, &pilout.symbols, 0, 0);
    fixed_cols::write_const_file(const_path.to_str().unwrap(), air, &plonk_values)
        .context("write_const_file failed for recurser_aggregator")?;

    // Compute the verkey. proofman generates the `.consttree` itself at register
    // time (its `calculate_fixed_tree` writes-or-loads the tree), so setup only
    // needs to land the verkey here.
    let verkey_json_path = artifacts.verkey_json_path();
    let const_root = bctree::compute_const_tree(
        const_path.to_str().unwrap(),
        config.vadcop_final_starkinfo_path,
        verkey_json_path.to_str().unwrap(),
    );
    let verkey_bin: Vec<u8> = const_root.iter().flat_map(|v| v.to_le_bytes()).collect();
    fs::write(artifacts.verkey_bin_path(), &verkey_bin)?;

    witness_tracker.await_all()?;
    Ok(())
}
