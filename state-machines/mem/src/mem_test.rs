#![cfg(test)]
use std::{collections::VecDeque, sync::Arc};

use crate::{
    MemCounters, MemModulePlanner, MemModulePlannerConfig, MemPlanCalculator, MEMORY_LOAD_OP,
    MEMORY_STORE_OP,
};
use zisk_common::{BusDevice, ChunkId, Plan, MEM_BUS_ID};

fn generate_test_plans(
    from_addr: u32,
    rows: u32,
    counters: Vec<(ChunkId, &MemCounters)>,
) -> Vec<Plan> {
    let addr_index = match from_addr {
        0x8000_0000 => 0,
        0x9000_0000 => 1,
        0xA000_0000 => 2,
        _ => panic!("invalid addr 0x{from_addr:X}"),
    };
    let mut planner = MemModulePlanner::new(
        MemModulePlannerConfig {
            airgroup_id: 0,
            air_id: addr_index + 10,
            addr_index,
            from_addr: from_addr >> 3,
            rows,
            consecutive_addr: true,
        },
        Arc::new(counters),
    );
    planner.module_plan();
    planner.collect_plans()
}

fn add_test_aligned_mem_reads(
    counter: &mut MemCounters,
    count: u64,
    step_delta: u64,
    addr: u32,
    step: u64,
    value: u64,
) {
    for i in 0..count {
        counter.process_data(
            &MEM_BUS_ID,
            &[MEMORY_LOAD_OP as u64, addr as u64, step + i * step_delta, 8, value],
            &mut VecDeque::new(),
        );
    }
}
#[derive(Debug, Default, Clone)]
struct ConfigNextOp {
    pub step_delta: u64,
    pub step_cycle: u64,
    pub addr_delta: u64,
    pub addr_cycle: u64,
}

#[allow(clippy::too_many_arguments)]
fn add_mem_data(
    counter: &mut MemCounters,
    count: u64,
    addr: u32,
    step: u64,
    value: u64,
    width: u64,
    is_write: bool,
    config: &ConfigNextOp,
) {
    let mut addr = addr as u64;
    let mut step = step;
    let op = if is_write { MEMORY_STORE_OP } else { MEMORY_LOAD_OP } as u64;
    for i in 0..count {
        counter.process_data(&MEM_BUS_ID, &[op, addr, step, width, value], &mut VecDeque::new());
        if config.step_cycle > 0 {
            if i > 0 && (config.step_cycle % i) == 0 {
                step += config.step_delta;
            }
        } else {
            step += config.step_delta;
        }
        if config.addr_cycle > 0 {
            if i > 0 && (config.addr_cycle % i) == 0 {
                addr += config.addr_delta;
            }
        } else {
            addr += config.addr_delta;
        }
    }
}

// fn add_mem_read(counter: &mut MemCounters, addr: u32, step: u64, value: u64, width: u64) {
//     counter.process_data(&MEM_BUS_ID, &[MEMORY_LOAD_OP as u64, addr as u64, step, width, value]);
// }

// fn add_mem_write(counter: &mut MemCounters, addr: u32, step: u64, value: u64, width: u64) {
//     counter.process_data(&MEM_BUS_ID, &[MEMORY_STORE_OP as u64, addr as u64, step, width, value]);
// }

fn add_mem_read64(counter: &mut MemCounters, addr: u32, step: u64, value: u64) {
    counter.process_data(
        &MEM_BUS_ID,
        &[MEMORY_LOAD_OP as u64, addr as u64, step, 8, value],
        &mut VecDeque::new(),
    );
}

fn add_mem_write64(counter: &mut MemCounters, addr: u32, step: u64, value: u64) {
    counter.process_data(
        &MEM_BUS_ID,
        &[MEMORY_STORE_OP as u64, addr as u64, step, 8, value],
        &mut VecDeque::new(),
    );
}

