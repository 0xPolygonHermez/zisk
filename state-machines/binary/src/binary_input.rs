use crate::{ChunkCollect, LTU_DIV_OP, XOR_KECCAK_OP};
use zisk_common::{ExtOperationData, OperationBusData};
use zisk_core::zisk_ops::ZiskOp;

pub struct BinaryInput {
    pub op: u8,
    pub a: u64,
    pub b: u64,
}

impl BinaryInput {
    #[allow(dead_code)]
    pub fn new(op: u8, a: u64, b: u64) -> Self {
        Self { op, a, b }
    }

    pub fn from(data: &ExtOperationData<u64>) -> Self {
        // `Arith` tags the comparison it injects on every division so that it carries no FROPS
        // (see `LTU_DIV_OP`). It is a plain `LTU` to the air, and this is where the tag is spent:
        // nothing downstream of the input, in Rust or in the PIL, knows it existed. The same goes
        // for the XORs `KeccakfProxy` assumes for a `keccak_xor` (see `XOR_KECCAK_OP`).
        let op = match OperationBusData::get_op(data) {
            LTU_DIV_OP => ZiskOp::LTU,
            XOR_KECCAK_OP => ZiskOp::XOR,
            op => op,
        };
        Self { op, a: OperationBusData::get_a(data), b: OperationBusData::get_b(data) }
    }

    /// Heap bytes the collectors of one instance hold once they have collected: each one takes the
    /// [`ChunkCollect::total_count`] inputs of its chunk.
    pub fn capacity_bytes<'a, const K: usize>(
        collects: impl IntoIterator<Item = &'a ChunkCollect<K>>,
    ) -> u64 {
        let inputs: u64 = collects.into_iter().map(ChunkCollect::total_count).sum();
        inputs * std::mem::size_of::<Self>() as u64
    }
}

/// Measures a witness computation from the point it is created to the end of its scope, so the
/// figure covers the whole fill however the function is shaped, and reports the share the input
/// flatten took of it.
///
/// The flatten is not accidental here, unlike the one `Mem` used to do: the binary fills are
/// parallel **over rows**, and each row picks up `flat_inputs[base + lane]` — random access into the
/// global operation sequence, which a lazily chained iterator cannot provide.
pub struct FlattenReport {
    pub name: &'static str,
    pub inputs: usize,
    pub flatten: std::time::Duration,
    pub started: std::time::Instant,
}

impl Drop for FlattenReport {
    fn drop(&mut self) {
        let total = self.started.elapsed().as_secs_f64() * 1e3;
        let flat = self.flatten.as_secs_f64() * 1e3;
        tracing::info!(
            "{} inputs: {} | flatten {:.1}ms of {:.0}ms total ({:.1}%)",
            self.name,
            self.inputs,
            flat,
            total,
            if total > 0.0 { 100.0 * flat / total } else { 0.0 }
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::KindCollect;
    use zisk_common::CollectSkipper;

    #[test]
    fn capacity_bytes_counts_every_kind_of_every_chunk() {
        let kind =
            |count| KindCollect { count, skipper: CollectSkipper::new(0), owns_frops: false };
        let first = ChunkCollect { kinds: [kind(3), kind(4)], force_execute_to_end: false };
        let second = ChunkCollect { kinds: [kind(0), kind(5)], force_execute_to_end: true };
        assert_eq!(
            BinaryInput::capacity_bytes([&first, &second]),
            12 * std::mem::size_of::<BinaryInput>() as u64
        );
    }
}
