mod aggregation;
mod build;
mod command;
mod utils;

use build::build_program_internal;

pub use aggregation::{
    guest_elf_map, resolve_aggregation, ResolvedAggregation, ResolvedCircuitPaths,
    ResolvedNormalizeGroup, ResolvedProgram,
};
// pub use build::{execute_build_program, generate_elf_paths};

use clap::Parser;

pub const RUSTUP_TOOLCHAIN_NAME: &str = "zisk";
pub const ZISK_LINKER_SCRIPT: &[u8] = include_bytes!("../zisk_linker_script.ld");

pub const ZISK_VERSION_MESSAGE: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " [",
    env!("ZISK_COMPUTE_MODE"),
    "]",
    " (",
    env!("VERGEN_GIT_SHA"),
    " ",
    env!("VERGEN_BUILD_TIMESTAMP"),
    ")"
);

pub const ZISK_TARGET: &str = "riscv64ima-zisk-zkvm-elf";

pub const HELPER_TARGET_SUBDIR: &str = "elf";

/// Global rustflags env vars, in cargo's precedence order. In a build script
/// (or a shell targeting the host) they describe the HOST build, so they must
/// never reach guest flag resolution — see `guest_rustflags`.
pub(crate) const HOST_RUSTFLAGS_VARS: &[&str] =
    &["CARGO_ENCODED_RUSTFLAGS", "RUSTFLAGS", "CARGO_BUILD_RUSTFLAGS"];

/// Cargo's per-target rustflags env var for the guest
/// (`CARGO_TARGET_RISCV64IMA_ZISK_ZKVM_ELF_RUSTFLAGS`).
fn guest_target_rustflags_var() -> String {
    format!("CARGO_TARGET_{}_RUSTFLAGS", ZISK_TARGET.to_uppercase().replace(['-', '.'], "_"))
}

/// Every rustflags env var cargo would consult for the guest target — the
/// single source of truth for the set that must be filtered from config
/// resolution and scrubbed from a child cargo. Callers apply their own filter.
fn all_rustflags_vars() -> impl Iterator<Item = String> {
    HOST_RUSTFLAGS_VARS.iter().map(|v| v.to_string()).chain([guest_target_rustflags_var()])
}

/// The rustflags cargo would take from the environment for the guest, honoring
/// cargo's precedence: exactly one source wins, first set of
/// `CARGO_ENCODED_RUSTFLAGS` (`\x1f`) > `RUSTFLAGS` (spaces) > the per-target
/// var (spaces) > `CARGO_BUILD_RUSTFLAGS` (spaces). Cargo treats these as
/// mutually exclusive — a set global source *overrides* the per-target var
/// rather than combining with it — so we must not append lower tiers on top.
fn env_rustflags() -> Vec<String> {
    use cargo_config2::Flags;
    // `to_string_lossy` at the boundary matches cargo-config2, whose `Flags`
    // parsers take `&str`; a non-UTF8 byte becomes U+FFFD either way.
    if let Some(encoded) = std::env::var_os("CARGO_ENCODED_RUSTFLAGS") {
        Flags::from_encoded(&encoded.to_string_lossy()).flags
    } else if let Some(rustflags) = std::env::var_os("RUSTFLAGS") {
        Flags::from_space_separated(&rustflags.to_string_lossy()).flags
    } else if let Some(target_flags) = std::env::var_os(guest_target_rustflags_var()) {
        Flags::from_space_separated(&target_flags.to_string_lossy()).flags
    } else if let Some(build_flags) = std::env::var_os("CARGO_BUILD_RUSTFLAGS") {
        Flags::from_space_separated(&build_flags.to_string_lossy()).flags
    } else {
        Vec::new()
    }
}

/// Arguments for building a ZisK program.
#[derive(Default, Clone, Parser, Debug)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
pub struct BuildArgs {
    #[clap(short = 'F', long)]
    pub features: Option<String>,

    #[clap(long)]
    all_features: bool,

    #[clap(long)]
    release: bool,

    #[clap(long)]
    no_default_features: bool,

    #[clap(long, value_name = "OUTPUT_DIRECTORY")]
    output_directory: Option<String>,

    #[clap(long, value_name = "ELF_NAME")]
    elf_name: Option<String>,

    #[clap(long, value_name = "ASM")]
    pub asm: Option<bool>,

    #[clap(long, value_name = "HINTS")]
    pub hints: Option<bool>,

