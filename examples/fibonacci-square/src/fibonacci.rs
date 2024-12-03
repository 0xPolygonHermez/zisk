use std::sync::Arc;

use proofman_common::{AirInstance, ExecutionCtx, ProofCtx, SetupCtx};
use proofman::{WitnessManager, WitnessComponent};

use p3_field::PrimeField;

use crate::{FibonacciSquareTrace, FibonacciSquareRomTrace, Module, FIBONACCI_SQUARE_AIRGROUP_ID, FIBONACCI_SQUARE_AIR_IDS};

pub struct FibonacciSquare<F: PrimeField> {
    module: Arc<Module<F>>,
}

impl<F: PrimeField + Copy> FibonacciSquare<F> {
    const MY_NAME: &'static str = "FiboSqre";

    pub fn new(wcm: Arc<WitnessManager<F>>, module: Arc<Module<F>>) -> Arc<Self> {
        let fibonacci = Arc::new(Self { module });

        wcm.register_component(fibonacci.clone(), Some(FIBONACCI_SQUARE_AIRGROUP_ID), Some(FIBONACCI_SQUARE_AIR_IDS));

        fibonacci
    }

    pub fn execute(&self, pctx: Arc<ProofCtx<F>>, ectx: Arc<ExecutionCtx>, sctx: Arc<SetupCtx>) {
        // TODO: We should create the instance here and fill the trace in calculate witness!!!
        if let Err(e) =
            Self::calculate_trace(self, FIBONACCI_SQUARE_AIRGROUP_ID, FIBONACCI_SQUARE_AIR_IDS[0], pctx, ectx, sctx)
        {
            panic!("Failed to calculate fibonacci: {:?}", e);
        }
    }

    fn calculate_trace(
        &self,
        airgroup_id: usize,
        air_id: usize,
        pctx: Arc<ProofCtx<F>>,
        ectx: Arc<ExecutionCtx>,
        sctx: Arc<SetupCtx>,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        log::debug!("{} ··· Starting witness computation stage {}", Self::MY_NAME, 1);

        let module = pctx.get_public_value("mod");
        let mut a = pctx.get_public_value("in1");
        let mut b = pctx.get_public_value("in2");

        let (buffer_size, offsets) = ectx.buffer_allocator.as_ref().get_buffer_info(
            &sctx,
            FIBONACCI_SQUARE_AIRGROUP_ID,
            FIBONACCI_SQUARE_AIR_IDS[0],
        )?;

        let mut buffer = vec![F::zero(); buffer_size as usize];

        let num_rows = pctx.global_info.airs[airgroup_id][air_id].num_rows;
        let mut trace = FibonacciSquareTrace::map_buffer(&mut buffer, num_rows, offsets[0] as usize)?;

        trace[0].a = F::from_canonical_u64(a);
        trace[0].b = F::from_canonical_u64(b);

        for i in 1..num_rows {
            let tmp = b;
            let result = self.module.calculate_module(a.pow(2) + b.pow(2), module);
            (a, b) = (tmp, result);

            trace[i].a = F::from_canonical_u64(a);
            trace[i].b = F::from_canonical_u64(b);
        }

        let (buffer_size_rom, offsets_rom, commit_id) = ectx.buffer_allocator.as_ref().get_buffer_info_custom_commit(
            &sctx,
            FIBONACCI_SQUARE_AIRGROUP_ID,
            FIBONACCI_SQUARE_AIR_IDS[0],
            "rom",
        )?;

        let mut buffer_rom = vec![F::zero(); buffer_size_rom as usize];

        let mut trace_custom_commits =
            FibonacciSquareRomTrace::map_buffer(&mut buffer_rom, num_rows, offsets_rom[0] as usize)?;
        for i in 0..num_rows {
            trace_custom_commits[i].line = F::from_canonical_u64(3 + i as u64);
            trace_custom_commits[i].flags = F::from_canonical_u64(2 + i as u64);
        }

        pctx.set_public_value_by_name(b, "out", None);

        pctx.set_proof_value("value1", F::from_canonical_u64(5));
        pctx.set_proof_value("value2", F::from_canonical_u64(125));

        // Not needed, for debugging!
        // let mut result = F::zero();
        // for (i, _) in buffer.iter().enumerate() {
        //     result += buffer[i] * F::from_canonical_u64(i as u64);
        // }
        // log::info!("Result Fibonacci buffer: {:?}", result);

        let mut air_instance =
            AirInstance::new(sctx.clone(), FIBONACCI_SQUARE_AIRGROUP_ID, FIBONACCI_SQUARE_AIR_IDS[0], Some(0), buffer);
        air_instance.set_airvalue(&sctx, "FibonacciSquare.fibo1", Some(vec![0]), F::from_canonical_u64(1));
        air_instance.set_airvalue(&sctx, "FibonacciSquare.fibo1", Some(vec![1]), F::from_canonical_u64(2));
        air_instance.set_airvalue_ext(&sctx, "FibonacciSquare.fibo3", None, vec![F::from_canonical_u64(5); 3]);
        match ectx.cached_buffers_path.as_ref().and_then(|cached_buffers| cached_buffers.get("rom").cloned()) {
            Some(buffer_path) => air_instance.set_custom_commit_cached_file(&sctx, commit_id, buffer_path),
            None => air_instance.set_custom_commit_id_buffer(&sctx, buffer_rom, commit_id),
        }
        let (is_myne, gid) =
            ectx.dctx.write().unwrap().add_instance(FIBONACCI_SQUARE_AIRGROUP_ID, FIBONACCI_SQUARE_AIR_IDS[0], 1);
        if is_myne {
            pctx.air_instance_repo.add_air_instance(air_instance, Some(gid));
        }

        Ok(b)
    }
}

impl<F: PrimeField + Copy> WitnessComponent<F> for FibonacciSquare<F> {
    fn calculate_witness(
        &self,
        _stage: u32,
        _air_instance_id: Option<usize>,
        _pctx: Arc<ProofCtx<F>>,
        _ectx: Arc<ExecutionCtx>,
        _sctx: Arc<SetupCtx>,
    ) {
    }
}
