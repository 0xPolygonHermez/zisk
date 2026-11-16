use std::collections::HashMap;

use proofman_fields::Goldilocks;
use zisk_common::{
    BusDeviceMetrics, BusDeviceMode, ChunkId, CollectSkipper, ComponentPlanBuilder, Metrics, Plan,
};
use zisk_core::{zisk_ops::ZiskOp, ZiskOperationType};
use zisk_precomp_floatdot::{FloatDotCounterInputGen, FloatDotManager};

fn payload(n: usize) -> Vec<u64> {
    let mut data = vec![
        ZiskOp::FLOATDOT.into(),
        ZiskOperationType::FloatDot as u64,
        0,
        0xa0001000,
        17,
        0xa0002000,
        0xa0012000,
        n as u64,
        0,
        0,
        (n as f32).to_bits().into(),
    ];
    data.extend(std::iter::repeat(0x3f800000_3f800000).take(2 * n.div_ceil(2)));
    data
}

fn plans_with_ids(chunks: &[(usize, Vec<usize>)]) -> Vec<Plan> {
    let counters = chunks
        .iter()
        .map(|(chunk, lengths)| {
            let mut counter = FloatDotCounterInputGen::<Goldilocks>::new(BusDeviceMode::Counter);
            for &n in lengths {
                counter.measure(&payload(n));
            }
            (ChunkId(*chunk), Box::new(counter) as Box<dyn BusDeviceMetrics>)
        })
        .collect();
    FloatDotManager::<Goldilocks>::planner(false).plan(counters)
}

fn plans(chunks: &[Vec<usize>]) -> Vec<Plan> {
    plans_with_ids(&chunks.iter().cloned().enumerate().collect::<Vec<_>>())
}

#[test]
fn shuffled_sparse_chunks_keep_their_real_ids() {
    let result = plans_with_ids(&[(99, vec![1]), (7, vec![4096, 4096]), (33, vec![])]);
    assert_eq!(result.len(), 2);
    for (plan, expected) in result.into_iter().zip([7, 99]) {
        let windows =
            plan.meta.unwrap().downcast::<HashMap<ChunkId, (u64, CollectSkipper)>>().unwrap();
        assert_eq!(windows.len(), 1);
        assert!(windows.contains_key(&ChunkId(expected)));
    }
}

#[test]
#[should_panic(expected = "duplicate FloatDot chunk")]
fn duplicate_chunks_are_rejected() {
    plans_with_ids(&[(7, vec![1]), (7, vec![1])]);
}

#[test]
fn malformed_lengths_and_payloads_are_rejected_before_planning() {
    for data in [payload(4097), payload(3)[..11].to_vec(), vec![0; 10]] {
        assert!(std::panic::catch_unwind(|| {
            FloatDotCounterInputGen::<Goldilocks>::new(BusDeviceMode::Counter).measure(&data);
        })
        .is_err());
    }
}

#[test]
fn sixty_four_short_calls_fit_one_instance() {
    assert_eq!(plans(&[vec![128; 64]]).len(), 1);
}

#[test]
fn whole_calls_spill_without_crossing_an_instance() {
    assert_eq!(plans(&[vec![4096, 4096]]).len(), 1);
    assert_eq!(plans(&[vec![4096, 4096, 1]]).len(), 2);
    assert_eq!(plans(&[vec![0; 8192]]).len(), 1);
    assert!(plans(&[vec![]]).is_empty());
}

#[test]
fn mixed_chunk_windows_cover_every_call_once_in_order() {
    let chunks = vec![vec![0, 1, 3, 128, 749], vec![], vec![4096, 4096, 1, 0, 128]];
    let result = plans(&chunks);
    assert_eq!(result.len(), 2);
    let mut observed = vec![];
    for plan in result {
        let windows =
            plan.meta.unwrap().downcast::<HashMap<ChunkId, (u64, CollectSkipper)>>().unwrap();
        let mut windows: Vec<_> = windows.into_iter().collect();
        windows.sort_by_key(|(id, _)| id.0);
        let mut rows = 0;
        for (chunk, (count, mut skip)) in windows {
            let mut accepted = 0;
            for (index, &length) in chunks[chunk.0].iter().enumerate() {
                if accepted == count {
                    break;
                }
                if !skip.should_skip() {
                    observed.push((chunk.0, index));
                    rows += 2 * length.max(1);
                    accepted += 1;
                }
            }
            assert_eq!(accepted, count);
        }
        assert!(rows <= 16384);
    }
    let expected: Vec<_> =
        chunks.iter().enumerate().flat_map(|(c, v)| (0..v.len()).map(move |i| (c, i))).collect();
    assert_eq!(observed, expected);
}