    #[clap(long = "package", value_name = "PACKAGE")]
    pub packages: Vec<String>,

    #[clap(long = "bin", value_name = "BIN")]
    pub binaries: Vec<String>,
}

/// Rustflags environment for compiling a Zisk guest: the flags
/// `.cargo/config.toml` declares for the guest target (resolved from
/// `program_dir`, defaulting to the current directory) plus `--cfg zisk_guest`
/// and the Zisk linker script. Returns the script's temp file — keep it alive
/// until cargo finishes — and the `CARGO_ENCODED_RUSTFLAGS` value.
///
/// Config rustflags are always preserved (e.g. the reth guest's
/// `--inline-threshold` tuning): since we set `CARGO_ENCODED_RUSTFLAGS`
/// ourselves, cargo would otherwise ignore them.
///
/// `inherit_env_rustflags` controls whether the winning env rustflags source
/// (see `env_rustflags`) is folded in too, appended after config. Pass `true`
/// from direct CLI invocations (`cargo-zisk build`/`run`), where cargo would
/// apply the user's exported vars to the guest; pass `false` from build
/// scripts, where the outer cargo sets `CARGO_ENCODED_RUSTFLAGS` to HOST flags
/// whose link args (e.g. `-Wl,--export-dynamic`) break the guest link.
pub fn guest_rustflags(
    program_dir: Option<&std::path::Path>,
    inherit_env_rustflags: bool,
) -> anyhow::Result<(tempfile::NamedTempFile, String)> {
    use anyhow::Context;
    use std::io::Write;

    // Write the linker script to a uniquely-named temp file. A predictable
    // path (`$TMPDIR/zisk.ld`) can race across concurrent invocations and is
    // exposed to temp-file symlink attacks. The caller keeps the handle alive
    // until cargo finishes so the file is not removed while the linker still
    // needs it.
    let mut linker_script = tempfile::Builder::new()
        .prefix("zisk-")
        .suffix(".ld")
        .tempfile()
        .context("Failed to create temporary Zisk linker script")?;
    linker_script
        .write_all(ZISK_LINKER_SCRIPT)
        .context("Failed to write Zisk linker script to temp file")?;

    // Resolve config rustflags with every rustflags env var filtered out: cargo
    // would otherwise let them override or shadow the program's config flags. We
    // fold them back in below (in inherit mode) so no source is lost.
    let ignored: Vec<std::ffi::OsString> =
        all_rustflags_vars().map(std::ffi::OsString::from).collect();
    let env = std::env::vars_os().filter(|(key, _)| !ignored.contains(key));
    let mut options = cargo_config2::ResolveOptions::default().env(env);
    // `[target.'cfg(...)']` sections must be evaluated with the ZisK rustc —
    // the host rustc cannot load the guest target spec. Resolution needs
    // rustc only for such sections, so a missing toolchain is not fatal here;
    // it is attached as context if resolution does fail.
    let zisk_rustc = command::zisk_rustc();
    if let Ok(rustc) = &zisk_rustc {
        options = options.rustc(cargo_config2::PathAndArgs::new(rustc));
    }
    // Canonicalized so the config walk covers the real ancestor chain — build
    // scripts pass relative dirs like "../guest", whose lexical ancestors
    // stop short of the directories cargo itself would consult.
    let cwd = match program_dir {
        Some(dir) => dir.canonicalize().with_context(|| {
            format!("Failed to canonicalize program directory {}", dir.display())
        })?,
        None => std::env::current_dir().context("Failed to get current directory")?,
    };
    // A config cargo would reject fails the build here too, instead of
    // silently dropping the program's flags.
    let mut flags = cargo_config2::Config::load_with_options(cwd, options)
        .and_then(|config| config.rustflags(ZISK_TARGET))
        .map(|flags| flags.map(|f| f.flags).unwrap_or_default())
        .map_err(|err| match zisk_rustc {
            Err(toolchain_err) => anyhow::Error::from(err).context(toolchain_err),
            Ok(_) => anyhow::Error::from(err),
        })
        .with_context(|| format!("Failed to resolve cargo rustflags for target {ZISK_TARGET}"))?;
    // Drop empty elements: encoded, they decode to an empty rustc argument,
    // which rustc rejects as an extra input filename.
    flags.retain(|f| !f.is_empty());
    // Append env rustflags after config so env wins (later rustc args win).
    if inherit_env_rustflags {
        flags.extend(env_rustflags());
    }
    flags.extend([
        "--cfg".to_string(),
        "zisk_guest".to_string(),
        "-C".to_string(),
        format!("link-arg=-T{}", linker_script.path().display()),
    ]);
    // `Flags::encode` `\x1f`-joins (wins over plain RUSTFLAGS, survives spaces)
    // and errors if a flag itself contains `\x1f` rather than silently
    // producing a value cargo would mis-split.
    let encoded =
        cargo_config2::Flags::from(flags).encode().context("Failed to encode guest rustflags")?;
    Ok((linker_script, encoded))
}

