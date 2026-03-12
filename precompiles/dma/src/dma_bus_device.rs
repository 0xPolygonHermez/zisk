//! The `DmaCounter` module defines a counter for tracking dma-related operations
//! sent over the data bus. It connects to the bus and gathers metrics for specific
//! `ZiskOperationType::Dma` instructions.

use std::fmt;
use std::ops::Add;

use precompiles_common::MemProcessor;
use precompiles_helpers::DmaInfo;
use zisk_common::{BusDevice, BusDeviceMode, BusId, Metrics, OPERATION_BUS_ID, OP_TYPE, STEP};
use zisk_common::{OP, OPERATION_PRECOMPILED_BUS_DATA_SIZE};
use zisk_core::zisk_ops::ZiskOp;
use zisk_core::ZiskOperationType;

use crate::{generate_dma_mem_inputs, skip_dma_mem_inputs};

// The `DmaOpMultiCounter` struct represents a counter that monitors and measures
// dma specific operation on the data bus.
//
// Dma              Full   OnlyMemCpy   OnlyInputCpy
// Dma64Aligned     Full4  OnlyMemCpy8  OnlyInputCpy4 OnlyMemSet8 Mem4
// DmaUnaligned     Full
// DmaPrePost       Full   OnlyMemCpy   OnlyInputCpy
//
// MEMCPY + XMEMCPY
//    dma_memcpy | dma_pre_post_memcpy | dma_unaligned  (unaligned_dst_src)
//    dma_memcpy | dma_pre_post_memcpy | dma_64_aligned_memcpy (aligned_dst_src + lcount > 4)
//    dma_memcpy | dma_pre_post_memcpy | dma_64_aligned_mem  (aligned_dst_src + lcount <= 4)
//
// MEMCMP
//    dma | dma_pre_post | dma_unaligned  (unaligned_dst_src)
//    dma | dma_pre_post | dma_64_aligned_mem  (aligned_dst_src)
//
// INPUTCPY
//    dma_inputcpy | dma_pre_post_inputcpy | dma_64_aligned_inputcpy
//
// XMEMSET
//    dma | dma_pre_post | dma_64_aligned_memset
//
// With this config, for the memcpy the limit was 4 words of 64-bits, more than 4 it's
// better a OnlyMemCpy8 vs Mem4
//
// DMA => 4 counters
// DMA_PRE_POST => 4
// DMA_UNALIGNED = 4
// DMA_UNALIGNED_INPUTS = 4
// DMA_64_ALIGNED_ROWS = 6
// DMA_64_ALIGNED_INPUTS = 4

pub const DMA_OFFSET: usize = 0;
pub const DMA_PRE_POST_OFFSET: usize = 4;
pub const DMA_UNALIGNED_OFFSET: usize = 8;
pub const DMA_UNALIGNED_INPUTS_OFFSET: usize = 12;
pub const DMA_64_ALIGNED_OFFSET: usize = 16;
pub const DMA_64_ALIGNED_INPUTS_OFFSET: usize = 22;
pub const DMA_INPUT_GEN_COUNTERS: usize = 26;

pub const DMA_COUNTER_MEMCPY: usize = 0;
pub const DMA_COUNTER_MEMSET: usize = 1;
pub const DMA_COUNTER_MEMCMP: usize = 2;
pub const DMA_COUNTER_INPUTCPY: usize = 3;
pub const DMA_COUNTER_MEMCPY_8: usize = 4;
pub const DMA_COUNTER_MEMSET_8: usize = 5;

pub const DMA_COUNTER_OPS: usize = 4;
pub const DMA_COUNTER_OPS_EXT: usize = 6;
#[derive(Debug)]
pub struct DmaCounterInputGen {
    pub counters: [usize; DMA_INPUT_GEN_COUNTERS],

    mode: BusDeviceMode,
}

