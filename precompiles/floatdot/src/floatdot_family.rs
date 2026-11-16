use std::{any::Any, collections::HashMap, marker::PhantomData, ops::Add, sync::Arc};

use proofman_common::{ProofCtx, SetupCtx};
use proofman_fields::PrimeField64;
use zisk_common::{
    plan_ladder, BusDevice, BusDeviceMetrics, BusDeviceMode, BusId, CheckPoint, ChunkId,
    CollectSkipper, ComponentBuilder, ComponentPlanBuilder, InstCount, Instance, InstanceCtx,
    InstanceType, Metrics, OperationBusData, PayloadType, Plan, Planner, StatsType, WitnessResult,
    B, OPERATION_BUS_FLOATDOT_DATA_SIZE, OPERATION_BUS_ID, OP_TYPE, STEP,
};
use zisk_core::ZiskOperationType;
use zisk_pil::{FloatDotTrace, FloatDotTraceRow, FloatDotTraceRowPacked, ZISK_AIRGROUP_ID};
use zisk_precomp_common::{MemProcessor, PrecompileMemInputs};

use crate::{FloatDotInput, FloatDotSM};

pub fn floatdot_rows(n: usize) -> usize {
    assert!(n <= 4096, "FloatDot length exceeds its ABI limit");
    2 * n.max(1)
}

pub struct FloatDotManager<F: PrimeField64> {
    sm: Arc<FloatDotSM<F>>,
}

impl<F: PrimeField64> FloatDotManager<F> {
    pub fn new() -> Arc<Self> {
        Arc::new(Self { sm: FloatDotSM::new() })
    }
}

impl<F: PrimeField64> ComponentPlanBuilder<F> for FloatDotManager<F> {
    type Counter = FloatDotCounterInputGen<F>;
    fn counter(is_asm: bool) -> Self::Counter {
        Self::Counter::new(if is_asm { BusDeviceMode::CounterAsm } else { BusDeviceMode::Counter })
    }
    fn planner(_: bool) -> Box<dyn Planner> {
        Box::new(FloatDotPlanner::<F>(PhantomData))
    }
}

impl<F: PrimeField64> ComponentBuilder<F> for FloatDotManager<F> {
    fn build_instance(&self, ictx: InstanceCtx) -> Box<dyn Instance<F>> {
        assert_eq!(ictx.plan.air_id, FloatDotTrace::<()>::AIR_ID);
        Box::new(FloatDotInstance { sm: self.sm.clone(), ictx })
    }
}

pub struct FloatDotPlanner<F>(PhantomData<F>);

impl<F: PrimeField64> Planner for FloatDotPlanner<F> {
    fn plan(&self, mut counters: Vec<(ChunkId, Box<dyn BusDeviceMetrics>)>) -> Vec<Plan> {
        counters.sort_by_key(|(id, _)| id.0);
        assert!(counters.windows(2).all(|pair| pair[0].0 != pair[1].0), "duplicate FloatDot chunk");
        let mut counts = vec![];
        let mut capacities = vec![];
        let (mut used, mut calls) = (0, 0);
        for (id, counter) in &counters {
            let counter =
                Metrics::as_any(&**counter).downcast_ref::<FloatDotCounterInputGen<F>>().unwrap();
            counts.push(InstCount::new(*id, counter.lengths.len() as u64));
            for &n in &counter.lengths {
                let rows = floatdot_rows(n.into());
                if used + rows > FloatDotTrace::<()>::NUM_ROWS {
                    capacities.push(calls);
                    used = 0;
                    calls = 0;
                }
                used += rows;
                calls += 1;
            }
        }
        if calls != 0 {
            capacities.push(calls);
        }
        // The shared helper labels windows by vector position, so restore actual chunk IDs.
        plan_ladder(&counts, &capacities)
            .into_iter()
            .map(|(_, _, windows)| {
                let windows: HashMap<_, _> = windows
                    .into_iter()
                    .map(|(ordinal, value)| (counters[ordinal.0].0, value))
                    .collect();
                let mut ids: Vec<_> = windows.keys().copied().collect();
                ids.sort_by_key(|id| id.0);
                Plan::new(
                    ZISK_AIRGROUP_ID,
                    FloatDotTrace::<()>::AIR_ID,
                    None,
                    InstanceType::Instance,
                    CheckPoint::Multiple(ids),
                    Some(Box::new(windows)),
                )
            })
            .collect()
    }
}

