use std::{any::Any, collections::HashMap, marker::PhantomData, ops::Add, sync::Arc};

use proofman_common::{ProofCtx, SetupCtx};
use proofman_fields::PrimeField64;
use zisk_common::{
    plan_ladder, BusDevice, BusDeviceMetrics, BusDeviceMode, BusId, CheckPoint, ChunkId,
    CollectSkipper, ComponentBuilder, ComponentPlanBuilder, InstCount, Instance, InstanceCtx,
    InstanceType, Metrics, PayloadType, Plan, Planner, StatsType, WitnessResult, B,
    OPERATION_BUS_ID, OPERATION_PRECOMPILED_BUS_DATA_SIZE, OP_TYPE, STEP,
};
use zisk_core::ZiskOperationType;
use zisk_pil::{DotF8Trace, DotF8TraceRow, DotF8TraceRowPacked, ZISK_AIRGROUP_ID};
use zisk_precomp_common::{MemProcessor, PrecompileMemInputs};

use crate::{dot_f8_rows, DotF8Input, DotF8SM};

pub struct DotF8Manager<F: PrimeField64> {
    sm: Arc<DotF8SM<F>>,
}

impl<F: PrimeField64> DotF8Manager<F> {
    pub fn new() -> Arc<Self> {
        Arc::new(Self { sm: DotF8SM::new() })
    }
}

impl<F: PrimeField64> ComponentPlanBuilder<F> for DotF8Manager<F> {
    type Counter = DotF8CounterInputGen<F>;
    fn counter(is_asm: bool) -> Self::Counter {
        Self::Counter::new(if is_asm { BusDeviceMode::CounterAsm } else { BusDeviceMode::Counter })
    }
    fn planner(_: bool) -> Box<dyn Planner> {
        Box::new(DotF8Planner::<F>(PhantomData))
    }
}

impl<F: PrimeField64> ComponentBuilder<F> for DotF8Manager<F> {
    fn build_instance(&self, ictx: InstanceCtx) -> Box<dyn Instance<F>> {
        assert_eq!(ictx.plan.air_id, DotF8Trace::<()>::AIR_ID);
        Box::new(DotF8Instance { sm: self.sm.clone(), ictx })
    }
}

pub struct DotF8Planner<F>(PhantomData<F>);

impl<F: PrimeField64> Planner for DotF8Planner<F> {
    fn plan(&self, mut counters: Vec<(ChunkId, Box<dyn BusDeviceMetrics>)>) -> Vec<Plan> {
        counters.sort_by_key(|(id, _)| id.0);
        assert!(counters.windows(2).all(|pair| pair[0].0 != pair[1].0), "duplicate DotF8 chunk");
        let mut counts = vec![];
        let mut capacities = vec![];
        let (mut used, mut calls) = (0, 0);
        for (id, counter) in &counters {
            let counter =
                Metrics::as_any(&**counter).downcast_ref::<DotF8CounterInputGen<F>>().unwrap();
            counts.push(InstCount::new(*id, counter.lengths.len() as u64));
            for &n in &counter.lengths {
                let rows = dot_f8_rows(n);
                assert!(
                    rows <= DotF8Trace::<()>::NUM_ROWS,
                    "dot_f8: a call of {n} values exceeds an instance"
                );
                if used + rows > DotF8Trace::<()>::NUM_ROWS {
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
                    DotF8Trace::<()>::AIR_ID,
                    None,
                    InstanceType::Instance,
                    CheckPoint::Multiple(ids),
                    Some(Box::new(windows)),
                )
            })
            .collect()
    }
}

pub struct DotF8Instance<F: PrimeField64> {
    sm: Arc<DotF8SM<F>>,
    ictx: InstanceCtx,
}

impl<F: PrimeField64> DotF8Instance<F> {
    pub fn build_dot_f8_collector(&self, chunk_id: ChunkId) -> DotF8Collector {
        assert_eq!(self.ictx.plan.air_id, DotF8Trace::<()>::AIR_ID);
        let windows = self
            .ictx
            .plan
            .meta
            .as_ref()
            .unwrap()
            .downcast_ref::<HashMap<ChunkId, (u64, CollectSkipper)>>()
            .unwrap();
        let (count, skip) = windows[&chunk_id];
        DotF8Collector::new(count, skip)
    }
}

impl<F: PrimeField64> Instance<F> for DotF8Instance<F> {
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
                let c = c.as_any().downcast::<DotF8Collector>().unwrap();
                c.into_inputs()
            })
            .collect();
        const ROWS: usize = DotF8Trace::<()>::NUM_ROWS;
        const AIR: usize = DotF8Trace::<()>::AIR_ID;
        Ok(Some(if packed {
            self.sm.compute_witness::<DotF8TraceRowPacked<F>, ROWS, AIR>(setup, &inputs, buffer)?
        } else {
            self.sm.compute_witness::<DotF8TraceRow<F>, ROWS, AIR>(setup, &inputs, buffer)?
        }))
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
        Some(Box::new(self.build_dot_f8_collector(chunk)))
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct DotF8Collector {
    inputs: Vec<DotF8Input>,
    count: u64,
    skip: CollectSkipper,
}

