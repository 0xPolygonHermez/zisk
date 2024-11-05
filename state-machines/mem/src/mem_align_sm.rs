use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};

use p3_field::Field;
use proofman::{WitnessComponent, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx, SetupCtx};
use rayon::Scope;
use sm_common::{MemUnalignedOp, OpResult, Provable};
use zisk_core::ZiskRequiredMemory;
use zisk_pil::{MEM_AIRGROUP_ID, MEM_UNALIGNED_AIR_IDS};

const PROVE_CHUNK_SIZE: usize = 1 << 12;
const CHUNKS: u64 = 8;

pub struct MemUnalignedSM {
    // Count of registered predecessors
    registered_predecessors: AtomicU32,

    // Inputs
    inputs: Mutex<Vec<MemUnalignedOp>>,
}

#[allow(unused, unused_variables)]
impl MemUnalignedSM {
    const MY_NAME: &'static str = "MemUnaligned";

    pub fn new<F>(wcm: Arc<WitnessManager<F>>) -> Arc<Self> {
        let mem_unaligned_sm =
            Self { registered_predecessors: AtomicU32::new(0), inputs: Mutex::new(Vec::new()) };
        let mem_unaligned_sm = Arc::new(mem_unaligned_sm);

        wcm.register_component(
            mem_unaligned_sm.clone(),
            Some(MEM_AIRGROUP_ID),
            Some(MEM_UNALIGNED_AIR_IDS),
        );

        mem_unaligned_sm
    }

    pub fn register_predecessor(&self) {
        self.registered_predecessors.fetch_add(1, Ordering::SeqCst);
    }

    pub fn unregister_predecessor<F: Field>(&self, scope: &Scope) {
        if self.registered_predecessors.fetch_sub(1, Ordering::SeqCst) == 1 {
            <MemUnalignedSM as Provable<MemUnalignedOp, OpResult>>::prove(self, &[], true, scope);
        }
    }

    fn read(
        &self,
        _addr: u64,
        _width: usize, /* , _ctx: &mut ProofCtx<F>, _ectx: &ExecutionCtx */
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        Ok((0, true))
    }

    fn write(
        &self,
        _addr: u64,
        _width: usize,
        _val: u64, /* , _ctx: &mut ProofCtx<F>, _ectx: &ExecutionCtx */
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        Ok((0, true))
    }

