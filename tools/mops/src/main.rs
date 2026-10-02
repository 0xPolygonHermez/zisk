use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
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
/// Stream layout: state-machines/mem-cpp/cpp/mops_format.hpp.
const MOPS_BLOCK_VALUES: u32 = 0x07;
/// Light form: a single write with this bit carries no value; a block record with the payload bit
/// carries no step word.
const MOPS_NO_VALUE_BIT: u64 = 58;
const MOPS_NO_PAYLOAD_BIT: u64 = 62;

/// Words of the record whose header word is `hdr` (bit 63 is the header tag).
fn mops_record_len(hdr: u64) -> usize {
    let mode = ((hdr >> 32) & 0x3F) as u32;
    let low = mode & 0x0F;
    let single = matches!(low, 1 | 2 | 4 | 8);
    if low == MOPS_ALIGNED_READ || (single && (mode & MOPS_WRITE_FLAG) == 0) {
        1
    } else if single || low == MOPS_ALIGNED_WRITE {
        if (hdr >> MOPS_NO_VALUE_BIT) & 1 == 1 { 1 } else { 2 }
    } else if low == MOPS_BLOCK_VALUES {
        2 + ((hdr >> 36) & 63) as usize
    } else if (hdr >> MOPS_NO_PAYLOAD_BIT) & 1 == 1 {
        1
    } else {
        2
    }
}

/// Decodes the record at `w`; a value block becomes an aligned block write of its words.
fn mops_decode_record(w: &[u64]) -> MemCountersBusData {
    let hdr = w[0];
    let mut flags = ((hdr >> 32) & 0x3FFF_FFFF) as u32;
    if flags & 0x0F == MOPS_BLOCK_VALUES {
        flags = MOPS_ALIGNED_BLOCK_WRITE | ((((hdr >> 36) & 63) as u32) << MOPS_BLOCK_COUNT_SBITS);
    }
    MemCountersBusData { addr: hdr as u32, flags }
}

/// The records of a chunk's word stream.
fn decode_stream(words: &[u64]) -> Vec<MemCountersBusData> {
    let mut out = Vec::with_capacity(words.len() / 2);
    let mut w = 0;
    while w < words.len() {
        out.push(mops_decode_record(&words[w..]));
        w += mops_record_len(words[w]);
    }
    out
}

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
    dual_count: usize,
    ram_read_count: usize,
    ram_write_count: usize,
}

impl MopsExpander {
    fn new() -> Self {
        Self {
            free_read_available: HashMap::new(),
            dual_count: 0,
            ram_read_count: 0,
            ram_write_count: 0,
        }
    }

