use anyhow::Result;
use rom_setup::{rom_merkle_setup_verkey, HashMode};
use std::borrow::Cow;
use std::fs;
use std::path::Path;
use zisk_common::io::ZiskStdin;
use zisk_common::ProgramVK;
use zisk_core::Riscv2zisk;
use ziskemu::ZiskEmulator;
pub use ziskemu::{EmuOptions, ProfilingMode};

/// Program identifier containing name and hash
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProgramId {
    pub name: Cow<'static, str>,
    pub hash_id: Cow<'static, str>,
}

impl ProgramId {
    /// Create a new ProgramId from static strings (const-compatible)
    pub const fn new_static(name: &'static str, hash_id: &'static str) -> Self {
        Self { name: Cow::Borrowed(name), hash_id: Cow::Borrowed(hash_id) }
    }

    pub fn get_hash(&self) -> &str {
        &self.hash_id
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }
}

impl std::fmt::Display for ProgramId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.name, self.hash_id)
    }
}

/// ELF binary data
#[derive(Debug, Clone)]
pub struct Elf {
    pub data: Cow<'static, [u8]>,
}

impl Elf {
    /// Create a new ELF from embedded static data (const-compatible)
    pub const fn from_embedded(bytes: &'static [u8]) -> Self {
        Self { data: Cow::Borrowed(bytes) }
    }

    /// Create a new ELF from owned data (for dynamic loading)
    pub fn new(data: Vec<u8>) -> Self {
        Self { data: Cow::Owned(data) }
    }
}

/// Guest program that can be executed and proven with Zisk
#[derive(Clone, Debug)]
pub struct GuestProgram {
    pub program_id: ProgramId,
    pub elf: Elf,
}

impl GuestProgram {
    /// Construct a `GuestProgram` from a static name, precomputed hash, and embedded ELF bytes.
    ///
    /// `const`-compatible. This is the constructor emitted by the `load_program!` proc macro,
    /// which computes the blake3 hash at compile time so the result can be a plain `const`/`static`
    /// (no runtime hashing, no `LazyLock`). Prefer the macro over calling this directly.
    #[doc(hidden)]
    pub const fn from_static(
        name: &'static str,
        hash_id: &'static str,
        elf: &'static [u8],
    ) -> Self {
        Self { program_id: ProgramId::new_static(name, hash_id), elf: Elf::from_embedded(elf) }
    }

    /// Create a new guest program from a URI (file://, http://, or plain path)
    pub fn from_uri(uri: &str) -> Result<Self> {
        let path = if let Some(pos) = uri.find("://") {
            let (scheme, rest) = uri.split_at(pos);
            let rest = &rest[3..]; // Skip "://"

            match scheme {
                "file" => rest,
                "http" | "https" => {
                    return Err(anyhow::anyhow!(
                        "HTTP loading not yet implemented. Use the 'http' feature to enable."
                    ));
                }
                _ => return Err(anyhow::anyhow!("Unknown URI scheme: {}", scheme)),
            }
        } else {
            uri
        };

        let elf_data = fs::read(path)
            .map_err(|e| anyhow::anyhow!("Error reading ELF file {}: {}", path, e))?;

        let name =
            Path::new(path).file_stem().and_then(|s| s.to_str()).unwrap_or("unknown").to_string();

        let hash_id = blake3::hash(&elf_data).to_hex().to_string();

        Ok(Self {
            program_id: ProgramId { name: Cow::Owned(name), hash_id: Cow::Owned(hash_id) },
            elf: Elf::new(elf_data),
        })
    }

    /// Create a new guest program directly from ELF bytes and a name, without reading from disk.
    pub fn from_bytes(name: impl Into<String>, elf_data: Vec<u8>) -> Self {
        let name = name.into();
        let hash_id = blake3::hash(&elf_data).to_hex().to_string();
        Self {
            program_id: ProgramId { name: Cow::Owned(name), hash_id: Cow::Owned(hash_id) },
            elf: Elf::new(elf_data),
        }
    }

    /// Get the ELF binary bytes
    pub fn elf(&self) -> &[u8] {
        &self.elf.data
    }

    /// Get the program name
    pub fn name(&self) -> &str {
        &self.program_id.name
    }

    /// Get the program ID (project name + program name)
    pub fn program_id(&self) -> &ProgramId {
        &self.program_id
    }

    /// Get the computed hash of the ELF binary
    pub fn hash(&self) -> &str {
        &self.program_id.hash_id
    }
    /// Verkey from the ELF, using the default [`HashMode`] (`Poseidon1`).
    ///
    /// For a key built with a different mode, use [`vk_with_mode`](Self::vk_with_mode);
    /// otherwise the verkey is rejected at verify time against that key's proofs.
    pub fn vk(&self) -> Result<ProgramVK> {
        self.vk_with_mode(HashMode::default())
    }

    /// Verkey from the ELF, under the proving key's [`HashMode`].
    pub fn vk_with_mode(&self, hash_mode: HashMode) -> Result<ProgramVK> {
        rom_merkle_setup_verkey(self.elf(), &None, hash_mode)
    }

    /// Run the ZisK emulator with the given stdin.
    ///
    /// Pass `Some(ProfilingMode)` to enable profiling output, or `None` for a plain run.
    pub fn run_emulation(&self, stdin: ZiskStdin, profiling: Option<ProfilingMode>) -> Result<()> {
        let riscv2zisk = Riscv2zisk::new(self.elf());

        let zisk_rom = riscv2zisk
            .run()
            .map_err(|e| anyhow::anyhow!("Failed to convert ELF to ZISK ROM: {e:?}"))?;

        let callback = None::<Box<dyn Fn(zisk_common::EmuTrace)>>;

        let inputs = stdin.read_data();

        let mut options = EmuOptions::default();
        // Temporary file written only when profiling needs symbol resolution.
        // Stored in a variable so it lives until after process_rom returns.
        let _tmp_elf;
        if let Some(mode) = profiling {
            options.apply_profiling(mode);
            let tmp_path =
                std::env::temp_dir().join(format!("zisk_elf_{}.elf", self.program_id.hash_id));
            if std::fs::write(&tmp_path, self.elf()).is_ok() {
                options.elf = Some(tmp_path.to_string_lossy().into_owned());
                _tmp_elf = Some(tmp_path);
            } else {
                _tmp_elf = None;
            }
        } else {
            _tmp_elf = None;
        }

        let result = ZiskEmulator::process_rom(&zisk_rom, &inputs, &options, callback);

        // Clean up temp ELF file used for symbol resolution
        if let Some(path) = _tmp_elf {
            let _ = std::fs::remove_file(path);
        }

        match result {
            Ok(_) => {
                if let Some(path) = &options.profiler_output {
                    println!("Profiler output written to: {path}");
                }
                Ok(())
            }
            Err(e) => {
                eprintln!("Error during emulation: {e:?}");
                Err(anyhow::anyhow!("Emulation failed"))
            }
        }
    }
}

// `load_program!` is now a proc macro defined in the `zisk-program-macros` crate (re-exported
// from this crate's root). For fully dynamic loading from a file path at runtime, use
// `GuestProgram::from_uri()` instead.
