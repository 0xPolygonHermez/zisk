use anyhow::{Context, Result};
use clap::Parser;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::PathBuf;

/// Utility to read, analyze, and filter Zisk hints files
#[derive(Parser, Debug)]
#[command(name = "hint_file")]
#[command(about = "Read and process Zisk hints files", long_about = None)]
struct Args {
    /// Input hints file
    input: PathBuf,

    /// Show detailed list of hints
    #[arg(short, long)]
    detail: bool,

    /// Show summary statistics (enabled by default if --detail is not used)
    #[arg(short, long)]
    summary: bool,

    /// Output file for filtered hints
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Filter out (exclude) hint types from output (comma-separated, decimal or hex with 0x prefix)
    /// Example: --filter 0xF0000,256,0x0100
    #[arg(short, long, value_delimiter = ',', conflicts_with = "extract")]
    filter: Vec<String>,

    /// Extract (include only) specific hint types to output (comma-separated, decimal or hex with 0x prefix)
    /// Example: --extract 0xF0000,256,0x0100
    #[arg(short, long, value_delimiter = ',', conflicts_with = "filter")]
    extract: Vec<String>,

    /// Input file to inject as hints (binary file without format)
    #[arg(long, requires = "output")]
    inject_input: Option<PathBuf>,

    /// Index position where to start injecting input chunks (0 = beginning, 1 = after first hint, etc.)
    #[arg(long, default_value = "0", requires = "inject_input")]
    inject_start: usize,

    /// Size of each input chunk in bytes (including 8-byte header)
    #[arg(long, default_value = "1024", requires = "inject_input")]
    inject_chunk_size: usize,

    /// Number of input chunks to inject together as a group
    #[arg(long, default_value = "1", requires = "inject_input")]
    inject_group_size: usize,

    /// Number of original hints to keep between injected groups
    #[arg(long, default_value = "1", requires = "inject_input")]
    inject_distance: usize,

    /// Hint type for injected input chunks (hex or decimal)
    #[arg(long, default_value = "0xF0000", requires = "inject_input")]
    inject_type: String,
}

/// Reads a u32 in little-endian format from the buffer
fn read_u32_le<R: Read>(reader: &mut R) -> std::io::Result<u32> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

/// Writes a u32 in little-endian format to the writer
fn write_u32_le<W: Write>(writer: &mut W, value: u32) -> std::io::Result<()> {
    writer.write_all(&value.to_le_bytes())
}

/// Structure representing a hint
#[derive(Debug, Clone)]
struct Hint {
    length_bytes: u32,     // Actual length in bytes (from header)
    length_aligned: usize, // Aligned length (multiple of 8 bytes)
    hint_type: u32,        // Hint type
    data: Vec<u8>,         // Hint data (aligned)
}

impl Hint {
    fn from_reader<R: Read>(reader: &mut R) -> std::io::Result<Option<Self>> {
        // Try to read the first 4 bytes (length in bytes)
        let length_bytes = match read_u32_le(reader) {
            Ok(len) => len,
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(e) => return Err(e),
        };

        // Read the next 4 bytes (hint type)
        let hint_type = read_u32_le(reader)?;

        // Calculate aligned length (rounded up to multiple of 8 bytes)
        let length_aligned = ((length_bytes as usize + 7) / 8) * 8;

        // Read the hint data (aligned to 8 bytes)
        let mut data = vec![0u8; length_aligned];
        reader.read_exact(&mut data)?;

        Ok(Some(Hint { length_bytes, length_aligned, hint_type, data }))
    }

    fn write_to<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        write_u32_le(writer, self.length_bytes)?;
        write_u32_le(writer, self.hint_type)?;
        writer.write_all(&self.data)?;
        Ok(())
    }
}

#[derive(Debug, Default)]
struct HintStats {
    count: usize,
    total_bytes: u64,
    min_bytes: u32,
    max_bytes: u32,
}

