//! Shared helpers used across CLI commands.

use anyhow::Result;
use std::env;
use std::fs;
use std::path::PathBuf;
use zisk_build::ZISK_TARGET;

/// Build the default proof output filename when the user passes no `--output`.
///
/// Format: `<timestamp>-<jobid if any>-proof[-plonk].bin`, where `<timestamp>`
/// is the current Unix time in seconds and the `-plonk` suffix is added only
/// for PLONK proofs.
pub(crate) fn default_proof_filename(job_id: Option<impl std::fmt::Display>) -> PathBuf {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let job_segment = job_id.map(|id| format!("{id}-")).unwrap_or_default();
    PathBuf::from(format!("{timestamp}-{job_segment}proof.bin"))
}

/// Cargo build profile used to locate the auto-detected guest ELF.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Profile {
    /// `target/elf/<target>/debug/<bin>` — the default.
    #[default]
    Debug,
    /// `target/elf/<target>/release/<bin>`.
    Release,
}

impl Profile {
    /// The Cargo profile sub-directory name under the ELF target dir.
    fn dir(self) -> &'static str {
        match self {
            Profile::Debug => "debug",
            Profile::Release => "release",
        }
    }
}

/// Build the guest ELF path for a given project root, profile, and binary name.
///
/// Pure (no filesystem access) so the layout is testable in isolation.
fn project_elf_path(
    project_root: &std::path::Path,
    profile: Profile,
    binary_name: &str,
) -> PathBuf {
    project_root.join("target").join("elf").join(ZISK_TARGET).join(profile.dir()).join(binary_name)
}

/// Auto-detect the current project's guest ELF for a specific [`Profile`].
///
/// Returns `Ok(None)` when there is no `Cargo.toml`, no package name, or no built
/// ELF for that profile. Looks in exactly one profile directory (no fallback).
fn detect_project_elf_for_profile(profile: Profile) -> Result<Option<PathBuf>> {
    let current_dir = env::current_dir()?;
    let cargo_toml = current_dir.join("Cargo.toml");
    if !cargo_toml.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&cargo_toml)?;
    let Some(binary_name) = parse_package_name_from_cargo_toml(&content) else {
        return Ok(None);
    };

    let candidate = project_elf_path(&current_dir, profile, &binary_name);
    Ok(candidate.exists().then_some(candidate))
}

/// Auto-detect the current project's guest ELF, preferring `release` then `debug`.
///
/// Used by the dev commands and `run`/`clean_cache`, which have no profile flag.
pub(crate) fn detect_current_project_elf() -> Result<Option<PathBuf>> {
    if let Some(release) = detect_project_elf_for_profile(Profile::Release)? {
        return Ok(Some(release));
    }
    detect_project_elf_for_profile(Profile::Debug)
}

/// Profile-selection flags shared by the user-facing `cargo-zisk` commands.
///
/// These only affect guest-ELF auto-detection; they are rejected by clap when an
/// explicit `--elf` is given. The default profile is [`Profile::Debug`]; pass
/// `--release` to pick up the release-profile ELF instead.
#[derive(clap::Args, Debug, Default)]
pub(crate) struct ProfileArgs {
    /// Use the release-profile guest ELF (auto-detection only; not valid with --elf)
    #[arg(long, conflicts_with_all = ["debug", "elf"])]
    release: bool,

    /// Use the debug-profile guest ELF (default; auto-detection only; not valid with --elf)
    #[arg(long, conflicts_with = "elf")]
    debug: bool,
}

impl ProfileArgs {
    pub(crate) fn profile(&self) -> Profile {
        if self.release {
            Profile::Release
        } else {
            Profile::Debug
        }
    }
}

/// Reject a `quic://` hints URI — the CLI has no event loop to host a live QUIC
/// stream, so it cannot serve QUIC hints to either the embedded or remote backend.
pub(crate) fn reject_quic_hints(hints: Option<&str>) -> Result<()> {
    if hints.is_some_and(|uri| uri.starts_with("quic://")) {
        anyhow::bail!("QUIC hints source is not supported in CLI mode.");
    }
    Ok(())
}

/// Resolve where to write a proof: an explicit `--output` path if given,
/// otherwise the generated [`default_proof_filename`] for the job. Pure, so the
/// explicit-vs-default branch is testable without a prover.
pub(crate) fn resolve_output_path(
    explicit: Option<PathBuf>,
    job_id: Option<impl std::fmt::Display>,
) -> PathBuf {
    explicit.unwrap_or_else(|| default_proof_filename(job_id))
}

/// Resolve the guest ELF: explicit path, otherwise auto-detect the given
/// [`Profile`]'s ELF in the current project. An explicit `--elf` always wins and
/// makes `profile` irrelevant (clap rejects the flag combination anyway).
pub(crate) fn resolve_elf(elf: Option<PathBuf>, profile: Profile) -> Result<PathBuf> {
    match elf {
        Some(elf) => Ok(elf),
        None => detect_project_elf_for_profile(profile)?.ok_or_else(|| {
            anyhow::anyhow!(
                "No ELF file provided, and could not detect a {} project ELF in the current directory. \
                 Build the guest{} or pass an ELF file with --elf.",
                profile.dir(),
                match profile {
                    Profile::Release => " with --release",
                    Profile::Debug => "",
                },
            )
        }),
    }
}

fn parse_package_name_from_cargo_toml(content: &str) -> Option<String> {
    let mut in_package = false;

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line == "[package]" {
            in_package = true;
            continue;
        }

        if line.starts_with('[') {
            in_package = false;
            continue;
        }

        if in_package && line.starts_with("name") {
            return parse_toml_string_value(line);
        }
    }

    None
}

