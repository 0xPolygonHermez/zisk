//! State machines of the `Poseidon2NonNative` airs (precompiles/poseidon/pil/poseidon2_non_native.pil),
//! one per instance. `poseidon2_non_native_sm!` generates the input type, the SM and its memory
//! inputs from the instance's trace, operation data and `Poseidon2Params`; the row values come
//! from `Poseidon2Params::trace_rows`.
//!
//! # Adding an instance
//!
//! The permutation, the air and the witness are generic; an instance is its constants plus the
//! registrations every precompile needs. Taking `Poseidon2KoalaBear` as the model:
//!
//! - Constants: `precompiles/helpers/src/poseidon2/<instance>.rs` (+ test vectors from the
//!   reference implementation, exported in `mod.rs` and `lib.rs`),
//!   `precompiles/poseidon/pil/poseidon2_<instance>.pil` and a constants block + entry point in
//!   `lib-c/c/src/poseidon2/poseidon2_non_native.{cpp,hpp}`.
//! - Air: `Poseidon2NonNative(...)` alias in `pil/zisk.pil`, `OP_*` in `pil/operations.pil`, cost
//!   in `pil/src/air_costs.rs`, compressor in `setup/starkstructs.poseidon.json`, then regenerate
//!   `pil/src/pil_helpers/traces.rs`.
//! - State machine: a `poseidon2_non_native_sm!(Name, PARAMS)` invocation below, a `zisk_precompile!` in
//!   `lib.rs` and a line in `executor/src/sm/precompiles.rs`.
//! - Operation: `ZiskOperationType` (`core/src/zisk_inst.rs`); `OpType`, its three conversions,
//!   the opcode entry and a one-line `opc_` over `opc_poseidon2_non_native` (`core/src/zisk_ops.rs`);
//!   cost (`core/src/zisk_ops_costs.rs`); the Poseidon arm of `core/src/zisk_rom_2_asm.rs`; bus
//!   data (`common/src/bus/data_bus_operation.rs`); `emulator/src/stats/stats.rs`.
//! - Guest: syscall id (`definitions/src/syscall.rs`), `ziskos/entrypoint/src/syscalls/`, the
//!   transpiler (`transpilers/riscv/src/riscv2zisk_context.rs`) and the ASM handler
//!   (`emulator-asm/src/emu.{c,hpp}`, one line over `opcode_poseidon2_non_native`).

