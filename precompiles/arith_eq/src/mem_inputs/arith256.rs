use super::ArithEqMemInputConfig;
use crate::executors::Arith256;
use zisk_core::zisk_ops::swap_endianness_elements;
use zisk_precomp_common::MemProcessor;

pub const ARITH_256_MEM_CONFIG: ArithEqMemInputConfig = ArithEqMemInputConfig {
    indirect_params: 5,
    rewrite_params: false,
    read_params: 3,
    write_params: 2,
    chunks_per_param: 4,
};

pub fn generate_arith256_mem_inputs<P: MemProcessor>(
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
    // let mut dh = [0u64; 4];
    // let mut dl = [0u64; 4];
    let mut d: [u64; 8] = [0u64; 8];
    let (dl, dh) = d.split_at_mut(4);

    let dh: &mut [u64; 4] = dh.try_into().expect("slice dh without correct length");
    let dl: &mut [u64; 4] = dl.try_into().expect("slice dl without correct length");

    Arith256::calculate(a, b, c, dl, dh);
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
        &ARITH_256_MEM_CONFIG,
    );
}

pub fn skip_arith256_mem_inputs<P: MemProcessor>(
    addr_main: u32,
    data: &[u64],
    mem_processors: &mut P,
) -> bool {
    super::skip_mem_inputs(addr_main, data, &ARITH_256_MEM_CONFIG, mem_processors)
}
