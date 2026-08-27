use proofman_fields::PrimeField64;
use zisk_common::OP;
use zisk_core::zisk_ops::ZiskOp;
use zisk_precomp_common::{MemProcessor, PrecompileMemInputs};

use crate::mem_inputs::{generate_babyjubjub_add_mem_inputs, skip_babyjubjub_add_mem_inputs};
use crate::BabyJubJubSM;

const BABYJUBJUB_ADD_OP: u8 = ZiskOp::BabyJubJubAdd.code();

impl<F: PrimeField64> PrecompileMemInputs for BabyJubJubSM<F> {
    fn generate<P: MemProcessor>(
        addr_main: u32,
        step_main: u64,
        data: &[u64],
        only_counters: bool,
        mem_processors: &mut P,
    ) {
        match data[OP] as u8 {
            BABYJUBJUB_ADD_OP => generate_babyjubjub_add_mem_inputs(
                addr_main,
                step_main,
                data,
                only_counters,
                mem_processors,
            ),
            _ => panic!("BabyJubJubSM::generate: unsupported sub-op {}", data[OP] as u8),
        }
    }

    fn should_skip<P: MemProcessor>(addr_main: u32, data: &[u64], mem_processors: &mut P) -> bool {
        match data[OP] as u8 {
            BABYJUBJUB_ADD_OP => skip_babyjubjub_add_mem_inputs(addr_main, data, mem_processors),
            _ => panic!("BabyJubJubSM::should_skip: unsupported sub-op {}", data[OP] as u8),
        }
    }
}
