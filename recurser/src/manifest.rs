use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::templates::NormalizeCircuit;

pub const MANIFEST_FILENAME: &str = "recurser.manifest.json";
pub const AGGREGATE_TEMPLATE_FILENAME: &str = "aggregate_publics.circom";
pub const NORMALIZE_TEMPLATE_FILENAME: &str = "normalize.circom";

/// Hash of the optional single normalization circuit.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NormalizeHash {
    pub template_blake3: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TemplateHashes {
    pub normalize: Option<NormalizeHash>,
    pub aggregate_publics_blake3: String,
    /// Unified free-input width per side (NormalizePublics + AggregatePublics).
    pub n_free: usize,
    /// Publics slots the aggregation populates; the rest are zero-filled. In the
    /// id so a used-width change forces a fresh setup.
    pub n_publics_agg: usize,
}

/// Everything the `recurser_id` is derived from — a blake3 of the JSON
/// serialization, so any change to this struct produces a fresh id.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecurserManifestInputs {
    pub zisk_vk: [String; 4],
    #[serde(default)]
    pub program_vks: Vec<[String; 4]>,
    pub templates: TemplateHashes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecurserManifest {
    pub recurser_id: String,
    pub inputs: RecurserManifestInputs,
}

impl RecurserManifestInputs {
    /// Build the id-bearing inputs. Every layer that derives an id must go
    /// through here so the derivation cannot diverge.
    pub fn new(
        zisk_vk: [String; 4],
        program_vks: Vec<[String; 4]>,
        normalize: Option<&NormalizeCircuit>,
        aggregate_publics_body: &str,
        n_free: usize,
        n_publics_agg: usize,
    ) -> Self {
        Self {
            zisk_vk,
            program_vks,
            templates: TemplateHashes {
                normalize: normalize
                    .map(|n| NormalizeHash { template_blake3: blake3_hex(n.body.as_bytes()) }),
                aggregate_publics_blake3: blake3_hex(aggregate_publics_body.as_bytes()),
                n_free,
                n_publics_agg,
            },
        }
    }

    pub fn compute_id(&self) -> String {
        let bytes =
            serde_json::to_vec(self).expect("RecurserManifestInputs is always serializable");
        blake3_hex(&bytes)
    }

    /// Single unified free-input width per side.
    pub fn n_free(&self) -> usize {
        self.templates.n_free
    }
}

impl RecurserManifest {
    pub fn load(dir: &Path) -> Result<Self> {
        let path = dir.join(MANIFEST_FILENAME);
        let bytes = fs::read(&path)
            .with_context(|| format!("Failed to read recurser manifest at {}", path.display()))?;
        serde_json::from_slice(&bytes)
            .with_context(|| format!("Failed to parse recurser manifest at {}", path.display()))
    }
}

pub fn write_manifest_and_templates(
    dir: &Path,
    manifest: &RecurserManifest,
    normalize: Option<&NormalizeCircuit>,
    aggregate_publics_body: &str,
) -> Result<()> {
    // Templates first, manifest last: the manifest is the commit marker for a
    // completed setup (see `RecurserArtifacts::is_active`), so everything else
    // must already be on disk when it appears — and it lands via rename so a
    // torn write can never look complete.
    if let Some(norm) = normalize {
        let path = dir.join(NORMALIZE_TEMPLATE_FILENAME);
        fs::write(&path, &norm.body)
            .with_context(|| format!("Failed to write {}", path.display()))?;
    }
    let path = dir.join(AGGREGATE_TEMPLATE_FILENAME);
    fs::write(&path, aggregate_publics_body)
        .with_context(|| format!("Failed to write {}", path.display()))?;

    let manifest_path = dir.join(MANIFEST_FILENAME);
    let manifest_json = serde_json::to_string_pretty(manifest)?;
    let tmp_path = manifest_path.with_extension("json.tmp");
    fs::write(&tmp_path, manifest_json)
        .with_context(|| format!("Failed to write {}", tmp_path.display()))?;
    fs::rename(&tmp_path, &manifest_path).with_context(|| {
        format!("Failed to rename {} -> {}", tmp_path.display(), manifest_path.display())
    })?;
    Ok(())
}