    pub fn process_slice(
        input: &Vec<ZiskRequiredMemory>,
        multiplicity: &mut [u64],
        range_check: &mut HashMap<u64, u64>,
    ) -> Vec<MemUnalign0Row<F>> {
        // Is a write or a read operation
        let wr = input[0].is_write;

        // Get the address
        let addr = input[0].address;
        let addr_prior = input[1].address; // addr / CHUNKS;
        let addr_next = input[2].address;  // addr / CHUNKS + CHUNKS;

        // Get the value
        let value = input[0].value;
        let value_first_read = input[1].value;
        let value_first_write = input[2].value;
        let value_second_read = input[3].value;
        let value_second_write = input[4].value;

        // Get the step
        let step = input[0].step;
        let step_first_read = input[1].step;
        let step_first_write = input[2].step;
        let step_second_read = input[3].step;
        let step_second_write = input[4].step;

        // Get the offset
        let offset = addr % CHUNKS;

        // Get the width
        let width = input[0].width;

        // Compute the shift
        let shift = (offset + width) % CHUNKS;

        // Get the program to be executed, its size and the pc to jump to
        let program = MemUnalignedRomSM::get_program(offset, width, wr);
        let program_size = MemUnalignedRomSM::get_program_size(offset, width, wr);
        let next_pc = MemUnalignedRomSM::calculate_next_pc(offset, width, wr);

        // Initialize and set the rows of the corresponding program
        let mut rows: Vec<MemUnalign0Row<F>> = Vec::with_capacity(program_size);
        match program {
            0 => { // RV
                let mut read_row = MemUnalign0Row::<F> {
                    step: F::from_canonical_u64(step_first_read),
                    addr: F::from_canonical_u64(addr_prior),
                    // offset: F::from_canonical_u64(0),
                    width: F::from_canonical_u64(CHUNKS),
                    // wr: F::from_bool(false),
                    // pc: F::from_canonical_u64(0),
                    reset: F::from_bool(true),
                    sel_up_to_down: F::from_bool(true),
                    ..Default::default()
                };

                let mut value_row = MemUnalign0Row::<F> {
                    step: F::from_canonical_u64(step),
                    addr: F::from_canonical_u64(addr),
                    offset: F::from_canonical_u64(offset),
                    width: F::from_canonical_u64(width),
                    // wr: F::from_bool(false),
                    pc: F::from_canonical_u64(next_pc),
                    // reset: F::from_bool(false),
                    sel_prove: F::from_bool(true),
                    ..Default::default()
                };

                for i in 0..CHUNKS {
                    read_row.reg[i] = F::from_canonical_u64(value_first_read[i]);
                    read_row.sel[i] = F::from_bool(true);

                    value_row.reg[i] = F::from_canonical_u64(value[shift + i]);
                    value_row.sel[i] = F::from_bool(i == offset);

                    // Store the range check
                    *range_check.entry(read_row.reg[i]).or_insert(0) += 1;
                    *range_check.entry(value_row.reg[i]).or_insert(0) += 1;
                }

                // Store the rows
                rows.push(read_row);
                rows.push(value_row);
            },
            1 => { // RWV
                let mut read_row = MemUnalign0Row::<F> {
                    step: F::from_canonical_u64(step_first_read),
                    addr: F::from_canonical_u64(addr_prior),
                    // offset: F::from_canonical_u64(0),
                    width: F::from_canonical_u64(CHUNKS),
                    // wr: F::from_bool(false),
                    // pc: F::from_canonical_u64(0),
                    reset: F::from_bool(true),
                    sel_up_to_down: F::from_bool(true),
                    ..Default::default()
                };

                let mut write_row = MemUnalign0Row::<F> {
                    step: F::from_canonical_u64(step_first_write),
                    addr: F::from_canonical_u64(addr_prior),
                    // offset: F::from_canonical_u64(0),
                    width: F::from_canonical_u64(CHUNKS),
                    wr: F::from_bool(true),
                    pc: F::from_canonical_u64(next_pc),
                    // reset: F::from_bool(false),
                    sel_up_to_down: F::from_bool(true),
                    ..Default::default()
                };

                let mut value_row = MemUnalign0Row::<F> {
                    step: F::from_canonical_u64(step),
                    addr: F::from_canonical_u64(addr),
                    offset: F::from_canonical_u64(offset),
                    width: F::from_canonical_u64(width),
                    // wr: F::from_bool(false),
                    pc: F::from_canonical_u64(next_pc + 1),
                    // reset: F::from_bool(false),
                    sel_prove: F::from_bool(true),
                    ..Default::default()
                };

                for i in 0..CHUNKS {
                    read_row.reg[i] = F::from_canonical_u64(value_first_read[i]);
                    read_row.sel[i] = F::from_bool(i < offset);

                    write_row.reg[i] = F::from_canonical_u64(value_first_write[i]);
                    write_row.sel[i] = F::from_bool(i >= offset);

                    value_row.reg[i] = F::from_canonical_u64(value[shift + i]);
                    value_row.sel[i] = F::from_bool(i == offset);

                    // Store the range check
                    *range_check.entry(read_row.reg[i]).or_insert(0) += 1;
                    *range_check.entry(write_row.reg[i]).or_insert(0) += 1;
                    *range_check.entry(value_row.reg[i]).or_insert(0) += 1;
                }

                // Store the rows
                rows.push(read_row);
                rows.push(write_row);
                rows.push(value_row);
            }
            2 => {
                // RVR
                let mut first_read_row = MemUnalign0Row::<F> {
                    step: F::from_canonical_u64(step_first_read),
                    addr: F::from_canonical_u64(addr_prior),
                    // offset: F::from_canonical_u64(0),
                    width: F::from_canonical_u64(CHUNKS),
                    // wr: F::from_bool(false),
                    // pc: F::from_canonical_u64(0),
                    reset: F::from_bool(true),
                    sel_up_to_down: F::from_bool(true),
                    ..Default::default()
                };

                let mut value_row = MemUnalign0Row::<F> {
                    step: F::from_canonical_u64(step),
                    addr: F::from_canonical_u64(addr),
                    offset: F::from_canonical_u64(offset),
                    width: F::from_canonical_u64(width),
                    // wr: F::from_bool(false),
                    pc: F::from_canonical_u64(next_pc),
                    // reset: F::from_bool(false),
                    sel_prove: F::from_bool(true),
                    ..Default::default()
                };

                let mut second_read_row = MemUnalign0Row::<F> {
                    step: F::from_canonical_u64(step_second_read),
                    addr: F::from_canonical_u64(addr_next),
                    // offset: F::from_canonical_u64(0),
                    width: F::from_canonical_u64(CHUNKS),
                    // wr: F::from_bool(false),
                    pc: F::from_canonical_u64(next_pc + 1),
                    // reset: F::from_bool(false),
                    sel_down_to_up: F::from_bool(true),
                    ..Default::default()
                };

                for i in 0..CHUNKS {
                    first_read_row.reg[i] = F::from_canonical_u64(value_first_read[i]);
                    first_read_row.sel[i] = F::from_bool(true);

                    value_row.reg[i] = F::from_canonical_u64(value[shift + i]);
                    value_row.sel[i] = F::from_bool(i == offset);

                    second_read_row.reg[i] = F::from_canonical_u64(value_second_read[i]);
                    second_read_row.sel[i] = F::from_bool(true);

                    // Store the range check
                    *range_check.entry(first_read_row.reg[i]).or_insert(0) += 1;
                    *range_check.entry(value_row.reg[i]).or_insert(0) += 1;
                    *range_check.entry(second_read_row.reg[i]).or_insert(0) += 1;
                }

                // Store the rows
                rows.push(first_read_row);
                rows.push(value_row);
                rows.push(second_read_row);
            }
            3 => {
                // RWVWR
                let mut first_read_row = MemUnalign0Row::<F> {
                    step: F::from_canonical_u64(step_first_read),
                    addr: F::from_canonical_u64(addr_prior),
                    // offset: F::from_canonical_u64(0),
                    width: F::from_canonical_u64(CHUNKS),
                    // wr: F::from_bool(false),
                    // pc: F::from_canonical_u64(0),
                    reset: F::from_bool(true),
                    sel_up_to_down: F::from_bool(true),
                    ..Default::default()
                };

                let mut first_write_row = MemUnalign0Row::<F> {
                    step: F::from_canonical_u64(step_first_write),
                    addr: F::from_canonical_u64(addr_prior),
                    // offset: F::from_canonical_u64(0),
                    width: F::from_canonical_u64(CHUNKS),
                    wr: F::from_bool(true),
                    pc: F::from_canonical_u64(next_pc),
                    // reset: F::from_bool(false),
                    sel_up_to_down: F::from_bool(true),
                    ..Default::default()
                };

                let mut value_row = MemUnalign0Row::<F> {
                    step: F::from_canonical_u64(step),
                    addr: F::from_canonical_u64(addr),
                    offset: F::from_canonical_u64(offset),
                    width: F::from_canonical_u64(width),
                    // wr: F::from_bool(false),
                    pc: F::from_canonical_u64(next_pc + 1),
                    // reset: F::from_bool(false),
                    sel_prove: F::from_bool(true),
                    ..Default::default()
                };

                let mut second_write_row = MemUnalign0Row::<F> {
                    step: F::from_canonical_u64(step_second_write),
                    addr: F::from_canonical_u64(addr_next),
                    // offset: F::from_canonical_u64(0),
                    width: F::from_canonical_u64(CHUNKS),
                    wr: F::from_bool(true),
                    pc: F::from_canonical_u64(next_pc + 2),
                    // reset: F::from_bool(false),
                    sel_down_to_up: F::from_bool(true),
                    ..Default::default()
                };

                let mut second_read_row = MemUnalign0Row::<F> {
                    step: F::from_canonical_u64(step_second_read),
                    addr: F::from_canonical_u64(addr_next),
                    // offset: F::from_canonical_u64(0),
                    width: F::from_canonical_u64(CHUNKS),
                    // wr: F::from_bool(false),
                    pc: F::from_canonical_u64(next_pc + 3),
                    reset: F::from_bool(false),
                    sel_down_to_up: F::from_bool(true),
                    ..Default::default()
                };

                for i in 0..CHUNKS {
                    first_read_row.reg[i] = F::from_canonical_u64(value_first_read[i]);
                    first_read_row.sel[i] = F::from_bool(i < offset);

                    first_write_row.reg[i] = F::from_canonical_u64(value_first_write[i]);
                    first_write_row.sel[i] = F::from_bool(i >= offset);

                    value_row.reg[i] = F::from_canonical_u64(value[shift + i]);
                    value_row.sel[i] = F::from_bool(i == offset);

                    second_write_row.reg[i] = F::from_canonical_u64(value_second_write[i]);
                    second_write_row.sel[i] = F::from_bool(i < shift);

                    second_read_row.reg[i] = F::from_canonical_u64(value_second_read[i]);
                    second_read_row.sel[i] = F::from_bool(i >= shift);

                    // Store the range check
                    *range_check.entry(first_read_row.reg[i]).or_insert(0) += 1;
                    *range_check.entry(first_write_row.reg[i]).or_insert(0) += 1;
                    *range_check.entry(value_row.reg[i]).or_insert(0) += 1;
                    *range_check.entry(second_write_row.reg[i]).or_insert(0) += 1;
                    *range_check.entry(second_read_row.reg[i]).or_insert(0) += 1;
                }

                // Store the rows
                rows.push(first_read_row);
                rows.push(first_write_row);
                rows.push(value_row);
                rows.push(second_write_row);
                rows.push(second_read_row);
            }
            _ => panic!("MemUnalignedSM::process_slice() got invalid program={}", program),
        }

        // TBD
        // for (i, a_byte) in a_bytes.iter().enumerate() {
        //     let row = MemUnalignedRomSM::<F>::calculate_table_row(
        //         mem_unaligned_rom_op,
        //         i as u64,
        //         *a_byte as u64,
        //         in2_low,
        //     );
        //     multiplicity[row as usize] += 1;
        // }

        // Return successfully
        rows
    }

