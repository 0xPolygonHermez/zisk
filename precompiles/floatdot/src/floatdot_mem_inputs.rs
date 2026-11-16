use proofman_fields::PrimeField64;
use zisk_common::{
    OperationBusData, B, OPERATION_BUS_FLOATDOT_DATA_SIZE, OPERATION_PRECOMPILED_BUS_DATA_SIZE,
    STEP,
};
use zisk_precomp_common::{MemBusHelpers, MemProcessor, PrecompileMemInputs};
use zisk_precomp_helpers::FloatDotCall;

use crate::FloatDotSM;

impl<F: PrimeField64> PrecompileMemInputs for FloatDotSM<F> {
    fn generate<P: MemProcessor>(
        addr_main: u32,
        step_main: u64,
        data: &[u64],
        _only_counters: bool,
        mem_processor: &mut P,
    ) {
        let call = decode(addr_main, data);
        assert_eq!(call.step(), step_main, "FloatDot memory step differs from operation header");
        for read in call.memory_reads() {
            MemBusHelpers::mem_aligned_read(read.address, read.step, read.value, mem_processor);
        }
    }

    fn should_skip<P: MemProcessor>(addr_main: u32, data: &[u64], mem_processor: &mut P) -> bool {
        let call = decode(addr_main, data);
        let mut skip = true;
        for read in call.memory_reads() {
            skip &= mem_processor.skip_addr(read.address);
        }
        skip
    }
}

fn decode(addr_main: u32, data: &[u64]) -> FloatDotCall<'_> {
    assert!(data.len() >= OPERATION_BUS_FLOATDOT_DATA_SIZE, "truncated FloatDot operation payload");
    let (fixed, extended) = data.split_at(OPERATION_BUS_FLOATDOT_DATA_SIZE);
    OperationBusData::validate_floatdot_payload(fixed, extended)
        .expect("invalid FloatDot memory payload");
    assert_eq!(fixed[B], u64::from(addr_main), "FloatDot descriptor differs from operation header");
    FloatDotCall::decode_parts(
        fixed[B],
        fixed[STEP],
        &fixed[OPERATION_PRECOMPILED_BUS_DATA_SIZE..],
        extended,
    )
    .expect("validated FloatDot memory input")
}

#[cfg(test)]
mod tests {
    use super::*;
    use proofman_fields::Goldilocks;
    use zisk_core::{zisk_ops::ZiskOp, ZiskOperationType};

    #[derive(Default)]
    struct Recorder {
        reads: Vec<[u64; 7]>,
        checked: Vec<u32>,
        wanted: Option<u32>,
    }

    impl MemProcessor for Recorder {
        fn process_mem_data(&mut self, data: &[u64; 7]) {
            self.reads.push(*data);
        }
        fn skip_addr(&mut self, addr: u32) -> bool {
            self.checked.push(addr);
            self.wanted != Some(addr)
        }
        fn skip_addr_range(&mut self, _from: u32, _to: u32) -> bool {
            panic!("individual reads required")
        }
    }

    fn payload(n: u64) -> Vec<u64> {
        let mut data = vec![
            u64::from(ZiskOp::FLOATDOT),
            ZiskOperationType::FloatDot as u64,
            0,
            0xa0001000,
            17,
            0xa0002000,
            0xa0002000,
            n,
            0xff800001,
            8,
            (8u64 << 32) | 0xff800001,
        ];
        if n != 0 {
            data.extend([11, 12, 21, 22]);
        }
        data
    }

    #[test]
    fn read_only_memory_adapter_keeps_padding_overlap_and_timestamp() {
        let data = payload(3);
        for only_counters in [false, true] {
            let mut recorder = Recorder::default();
            FloatDotSM::<Goldilocks>::generate(
                data[B] as u32,
                data[STEP],
                &data,
                only_counters,
                &mut recorder,
            );
            let expected = [
                (0xa0001000, 0xa0002000),
                (0xa0001008, 0xa0002000),
                (0xa0001010, 3),
                (0xa0001018, 0xff800001),
                (0xa0001020, 8),
                (0xa0002000, 11),
                (0xa0002008, 12),
                (0xa0002000, 21),
                (0xa0002008, 22),
            ]
            .map(|(addr, value)| [1, addr, 71, 8, value, 0, 0]);
            assert_eq!(recorder.reads, expected);
        }
        let mut recorder = Recorder { wanted: Some(0xa0002008), ..Default::default() };
        assert!(!FloatDotSM::<Goldilocks>::should_skip(data[B] as u32, &data, &mut recorder));
        assert_eq!(
            recorder.checked,
            [
                0xa0001000, 0xa0001008, 0xa0001010, 0xa0001018, 0xa0001020, 0xa0002000, 0xa0002008,
                0xa0002000, 0xa0002008
            ]
        );
        let mut recorder = Recorder::default();
        assert!(FloatDotSM::<Goldilocks>::should_skip(data[B] as u32, &data, &mut recorder));
    }

    #[test]
    fn empty_call_reads_only_descriptor_and_rejects_transport_mismatches() {
        let data = payload(0);
        let mut recorder = Recorder::default();
        FloatDotSM::<Goldilocks>::generate(data[B] as u32, data[STEP], &data, false, &mut recorder);
        assert_eq!(recorder.reads.len(), 5);
        for (address, step, len) in
            [(data[B] + 8, 17, data.len()), (data[B], 18, data.len()), (data[B], 17, 10)]
        {
            let mut recorder = Recorder::default();
            assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                FloatDotSM::<Goldilocks>::generate(
                    address as u32,
                    step,
                    &data[..len],
                    false,
                    &mut recorder,
                );
            }))
            .is_err());
            assert!(recorder.reads.is_empty());
        }
    }
}