impl HintStats {
    fn update(&mut self, length: u32) {
        self.count += 1;
        self.total_bytes += length as u64;
        if self.count == 1 {
            self.min_bytes = length;
            self.max_bytes = length;
        } else {
            self.min_bytes = self.min_bytes.min(length);
            self.max_bytes = self.max_bytes.max(length);
        }
    }
}

fn parse_hint_types(filter_strings: &[String]) -> Result<Vec<u32>> {
    let mut types = Vec::new();
    for s in filter_strings {
        let s = s.trim();
        let value = if s.starts_with("0x") || s.starts_with("0X") {
            u32::from_str_radix(&s[2..], 16)
                .with_context(|| format!("Invalid hex hint type: {}", s))?
        } else {
            s.parse::<u32>().with_context(|| format!("Invalid decimal hint type: {}", s))?
        };
        types.push(value);
    }
    Ok(types)
}

fn parse_hint_type(type_string: &str) -> Result<u32> {
    let s = type_string.trim();
    let value = if s.starts_with("0x") || s.starts_with("0X") {
        u32::from_str_radix(&s[2..], 16).with_context(|| format!("Invalid hex hint type: {}", s))?
    } else {
        s.parse::<u32>().with_context(|| format!("Invalid decimal hint type: {}", s))?
    };
    Ok(value)
}

/// Read input file and split it into chunks as Hints
fn create_input_chunks(
    input_path: &PathBuf,
    chunk_size: usize,
    hint_type: u32,
) -> Result<Vec<Hint>> {
    if chunk_size <= 8 {
        anyhow::bail!("Chunk size must be greater than 8 bytes (to accommodate header)");
    }

    let mut file = File::open(input_path)
        .with_context(|| format!("Cannot open inject input file: {}", input_path.display()))?;

    let mut input_data = Vec::new();
    file.read_to_end(&mut input_data).context("Failed to read inject input file")?;

    if input_data.is_empty() {
        anyhow::bail!("Inject input file is empty");
    }

    let data_size_per_chunk = chunk_size - 8; // Subtract 8 bytes for header
    let mut chunks = Vec::new();

    for chunk_data in input_data.chunks(data_size_per_chunk) {
        let length_bytes = chunk_data.len() as u32;
        let length_aligned = ((length_bytes as usize + 7) / 8) * 8;

        // Pad data to 8-byte alignment
        let mut data = chunk_data.to_vec();
        data.resize(length_aligned, 0);

        chunks.push(Hint { length_bytes, length_aligned, hint_type, data });
    }

    Ok(chunks)
}

/// Interleave input chunks with original hints and write to output
fn write_interleaved_hints<W: Write>(
    writer: &mut W,
    original_hints: &[Hint],
    inject_chunks: &[Hint],
    start_index: usize,
    group_size: usize,
    distance: usize,
    stats: &mut HashMap<u32, HintStats>,
) -> Result<()> {
    let mut original_idx = 0;
    let mut inject_idx = 0;

    // Write initial hints before inject_start
    while original_idx < start_index && original_idx < original_hints.len() {
        original_hints[original_idx].write_to(writer)?;
        original_idx += 1;
    }

    // Interleave inject chunks with original hints
    while inject_idx < inject_chunks.len() && original_idx < original_hints.len() {
        // Write a group of inject chunks
        let mut group_count = 0;
        while group_count < group_size && inject_idx < inject_chunks.len() {
            let chunk = &inject_chunks[inject_idx];
            chunk.write_to(writer)?;

            // Update statistics for injected chunk
            stats
                .entry(chunk.hint_type)
                .or_insert_with(HintStats::default)
                .update(chunk.length_bytes);

            inject_idx += 1;
            group_count += 1;
        }

        // Write distance number of original hints
        let mut distance_count = 0;
        while distance_count < distance && original_idx < original_hints.len() {
            original_hints[original_idx].write_to(writer)?;
            original_idx += 1;
            distance_count += 1;
        }
    }

    // Write remaining original hints
    while original_idx < original_hints.len() {
        original_hints[original_idx].write_to(writer)?;
        original_idx += 1;
    }

    // Write remaining inject chunks (if any left)
    while inject_idx < inject_chunks.len() {
        let chunk = &inject_chunks[inject_idx];
        chunk.write_to(writer)?;

        // Update statistics for injected chunk
        stats.entry(chunk.hint_type).or_insert_with(HintStats::default).update(chunk.length_bytes);

        inject_idx += 1;
    }

    Ok(())
}

