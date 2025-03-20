//! The `ArithEqCounter` module defines a counter for tracking arith_eq-related operations
//! sent over the data bus. It connects to the bus and gathers metrics for specific
//! `ZiskOperationType::ArithEq` instructions.

use std::ops::Add;

use data_bus::{
    BusDevice, BusId, ExtOperationData, OperationBusData, MEM_BUS_ID, OPERATION_BUS_DATA_SIZE,
    OPERATION_BUS_ID,
};
use precompiles_common::MemBusHelpers;
use sm_common::{BusDeviceMode, Counter, Metrics};
use zisk_core::ZiskOperationType;

use crate::executors::{Arith256, Arith256Mod, Secp256k1};

/// The `ArithEqCounter` struct represents a counter that monitors and measures
/// arith_eq-related operations on the data bus.
///
/// It tracks specific operation types (`ZiskOperationType`) and updates counters for each
/// accepted operation type whenever data is processed on the bus.
pub struct ArithEqCounterInputGen {
    /// ArithEq counter.
    counter: Counter,

    /// Bus device mode (counter or input generator).
    mode: BusDeviceMode,
}

#[derive(Debug)]
struct ArithEqMemInputConfig {
    indirect_params: usize,
    rewrite_params: bool,
    read_params: usize,
    write_params: usize,
    chunks_per_param: usize,
}

const ARITH_256_MEM_CONFIG: ArithEqMemInputConfig = ArithEqMemInputConfig {
    indirect_params: 5,
    rewrite_params: false,
    read_params: 3,
    write_params: 2,
    chunks_per_param: 4,
};

const ARITH_256_MOD_MEM_CONFIG: ArithEqMemInputConfig = ArithEqMemInputConfig {
    indirect_params: 5,
    rewrite_params: false,
    read_params: 4,
    write_params: 1,
    chunks_per_param: 4,
};

const SECP256K1_ADD_MEM_CONFIG: ArithEqMemInputConfig = ArithEqMemInputConfig {
    indirect_params: 2,
    rewrite_params: true,
    read_params: 2,
    write_params: 1,
    chunks_per_param: 8,
};

const SECP256K1_DBL_MEM_CONFIG: ArithEqMemInputConfig = ArithEqMemInputConfig {
    indirect_params: 0,
    rewrite_params: true,
    read_params: 1,
    write_params: 1,
    chunks_per_param: 8,
};

impl ArithEqCounterInputGen {
    /// Creates a new instance of `ArithEqCounter`.
    ///
    /// # Arguments
    /// * `bus_id` - The ID of the bus to which this counter is connected.
    /// * `op_type` - A vector of `ZiskOperationType` instructions to monitor.
    ///
    /// # Returns
    /// A new `ArithEqCounter` instance.
    pub fn new(mode: BusDeviceMode) -> Self {
        Self { counter: Counter::default(), mode }
    }

    /// Retrieves the count of instructions for a specific `ZiskOperationType`.
    ///
    /// # Arguments
    /// * `op_type` - The operation type to retrieve the count for.
    ///
    /// # Returns
    /// Returns the count of instructions for the specified operation type.
    pub fn inst_count(&self, op_type: ZiskOperationType) -> Option<u64> {
        (op_type == ZiskOperationType::ArithEq).then_some(self.counter.inst_count)
    }

    fn generate_arith256_mem_inputs(
        addr_main: u32,
        step_main: u64,
        data: &[u64],
        only_counters: bool,
    ) -> Option<Vec<(BusId, Vec<u64>)>> {
        // op,op_type,a,b,addr[5],...
        let a: &[u64; 4] = &data[9..13].try_into().unwrap();
        let b: &[u64; 4] = &data[13..17].try_into().unwrap();
        let c: &[u64; 4] = &data[17..21].try_into().unwrap();
        // let mut dh = [0u64; 4];
        // let mut dl = [0u64; 4];
        let mut d: [u64; 8] = [0u64; 8];
        let (dh, dl) = d.split_at_mut(4);

        let dh: &mut [u64; 4] = dh.try_into().expect("slice dh without correct length");
        let dl: &mut [u64; 4] = dl.try_into().expect("slice dl without correct length");

        Arith256::calculate(a, b, c, dh, dl);
        Some(Self::generate_mem_inputs(
            addr_main,
            step_main,
            data,
            Some(&d),
            only_counters,
            &ARITH_256_MEM_CONFIG,
        ))
    }

