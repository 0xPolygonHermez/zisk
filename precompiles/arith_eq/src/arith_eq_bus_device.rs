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

struct ArithEqMemInputConfig {
    indirect_params: bool,
    rewrite_params: bool,
    read_params: usize,
    write_params: usize,
    chunks_per_param: usize,
}

const ARITH_256_MEM_CONFIG: ArithEqMemInputConfig = ArithEqMemInputConfig {
    indirect_params: true,
    rewrite_params: false,
    read_params: 3,
    write_params: 2,
    chunks_per_param: 4,
};

const ARITH_256_MOD_MEM_CONFIG: ArithEqMemInputConfig = ArithEqMemInputConfig {
    indirect_params: true,
    rewrite_params: false,
    read_params: 4,
    write_params: 1,
    chunks_per_param: 4,
};

const SECP256K1_ADD_MEM_CONFIG: ArithEqMemInputConfig = ArithEqMemInputConfig {
    indirect_params: true,
    rewrite_params: true,
    read_params: 2,
    write_params: 1,
    chunks_per_param: 8,
};

const SECP256K1_DBL_MEM_CONFIG: ArithEqMemInputConfig = ArithEqMemInputConfig {
    indirect_params: false,
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
        (op_type == ZiskOperationType::Arith256).then_some(self.counter.inst_count)
    }

    fn generate_mem_inputs(
        addr_main: u32,
        step_main: u64,
        data: &[u64],
        config: &ArithEqMemInputConfig,
    ) -> Option<Vec<(BusId, Vec<u64>)>> {
        let mut mem_inputs = Vec::new();
        let params_count = config.read_params + config.write_params;
        let params_offset =
            OPERATION_BUS_DATA_SIZE + if config.indirect_params { params_count } else { 0 };
        for iparam in 0..params_count {
            let param_index = if config.rewrite_params && iparam >= config.read_params {
                iparam - config.read_params
            } else {
                iparam
            };
            let param_addr = if config.indirect_params {
                // read indirect parameters, means stored the address of parameter
                let param_addr = data[OPERATION_BUS_DATA_SIZE + param_index as usize];
                mem_inputs.push((
                    MEM_BUS_ID,
                    MemBusHelpers::mem_aligned_load(
                        addr_main + param_index as u32 * 8,
                        step_main,
                        param_addr,
                    )
                    .to_vec(),
                ));
                param_addr as u32
            } else {
                addr_main + (param_index * 8 * config.chunks_per_param) as u32
            };

            // read/write all chunks of the iparam parameter
            let is_write = iparam >= config.read_params;
            for ichunk in 0..config.chunks_per_param {
                let chunk_data = data[params_offset + config.chunks_per_param * iparam + ichunk];
                mem_inputs.push((
                    MEM_BUS_ID,
                    MemBusHelpers::mem_aligned_op(
                        param_addr as u32 + ichunk as u32 * 8,
                        step_main,
                        chunk_data,
                        is_write,
                    )
                    .to_vec(),
                ));
            }
        }
        Some(mem_inputs)
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

        match op_data {
            ExtOperationData::OperationArith256Data(_) => {
                if self.mode == BusDeviceMode::Counter {
                    self.measure(data);
                }
                Self::generate_mem_inputs(addr_main, step_main, data, &ARITH_256_MEM_CONFIG)
            }
            ExtOperationData::OperationArith256ModData(_) => {
                if self.mode == BusDeviceMode::Counter {
                    self.measure(data);
                }
                Self::generate_mem_inputs(addr_main, step_main, data, &ARITH_256_MOD_MEM_CONFIG)
            }
            ExtOperationData::OperationSecp256k1AddData(_) => {
                if self.mode == BusDeviceMode::Counter {
                    self.measure(data);
                }
                Self::generate_mem_inputs(addr_main, step_main, data, &SECP256K1_ADD_MEM_CONFIG)
            }
            ExtOperationData::OperationSecp256k1DblData(_) => {
                if self.mode == BusDeviceMode::Counter {
                    self.measure(&data);
                }
                Self::generate_mem_inputs(addr_main, step_main, data, &SECP256K1_DBL_MEM_CONFIG)
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