fn parse_toml_string_value(line: &str) -> Option<String> {
    let (_, value) = line.split_once('=')?;
    let value = value.trim();
    if !(value.starts_with('"') && value.ends_with('"')) {
        return None;
    }
    Some(value.trim_matches('"').to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_proof_filename_without_job_id() {
        let name = default_proof_filename(None::<&str>);
        let s = name.to_str().unwrap();
        assert!(s.ends_with("-proof.bin"), "unexpected name: {s}");
        // <timestamp>-proof.bin → exactly one '-' separating timestamp and suffix.
        assert_eq!(s.matches('-').count(), 1, "unexpected name: {s}");
    }

    #[test]
    fn default_proof_filename_with_job_id() {
        let name = default_proof_filename(Some("job42"));
        let s = name.to_str().unwrap();
        assert!(s.contains("-job42-"), "job id missing: {s}");
        assert!(s.ends_with("-job42-proof.bin"), "unexpected name: {s}");
    }

    #[test]
    fn reject_quic_hints_rejects_quic_scheme() {
        assert!(reject_quic_hints(Some("quic://host:1234")).is_err());
    }

    #[test]
    fn reject_quic_hints_allows_other_sources() {
        assert!(reject_quic_hints(None).is_ok());
        assert!(reject_quic_hints(Some("file:///tmp/hints.bin")).is_ok());
        assert!(reject_quic_hints(Some("/tmp/hints.bin")).is_ok());
        assert!(reject_quic_hints(Some("unix:///tmp/sock")).is_ok());
    }

    #[test]
    fn resolve_elf_returns_explicit_path_verbatim() {
        // An explicit ELF wins regardless of the selected profile.
        let explicit = PathBuf::from("/some/where/guest.elf");
        for profile in [Profile::Debug, Profile::Release] {
            let resolved = resolve_elf(Some(explicit.clone()), profile).unwrap();
            assert_eq!(resolved, explicit);
        }
    }

    #[test]
    fn project_elf_path_uses_profile_subdir() {
        let root = PathBuf::from("/proj");
        let debug = project_elf_path(&root, Profile::Debug, "guest");
        let release = project_elf_path(&root, Profile::Release, "guest");

        let suffix = PathBuf::from("target").join("elf").join(ZISK_TARGET);
        assert_eq!(debug, root.join(&suffix).join("debug").join("guest"));
        assert_eq!(release, root.join(&suffix).join("release").join("guest"));
    }

    #[test]
    fn profile_args_defaults_to_debug_and_release_opts_in() {
        assert_eq!(ProfileArgs::default().profile(), Profile::Debug);
        let explicit_debug = ProfileArgs { release: false, debug: true };
        assert_eq!(explicit_debug.profile(), Profile::Debug);
        let release = ProfileArgs { release: true, debug: false };
        assert_eq!(release.profile(), Profile::Release);
    }

    #[test]
    fn resolve_output_path_prefers_explicit() {
        let explicit = PathBuf::from("/out/proof.bin");
        let resolved = resolve_output_path(Some(explicit.clone()), Some("job1"));
        assert_eq!(resolved, explicit);
    }

    #[test]
    fn resolve_output_path_falls_back_to_default() {
        let resolved = resolve_output_path(None, Some("job1"));
        let s = resolved.to_str().unwrap();
        assert!(s.ends_with("-job1-proof.bin"), "unexpected default: {s}");
    }

    #[test]
    fn resolve_output_path_default_without_job() {
        let resolved = resolve_output_path(None, None::<&str>);
        assert!(resolved.to_str().unwrap().ends_with("-proof.bin"));
    }

    #[test]
    fn parse_package_name_basic() {
        let toml = "[package]\nname = \"my-guest\"\nversion = \"0.1.0\"\n";
        assert_eq!(parse_package_name_from_cargo_toml(toml).as_deref(), Some("my-guest"));
    }

    #[test]
    fn parse_package_name_skips_comments_and_blank_lines() {
        let toml = "# a comment\n\n[package]\n\n# name lives here\nname = \"guest\"\n";
        assert_eq!(parse_package_name_from_cargo_toml(toml).as_deref(), Some("guest"));
    }

    #[test]
    fn parse_package_name_ignores_name_outside_package_section() {
        // `name` under [dependencies] must not be mistaken for the package name.
        let toml = "[dependencies]\nname = \"not-it\"\n\n[package]\nname = \"real\"\n";
        assert_eq!(parse_package_name_from_cargo_toml(toml).as_deref(), Some("real"));
    }

    #[test]
    fn parse_package_name_stops_at_next_section() {
        // Once we leave [package] without finding a name, a later section's name
        // must not be picked up.
        let toml = "[package]\nversion = \"0.1.0\"\n\n[bin]\nname = \"other\"\n";
        assert_eq!(parse_package_name_from_cargo_toml(toml), None);
    }

    #[test]
    fn parse_package_name_missing_returns_none() {
        assert_eq!(parse_package_name_from_cargo_toml("[package]\nversion = \"1\"\n"), None);
        assert_eq!(parse_package_name_from_cargo_toml(""), None);
    }

    #[test]
    fn parse_toml_string_value_handles_quotes_and_whitespace() {
        assert_eq!(parse_toml_string_value("name = \"guest\"").as_deref(), Some("guest"));
        assert_eq!(parse_toml_string_value("name=\"guest\"").as_deref(), Some("guest"));
        assert_eq!(
            parse_toml_string_value("name =   \"  spaced  \"  ").as_deref(),
            Some("  spaced  ")
        );
    }

    #[test]
    fn parse_toml_string_value_rejects_unquoted_or_malformed() {
        assert_eq!(parse_toml_string_value("name = guest"), None);
        assert_eq!(parse_toml_string_value("name = \"guest"), None);
        assert_eq!(parse_toml_string_value("noequals"), None);
    }
}