    fn generate_arith256_mod_mem_inputs(
        addr_main: u32,
        step_main: u64,
        data: &[u64],
        only_counters: bool,
    ) -> Option<Vec<(BusId, Vec<u64>)>> {
        // op,op_type,a,b,addr[5],...
        let a: &[u64; 4] = &data[9..13].try_into().unwrap();
        let b: &[u64; 4] = &data[13..17].try_into().unwrap();
        let c: &[u64; 4] = &data[17..21].try_into().unwrap();
        let module: &[u64; 4] = &data[21..25].try_into().unwrap();
        let mut d: [u64; 4] = [0u64; 4];

        Arith256Mod::calculate(a, b, c, module, &mut d);
        Some(Self::generate_mem_inputs(
            addr_main,
            step_main,
            data,
            Some(&d),
            only_counters,
            &ARITH_256_MOD_MEM_CONFIG,
        ))
    }

    fn generate_secp256k1_add_mem_inputs(
        addr_main: u32,
        step_main: u64,
        data: &[u64],
        only_counters: bool,
    ) -> Option<Vec<(BusId, Vec<u64>)>> {
        // op,op_type,a,b,addr[2],...
        let p1: &[u64; 8] = &data[6..14].try_into().unwrap();
        let p2: &[u64; 8] = &data[14..22].try_into().unwrap();
        let mut p3 = [0u64; 8];

        Secp256k1::calculate_add(p1, p2, &mut p3);
        Some(Self::generate_mem_inputs(
            addr_main,
            step_main,
            data,
            Some(&p3),
            only_counters,
            &SECP256K1_ADD_MEM_CONFIG,
        ))
    }

    fn generate_secp256k1_dbl_mem_inputs(
        addr_main: u32,
        step_main: u64,
        data: &[u64],
        only_counters: bool,
    ) -> Option<Vec<(BusId, Vec<u64>)>> {
        // op,op_type,a,b,addr[2],...
        let p1: &[u64; 8] = &data[4..12].try_into().unwrap();
        let mut p3 = [0u64; 8];

        Secp256k1::calculate_dbl(p1, &mut p3);
        Some(Self::generate_mem_inputs(
            addr_main,
            step_main,
            data,
            Some(&p3),
            only_counters,
            &SECP256K1_DBL_MEM_CONFIG,
        ))
    }

    fn generate_mem_inputs(
        addr_main: u32,
        step_main: u64,
        data: &[u64],
        write_data: Option<&[u64]>,
        only_counters: bool,
        config: &ArithEqMemInputConfig,
    ) -> Vec<(BusId, Vec<u64>)> {
        let mut mem_inputs = Vec::new();
        let params_count = config.read_params + config.write_params;
        let params_offset = OPERATION_BUS_DATA_SIZE + config.indirect_params;
        println!("DATA: {:?}", data);
        println!("WRITE_DATA: {:?}", write_data);
        for iparam in 0..config.indirect_params {
            mem_inputs.push((
                MEM_BUS_ID,
                MemBusHelpers::mem_aligned_load(
                    addr_main + iparam as u32 * 8,
                    step_main,
                    data[OPERATION_BUS_DATA_SIZE + iparam],
                )
                .to_vec(),
            ));
        }
        for iparam in 0..params_count {
            let param_index = if config.rewrite_params && iparam >= config.read_params {
                iparam - config.read_params
            } else {
                iparam
            };
            let param_addr = if config.indirect_params > 0 {
                // read indirect parameters, means stored the address of parameter
                data[OPERATION_BUS_DATA_SIZE + param_index] as u32
            } else {
                addr_main + (param_index * 8 * config.chunks_per_param) as u32
            };

            // read/write all chunks of the iparam parameter
            let is_write = iparam >= config.read_params;
            let current_param_offset = if is_write {
                // if write calculate index over write_data
                config.chunks_per_param * (iparam - config.read_params)
            } else {
                // if read calculate param
                params_offset + config.chunks_per_param * iparam
            };
            for ichunk in 0..config.chunks_per_param {
                let chunk_data = if only_counters && is_write {
                    0
                } else if is_write {
                    let wlen = write_data.unwrap().len();
                    if current_param_offset + ichunk >= wlen {
                        println!(
                            "params_offset:{} current_param_offset:{} iparam:{} ichunk:{} index:{} len:{} config:{:?}",
                            params_offset, current_param_offset,
                            iparam,
                            ichunk,
                            current_param_offset + ichunk,
                            wlen,
                            config
                        );
                    }
                    write_data.unwrap()[current_param_offset + ichunk]
                } else {
                    if current_param_offset + ichunk >= data.len() {
                        println!(
                            "params_offset:{} current_param_offset:{} iparam:{} ichunk:{} index:{} len:{} config:{:?}",
                            params_offset, current_param_offset,
                            iparam,
                            ichunk,
                            current_param_offset + ichunk,
                            data.len(),
                            config
                        );
                    }
                    data[current_param_offset + ichunk]
                };
                mem_inputs.push((
                    MEM_BUS_ID,
                    MemBusHelpers::mem_aligned_op(
                        param_addr + ichunk as u32 * 8,
                        step_main,
                        chunk_data,
                        is_write,
                    )
                    .to_vec(),
                ));
            }
        }
        for (index, (_, mem_input)) in mem_inputs.iter().enumerate() {
            // 0 MEMORY_LOAD_OP,
            // 1 addr as u64,
            // 2 MEM_STEP_BASE + MAX_MEM_OPS_BY_MAIN_STEP * step + 2,
            // 3 8,
            // 4 mem_value,

            println!(
                "MEM_TO_BUS #{:2} {} 0x{:08X} {:10} {}{}",
                index,
                if mem_input[0] == 1 { "R" } else { "W" },
                mem_input[1],
                mem_input[2],
                if mem_input[0] == 1 { mem_input[4] } else { mem_input[6] },
                if only_counters { " (only counters)" } else { "" }
            );
        }
        mem_inputs
    }
}