pub struct FloatDotInstance<F: PrimeField64> {
    sm: Arc<FloatDotSM<F>>,
    ictx: InstanceCtx,
}

impl<F: PrimeField64> FloatDotInstance<F> {
    pub fn build_float_dot_collector(&self, chunk_id: ChunkId) -> FloatDotCollector {
        assert_eq!(self.ictx.plan.air_id, FloatDotTrace::<()>::AIR_ID);
        let windows = self
            .ictx
            .plan
            .meta
            .as_ref()
            .unwrap()
            .downcast_ref::<HashMap<ChunkId, (u64, CollectSkipper)>>()
            .unwrap();
        let (count, skip) = windows[&chunk_id];
        FloatDotCollector::new(count, skip)
    }
}

impl<F: PrimeField64> Instance<F> for FloatDotInstance<F> {
    fn compute_witness(
        &self,
        _: &ProofCtx<F>,
        setup: &SetupCtx<F>,
        mut collectors: Vec<(usize, Box<dyn BusDevice<PayloadType>>)>,
        buffer: Vec<F>,
        packed: bool,
    ) -> WitnessResult<F> {
        collectors.sort_by_key(|(chunk, _)| *chunk);
        let inputs: Vec<_> = collectors
            .into_iter()
            .map(|(_, c)| {
                let c = c.as_any().downcast::<FloatDotCollector>().unwrap();
                c.into_inputs()
            })
            .collect();
        if packed {
            Ok(Some(self.sm.compute_witness::<FloatDotTraceRowPacked<F>,
                { FloatDotTrace::<()>::NUM_ROWS }, { FloatDotTrace::<()>::AIR_ID }>(setup, &inputs, buffer)?))
        } else {
            Ok(Some(self.sm.compute_witness::<FloatDotTraceRow<F>,
                { FloatDotTrace::<()>::NUM_ROWS }, { FloatDotTrace::<()>::AIR_ID }>(setup, &inputs, buffer)?))
        }
    }
    fn check_point(&self) -> &CheckPoint {
        &self.ictx.plan.check_point
    }
    fn instance_type(&self) -> InstanceType {
        InstanceType::Instance
    }
    fn stats_type(&self) -> StatsType {
        StatsType::Precompiled
    }
    fn build_inputs_collector(&self, chunk: ChunkId) -> Option<Box<dyn BusDevice<PayloadType>>> {
        Some(Box::new(self.build_float_dot_collector(chunk)))
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct FloatDotCollector {
    inputs: Vec<FloatDotInput>,
    count: u64,
    skip: CollectSkipper,
}

impl FloatDotCollector {
    pub fn new(count: u64, skip: CollectSkipper) -> Self {
        Self { inputs: Vec::with_capacity(count as usize), count, skip }
    }
    fn into_inputs(self) -> Vec<FloatDotInput> {
        assert_eq!(self.inputs.len() as u64, self.count, "missing FloatDot calls");
        self.inputs
    }

    pub fn process_data(&mut self, bus: &BusId, data: &[u64]) -> bool {
        self.process_data_ext(bus, data, &[])
    }
    pub fn process_data_ext(&mut self, bus: &BusId, data: &[u64], extended: &[u64]) -> bool {
        debug_assert_eq!(*bus, OPERATION_BUS_ID);
        if self.inputs.len() as u64 == self.count {
            return false;
        }
        if data[OP_TYPE] != ZiskOperationType::FloatDot as u64 {
            return true;
        }
        OperationBusData::validate_floatdot_payload(data, extended)
            .expect("invalid FloatDot payload");
        if self.skip.should_skip() {
            return true;
        }
        self.inputs.push(FloatDotInput::from_extended(data.try_into().unwrap(), extended));
        (self.inputs.len() as u64) < self.count
    }
}

impl BusDevice<PayloadType> for FloatDotCollector {
    fn as_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

pub struct FloatDotCounterInputGen<F> {
    lengths: Vec<u16>,
    mode: BusDeviceMode,
    marker: PhantomData<F>,
}

impl<F: PrimeField64> FloatDotCounterInputGen<F> {
    pub fn new(mode: BusDeviceMode) -> Self {
        Self { lengths: vec![], mode, marker: PhantomData }
    }
    pub fn inst_count(&self, op: ZiskOperationType) -> Option<u64> {
        (op == ZiskOperationType::FloatDot).then_some(self.lengths.len() as u64)
    }
    pub fn process_data<P: MemProcessor>(
        &mut self,
        bus: &BusId,
        data: &[u64],
        memory: &mut P,
    ) -> bool {
        self.process_data_ext(bus, data, &[], memory)
    }
    pub fn process_data_ext<P: MemProcessor>(
        &mut self,
        bus: &BusId,
        data: &[u64],
        extended: &[u64],
        memory: &mut P,
    ) -> bool {
        debug_assert_eq!(*bus, OPERATION_BUS_ID);
        if data[OP_TYPE] != ZiskOperationType::FloatDot as u64 {
            return true;
        }
        OperationBusData::validate_floatdot_payload(data, extended)
            .expect("invalid FloatDot payload");
        let full = [data, extended].concat();
        match self.mode {
            BusDeviceMode::Counter => {
                self.measure(&full);
                FloatDotSM::<F>::generate(data[B] as u32, data[STEP], &full, true, memory);
            }
            BusDeviceMode::CounterAsm => self.measure(&full),
            BusDeviceMode::InputGenerator => {
                if !FloatDotSM::<F>::should_skip(data[B] as u32, &full, memory) {
                    FloatDotSM::<F>::generate(data[B] as u32, data[STEP], &full, false, memory);
                }
            }
        }
        true
    }
}

impl<F: PrimeField64> Metrics for FloatDotCounterInputGen<F> {
    fn measure(&mut self, data: &[u64]) {
        assert!(data.len() >= OPERATION_BUS_FLOATDOT_DATA_SIZE, "truncated FloatDot counter input");
        let (header, words) = data.split_at(OPERATION_BUS_FLOATDOT_DATA_SIZE);
        OperationBusData::validate_floatdot_payload(header, words)
            .expect("invalid FloatDot counter input");
        self.lengths.push(header[7] as u16);
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<F> Add for FloatDotCounterInputGen<F> {
    type Output = Self;
    fn add(mut self, other: Self) -> Self {
        self.lengths.extend(other.lengths);
        self
    }
}

impl<F: PrimeField64> BusDevice<u64> for FloatDotCounterInputGen<F> {
    fn as_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proofman_fields::Goldilocks;
    use zisk_core::zisk_ops::ZiskOp;

    #[derive(Default)]
    struct Memory(Vec<[u64; 7]>);
    impl MemProcessor for Memory {
        fn process_mem_data(&mut self, data: &[u64; 7]) {
            self.0.push(*data);
        }
        fn skip_addr(&mut self, _: u32) -> bool {
            false
        }
        fn skip_addr_range(&mut self, _: u32, _: u32) -> bool {
            panic!("individual reads required")
        }
    }

    /// Odd IDs use BF16 operands: unaligned contiguous x and stride-1024 y.
    fn payload(id: u64, n: usize) -> Vec<u64> {
        if id % 2 == 1 {
            let mut data = payload(id - 1, n);
            data[3] += 40;
            data[4] += 1;
            data[5] = 0xa0002002 | 1 << 32;
            data[6] = 0xa0012006 | 1024 << 32;
            data[9] = 1 << 8;
            data.truncate(11);
            data.extend(std::iter::repeat(0x3f80_3f80_3f80_3f80).take(2 * n));
            return data;
        }
        let mut data = vec![
            ZiskOp::FLOATDOT.into(),
            ZiskOperationType::FloatDot as u64,
            0,
            0xa0001000 + 40 * id,
            17 + id,
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

    #[test]
    #[should_panic(expected = "missing FloatDot calls")]
    fn incomplete_collector_cannot_supply_witness_inputs() {
        let data = payload(0, 1);
        let mut collector = FloatDotCollector::new(2, CollectSkipper::new(0));
        assert!(collector.process_data_ext(&OPERATION_BUS_ID, &data[..11], &data[11..]));
        collector.into_inputs();
    }

    #[test]
    fn real_counter_planner_collectors_preserve_calls_and_memory_reads() {
        let chunks =
            vec![(99, vec![3, 0, 1]), (7, vec![4096, 3]), (33, vec![]), (55, vec![4096, 128])];
        let mut streams = HashMap::new();
        let mut counters: Vec<(ChunkId, Box<dyn BusDeviceMetrics>)> = vec![];
        let mut id = 0;
        for (chunk, lengths) in chunks {
            let mut stream = vec![];
            let mut counter = FloatDotCounterInputGen::<Goldilocks>::new(BusDeviceMode::Counter);
            let mut generator =
                FloatDotCounterInputGen::<Goldilocks>::new(BusDeviceMode::InputGenerator);
            for n in lengths {
                let data = payload(id, n);
                id += 1;
                let (header, words) = data.split_at(OPERATION_BUS_FLOATDOT_DATA_SIZE);
                let mut counted = Memory::default();
                let mut generated = Memory::default();
                assert!(counter.process_data_ext(&OPERATION_BUS_ID, header, words, &mut counted));
                assert!(generator.process_data_ext(
                    &OPERATION_BUS_ID,
                    header,
                    words,
                    &mut generated
                ));
                let mut expected = vec![];
                let mut read = |address, value| {
                    expected.push([1, address, 4 * header[STEP] + 3, 8, value, 0, 0])
                };
                for i in 0..5 {
                    read(header[B] + 8 * i as u64, header[5 + i]);
                }
                let half = words.len() / 2;
                for vector in 0..2 {
                    let (base, stride) =
                        (header[5 + vector] & 0xffff_ffff, header[5 + vector] >> 32);
                    for word in 0..half {
                        let address = if header[9] >> 8 == 0 {
                            base + 8 * word as u64
                        } else {
                            (base + 2 * stride * word as u64) & !7
                        };
                        read(address, words[vector * half + word]);
                    }
                }
                assert_eq!(counted.0, expected);
                assert_eq!(generated.0, expected);
                stream.push(data);
            }
            assert_eq!(counter.inst_count(ZiskOperationType::FloatDot), Some(stream.len() as u64));
            assert_eq!(generator.inst_count(ZiskOperationType::FloatDot), Some(0));
            counters.push((ChunkId(chunk), Box::new(counter)));
            streams.insert(chunk, stream);
        }
        let plans = FloatDotManager::<Goldilocks>::planner(false).plan(counters);
        assert_eq!(plans.len(), 2);
        let manager = FloatDotManager::<Goldilocks>::new();
        let mut observed = vec![];
        for (index, plan) in plans.into_iter().enumerate() {
            let instance = manager.build_instance(InstanceCtx::new(index, plan));
            let instance =
                instance.as_any().downcast_ref::<FloatDotInstance<Goldilocks>>().unwrap();
            let CheckPoint::Multiple(chunks) = instance.check_point() else {
                panic!("chunk windows required")
            };
            for chunk in chunks {
                let mut collector = instance.build_float_dot_collector(*chunk);
                for data in &streams[&chunk.0] {
                    if !collector.process_data_ext(&OPERATION_BUS_ID, &data[..11], &data[11..]) {
                        break;
                    }
                }
                assert_eq!(collector.inputs.len() as u64, collector.count);
                observed
                    .extend(collector.into_inputs().into_iter().map(|input| format!("{input:?}")));
            }
        }
        let mut chunks: Vec<_> = streams.into_iter().collect();
        chunks.sort_by_key(|(id, _)| *id);
        let expected: Vec<_> = chunks
            .into_iter()
            .flat_map(|(_, calls)| calls)
            .map(|data| {
                format!(
                    "{:?}",
                    FloatDotInput::from_extended(data[..11].try_into().unwrap(), &data[11..])
                )
            })
            .collect();
        assert_eq!(observed, expected);
    }
}
