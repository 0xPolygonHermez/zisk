use std::sync::Arc;

use proofman_common::{add_air_instance, AirInstance, FromTrace, ProofCtx};
use witness::WitnessComponent;

use p3_field::PrimeField64;

use crate::{
    FibonacciSquareRomTrace, BuildPublicValues, BuildProofValues, FibonacciSquareAirValues, FibonacciSquareTrace,
    Module,
};

pub struct FibonacciSquare<F: PrimeField64> {
    module: Arc<Module<F>>,
}

impl<F: PrimeField64 + Copy> FibonacciSquare<F> {
    const MY_NAME: &'static str = "FiboSqre";

    pub fn new(module: Arc<Module<F>>) -> Arc<Self> {
        Arc::new(Self { module })
    }
}

impl<F: PrimeField64 + Copy> WitnessComponent<F> for FibonacciSquare<F> {
    fn execute(&self, pctx: Arc<ProofCtx<F>>) {
        log::debug!("{} ··· Starting witness computation stage {}", Self::MY_NAME, 1);

        let mut publics = BuildPublicValues::from_vec_guard(pctx.get_publics());

        let module = F::as_canonical_u64(&publics.module);
        let mut a = F::as_canonical_u64(&publics.in1);
        let mut b = F::as_canonical_u64(&publics.in2);

        let mut trace = FibonacciSquareTrace::new_zeroes();

        trace[0].a = F::from_canonical_u64(a);
        trace[0].b = F::from_canonical_u64(b);

        for i in 1..trace.num_rows() {
            let tmp = b;
            let result = self.module.calculate_module(a.pow(2) + b.pow(2), module);
            (a, b) = (tmp, result);

            trace[i].a = F::from_canonical_u64(a);
            trace[i].b = F::from_canonical_u64(b);
        }

        publics.out = trace[trace.num_rows() - 1].b;

        let mut trace_rom = FibonacciSquareRomTrace::new_zeroes();

        for i in 0..trace_rom.num_rows() {
            trace_rom[i].line = F::from_canonical_u64(3 + i as u64);
            trace_rom[i].flags = F::from_canonical_u64(2 + i as u64);
        }

        let mut proof_values = BuildProofValues::from_vec_guard(pctx.get_proof_values());
        proof_values.value1 = F::from_canonical_u64(5);
        proof_values.value2 = F::from_canonical_u64(125);

        let mut air_values = FibonacciSquareAirValues::<F>::new();
        air_values.fibo1[0] = F::from_canonical_u64(1);
        air_values.fibo1[1] = F::from_canonical_u64(2);
        air_values.fibo3 = [F::from_canonical_u64(5), F::from_canonical_u64(5), F::from_canonical_u64(5)];

        let air_instance = AirInstance::new_from_trace(
            FromTrace::new(&mut trace).with_custom_traces(vec![&mut trace_rom]).with_air_values(&mut air_values),
        );
        add_air_instance::<F>(air_instance, pctx.clone());
    }

    fn debug(&self, _pctx: Arc<ProofCtx<F>>) {
        // let trace = FibonacciSquareTrace::from_vec(pctx.get_air_instance_trace(0, 0, 0));
        // let air_values = FibonacciSquareAirValues::from_vec(pctx.get_air_instance_air_values(0, 0, 0));
        // let airgroup_values = FibonacciSquareAirGroupValues::from_vec(pctx.get_air_instance_airgroup_values(0, 0, 0));

        // let publics = BuildPublicValues::from_vec_guard(pctx.get_publics());
        // let proof_values = BuildProofValues::from_vec_guard(pctx.get_proof_values());

        // log::info!("{}    First row 1: {:?}", Self::MY_NAME, trace[1]);
        // log::info!("{}    Air values: {:?}", Self::MY_NAME, air_values);
        // log::info!("{}    Airgroup values: {:?}", Self::MY_NAME, airgroup_values);
        // log::info!("{}    Publics: {:?}", Self::MY_NAME, publics);
        // log::info!("{}    Proof values: {:?}", Self::MY_NAME, proof_values);
    }
}
