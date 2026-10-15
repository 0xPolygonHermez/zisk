use proofman_fields::PrimeField64;
use zisk_common::OP;
use zisk_core::zisk_ops::ZiskOp;
use zisk_precomp_common::{MemProcessor, PrecompileMemInputs};

use crate::executors::TwistedEdwardsCurve;
use crate::mem_inputs::{
    generate_twisted_edwards_add_mem_inputs, skip_twisted_edwards_add_mem_inputs,
};
use crate::TwistedEdwardsSM;

impl<F: PrimeField64> PrecompileMemInputs for TwistedEdwardsSM<F> {
    fn generate<P: MemProcessor>(
        addr_main: u32,
        step_main: u64,
        data: &[u64],
        only_counters: bool,
        mem_processors: &mut P,
    ) {
        let curve = match data[OP] as u8 {
            ZiskOp::BABYJUBJUB_ADD => TwistedEdwardsCurve::BabyJubJub,
            ZiskOp::ED25519_ADD => TwistedEdwardsCurve::Ed25519,
            _ => panic!("TwistedEdwardsSM::generate: unsupported sub-op {}", data[OP] as u8),
        };
        generate_twisted_edwards_add_mem_inputs(
            curve,
            addr_main,
            step_main,
            data,
            only_counters,
            mem_processors,
        )
    }

    fn should_skip<P: MemProcessor>(addr_main: u32, data: &[u64], mem_processors: &mut P) -> bool {
        match data[OP] as u8 {
            ZiskOp::BABYJUBJUB_ADD | ZiskOp::ED25519_ADD => {
                skip_twisted_edwards_add_mem_inputs(addr_main, data, mem_processors)
            }
            _ => panic!("TwistedEdwardsSM::should_skip: unsupported sub-op {}", data[OP] as u8),
        }
    }
}
