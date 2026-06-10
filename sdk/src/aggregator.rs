//! Recurser-aggregator handle and the builder that constructs it.
//!
//! A [`RecurserAggregator`] is the sibling of [`GuestProgram`] for proof
//! aggregation: it identifies a content-addressed recurser setup and flows
//! through `client.upload()` / `client.setup()` / `client.aggregate_proof()`.

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use anyhow::{anyhow, bail, Context, Result};
use recurser::manifest::RecurserManifestInputs;
use serde_json::Value;
use zisk_common::{ProgramVK, ZiskPaths};
use zisk_prover_backend::GuestProgram;

use crate::Client;

/// Handle to a recurser-aggregator. Cheap to clone (paths plus an `Arc`-shared
/// VK cache). Heavy setup artifacts live on disk under `output_dir`.
#[derive(Clone)]
pub struct RecurserAggregator {
    pub(crate) recurser_id: String,
    pub(crate) program_vks: Vec<[String; 4]>,
    pub(crate) n_private_inputs: usize,
    pub(crate) prepare_publics_template: Option<String>,
    pub(crate) check_publics_template: Option<String>,
    pub(crate) aggregate_publics_template: String,
    // SDK-managed paths — not exposed to the user.
    pub(crate) setup_dir: String,
    pub(crate) output_dir: String,
    pub(crate) vk_cache: Arc<OnceLock<ProgramVK>>,
}

impl RecurserAggregator {
    /// Content-addressed identifier; stable across runs for identical inputs.
    pub fn recurser_id(&self) -> &str {
        &self.recurser_id
    }

    /// 4-limb verification key. Available only after `client.setup(&agg).run()`
    /// has completed. Cached internally.
    pub fn vk(&self) -> Result<ProgramVK> {
        if let Some(vk) = self.vk_cache.get() {
            return Ok(vk.clone());
        }
        let stem =
            recurser_setup_dir(&self.output_dir, &self.recurser_id).join("recurser_aggregator");
        let limbs = read_verkey_bin_4(&stem).with_context(|| {
            format!(
                "Failed to read recurser-aggregator verkey at {}. Did `client.setup(&agg).run()` complete?",
                stem.with_extension("verkey.bin").display()
            )
        })?;
        let vk = ProgramVK { vk: limbs.to_vec() };
        let _ = self.vk_cache.set(vk.clone());
        Ok(vk)
    }
}

pub(crate) fn recurser_setup_dir(output_dir: &str, recurser_id: &str) -> PathBuf {
    PathBuf::from(output_dir).join("provingKey").join("recurser").join(recurser_id)
}

fn read_verkey_bin_4(stem: &std::path::Path) -> Result<[u64; 4]> {
    use std::io::Read;
    let mut path = stem.as_os_str().to_owned();
    path.push(".verkey.bin");
    let mut file = std::fs::File::open(std::path::Path::new(&path))
        .with_context(|| format!("Failed to open {:?}", path))?;
    let mut bytes = [0u8; 32];
    file.read_exact(&mut bytes)
        .with_context(|| format!("Failed to read 32 bytes from {:?}", path))?;
    let mut limbs = [0u64; 4];
    for i in 0..4 {
        let chunk: [u8; 8] = bytes[i * 8..(i + 1) * 8].try_into().unwrap();
        limbs[i] = u64::from_le_bytes(chunk);
    }
    Ok(limbs)
}

/// Builder returned by `client.register_setup_aggregation(...)`.
pub struct RegisterAggregationRequest<'a, C> {
    client: &'a C,
    programs: &'a [&'a GuestProgram],
    aggregate_publics_template: Option<String>,
    prepare_publics_template: Option<String>,
    check_publics_template: Option<String>,
    n_private_inputs: usize,
}

