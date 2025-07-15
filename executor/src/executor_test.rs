use fields::PrimeField64;
use precomp_arith_eq::{ArithEqInput, ARITH_EQ_ROWS_BY_OP};
use precomp_keccakf::{KeccakfInput, NUM_KECCAKF_PER_CIRCUIT};
use precomp_sha256f::{Sha256fInput, NUM_SHA256F_PER_CIRCUIT};
use proofman_common::{create_pool, BufferPool, PreCalculate, ProofCtx, SetupCtx};
use sm_mem::{MemAlignInput, MemInput, MemInstanceInfo};
use witness::WitnessComponent;
use zisk_common::{BinaryAddInput, Input};
use zisk_core::ZiskRom;
use zisk_pil::{
    ArithEqTrace, ArithTrace, BinaryAddTrace, BinaryExtensionTrace, BinaryTrace, KeccakfTrace,
    MemAlignTrace, Sha256fTrace, ARITH_AIR_IDS, ARITH_EQ_AIR_IDS, BINARY_ADD_AIR_IDS,
    BINARY_AIR_IDS, BINARY_EXTENSION_AIR_IDS, INPUT_DATA_AIR_IDS, KECCAKF_AIR_IDS, MEM_AIR_IDS,
    MEM_ALIGN_AIR_IDS, ROM_DATA_AIR_IDS, SHA_256_F_AIR_IDS, ZISK_AIRGROUP_ID,
};

use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::{Arc, RwLock},
};

use rayon::prelude::*;

use crate::StaticSMBundle;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
enum InstanceInput {
    Binary(Input),
    BinaryAdd(BinaryAddInput),
    BinaryExtension(Input),
    Arith(Input),
    Mem(MemInput),
    RomData(MemInput),
    InputData(MemInput),
    MemAlign(MemAlignInput),
    Keccakf(KeccakfInput),
    Sha256f(Sha256fInput),
    ArithEq(ArithEqInput),
}

/// The `ZiskExecutor` struct orchestrates the execution of the ZisK ROM program, managing state
/// machines, planning, and witness computation.
pub struct ZiskExecutorTest<F: PrimeField64> {
    /// ZisK ROM, a binary file containing the ZisK program to be executed.
    _zisk_rom: Arc<Option<ZiskRom>>,
    inputs: RwLock<HashMap<usize, Vec<InstanceInput>>>,
    mem_instance_info: RwLock<HashMap<usize, MemInstanceInfo>>,
    sm_bundle: StaticSMBundle<F>,
}

impl<F: PrimeField64> ZiskExecutorTest<F> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(sm_bundle: StaticSMBundle<F>, zisk_rom: Arc<Option<ZiskRom>>) -> Self {
        Self {
            inputs: RwLock::new(HashMap::new()),
            mem_instance_info: RwLock::new(HashMap::new()),
            sm_bundle,
            _zisk_rom: zisk_rom,
        }
    }

    fn process_inputs(
        &self,
        pctx: &ProofCtx<F>,
        global_ids: &mut Vec<usize>,
        mut inputs: Vec<InstanceInput>,
        num_inputs_instance: usize,
        air_id: usize,
        pre_calculate: PreCalculate,
    ) {
        while !inputs.is_empty() {
            let chunk: Vec<InstanceInput> =
                inputs.drain(..num_inputs_instance.min(inputs.len())).collect();

            let global_id = pctx.add_instance(ZISK_AIRGROUP_ID, air_id, pre_calculate, 1);
            global_ids.push(global_id);

            {
                let mut map = self.inputs.write().unwrap();
                map.entry(global_id).or_default().extend(chunk);
            }
        }
    }
}

