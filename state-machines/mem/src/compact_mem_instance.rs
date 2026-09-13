//! The instance of the fused `CompactMem` air: segment 0 of the three memory areas, in one proof.

use std::sync::Arc;

use proofman_common::{AirInstance, ProofCtx, ProofmanResult, SetupCtx};
use proofman_fields::PrimeField64;
use zisk_common::StatsType;
#[allow(unused_imports)]
use zisk_common::{phase_end, phase_log, phase_ms, phase_start};
use zisk_common::{
    BusDevice, CheckPoint, ChunkId, Instance, InstanceCtx, InstanceType, PayloadType, SegmentId,
};
use zisk_sm_mem_common::{CompactMemSegmentCheckPoint, MemArea, MemModuleSegmentCheckPoint};

use crate::{
    CompactMemBlockInputs, CompactMemCollector, CompactMemInputs, CompactMemSM, MemInput,
    MemModule, MemModuleCollector, MemOps, MemPreviousSegment,
};

/// What a block of the instance needs to build its collectors, read off the module of the area it
/// stands for so it cannot drift from what that area's own air does.
struct BlockModule {
    area: MemArea,
    min_addr: u32,
    is_dual: bool,
    init: bool,
}

pub struct CompactMemInstance<F: PrimeField64> {
    /// Instance context
    ictx: InstanceCtx,

    sm: Arc<CompactMemSM<F>>,

    check_point: CompactMemSegmentCheckPoint,

    blocks: Vec<BlockModule>,
}

impl<F: PrimeField64> CompactMemInstance<F> {
    pub fn new(
        sm: Arc<CompactMemSM<F>>,
        mem_module: Arc<dyn MemModule<F>>,
        input_data_module: Arc<dyn MemModule<F>>,
        rom_data_module: Arc<dyn MemModule<F>>,
        ictx: InstanceCtx,
    ) -> Self {
        let meta = ictx.plan.meta.as_ref().unwrap();
        let check_point = meta.downcast_ref::<CompactMemSegmentCheckPoint>().unwrap().clone();

        let blocks = [
            (MemArea::Mem, mem_module),
            (MemArea::InputData, input_data_module),
            (MemArea::RomData, rom_data_module),
        ]
        .into_iter()
        .map(|(area, module)| {
            debug_assert_eq!(
                area.name(),
                module.get_mem_name(),
                "MemArea::name must match the module's own name, the witness logs use both"
            );
            BlockModule {
                area,
                min_addr: module.get_addr_range().0,
                is_dual: module.is_dual(),
                init: module.is_initializable(),
            }
        })
        .collect();

        Self { ictx, sm, check_point, blocks }
    }

    fn segment(&self, area: MemArea) -> &MemModuleSegmentCheckPoint {
        self.check_point.area(area)
    }

    /// The collectors of one chunk: one per area whose segment reaches this chunk.
    fn build_collectors(
        &self,
        chunk_id: ChunkId,
        mem_sections: Option<&dyn zisk_core::MemDataSection>,
    ) -> CompactMemCollector {
        let collectors = self
            .blocks
            .iter()
            .filter_map(|block| {
                let segment = self.segment(block.area);
                // An area whose segment does not touch this chunk simply has nothing to collect
                // here; the union of the three chunk sets is what the plan subscribes to.
                let chunk_check_point = segment.chunks.get(&chunk_id)?;
                let mut collector = MemModuleCollector::new(
                    chunk_check_point,
                    block.min_addr,
                    SegmentId(0),
                    Some(chunk_id) == segment.first_chunk_id,
                    block.is_dual,
                    #[cfg(feature = "save_addr_action")]
                    block.area.name(),
                    #[cfg(feature = "save_addr_action")]
                    chunk_id.0,
                );
                if block.init && chunk_id == ChunkId(0) {
                    let sections = mem_sections.expect(
                        "CompactMemInstance: an initializable area needs the memory sections \
                         on chunk 0",
                    );
                    collector.init_with_mem_sections(sections);
                }
                Some((block.area, collector))
            })
            .collect();

        CompactMemCollector::new(collectors)
    }

    pub fn build_mem_collector(
        &self,
        chunk_id: ChunkId,
        mem_sections: &dyn zisk_core::MemDataSection,
    ) -> CompactMemCollector {
        self.build_collectors(chunk_id, Some(mem_sections))
    }
}

