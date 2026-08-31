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
    pub(crate) templates: zisk_recurser::CircomTemplates,
    // SDK-managed paths — not exposed to the user.
    pub(crate) setup_dir: String,
    /// The proving key's hash family, captured at build time from the same
    /// globalInfo.json the `recurser_id` was derived against.
    pub(crate) hash_mode: HashMode,
    pub(crate) output_dir: String,
    pub(crate) vk_cache: Arc<OnceLock<ProgramVK>>,
}

impl Recurser {
    /// Content-addressed identifier; stable across runs for identical inputs.
    pub fn recurser_id(&self) -> &str {
        &self.recurser_id
    }

    /// Unified number of free values per side. It is the width of the single
    /// free array the caller supplies per proof: on a leaf it is normalized
    /// internally (free_in), on an aggregated proof it is used directly
    /// (free_out). 0 when the recurser declares no free values.
    pub fn n_free(&self) -> usize {
        self.templates.n_free()
    }

    /// 4-limb verification key. Available only after `client.setup(&agg).run()`
    /// has completed. Cached internally.
    pub fn vk(&self) -> Result<ProgramVK> {
        if let Some(vk) = self.vk_cache.get() {
            return Ok(vk.clone());
        }
        let artifacts = zisk_recurser::RecurserArtifacts::new(&self.output_dir, &self.recurser_id);
        let limbs = artifacts.read_verkey().map_err(|e| {
            SdkError::Recurser(format!(
                "failed to read recurser verkey ({e}). \
                 Did `client.setup(&agg).run()` complete?"
            ))
        })?;
        let vk = ProgramVK { vk: limbs.to_vec(), hash_mode: self.hash_mode };
        let _ = self.vk_cache.set(vk.clone());
        Ok(vk)
    }
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

/// Derive the 4-limb VK of each allow-listed guest program, in order, with a
/// duplicate check. Empty input → empty output (the VK-agnostic default), so
/// callers pay the ELF-reading cost only when they supply an allow-list.
///
/// Derived under the proving key's `hash_mode`; a verkey is only valid relative
/// to one. Duplicates are rejected: the circuit's membership check
/// (`1 - ∏(1-eq)`) assumes the baked `programVKs[]` are unique.
fn derive_program_vks(programs: &[&GuestProgram], hash_mode: HashMode) -> Result<Vec<[String; 4]>> {
    let mut vks: Vec<[String; 4]> = Vec::with_capacity(programs.len());
    for prog in programs {
        let pvk = prog.vk_with_mode(hash_mode).map_err(|e| {
            SdkError::Recurser(format!("failed to derive VK for program '{}': {e}", prog.name()))
        })?;
        let limbs: [u64; 4] = <[u64; 4]>::try_from(pvk.vk.as_slice()).map_err(|_| {
            SdkError::Recurser(format!(
                "program VK for '{}' did not decode into 4 u64 limbs",
                prog.name()
            ))
        })?;
        let limbs_str: [String; 4] = limbs.map(|w| w.to_string());
        if let Some(prior) = vks.iter().position(|existing| existing == &limbs_str) {
            return Err(SdkError::Recurser(format!(
                "duplicate program VK at index {} ('{}'); already registered at index {}",
                vks.len(),
                prog.name(),
                prior,
            )));
        }
        vks.push(limbs_str);
    }
    Ok(vks)
}

/// Client-independent builder for a [`Recurser`] — the proof-folding
/// sibling of [`GuestProgram`].
///
/// Most users never construct this directly: [`load_aggregation_program!`](crate::load_aggregation_program)
/// expands a TOML definition into exactly this builder call. Build it by
/// hand when the circuits are only known at runtime.
///
/// ```ignore
/// let recurser = AggregationProgramBuilder::new(load_circuit!("aggregate.circom"), 6)
///     .normalize(load_circuit!("normalize.circom"))
///     .free_inputs(1)
///     .build()?;
/// client.setup(&recurser).run()?.await?;
/// ```
pub struct AggregationProgramBuilder<'a> {
    aggregate: CircomCircuit,
    n_free: usize,
    /// Number of publics slots the aggregation populates (required, set at
    /// [`new`](Self::new)). `AggregatePublics` outputs a `n_publics_agg`-wide
    /// array; the generator zero-fills the remaining tail.
    n_publics_agg: usize,
    normalize: Option<CircomCircuit>,
    /// Optional leaf allow-list. Empty = VK-agnostic (any valid leaf accepted).
    /// Order is significant (fixes each program's `programVKs[]` index).
    programs: Vec<&'a GuestProgram>,
}

impl<'a> AggregationProgramBuilder<'a> {
    /// `aggregate` is the `AggregatePublics` Circom body: the consistency
    /// constraints between the two folded proofs' publics plus the merge
    /// into the output publics.
    ///
    /// `n_publics_agg` is the number of publics slots the aggregation populates:
    /// `AggregatePublics` outputs a `n_publics_agg`-wide array (slots
    /// `[0, n_publics_agg)`) and the recurser scaffolding zero-fills the remaining
    /// `[n_publics_agg, ZISK_PUBLICS())` tail, so the circuit never writes a
    /// padding loop. Must be in `1..=ZISK_PUBLICS()` (checked at [`build`](Self::build)).
    pub fn new(aggregate: impl Into<CircomCircuit>, n_publics_agg: usize) -> Self {
        Self {
            aggregate: aggregate.into(),
            n_free: 0,
            n_publics_agg,
            normalize: None,
            programs: Vec::new(),
        }
    }

