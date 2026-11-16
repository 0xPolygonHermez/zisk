use proofman_fields::Goldilocks;
use std::panic::{catch_unwind, AssertUnwindSafe};
use zisk_common::{BusDeviceMode, CollectSkipper, OperationFloatDotData, OPERATION_BUS_ID};
use zisk_core::{zisk_ops::ZiskOp, ZiskOperationType};
use zisk_precomp_common::MemProcessor;
use zisk_precomp_floatdot::{FloatDotCollector, FloatDotCounterInputGen};
use zisk_precomp_sha256f::{Sha256fCollector, Sha256fCounterInputGen};

#[derive(Default)]
struct Recorder(Vec<[u64; 7]>);
impl MemProcessor for Recorder {
    fn process_mem_data(&mut self, data: &[u64; 7]) {
        self.0.push(*data);
    }
    fn skip_addr(&mut self, _: u32) -> bool {
        false
    }
    fn skip_addr_range(&mut self, _: u32, _: u32) -> bool {
        false
    }
}

fn header() -> OperationFloatDotData<u64> {
    [
        u64::from(ZiskOp::FLOATDOT),
        ZiskOperationType::FloatDot as u64,
        0,
        0x3000,
        77,
        0x1000,
        0x1000,
        3,
        0,
        0,
        3.0f32.to_bits().into(),
    ]
}

#[test]
fn registry_forwards_extended_words_to_the_real_floatdot_family() {
    let header = header();
    let words = [0x3f800000_3f800000, 0x3f800000, 0x3f800000_3f800000, 0x3f800000];
    let mut collector = FloatDotCollector::new(1, CollectSkipper::new(0));
    assert!(catch_unwind(AssertUnwindSafe(|| collector.process_data(&OPERATION_BUS_ID, &header)))
        .is_err());
    assert!(!zisk_executor::register_precompiles!(
        @collector [true], collector, &OPERATION_BUS_ID, &header, &words
    ));

    for mode in [BusDeviceMode::Counter, BusDeviceMode::CounterAsm, BusDeviceMode::InputGenerator] {
        let asm = matches!(mode, BusDeviceMode::CounterAsm);
        let counts = !matches!(mode, BusDeviceMode::InputGenerator);
        let mut counter = FloatDotCounterInputGen::<Goldilocks>::new(mode);
        let mut mem = Recorder::default();
        assert!(catch_unwind(AssertUnwindSafe(|| counter.process_data(
            &OPERATION_BUS_ID,
            &header,
            &mut mem
        )))
        .is_err());
        assert_eq!(counter.inst_count(ZiskOperationType::FloatDot), Some(0));
        assert!(mem.0.is_empty());
        assert!(zisk_executor::register_precompiles!(
            @counter [true], counter, &OPERATION_BUS_ID, &header, &words, &mut mem
        ));
        assert_eq!(counter.inst_count(ZiskOperationType::FloatDot), Some(u64::from(counts)));
        let expected = if asm { vec![] } else { [&header[5..10], &words].concat() };
        assert_eq!(mem.0.iter().map(|row| row[4]).collect::<Vec<_>>(), expected);
    }
}

#[test]
fn fixed_registry_clients_keep_legacy_payload_behavior() {
    let mut data = [0; 17];
    data[..5].copy_from_slice(&[
        u64::from(ZiskOp::SHA256),
        ZiskOperationType::Sha256 as u64,
        0x1000,
        0x2000,
        77,
    ]);
    let mut collector = Sha256fCollector::new(1, CollectSkipper::new(0));
    assert!(!zisk_executor::register_precompiles!(
        @collector [], collector, &OPERATION_BUS_ID, &data, &[u64::MAX]
    ));
    let mut counter = Sha256fCounterInputGen::<Goldilocks>::new(BusDeviceMode::CounterAsm);
    let mut mem = Recorder::default();
    assert!(zisk_executor::register_precompiles!(
        @counter [], counter, &OPERATION_BUS_ID, &data, &[u64::MAX], &mut mem
    ));
    assert_eq!(counter.inst_count(ZiskOperationType::Sha256), Some(1));
}