/// Configures `command` (a child `cargo` invocation) to build the guest with
/// the correct rustflags: sets `CARGO_ENCODED_RUSTFLAGS` from `guest_rustflags`
/// and scrubs the other rustflags env vars so cargo cannot re-apply them on top
/// or shift precedence. Returns the linker-script temp file — the caller MUST
/// keep it alive until `cargo` finishes. Pairing the set and scrub in one place
/// keeps them from drifting apart across call sites. See `guest_rustflags` for
/// `program_dir` / `inherit_env_rustflags`.
pub fn apply_guest_rustflags(
    command: &mut std::process::Command,
    program_dir: Option<&std::path::Path>,
    inherit_env_rustflags: bool,
) -> anyhow::Result<tempfile::NamedTempFile> {
    let (linker_script, encoded_rustflags) = guest_rustflags(program_dir, inherit_env_rustflags)?;
    command.env("CARGO_ENCODED_RUSTFLAGS", encoded_rustflags);
    // `CARGO_ENCODED_RUSTFLAGS` is the value we just set; scrub the rest.
    for var in all_rustflags_vars().filter(|v| v != "CARGO_ENCODED_RUSTFLAGS") {
        command.env_remove(var);
    }
    Ok(linker_script)
}

pub fn build_program(path: &str) {
    build_program_internal(path, None)
}

pub fn build_program_asm(path: &str) {
    let args = BuildArgs { asm: Some(true), ..Default::default() };
    build_program_internal(path, Some(args))
}

