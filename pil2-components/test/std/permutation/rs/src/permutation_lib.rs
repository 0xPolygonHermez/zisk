use std::{error::Error, path::PathBuf, sync::Arc};

use pil_std_lib::Std;
use proofman::{WitnessLibrary, WitnessManager};
use proofman_common::{initialize_logger, VerboseMode, ExecutionCtx, ProofCtx, SetupCtx, WitnessPilout};

use p3_field::PrimeField;
use p3_goldilocks::Goldilocks;
use rand::{distributions::Standard, prelude::Distribution};

use crate::{Permutation1_6, Permutation1_7, Permutation1_8, Permutation2, Pilout};

pub struct PermutationWitness<F: PrimeField> {
    pub wcm: Option<Arc<WitnessManager<F>>>,
    pub permutation1_6: Option<Arc<Permutation1_6<F>>>,
    pub permutation1_7: Option<Arc<Permutation1_7<F>>>,
    pub permutation1_8: Option<Arc<Permutation1_8<F>>>,
    pub permutation2: Option<Arc<Permutation2<F>>>,
    pub std_lib: Option<Arc<Std<F>>>,
}

impl<F: PrimeField> Default for PermutationWitness<F>
where
    Standard: Distribution<F>,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<F: PrimeField> PermutationWitness<F>
where
    Standard: Distribution<F>,
{
    pub fn new() -> Self {
        PermutationWitness {
            wcm: None,
            permutation1_6: None,
            permutation1_7: None,
            permutation1_8: None,
            permutation2: None,
            std_lib: None,
        }
    }

    pub fn initialize(&mut self, pctx: Arc<ProofCtx<F>>, ectx: Arc<ExecutionCtx>, sctx: Arc<SetupCtx>) {
        let wcm = Arc::new(WitnessManager::new(pctx, ectx, sctx));

        let std_lib = Std::new(wcm.clone(), None);
        let permutation1_6 = Permutation1_6::new(wcm.clone());
        let permutation1_7 = Permutation1_7::new(wcm.clone());
        let permutation1_8 = Permutation1_8::new(wcm.clone());
        let permutation2 = Permutation2::new(wcm.clone());

        self.wcm = Some(wcm);
        self.permutation1_6 = Some(permutation1_6);
        self.permutation1_7 = Some(permutation1_7);
        self.permutation1_8 = Some(permutation1_8);
        self.permutation2 = Some(permutation2);
        self.std_lib = Some(std_lib);
    }
}

impl<F: PrimeField> WitnessLibrary<F> for PermutationWitness<F>
where
    Standard: Distribution<F>,
{
    fn start_proof(&mut self, pctx: Arc<ProofCtx<F>>, ectx: Arc<ExecutionCtx>, sctx: Arc<SetupCtx>) {
        self.initialize(pctx.clone(), ectx.clone(), sctx.clone());

        self.wcm.as_ref().unwrap().start_proof(pctx, ectx, sctx);
    }

    fn end_proof(&mut self) {
        self.wcm.as_ref().unwrap().end_proof();
    }

    fn execute(&self, pctx: Arc<ProofCtx<F>>, ectx: Arc<ExecutionCtx>, sctx: Arc<SetupCtx>) {
        // Execute those components that need to be executed
        self.permutation1_6.as_ref().unwrap().execute(pctx.clone(), ectx.clone(), sctx.clone());
        self.permutation1_7.as_ref().unwrap().execute(pctx.clone(), ectx.clone(), sctx.clone());
        self.permutation1_8.as_ref().unwrap().execute(pctx.clone(), ectx.clone(), sctx.clone());
        self.permutation2.as_ref().unwrap().execute(pctx, ectx, sctx);
    }

    fn calculate_witness(&mut self, stage: u32, pctx: Arc<ProofCtx<F>>, ectx: Arc<ExecutionCtx>, sctx: Arc<SetupCtx>) {
        self.wcm.as_ref().unwrap().calculate_witness(stage, pctx, ectx, sctx);
    }

    fn pilout(&self) -> WitnessPilout {
        Pilout::pilout()
    }
}

#[no_mangle]
pub extern "Rust" fn init_library(ectx: &ExecutionCtx) -> Result<Box<dyn WitnessLibrary<Goldilocks>>, Box<dyn Error>> {
    initialize_logger(VerboseMode::Trace);

    let permutation_witness = PermutationWitness::new();
    Ok(Box::new(permutation_witness))
}

// #[cfg(test)]
// mod tests {
//     use proofman_cli::commands::verify_constraints::{Field, VerifyConstraintsCmd};

//     #[test]
//     fn test_verify_constraints() {
//         let root_path = std::env::current_dir().expect("Failed to get current directory").join("../../../../");
//         let root_path = std::fs::canonicalize(root_path).expect("Failed to canonicalize root path");

//         let verify_constraints = VerifyConstraintsCmd {
//             witness_lib: root_path.join("target/debug/libpermutation.so"),
//             rom: None,
//             public_inputs: None,
//             proving_key: root_path.join("test/std/permutation/build/provingKey"),
//             field: Field::Goldilocks,
//             verbose: 0,
//         };

//         if let Err(e) = verify_constraints.run() {
//             eprintln!("Failed to verify constraints: {:?}", e);
//             std::process::exit(1);
//         }
//     }
// }