/// The operations of one area, gathered from every chunk's collector for it.
struct BlockGather {
    inputs: Vec<Vec<MemInput>>,
    previous_segment: Option<MemPreviousSegment>,
}

impl<F: PrimeField64> Instance<F> for CompactMemInstance<F> {
    fn compute_witness(
        &self,
        _pctx: &ProofCtx<F>,
        _sctx: &SetupCtx<F>,
        collectors: Vec<(usize, Box<dyn BusDevice<PayloadType>>)>,
        trace_buffer: Vec<F>,
        packed: bool,
    ) -> ProofmanResult<Option<AirInstance<F>>> {
        phase_start!(t_gather);
        #[cfg(feature = "witness_timers")]
        let n_collectors = collectors.len();

        // One gather per area. The per-chunk vectors are kept as they are and walked lazily by
        // `MemOps`: concatenating them used to be the largest single cost of a memory witness.
        let mut gathers: Vec<BlockGather> = self
            .blocks
            .iter()
            .map(|_| BlockGather { inputs: Vec::new(), previous_segment: None })
            .collect();

        for (_, collector) in collectors {
            let compact = collector.as_any().downcast::<CompactMemCollector>().unwrap();
            for (area, mem_module_collector) in compact.into_collectors() {
                let index = self.blocks.iter().position(|b| b.area == area).unwrap();
                let gather = &mut gathers[index];
                if mem_module_collector.prev_segment.is_some() {
                    assert!(
                        gather.previous_segment.is_none(),
                        "CompactMemInstance: two chunks claim the previous segment of {}",
                        area.name()
                    );
                    gather.previous_segment = mem_module_collector.prev_segment;
                }
                gather.inputs.push(mem_module_collector.inputs);
            }
        }

        let block_inputs: Vec<CompactMemBlockInputs<'_>> = self
            .blocks
            .iter()
            .zip(gathers.iter())
            .map(|(block, gather)| {
                let mem_ops = MemOps::new(&gather.inputs);
                assert!(
                    !mem_ops.is_empty(),
                    "CompactMemInstance: the {} block has no operation; CompactMem is only \
                     planned when the three areas have a segment to prove",
                    block.area.name()
                );
                CompactMemBlockInputs {
                    mem_ops,
                    // No hand-over means this area starts where its cycle was opened: the base
                    // address, step 0, value 0 -- the same default the standalone instance uses.
                    previous_segment: match &gather.previous_segment {
                        Some(previous) => MemPreviousSegment {
                            addr: previous.addr,
                            step: previous.step,
                            value: previous.value,
                        },
                        None => MemPreviousSegment { addr: block.min_addr, step: 0, value: 0 },
                    },
                }
            })
            .collect();

        phase_end!(d_gather, t_gather);
        phase_log!(
            "CompactMem gather: {} + {} + {} ops from {} chunks in {:.0}ms",
            block_inputs[0].mem_ops.len(),
            block_inputs[1].mem_ops.len(),
            block_inputs[2].mem_ops.len(),
            n_collectors,
            phase_ms!(d_gather)
        );

        let mut block_inputs = block_inputs.into_iter();
        let inputs = CompactMemInputs {
            mem: block_inputs.next().unwrap(),
            input_data: block_inputs.next().unwrap(),
            rom_data: block_inputs.next().unwrap(),
        };

        Ok(Some(self.sm.compute_witness(inputs, &self.check_point, trace_buffer, packed)?))
    }

    fn build_inputs_collector(&self, chunk_id: ChunkId) -> Option<Box<dyn BusDevice<PayloadType>>> {
        // Two of the three areas are initializable, and initialization needs the memory sections,
        // which this entry point does not carry. `build_mem_collector` is the one the executor
        // calls for memory airs.
        panic!(
            "CompactMemInstance: build_inputs_collector has no memory sections to initialize \
             chunk {chunk_id:?} with; use build_mem_collector"
        );
    }

    fn check_point(&self) -> &CheckPoint {
        &self.ictx.plan.check_point
    }

    fn instance_type(&self) -> InstanceType {
        InstanceType::Instance
    }

    fn stats_type(&self) -> StatsType {
        StatsType::Memory
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
