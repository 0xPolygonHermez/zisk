use std::io::Read;
use std::{fs::File, sync::Arc};

use proofman_common::{ExecutionCtx, ProofCtx, WitnessPilout, SetupCtx};
use proofman::{WitnessLibrary, WitnessManager};
use pil_std_lib::{RCAirData, RangeCheckAir, Std};
use p3_field::PrimeField;
use p3_goldilocks::Goldilocks;
use crate::pil_helpers::*;

use std::error::Error;
use std::path::PathBuf;
use crate::FibonacciSquarePublics;

use crate::{FibonacciSquare, Pilout, Module, U_8_AIR_AIRGROUP_ID, U_8_AIR_AIR_IDS};

pub struct FibonacciWitness<F: PrimeField> {
    public_inputs_path: Option<PathBuf>,
    wcm: Option<Arc<WitnessManager<F>>>,
    fibonacci: Option<Arc<FibonacciSquare<F>>>,
    module: Option<Arc<Module<F>>>,
    std_lib: Option<Arc<Std<F>>>,
}

impl<F: PrimeField> FibonacciWitness<F> {
    pub fn new(public_inputs_path: Option<PathBuf>) -> Self {
        Self { public_inputs_path, wcm: None, fibonacci: None, module: None, std_lib: None }
    }
}

impl<F: PrimeField> WitnessLibrary<F> for FibonacciWitness<F> {
    fn start_proof(&mut self, pctx: Arc<ProofCtx<F>>, ectx: Arc<ExecutionCtx>, sctx: Arc<SetupCtx>) {
        let wcm = Arc::new(WitnessManager::new(pctx.clone(), ectx.clone(), sctx.clone()));

        let rc_air_data = vec![RCAirData {
            air_name: RangeCheckAir::U8Air,
            airgroup_id: U_8_AIR_AIRGROUP_ID,
            air_id: U_8_AIR_AIR_IDS[0],
        }];

        let std_lib = Std::new(wcm.clone(), Some(rc_air_data));
        let module = Module::new(wcm.clone(), std_lib.clone());
        let fibonacci = FibonacciSquare::new(wcm.clone(), module.clone());

        self.wcm = Some(wcm.clone());
        self.fibonacci = Some(fibonacci);
        self.module = Some(module);
        self.std_lib = Some(std_lib);

        let public_inputs: FibonacciSquarePublics = if let Some(path) = &self.public_inputs_path {
            let mut file = File::open(path).unwrap();

            if !file.metadata().unwrap().is_file() {
                panic!("Public inputs file not found");
            }

            let mut contents = String::new();

            let _ =
                file.read_to_string(&mut contents).map_err(|err| format!("Failed to read public inputs file: {}", err));

            serde_json::from_str(&contents).unwrap()
        } else {
            FibonacciSquarePublics::default()
        };

        let pi: Vec<u8> = public_inputs.into();
        *pctx.public_inputs.inputs.write().unwrap() = pi;

        wcm.start_proof(pctx, ectx, sctx);
    }

    fn end_proof(&mut self) {
        self.wcm.as_ref().unwrap().end_proof();
    }

    fn execute(&self, pctx: Arc<ProofCtx<F>>, ectx: Arc<ExecutionCtx>, sctx: Arc<SetupCtx>) {
        self.fibonacci.as_ref().unwrap().execute(pctx, ectx, sctx);
        self.module.as_ref().unwrap().execute(pctx, ectx, sctx);
    }

    fn calculate_witness(&mut self, stage: u32, pctx: Arc<ProofCtx<F>>, ectx: Arc<ExecutionCtx>, sctx: Arc<SetupCtx>) {
        self.wcm.as_ref().unwrap().calculate_witness(stage, pctx, ectx, sctx);
    }

    fn pilout(&self) -> WitnessPilout {
        Pilout::pilout()
    }
}

#[no_mangle]
pub extern "Rust" fn init_library(
    _rom_path: Option<PathBuf>,
    public_inputs_path: Option<PathBuf>,
) -> Result<Box<dyn WitnessLibrary<Goldilocks>>, Box<dyn Error>> {
    env_logger::builder()
        .format_timestamp(None)
        .format_level(true)
        .format_target(false)
        .filter_level(log::LevelFilter::Debug)
        .init();
    let fibonacci_witness = FibonacciWitness::new(public_inputs_path);
    Ok(Box::new(fibonacci_witness))
}
