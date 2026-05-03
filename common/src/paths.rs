use std::env;
use std::path::PathBuf;
use std::sync::OnceLock;

/// Layout-independent view over the ZisK install directory tree.
///
/// All callers should go through `ZiskPaths::global()` rather than
/// reading `HOME` or hardcoding `.zisk` substrings. This lets the same
/// binary serve both user-mode (`$HOME/.zisk/...`) and service-mode
/// (`/opt/zisk/...` + `/var/lib/zisk-*/cache`) installs by reading
/// `ZISK_HOME` and `ZISK_CACHE_DIR` env vars.
///
/// Defaults preserve current user-mode behavior:
/// - `ZISK_HOME` unset → `$HOME/.zisk`
/// - `ZISK_CACHE_DIR` unset → `${ZISK_HOME}/cache`
#[derive(Clone, Debug)]
pub struct ZiskPaths {
    pub home: PathBuf,
    pub bin: PathBuf,
    pub share: PathBuf,
    pub proving_key: PathBuf,
    pub proving_key_snark: PathBuf,
    pub cache: PathBuf,
    pub libziskclib: PathBuf,
    pub emulator_asm: PathBuf,
}

impl ZiskPaths {
    pub fn from_env() -> Self {
        let home = env::var_os("ZISK_HOME").map(PathBuf::from).unwrap_or_else(default_home);
        let cache =
            env::var_os("ZISK_CACHE_DIR").map(PathBuf::from).unwrap_or_else(|| home.join("cache"));
        let bin = home.join("bin");
        let share = home.join("zisk");
        Self {
            libziskclib: bin.join("libziskclib.a"),
            emulator_asm: share.join("emulator-asm"),
            proving_key: home.join("provingKey"),
            proving_key_snark: home.join("provingKeySnark"),
            bin,
            share,
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
    dirs::home_dir().expect("HOME directory not resolvable; set ZISK_HOME explicitly").join(".zisk")
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
            let expected_home = dirs::home_dir().unwrap().join(".zisk");
            assert_eq!(p.home, expected_home);
            assert_eq!(p.bin, expected_home.join("bin"));
            assert_eq!(p.share, expected_home.join("zisk"));
            assert_eq!(p.cache, expected_home.join("cache"));
            assert_eq!(p.proving_key, expected_home.join("provingKey"));
            assert_eq!(p.proving_key_snark, expected_home.join("provingKeySnark"));
            assert_eq!(p.libziskclib, expected_home.join("bin/libziskclib.a"));
            assert_eq!(p.emulator_asm, expected_home.join("zisk/emulator-asm"));
        });
    }

    #[test]
    fn zisk_home_overrides_root() {
        with_env(&[("ZISK_HOME", Some("/opt/zisk")), ("ZISK_CACHE_DIR", None)], || {
            let p = ZiskPaths::from_env();
            assert_eq!(p.home, PathBuf::from("/opt/zisk"));
            assert_eq!(p.bin, PathBuf::from("/opt/zisk/bin"));
            assert_eq!(p.share, PathBuf::from("/opt/zisk/zisk"));
            assert_eq!(p.cache, PathBuf::from("/opt/zisk/cache"));
            assert_eq!(p.proving_key, PathBuf::from("/opt/zisk/provingKey"));
            assert_eq!(p.libziskclib, PathBuf::from("/opt/zisk/bin/libziskclib.a"));
            assert_eq!(p.emulator_asm, PathBuf::from("/opt/zisk/zisk/emulator-asm"));
        });
    }

    #[test]
    fn cache_dir_independent_of_home() {
        with_env(
            &[
                ("ZISK_HOME", Some("/opt/zisk")),
                ("ZISK_CACHE_DIR", Some("/var/lib/zisk-worker/cache")),
            ],
            || {
                let p = ZiskPaths::from_env();
                assert_eq!(p.home, PathBuf::from("/opt/zisk"));
                assert_eq!(p.cache, PathBuf::from("/var/lib/zisk-worker/cache"));
                // Other paths still derive from home.
                assert_eq!(p.bin, PathBuf::from("/opt/zisk/bin"));
            },
        );
    }

    #[test]
    fn cache_dir_alone_falls_back_to_default_home() {
        with_env(&[("ZISK_HOME", None), ("ZISK_CACHE_DIR", Some("/tmp/zisk-cache"))], || {
            let p = ZiskPaths::from_env();
            assert_eq!(p.cache, PathBuf::from("/tmp/zisk-cache"));
            assert_eq!(p.home, dirs::home_dir().unwrap().join(".zisk"));
        });
    }

    #[test]
    fn elf_cache_path_uses_cache_dir() {
        with_env(
            &[
                ("ZISK_HOME", Some("/opt/zisk")),
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
        with_env(
            &[("ZISK_HOME", Some("/Library/Application Support/ZisK")), ("ZISK_CACHE_DIR", None)],
            || {
                let p = ZiskPaths::from_env();
                assert_eq!(p.home, PathBuf::from("/Library/Application Support/ZisK"));
                assert_eq!(p.bin, PathBuf::from("/Library/Application Support/ZisK/bin"));
                assert_eq!(
                    p.libziskclib,
                    PathBuf::from("/Library/Application Support/ZisK/bin/libziskclib.a")
                );
            },
        );
    }
}