fn blake3_hex(b: &[u8]) -> String {
    blake3::hash(b).to_hex().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::templates::NormalizeCircuit;

    fn vk(prefix: &str) -> [String; 4] {
        [format!("{prefix}1"), format!("{prefix}2"), format!("{prefix}3"), format!("{prefix}4")]
    }

    fn norm() -> NormalizeCircuit {
        NormalizeCircuit { body: "// b".into() }
    }

    #[test]
    fn id_is_deterministic() {
        let a = RecurserManifestInputs::new(vk("z"), vec![], None, "agg", 0, 6);
        let b = RecurserManifestInputs::new(vk("z"), vec![], None, "agg", 0, 6);
        assert_eq!(a.compute_id(), b.compute_id());
    }

    #[test]
    fn id_changes_when_any_input_changes() {
        let id = RecurserManifestInputs::new(vk("z"), vec![], None, "agg", 0, 6).compute_id();

        // zisk_vk change
        assert_ne!(
            id,
            RecurserManifestInputs::new(vk("Z"), vec![], None, "agg", 0, 6).compute_id()
        );
        // aggregate body change
        assert_ne!(
            id,
            RecurserManifestInputs::new(vk("z"), vec![], None, "AGG", 0, 6).compute_id()
        );
        // n_free change
        assert_ne!(
            id,
            RecurserManifestInputs::new(vk("z"), vec![], None, "agg", 5, 6).compute_id()
        );
        // n_publics_agg change
        assert_ne!(
            id,
            RecurserManifestInputs::new(vk("z"), vec![], None, "agg", 0, 7).compute_id()
        );
        // None vs Some
        assert_ne!(
            id,
            RecurserManifestInputs::new(vk("z"), vec![], Some(&norm()), "agg", 0, 6).compute_id()
        );
        // normalize body change
        let id_with_norm =
            RecurserManifestInputs::new(vk("z"), vec![], Some(&norm()), "agg", 0, 6).compute_id();
        let norm_alt = NormalizeCircuit { body: "// c".into() };
        assert_ne!(
            id_with_norm,
            RecurserManifestInputs::new(vk("z"), vec![], Some(&norm_alt), "agg", 0, 6).compute_id()
        );
    }

    #[test]
    fn id_changes_with_program_allowlist() {
        let none = RecurserManifestInputs::new(vk("z"), vec![], None, "agg", 0, 6).compute_id();
        // Adding an allow-list changes the id.
        let one =
            RecurserManifestInputs::new(vk("z"), vec![vk("p")], None, "agg", 0, 6).compute_id();
        assert_ne!(none, one);
        // A different allow-list member changes the id.
        let one_alt =
            RecurserManifestInputs::new(vk("z"), vec![vk("q")], None, "agg", 0, 6).compute_id();
        assert_ne!(one, one_alt);
        // Order is significant.
        let ab = RecurserManifestInputs::new(vk("z"), vec![vk("p"), vk("q")], None, "agg", 0, 6)
            .compute_id();
        let ba = RecurserManifestInputs::new(vk("z"), vec![vk("q"), vk("p")], None, "agg", 0, 6)
            .compute_id();
        assert_ne!(ab, ba);
    }

    #[test]
    fn id_is_64_hex_chars() {
        let id = RecurserManifestInputs::new(vk("z"), vec![], None, "c", 0, 6).compute_id();
        assert_eq!(id.len(), 64);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn n_free_accessor() {
        let m = RecurserManifestInputs::new(vk("z"), vec![], Some(&norm()), "agg", 7, 6);
        assert_eq!(m.n_free(), 7);

        let m_no_norm = RecurserManifestInputs::new(vk("z"), vec![], None, "agg", 5, 6);
        assert_eq!(m_no_norm.n_free(), 5);
    }

    #[test]
    fn manifest_json_roundtrips() {
        let inputs = RecurserManifestInputs::new(vk("z"), vec![], Some(&norm()), "agg", 0, 6);
        let recurser_id = inputs.compute_id();
        let manifest = RecurserManifest { recurser_id: recurser_id.clone(), inputs };

        let json = serde_json::to_string_pretty(&manifest).unwrap();
        let loaded: RecurserManifest = serde_json::from_str(&json).unwrap();

        assert_eq!(loaded.recurser_id, recurser_id);
        assert_eq!(loaded.inputs, manifest.inputs);
        // Confirm normalize hash is present in the roundtripped value
        assert!(loaded.inputs.templates.normalize.is_some());
    }
}
