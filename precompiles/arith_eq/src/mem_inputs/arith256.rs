use super::ArithEqMemInputConfig;
use crate::executors::Arith256;
use std::collections::VecDeque;
use zisk_common::BusId;
use zisk_common::MemCollectorInfo;

pub const ARITH_256_MEM_CONFIG: ArithEqMemInputConfig = ArithEqMemInputConfig {
    indirect_params: 5,
    rewrite_params: false,
    read_params: 3,
    write_params: 2,
    chunks_per_param: 4,
};

pub fn generate_arith256_mem_inputs(
    addr_main: u32,
    step_main: u64,
    data: &[u64],
    only_counters: bool,
    pending: &mut VecDeque<(BusId, Vec<u64>)>,
) {
    // op,op_type,a,b,addr[5],...
    let a: &[u64; 4] = &data[10..14].try_into().unwrap();
    let b: &[u64; 4] = &data[14..18].try_into().unwrap();
    let c: &[u64; 4] = &data[18..22].try_into().unwrap();
    // let mut dh = [0u64; 4];
    // let mut dl = [0u64; 4];
    let mut d: [u64; 8] = [0u64; 8];
    let (dl, dh) = d.split_at_mut(4);

    let dh: &mut [u64; 4] = dh.try_into().expect("slice dh without correct length");
    let dl: &mut [u64; 4] = dl.try_into().expect("slice dl without correct length");

    Arith256::calculate(a, b, c, dl, dh);
    super::generate_mem_inputs(
        addr_main,
        step_main,
        data,
        Some(&d),
        only_counters,
        pending,
        &ARITH_256_MEM_CONFIG,
    );
}

pub fn skip_arith256_mem_inputs(
    addr_main: u32,
    data: &[u64],
    mem_collectors_info: &[MemCollectorInfo],
) -> bool {
    super::skip_mem_inputs(addr_main, data, &ARITH_256_MEM_CONFIG, mem_collectors_info)
}
