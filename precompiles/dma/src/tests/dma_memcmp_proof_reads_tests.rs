//! Parity between the memory reads the proof issues for a `memcmp`, as generated here, and the ones
//! the emulator reports to its occupancy analyses (`zisk_core::report_memcmp_proof_reads`). The
//! dual-row analysis is only right if both see every word the same number of times.

use super::generate_dma_memcmp_mem_inputs;
use zisk_common::{A, B, DMA_ENCODED, DMA_MEMCMP_COUNT_BUS, OP, STEP};
use zisk_core::{report_memcmp_proof_reads, zisk_ops::OpStats, RAM_ADDR};
use zisk_precomp_common::MemProcessor;
use zisk_precomp_helpers::DmaInfo;

/// Memory operation code of a store, as `MemBusHelpers` emits it.
const MEMORY_STORE_OP: u64 = 2;

/// Collects the addresses of the memory operations the generator sends.
#[derive(Default)]
struct Recorder {
    reads: Vec<u64>,
    writes: usize,
}

impl MemProcessor for Recorder {
    fn process_mem_data(&mut self, data: &[u64; 7]) {
        // `data[0]` is the memory operation: 1 for a load, 2 for a store.
        if data[0] == MEMORY_STORE_OP {
            self.writes += 1;
        } else {
            self.reads.push(data[1]);
        }
    }
    fn skip_addr(&mut self, _addr: u32) -> bool {
        false
    }
    fn skip_addr_range(&mut self, _addr_from: u32, _addr_to: u32) -> bool {
        false
    }
}

/// Collects the addresses the emulator reports as read by the proof.
#[derive(Default)]
struct StatsRecorder {
    reads: Vec<u64>,
}

impl OpStats for StatsRecorder {
    fn mem_align_read(&mut self, _addr: u64, _count: usize) {
        unreachable!("the proof reads go through mem_proof_read only");
    }
    fn mem_align_write(&mut self, _addr: u64, _count: usize) {
        unreachable!("a compare does not write");
    }
    fn mem_cost_read(&mut self, _addr: u64, _count: usize) {
        unreachable!("the proof reads go through mem_proof_read only");
    }
    fn mem_proof_read(&mut self, addr: u64, count: usize) {
        self.reads.extend((0..count as u64).map(|i| addr + 8 * i));
    }
    fn set_variable_cost(&mut self, _cost: u64) {}
    fn dma_loop(&mut self, _kind: u8, _loop_count: usize, _aligned: bool) {}
}

#[test]
fn the_emulator_reports_exactly_the_reads_the_proof_issues() {
    for neq in [false, true] {
        for dst_offset in 0..8u64 {
            for src_offset in 0..8u64 {
                for count in 1..48usize {
                    let dst = RAM_ADDR + 0x100 + dst_offset;
                    let src = RAM_ADDR + 0x1000 + src_offset;
                    let encoded = DmaInfo::encode_memcmp(dst, src, count, neq as u64);

                    let mut data = vec![0u64; DMA_MEMCMP_COUNT_BUS + 1];
                    // The extended opcode: the plain one also reads the count from memory, which the
                    // emulator reports apart from the compare itself.
                    data[OP] = zisk_core::zisk_ops::ZiskOp::DmaXMemCmp.code() as u64;
                    data[A] = dst;
                    data[B] = src;
                    data[DMA_ENCODED] = encoded;
                    data[STEP] = 1;
                    let data_ext = vec![0u64; 256];
                    let mut proof = Recorder::default();
                    generate_dma_memcmp_mem_inputs(&data, &data_ext, &mut proof);

                    let mut emulator = StatsRecorder::default();
                    report_memcmp_proof_reads(dst, src, encoded, &mut emulator);

                    proof.reads.sort_unstable();
                    emulator.reads.sort_unstable();
                    assert_eq!(proof.writes, 0);
                    assert_eq!(
                        emulator.reads, proof.reads,
                        "dst offset {dst_offset}, src offset {src_offset}, count {count}, neq {neq}"
                    );
                }
            }
        }
    }
}