impl DotF8Collector {
    pub fn new(count: u64, skip: CollectSkipper) -> Self {
        Self { inputs: Vec::with_capacity(count as usize), count, skip }
    }
    fn into_inputs(self) -> Vec<DotF8Input> {
        assert_eq!(self.inputs.len() as u64, self.count, "missing DotF8 calls");
        self.inputs
    }

    pub fn process_data_ext(&mut self, bus: &BusId, data: &[u64], extended: &[u64]) -> bool {
        debug_assert_eq!(*bus, OPERATION_BUS_ID);
        if self.inputs.len() as u64 == self.count {
            return false;
        }
        if data[OP_TYPE] != ZiskOperationType::DotF8 as u64 {
            return true;
        }
        if self.skip.should_skip() {
            return true;
        }
        self.inputs.push(DotF8Input::new(data, extended));
        (self.inputs.len() as u64) < self.count
    }
}

impl BusDevice<PayloadType> for DotF8Collector {
    fn as_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

pub struct DotF8CounterInputGen<F> {
    lengths: Vec<u64>,
    mode: BusDeviceMode,
    marker: PhantomData<F>,
}

impl<F: PrimeField64> DotF8CounterInputGen<F> {
    pub fn new(mode: BusDeviceMode) -> Self {
        Self { lengths: vec![], mode, marker: PhantomData }
    }
    pub fn process_data_ext<P: MemProcessor>(
        &mut self,
        bus: &BusId,
        data: &[u64],
        extended: &[u64],
        memory: &mut P,
    ) -> bool {
        debug_assert_eq!(*bus, OPERATION_BUS_ID);
        if data[OP_TYPE] != ZiskOperationType::DotF8 as u64 {
            return true;
        }
        match self.mode {
            BusDeviceMode::Counter => {
                self.measure(data);
                let full = [data, extended].concat();
                DotF8SM::<F>::generate(data[B] as u32, data[STEP], &full, true, memory);
            }
            BusDeviceMode::CounterAsm => self.measure(data),
            BusDeviceMode::InputGenerator => {
                let full = [data, extended].concat();
                if !DotF8SM::<F>::should_skip(data[B] as u32, &full, memory) {
                    DotF8SM::<F>::generate(data[B] as u32, data[STEP], &full, false, memory);
                }
            }
        }
        true
    }
}

impl<F: PrimeField64> Metrics for DotF8CounterInputGen<F> {
    fn measure(&mut self, data: &[u64]) {
        self.lengths.push(data[OPERATION_PRECOMPILED_BUS_DATA_SIZE + 2]);
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<F> Add for DotF8CounterInputGen<F> {
    type Output = Self;
    fn add(mut self, other: Self) -> Self {
        self.lengths.extend(other.lengths);
        self
    }
}

impl<F: PrimeField64> BusDevice<u64> for DotF8CounterInputGen<F> {
    fn as_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proofman_fields::Goldilocks;
    use zisk_definitions::DOT_F8_MAX_N;

    #[test]
    fn planner_keeps_calls_whole_across_sparse_chunks() {
        fn chunk(id: usize, lengths: &[u64]) -> (ChunkId, Box<dyn BusDeviceMetrics>) {
            let mut counter = DotF8CounterInputGen::<Goldilocks>::new(BusDeviceMode::Counter);
            counter.lengths.extend_from_slice(lengths);
            (ChunkId(id), Box::new(counter))
        }

        let planner = DotF8Planner::<Goldilocks>(PhantomData);
        let plans = planner.plan(vec![chunk(9, &[0, DOT_F8_MAX_N]), chunk(2, &[DOT_F8_MAX_N - 8])]);
        let windows: Vec<_> = plans
            .iter()
            .map(|plan| {
                plan.meta
                    .as_ref()
                    .unwrap()
                    .downcast_ref::<HashMap<ChunkId, (u64, CollectSkipper)>>()
                    .unwrap()
            })
            .collect();
        assert_eq!(windows.len(), 2);
        assert_eq!(windows[0].len(), 2);
        assert_eq!(windows[0][&ChunkId(2)], (1, CollectSkipper::new(0)));
        assert_eq!(windows[0][&ChunkId(9)], (1, CollectSkipper::new(0)));
        assert_eq!(windows[1].len(), 1);
        assert_eq!(windows[1][&ChunkId(9)], (1, CollectSkipper::new(1)));
    }
}
