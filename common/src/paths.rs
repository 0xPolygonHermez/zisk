use std::env;
use std::path::PathBuf;
use std::sync::OnceLock;

// ── User mode ────────────────────────────────────────────────────────────────
/// Suffix appended under `$HOME` when `ZISK_HOME` is unset.
pub const USER_HOME_SUBDIR: &str = ".zisk";

// ── Linux service mode ───────────────────────────────────────────────────────
/// Canonical bundle root for Linux service installs.
pub const LINUX_SERVICE_HOME: &str = "/opt/zisk";

// ── macOS service mode ───────────────────────────────────────────────────────
/// Canonical bundle root for macOS service installs.
/// FHS doesn't apply on macOS; the bundle lives under Application Support.
pub const MACOS_SERVICE_HOME: &str = "/Library/Application Support/ZisK";

// ── Common folder/file names (relative, under `home`) ─────────────────────────────
pub const BIN_DIR: &str = "bin";
pub const CACHE_DIR: &str = "cache";
pub const PROVING_KEY_DIR: &str = "provingKey";
pub const PROVING_KEY_SNARK_DIR: &str = "provingKeySnark";
pub const TOOLCHAINS_DIR: &str = "toolchains";
pub const VERIFY_KEY_DIR: &str = "verifyKey";
pub const ZISK_DIR: &str = "zisk";
pub const EMULATOR_ASM_DIR: &str = "emulator-asm";
pub const LIBZISKCLIB_FILE: &str = "libziskclib.a";

/// Layout-independent view over the ZisK install directory tree.
///
/// All callers should go through `ZiskPaths::global()` rather than
/// reading `HOME` or hardcoding `.zisk` substrings. This lets the same
/// binary serve both user-mode and service-mode installs by reading
/// `ZISK_HOME` and `ZISK_CACHE_DIR` env vars.
///
/// Defaults preserve current user-mode behavior:
/// - `ZISK_HOME` unset → `$HOME/.zisk`
/// - `ZISK_CACHE_DIR` unset → `${ZISK_HOME}/cache`
///
/// File locations by mode:
///
/// | Field               | User mode (Linux/macOS)               | Service mode (Linux)                       | Service mode (macOS)                                        |
/// |---------------------|---------------------------------------|--------------------------------------------|-------------------------------------------------------------|
/// | `home`              | `$HOME/.zisk`                         | `/opt/zisk`                                | `/Library/Application Support/ZisK`                         |
/// | `bin`               | `$HOME/.zisk/bin`                     | `/opt/zisk/bin`                            | `/Library/Application Support/ZisK/bin`                     |
/// | `cache`             | `$HOME/.zisk/cache`                   | `/var/lib/zisk-{coordinator,worker}/cache` | `/Library/Application Support/ZisK/cache`                   |
/// | `proving_key`       | `$HOME/.zisk/provingKey`              | `/opt/zisk/provingKey`                     | `/Library/Application Support/ZisK/provingKey`              |
/// | `proving_key_snark` | `$HOME/.zisk/provingKeySnark`         | `/opt/zisk/provingKeySnark`                | `/Library/Application Support/ZisK/provingKeySnark`         |
/// | `toolchains`        | `$HOME/.zisk/toolchains`              | `/opt/zisk/toolchains`                     | `/Library/Application Support/ZisK/toolchains`              |
/// | `verify_key`        | `$HOME/.zisk/verifyKey`               | `/opt/zisk/verifyKey`                      | `/Library/Application Support/ZisK/verifyKey`               |
/// | `zisk`              | `$HOME/.zisk/zisk`                    | `/opt/zisk/zisk`                           | `/Library/Application Support/ZisK/zisk`                    |
/// | `libziskclib`       | `$HOME/.zisk/bin/libziskclib.a`       | `/opt/zisk/bin/libziskclib.a`              | `/Library/Application Support/ZisK/bin/libziskclib.a`       |
/// | `emulator_asm`      | `$HOME/.zisk/zisk/emulator-asm`       | `/opt/zisk/zisk/emulator-asm`              | `/Library/Application Support/ZisK/zisk/emulator-asm`       |
///
/// In service mode, `home` is read-only bundle state (set via `ZISK_HOME`)
/// while `cache` is per-service writable state (set via `ZISK_CACHE_DIR`).
/// `toolchains/` holds rustup-linked Zisk Rust toolchains and is managed by
/// ziskup; the field is exposed so callers can locate it without hardcoding
/// the substring.
///
/// So a fully-populated user-mode `~/.zisk` looks like:
///   `bin/  cache/  provingKey/  [provingKeySnark/]  toolchains/  [verifyKey/]  zisk/`
#[derive(Clone, Debug)]
pub struct ZiskPaths {
    pub home: PathBuf,
    pub bin: PathBuf,
    pub cache: PathBuf,
    pub proving_key: PathBuf,
    pub proving_key_snark: PathBuf,
    pub toolchains: PathBuf,
    pub verify_key: PathBuf,
    pub zisk: PathBuf,
    pub libziskclib: PathBuf,
    pub emulator_asm: PathBuf,
}