/// Generates `<$name>Input`, `<$name>SM` and its `PrecompileMemInputs` impl for one instance.
macro_rules! poseidon2_non_native_sm {
    ($name:ident, $params:path) => {
        paste::paste! {
            /// Words of the state, two lanes per word.
            const [<$name:snake:upper _WORDS>]: usize = $params.width() / 2;

            /// Per-operation input record assembled from the bus payload.
            #[derive(Debug)]
            pub struct [<$name Input>] {
                pub step_main: u64,
                /// Main operand b: the state address (the result overwrites it).
                pub addr_main: u32,
                pub state: [u64; [<$name:snake:upper _WORDS>]],
            }

            impl [<$name Input>] {
                pub fn from(values: &zisk_common::[<Operation $name Data>]<u64>) -> Self {
                    // op,op_type,a,b,step,state
                    Self {
                        step_main: values[4],
                        addr_main: values[3] as u32,
                        state: values[zisk_common::OPERATION_PRECOMPILED_BUS_DATA_SIZE..]
                            .try_into()
                            .unwrap(),
                    }
                }
            }

            pub struct [<$name SM>]<F: proofman_fields::PrimeField64> {
                _phantom: std::marker::PhantomData<F>,
            }

            impl<F: proofman_fields::PrimeField64> [<$name SM>]<F> {
                pub fn new() -> std::sync::Arc<Self> {
                    std::sync::Arc::new(Self { _phantom: std::marker::PhantomData })
                }

                /// Fills the CLOCKS rows of one permutation block.
                fn fill_block<R: zisk_pil::[<$name TraceRowOps>]<F>>(
                    trace: &mut [R],
                    state: &[u64],
                    active: Option<(u64, u32)>,
                ) {
                    let rows = $params.trace_rows(&zisk_precomp_helpers::Poseidon2Params::lanes(state));
                    let num_steps = $params.sbox_steps().len();
                    // Each value is two 16-bit limbs
                    let limbs = |v: u32| [(0, v as u16), (1, (v >> 16) as u16)];
                    for (t, row) in trace.iter_mut().zip(rows.iter()) {
                        for i in 0..$params.width() {
                            for (j, l) in limbs(row.chunks[i]) {
                                t.set_chunks(i, j, l);
                            }
                            for (j, l) in limbs(row.linear_q[i]) {
                                t.set_linear_q(i, j, l);
                            }
                            for k in 0..num_steps {
                                for (j, l) in limbs(row.sbox[i][k]) {
                                    t.set_sbox(i, k, j, l);
                                }
                                for (j, l) in limbs(row.sbox_q[i][k]) {
                                    t.set_sbox_q(i, k, j, l);
                                }
                            }
                        }
                    }
                    if let Some((step_main, addr_main)) = active {
                        trace[0].set_step_addr(step_main);
                        trace[1].set_step_addr(addr_main as u64);
                        for t in trace.iter_mut() {
                            t.set_in_use(true);
                        }
                    }
                }

                pub fn compute_witness<
                    R: zisk_pil::[<$name TraceRowOps>]<F>,
                    const NUM_ROWS: usize,
                    const AIR_ID: usize,
                >(
                    &self,
                    _sctx: &proofman_common::SetupCtx<F>,
                    inputs: &[Vec<[<$name Input>]>],
                    trace_buffer: Vec<F>,
                ) -> proofman_common::ProofmanResult<proofman_common::AirInstance<F>> {
                    use rayon::prelude::*;

                    let mut trace = proofman_common::GenericTrace::<
                        R,
                        NUM_ROWS,
                        { zisk_pil::ZISK_AIRGROUP_ID },
                        AIR_ID,
                    >::new_from_vec_zeroes(trace_buffer)?;
                    let clocks = $params.clocks();
                    let num_available = $params.num_available(NUM_ROWS);
                    let num_inputs = inputs.iter().map(|v| v.len()).sum::<usize>();
                    assert!(
                        num_inputs <= num_available,
                        "Exceeded available {} inputs: requested {num_inputs}, but only {num_available} are available.",
                        stringify!($name)
                    );
                    tracing::debug!(
                        "··· Creating {} instance [{} / {} rows filled {:.2}%]",
                        stringify!($name),
                        num_inputs * clocks,
                        NUM_ROWS,
                        (num_inputs * clocks) as f64 / NUM_ROWS as f64 * 100.0
                    );

                    // Whole blocks only: the rows past num_available * clocks have no clock active
                    // and stay zero, so `padding` is a whole number of blocks too
                    let (used, padding) =
                        trace.buffer[..num_available * clocks].split_at_mut(num_inputs * clocks);
                    let ops: Vec<_> = inputs.iter().flatten().collect();
                    used.par_chunks_mut(clocks).zip(ops.par_iter()).for_each(|(block, input)| {
                        Self::fill_block(block, &input.state, Some((input.step_main, input.addr_main)));
                    });

                    // Padding blocks prove the permutation of the zero state, without using the bus
                    if !padding.is_empty() {
                        let (first, rest) = padding.split_at_mut(clocks);
                        Self::fill_block(first, &[0; [<$name:snake:upper _WORDS>]], None);
                        rest.par_chunks_mut(clocks).for_each(|chunk| chunk.copy_from_slice(first));
                    }

                    Ok(proofman_common::AirInstance::new_from_trace(
                        proofman_common::FromTrace::new(&mut trace),
                    ))
                }
            }

            impl<F: proofman_fields::PrimeField64> zisk_precomp_common::PrecompileMemInputs
                for [<$name SM>]<F>
            {
                fn generate<P: zisk_precomp_common::MemProcessor>(
                    addr_main: u32,
                    step_main: u64,
                    data: &[u64],
                    only_counters: bool,
                    mem_processors: &mut P,
                ) {
                    use zisk_precomp_common::MemBusHelpers;
                    const START: usize = zisk_common::OPERATION_PRECOMPILED_BUS_DATA_SIZE;
                    let input = &data[START..START + [<$name:snake:upper _WORDS>]];
                    MemBusHelpers::mem_aligned_read_from_slice(addr_main, step_main, input, mem_processors);
                    let mut output = [0u64; [<$name:snake:upper _WORDS>]];
                    if !only_counters {
                        output.copy_from_slice(input);
                        $params.permute_words(&mut output).expect("non-canonical state on the bus");
                    }
                    MemBusHelpers::mem_aligned_write_from_slice(addr_main, step_main, &output, mem_processors);
                }

                fn should_skip<P: zisk_precomp_common::MemProcessor>(
                    addr_main: u32,
                    _data: &[u64],
                    mem_processors: &mut P,
                ) -> bool {
                    (0..[<$name:snake:upper _WORDS>] as u32).all(|i| mem_processors.skip_addr(addr_main + 8 * i))
                }
            }
        }
    };
}

poseidon2_non_native_sm!(Poseidon2KoalaBear, zisk_precomp_helpers::POSEIDON2_KOALABEAR);
