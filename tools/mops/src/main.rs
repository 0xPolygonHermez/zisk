use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const ALIGN_MASK: u32 = 0xFFFF_FFF8;
const RAM_ADDR: u32 = 0xA000_0000;

// MOPS flags (from mem_config.hpp)
#[allow(dead_code)]
const MOPS_WRITE_FLAG: u32 = 0x10;

const MOPS_READ_8: u32 = 0x08;
const MOPS_READ_4: u32 = 0x04;
const MOPS_READ_2: u32 = 0x02;
const MOPS_READ_1: u32 = 0x01;

const MOPS_WRITE_8: u32 = 0x18;
const MOPS_WRITE_4: u32 = 0x14;
const MOPS_WRITE_2: u32 = 0x12;
const MOPS_WRITE_1: u32 = 0x11;

const MOPS_CWRITE_1: u32 = 0x31;

const MOPS_BLOCK_READ: u32 = 0x0A;
const MOPS_BLOCK_WRITE: u32 = 0x0B;
const MOPS_ALIGNED_READ: u32 = 0x0C;
const MOPS_ALIGNED_WRITE: u32 = 0x0D;
const MOPS_ALIGNED_BLOCK_READ: u32 = 0x0E;
const MOPS_ALIGNED_BLOCK_WRITE: u32 = 0x0F;

const MOPS_BLOCK_COUNT_SBITS: u32 = 4;

/// MemCountersBusData: 8 bytes packed (addr: u32, flags: u32)
#[repr(C, packed)]
#[derive(Clone, Copy)]
struct MemCountersBusData {
    addr: u32,
    flags: u32,
}

/// Tracks free_read_available state across chunks, matching MemCounterSingle logic.
struct MopsExpander {
    free_read_available: HashMap<u32, bool>,
}

impl MopsExpander {
    fn new() -> Self {
        Self { free_read_available: HashMap::new() }
    }

    /// Mirrors MemCounterSingle::add_aligned_read.
    /// For RAM: if free_read_available is true, consumes it (no push).
    ///          If false, sets true and pushes addr once.
    /// For non-RAM: always pushes addr.
    fn add_aligned_read(&mut self, addr: u32, output: &mut Vec<u32>) {
        let is_ram = addr >= RAM_ADDR;
        if is_ram {
            if self.free_read_available.get(&addr).copied().unwrap_or(false) {
                self.free_read_available.insert(addr, false);
            } else {
                self.free_read_available.insert(addr, true);
                output.push(addr);
            }
        } else {
            output.push(addr);
        }
    }

    /// Mirrors MemCounterSingle::add_aligned_write.
    /// For RAM: sets free_read_available to true.
    /// Always pushes addr.
    fn add_aligned_write(&mut self, addr: u32, output: &mut Vec<u32>) {
        let is_ram = addr >= RAM_ADDR;
        if is_ram {
            self.free_read_available.insert(addr, true);
        }
        output.push(addr);
    }

    /// Mirrors MemCounterSingle::add_aligned_read_write.
    /// For RAM: if free_read_available is false, pushes addr (read), then pushes addr (write),
    ///          sets free_read_available to true.
    ///          If free_read_available is true, pushes addr (write only),
    ///          free_read_available stays true.
    /// For non-RAM: pushes addr twice (read + write).
    fn add_aligned_read_write(&mut self, addr: u32, output: &mut Vec<u32>) {
        let is_ram = addr >= RAM_ADDR;
        if is_ram {
            if !self.free_read_available.get(&addr).copied().unwrap_or(false) {
                output.push(addr); // read
            }
            output.push(addr); // write
            self.free_read_available.insert(addr, true);
        } else {
            output.push(addr); // read
            output.push(addr); // write
        }
    }

