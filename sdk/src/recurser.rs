//! Recurser handle and the builder that constructs it.
//!
//! A [`Recurser`] is the sibling of [`GuestProgram`] for proof
//! folding: it identifies a content-addressed recurser setup and flows
//! through `client.upload()` / `client.setup()` / `client.aggregate_proofs()`.
//! It is built client-independently via [`AggregationProgram`].

use std::sync::{Arc, OnceLock};

use anyhow::{anyhow, bail, Context, Result};
use zisk_common::{HashMode, ProgramVK, ZiskPaths};
use zisk_prover_backend::{CircomCircuit, GuestProgram};

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

    /// Size of the circuit's per-side free-input arrays: the worst case
    /// across normalization groups.
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
        let limbs = artifacts
            .read_verkey()
            .context("Failed to read recurser verkey. Did `client.setup(&agg).run()` complete?")?;
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
    recurser::setup::read_proving_key_hash(setup_dir)?.parse::<HashMode>()
}

/// The body must declare `template <name>(...)` exactly once — the same check
/// the TOML resolver applies at host-build time (`ziskbuild::aggregation`).
fn expect_template_decl(circuit: &CircomCircuit, template: &str) -> Result<()> {
    let needle = format!("template {template}(");
    match circuit.source().matches(&needle).count() {
        1 => Ok(()),
        n => bail!(
            "circuit '{}' must define `template {template}(...)` exactly once, found {n}",
            circuit.name(),
        ),
    }
}

/// One `normalize_with(...)` entry: a normalization circuit attached to a
/// subset of the registered guests, plus the number of free inputs that
/// circuit consumes. Builder-side shape; resolved to indices as
/// [`recurser::NormalizeGroup`] in `build()`.
struct NormalizeEntry<'a> {
    guests: Vec<&'a GuestProgram>,
    circuit: CircomCircuit,
    n_free_inputs: usize,
}

/// Client-independent builder for a [`Recurser`] — the proof-folding
/// sibling of [`GuestProgram`].
///
/// Most users never construct this directly: [`load_aggregation_program!`]
/// expands a TOML definition into exactly this builder call. Build it by
/// hand when the program set or circuits are only known at runtime.
///
/// ```ignore
/// let recurser = AggregationProgramBuilder::new(&[&PROG_A, &PROG_B], load_circuit!("aggregate.circom"))
///     .normalize_with(&[&PROG_A, &PROG_B], load_circuit!("normalize.circom"), 1)
///     .build()?;
/// client.setup(&recurser).run()?.await?;
/// ```
pub struct AggregationProgramBuilder<'a> {
    guests: Vec<&'a GuestProgram>,
    aggregate: CircomCircuit,
    normalize: Vec<NormalizeEntry<'a>>,
}

impl<'a> AggregationProgramBuilder<'a> {
    /// `guests` is the full leaf allowlist — order is significant, it fixes
    /// each program's `programVKs[]` index, so keep it stable across runs.
    /// `aggregate` is the `AggregatePublics` Circom body: the consistency
    /// constraints between the two folded proofs' publics plus the merge
    /// into the output publics.
    pub fn new(guests: &[&'a GuestProgram], aggregate: impl Into<CircomCircuit>) -> Self {
        Self { guests: guests.to_vec(), aggregate: aggregate.into(), normalize: Vec::new() }
    }

    /// Attach a `NormalizePublics` circuit to a subset of
    /// the guests passed to [`AggregationProgramBuilder::new`]. Each guest's publics
    /// are run through its group's circuit the first time they enter the
    /// recursion; guests not referenced by any group get the identity.
    ///
    /// `n_free_inputs` is the number of prover-supplied side inputs this
    /// circuit consumes; the recurser's shared free-input array is sized
    /// to the worst case across all groups.
    #[must_use]
    pub fn normalize_with(
        mut self,
        guests: &[&'a GuestProgram],
        circuit: impl Into<CircomCircuit>,
        n_free_inputs: usize,
    ) -> Self {
        self.normalize.push(NormalizeEntry {
            guests: guests.to_vec(),
            circuit: circuit.into(),
            n_free_inputs,
        });
        self
    }

    /// Resolves the inputs into a [`Recurser`]. Cheap: derives each
    /// program's 4-limb VK and computes the content-addressed `recurser_id`.
    /// Reads this machine's local vadcop_final verkey — even when proving
    /// remotely it must match the workers' copy or `recurser_id` diverges.
    pub fn build(self) -> Result<Recurser> {
        if self.guests.is_empty() {
            bail!("at least one guest program is required");
        }

        expect_template_decl(&self.aggregate, "AggregatePublics")?;
        for group in &self.normalize {
            expect_template_decl(&group.circuit, "NormalizePublics")?;
        }

        // Validate normalize groups against the allowlist.
        for group in &self.normalize {
            if group.guests.is_empty() {
                bail!("normalize_with(...) requires at least one guest program");
            }
            for guest in &group.guests {
                if !self.guests.iter().any(|g| g.program_id() == guest.program_id()) {
                    bail!(
                        "normalize_with(...) references program '{}' which is not in the \
                         allowlist passed to AggregationProgramBuilder::new(...)",
                        guest.name(),
                    );
                }
            }
        }
        for (i, group) in self.normalize.iter().enumerate() {
            for guest in &group.guests {
                if self.normalize[..i]
                    .iter()
                    .any(|prior| prior.guests.iter().any(|g| g.program_id() == guest.program_id()))
                {
                    bail!(
                        "program '{}' appears in more than one normalize_with(...) group",
                        guest.name(),
                    );
                }
            }
        }

        // Resolve each group's guests to indices into the allowlist — the
        // circuit muxes by `programVKs[]` position, so index order is the
        // contract (and the reason new()'s guest order must stay stable).
        let normalize_groups: Vec<recurser::NormalizeGroup> = self
            .normalize
            .iter()
            .map(|group| recurser::NormalizeGroup {
                member_indices: group
                    .guests
                    .iter()
                    .map(|guest| {
                        self.guests
                            .iter()
                            .position(|g| g.program_id() == guest.program_id())
                            .expect("membership validated above")
                    })
                    .collect(),
                body: group.circuit.source().to_string(),
                n_free_inputs: group.n_free_inputs,
            })
            .collect();
        let templates = recurser::CircomTemplates {
            normalize_groups,
            aggregate_publics: self.aggregate.source().to_string(),
        };

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

        let zisk_vk = recurser::setup::read_vadcop_final_verkey(&setup_dir).with_context(|| {
            "Failed to locate local vadcop_final verkey. Run `cargo-zisk setup --recursive` \
             on this machine (required even when using a remote coordinator)."
        })?;

        let mut program_vks: Vec<[String; 4]> = Vec::with_capacity(self.guests.len());
        for prog in &self.guests {
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
                    self.guests[prior_idx].name(),
                );
            }
            program_vks.push(limbs_str);
        }

        // The shared constructor owns the hashing, so this id is
        // byte-identical to the one setup and the worker derive.
        let inputs = recurser::RecurserManifestInputs::new(
            zisk_vk,
            program_vks.clone(),
            &templates.normalize_groups,
            &templates.aggregate_publics,
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
/// static AGG: AggregationProgram = load_aggregation_program!("chain");
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
                normalize_groups: vec![],
                aggregate_publics: "// body".into(),
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
