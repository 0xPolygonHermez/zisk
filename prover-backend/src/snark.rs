//! The protocol of the final SNARK a SNARK proving key is set up for.

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

use anyhow::{anyhow, Result};
use proofman::FinalSnarkKey;
use proofman_common::final_witness::FinalWitnessLibrary;
use zisk_common::SnarkProtocol;

/// The protocol ids a zkey names in its header, snarkjs's (proofman's `SnarkProtocol` ids).
const ZKEY_PLONK: u32 = 2;
const ZKEY_FFLONK: u32 = 10;

/// The protocol of the final SNARK key of `proving_key_snark`, found as the prover finds it.
/// Errors when it has no usable final SNARK key, or one of a protocol ZisK does not prove with.
pub fn snark_protocol_of_key(proving_key_snark: &Path) -> Result<SnarkProtocol> {
    let refuse =
        |found: String| anyhow!("the SNARK proving key {} {found}", proving_key_snark.display());
    match FinalSnarkKey::find(&proving_key_snark.join("final").join("final")) {
        Ok(FinalSnarkKey::Pilfflonk(_)) => Ok(SnarkProtocol::Pilfflonk),
        Ok(FinalSnarkKey::Zkey(zkey)) => match zkey_protocol(&zkey) {
            Ok(ZKEY_PLONK) => Ok(SnarkProtocol::Plonk),
            Ok(ZKEY_FFLONK) => Err(refuse("is set up for FFLONK".to_string())),
            Ok(id) => Err(refuse(format!("names protocol {id} in {}", zkey.display()))),
            Err(e) => Err(refuse(format!("has an unreadable {}: {e}", zkey.display()))),
        },
        Err(e) => Err(refuse(format!("has no usable final SNARK key: {e}"))),
    }
}

/// Errors unless the final SNARK key of `proving_key_snark` is set up for `protocol`.
pub fn ensure_snark_key(protocol: SnarkProtocol, proving_key_snark: &Path) -> Result<()> {
    match snark_protocol_of_key(proving_key_snark) {
        Ok(found) if found == protocol => Ok(()),
        Ok(found) => Err(anyhow!(
            "a {} final SNARK was asked for, but the SNARK proving key {} is set up for {}",
            protocol.name(),
            proving_key_snark.display(),
            found.name()
        )),
        Err(e) => Err(anyhow!("a {} final SNARK was asked for, but {e}", protocol.name())),
    }
}

/// Errors unless the witness library of the final SNARK circuit of `proving_key_snark`
/// (`final/final.so`) loads, as the wrap loads it once the STARK proofs are done. Checked before
/// any proving, so that a library the wrap cannot load fails the run at startup, with the error the
/// wrap would give. A key with no witness library is left to the wrap, which reports it.
pub fn ensure_final_witness_loads(proving_key_snark: &Path) -> Result<()> {
    let (library, dat) = FinalWitnessLibrary::paths(&proving_key_snark.join("final").join("final"));
    if !library.exists() {
        return Ok(());
    }
    let Err(e) = FinalWitnessLibrary::load(&library, &dat) else {
        return Ok(());
    };
    if !requires_executable_stack(&library).unwrap_or(false) {
        return Err(anyhow!("{e}"));
    }
    Err(anyhow!(
        "{e}\n{} requests an executable stack (GNU_STACK RWE), which glibc 2.41 and later refuse to \
         load. Use a SNARK proving key whose final.so is built without an executable stack, or set \
         GLIBC_TUNABLES=glibc.rtld.execstack=2, which allows executable stacks for the whole process.",
        library.display()
    ))
}

/// Whether the ELF object at `path` asks for an executable stack: its `PT_GNU_STACK` program
/// header has `PF_X` set. False for a file that is not a 64-bit little-endian ELF, and for one
/// without that header.
fn requires_executable_stack(path: &Path) -> io::Result<bool> {
    const PT_GNU_STACK: u32 = 0x6474_e551;
    const PF_X: u32 = 1;

    let mut file = File::open(path)?;
    let mut ident = [0u8; 6];
    file.read_exact(&mut ident)?;
    // ELFCLASS64, ELFDATA2LSB.
    if &ident[..4] != b"\x7fELF" || ident[4] != 2 || ident[5] != 1 {
        return Ok(false);
    }
    file.seek(SeekFrom::Start(0x20))?;
    let phoff = read_u64(&mut file)?;
    file.seek(SeekFrom::Start(0x36))?;
    let mut sizes = [0u8; 4];
    file.read_exact(&mut sizes)?;
    let phentsize = u16::from_le_bytes([sizes[0], sizes[1]]) as u64;
    let phnum = u16::from_le_bytes([sizes[2], sizes[3]]) as u64;
    for i in 0..phnum {
        file.seek(SeekFrom::Start(phoff.saturating_add(i * phentsize)))?;
        if read_u32(&mut file)? == PT_GNU_STACK {
            return Ok(read_u32(&mut file)? & PF_X != 0);
        }
    }
    Ok(false)
}

