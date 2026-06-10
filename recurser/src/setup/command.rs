use std::fs;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use serde_json::Value;

use pil2_stark_setup::output::witness_gen::WitnessTracker;

use super::proving_key::{gen_recurser_aggregator_setup, RecurserAggregatorConfig};
use super::resolve::{resolve_circom_exec, resolve_path_env};
use crate::manifest::{write_manifest_and_templates, RecurserManifest, RecurserManifestInputs};
use crate::templates::{DEFAULT_CHECK_PUBLICS, DEFAULT_PREPARE_PUBLICS};
use crate::CircomTemplates;

pub struct SetupRecurserAggregatorOptions {
    /// ZisK setup directory containing `provingKey/<name>/vadcop_final/`.
    pub setup_dir: String,
    /// Where to write the generated artifacts. Must differ from `setup_dir`.
    pub output_dir: String,
    /// Registered program VKs (4 Goldilocks limbs each, decimal strings).
    pub program_vks: Vec<[String; 4]>,
    /// Number of side inputs threaded into the user's `PreparePublics`.
    pub n_private_inputs: usize,
    /// Path to a `PreparePublics` Circom body. `None` uses the identity default.
    pub prepare_publics_template: Option<String>,
    /// Path to a `CheckPublics` Circom body. `None` uses the no-op default.
    pub check_publics_template: Option<String>,
    /// Path to the `AggregatePublics` Circom body (required).
    pub aggregate_publics_template: String,
}