    /// Expand one chunk of mops trace entries, maintaining free_read_available state.
    fn expand_chunk(&mut self, data: &[MemCountersBusData]) -> Vec<u32> {
        let mut output: Vec<u32> = Vec::with_capacity(data.len() * 2);

        for entry in data {
            let addr = entry.addr;
            let flags = entry.flags;
            let mode = flags & 0x3F;
            let aligned_addr = addr & ALIGN_MASK;

            match mode {
                // 1 byte read
                MOPS_READ_1 => {
                    self.add_aligned_read(aligned_addr, &mut output);
                }
                // 1 byte conditional write
                MOPS_CWRITE_1 => {
                    self.add_aligned_read_write(aligned_addr, &mut output);
                }
                // 1 byte write
                MOPS_WRITE_1 => {
                    self.add_aligned_read_write(aligned_addr, &mut output);
                }

                // 2 byte read
                MOPS_READ_2 => {
                    self.add_aligned_read(aligned_addr, &mut output);
                    if (addr & 0x07) > 6 {
                        self.add_aligned_read(aligned_addr + 8, &mut output);
                    }
                }
                // 2 byte write
                MOPS_WRITE_2 => {
                    self.add_aligned_read_write(aligned_addr, &mut output);
                    if (addr & 0x07) > 6 {
                        self.add_aligned_read_write(aligned_addr + 8, &mut output);
                    }
                }

                // 4 byte read
                MOPS_READ_4 => {
                    self.add_aligned_read(aligned_addr, &mut output);
                    if (addr & 0x07) > 4 {
                        self.add_aligned_read(aligned_addr + 8, &mut output);
                    }
                }
                // 4 byte write
                MOPS_WRITE_4 => {
                    self.add_aligned_read_write(aligned_addr, &mut output);
                    if (addr & 0x07) > 4 {
                        self.add_aligned_read_write(aligned_addr + 8, &mut output);
                    }
                }

                // 8 byte read
                MOPS_READ_8 => {
                    self.add_aligned_read(aligned_addr, &mut output);
                    if (addr & 0x07) > 0 {
                        self.add_aligned_read(aligned_addr + 8, &mut output);
                    }
                }
                // 8 byte write
                MOPS_WRITE_8 => {
                    if addr == aligned_addr {
                        self.add_aligned_write(aligned_addr, &mut output);
                    } else {
                        self.add_aligned_read_write(aligned_addr, &mut output);
                        self.add_aligned_read_write(aligned_addr + 8, &mut output);
                    }
                }

                // Aligned read
                MOPS_ALIGNED_READ => {
                    self.add_aligned_read(addr, &mut output);
                }
                // Aligned write
                MOPS_ALIGNED_WRITE => {
                    self.add_aligned_write(addr, &mut output);
                }

                // Block read / Aligned block read
                m if (m & 0x0F) == MOPS_BLOCK_READ || (m & 0x0F) == MOPS_ALIGNED_BLOCK_READ => {
                    let count = flags >> MOPS_BLOCK_COUNT_SBITS;
                    for i in 0..count {
                        self.add_aligned_read(addr + i * 8, &mut output);
                    }
                }
                // Block write / Aligned block write
                m if (m & 0x0F) == MOPS_BLOCK_WRITE || (m & 0x0F) == MOPS_ALIGNED_BLOCK_WRITE => {
                    let count = flags >> MOPS_BLOCK_COUNT_SBITS;
                    for i in 0..count {
                        self.add_aligned_write(addr + i * 8, &mut output);
                    }
                }

                _ => {
                    let bytes = flags & 0x0F;
                    eprintln!(
                        "WARNING: invalid mode 0x{:02x} (bytes={}, addr=0x{:08x}), skipping",
                        mode, bytes, addr
                    );
                }
            }
        }
        output
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct ChunkMemAlignCounters {
    chunk_id: u32,
    full_5: u32,
    full_3: u32,
    full_2: u32,
    read_byte: u32,
    write_byte: u32,
}

/// Compute per-chunk stats (full_5, full_3, full_2, read_byte, write_byte) matching
/// the MemCounterSingle logic.
fn stats_chunk(chunk_id: u32, data: &[MemCountersBusData]) -> ChunkMemAlignCounters {
    let mut s = ChunkMemAlignCounters { chunk_id, ..Default::default() };

    for entry in data {
        let addr = entry.addr;
        let flags = entry.flags;
        let mode = flags & 0x3F;

        match mode {
            MOPS_READ_1 => {
                s.read_byte += 1;
            }
            MOPS_CWRITE_1 => {
                s.write_byte += 1;
            }
            MOPS_WRITE_1 => {
                s.full_3 += 1;
            }

            MOPS_READ_2 => {
                if (addr & 0x07) > 6 {
                    s.full_3 += 1;
                } else {
                    s.full_2 += 1;
                }
            }
            MOPS_WRITE_2 => {
                if (addr & 0x07) > 6 {
                    s.full_5 += 1;
                } else {
                    s.full_3 += 1;
                }
            }

            MOPS_READ_4 => {
                if (addr & 0x07) > 4 {
                    s.full_3 += 1;
                } else {
                    s.full_2 += 1;
                }
            }
            MOPS_WRITE_4 => {
                if (addr & 0x07) > 4 {
                    s.full_5 += 1;
                } else {
                    s.full_3 += 1;
                }
            }

            MOPS_READ_8 => {
                if (addr & 0x07) > 0 {
                    s.full_3 += 1;
                }
            }
            MOPS_WRITE_8 => {
                if (addr & 0x07) > 0 {
                    s.full_5 += 1;
                }
            }

            // Aligned read/write, block read/write: no full_* / byte counters
            _ => {}
        }
    }
    s
}

/// Read a chunk binary file into a Vec<MemCountersBusData>
fn read_chunk_file(path: &Path) -> Result<Vec<MemCountersBusData>> {
    let data = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let entry_size = std::mem::size_of::<MemCountersBusData>();
    if data.len() % entry_size != 0 {
        bail!(
            "File {} size {} is not a multiple of entry size {}",
            path.display(),
            data.len(),
            entry_size
        );
    }
    let count = data.len() / entry_size;
    let mut entries = Vec::with_capacity(count);
    let mut cursor = &data[..];
    for _ in 0..count {
        let mut addr_bytes = [0u8; 4];
        let mut flags_bytes = [0u8; 4];
        cursor.read_exact(&mut addr_bytes)?;
        cursor.read_exact(&mut flags_bytes)?;
        entries.push(MemCountersBusData {
            addr: u32::from_le_bytes(addr_bytes),
            flags: u32::from_le_bytes(flags_bytes),
        });
    }
    Ok(entries)
}

/// Write expanded addresses as a binary file of little-endian u32 values
fn write_output_file(path: &Path, addresses: &[u32]) -> Result<()> {
    let mut file =
        fs::File::create(path).with_context(|| format!("creating {}", path.display()))?;
    for &addr in addresses {
        file.write_all(&addr.to_le_bytes())?;
    }
    Ok(())
}

#[derive(Parser, Debug)]
#[command(name = "mops", about = "Mops trace utilities")]
#[command(subcommand_required = true, arg_required_else_help = true)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Expand mops trace files into aligned addresses
    Expand(ExpandArgs),
    /// Extract per-chunk mem align counts (full_5, full_3, full_2, read_byte, write_byte) as CSV
    #[command(name = "mem_align_count")]
    MemAlignCount(MemAlignCountArgs),
}

#[derive(Parser, Debug)]
struct ExpandArgs {
    /// Input directory containing mem_count_data_*.bin files
    #[arg(short, long)]
    input: PathBuf,