pub fn build_program_with_args(path: &str, args: BuildArgs) {
    build_program_internal(path, Some(args))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Serializes tests that touch process-global env vars (tests run in
    /// parallel threads by default).
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// Unsets env vars for the test's duration and restores their original
    /// values on drop (also on panic), so parallel tests aren't contaminated.
    struct EnvVarGuard {
        saved: Vec<(String, Option<std::ffi::OsString>)>,
        _lock: std::sync::MutexGuard<'static, ()>,
    }

    impl EnvVarGuard {
        /// Unsets the host and per-target rustflags vars and points CARGO_HOME
        /// at an empty dir under `dir`, so the machine's ~/.cargo/config.toml
        /// cannot inject flags into the test; restores everything on drop.
        fn hermetic(dir: &std::path::Path) -> Self {
            let lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            let cleared: Vec<String> = all_rustflags_vars().collect();
            let mut saved: Vec<(String, Option<std::ffi::OsString>)> =
                cleared.iter().map(|k| (k.clone(), std::env::var_os(k))).collect();
            saved.push(("CARGO_HOME".to_string(), std::env::var_os("CARGO_HOME")));
            for k in &cleared {
                std::env::remove_var(k);
            }
            let cargo_home = dir.join("cargo-home");
            std::fs::create_dir_all(&cargo_home).unwrap();
            std::env::set_var("CARGO_HOME", cargo_home);
            Self { saved, _lock: lock }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            for (key, value) in &self.saved {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
        }
    }

    /// The program's `.cargo/config.toml` rustflags must survive the injection:
    /// setting the rustflags env makes cargo ignore config files, so dropping
    /// them here silently de-optimizes guests (see `guest_rustflags`).
    #[test]
    fn guest_rustflags_keeps_config_and_appends_zisk_flags() {
        let dir = tempfile::tempdir().unwrap();
        // Env rustflags would shadow the config file (cargo precedence).
        let _env = EnvVarGuard::hermetic(dir.path());
        std::fs::create_dir(dir.path().join(".cargo")).unwrap();
        std::fs::write(
            dir.path().join(".cargo/config.toml"),
            format!(
                "[target.{ZISK_TARGET}]\nrustflags = [\"-C\", \"llvm-args=--inline-threshold=1234\"]\n"
            ),
        )
        .unwrap();

        let (script, encoded) = guest_rustflags(Some(dir.path()), false).unwrap();
        let flags: Vec<&str> = encoded.split('\u{1f}').collect();

        let config_flag = flags
            .iter()
            .position(|f| *f == "llvm-args=--inline-threshold=1234")
            .expect("config rustflags dropped");
        let cfg_flag =
            flags.iter().position(|f| *f == "zisk_guest").expect("missing --cfg zisk_guest");
        let t_flag = format!("link-arg=-T{}", script.path().display());
        assert!(flags.contains(&t_flag.as_str()), "missing linker-script flag");
        // Zisk additions come after the program's own flags.
        assert!(config_flag < cfg_flag);
        // The temp file holds the Zisk linker script.
        assert_eq!(std::fs::read(script.path()).unwrap(), ZISK_LINKER_SCRIPT);
    }

    /// An empty encoded-rustflags element decodes to an empty rustc argument,
    /// which rustc rejects as an extra input filename.
    #[test]
    fn guest_rustflags_drops_empty_flags() {
        let dir = tempfile::tempdir().unwrap();
        let _env = EnvVarGuard::hermetic(dir.path());
        std::fs::create_dir(dir.path().join(".cargo")).unwrap();
        std::fs::write(
            dir.path().join(".cargo/config.toml"),
            format!("[target.{ZISK_TARGET}]\nrustflags = [\"\"]\n"),
        )
        .unwrap();

        let (_script, encoded) = guest_rustflags(Some(dir.path()), false).unwrap();
        assert!(
            encoded.split('\u{1f}').all(|f| !f.is_empty()),
            "empty flag element in {encoded:?}"
        );
    }

    /// Build scripts run with `CARGO_ENCODED_RUSTFLAGS` set by the outer cargo
    /// to the HOST build's flags (and possibly a user-exported `RUSTFLAGS`).
    /// Those must not reach the guest: host link args such as
    /// `-Wl,--export-dynamic` are rejected by the guest's rust-lld.
    #[test]
    fn guest_rustflags_ignores_host_env_rustflags() {
        let dir = tempfile::tempdir().unwrap();
        let _env = EnvVarGuard::hermetic(dir.path());
        std::env::set_var("CARGO_ENCODED_RUSTFLAGS", "-C\u{1f}link-arg=-Wl,--export-dynamic");
        std::env::set_var("RUSTFLAGS", "-C target-cpu=native");
        std::env::set_var("CARGO_BUILD_RUSTFLAGS", "-C link-arg=--host-only-marker");

        let (_script, encoded) = guest_rustflags(Some(dir.path()), false).unwrap();
        assert!(
            !encoded.contains("export-dynamic"),
            "host CARGO_ENCODED_RUSTFLAGS leaked into guest flags: {encoded:?}"
        );
        assert!(
            !encoded.contains("target-cpu"),
            "host RUSTFLAGS leaked into guest flags: {encoded:?}"
        );
        assert!(
            !encoded.contains("host-only-marker"),
            "host CARGO_BUILD_RUSTFLAGS leaked into guest flags: {encoded:?}"
        );
    }

    /// Direct CLI invocations (`cargo-zisk build`/`run`) inherit user-exported
    /// rustflags env vars, matching what cargo itself would apply to the guest.
    #[test]
    fn guest_rustflags_inherits_env_rustflags_when_requested() {
        let dir = tempfile::tempdir().unwrap();
        let _env = EnvVarGuard::hermetic(dir.path());
        std::env::set_var("RUSTFLAGS", "--cfg my_guest_feature");

        let (_script, encoded) = guest_rustflags(Some(dir.path()), true).unwrap();
        let flags: Vec<&str> = encoded.split('\u{1f}').collect();
        assert!(
            flags.contains(&"my_guest_feature"),
            "user RUSTFLAGS dropped in inherit mode: {encoded:?}"
        );
    }

    /// Cargo makes env and config rustflags mutually exclusive, but in inherit
    /// mode the guest must keep both — config tuning and the user's env flags —
    /// with env after config so it keeps precedence.
    #[test]
    fn guest_rustflags_keeps_both_config_and_env_in_inherit_mode() {
        let dir = tempfile::tempdir().unwrap();
        let _env = EnvVarGuard::hermetic(dir.path());
        std::fs::create_dir(dir.path().join(".cargo")).unwrap();
        std::fs::write(
            dir.path().join(".cargo/config.toml"),
            format!(
                "[target.{ZISK_TARGET}]\nrustflags = [\"-C\", \"llvm-args=--inline-threshold=1234\"]\n"
            ),
        )
        .unwrap();
        std::env::set_var("RUSTFLAGS", "--cfg my_guest_feature");

        let (_script, encoded) = guest_rustflags(Some(dir.path()), true).unwrap();
        let flags: Vec<&str> = encoded.split('\u{1f}').collect();

        let config_flag = flags
            .iter()
            .position(|f| *f == "llvm-args=--inline-threshold=1234")
            .expect("config rustflags dropped in inherit mode");
        let env_flag = flags
            .iter()
            .position(|f| *f == "my_guest_feature")
            .expect("env RUSTFLAGS dropped in inherit mode");
        // Env flags come after config so env keeps cargo's precedence.
        assert!(config_flag < env_flag, "env rustflags must follow config: {encoded:?}");
    }

    /// Cargo's per-target `CARGO_TARGET_<TRIPLE>_RUSTFLAGS` var applies to the
    /// guest target: it must not suppress config resolution, and in inherit
    /// mode its flags must be folded in after config.
    #[test]
    fn guest_rustflags_handles_per_target_env_var() {
        let dir = tempfile::tempdir().unwrap();
        let _env = EnvVarGuard::hermetic(dir.path());
        std::fs::create_dir(dir.path().join(".cargo")).unwrap();
        std::fs::write(
            dir.path().join(".cargo/config.toml"),
            format!(
                "[target.{ZISK_TARGET}]\nrustflags = [\"-C\", \"llvm-args=--inline-threshold=1234\"]\n"
            ),
        )
        .unwrap();
        std::env::set_var(guest_target_rustflags_var(), "--cfg per_target_feature");

        let (_script, encoded) = guest_rustflags(Some(dir.path()), true).unwrap();
        let flags: Vec<&str> = encoded.split('\u{1f}').collect();

        let config_flag = flags
            .iter()
            .position(|f| *f == "llvm-args=--inline-threshold=1234")
            .expect("config rustflags dropped by per-target env var");
        let env_flag = flags
            .iter()
            .position(|f| *f == "per_target_feature")
            .expect("per-target env rustflags dropped in inherit mode");
        assert!(config_flag < env_flag, "per-target env rustflags must follow config: {encoded:?}");
    }

    /// Cargo's env rustflags sources are mutually exclusive: when a global
    /// source (`RUSTFLAGS`/`CARGO_ENCODED_RUSTFLAGS`) is set, the per-target var
    /// is dropped, not combined. Folding both in would apply flags cargo never
    /// would and let the wrong one win at rustc's last-flag-wins.
    #[test]
    fn guest_rustflags_env_sources_are_mutually_exclusive() {
        let dir = tempfile::tempdir().unwrap();
        let _env = EnvVarGuard::hermetic(dir.path());
        std::env::set_var("RUSTFLAGS", "--cfg global_source");
        std::env::set_var(guest_target_rustflags_var(), "--cfg per_target_source");

        let (_script, encoded) = guest_rustflags(Some(dir.path()), true).unwrap();
        let flags: Vec<&str> = encoded.split('\u{1f}').collect();
        assert!(flags.contains(&"global_source"), "global RUSTFLAGS dropped: {encoded:?}");
        assert!(
            !flags.contains(&"per_target_source"),
            "per-target var must be dropped when a global source is set: {encoded:?}"
        );
    }

    /// `CARGO_BUILD_RUSTFLAGS` is cargo's lowest-precedence env source. When it
    /// is the only source set (inherit mode), it must still reach the guest —
    /// cargo would apply it via `build.rustflags`.
    #[test]
    fn guest_rustflags_applies_sole_cargo_build_rustflags() {
        let dir = tempfile::tempdir().unwrap();
        let _env = EnvVarGuard::hermetic(dir.path());
        std::env::set_var("CARGO_BUILD_RUSTFLAGS", "--cfg build_source");

        let (_script, encoded) = guest_rustflags(Some(dir.path()), true).unwrap();
        let flags: Vec<&str> = encoded.split('\u{1f}').collect();
        assert!(
            flags.contains(&"build_source"),
            "sole CARGO_BUILD_RUSTFLAGS dropped in inherit mode: {encoded:?}"
        );
    }
}
