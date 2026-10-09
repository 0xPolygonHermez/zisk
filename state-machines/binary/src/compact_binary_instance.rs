//! The instance of the fused `CompactBinary` air: a slice of each binary family, in one proof.

use crate::{
    BinaryAddCollector, BinaryAddHiCollector, BinaryBasicCollector, BinaryExtensionCollector,
    CompactBinaryCollectInfo, CompactBinaryCollector, CompactBinaryInputs, CompactBinarySM,
};
use pil2_std_lib::Std;
use proofman_common::{AirInstance, ProofCtx, ProofmanResult, SetupCtx};
use proofman_fields::PrimeField64;
use std::sync::Arc;
use zisk_common::StatsType;
use zisk_common::{
    BusDevice, CheckPoint, ChunkId, Instance, InstanceCtx, InstanceType, PayloadType,
};

pub struct CompactBinaryInstance<F: PrimeField64> {
    sm: Arc<CompactBinarySM<F>>,

    /// Instance context.
    ictx: InstanceCtx,

    /// What each block takes from each chunk.
    collect_info: CompactBinaryCollectInfo,

    /// Standard library instance, the collectors publish the frops multiplicities through it.
    std: Arc<Std<F>>,
}

impl<F: PrimeField64> CompactBinaryInstance<F> {
    pub fn new(sm: Arc<CompactBinarySM<F>>, mut ictx: InstanceCtx, std: Arc<Std<F>>) -> Self {
        let meta = ictx.plan.meta.take().expect("Expected metadata in ictx.plan.meta");
        let collect_info = *meta
            .downcast::<CompactBinaryCollectInfo>()
            .expect("Failed to downcast ictx.plan.meta to CompactBinaryCollectInfo");

        Self { sm, ictx, collect_info, std }
    }

    /// The collectors of one chunk: one per block whose plan reaches it.
    pub fn build_compact_binary_collector(&self, chunk_id: ChunkId) -> CompactBinaryCollector<F> {
        let info = &self.collect_info;
        CompactBinaryCollector {
            basic: info
                .basic
                .get(&chunk_id)
                .map(|collect| BinaryBasicCollector::new(*collect, self.std.clone())),
            add: info
                .add
                .get(&chunk_id)
                .map(|collect| BinaryAddCollector::new(*collect, self.std.clone())),
            add_hi: info
                .add_hi
                .get(&chunk_id)
                .map(|collect| BinaryAddHiCollector::new(*collect, self.std.clone())),
            ext: info
                .ext
                .get(&chunk_id)
                .map(|collect| BinaryExtensionCollector::new(*collect, self.std.clone())),
        }
    }
}

impl<F: PrimeField64> Instance<F> for CompactBinaryInstance<F> {
    fn compute_witness(
        &self,
        _pctx: &ProofCtx<F>,
        _sctx: &SetupCtx<F>,
        collectors: Vec<(usize, Box<dyn BusDevice<PayloadType>>)>,
        trace_buffer: Vec<F>,
        packed: bool,
    ) -> ProofmanResult<Option<AirInstance<F>>> {
        // One list per block, the chunks in collect order: each block's fill walks them exactly as
        // its standalone air walks the chunks of its own collectors.
        let mut inputs = CompactBinaryInputs::default();
        for (_, collector) in collectors {
            let compact = collector.as_any().downcast::<CompactBinaryCollector<F>>().unwrap();
            if let Some(c) = compact.basic {
                inputs.basic.push(c.inputs);
            }
            if let Some(c) = compact.add {
                inputs.add.push(c.inputs);
            }
            if let Some(c) = compact.add_hi {
                inputs.add_hi.push(c.inputs);
            }
            if let Some(c) = compact.ext {
                inputs.ext.push(c.inputs);
            }
        }

        Ok(Some(self.sm.compute_witness(&inputs, trace_buffer, packed)?))
    }

    fn check_point(&self) -> &CheckPoint {
        &self.ictx.plan.check_point
    }

    fn instance_type(&self) -> InstanceType {
        InstanceType::Instance
    }

    fn stats_type(&self) -> StatsType {
        StatsType::Opcodes
    }

    fn build_inputs_collector(&self, chunk_id: ChunkId) -> Option<Box<dyn BusDevice<PayloadType>>> {
        Some(Box::new(self.build_compact_binary_collector(chunk_id)))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