impl<F: PrimeField64> WitnessComponent<F> for ZiskExecutorTest<F> {
    fn execute(&self, pctx: Arc<ProofCtx<F>>, input_data_path: Option<PathBuf>) -> Vec<usize> {
        let json_text = fs::read_to_string(input_data_path.unwrap())
            .expect("failed to read input_data JSON file");

        let mut input_data: HashMap<String, Vec<InstanceInput>> = HashMap::new();

        let raw: HashMap<String, Vec<serde_json::Value>> =
            serde_json::from_str(&json_text).expect("JSON must be an object of arrays");

        for (name, arr) in raw {
            let mut instances = Vec::with_capacity(arr.len());
            for v in arr {
                let tagged = serde_json::json!({
                    "type": name,
                    "data": v
                });
                let inst: InstanceInput = serde_json::from_value(tagged)
                    .unwrap_or_else(|e| panic!("Bad `{name}` entry: {e}"));
                instances.push(inst);
            }

            input_data.entry(name).or_default().extend(instances);
        }

        let mut global_ids = Vec::new();
        for (instance_name, mut inputs) in input_data {
            match instance_name.as_str() {
                "Binary" => {
                    self.process_inputs(
                        &pctx,
                        &mut global_ids,
                        inputs,
                        BinaryTrace::<usize>::NUM_ROWS,
                        BINARY_AIR_IDS[0],
                        PreCalculate::Fast,
                    );
                }
                "BinaryAdd" => {
                    self.process_inputs(
                        &pctx,
                        &mut global_ids,
                        inputs,
                        BinaryAddTrace::<usize>::NUM_ROWS,
                        BINARY_ADD_AIR_IDS[0],
                        PreCalculate::Fast,
                    );
                }
                "BinaryExtension" => {
                    self.process_inputs(
                        &pctx,
                        &mut global_ids,
                        inputs,
                        BinaryExtensionTrace::<usize>::NUM_ROWS,
                        BINARY_EXTENSION_AIR_IDS[0],
                        PreCalculate::Fast,
                    );
                }
                "Arith" => {
                    self.process_inputs(
                        &pctx,
                        &mut global_ids,
                        inputs,
                        ArithTrace::<usize>::NUM_ROWS,
                        ARITH_AIR_IDS[0],
                        PreCalculate::Fast,
                    );
                }
                "Keccakf" => {
                    let circuit_size = self.sm_bundle.keccakf_sm.get_circuit_size();

                    let num_inputs_instance =
                        (KeccakfTrace::<usize>::NUM_ROWS / circuit_size) * NUM_KECCAKF_PER_CIRCUIT;
                    self.process_inputs(
                        &pctx,
                        &mut global_ids,
                        inputs,
                        num_inputs_instance,
                        KECCAKF_AIR_IDS[0],
                        PreCalculate::Fast,
                    );
                }
                "Sha256f" => {
                    let circuit_size = self.sm_bundle.sha256f_sm.get_circuit_size();
                    let num_inputs_instance = ((Sha256fTrace::<usize>::NUM_ROWS - 1)
                        / circuit_size)
                        * NUM_SHA256F_PER_CIRCUIT;
                    self.process_inputs(
                        &pctx,
                        &mut global_ids,
                        inputs,
                        num_inputs_instance,
                        SHA_256_F_AIR_IDS[0],
                        PreCalculate::Fast,
                    );
                }
                "ArithEq" => {
                    self.process_inputs(
                        &pctx,
                        &mut global_ids,
                        inputs,
                        ArithEqTrace::<usize>::NUM_ROWS / ARITH_EQ_ROWS_BY_OP,
                        ARITH_EQ_AIR_IDS[0],
                        PreCalculate::Fast,
                    );
                }
                "MemAlign" => {
                    let num_rows = MemAlignTrace::<usize>::NUM_ROWS;

                    let rows_per_input: Vec<usize> = inputs
                        .par_iter()
                        .map(|inp| {
                            let rows = match &inp {
                                InstanceInput::MemAlign(m) => {
                                    self.sm_bundle.mem_sm.get_rows_input_mem_align(m)
                                }
                                _ => panic!("Expected MemAlign"),
                            };
                            rows
                        })
                        .collect();

                    while !inputs.is_empty() {
                        let mut num_inputs = 0;
                        let mut rows_used = 0;
                        while rows_used < num_rows && num_inputs < inputs.len() {
                            if rows_per_input[num_inputs] + rows_used > num_rows {
                                break;
                            }
                            rows_used += rows_per_input[num_inputs];
                            num_inputs += 1;
                        }
                        let chunk: Vec<InstanceInput> = inputs.drain(..num_inputs).collect();

                        let global_id = pctx.add_instance(
                            ZISK_AIRGROUP_ID,
                            MEM_ALIGN_AIR_IDS[0],
                            PreCalculate::Fast,
                            1,
                        );
                        global_ids.push(global_id);
                        let mut map = self.inputs.write().unwrap();
                        map.entry(global_id).or_default().extend(chunk);
                    }
                }
                "Mem" => {
                    panic!("Mem state machine is not supported in test mode");
                }
                "InputData" => {
                    panic!("InputData state machine is not supported in test mode");
                }
                "RomData" => {
                    panic!("RomData state machine is not supported in test mode");
                }
                _ => {
                    panic!("{}", format!("Unsupported state machine: {}", instance_name.as_str()));
                }
            }
        }
        global_ids
    }

