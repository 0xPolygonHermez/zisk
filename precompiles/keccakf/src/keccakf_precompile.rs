use tiny_keccak::keccakf;
use zisk_common::{MemPrecompilesOps, PrecompiledEmulationMode, ZiskPrecompile};

use zisk_core::REG_A0;
// fn read_reg_fn(&self, reg:usize) -> u64 {
//     self.ctx.regs[reg as usize]
// }

// fn read_mem_fn(&self, address:u64) -> u64 {
//     self.ctx.mem.read(address, 8)
// }

// fn write_mem_fn(&self, address:u64, data:u64) {
//     self.ctx.mem.write(address, data, 8);
// }

// fn read_mem_gen_mem_reads_fn(&self, address:u64) -> u64 {
//     let value = self.ctx.mem.read(address, 8);
//     self.ctx.mem_reads.append(value);

//     value
// }

// fn read_mem_reads_fn(&self) -> u64 {
//     self.ctx.mem_reads[index]
//     self.ctx.mem_reads_index += 1;
// }

pub struct KeccakOp;

impl ZiskPrecompile for KeccakOp {
    fn execute(
        &self,
        _a: u64,
        _b: u64,
        emulation_mode: PrecompiledEmulationMode,
        mut mem_ops: MemPrecompilesOps,
    ) -> (u64, bool) {
        // Get address from register a0 = x10
        let address = (mem_ops.read_reg_fn)(REG_A0);
        assert!(address & 0x7 == 0, "opc_keccak() found address not aligned to 8 bytes");

        // Allocate room for 25 u64 = 128 bytes = 1600 bits
        const WORDS: usize = 25;
        let mut data = [0u64; WORDS];

        // Get input data from memory or from the precompiled context
        match emulation_mode {
            PrecompiledEmulationMode::None | PrecompiledEmulationMode::GenerateMemReads => {
                for (i, d) in data.iter_mut().enumerate() {
                    *d = (mem_ops.read_mem_fn)(
                        address + (8 * i as u64),
                        emulation_mode == PrecompiledEmulationMode::GenerateMemReads,
                    );
                }
            }
            PrecompiledEmulationMode::ConsumeMemReads => {
                let mut input_data = Vec::new();
                for d in data.iter_mut() {
                    *d = (mem_ops.get_mem_read).as_mut().unwrap()();
                    input_data.push(*d);
                }
                (mem_ops.write_input_data)(input_data);
            }
        }

        // Call keccakf
        keccakf(&mut data);

        // Write data to the memory address
        for (i, d) in data.iter().enumerate() {
            (mem_ops.write_mem_fn)(address + (8 * i as u64), *d);
        }

        (0, false)
    }
}

// impl PrecompileCall for KeccakfSM {
//     fn execute(&self, opcode: PrecompileCode, ctx: &mut InstContext) -> Option<(u64, bool)> {
//         println!("Executing Keccakf XXXXXXXXXXXXXX");
//         if opcode.value() != ZiskOp::Keccak as u16 {
//             panic!("Invalid opcode for Keccakf");
//         }

//         let address = ctx.b;

//         // Allocate room for 25 u64 = 128 bytes = 1600 bits
//         const WORDS: usize = 25;
//         let mut data = [0u64; WORDS];

//         // Read data from memory
//         for (i, d) in data.iter_mut().enumerate() {
//             *d = ctx.mem.read(address + (8 * i as u64), 8);
//         }

//         // Call keccakf
//         keccakf(&mut data);

//         // Write the modified data back to memory at the same address
//         for (i, d) in data.iter().enumerate() {
//             ctx.mem.write(address + (8 * i as u64), *d, 8);
//         }

//         Some((0, false))
//     }

//     fn execute_experimental<MemReadFn, MemWriteFn>(
//         &self,
//         opcode: PrecompileCode,
//         _a: u64,
//         b: u64,
//         mem_read: MemReadFn,
//         mem_write: MemWriteFn,
//     ) -> Option<(u64, bool)>
//     where
//         MemReadFn: Fn(u64) -> u64,
//         MemWriteFn: Fn(u64, u64),
//     {
//         if opcode.value() != ZiskOp::Keccak as u16 {
//             panic!("Invalid opcode for Keccakf");
//         }

//         let address = b;

//         // Allocate room for 25 u64 = 128 bytes = 1600 bits
//         const WORDS: usize = 25;
//         let mut data = [0u64; WORDS];

//         // Read data from memory
//         for (i, d) in data.iter_mut().enumerate() {
//             *d = mem_read(address + (8 * i as u64));
//         }

//         // Call keccakf
//         keccakf(&mut data);

//         // Write the modified data back to memory at the same address
//         for (i, d) in data.iter().enumerate() {
//             mem_write(address + (8 * i as u64), *d);
//         }

//         Some((0, false))
//     }
// }

// #[cfg(test)]
// mod tests {
//     use data_bus::{DataBus, DataBusPlayer, OPERATION_BUS_ID};
//     use p3_goldilocks::Goldilocks;
//     use sm_common::{BusDeviceInstanceWrapper, ComponentBuilder, Plan};
//     use zisk_core::zisk_ops::ZiskOp;
//     use zisk_pil::{ARITH_AIR_IDS, ZISK_AIRGROUP_ID};

//     use crate::ArithSM;

//     use super::*;

//     /// Tests the basic functionality of the `plan` function with multiple chunks.
//     #[test]
//     fn test_rom_instance() {
//         type F = Goldilocks;

//         let collect_skipper = Box::new(CollectSkipper::new(0));
//         let plan = Plan::new(
//             ZISK_AIRGROUP_ID,
//             ARITH_AIR_IDS[0],
//             None,
//             InstanceType::Instance,
//             CheckPoint::Single(0),
//             Some(collect_skipper),
//             None,
//         );
//         let ictx = InstanceCtx::new(0, plan);

//         let arith_sm = ArithSM::new();
//         let arith_bus_device_instance = arith_sm.build_inputs_collector(ictx);

//         let mut data_bus = DataBus::<u64, BusDeviceInstanceWrapper<F>>::new();
//         data_bus.connect_device(
//             vec![OPERATION_BUS_ID],
//             Box::new(BusDeviceInstanceWrapper::new(arith_bus_device_instance)),
//         );

//         let operation_bus_data = OperationBusData::<u64>::from_values(
//             0,
//             ZiskOp::Mul as u8,
//             ZiskOperationType::Arith as u64,
//             1,
//             2,
//         );
//         let data_slice = vec![(5000, operation_bus_data.to_vec())];
//         DataBusPlayer::play(&mut data_bus, data_slice);

//         let mut device = data_bus.devices.remove(0).inner;

//         let air_instance = device.compute_witness(None).unwrap();
//     }
// }