#[allow(private_bounds)]
impl<'a, C: Client> RegisterAggregationRequest<'a, C> {
    pub(crate) fn new(client: &'a C, programs: &'a [&'a GuestProgram]) -> Self {
        Self {
            client,
            programs,
            aggregate_publics_template: None,
            prepare_publics_template: None,
            check_publics_template: None,
            n_private_inputs: 0,
        }
    }

    /// `AggregatePublics` Circom body. Required.
    #[must_use]
    pub fn aggregate_template(mut self, body: impl Into<String>) -> Self {
        self.aggregate_publics_template = Some(body.into());
        self
    }

    /// `PreparePublics` Circom body. Defaults to the built-in identity.
    #[must_use]
    pub fn prepare_template(mut self, body: impl Into<String>) -> Self {
        self.prepare_publics_template = Some(body.into());
        self
    }

    /// `CheckPublics` Circom body. Defaults to the built-in no-op.
    #[must_use]
    pub fn check_template(mut self, body: impl Into<String>) -> Self {
        self.check_publics_template = Some(body.into());
        self
    }

    /// Number of private inputs threaded into all three sub-templates. Default 0.
    #[must_use]
    pub fn n_private_inputs(mut self, n: usize) -> Self {
        self.n_private_inputs = n;
        self
    }

    /// Resolves the inputs into a [`RecurserAggregator`]. Cheap: derives each
    /// program's 4-limb VK and computes the content-addressed `recurser_id`.
    /// Even on remote, reads the SDK process's local vadcop_final verkey —
    /// it must match the workers' copy or `recurser_id` will diverge.
    pub fn run(self) -> Result<RecurserAggregator> {
        let _client = self.client; // captured for API parity; construction is pure data
        let aggregate_publics_template = self
            .aggregate_publics_template
            .ok_or_else(|| anyhow!("aggregate_template(...) is required"))?;
        if self.programs.is_empty() {
            bail!("at least one program is required");
        }

        let setup_dir = ZiskPaths::global()
            .home
            .to_str()
            .context("default ~/.zisk path is not valid UTF-8")?
            .to_string();
        let output_dir = ZiskPaths::global()
            .home
            .join("recurser")
            .to_str()
            .context("~/.zisk/recurser path is not valid UTF-8")?
            .to_string();

        let zisk_vk = read_vadcop_final_verkey(&setup_dir).with_context(|| {
            "Failed to locate local vadcop_final verkey. Run `cargo-zisk setup --recursive` \
             on this machine (required even when using a remote coordinator)."
        })?;

        let mut program_vks: Vec<[String; 4]> = Vec::with_capacity(self.programs.len());
        for prog in self.programs {
            let pvk = prog
                .vk()
                .with_context(|| format!("Failed to derive VK for program '{}'", prog.name()))?;
            let limbs: [u64; 4] = <[u64; 4]>::try_from(pvk.vk.as_slice()).map_err(|_| {
                anyhow!("Program VK for '{}' did not decode into 4 u64 limbs", prog.name())
            })?;
            let limbs_str: [String; 4] = limbs.map(|w| w.to_string());
            if let Some(prior_idx) = program_vks.iter().position(|existing| existing == &limbs_str)
            {
                bail!(
                    "Duplicate program VK at index {} ('{}'); already registered at index {} ('{}')",
                    program_vks.len(),
                    prog.name(),
                    prior_idx,
                    self.programs[prior_idx].name(),
                );
            }
            program_vks.push(limbs_str);
        }

        // Resolve template defaults here so the manifest hash matches the recurser crate's.
        let resolved_prepare = self
            .prepare_publics_template
            .as_deref()
            .unwrap_or(recurser::templates::DEFAULT_PREPARE_PUBLICS);
        let resolved_check = self
            .check_publics_template
            .as_deref()
            .unwrap_or(recurser::templates::DEFAULT_CHECK_PUBLICS);

        let inputs = RecurserManifestInputs::new(
            zisk_vk,
            self.n_private_inputs,
            program_vks.clone(),
            resolved_prepare,
            resolved_check,
            aggregate_publics_template.as_str(),
        );
        let recurser_id = inputs.compute_id();

        Ok(RecurserAggregator {
            recurser_id,
            program_vks,
            n_private_inputs: self.n_private_inputs,
            prepare_publics_template: self.prepare_publics_template,
            check_publics_template: self.check_publics_template,
            aggregate_publics_template,
            setup_dir,
            output_dir,
            vk_cache: Arc::new(OnceLock::new()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_agg() -> RecurserAggregator {
        RecurserAggregator {
            recurser_id: "rid".into(),
            program_vks: vec![],
            n_private_inputs: 0,
            prepare_publics_template: None,
            check_publics_template: None,
            aggregate_publics_template: "// body".into(),
            setup_dir: "/tmp/zisk-test-setup".into(),
            output_dir: "/tmp/zisk-test-output".into(),
            vk_cache: Arc::new(OnceLock::new()),
        }
    }

    /// `vk_cache` must be shared across clones — the remote `setup.rs` hook
    /// writes the cache via a cloned handle; the user's original must observe it.
    #[test]
    fn vk_cache_is_shared_across_clones() {
        let agg = dummy_agg();
        let agg_clone = agg.clone();

        let _ = agg_clone.vk_cache.set(ProgramVK { vk: vec![1, 2, 3, 4] });

        assert_eq!(agg.vk_cache.get().map(|v| v.vk.clone()), Some(vec![1, 2, 3, 4]));
        assert_eq!(agg_clone.vk_cache.get().map(|v| v.vk.clone()), Some(vec![1, 2, 3, 4]));

        // OnceLock: second set is rejected.
        assert!(agg.vk_cache.set(ProgramVK { vk: vec![9, 9, 9, 9] }).is_err());
        assert_eq!(agg.vk_cache.get().map(|v| v.vk.clone()), Some(vec![1, 2, 3, 4]));
    }
}

/// Reads the vadcop_final verkey as 4 decimal-string limbs.
fn read_vadcop_final_verkey(setup_dir: &str) -> Result<[String; 4]> {
    let global_info_path =
        PathBuf::from(setup_dir).join("provingKey").join("pilout.globalInfo.json");
    let global_info: Value =
        serde_json::from_str(&std::fs::read_to_string(&global_info_path).with_context(|| {
            format!("Failed to read global info at {}", global_info_path.display())
        })?)?;
    let name = global_info.get("name").and_then(|v| v.as_str()).unwrap_or("pilout").to_string();

    let verkey_path = PathBuf::from(setup_dir)
        .join("provingKey")
        .join(&name)
        .join("vadcop_final")
        .join("vadcop_final.verkey.json");
    let s = std::fs::read_to_string(&verkey_path)
        .with_context(|| format!("Failed to read verkey at {}", verkey_path.display()))?;
    let v: Vec<Value> = serde_json::from_str(&s)
        .with_context(|| format!("Failed to parse verkey JSON at {}", verkey_path.display()))?;
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