    pub fn prove_instance(
        &self,
        operations: Vec<ZiskRequiredMemory>,
        prover_buffer: &mut [F],
        offset: u64,
    ) {
        Self::prove_internal(
            &self.wcm,
            &self.mem_unaligned_rom_sm,
            &self.std,
            operations,
            prover_buffer,
            offset,
        );
    }

    fn prove_internal(
        wcm: &WitnessManager<F>,
        mem_unaligned_rom_sm: &MemUnalignedRomSM<F>,
        std: &Std<F>,
        operations: Vec<ZiskRequiredMemory>,
        prover_buffer: &mut [F],
        offset: u64,
    ) {
        let pctx = wcm.get_pctx();

        let air = pctx.pilout.get_air(MEM_UNALIGNED_AIRGROUP_ID, MEM_UNALIGNED_AIR_IDS[0]);
        let air_mem_unaligned_rom = pctx
            .pilout
            .get_air(MEM_UNALIGNED_ROM_AIRGROUP_ID, MEM_UNALIGNED_ROM_AIR_IDS[0]);
        assert!(operations.len() <= air.num_rows());

        info!(
            "{}: ··· Creating Binary extension instance [{} / {} rows filled {:.2}%]",
            Self::MY_NAME,
            operations.len(),
            air.num_rows(),
            operations.len() as f64 / air.num_rows() as f64 * 100.0
        );

        let mut multiplicity_table = vec![0u64; air_mem_unaligned_rom.num_rows()];
        let mut range_check: HashMap<u64, u64> = HashMap::new();
        let mut trace_buffer =
            BinaryExtension0Trace::<F>::map_buffer(prover_buffer, air.num_rows(), offset as usize)
                .unwrap();

        for (i, operation) in operations.iter().enumerate() {
            let row = Self::process_slice(operation, &mut multiplicity_table, &mut range_check);
            trace_buffer[i] = row;
        }

        let padding_row =
            BinaryExtension0Row::<F> { op: F::from_canonical_u64(0x25), ..Default::default() };

        for i in operations.len()..air.num_rows() {
            trace_buffer[i] = padding_row;
        }

        let padding_size = air.num_rows() - operations.len();
        for i in 0..8 {
            let multiplicity = padding_size as u64;
            let row = MemUnalignedRomSM::<F>::calculate_table_row(
                BinaryExtensionTableOp::SignExtendW,
                i,
                0,
                0,
            );
            multiplicity_table[row as usize] += multiplicity;
        }

        mem_unaligned_rom_sm.process_slice(&multiplicity_table);

        let range_id = std.get_range(BigInt::from(0), BigInt::from(0xFFFFFF), None);

        for (value, multiplicity) in &range_check {
            std.range_check(
                F::from_canonical_u64(*value),
                F::from_canonical_u64(*multiplicity),
                range_id,
            );
        }


        std::thread::spawn(move || {
            drop(operations);
            drop(multiplicity_table);
            drop(range_check);
        });
    }
}

