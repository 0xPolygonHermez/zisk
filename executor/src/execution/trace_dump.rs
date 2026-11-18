//! A debugging aid: with `ZISK_DUMP_MIN_TRACES=<path>` set, every execution writes its minimal
//! traces to `<path>`, one line per chunk: the chunk's start state, its end and a hash of its
//! memory reads. Two executions of the same program and input must write the same file, whatever
//! emulator ran them (the assembly or the Rust one) and however it ran them, so a `diff` points
//! at the first chunk where they part.

use std::{ffi::OsStr, fs::File, io::BufWriter, io::Write, path::Path, sync::Arc};

use zisk_common::EmuTrace;

/// The variable that turns the dump on and names its file
pub const DUMP_MIN_TRACES_VAR: &str = "ZISK_DUMP_MIN_TRACES";

/// FNV-1a over 64-bit words: stable across builds, so dumps from different binaries compare
fn fnv1a(words: &[u64]) -> u64 {
    words.iter().fold(0xcbf2_9ce4_8422_2325u64, |hash, &word| {
        word.to_le_bytes()
            .iter()
            .fold(hash, |h, &byte| (h ^ byte as u64).wrapping_mul(0x0000_0100_0000_01b3))
    })
}

/// One line of the dump: the chunk's index, start state, end, and memory reads
fn chunk_line(index: usize, trace: &EmuTrace) -> String {
    let start = &trace.start_state;
    format!(
        "{index} step={} pc={:#x} sp={:#x} c={:#x} regs={:016x} steps={} last_c={:#x} end={} \
         reads={} reads_hash={:016x}",
        start.step,
        start.pc,
        start.sp,
        start.c,
        fnv1a(&start.regs),
        trace.steps,
        trace.last_c,
        trace.end,
        trace.mem_reads.len(),
        fnv1a(&trace.mem_reads),
    )
}

/// Writes `traces` to `path` (see the module doc)
pub fn dump_min_traces(path: &OsStr, traces: &[Arc<EmuTrace>]) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(Path::new(path))?);
    for (index, trace) in traces.iter().enumerate() {
        writeln!(file, "{}", chunk_line(index, trace))?;
    }
    file.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_changes_with_any_read() {
        let mut trace = EmuTrace { mem_reads: vec![1, 2, 3].into(), ..Default::default() };
        let line = chunk_line(0, &trace);
        trace.mem_reads = vec![1, 2, 4].into();
        assert_ne!(line, chunk_line(0, &trace));
        assert_eq!(fnv1a(&[]), 0xcbf2_9ce4_8422_2325, "FNV-1a's offset basis");
    }
}