    /// Output directory for expanded address files
    #[arg(short, long)]
    output: PathBuf,

    /// Input filename prefix (default: mem_count_data_)
    #[arg(long, default_value = "mem_count_data_")]
    prefix: String,

    /// Output filename prefix (default: mem_aligned_)
    #[arg(long, default_value = "mem_aligned_")]
    out_prefix: String,
}

#[derive(Parser, Debug)]
struct MemAlignCountArgs {
    /// Input directory containing mem_count_data_*.bin files
    #[arg(short, long)]
    input: PathBuf,

    /// Output CSV file for mem align counts
    #[arg(short, long)]
    output: PathBuf,

    /// Input filename prefix
    #[arg(long, default_value = "mem_count_data_")]
    prefix: String,
}

fn cmd_mem_align_count(args: &MemAlignCountArgs) -> Result<()> {
    if !args.input.is_dir() {
        bail!("Input path {} is not a directory", args.input.display());
    }

    if let Some(parent) = args.output.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating output directory {}", parent.display()))?;
    }

    let mut chunk_id: u32 = 0;
    let mut counters: Vec<ChunkMemAlignCounters> = Vec::new();

    loop {
        let input_file = args.input.join(format!("{}{}.bin", args.prefix, chunk_id));
        if !input_file.exists() {
            break;
        }

        let entries = read_chunk_file(&input_file)?;
        let s = stats_chunk(chunk_id, &entries);
        counters.push(s);
        chunk_id += 1;
    }