    /// Mirrors MemCounterSingle::add_aligned_read.
    /// For RAM: if free_read_available is true, consumes it (no push).
    ///          If false, sets true and pushes addr once.
    /// For non-RAM: always pushes addr.
    fn add_aligned_read(&mut self, addr: u32, output: &mut Vec<u32>) {
        let is_ram = addr >= RAM_ADDR;
        if is_ram {
            self.ram_read_count += 1;
            if self.free_read_available.get(&addr).copied().unwrap_or(false) {
                self.dual_count += 1;
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
            self.ram_write_count += 1;
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
            self.ram_read_count += 1;
            self.ram_write_count += 1;
            if !self.free_read_available.get(&addr).copied().unwrap_or(false) {
                output.push(addr); // read
            } else {
                self.dual_count += 1;
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
        self.free_read_available.clear();
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

/// Read a chunk binary file (the chunk's stream words) into decoded records
fn read_chunk_file(path: &Path) -> Result<Vec<MemCountersBusData>> {
    let data = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    if data.len() % 8 != 0 {
        bail!("File {} size {} is not a multiple of 8", path.display(), data.len());
    }
    let words: Vec<u64> =
        data.chunks_exact(8).map(|b| u64::from_le_bytes(b.try_into().unwrap())).collect();
    Ok(decode_stream(&words))
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
    /// Count expanded aligned addresses per chunk without writing output files
    Count(CountArgs),
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
struct CountArgs {
    /// Input directory containing mem_count_data_*.bin files
    #[arg(short, long)]
    input: PathBuf,

    /// Input filename prefix (default: mem_count_data_)
    #[arg(long, default_value = "mem_count_data_")]
    prefix: String,
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

fn cmd_count(args: &CountArgs) -> Result<()> {
    if !args.input.is_dir() {
        bail!("Input path {} is not a directory", args.input.display());
    }

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

        println!(
            "chunk {:>6}: {:>8} mops -> {:>8} aligned addresses",
            chunk_id,
            entries.len(),
            expanded.len(),
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
    println!(
        "[RAM] Total:{} Read:{} Write:{} Dual:{} Duals {:.2}% reads, {:.2}% total",
        expander.ram_read_count + expander.ram_write_count,
        expander.ram_read_count,
        expander.ram_write_count,
        expander.dual_count,
        (expander.dual_count as f64 / expander.ram_read_count as f64) * 100.0,
        (expander.dual_count as f64 / (expander.ram_read_count + expander.ram_write_count) as f64)
            * 100.0
    );

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
    println!(
        "[RAM] Total:{} Read:{} Write:{} Dual:{} Duals {:.2}% reads, {:.2}% total",
        expander.ram_read_count + expander.ram_write_count,
        expander.ram_read_count,
        expander.ram_write_count,
        expander.dual_count,
        (expander.dual_count as f64 / expander.ram_read_count as f64) * 100.0,
        (expander.dual_count as f64 / (expander.ram_read_count + expander.ram_write_count) as f64)
            * 100.0
    );

    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    match &args.command {
        Command::Expand(expand_args) => cmd_expand(expand_args),
        Command::Count(count_args) => cmd_count(count_args),
        Command::MemAlignCount(mac_args) => cmd_mem_align_count(mac_args),
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    /// One record of every kind of both forms, heavy and light. The same words and expectations
    /// are in state-machines/mem-cpp/cpp/test/test_mops_format.cpp.
    const WORDS: &[u64] = &[
        0x8048d141a0000003, 0xc048d158a0000010, 0x0000000000000001, 0x84000014a0000006,
        0x8048d171a0000001, 0x000000000000005a, 0x8048d14c80001000, 0x8048d14da0002000,
        0x0000000000000007, 0x8400000da0002008, 0x8000005a40000008, 0x0048d14000000000,
        0x8000064e40001000, 0x0048d14000000000, 0xc000007a40002000, 0xc000003ba0003000,
        0xc00000cfa0004000, 0x8000009fa0005000, 0x0048d14000000000, 0xaaf37bf7a0100000,
        0x0000000000000020, 0x0000000000000000, 0x0101010101010101, 0x0202020202020202,
        0x0303030303030303, 0x0404040404040404, 0x0505050505050505, 0x0606060606060606,
        0x0707070707070707, 0x0808080808080808, 0x0909090909090909, 0x0a0a0a0a0a0a0a0a,
        0x0b0b0b0b0b0b0b0b, 0x0c0c0c0c0c0c0c0c, 0x0d0d0d0d0d0d0d0d, 0x0e0e0e0e0e0e0e0e,
        0x0f0f0f0f0f0f0f0f, 0x1010101010101010, 0x1111111111111111, 0x1212121212121212,
        0x1313131313131313, 0x1414141414141414, 0x1515151515151515, 0x1616161616161616,
        0x1717171717171717, 0x1818181818181818, 0x1919191919191919, 0x1a1a1a1a1a1a1a1a,
        0x1b1b1b1b1b1b1b1b, 0x1c1c1c1c1c1c1c1c, 0x1d1d1d1d1d1d1d1d, 0x1e1e1e1e1e1e1e1e,
        0x1f1f1f1f1f1f1f1f, 0x2020202020202020, 0x2121212121212121, 0x2222222222222222,
        0x2323232323232323, 0x2424242424242424, 0x2525252525252525, 0x2626262626262626,
        0x2727272727272727, 0x2828282828282828, 0x2929292929292929, 0x2a2a2a2a2a2a2a2a,
        0x2b2b2b2b2b2b2b2b, 0x2c2c2c2c2c2c2c2c, 0x2d2d2d2d2d2d2d2d, 0x2e2e2e2e2e2e2e2e,
        0x2f2f2f2f2f2f2f2f, 0x3030303030303030, 0x3131313131313131, 0x3232323232323232,
        0x3333333333333333, 0x3434343434343434, 0x3535353535353535, 0x3636363636363636,
        0x3737373737373737, 0x3838383838383838, 0x3939393939393939, 0x3a3a3a3a3a3a3a3a,
        0x3b3b3b3b3b3b3b3b, 0x3c3c3c3c3c3c3c3c, 0x3d3d3d3d3d3d3d3d, 0x3e3e3e3e3e3e3e3e,
        0xaaf37817a0200000, 0x0000000000000000, 0x0000000000000042
    ];
    const EXPECT: &[(usize, u32, u32, i64, &str)] = &[
        (1, 0xa0000003, 0x01, -1, "read_1 unaligned"),
        (2, 0xa0000010, 0x18, -1, "write_8 aligned, bit 63 value"),
        (1, 0xa0000006, 0x14, -1, "write_4 no value"),
        (2, 0xa0000001, 0x31, -1, "cwrite_1 with value"),
        (1, 0x80001000, 0x0c, -1, "aligned read"),
        (2, 0xa0002000, 0x0d, -1, "aligned write with value"),
        (1, 0xa0002008, 0x0d, -1, "aligned write no value"),
        (2, 0x40000008, 0x0a, 5, "block read 5"),
        (2, 0x40001000, 0x0e, 100, "aligned block read 100"),
        (1, 0x40002000, 0x0a, 7, "block read 7, no payload"),
        (1, 0xa0003000, 0x0b, 3, "block write 3, no payload"),
        (1, 0xa0004000, 0x0f, 12, "aligned block write 12, no payload"),
        (2, 0xa0005000, 0x0f, 9, "aligned block write 9 with step payload"),
        (65, 0xa0100000, 0x0f, 63, "value block 63"),
        (3, 0xa0200000, 0x0f, 1, "value block 1"),
    ];

    #[test]
    fn decoders_agree_with_the_table() {
        let mut k = 0;
        for &(len, addr, mode, count, name) in EXPECT {
            assert!(k < WORDS.len(), "{name}: stream ended");
            let got_len = mops_record_len(WORDS[k]);
            let d = mops_decode_record(&WORDS[k..]);
            let low = d.flags & 0x0F;
            let block = low >= 0x0A && low != 0x0C && low != 0x0D;
            let got_count = if block { (d.flags >> MOPS_BLOCK_COUNT_SBITS) as i64 } else { -1 };
            let got_mode = if block { low } else { d.flags & 0x3F };
            assert_eq!((got_len, d.addr, got_mode, got_count), (len, addr, mode, count), "{name}");
            k += got_len;
        }
        assert_eq!(k, WORDS.len(), "words consumed");
    }
}