impl Metrics for ArithEqCounterInputGen {
    /// Tracks activity on the connected bus and updates counters for recognized operations.
    ///
    /// # Arguments
    /// * `_bus_id` - The ID of the bus (unused in this implementation).
    /// * `_data` - The data received from the bus.
    ///
    /// # Returns
    /// An empty vector, as this implementation does not produce any derived inputs for the bus.
    fn measure(&mut self, _data: &[u64]) {
        self.counter.update(1);
    }

    /// Provides a dynamic reference for downcasting purposes.
    ///
    /// # Returns
    /// A reference to `self` as `dyn std::any::Any`.
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Add for ArithEqCounterInputGen {
    type Output = ArithEqCounterInputGen;

    /// Combines two `Arith256Counter` instances by summing their counters.
    ///
    /// # Arguments
    /// * `self` - The first `Arith256Counter` instance.
    /// * `other` - The second `Arith256Counter` instance.
    ///
    /// # Returns
    /// A new `Arith256Counter` with combined counters.
    fn add(self, other: Self) -> ArithEqCounterInputGen {
        ArithEqCounterInputGen { counter: &self.counter + &other.counter, mode: self.mode }
    }
}

impl BusDevice<u64> for ArithEqCounterInputGen {
    /// Processes data received on the bus, updating counters and generating inputs when applicable.
    ///
    /// # Arguments
    /// * `bus_id` - The ID of the bus sending the data.
    /// * `data` - The data received from the bus.
    ///
    /// # Returns
    /// A vector of derived inputs to be sent back to the bus.
    #[inline]
    fn process_data(&mut self, bus_id: &BusId, data: &[u64]) -> Option<Vec<(BusId, Vec<u64>)>> {
        debug_assert!(*bus_id == OPERATION_BUS_ID);

        let op_data: ExtOperationData<u64> = data.try_into().ok()?;
        let step_main = OperationBusData::get_a(&op_data);
        let addr_main = OperationBusData::get_b(&op_data) as u32;
        if OperationBusData::get_op_type(&op_data) as u32 != ZiskOperationType::ArithEq as u32 {
            return None;
        }

        let only_counters = self.mode == BusDeviceMode::Counter;
        if only_counters {
            self.measure(data);
        }
        match op_data {
            ExtOperationData::OperationArith256Data(_) => {
                Self::generate_arith256_mem_inputs(addr_main, step_main, data, only_counters)
            }
            ExtOperationData::OperationArith256ModData(_) => {
                Self::generate_arith256_mod_mem_inputs(addr_main, step_main, data, only_counters)
            }
            ExtOperationData::OperationSecp256k1AddData(_) => {
                Self::generate_secp256k1_add_mem_inputs(addr_main, step_main, data, only_counters)
            }
            ExtOperationData::OperationSecp256k1DblData(_) => {
                Self::generate_secp256k1_dbl_mem_inputs(addr_main, step_main, data, only_counters)
            }

            _ => None,
        }
    }

    /// Returns the bus IDs associated with this counter.
    ///
    /// # Returns
    /// A vector containing the connected bus ID.
    fn bus_id(&self) -> Vec<BusId> {
        vec![OPERATION_BUS_ID]
    }

    /// Provides a dynamic reference for downcasting purposes.
    fn as_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}
