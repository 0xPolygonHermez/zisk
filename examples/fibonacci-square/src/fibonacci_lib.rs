use std::io::Read;
use std::{fs::File, sync::Arc};

use proofman_common::{initialize_logger, ExecutionCtx, ProofCtx, SetupCtx, VerboseMode, WitnessPilout};
use proofman::{WitnessLibrary, WitnessManager};
use pil_std_lib::Std;
use p3_field::PrimeField;
use p3_goldilocks::Goldilocks;

use std::error::Error;
use std::path::PathBuf;
use crate::FibonacciSquarePublics;

use crate::{FibonacciSquare, Pilout, Module};

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
    fn start_proof(&mut self, pctx: Arc<ProofCtx<F>>, ectx: Arc<ExecutionCtx<F>>, sctx: Arc<SetupCtx<F>>) {
        let wcm = Arc::new(WitnessManager::new(pctx.clone(), ectx.clone(), sctx.clone()));

        let std_lib = Std::new(wcm.clone());
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

        pctx.set_public_value_by_name(public_inputs.module, "mod", None);
        pctx.set_public_value_by_name(public_inputs.a, "in1", None);
        pctx.set_public_value_by_name(public_inputs.b, "in2", None);

        wcm.start_proof(pctx, ectx, sctx);
    }

    fn end_proof(&mut self) {
        self.wcm.as_ref().unwrap().end_proof();
    }

    fn execute(&self, pctx: Arc<ProofCtx<F>>, ectx: Arc<ExecutionCtx<F>>, sctx: Arc<SetupCtx<F>>) {
        self.fibonacci.as_ref().unwrap().execute(pctx.clone(), ectx.clone(), sctx.clone());
        self.module.as_ref().unwrap().execute(pctx, ectx, sctx);
    }

    fn calculate_witness(
        &mut self,
        stage: u32,
        pctx: Arc<ProofCtx<F>>,
        ectx: Arc<ExecutionCtx<F>>,
        sctx: Arc<SetupCtx<F>>,
    ) {
        self.wcm.as_ref().unwrap().calculate_witness(stage, pctx, ectx, sctx);
    }

    fn pilout(&self) -> WitnessPilout {
        Pilout::pilout()
    }
}

#[no_mangle]
pub extern "Rust" fn init_library(
    _: Option<PathBuf>,
    public_inputs_path: Option<PathBuf>,
    verbose_mode: VerboseMode,
) -> Result<Box<dyn WitnessLibrary<Goldilocks>>, Box<dyn Error>> {
    initialize_logger(verbose_mode);

    let fibonacci_witness = FibonacciWitness::new(public_inputs_path);
    Ok(Box::new(fibonacci_witness))
}
