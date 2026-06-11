//! Shared on-disk layout for recurser artifacts.
//!
//! Both halves of the recurser — `setup` (which writes the artifacts) and
//! `prove` (which loads them) — must agree on *where* every file lives and
//! *what* it is named. That agreement lives here, in one feature-gate-free
//! place, so neither half hardcodes paths the other half owns.
//!
//! The layout, rooted at `<output_dir>/provingKey/recurser/<recurser_id>/`:
//!
//! | File | Written by | Read by |
//! |---|---|---|
//! | `recurser_aggregator.const` | setup | proofman (const-tree build) |
//! | `recurser_aggregator.consttree` | proofman (at register) | proofman (prove) |
//! | `recurser_aggregator.verkey.json` | setup | — (human/tooling) |
//! | `recurser_aggregator.verkey.bin` | setup | prove (default `rootC`) |
//! | `recurser_aggregator.exec` | setup | proofman (witness) |
//! | `recurser.manifest.json` | setup | prove (input validation) |

use std::path::{Path, PathBuf};

use crate::manifest::MANIFEST_FILENAME;

/// Stem of every per-setup proofman artifact (`<stem>.const`, `<stem>.consttree`,
/// `<stem>.verkey.bin`, …).
pub const SETUP_STEM: &str = "recurser_aggregator";

/// Resolves the on-disk locations of a single recurser setup.
///
/// Construct once from the `output_dir` the setup ran against plus the
/// content-addressed `recurser_id`, then ask it for paths — never rebuild
/// them by hand.
#[derive(Debug, Clone)]
pub struct RecurserArtifacts {
    dir: PathBuf,
}

impl RecurserArtifacts {
    /// Locate the artifacts for `recurser_id` under `output_dir`.
    pub fn new(output_dir: impl AsRef<Path>, recurser_id: &str) -> Self {
        let dir = output_dir.as_ref().join("provingKey").join("recurser").join(recurser_id);
        Self { dir }
    }

    /// The directory holding every artifact for this setup.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The shared stem proofman expects: `<dir>/recurser_aggregator`.
    pub fn setup_stem(&self) -> PathBuf {
        self.dir.join(SETUP_STEM)
    }

    /// `<dir>/recurser_aggregator.<ext>` (e.g. `ext = "verkey.bin"`).
    pub fn stem_with_ext(&self, ext: &str) -> PathBuf {
        self.dir.join(format!("{SETUP_STEM}.{ext}"))
    }

    /// The recurser manifest written by setup.
    pub fn manifest_path(&self) -> PathBuf {
        self.dir.join(MANIFEST_FILENAME)
    }

    /// Const-polynomial Merkle tree. proofman writes this itself at register
    /// time (and loads it on subsequent registrations); setup does not produce it.
    pub fn const_tree_path(&self) -> PathBuf {
        self.stem_with_ext("consttree")
    }

    /// Const polynomials.
    pub fn const_path(&self) -> PathBuf {
        self.stem_with_ext("const")
    }

    /// Verkey root as a JSON array.
    pub fn verkey_json_path(&self) -> PathBuf {
        self.stem_with_ext("verkey.json")
    }

    /// Verkey root as little-endian `u64` limbs.
    pub fn verkey_bin_path(&self) -> PathBuf {
        self.stem_with_ext("verkey.bin")
    }

    /// Read the recurser's own 4-limb verkey (32 bytes, little-endian).
    /// Available once setup has completed.
    pub fn read_verkey(&self) -> anyhow::Result<[u64; 4]> {
        use anyhow::Context;
        let path = self.verkey_bin_path();
        let bytes =
            std::fs::read(&path).with_context(|| format!("Failed to read {}", path.display()))?;
        let bytes: [u8; 32] = bytes.as_slice().try_into().map_err(|_| {
            anyhow::anyhow!("{} is {} bytes, expected 32", path.display(), bytes.len())
        })?;
        let mut limbs = [0u64; 4];
        for (i, limb) in limbs.iter_mut().enumerate() {
            *limb = u64::from_le_bytes(bytes[i * 8..(i + 1) * 8].try_into().unwrap());
        }
        Ok(limbs)
    }

    /// Witness execution plan.
    pub fn exec_path(&self) -> PathBuf {
        self.stem_with_ext("exec")
    }

    /// True once setup has produced its artifacts — i.e. `prove` can register
    /// this setup. The manifest is written last in the setup flow, after the
    /// verkey/const land and the witness-library build is awaited, so its
    /// presence is the commit marker: without it a setup that died mid-way
    /// (e.g. at the witness `.so` build) would look active while missing
    /// load-bearing files. proofman builds the `.consttree` itself at register
    /// time, so that one is deliberately not part of the check.
    pub fn is_active(&self) -> bool {
        self.missing_artifacts().is_empty()
    }

    /// File names of the activeness-required artifacts that are missing — for
    /// error messages that say *what* is incomplete rather than just "not ready".
    pub fn missing_artifacts(&self) -> Vec<String> {
        [self.manifest_path(), self.verkey_json_path(), self.verkey_bin_path(), self.const_path()]
            .into_iter()
            .filter(|p| !p.is_file())
            .map(|p| p.file_name().unwrap_or_default().to_string_lossy().into_owned())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_are_rooted_under_provingkey_recurser_id() {
        let a = RecurserArtifacts::new("/out", "abc123");
        assert_eq!(a.dir(), Path::new("/out/provingKey/recurser/abc123"));
        assert_eq!(
            a.setup_stem(),
            Path::new("/out/provingKey/recurser/abc123/recurser_aggregator")
        );
        assert_eq!(
            a.verkey_bin_path(),
            Path::new("/out/provingKey/recurser/abc123/recurser_aggregator.verkey.bin")
        );
        assert_eq!(
            a.const_tree_path(),
            Path::new("/out/provingKey/recurser/abc123/recurser_aggregator.consttree")
        );
        assert_eq!(
            a.manifest_path(),
            Path::new("/out/provingKey/recurser/abc123/recurser.manifest.json")
        );
    }
}