    if chunk_id == 0 {
        bail!("No chunk files found in {} with prefix '{}'", args.input.display(), args.prefix);
    }

    // CSV header
    let header = "chunk_id,full_5,full_3,full_2,read_byte,write_byte";
    println!("{}", header);

    let mut csv_content = String::new();
    csv_content.push_str(header);
    csv_content.push('\n');

    for s in &counters {
        let line = format!(
            "{},{},{},{},{},{}",
            s.chunk_id, s.full_5, s.full_3, s.full_2, s.read_byte, s.write_byte
        );
        println!("{}", line);
        csv_content.push_str(&line);
        csv_content.push('\n');
    }

    // Totals row
    let tot_5: u32 = counters.iter().map(|s| s.full_5).sum();
    let tot_3: u32 = counters.iter().map(|s| s.full_3).sum();
    let tot_2: u32 = counters.iter().map(|s| s.full_2).sum();
    let tot_rb: u32 = counters.iter().map(|s| s.read_byte).sum();
    let tot_wb: u32 = counters.iter().map(|s| s.write_byte).sum();
    let totals_line = format!("total,{},{},{},{},{}", tot_5, tot_3, tot_2, tot_rb, tot_wb);
    println!("{}", totals_line);
    csv_content.push_str(&totals_line);
    csv_content.push('\n');

    // Write CSV file
    fs::write(&args.output, &csv_content)
        .with_context(|| format!("writing {}", args.output.display()))?;
    println!("\nWritten to {}", args.output.display());

    Ok(())
}

fn cmd_expand(args: &ExpandArgs) -> Result<()> {
    if !args.input.is_dir() {
        bail!("Input path {} is not a directory", args.input.display());
    }

    fs::create_dir_all(&args.output)
        .with_context(|| format!("creating output directory {}", args.output.display()))?;

    let mut expander = MopsExpander::new();
    let mut chunk_id: u32 = 0;
    let mut total_input_entries: usize = 0;
    let mut total_output_entries: usize = 0;

    loop {
        let input_file = args.input.join(format!("{}{}.bin", args.prefix, chunk_id));
        if !input_file.exists() {
            break;
        }

        let entries = read_chunk_file(&input_file)?;
        let expanded = expander.expand_chunk(&entries);

        let output_file = args.output.join(format!("{}{}.bin", args.out_prefix, chunk_id));
        write_output_file(&output_file, &expanded)?;

        println!(
            "chunk {:>6}: {:>8} mops -> {:>8} aligned addresses  ({})",
            chunk_id,
            entries.len(),
            expanded.len(),
            output_file.display()
        );

        total_input_entries += entries.len();
        total_output_entries += expanded.len();
        chunk_id += 1;
    }

    if chunk_id == 0 {
        bail!("No chunk files found in {} with prefix '{}'", args.input.display(), args.prefix);
    }

    println!(
        "\nTotal: {} chunks, {} mops -> {} aligned addresses",
        chunk_id, total_input_entries, total_output_entries
    );

    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    match &args.command {
        Command::Expand(expand_args) => cmd_expand(expand_args),
        Command::MemAlignCount(mac_args) => cmd_mem_align_count(mac_args),
    }
}