impl<F> WitnessComponent<F> for MemUnalignedSM {
    fn calculate_witness(
        &self,
        _stage: u32,
        _air_instance: Option<usize>,
        _pctx: Arc<ProofCtx<F>>,
        _ectx: Arc<ExecutionCtx>,
        _sctx: Arc<SetupCtx>,
    ) {
    }
}

impl Provable<MemUnalignedOp, OpResult> for MemUnalignedSM {
    fn calculate(&self, operation: MemUnalignedOp) -> Result<OpResult, Box<dyn std::error::Error>> {
        // TODO: Perform the aligned read/writes

        match operation {
            MemUnalignedOp::Read(addr, width) => self.read(addr, width),
            MemUnalignedOp::Write(addr, width, val) => self.write(addr, width, val),
        }
    }

    fn prove(&self, operations: &[ZiskRequiredMemory], drain: bool, _scope: &Scope) {
        if let Ok(mut inputs) = self.inputs.lock() {
            inputs.extend_from_slice(operations);

            let pctx = self.wcm.get_pctx();
            let air =
                pctx.pilout.get_air(MEM_UNALIGNED_AIRGROUP_ID, MEM_UNALIGNED_AIR_IDS[0]);

            while inputs.len() >= air.num_rows() || (drain && !inputs.is_empty()) {
                let num_drained = std::cmp::min(air.num_rows(), inputs.len());
                let drained_inputs = inputs.drain(..num_drained).collect::<Vec<_>>();

                let mem_unaligned_rom_sm = self.mem_unaligned_rom_sm.clone();
                let wcm = self.wcm.clone();

                let std = self.std.clone();

                let sctx = self.wcm.get_sctx().clone();

                let (mut prover_buffer, offset) = create_prover_buffer(
                    &wcm.get_ectx(),
                    &wcm.get_sctx(),
                    MEM_UNALIGNED_AIRGROUP_ID,
                    MEM_UNALIGNED_AIR_IDS[0],
                );

                Self::prove_internal(
                    &wcm,
                    &mem_unaligned_rom_sm,
                    &std,
                    drained_inputs,
                    &mut prover_buffer,
                    offset,
                );

                let air_instance = AirInstance::new(
                    sctx,
                    MEM_UNALIGNED_AIRGROUP_ID,
                    MEM_UNALIGNED_AIR_IDS[0],
                    None,
                    prover_buffer,
                );
                wcm.get_pctx().air_instance_repo.add_air_instance(air_instance, None);
            }
        }
    }

    fn calculate_prove(
        &self,
        operation: MemUnalignedOp,
        drain: bool,
        scope: &Scope,
    ) -> Result<OpResult, Box<dyn std::error::Error>> {
        let result = self.calculate(operation.clone());

        self.prove(&[operation], drain, scope);

        result
    }
}