impl fmt::Display for DmaCounterInputGen {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ROWS:\n                   \
                              memcpy4  memcpy8   memcmp inputcpy  memset4  memset8\n  \
             dma             {:>8}          {:>8} {:>8} {:>8}         \n  \
             dma_pre_post    {:>8}          {:>8} {:>8} {:>8}         \n  \
             dma_64_aligned  {:>8} {:>8} {:>8} {:>8} {:>8} {:>8}\n  \
             dma_unaligned   {:>8}          {:>8} {:>8} {:>8}         \n\n  \
             INPUTS\n                   \
                              memcpy4  memcpy8   memcmp inputcpy  memset4  memset8\n  \
             dma_64_aligned  {:>8}          {:>8} {:>8} {:>8}         \n  \
             dma_unaligned   {:>8}          {:>8} {:>8} {:>8}         \n\n",
            self.counters[DMA_OFFSET + DMA_COUNTER_MEMCPY],
            self.counters[DMA_OFFSET + DMA_COUNTER_MEMCMP],
            self.counters[DMA_OFFSET + DMA_COUNTER_INPUTCPY],
            self.counters[DMA_OFFSET + DMA_COUNTER_MEMSET],
            self.counters[DMA_PRE_POST_OFFSET + DMA_COUNTER_MEMCPY],
            self.counters[DMA_PRE_POST_OFFSET + DMA_COUNTER_MEMCMP],
            self.counters[DMA_PRE_POST_OFFSET + DMA_COUNTER_INPUTCPY],
            self.counters[DMA_PRE_POST_OFFSET + DMA_COUNTER_MEMSET],
            self.counters[DMA_64_ALIGNED_OFFSET + DMA_COUNTER_MEMCPY],
            self.counters[DMA_64_ALIGNED_OFFSET + DMA_COUNTER_MEMCPY_8],
            self.counters[DMA_64_ALIGNED_OFFSET + DMA_COUNTER_MEMCMP],
            self.counters[DMA_64_ALIGNED_OFFSET + DMA_COUNTER_INPUTCPY],
            self.counters[DMA_64_ALIGNED_OFFSET + DMA_COUNTER_MEMSET],
            self.counters[DMA_64_ALIGNED_OFFSET + DMA_COUNTER_MEMSET_8],
            self.counters[DMA_UNALIGNED_OFFSET + DMA_COUNTER_MEMCPY],
            self.counters[DMA_UNALIGNED_OFFSET + DMA_COUNTER_MEMCMP],
            self.counters[DMA_UNALIGNED_OFFSET + DMA_COUNTER_INPUTCPY],
            self.counters[DMA_UNALIGNED_OFFSET + DMA_COUNTER_MEMSET],
            self.counters[DMA_64_ALIGNED_INPUTS_OFFSET + DMA_COUNTER_MEMCPY],
            self.counters[DMA_64_ALIGNED_INPUTS_OFFSET + DMA_COUNTER_MEMCMP],
            self.counters[DMA_64_ALIGNED_INPUTS_OFFSET + DMA_COUNTER_INPUTCPY],
            self.counters[DMA_64_ALIGNED_INPUTS_OFFSET + DMA_COUNTER_MEMSET],
            self.counters[DMA_UNALIGNED_INPUTS_OFFSET + DMA_COUNTER_MEMCPY],
            self.counters[DMA_UNALIGNED_INPUTS_OFFSET + DMA_COUNTER_MEMCMP],
            self.counters[DMA_UNALIGNED_INPUTS_OFFSET + DMA_COUNTER_INPUTCPY],
            self.counters[DMA_UNALIGNED_INPUTS_OFFSET + DMA_COUNTER_MEMSET],
        )
    }
}

impl DmaCounterInputGen {
    /// Creates a new instance of `DmaCounter`.
    ///
    /// # Arguments
    /// * `mode` - The ID of the bus to which this counter is connected.
    ///
    /// # Returns
    /// A new `DmaCounter` instance.
    pub fn new(mode: BusDeviceMode) -> Self {
        Self { counters: [0; DMA_INPUT_GEN_COUNTERS], mode }
    }
    const OPS_X_ROW: [usize; 6] = [
        4, // MEMCPY_4
        4, // MEMSET_4
        4, // MEMCMP
        4, // INPUTCPY
        8, // MEMCPY_8
        8, // MEMSET_8
    ];
    const IS_DOUBLE: [usize; 6] = [
        1, // MEMCPY_4
        1, // MEMSET_4
        0, // MEMCMP
        0, // INPUTCPY
        0, // MEMCPY_8
        0, // MEMSET_8
    ];

    fn incr_counters(&mut self, encoded: u64, operation: usize, _step: u64) {
        if !DmaInfo::is_direct(encoded) {
            if DmaInfo::get_pre_count(encoded) > 0 {
                self.counters[DMA_PRE_POST_OFFSET + operation] += 1;
            }
            if DmaInfo::get_post_count(encoded) > 0 {
                self.counters[DMA_PRE_POST_OFFSET + operation] += 1;
            }
            self.counters[DMA_OFFSET + operation] += 1;
        }
        let loop_count = DmaInfo::get_loop_count(encoded);
        // it's effective loop count
        let use_src = operation != DMA_COUNTER_MEMSET && operation != DMA_COUNTER_INPUTCPY;
        if loop_count > 0 {
            if DmaInfo::dst_is_aligned_with_src(encoded) || !use_src {
                let rows = loop_count.div_ceil(Self::OPS_X_ROW[operation]);
                self.counters[DMA_64_ALIGNED_OFFSET + operation] += rows;
                self.counters[DMA_64_ALIGNED_INPUTS_OFFSET + operation] += 1;
                if Self::IS_DOUBLE[operation] == 1 {
                    let rows = loop_count.div_ceil(Self::OPS_X_ROW[operation + 4]);
                    self.counters[DMA_64_ALIGNED_OFFSET + operation + 4] += rows;
                }
            } else {
                self.counters[DMA_UNALIGNED_OFFSET + operation] += loop_count + 1;
                self.counters[DMA_UNALIGNED_INPUTS_OFFSET + operation] += 1;
            }
        }
    }

