use crate::CheckPointSkip;
use zisk_common::InstObserver;
use zisk_core::{InstContext, ZiskInst, ZiskOperationType, ZiskRequiredOperation};

use zisk_core::ZiskRom;
use ziskemu::{EmuTrace, ZiskEmulator};

pub struct InputsCollector {
    check_point: CheckPointSkip,
    num_rows: usize,
    op_type: ZiskOperationType,

    skipping: bool,
    skipped: u64,
    inputs: Vec<ZiskRequiredOperation>,
}

impl InputsCollector {
    pub fn collect(
        check_point: CheckPointSkip,
        num_rows: usize,
        zisk_rom: &ZiskRom,
        min_traces: &[EmuTrace],
        op_type: ZiskOperationType,
    ) -> Result<Vec<ZiskRequiredOperation>, Box<dyn std::error::Error + Send>> {
        let mut instance =
            Self { check_point, num_rows, skipping: true, skipped: 0, inputs: Vec::new(), op_type };

        let chunk_id = instance.check_point.chunk_id;

        let observer: &mut dyn InstObserver = &mut instance;

        // let mut data_bus = DataBus::<PayloadType, BusDeviceInstanceWrapper<F>>::new();

        // data_bus.connect_device(
        //     vec![5000],
        //     Box::new(BusDeviceInstanceWrapper::new(sec_instance)),
        // );

        ZiskEmulator::process_rom_slice_plan(zisk_rom, min_traces, chunk_id, observer);

        Ok(std::mem::take(&mut instance.inputs))
    }

    pub fn collect2(
        check_point: CheckPointSkip,
        num_rows: usize,
        zisk_rom: &ZiskRom,
        min_traces: &[EmuTrace],
        op_type: ZiskOperationType,
    ) -> Result<Vec<ZiskRequiredOperation>, Box<dyn std::error::Error + Send>> {
        let mut instance =
            Self { check_point, num_rows, skipping: true, skipped: 0, inputs: Vec::new(), op_type };

        let chunk_id = instance.check_point.chunk_id;

        let observer: &mut dyn InstObserver = &mut instance;

        // let mut data_bus = DataBus::<PayloadType, BusDeviceInstanceWrapper<F>>::new();

        // data_bus.connect_device(
        //     vec![5000],
        //     Box::new(BusDeviceInstanceWrapper::new(sec_instance)),
        // );

        ZiskEmulator::process_rom_slice_plan(zisk_rom, min_traces, chunk_id, observer);

        Ok(std::mem::take(&mut instance.inputs))
    }
}

impl InstObserver for InputsCollector {
    #[inline(always)]
    fn on_instruction(&mut self, zisk_inst: &ZiskInst, inst_ctx: &InstContext) -> bool {
        if zisk_inst.op_type != self.op_type {
            return false;
        }

        if self.skipping {
            if self.check_point.collect_info.skip == 0
                || self.skipped == self.check_point.collect_info.skip
            {
                self.skipping = false;
            } else {
                self.skipped += 1;
                return false;
            }
        }

        let a = if zisk_inst.m32 { inst_ctx.a & 0xffffffff } else { inst_ctx.a };
        let b = if zisk_inst.m32 { inst_ctx.b & 0xffffffff } else { inst_ctx.b };

        self.inputs.push(ZiskRequiredOperation { step: inst_ctx.step, opcode: zisk_inst.op, a, b });

        self.inputs.len() == self.num_rows
    }
}
