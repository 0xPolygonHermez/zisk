use anyhow::Result;
use std::fmt::Write;
use std::fs;
use std::path::Path;
use zisk_common::io::ZiskStdin;
use zisk_core::Riscv2zisk;
pub use ziskemu::EmuOptions;
use ziskemu::ZiskEmulator;

/// Program identifier containing project and program names
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramId {
    pub project_name: String,
    pub program_name: String,
}

/// ELF binary with computed hash
#[derive(Debug, Clone)]
pub struct Elf {
    data: Vec<u8>,
    hash_id: String,
}

impl Elf {
    /// Create a new ELF with automatic hash computation
    pub fn new(data: Vec<u8>) -> Self {
        let hash_id = blake3::hash(&data).to_hex().to_string();
        Self { data, hash_id }
    }
}

/// Embedded guest program data (returned by include_guest_elf! macro)
#[derive(Debug, Clone, Copy)]
pub struct EmbeddedGuestElf {
    pub elf_bytes: &'static [u8],
    pub program_name: &'static str,
    pub project_name: &'static str,
    pub uri: &'static str,
}

/// Guest program that can be executed and proven with Zisk
#[derive(Clone, Debug)]
pub struct GuestProgram {
    program_id: ProgramId,
    elf: Elf,
}

impl GuestProgram {
    /// Create a new guest program from embedded ELF data
    pub fn from_elf(elf: EmbeddedGuestElf) -> Self {
        Self {
            program_id: ProgramId {
                project_name: elf.project_name.to_string(),
                program_name: elf.program_name.to_string(),
            },
            elf: Elf::new(elf.elf_bytes.to_vec()),
        }
    }

    /// Create a new guest program from a URI (file://, http://, or plain path)
    pub fn from_uri(uri: &str, project_name: String) -> Result<Self> {
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

        let program_name =
            Path::new(path).file_stem().and_then(|s| s.to_str()).unwrap_or("unknown").to_string();

        Ok(Self { program_id: ProgramId { project_name, program_name }, elf: Elf::new(elf_data) })
    }

    /// Get the ELF binary bytes
    pub fn elf(&self) -> &[u8] {
        &self.elf.data
    }

    /// Get the program name
    pub fn name(&self) -> &str {
        &self.program_id.program_name
    }

    /// Get the project name
    pub fn project_name(&self) -> &str {
        &self.program_id.project_name
    }

    /// Get the program ID (project name + program name)
    pub fn program_id(&self) -> &ProgramId {
        &self.program_id
    }

    /// Get the computed hash of the ELF binary
    pub fn hash(&self) -> &str {
        &self.elf.hash_id
    }

    /// Run the ZisK emulator with the given stdin and options
    pub fn run(&self, stdin: ZiskStdin, options: &EmuOptions) -> Result<()> {
        let riscv2zisk = Riscv2zisk::new(self.elf());

        let zisk_rom = riscv2zisk
            .run()
            .map_err(|e| anyhow::anyhow!("Failed to convert ELF to ZISK ROM: {e:?}"))?;

        let callback = None::<Box<dyn Fn(zisk_common::EmuTrace)>>;

        let inputs = stdin.read_raw_bytes();

        // LATER: READ SYMBOLS
        let result = ZiskEmulator::process_rom(&zisk_rom, &inputs, options, callback);
        match result {
            Ok(result) => {
                result.iter().fold(String::new(), |mut acc, byte| {
                    write!(&mut acc, "{byte:02x}").unwrap();
                    acc
                });
                Ok(())
            }
            Err(e) => {
                eprintln!("Error during emulation: {e:?}");
                Err(anyhow::anyhow!("Emulation failed"))
            }
        }
    }
}

/// Macro to include guest program data at compile time
///
/// Returns an `EmbeddedGuestElf` struct containing:
/// - `elf_bytes`: &'static [u8]
/// - `program_name`: &'static str
/// - `project_name`: &'static str
/// - `uri`: &'static str (path to the ELF file)
///
/// This macro uses environment variables set by zisk-build:
/// - `ZISK_ELF_{name}`: Path to the ELF file
/// - `CARGO_PKG_NAME`: Project name from Cargo.toml
///
/// # Example
/// ```ignore
/// let embedded = include_guest_elf!("my_program");
/// let program = GuestProgram::from_embedded(embedded);
/// ```
#[macro_export]
macro_rules! include_guest_elf {
    ($arg:literal) => {{
        $crate::EmbeddedGuestElf {
            elf_bytes: include_bytes!(env!(concat!("ZISK_ELF_", $arg))),
            program_name: $arg,
            project_name: match option_env!("CARGO_PKG_NAME") {
                Some(name) => name,
                None => "unknown",
            },
            uri: env!(concat!("ZISK_ELF_", $arg)),
        }
    }};
}