    /// Optional leaf allow-list (access control). When set, the circuit accepts
    /// only these guest programs' proofs as raw vadcop_final leaves; a leaf with
    /// any other programVK is rejected (the proof is unsatisfiable). Omit (or
    /// pass an empty slice) to accept any valid leaf. Order is significant and
    /// must stay stable across runs — it is part of the `recurser_id` digest.
    ///
    /// Deriving each program's VK reads its ELF, so this makes `build()` heavier
    /// than the VK-agnostic default; the cost is only paid when an allow-list is
    /// supplied.
    #[must_use]
    pub fn programs(mut self, programs: &[&'a GuestProgram]) -> Self {
        self.programs = programs.to_vec();
        self
    }

    /// Unified number of free values per side (defaults to 0). On a leaf the
    /// `NormalizePublics` circuit consumes this many free inputs and emits the
    /// same number of free outputs; on an aggregated proof `AggregatePublics`
    /// reads them directly. Same width for both stages.
    #[must_use]
    pub fn free_inputs(mut self, n_free: usize) -> Self {
        self.n_free = n_free;
        self
    }

    /// Attach the single optional `NormalizePublics` circuit (runs on all leaves).
    #[must_use]
    pub fn normalize(mut self, circuit: impl Into<CircomCircuit>) -> Self {
        self.normalize = Some(circuit.into());
        self
    }

    /// Resolves the inputs into a [`Recurser`]. Cheap: computes the
    /// content-addressed `recurser_id`. Reads this machine's local
    /// vadcop_final verkey — even when proving remotely it must match the
    /// workers' copy or `recurser_id` diverges.
    pub fn build(self) -> Result<Recurser> {
        expect_template_decl(&self.aggregate, "AggregatePublics")?;
        if let Some(circuit) = &self.normalize {
            expect_template_decl(circuit, "NormalizePublics")?;
        }

        // Range-check here so a bad width fails at the client boundary rather
        // than deep in setup after `recurser_id` is already computed/registered.
        let n_publics_agg = self.n_publics_agg;
        let max_publics = zisk_recurser::templates::ZISK_PUBLICS;
        if n_publics_agg == 0 || n_publics_agg > max_publics {
            return Err(SdkError::Recurser(format!(
                "n_publics_agg must be in 1..={max_publics}, got {n_publics_agg}"
            )));
        }

        let normalize = self
            .normalize
            .as_ref()
            .map(|c| zisk_recurser::NormalizeCircuit { body: c.source().to_string() });

        let setup_dir = ZiskPaths::global()
            .home
            .to_str()
            .ok_or_else(|| SdkError::Recurser("default ~/.zisk path is not valid UTF-8".into()))?
            .to_string();

        // Derive the optional leaf allow-list VKs. Only touches ELFs when a
        // `programs` list was supplied — the VK-agnostic default stays cheap.
        let hash_mode = HashMode::local().map_err(SdkError::backend)?;
        let program_vks = derive_program_vks(&self.programs, hash_mode)?;
        let templates = zisk_recurser::CircomTemplates {
            normalize: normalize.clone(),
            aggregate_publics: self.aggregate.source().to_string(),
            n_free: self.n_free,
            n_publics_agg,
            program_vks: program_vks.clone(),
        };

        let output_dir = ZiskPaths::global()
            .home
            .join("recurser")
            .to_str()
            .ok_or_else(|| SdkError::Recurser("~/.zisk/recurser path is not valid UTF-8".into()))?
            .to_string();

        let zisk_vk = zisk_recurser::setup::read_vadcop_final_verkey(&setup_dir).map_err(|e| {
            SdkError::Recurser(format!(
                "failed to locate local vadcop_final verkey ({e}). \
                 Run `cargo-zisk setup --recursive` on this machine \
                 (required even when using a remote coordinator)."
            ))
        })?;

        let inputs = zisk_recurser::RecurserManifestInputs::new(
            zisk_vk,
            program_vks,
            normalize.as_ref(),
            &templates.aggregate_publics,
            templates.n_free,
            templates.n_publics_agg,
        );
        let recurser_id = inputs.compute_id();

        Ok(Recurser {
            recurser_id,
            templates,
            setup_dir,
            hash_mode,
            output_dir,
            vk_cache: Arc::new(OnceLock::new()),
        })
    }
}

/// A lazily-built [`Recurser`] for module-level declaration via
/// [`load_aggregation_program!`](crate::load_aggregation_program). Derefs to [`Recurser`], so a `static` of
/// this type is used exactly like a `Recurser` reference.
pub struct AggregationProgram(std::sync::LazyLock<Recurser>);

impl AggregationProgram {
    /// Used by [`load_aggregation_program!`](crate::load_aggregation_program); `init` runs on first use.
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
/// `build_program` resolves at host-build time (circuit bodies embedded) into
/// the [`AggregationProgramBuilder`] call this macro expands to.
///
/// ```ignore
/// static AGG: AggregationProgram = load_aggregation_program!("my_aggregation");
/// ```
///
/// The build is lazy — it runs on first use, because it does runtime work
/// (reads the local vadcop_final verkey) that can't happen in a `const`. A
/// build failure panics; for fallible handling, construct an
/// [`AggregationProgramBuilder`] yourself and call
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
            templates: zisk_recurser::CircomTemplates {
                normalize: None,
                aggregate_publics: "// body".into(),
                n_free: 0,
                n_publics_agg: 6,
                program_vks: vec![],
            },
            setup_dir: "/tmp/zisk-test-setup".into(),
            hash_mode: HashMode::default(),
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