    /// Retrieves the count of instructions for a specific `ZiskOperationType`.
    ///
    /// # Arguments
    /// * `dst` - The destination address of operation.
    /// * `src` - The source address of operation.
    /// * `count` - The bytes of operation.
    pub fn inst_count(&mut self, encoded: u64, op: u8, step: u64) {
        // count and plan no need the count, need the effective count:
        // effective_count = if is_equal { count } else { count_eq + 1 }
        // the count encoded was effective
        match op {
            ZiskOp::DMA_MEMCPY | ZiskOp::DMA_XMEMCPY => {
                // if DmaInfo::dst_is_aligned_with_src(encoded) {
                self.incr_counters(encoded, DMA_COUNTER_MEMCPY, step);
                // }
            }
            ZiskOp::DMA_MEMCMP | ZiskOp::DMA_XMEMCMP => {
                self.incr_counters(encoded, DMA_COUNTER_MEMCMP, step)
            }
            ZiskOp::DMA_INPUTCPY => self.incr_counters(encoded, DMA_COUNTER_INPUTCPY, step),
            ZiskOp::DMA_XMEMSET => self.incr_counters(encoded, DMA_COUNTER_MEMSET, step),
            _ => panic!("Unknown DMA Cmd 0x{op:02X}"),
        }
    }

    /// Processes data received on the bus, updating counters and generating inputs when applicable.
    ///
    /// # Arguments
    /// * `bus_id` - The ID of the bus sending the data.
    /// * `data` - The data received from the bus.
    /// * `mem_processors` – A queue of mem_processors bus operations used to send derived inputs.
    ///
    /// # Returns
    /// A boolean indicating whether the program should continue execution or terminate.
    /// Returns `true` to continue execution, `false` to stop.
    #[inline(always)]
    pub fn process_data<P: MemProcessor>(
        &mut self,
        bus_id: &BusId,
        data: &[u64],
        data_ext: &[u64],
        mem_processors: &mut P,
    ) -> bool {
        debug_assert!(*bus_id == OPERATION_BUS_ID);

        if data[OP_TYPE] as u32 != ZiskOperationType::Dma as u32 {
            return true;
        }

        match self.mode {
            BusDeviceMode::Counter => {
                self.measure(data);
                generate_dma_mem_inputs(data, data_ext, true, mem_processors);
            }
            BusDeviceMode::CounterAsm => {
                self.measure(data);
            }
            BusDeviceMode::InputGenerator => {
                if skip_dma_mem_inputs(data, data_ext, mem_processors) {
                    return true;
                }
                generate_dma_mem_inputs(data, data_ext, false, mem_processors);
            }
        }

        true
    }
}

impl Metrics for DmaCounterInputGen {
    /// Tracks activity on the connected bus and updates counters for recognized operations.
    ///
    /// # Arguments
    /// * `_bus_id` - The ID of the bus (unused in this implementation).
    /// * `_data` - The data received from the bus.
    ///
    /// # Returns
    /// An empty vector, as this implementation does not produce any derived inputs for the bus.
    #[inline(always)]
    fn measure(&mut self, data: &[u64]) {
        if data[OP_TYPE] != ZiskOperationType::Dma as u64 {
            return;
        }
        let op = data[OP] as u8;
        let encoded = data[OPERATION_PRECOMPILED_BUS_DATA_SIZE];
        self.inst_count(encoded, op, data[STEP]);
    }

    /// Provides a dynamic reference for downcasting purposes.
    ///
    /// # Returns
    /// A reference to `self` as `dyn std::any::Any`.
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Add for DmaCounterInputGen {
    type Output = DmaCounterInputGen;

    /// Combines two `DmaCounter` instances by summing their counters.
    ///
    /// # Arguments
    /// * `self` - The first `DmaCounter` instance.
    /// * `other` - The second `DmaCounter` instance.
    ///
    /// # Returns
    /// A new `DmaCounter` with combined counters.
    fn add(self, other: Self) -> DmaCounterInputGen {
        DmaCounterInputGen {
            counters: std::array::from_fn(|i| self.counters[i] + other.counters[i]),
            mode: self.mode.clone(),
        }
    }
}

impl Add<&DmaCounterInputGen> for &DmaCounterInputGen {
    type Output = DmaCounterInputGen;

    /// Combines two `DmaCounter` references by summing their counters.
    ///
    /// # Arguments
    /// * `self` - Reference to the first `DmaCounter` instance.
    /// * `other` - Reference to the second `DmaCounter` instance.
    ///
    /// # Returns
    /// A new `DmaCounter` with combined counters.
    fn add(self, other: &DmaCounterInputGen) -> DmaCounterInputGen {
        DmaCounterInputGen {
            counters: std::array::from_fn(|i| self.counters[i] + other.counters[i]),
            mode: self.mode.clone(),
        }
    }
}
impl BusDevice<u64> for DmaCounterInputGen {
    /// Provides a dynamic reference for downcasting purposes.
    fn as_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}