pub fn run_setup_recurser_aggregator(opts: &SetupRecurserAggregatorOptions) -> Result<()> {
    let setup_dir = &opts.setup_dir;
    let output_dir = &opts.output_dir;

    if setup_dir == output_dir {
        bail!("setup_dir and output_dir must differ (got {:?})", setup_dir);
    }

    let global_info_path =
        PathBuf::from(setup_dir).join("provingKey").join("pilout.globalInfo.json");
    if !global_info_path.exists() {
        bail!("Global info file not found: {:?}. Run `setup --recursive` first.", global_info_path);
    }
    let global_info: Value = serde_json::from_str(&fs::read_to_string(&global_info_path)?)?;
    let name = global_info.get("name").and_then(|v| v.as_str()).unwrap_or("pilout").to_string();
    let hash = global_info.get("hash").and_then(|v| v.as_str()).unwrap_or("Poseidon1").to_string();

    let vadcop_final_dir =
        PathBuf::from(setup_dir).join("provingKey").join(&name).join("vadcop_final");
    let verkey_path = vadcop_final_dir.join("vadcop_final.verkey.json");
    let starkinfo_path = vadcop_final_dir.join("vadcop_final.starkinfo.json");
    let verifier_info_path = vadcop_final_dir.join("vadcop_final.verifierinfo.json");
    for p in [&verkey_path, &starkinfo_path, &verifier_info_path] {
        if !p.exists() {
            bail!("Required file not found: {:?}. Run `setup-final` first.", p);
        }
    }

    let zisk_vk =
        parse_verkey_4(&verkey_path).context("Failed to parse vadcop_final.verkey.json")?;
    let stark_info: Value = serde_json::from_str(&fs::read_to_string(&starkinfo_path)?)?;
    let verifier_info: Value = serde_json::from_str(&fs::read_to_string(&verifier_info_path)?)?;

    if opts.program_vks.is_empty() {
        bail!("program_vks must contain at least one entry");
    }
    let program_vks = &opts.program_vks;

    let load_optional = |opt: &Option<String>, name: &str| -> Result<Option<String>> {
        match opt {
            Some(path) => Ok(Some(
                fs::read_to_string(path)
                    .with_context(|| format!("Failed to read {}: {}", name, path))?,
            )),
            None => Ok(None),
        }
    };
    let circom_templates = CircomTemplates {
        prepare_publics: load_optional(&opts.prepare_publics_template, "prepare_publics_template")?,
        check_publics: load_optional(&opts.check_publics_template, "check_publics_template")?,
        aggregate_publics: fs::read_to_string(&opts.aggregate_publics_template).with_context(
            || {
                format!(
                    "Failed to read aggregate_publics_template: {}",
                    opts.aggregate_publics_template
                )
            },
        )?,
    };

    // Resolve defaults up-front so the manifest hashes match the aggregator's
    // actual inputs, not the caller's None-vs-Some distinction.
    let resolved_prepare =
        circom_templates.prepare_publics.as_deref().unwrap_or(DEFAULT_PREPARE_PUBLICS);
    let resolved_check = circom_templates.check_publics.as_deref().unwrap_or(DEFAULT_CHECK_PUBLICS);
    let resolved_aggregate = circom_templates.aggregate_publics.as_str();

    let manifest_inputs = RecurserManifestInputs::new(
        zisk_vk.clone(),
        opts.n_private_inputs,
        program_vks.clone(),
        resolved_prepare,
        resolved_check,
        resolved_aggregate,
    );
    let recurser_id = manifest_inputs.compute_id();
    tracing::info!("Recurser id: {}", recurser_id);

    let circuits_gl_path = resolve_path_env(
        "CIRCUITS_GL_PATH",
        "setup/stark-recurser/stark2circom/circom_verifier/circuits.gl",
    );
    let recurser_circuits_path = resolve_path_env(
        "RECURSER_CIRCUITS_COMPRESSED_FINAL_PATH",
        "setup/stark-recurser/stark2circom/circom_verifier/helper_circuits",
    );
    let circom_helpers_dir = resolve_path_env("CIRCOM_HELPERS_DIR", "setup/circom");
    let goldilocks_src_dir =
        resolve_path_env("GOLDILOCKS_SRC_DIR", "pil2-stark/src/goldilocks/src");
    let std_pil_path = resolve_path_env("STD_PIL_PATH", "pil2-components/lib/std/pil");
    let recurser_pil_path =
        resolve_path_env("RECURSER_PIL_PATH", "setup/stark-recurser/plonk2pil/pil");
    let circom_exec = resolve_circom_exec(&circom_helpers_dir);
    let witness_tracker = WitnessTracker::with_goldilocks_src(&goldilocks_src_dir);

    let vadcop_final_starkinfo_path =
        starkinfo_path.to_str().context("vadcop_final starkinfo path is not valid UTF-8")?;

    let config = RecurserAggregatorConfig {
        output_dir,
        recurser_id: &recurser_id,
        hash: &hash,
        zisk_vk: &zisk_vk,
        stark_info: &stark_info,
        verifier_info: &verifier_info,
        n_private_inputs: opts.n_private_inputs,
        program_vks,
        circom_templates: &circom_templates,
        circom_exec: &circom_exec,
        circuits_gl_path: &circuits_gl_path,
        recurser_circuits_path: &recurser_circuits_path,
        circom_helpers_dir: &circom_helpers_dir,
        std_pil_path: &std_pil_path,
        recurser_pil_path: &recurser_pil_path,
        vadcop_final_starkinfo_path,
    };

    tracing::info!("Running recurser-aggregator setup for '{}'", name);
    gen_recurser_aggregator_setup(&config, &witness_tracker)
        .context("Recurser-aggregator setup failed")?;
    witness_tracker.await_all()?;

    let files_dir =
        PathBuf::from(output_dir).join("provingKey").join("recurser").join(&recurser_id);
    let manifest = RecurserManifest { recurser_id: recurser_id.clone(), inputs: manifest_inputs };
    write_manifest_and_templates(
        &files_dir,
        &manifest,
        resolved_prepare,
        resolved_check,
        resolved_aggregate,
    )
    .context("Failed to write recurser manifest")?;

    tracing::info!("Recurser-aggregator setup complete");
    Ok(())
}

fn parse_verkey_4(path: &std::path::Path) -> Result<[String; 4]> {
    let s =
        fs::read_to_string(path).with_context(|| format!("Failed to read verkey: {:?}", path))?;
    let v: Vec<Value> = serde_json::from_str(&s)
        .with_context(|| format!("Failed to parse verkey JSON: {:?}", path))?;
    if v.len() != 4 {
        bail!("verkey.json has {} elements, expected 4", v.len());
    }
    let to_str = |i: usize, e: &Value| -> Result<String> {
        if let Some(s) = e.as_str() {
            Ok(s.to_string())
        } else if let Some(n) = e.as_u64() {
            Ok(n.to_string())
        } else {
            bail!("verkey.json element {} is not a number or string: {}", i, e)
        }
    };
    Ok([to_str(0, &v[0])?, to_str(1, &v[1])?, to_str(2, &v[2])?, to_str(3, &v[3])?])
}