impl ZiskPaths {
    pub fn from_env() -> Self {
        let home = env::var_os("ZISK_HOME").map(PathBuf::from).unwrap_or_else(default_home);
        let bin = home.join(BIN_DIR);
        let cache = env::var_os("ZISK_CACHE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(CACHE_DIR));
        let proving_key = home.join(PROVING_KEY_DIR);
        let proving_key_snark = home.join(PROVING_KEY_SNARK_DIR);
        let toolchains = home.join(TOOLCHAINS_DIR);
        let verify_key = home.join(VERIFY_KEY_DIR);
        let zisk = home.join(ZISK_DIR);
        let libziskclib = bin.join(LIBZISKCLIB_FILE);
        let emulator_asm = zisk.join(EMULATOR_ASM_DIR);

        Self {
            libziskclib,
            emulator_asm,
            proving_key,
            proving_key_snark,
            bin,
            zisk,
            toolchains,
            verify_key,
            cache,
            home,
        }
    }

    pub fn global() -> &'static Self {
        static INSTANCE: OnceLock<ZiskPaths> = OnceLock::new();
        INSTANCE.get_or_init(Self::from_env)
    }

    /// Content-addressed ELF cache path for a given hash.
    ///
    /// Layout: `${cache}/{hash_id}.elf`.
    pub fn elf_cache(&self, hash_id: &str) -> PathBuf {
        self.cache.join(format!("{}.elf", hash_id))
    }
}