/// The protocol id of a zkey: the first `u32` of its header section (1), which need not be the
/// first section. Reads the section table only, not the key.
fn zkey_protocol(zkey: &Path) -> io::Result<u32> {
    let mut file = File::open(zkey)?;
    let mut magic = [0u8; 4];
    file.read_exact(&mut magic)?;
    if &magic != b"zkey" {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "not a zkey"));
    }
    let _version = read_u32(&mut file)?;
    let sections = read_u32(&mut file)?;
    for _ in 0..sections {
        let id = read_u32(&mut file)?;
        let size = read_u64(&mut file)?;
        if id == 1 {
            return read_u32(&mut file);
        }
        file.seek(SeekFrom::Current(i64::try_from(size).map_err(io::Error::other)?))?;
    }
    Err(io::Error::new(io::ErrorKind::InvalidData, "no header section"))
}

fn read_u32(file: &mut File) -> io::Result<u32> {
    let mut bytes = [0u8; 4];
    file.read_exact(&mut bytes)?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_u64(file: &mut File) -> io::Result<u64> {
    let mut bytes = [0u8; 8];
    file.read_exact(&mut bytes)?;
    Ok(u64::from_le_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::{
        ensure_final_witness_loads, requires_executable_stack, snark_protocol_of_key,
        FinalWitnessLibrary, ZKEY_FFLONK, ZKEY_PLONK,
    };
    use std::path::{Path, PathBuf};
    use zisk_common::SnarkProtocol;

    /// A `provingKeySnark/` in a fresh directory, removed on drop.
    struct SnarkKeyDir(PathBuf);

    impl SnarkKeyDir {
        fn new(tag: &str) -> Self {
            let dir = std::env::temp_dir()
                .join(format!("prover_backend_snark_key_{tag}_{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(dir.join("final")).unwrap();
            Self(dir)
        }

        /// A `final.zkey` of `protocol`, as far as its section table: an empty section 3 before
        /// the header section, as a PLONK zkey has.
        fn with_zkey(tag: &str, protocol: u32) -> Self {
            let key = Self::new(tag);
            let mut zkey = b"zkey".to_vec();
            zkey.extend(1u32.to_le_bytes()); // version
            zkey.extend(2u32.to_le_bytes()); // sections
            zkey.extend(3u32.to_le_bytes());
            zkey.extend(0u64.to_le_bytes());
            zkey.extend(1u32.to_le_bytes());
            zkey.extend(4u64.to_le_bytes());
            zkey.extend(protocol.to_le_bytes());
            std::fs::write(key.0.join("final").join("final.zkey"), zkey).unwrap();
            key
        }
    }

    impl Drop for SnarkKeyDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn refusal(key: &SnarkKeyDir) -> String {
        snark_protocol_of_key(&key.0).unwrap_err().to_string()
    }

    #[test]
    fn a_plonk_zkey_is_a_plonk_key() {
        let key = SnarkKeyDir::with_zkey("plonk", ZKEY_PLONK);
        assert_eq!(snark_protocol_of_key(&key.0).unwrap(), SnarkProtocol::Plonk);
    }

    #[test]
    fn an_fflonk_key_is_refused() {
        let key = SnarkKeyDir::with_zkey("fflonk", ZKEY_FFLONK);
        assert!(refusal(&key).ends_with("is set up for FFLONK"));
    }

    #[test]
    fn a_zkey_of_another_protocol_is_refused() {
        let key = SnarkKeyDir::with_zkey("groth16", 1);
        assert!(refusal(&key).contains("names protocol 1 in"));
    }

    #[test]
    fn a_zkey_that_is_not_one_is_refused() {
        let key = SnarkKeyDir::new("garbage");
        std::fs::write(key.0.join("final").join("final.zkey"), "garbage").unwrap();
        assert!(refusal(&key).contains("has an unreadable"));
    }

    #[test]
    fn a_key_without_a_final_snark_is_refused() {
        let key = SnarkKeyDir::new("empty");
        assert!(refusal(&key).contains("has no usable final SNARK key"));
    }

    /// `ZISK_TEST_PILFFLONK_KEY`: a `provingKeySnark/` set up for pilfflonk.
    #[test]
    #[ignore = "needs a pilfflonk SNARK proving key, in ZISK_TEST_PILFFLONK_KEY"]
    fn a_pilfflonk_key_is_a_pilfflonk_key() {
        let key = PathBuf::from(std::env::var_os("ZISK_TEST_PILFFLONK_KEY").unwrap());
        assert_eq!(snark_protocol_of_key(&key).unwrap(), SnarkProtocol::Pilfflonk);
    }

    /// The headers of a 64-bit little-endian ELF with one program header of type `p_type` and
    /// flags `p_flags`, after a `PT_LOAD` one: no loader takes it.
    fn elf_with_program_header(p_type: u32, p_flags: u32) -> Vec<u8> {
        let mut elf = vec![0u8; 64 + 2 * 56];
        elf[..6].copy_from_slice(b"\x7fELF\x02\x01");
        elf[0x20..0x28].copy_from_slice(&64u64.to_le_bytes()); // e_phoff
        elf[0x36..0x38].copy_from_slice(&56u16.to_le_bytes()); // e_phentsize
        elf[0x38..0x3a].copy_from_slice(&2u16.to_le_bytes()); // e_phnum
        elf[64..68].copy_from_slice(&1u32.to_le_bytes()); // PT_LOAD
        elf[120..124].copy_from_slice(&p_type.to_le_bytes());
        elf[124..128].copy_from_slice(&p_flags.to_le_bytes());
        elf
    }

    const PT_GNU_STACK: u32 = 0x6474_e551;
    const RW: u32 = 6;
    const RWE: u32 = 7;

    /// The witness library of `key`, `final/final.so` (`final.dylib` on macOS).
    fn final_library(key: &SnarkKeyDir) -> PathBuf {
        FinalWitnessLibrary::paths(&key.0.join("final").join("final")).0
    }

    fn executable_stack(key: &SnarkKeyDir, contents: &[u8]) -> bool {
        let library = final_library(key);
        std::fs::write(&library, contents).unwrap();
        requires_executable_stack(Path::new(&library)).unwrap()
    }

    #[test]
    fn an_executable_stack_is_found_in_gnu_stack_only() {
        let key = SnarkKeyDir::new("gnu_stack");
        assert!(executable_stack(&key, &elf_with_program_header(PT_GNU_STACK, RWE)));
        assert!(!executable_stack(&key, &elf_with_program_header(PT_GNU_STACK, RW)));
        assert!(!executable_stack(&key, &elf_with_program_header(2, RWE)));
        assert!(!executable_stack(&key, b"not an ELF file"));
    }

    #[test]
    fn a_key_without_a_final_witness_library_is_left_to_the_wrap() {
        let key = SnarkKeyDir::new("no_library");
        ensure_final_witness_loads(&key.0).unwrap();
    }

    #[test]
    fn a_final_witness_library_that_does_not_load_is_refused() {
        let key = SnarkKeyDir::new("not_a_library");
        std::fs::write(final_library(&key), b"not an ELF file").unwrap();
        let message = ensure_final_witness_loads(&key.0).unwrap_err().to_string();
        assert!(message.starts_with("Dynamic library error:"), "{message}");
        assert!(!message.contains("GNU_STACK"), "{message}");
    }

    #[test]
    fn a_final_witness_library_with_an_executable_stack_says_so() {
        let key = SnarkKeyDir::new("execstack");
        std::fs::write(final_library(&key), elf_with_program_header(PT_GNU_STACK, RWE)).unwrap();
        let message = ensure_final_witness_loads(&key.0).unwrap_err().to_string();
        assert!(message.starts_with("Dynamic library error:"), "{message}");
        assert!(message.contains("requests an executable stack (GNU_STACK RWE)"), "{message}");
        assert!(message.contains("GLIBC_TUNABLES=glibc.rtld.execstack=2"), "{message}");
    }
}
