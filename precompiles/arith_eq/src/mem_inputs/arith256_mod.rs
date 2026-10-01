use super::ArithEqMemInputConfig;
use crate::executors::Arith256Mod;
use zisk_core::zisk_ops::swap_endianness_elements;
use zisk_precomp_common::MemProcessor;

pub const ARITH_256_MOD_MEM_CONFIG: ArithEqMemInputConfig = ArithEqMemInputConfig {
    indirect_params: 5,
    rewrite_params: false,
    read_params: 4,
    write_params: 1,
    chunks_per_param: 4,
};
pub fn generate_arith256_mod_mem_inputs<P: MemProcessor>(
    addr_main: u32,
    step_main: u64,
    data: &[u64],
    only_counters: bool,
    big_endian: bool,
    mem_processors: &mut P,
) {
    // op,op_type,a,b,addr[5],...
    let a: &[u64; 4] = &super::operand::<4>(&data[10..14], big_endian);
    let b: &[u64; 4] = &super::operand::<4>(&data[14..18], big_endian);
    let c: &[u64; 4] = &super::operand::<4>(&data[18..22], big_endian);
    let module: &[u64; 4] = &super::operand::<4>(&data[22..26], big_endian);
    let mut d: [u64; 4] = [0u64; 4];

    Arith256Mod::calculate(a, b, c, module, &mut d);
    if big_endian {
        swap_endianness_elements(&mut d, 4);
    }
    super::generate_mem_inputs(
        addr_main,
        step_main,
        data,
        Some(&d),
        only_counters,
        mem_processors,
        &ARITH_256_MOD_MEM_CONFIG,
    );
}

pub fn skip_arith256_mod_mem_inputs<P: MemProcessor>(
    addr_main: u32,
    data: &[u64],
    mem_processors: &mut P,
) -> bool {
    super::skip_mem_inputs(addr_main, data, &ARITH_256_MOD_MEM_CONFIG, mem_processors)
}