fn process_hints_file(args: &Args) -> Result<()> {
    let file = File::open(&args.input)
        .with_context(|| format!("Cannot open file: {}", args.input.display()))?;

    let mut reader = BufReader::new(file);

    // Read the 8-byte header at the beginning of the file
    let mut header = [0u8; 8];
    reader
        .read_exact(&mut header)
        .context("Failed to read 8-byte header at the beginning of the file")?;

    // Parse filter types (exclude) or extract types (include only)
    let filter_types =
        if !args.filter.is_empty() { Some(parse_hint_types(&args.filter)?) } else { None };

    let extract_types =
        if !args.extract.is_empty() { Some(parse_hint_types(&args.extract)?) } else { None };

    // Read input chunks if inject mode is enabled
    let inject_chunks = if let Some(ref inject_path) = args.inject_input {
        let inject_type = parse_hint_type(&args.inject_type)?;
        Some(create_input_chunks(inject_path, args.inject_chunk_size, inject_type)?)
    } else {
        None
    };

    let mut hints = Vec::new();
    let mut stats: HashMap<u32, HintStats> = HashMap::new();
    let mut hint_index = 0;

    // Show detail header if requested
    if args.detail {
        println!("Reading hints file: {}", args.input.display());
        println!("Header: {:02x?}", header);
        println!("{:-<80}", "");
        println!(
            "{:>6} | {:>12} | {:>12} | {:>14}",
            "Index", "Type (hex)", "Len (bytes)", "Aligned (bytes)"
        );
        println!("{:-<80}", "");
    }

    // Read all hints from the input file
    let mut final_tag: Option<Hint> = None;
    loop {
        match Hint::from_reader(&mut reader) {
            Ok(Some(hint)) => {
                // Check if this is the final tag (type=1)
                if hint.hint_type == 1 {
                    if args.detail {
                        println!("{:-<80}", "");
                        println!("Total hints processed: {}", hint_index);
                        println!(
                            "Final tag: length={}, type=1 (0x{:08X})",
                            hint.length_bytes, hint.hint_type
                        );
                    }

                    if hint.length_bytes != 0 {
                        eprintln!(
                            "Warning: Expected length=0 in final tag, got {}",
                            hint.length_bytes
                        );
                    }

                    // Check for garbage after the final tag
                    let mut garbage_buf = [0u8; 1];
                    match reader.read(&mut garbage_buf) {
                        Ok(0) => {
                            // No more data, good
                        }
                        Ok(n) => {
                            eprintln!(
                                "Warning: Found {} extra bytes after final tag (garbage data)",
                                n
                            );
                            // Try to read more to see how much garbage there is
                            let mut extra_buf = Vec::new();
                            if let Ok(extra) = reader.read_to_end(&mut extra_buf) {
                                eprintln!("Warning: Total garbage bytes: {}", n + extra);
                            }
                        }
                        Err(_) => {
                            // Error reading, probably end of file
                        }
                    }

                    final_tag = Some(hint);
                    break;
                }

                // Not the final tag, it's a normal hint
                if args.detail {
                    println!(
                        "{:>6} | {:>12} | {:>12} | {:>14}",
                        hint_index,
                        format!("0x{:08X}", hint.hint_type),
                        hint.length_bytes,
                        hint.length_aligned
                    );
                }

                // Update statistics
                stats
                    .entry(hint.hint_type)
                    .or_insert_with(HintStats::default)
                    .update(hint.length_bytes);

                hints.push(hint);
                hint_index += 1;
            }
            Ok(None) => {
                // End of file without final tag
                if args.detail {
                    println!("{:-<80}", "");
                    println!("Total hints processed: {}", hint_index);
                }
                eprintln!("Warning: Reached end of file without finding final tag (type=1)");
                break;
            }
            Err(e) => {
                // Read error
                if hint_index == 0 {
                    return Err(e).context("Error reading first hint");
                } else {
                    eprintln!("Error reading hint {}: {}", hint_index, e);
                    break;
                }
            }
        }
    }

    // Process output if needed (inject mode or filter/extract mode)
    if let Some(ref output_path) = args.output {
        let output_file = File::create(output_path)
            .with_context(|| format!("Cannot create output file: {}", output_path.display()))?;
        let mut writer = BufWriter::new(output_file);

        // Write the header to output file
        writer.write_all(&header)?;

        if let Some(ref inject_chunks) = inject_chunks {
            // Inject mode: interleave input chunks with original hints
            write_interleaved_hints(
                &mut writer,
                &hints,
                inject_chunks,
                args.inject_start,
                args.inject_group_size,
                args.inject_distance,
                &mut stats,
            )?;
        } else {
            // Filter/extract mode: write hints based on filter/extract criteria
            for hint in &hints {
                let should_write = if let Some(ref extract) = extract_types {
                    // Extract mode: write only if hint type is in the list
                    extract.contains(&hint.hint_type)
                } else if let Some(ref filter) = filter_types {
                    // Filter mode: write only if hint type is NOT in the list (exclude)
                    !filter.contains(&hint.hint_type)
                } else {
                    // No filter or extract, write all hints
                    true
                };

                if should_write {
                    hint.write_to(&mut writer)?;
                }
            }
        }

        // Write final tag if present
        if let Some(ref tag) = final_tag {
            tag.write_to(&mut writer)?;
        }

        writer.flush()?;
    }

    // Show summary if requested or if detail is not shown
    if args.summary || !args.detail {
        println!();
        println!("=== Summary ===");
        println!("File: {}", args.input.display());
        println!("Total hints: {}", hint_index);
        println!();
        println!(
            "{:>12} | {:>8} | {:>14} | {:>12} | {:>12}",
            "Type (hex)", "Count", "Total (bytes)", "Min (bytes)", "Max (bytes)"
        );
        println!("{:-<80}", "");

        // Sort by hint type for consistent output
        let mut sorted_stats: Vec<_> = stats.iter().collect();
        sorted_stats.sort_by_key(|(type_id, _)| *type_id);

        for (hint_type, stat) in sorted_stats {
            println!(
                "{:>12} | {:>8} | {:>14} | {:>12} | {:>12}",
                format!("0x{:08X}", hint_type),
                stat.count,
                stat.total_bytes,
                stat.min_bytes,
                stat.max_bytes
            );
        }
    }

    // Report output file if created
    if let Some(ref output_path) = args.output {
        println!();
        if let Some(ref inject_chunks) = inject_chunks {
            let inject_type = parse_hint_type(&args.inject_type)?;
            println!(
                "Hints with injected input written to: {} ({} chunks injected, type: 0x{:08X})",
                output_path.display(),
                inject_chunks.len(),
                inject_type
            );
            println!(
                "  Inject parameters: start={}, chunk_size={}, group_size={}, distance={}",
                args.inject_start,
                args.inject_chunk_size,
                args.inject_group_size,
                args.inject_distance
            );
        } else if let Some(ref extract) = extract_types {
            println!(
                "Extracted hints written to: {} (included types: {})",
                output_path.display(),
                extract.iter().map(|t| format!("0x{:08X}", t)).collect::<Vec<_>>().join(", ")
            );
        } else if let Some(ref filter) = filter_types {
            println!(
                "Filtered hints written to: {} (excluded types: {})",
                output_path.display(),
                filter.iter().map(|t| format!("0x{:08X}", t)).collect::<Vec<_>>().join(", ")
            );
        } else {
            println!("All hints written to: {}", output_path.display());
        }
    }

    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    process_hints_file(&args)
}
