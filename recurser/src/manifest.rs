use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::templates::NormalizeGroup;

pub const MANIFEST_FILENAME: &str = "recurser.manifest.json";
pub const AGGREGATE_TEMPLATE_FILENAME: &str = "aggregate_publics.circom";

pub fn normalize_template_filename(group_idx: usize) -> String {
    format!("normalize_{group_idx}.circom")
}

/// One normalization group as committed into the `recurser_id`: which
/// programs it covers, the hash of its Circom body, and its side-input count.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NormalizeGroupHash {
    pub member_indices: Vec<usize>,
    pub template_blake3: String,
    pub n_free_inputs: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TemplateHashes {
    pub normalize_groups: Vec<NormalizeGroupHash>,
    pub aggregate_publics_blake3: String,
}

/// Everything the `recurser_id` is derived from. The id is a blake3 of the
/// JSON serialization, so any change to this struct's shape or contents
/// produces a fresh id — no explicit schema versioning needed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecurserManifestInputs {
    pub zisk_vk: [String; 4],
    pub program_vks: Vec<[String; 4]>,
    pub templates: TemplateHashes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecurserManifest {
    pub recurser_id: String,
    pub inputs: RecurserManifestInputs,
}

impl RecurserManifestInputs {
    /// Build the id-bearing inputs from the resolved generation inputs.
    /// Every layer that derives an id (SDK builder, setup command, worker
    /// claimed-id check) must go through here so the derivation cannot
    /// diverge.
    pub fn new(
        zisk_vk: [String; 4],
        program_vks: Vec<[String; 4]>,
        normalize_groups: &[NormalizeGroup],
        aggregate_publics_body: &str,
    ) -> Self {
        Self {
            zisk_vk,
            program_vks,
            templates: TemplateHashes {
                normalize_groups: normalize_groups
                    .iter()
                    .map(|g| NormalizeGroupHash {
                        member_indices: g.member_indices.clone(),
                        template_blake3: blake3_hex(g.body.as_bytes()),
                        n_free_inputs: g.n_free_inputs,
                    })
                    .collect(),
                aggregate_publics_blake3: blake3_hex(aggregate_publics_body.as_bytes()),
            },
        }
    }

    pub fn compute_id(&self) -> String {
        let bytes =
            serde_json::to_vec(self).expect("RecurserManifestInputs is always serializable");
        blake3_hex(&bytes)
    }

    /// Size of the circuit's per-side `freeInputs` arrays: the worst case
    /// across normalization groups.
    pub fn n_free_inputs(&self) -> usize {
        self.templates.normalize_groups.iter().map(|g| g.n_free_inputs).max().unwrap_or(0)
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
    normalize_groups: &[NormalizeGroup],
    aggregate_publics_body: &str,
) -> Result<()> {
    // Templates first, manifest last: the manifest is the commit marker for a
    // completed setup (see `RecurserArtifacts::is_active`), so everything else
    // must already be on disk when it appears — and it lands via rename so a
    // torn write can never look complete.
    for (idx, group) in normalize_groups.iter().enumerate() {
        let path = dir.join(normalize_template_filename(idx));
        fs::write(&path, &group.body)
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

    fn vk(prefix: &str) -> [String; 4] {
        [format!("{prefix}1"), format!("{prefix}2"), format!("{prefix}3"), format!("{prefix}4")]
    }

    fn group(members: &[usize], body: &str, n: usize) -> NormalizeGroup {
        NormalizeGroup {
            member_indices: members.to_vec(),
            body: body.to_string(),
            n_free_inputs: n,
        }
    }

    #[test]
    fn id_is_deterministic() {
        let g = [group(&[0], "norm", 3)];
        let a = RecurserManifestInputs::new(vk("z"), vec![vk("p")], &g, "agg");
        let b = RecurserManifestInputs::new(vk("z"), vec![vk("p")], &g, "agg");
        assert_eq!(a.compute_id(), b.compute_id());
    }

    #[test]
    fn id_changes_when_any_input_changes() {
        let g = [group(&[0], "norm", 3)];
        let id = RecurserManifestInputs::new(vk("z"), vec![vk("p")], &g, "agg").compute_id();

        assert_ne!(id, RecurserManifestInputs::new(vk("Z"), vec![vk("p")], &g, "agg").compute_id(),);
        assert_ne!(id, RecurserManifestInputs::new(vk("z"), vec![vk("q")], &g, "agg").compute_id(),);
        assert_ne!(
            id,
            RecurserManifestInputs::new(vk("z"), vec![vk("p"), vk("q")], &g, "agg").compute_id(),
        );
        assert_ne!(
            id,
            RecurserManifestInputs::new(vk("z"), vec![vk("p")], &[group(&[0], "NORM", 3)], "agg")
                .compute_id(),
        );
        assert_ne!(
            id,
            RecurserManifestInputs::new(vk("z"), vec![vk("p")], &[group(&[0], "norm", 4)], "agg")
                .compute_id(),
        );
        assert_ne!(id, RecurserManifestInputs::new(vk("z"), vec![vk("p")], &g, "AGG").compute_id(),);
        // No groups at all is a distinct configuration too.
        assert_ne!(
            id,
            RecurserManifestInputs::new(vk("z"), vec![vk("p")], &[], "agg").compute_id(),
        );
    }

    #[test]
    fn id_is_64_hex_chars() {
        let id = RecurserManifestInputs::new(vk("z"), vec![vk("p")], &[], "c").compute_id();
        assert_eq!(id.len(), 64);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn n_free_inputs_is_max_across_groups() {
        let groups = [group(&[0], "a", 2), group(&[1], "b", 55), group(&[2], "c", 0)];
        let m =
            RecurserManifestInputs::new(vk("z"), vec![vk("p"), vk("q"), vk("r")], &groups, "agg");
        assert_eq!(m.n_free_inputs(), 55);

        let empty = RecurserManifestInputs::new(vk("z"), vec![vk("p")], &[], "agg");
        assert_eq!(empty.n_free_inputs(), 0);
    }

    #[test]
    fn manifest_json_roundtrips() {
        let groups = [group(&[0, 1], "norm", 2)];
        let inputs = RecurserManifestInputs::new(vk("z"), vec![vk("p"), vk("q")], &groups, "agg");
        let recurser_id = inputs.compute_id();
        let manifest = RecurserManifest { recurser_id: recurser_id.clone(), inputs };

        let json = serde_json::to_string_pretty(&manifest).unwrap();
        let loaded: RecurserManifest = serde_json::from_str(&json).unwrap();

        assert_eq!(loaded.recurser_id, recurser_id);
        assert_eq!(loaded.inputs, manifest.inputs);
    }
}