#[test]
fn test_mem_module_planner_empty() {
    let counter = MemCounters::new();
    let counters: Vec<(ChunkId, &MemCounters)> = vec![(ChunkId(0), &counter)];
    let plans = generate_test_plans(0xA000_0000, 4, counters);
    assert_eq!(plans.len(), 0);
}
#[test]
fn test_mem_module_planner_with_exact_one_segment() {
    let mut counter = MemCounters::new();
    add_test_aligned_mem_reads(&mut counter, 4, 10, 0xA000_0000, 10, 0x0000_0000);
    counter.close();
    let counters: Vec<(ChunkId, &MemCounters)> = vec![(ChunkId(0), &counter)];

    let plans = generate_test_plans(0xA000_0000, 4, counters);
    assert_eq!(plans.len(), 1);
}

#[test]
fn test_mem_module_planner() {
    let mut counter = MemCounters::new();
    add_test_aligned_mem_reads(&mut counter, 5, 10, 0xA000_0000, 10, 0x0000_0000);
    counter.close();
    let counters: Vec<(ChunkId, &MemCounters)> = vec![(ChunkId(0), &counter)];

    let plans = generate_test_plans(0xA000_0000, 4, counters);
    assert_eq!(plans.len(), 2);
}

#[test]
fn test_counters() {
    let mut counter = MemCounters::new();
    let cfg = ConfigNextOp { step_delta: 1, step_cycle: 0, addr_delta: 8, addr_cycle: 0 };
    add_mem_data(&mut counter, 10, 0x8000_0000, 10, 0x01020304_05060708, 8, true, &cfg);
    add_mem_data(&mut counter, 10, 0x8000_0000, 100, 0x01020304_05060708, 8, false, &cfg);
    add_mem_write64(&mut counter, 0xA000_0002, 18, 0x2222_2222_2222_2222);
    add_mem_read64(&mut counter, 0xA000_0000, 12, 0x1111_1111_1111_1111);
    add_mem_read64(&mut counter, 0xA000_0000, 40, 0x2222_2222_2222_1111);
    add_mem_read64(&mut counter, 0xA000_0016, 50, 0x3333_3333_3333_3333);
    add_mem_data(&mut counter, 10, 0x9000_0000, 10, 0x4041_4243_4445_4647, 8, false, &cfg);
    counter.close();
    assert_eq!(format!("{counter:?}"), "[MEM_0,#:10 => 0x80000000:2 0x80000008:2 0x80000010:2 0x80000018:2 0x80000020:2 0x80000028:2 0x80000030:2 0x80000038:2 0x80000040:2 0x80000048:2][MEM_1,#:10 => 0x90000000:1 0x90000008:1 0x90000010:1 0x90000018:1 0x90000020:1 0x90000028:1 0x90000030:1 0x90000038:1 0x90000040:1 0x90000048:1][MEM_2,#:4 => 0xA0000000:4 0xA0000008:2 0xA0000010:1 0xA0000018:1]");
}

/*
#[test]
fn test_mem() {
    let mem_sm = MemSM::new();
    let std_sm =

    let mem_bus_device = <MemSM as sm_common::ComponentBuilder<Goldilocks>>::build_counter(&mem_sm);

    let mut data_bus = DataBus::<u64, BusDeviceMetricsWrapper>::new();
    data_bus.connect_device(
        vec![OPERATION_BUS_ID],
        Box::new(BusDeviceMetricsWrapper::new(arith_bus_device, false)),
    );

    let data = vec![
        (OPERATION_BUS_ID, OperationBusData::from_values(Mul as u8, Arith as u64, 1, 2).into()),
        (OPERATION_BUS_ID, OperationBusData::from_values(Div as u8, Arith as u64, 1, 2).into()),
        (OPERATION_BUS_ID, OperationBusData::from_values(Add as u8, Binary as u64, 1, 2).into()),
        (OPERATION_BUS_ID, OperationBusData::from_values(Sub as u8, Binary as u64, 1, 2).into()),
    ];

    DataBusPlayer::play(&mut data_bus, data);

    let arith_counter = data_bus.devices.remove(0).inner;

    let arith_planner =
        <ArithSM as sm_common::ComponentBuilder<Goldilocks>>::build_planner(&arith_sm);

    let plan = arith_planner.plan(vec![(0, arith_counter)]);

    println!("Plan: {:?}", plan);
}
*/
#[test]
fn full() {}
