use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Bump when the layout of `RecurserManifestInputs` changes — old ids stop
/// matching new ones, which is the point.
pub const MANIFEST_SCHEMA_VERSION: u32 = 1;

pub const MANIFEST_FILENAME: &str = "recurser.manifest.json";
pub const PREPARE_TEMPLATE_FILENAME: &str = "prepare_publics.circom";
pub const CHECK_TEMPLATE_FILENAME: &str = "check_publics.circom";
pub const AGGREGATE_TEMPLATE_FILENAME: &str = "aggregate_publics.circom";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TemplateHashes {
    pub prepare_publics_blake3: String,
    pub check_publics_blake3: String,
    pub aggregate_publics_blake3: String,
}

/// Everything the `recurser_id` is derived from. Field order is the
/// serialization order and the id depends on it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecurserManifestInputs {
    pub schema_version: u32,
    pub zisk_vk: [String; 4],
    pub n_private_inputs: usize,
    pub program_vks: Vec<[String; 4]>,
    pub templates: TemplateHashes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecurserManifest {
    pub recurser_id: String,
    pub inputs: RecurserManifestInputs,
}

impl RecurserManifestInputs {
    pub fn new(
        zisk_vk: [String; 4],
        n_private_inputs: usize,
        program_vks: Vec<[String; 4]>,
        prepare_publics_body: &str,
        check_publics_body: &str,
        aggregate_publics_body: &str,
    ) -> Self {
        Self {
            schema_version: MANIFEST_SCHEMA_VERSION,
            zisk_vk,
            n_private_inputs,
            program_vks,
            templates: TemplateHashes {
                prepare_publics_blake3: blake3_hex(prepare_publics_body.as_bytes()),
                check_publics_blake3: blake3_hex(check_publics_body.as_bytes()),
                aggregate_publics_blake3: blake3_hex(aggregate_publics_body.as_bytes()),
            },
        }
    }

    pub fn compute_id(&self) -> String {
        let bytes =
            serde_json::to_vec(self).expect("RecurserManifestInputs is always serializable");
        blake3_hex(&bytes)
    }
}

/// Template bodies after default resolution — exactly the bytes the
/// `recurser_id` hashes. Persist these, not the caller's pre-resolution
/// originals.
pub struct ResolvedTemplates {
    pub prepare_publics: String,
    pub check_publics: String,
    pub aggregate_publics: String,
}

/// Resolve a recurser spec's optional templates to the crate defaults and
/// build the manifest inputs the `recurser_id` is derived from. Every layer
/// that derives an id (SDK builder, setup command, worker claimed-id check)
/// must go through here so the derivation cannot diverge.
pub fn resolve_manifest_inputs(
    zisk_vk: [String; 4],
    n_private_inputs: usize,
    program_vks: Vec<[String; 4]>,
    prepare_publics_body: Option<&str>,
    check_publics_body: Option<&str>,
    aggregate_publics_body: &str,
) -> (RecurserManifestInputs, ResolvedTemplates) {
    let prepare = prepare_publics_body.unwrap_or(crate::templates::DEFAULT_PREPARE_PUBLICS);
    let check = check_publics_body.unwrap_or(crate::templates::DEFAULT_CHECK_PUBLICS);
    let inputs = RecurserManifestInputs::new(
        zisk_vk,
        n_private_inputs,
        program_vks,
        prepare,
        check,
        aggregate_publics_body,
    );
    let resolved = ResolvedTemplates {
        prepare_publics: prepare.to_string(),
        check_publics: check.to_string(),
        aggregate_publics: aggregate_publics_body.to_string(),
    };
    (inputs, resolved)
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
    prepare_publics_body: &str,
    check_publics_body: &str,
    aggregate_publics_body: &str,
) -> Result<()> {
    // Templates first, manifest last: the manifest is the warmness commit
    // marker, so everything else must already be on disk when it appears —
    // and it lands via rename so a torn write can never look warm.
    for (name, body) in [
        (PREPARE_TEMPLATE_FILENAME, prepare_publics_body),
        (CHECK_TEMPLATE_FILENAME, check_publics_body),
        (AGGREGATE_TEMPLATE_FILENAME, aggregate_publics_body),
    ] {
        let path = dir.join(name);
        fs::write(&path, body).with_context(|| format!("Failed to write {}", path.display()))?;
    }

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

    #[test]
    fn id_is_deterministic() {
        let a = RecurserManifestInputs::new(vk("z"), 3, vec![vk("p")], "prep", "check", "agg");
        let b = RecurserManifestInputs::new(vk("z"), 3, vec![vk("p")], "prep", "check", "agg");
        assert_eq!(a.compute_id(), b.compute_id());
    }

    #[test]
    fn id_changes_when_any_input_changes() {
        let base = RecurserManifestInputs::new(vk("z"), 3, vec![vk("p")], "prep", "check", "agg");
        let id = base.compute_id();

        assert_ne!(
            id,
            RecurserManifestInputs::new(vk("Z"), 3, vec![vk("p")], "prep", "check", "agg")
                .compute_id(),
        );
        assert_ne!(
            id,
            RecurserManifestInputs::new(vk("z"), 4, vec![vk("p")], "prep", "check", "agg")
                .compute_id(),
        );
        assert_ne!(
            id,
            RecurserManifestInputs::new(vk("z"), 3, vec![vk("q")], "prep", "check", "agg")
                .compute_id(),
        );
        assert_ne!(
            id,
            RecurserManifestInputs::new(vk("z"), 3, vec![vk("p"), vk("q")], "prep", "check", "agg")
                .compute_id(),
        );
        assert_ne!(
            id,
            RecurserManifestInputs::new(vk("z"), 3, vec![vk("p")], "PREP", "check", "agg")
                .compute_id(),
        );
        assert_ne!(
            id,
            RecurserManifestInputs::new(vk("z"), 3, vec![vk("p")], "prep", "CHECK", "agg")
                .compute_id(),
        );
        assert_ne!(
            id,
            RecurserManifestInputs::new(vk("z"), 3, vec![vk("p")], "prep", "check", "AGG")
                .compute_id(),
        );
    }

    #[test]
    fn id_is_64_hex_chars() {
        let id = RecurserManifestInputs::new(vk("z"), 0, vec![vk("p")], "a", "b", "c").compute_id();
        assert_eq!(id.len(), 64);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn manifest_json_roundtrips() {
        let inputs = RecurserManifestInputs::new(vk("z"), 2, vec![vk("p"), vk("q")], "a", "b", "c");
        let recurser_id = inputs.compute_id();
        let manifest = RecurserManifest { recurser_id: recurser_id.clone(), inputs };

        let json = serde_json::to_string_pretty(&manifest).unwrap();
        let loaded: RecurserManifest = serde_json::from_str(&json).unwrap();

        assert_eq!(loaded.recurser_id, recurser_id);
        assert_eq!(loaded.inputs, manifest.inputs);
    }
}
