use std::io::Read;
use std::{fs::File, sync::Arc};

use proofman_common::{ExecutionCtx, ProofCtx, WitnessPilout, SetupCtx};
use proofman::{WitnessLibrary, WitnessManager};
use pil_std_lib::Std;
use p3_field::PrimeField;
use p3_goldilocks::Goldilocks;

use std::error::Error;
use std::path::PathBuf;
use crate::FibonacciSquarePublics;

use crate::{FibonacciSquare, Pilout, Module};

pub struct FibonacciWitness<F> {
    pub wcm: WitnessManager<F>,
    pub public_inputs_path: PathBuf,
    pub fibonacci: Arc<FibonacciSquare<F>>,
    pub module: Arc<Module<F>>,
    pub std_lib: Arc<Std<F>>,
}

impl<F: PrimeField> FibonacciWitness<F> {
    pub fn new(public_inputs_path: PathBuf) -> Self {
        let mut wcm = WitnessManager::new();

        let std_lib = Std::new(&mut wcm, None);
        let module = Module::new(&mut wcm, std_lib.clone());
        let fibonacci = FibonacciSquare::new(&mut wcm, module.clone());

        FibonacciWitness { wcm, public_inputs_path, fibonacci, module, std_lib }
    }
}

impl<F: PrimeField> WitnessLibrary<F> for FibonacciWitness<F> {
    fn start_proof(&mut self, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx, sctx: &SetupCtx) {
        let mut file = File::open(&self.public_inputs_path).unwrap();

        if !file.metadata().unwrap().is_file() {
            panic!("Public inputs file not found");
        }

        let mut contents = String::new();
        let _ = file.read_to_string(&mut contents);

        let public_inputs: FibonacciSquarePublics = serde_json::from_str(&contents).unwrap();
        pctx.public_inputs = public_inputs.into();

        self.wcm.start_proof(pctx, ectx, sctx);
    }

    fn end_proof(&mut self) {
        self.wcm.end_proof();
    }

    fn execute(&self, pctx: &mut ProofCtx<F>, ectx: &mut ExecutionCtx, sctx: &SetupCtx) {
        self.fibonacci.execute(pctx, ectx, sctx);
    }

    fn calculate_witness(&mut self, stage: u32, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx, sctx: &SetupCtx) {
        self.wcm.calculate_witness(stage, pctx, ectx, sctx);
    }

    fn pilout(&self) -> WitnessPilout {
        Pilout::pilout()
    }
}

#[no_mangle]
pub extern "Rust" fn init_library(
    _rom_path: Option<PathBuf>,
    public_inputs_path: PathBuf,
) -> Result<Box<dyn WitnessLibrary<Goldilocks>>, Box<dyn Error>> {
    env_logger::builder()
        .format_timestamp(None)
        .format_level(true)
        .format_target(false)
        .filter_level(log::LevelFilter::Trace)
        .init();
    let fibonacci_witness = FibonacciWitness::new(public_inputs_path);
    Ok(Box::new(fibonacci_witness))
}
