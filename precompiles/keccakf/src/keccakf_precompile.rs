use precompiles_common::{PrecompileCall, PrecompileCode};

use tiny_keccak::keccakf;

use crate::KeccakfSM;

use zisk_core::ZiskOp;

impl PrecompileCall for KeccakfSM {
    fn execute(&self, opcode: PrecompileCode, ctx: &mut InstContext) -> Option<(u64, bool)> {
        if opcode != ZiskOp::Keccakf.into() {
            panic!("Invalid opcode for Keccakf");
        }

        let step = ctx.a;
        let address = ctx.b;
    
        // Get address from register a0 = x10
        // let address = ctx.mem.read(REG_A0, 8);

        // Allocate room for 25 u64 = 128 bytes = 1600 bits
        const WORDS: usize = 25;
        let mut data = [0u64; WORDS];

        // Read them from the address
        for (i, d) in data.iter_mut().enumerate() {
            *d = ctx.mem.read(address + (8 * i as u64), 8);
        }

        // Call keccakf
        keccakf(&mut data);

        // Write them from the address
        for (i, d) in data.iter().enumerate() {
            ctx.mem.write(address + (8 * i as u64), *d, 8);
        }
    }
}

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