    /// Computes the witness for the main and secondary state machines.
    ///
    /// # Arguments
    /// * `stage` - The current stage id
    /// * `pctx` - Proof context.
    /// * `sctx` - Setup context.
    /// * `global_ids` - Global IDs of the instances to compute witness for.
    fn calculate_witness(
        &self,
        stage: u32,
        pctx: Arc<ProofCtx<F>>,
        _sctx: Arc<SetupCtx<F>>,
        global_ids: &[usize],
        n_cores: usize,
        _buffer_pool: &dyn BufferPool<F>,
    ) {
        if stage != 1 {
            return;
        }

        let pool = create_pool(n_cores);
        pool.install(|| {
            for &global_id in global_ids {
                let (_airgroup_id, air_id) = pctx.dctx_get_instance_info(global_id);

                let inputs: Vec<InstanceInput> = {
                    let mut map = self.inputs.write().unwrap();
                    map.remove(&global_id).unwrap()
                };
                let air_instance_id = if air_id == BINARY_AIR_IDS[0] {
                    let binary_inputs = inputs
                        .into_iter()
                        .map(|input| match input {
                            InstanceInput::Binary(input) => vec![input],
                            _ => panic!("Expected Binary input"),
                        })
                        .collect::<Vec<_>>();
                    self.sm_bundle.binary_sm.compute_witness_binary(&binary_inputs, None)
                } else if air_id == BINARY_ADD_AIR_IDS[0] {
                    let binary_add_inputs = inputs
                        .into_iter()
                        .map(|input| match input {
                            InstanceInput::BinaryAdd(input) => vec![input],
                            _ => panic!("Expected BinaryAdd input"),
                        })
                        .collect::<Vec<_>>();

                    self.sm_bundle.binary_sm.compute_witness_binary_add(&binary_add_inputs, None)
                } else if air_id == BINARY_EXTENSION_AIR_IDS[0] {
                    let binary_extension_inputs = inputs
                        .into_iter()
                        .map(|input| match input {
                            InstanceInput::BinaryExtension(input) => vec![input],
                            _ => panic!("Expected BinaryExtension input"),
                        })
                        .collect::<Vec<_>>();

                    self.sm_bundle
                        .binary_sm
                        .compute_witness_binary_extension(&binary_extension_inputs, None)
                } else if air_id == ARITH_AIR_IDS[0] {
                    let arith_inputs = inputs
                        .into_iter()
                        .map(|input| match input {
                            InstanceInput::Arith(input) => vec![input],
                            _ => panic!("Expected Arith input"),
                        })
                        .collect::<Vec<_>>();

                    self.sm_bundle.arith_sm.compute_witness_arith::<F>(&arith_inputs, None)
                } else if air_id == KECCAKF_AIR_IDS[0] {
                    let keccakf_inputs = inputs
                        .into_iter()
                        .map(|input| match input {
                            InstanceInput::Keccakf(input) => vec![input],
                            _ => panic!("Expected KeccakF input"),
                        })
                        .collect::<Vec<_>>();

                    self.sm_bundle.keccakf_sm.compute_witness_keccakf(&keccakf_inputs, None)
                } else if air_id == SHA_256_F_AIR_IDS[0] {
                    let sha256f_inputs = inputs
                        .into_iter()
                        .map(|input| match input {
                            InstanceInput::Sha256f(input) => vec![input],
                            _ => panic!("Expected Sha256f input"),
                        })
                        .collect::<Vec<_>>();

                    self.sm_bundle.sha256f_sm.compute_witness_sha256f(&sha256f_inputs, None)
                } else if air_id == ARITH_EQ_AIR_IDS[0] {
                    let arith_eq_inputs = inputs
                        .into_iter()
                        .map(|input| match input {
                            InstanceInput::ArithEq(input) => vec![input],
                            _ => panic!("Expected ArithEq input"),
                        })
                        .collect::<Vec<_>>();

                    self.sm_bundle.arith_eq_sm.compute_witness_arith_eq(&arith_eq_inputs, None)
                } else if air_id == MEM_ALIGN_AIR_IDS[0] {
                    let mem_align_inputs = inputs
                        .into_iter()
                        .map(|input| match input {
                            InstanceInput::MemAlign(input) => vec![input],
                            _ => panic!("Expected MemAlign input"),
                        })
                        .collect::<Vec<_>>();

                    self.sm_bundle.mem_sm.compute_witness_mem_align(&mem_align_inputs, None)
                } else {
                    panic!("Unsupported air id, {air_id}");
                };

                // Store the computed witness in the proof context.
                pctx.add_air_instance(air_instance_id, global_id);
            }
        });
    }
}
