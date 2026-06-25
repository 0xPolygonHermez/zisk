//! Recurser handle and the builder that constructs it.
//!
//! A [`Recurser`] is the sibling of [`GuestProgram`] for proof
//! folding: it identifies a content-addressed recurser setup and flows
//! through `client.upload()` / `client.setup()` / `client.aggregate_proofs()`.
//! It is built client-independently via [`AggregationProgram`].

use std::sync::{Arc, OnceLock};

use zisk_common::{HashMode, ProgramVK, ZiskPaths};
use zisk_prover_backend::{CircomCircuit, GuestProgram};

use crate::{Result, SdkError};

/// Handle to a recurser. Cheap to clone (paths plus an `Arc`-shared
/// VK cache). Heavy setup artifacts live on disk under `output_dir`.
#[derive(Clone)]
pub struct Recurser {
    pub(crate) recurser_id: String,
    pub(crate) program_vks: Vec<[String; 4]>,
    pub(crate) templates: recurser::CircomTemplates,
    // SDK-managed paths — not exposed to the user.
    pub(crate) setup_dir: String,
    pub(crate) output_dir: String,
    pub(crate) vk_cache: Arc<OnceLock<ProgramVK>>,
}

impl Recurser {
    /// Content-addressed identifier; stable across runs for identical inputs.
    pub fn recurser_id(&self) -> &str {
        &self.recurser_id
    }

    /// Size of the circuit's per-side free-input arrays (the aggregate
    /// stage's free-input count).
    pub fn n_free_inputs(&self) -> usize {
        self.templates.max_free_inputs()
    }

    /// 4-limb verification key. Available only after `client.setup(&agg).run()`
    /// has completed. Cached internally.
    pub fn vk(&self) -> Result<ProgramVK> {
        if let Some(vk) = self.vk_cache.get() {
            return Ok(vk.clone());
        }
        let artifacts = recurser::RecurserArtifacts::new(&self.output_dir, &self.recurser_id);
        let limbs = artifacts.read_verkey().map_err(|e| {
            SdkError::Recurser(format!(
                "failed to read recurser verkey ({e}). \
                 Did `client.setup(&agg).run()` complete?"
            ))
        })?;
        // The hash family is a property of the proving key the recurser was set
        // up against; read it from the same globalInfo.json the setup did so the
        // verkey's mode matches the proofs it will be verified against.
        let hash_mode = read_setup_hash_mode(&self.setup_dir)?;
        let vk = ProgramVK { vk: limbs.to_vec(), hash_mode };
        let _ = self.vk_cache.set(vk.clone());
        Ok(vk)
    }
}

/// Read the recurser's hash family from the proving key's `globalInfo.json`,
/// the same source `run_setup_recurser_aggregator` uses.
fn read_setup_hash_mode(setup_dir: &str) -> Result<HashMode> {
    recurser::setup::read_proving_key_hash(setup_dir)
        .map_err(SdkError::backend)?
        .parse::<HashMode>()
        .map_err(SdkError::backend)
}

/// The body must declare `template <name>(...)` exactly once — the same check
/// the TOML resolver applies at host-build time (`ziskbuild::aggregation`).
fn expect_template_decl(circuit: &CircomCircuit, template: &str) -> Result<()> {
    let needle = format!("template {template}(");
    match circuit.source().matches(&needle).count() {
        1 => Ok(()),
        n => Err(SdkError::Recurser(format!(
            "circuit '{}' must define `template {template}(...)` exactly once, found {n}",
            circuit.name(),
        ))),
    }
}

/// Client-independent builder for a [`Recurser`] — the proof-folding
/// sibling of [`GuestProgram`].
///
/// Most users never construct this directly: [`load_aggregation_program!`]
/// expands a TOML definition into exactly this builder call. Build it by
/// hand when the program set or circuits are only known at runtime.
///
/// All registered guests must emit the same publics layout — the recurser
/// folds publics through raw, with no per-program canonicalization.
///
/// ```ignore
/// let recurser = AggregationProgramBuilder::new(&[&PROG_A, &PROG_B], load_circuit!("aggregate.circom"))
///     .build()?;
/// client.setup(&recurser).run()?.await?;
/// ```
pub struct AggregationProgramBuilder<'a> {
    guests: Vec<&'a GuestProgram>,
    aggregate: CircomCircuit,
    aggregate_n_free_inputs: usize,
}

