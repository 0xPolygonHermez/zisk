use std::fmt::Write;
use zisk_common::io::{ZiskIO, ZiskStdin};
use zisk_common::ElfBinaryLike;
use zisk_core::Riscv2zisk;
pub use ziskemu::EmuOptions;
use ziskemu::ZiskEmulator;

pub fn ziskemu(
    elf: &impl ElfBinaryLike,
    stdin: ZiskStdin,
    options: &EmuOptions,
) -> anyhow::Result<()> {
    let riscv2zisk = Riscv2zisk::new(elf.elf());

    let zisk_rom = riscv2zisk
        .run()
        .map_err(|e| anyhow::anyhow!("Failed to convert ELF to ZISK ROM: {e:?}"))?;

    let callback = None::<Box<dyn Fn(zisk_common::EmuTrace)>>;

    let inputs = stdin.read_bytes();

    let options = EmuOptions { log_output: true, ..options.clone() };
    let result = ZiskEmulator::process_rom(&zisk_rom, &inputs, &options, callback);
    match result {
        Ok(result) => {
            // println!("Emulation completed successfully");
            result.iter().fold(String::new(), |mut acc, byte| {
                write!(&mut acc, "{byte:02x}").unwrap();
                acc
            });
            Ok(())
            // print!("Result: 0x{}", hex_string);
        }
        Err(e) => {
            eprintln!("Error during emulation: {e:?}");
            Err(anyhow::anyhow!("Emulation failed"))
        }
    }
}