fn default_home() -> PathBuf {
    dirs::home_dir()
        .expect("HOME directory not resolvable; set ZISK_HOME explicitly")
        .join(USER_HOME_SUBDIR)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Tests mutate process env vars; serialize to avoid cross-test races.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_env<F: FnOnce()>(vars: &[(&str, Option<&str>)], f: F) {
        let _guard = ENV_LOCK.lock().unwrap();
        let prev: Vec<(String, Option<String>)> =
            vars.iter().map(|(k, _)| (k.to_string(), env::var(k).ok())).collect();
        for (k, v) in vars {
            match v {
                Some(val) => env::set_var(k, val),
                None => env::remove_var(k),
            }
        }
        f();
        for (k, v) in prev {
            match v {
                Some(val) => env::set_var(&k, val),
                None => env::remove_var(&k),
            }
        }
    }

    #[test]
    fn defaults_to_home_dot_zisk() {
        with_env(&[("ZISK_HOME", None), ("ZISK_CACHE_DIR", None)], || {
            let p = ZiskPaths::from_env();
            let expected_home = dirs::home_dir().unwrap().join(USER_HOME_SUBDIR);
            assert_eq!(p.home, expected_home);
            assert_eq!(p.bin, expected_home.join(BIN_DIR));
            assert_eq!(p.zisk, expected_home.join(ZISK_DIR));
            assert_eq!(p.toolchains, expected_home.join(TOOLCHAINS_DIR));
            assert_eq!(p.verify_key, expected_home.join(VERIFY_KEY_DIR));
            assert_eq!(p.cache, expected_home.join(CACHE_DIR));
            assert_eq!(p.proving_key, expected_home.join(PROVING_KEY_DIR));
            assert_eq!(p.proving_key_snark, expected_home.join(PROVING_KEY_SNARK_DIR));
            assert_eq!(p.libziskclib, expected_home.join(BIN_DIR).join(LIBZISKCLIB_FILE));
            assert_eq!(p.emulator_asm, expected_home.join(ZISK_DIR).join(EMULATOR_ASM_DIR));
        });
    }

    #[test]
    fn zisk_home_overrides_root() {
        with_env(&[("ZISK_HOME", Some(LINUX_SERVICE_HOME)), ("ZISK_CACHE_DIR", None)], || {
            let p = ZiskPaths::from_env();
            let home = PathBuf::from(LINUX_SERVICE_HOME);
            assert_eq!(p.home, home);
            assert_eq!(p.bin, home.join(BIN_DIR));
            assert_eq!(p.zisk, home.join(ZISK_DIR));
            assert_eq!(p.toolchains, home.join(TOOLCHAINS_DIR));
            assert_eq!(p.verify_key, home.join(VERIFY_KEY_DIR));
            assert_eq!(p.cache, home.join(CACHE_DIR));
            assert_eq!(p.proving_key, home.join(PROVING_KEY_DIR));
            assert_eq!(p.libziskclib, home.join(BIN_DIR).join(LIBZISKCLIB_FILE));
            assert_eq!(p.emulator_asm, home.join(ZISK_DIR).join(EMULATOR_ASM_DIR));
        });
    }

    #[test]
    fn cache_dir_independent_of_home() {
        with_env(
            &[
                ("ZISK_HOME", Some(LINUX_SERVICE_HOME)),
                ("ZISK_CACHE_DIR", Some("/var/lib/zisk-worker/cache")),
            ],
            || {
                let p = ZiskPaths::from_env();
                assert_eq!(p.home, PathBuf::from(LINUX_SERVICE_HOME));
                assert_eq!(p.cache, PathBuf::from("/var/lib/zisk-worker/cache"));
                // Other paths still derive from home.
                assert_eq!(p.bin, PathBuf::from(LINUX_SERVICE_HOME).join(BIN_DIR));
            },
        );
    }

    #[test]
    fn cache_dir_alone_falls_back_to_default_home() {
        with_env(&[("ZISK_HOME", None), ("ZISK_CACHE_DIR", Some("/tmp/zisk-cache"))], || {
            let p = ZiskPaths::from_env();
            assert_eq!(p.cache, PathBuf::from("/tmp/zisk-cache"));
            assert_eq!(p.home, dirs::home_dir().unwrap().join(USER_HOME_SUBDIR));
        });
    }

    #[test]
    fn elf_cache_path_uses_cache_dir() {
        with_env(
            &[
                ("ZISK_HOME", Some(LINUX_SERVICE_HOME)),
                ("ZISK_CACHE_DIR", Some("/var/lib/zisk-worker/cache")),
            ],
            || {
                let p = ZiskPaths::from_env();
                assert_eq!(
                    p.elf_cache("abc123"),
                    PathBuf::from("/var/lib/zisk-worker/cache/abc123.elf")
                );
            },
        );
    }

    #[test]
    fn macos_apple_application_support_path() {
        with_env(&[("ZISK_HOME", Some(MACOS_SERVICE_HOME)), ("ZISK_CACHE_DIR", None)], || {
            let p = ZiskPaths::from_env();
            let home = PathBuf::from(MACOS_SERVICE_HOME);
            assert_eq!(p.home, home);
            assert_eq!(p.bin, home.join(BIN_DIR));
            assert_eq!(p.libziskclib, home.join(BIN_DIR).join(LIBZISKCLIB_FILE));
        });
    }
}