impl<'a> AggregationProgramBuilder<'a> {
    /// `guests` is the full leaf allowlist — order is significant, it fixes
    /// each program's `programVKs[]` index, so keep it stable across runs.
    /// `aggregate` is the `AggregatePublics` Circom body: the consistency
    /// constraints between the two folded proofs' publics plus the merge
    /// into the output publics.
    pub fn new(guests: &[&'a GuestProgram], aggregate: impl Into<CircomCircuit>) -> Self {
        Self { guests: guests.to_vec(), aggregate: aggregate.into(), aggregate_n_free_inputs: 0 }
    }

    /// Number of prover-supplied side inputs the `AggregatePublics` circuit
    /// reads directly (defaults to 0). Use this for hash-style publics: feed
    /// each side's preimage as free inputs so `AggregatePublics` can check the
    /// hash, recombine the preimages, and re-hash. Sizes the recurser's
    /// per-side free-input array.
    #[must_use]
    pub fn aggregate_free_inputs(mut self, n_free_inputs: usize) -> Self {
        self.aggregate_n_free_inputs = n_free_inputs;
        self
    }

    /// Resolves the inputs into a [`Recurser`]. Cheap: derives each
    /// program's 4-limb VK and computes the content-addressed `recurser_id`.
    /// Reads this machine's local vadcop_final verkey — even when proving
    /// remotely it must match the workers' copy or `recurser_id` diverges.
    pub fn build(self) -> Result<Recurser> {
        if self.guests.is_empty() {
            return Err(SdkError::Recurser("at least one guest program is required".into()));
        }

        expect_template_decl(&self.aggregate, "AggregatePublics")?;

        let templates = recurser::CircomTemplates {
            aggregate_publics: self.aggregate.source().to_string(),
            aggregate_n_free_inputs: self.aggregate_n_free_inputs,
        };

        let setup_dir = ZiskPaths::global()
            .home
            .to_str()
            .ok_or_else(|| SdkError::Recurser("default ~/.zisk path is not valid UTF-8".into()))?
            .to_string();
        let output_dir = ZiskPaths::global()
            .home
            .join("recurser")
            .to_str()
            .ok_or_else(|| SdkError::Recurser("~/.zisk/recurser path is not valid UTF-8".into()))?
            .to_string();

        let zisk_vk = recurser::setup::read_vadcop_final_verkey(&setup_dir).map_err(|e| {
            SdkError::Recurser(format!(
                "failed to locate local vadcop_final verkey ({e}). \
                 Run `cargo-zisk setup --recursive` on this machine \
                 (required even when using a remote coordinator)."
            ))
        })?;

        let mut program_vks: Vec<[String; 4]> = Vec::with_capacity(self.guests.len());
        for prog in &self.guests {
            let pvk = prog.vk().map_err(|e| {
                SdkError::Recurser(format!(
                    "failed to derive VK for program '{}': {e}",
                    prog.name()
                ))
            })?;
            let limbs: [u64; 4] = <[u64; 4]>::try_from(pvk.vk.as_slice()).map_err(|_| {
                SdkError::Recurser(format!(
                    "program VK for '{}' did not decode into 4 u64 limbs",
                    prog.name()
                ))
            })?;
            let limbs_str: [String; 4] = limbs.map(|w| w.to_string());
            if let Some(prior_idx) = program_vks.iter().position(|existing| existing == &limbs_str)
            {
                return Err(SdkError::Recurser(format!(
                    "duplicate program VK at index {} ('{}'); already registered at index {} ('{}')",
                    program_vks.len(),
                    prog.name(),
                    prior_idx,
                    self.guests[prior_idx].name(),
                )));
            }
            program_vks.push(limbs_str);
        }

        // The shared constructor owns the hashing, so this id is
        // byte-identical to the one setup and the worker derive.
        let inputs = recurser::RecurserManifestInputs::new(
            zisk_vk,
            program_vks.clone(),
            &templates.aggregate_publics,
            templates.aggregate_n_free_inputs,
        );
        let recurser_id = inputs.compute_id();

        Ok(Recurser {
            recurser_id,
            program_vks,
            templates,
            setup_dir,
            output_dir,
            vk_cache: Arc::new(OnceLock::new()),
        })
    }
}

/// A lazily-built [`Recurser`] for module-level declaration via
/// [`load_aggregation_program!`]. Derefs to [`Recurser`], so a `static` of
/// this type is used exactly like a `Recurser` reference.
pub struct AggregationProgram(std::sync::LazyLock<Recurser>);

impl AggregationProgram {
    /// Used by [`load_aggregation_program!`]; `init` runs on first use.
    pub const fn new(init: fn() -> Recurser) -> Self {
        Self(std::sync::LazyLock::new(init))
    }
}

impl std::ops::Deref for AggregationProgram {
    type Target = Recurser;
    fn deref(&self) -> &Recurser {
        &self.0
    }
}

/// Declare a module-level aggregation program from its build-processed
/// definition, mirroring [`load_program!`] for guest programs.
///
/// The name is the file stem of `<programs>/aggregations/<name>.toml`, which
/// `build_program` resolves at host-build time (guest ELFs pinned, circuit
/// bodies embedded) into the [`AggregationProgramBuilder`] call this macro
/// expands to.
///
/// ```ignore
/// static AGG: AggregationProgram = load_aggregation_program!("my_aggregation");
/// ```
///
/// The build is lazy — it runs on first use, because it does runtime work
/// (reads the local vadcop_final verkey, derives program VKs) that can't
/// happen in a `const`. A build failure panics; for fallible handling,
/// construct an [`AggregationProgramBuilder`] yourself and call
/// [`AggregationProgramBuilder::build`].
///
/// [`load_program!`]: crate::load_program
#[macro_export]
macro_rules! load_aggregation_program {
    ($name:literal) => {{
        #[cfg(zisk_skip_guest_build)]
        {
            $crate::AggregationProgram::new(|| {
                panic!(concat!(
                    "aggregation program `",
                    $name,
                    "` is unavailable: the guest build was skipped"
                ))
            })
        }
        #[cfg(not(zisk_skip_guest_build))]
        {
            $crate::AggregationProgram::new(|| {
                include!(env!(
                    concat!("ZISK_AGG_", $name),
                    concat!(
                        "no aggregation program named `",
                        $name,
                        "` was processed by `build_program` — expected \
                         `<programs>/aggregations/",
                        $name,
                        ".toml` (after creating the aggregations dir, trigger \
                         one rebuild, e.g. touch build.rs)"
                    )
                ))
                .build()
                .expect(concat!(
                    "failed to build aggregation program `",
                    $name,
                    "`"
                ))
            })
        }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_agg() -> Recurser {
        Recurser {
            recurser_id: "rid".into(),
            program_vks: vec![],
            templates: recurser::CircomTemplates {
                aggregate_publics: "// body".into(),
                aggregate_n_free_inputs: 0,
            },
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

        let _ = agg_clone.vk_cache.set(ProgramVK { vk: vec![1, 2, 3, 4], ..Default::default() });

        assert_eq!(agg.vk_cache.get().map(|v| v.vk.clone()), Some(vec![1, 2, 3, 4]));
        assert_eq!(agg_clone.vk_cache.get().map(|v| v.vk.clone()), Some(vec![1, 2, 3, 4]));

        // OnceLock: second set is rejected.
        assert!(agg
            .vk_cache
            .set(ProgramVK { vk: vec![9, 9, 9, 9], ..Default::default() })
            .is_err());
        assert_eq!(agg.vk_cache.get().map(|v| v.vk.clone()), Some(vec![1, 2, 3, 4]));
    }
}
