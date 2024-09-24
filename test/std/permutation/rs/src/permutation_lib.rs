use std::{error::Error, path::PathBuf, sync::Arc};

use pil_std_lib::Std;
use proofman::{WitnessLibrary, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx, SetupCtx, WitnessPilout};

use p3_field::PrimeField;
use p3_goldilocks::Goldilocks;
use rand::{distributions::Standard, prelude::Distribution};

use crate::{Permutation1_6, Permutation1_7, Permutation1_8, Permutation2, Pilout};

pub struct PermutationWitness<F: PrimeField> {
    pub wcm: WitnessManager<F>,
    pub permutation1_6: Arc<Permutation1_6<F>>,
    pub permutation1_7: Arc<Permutation1_7<F>>,
    pub permutation1_8: Arc<Permutation1_8<F>>,
    pub permutation2: Arc<Permutation2<F>>,
    pub std_lib: Arc<Std<F>>,
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
        let mut wcm = WitnessManager::new();

        let std_lib = Std::new(&mut wcm, None);
        let permutation1_6 = Permutation1_6::new(&mut wcm);
        let permutation1_7 = Permutation1_7::new(&mut wcm);
        let permutation1_8 = Permutation1_8::new(&mut wcm);
        let permutation2 = Permutation2::new(&mut wcm);

        PermutationWitness {
            wcm,
            permutation1_6,
            permutation1_7,
            permutation1_8,
            permutation2,
            std_lib,
        }
    }
}

impl<F: PrimeField> WitnessLibrary<F> for PermutationWitness<F>
where
    Standard: Distribution<F>,
{
    fn start_proof(&mut self, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx, sctx: &SetupCtx) {
        self.wcm.start_proof(pctx, ectx, sctx);
    }

    fn end_proof(&mut self) {
        self.wcm.end_proof();
    }

    fn execute(&self, pctx: &mut ProofCtx<F>, ectx: &mut ExecutionCtx, sctx: &SetupCtx) {
        // Execute those components that need to be executed
        self.permutation1_6.execute(pctx, ectx, sctx);
        self.permutation1_7.execute(pctx, ectx, sctx);
        self.permutation1_8.execute(pctx, ectx, sctx);
        self.permutation2.execute(pctx, ectx, sctx);
    }

    fn calculate_witness(
        &mut self,
        stage: u32,
        pctx: &mut ProofCtx<F>,
        ectx: &ExecutionCtx,
        sctx: &SetupCtx,
    ) {
        self.wcm.calculate_witness(stage, pctx, ectx, sctx);
    }

    fn pilout(&self) -> WitnessPilout {
        Pilout::pilout()
    }
}

#[no_mangle]
pub extern "Rust" fn init_library(
    _rom_path: Option<PathBuf>,
    _public_inputs_path: Option<PathBuf>,
) -> Result<Box<dyn WitnessLibrary<Goldilocks>>, Box<dyn Error>> {
    env_logger::builder()
        .format_timestamp(None)
        .format_level(true)
        .format_target(false)
        .filter_level(log::LevelFilter::Trace)
        .init();
    let permutation_witness = PermutationWitness::new();
    Ok(Box::new(permutation_witness))
}

#[cfg(test)]
mod tests {
    use proofman_cli::commands::verify_constraints::{Field, VerifyConstraintsCmd};

    #[test]
    fn test_verify_constraints() {
        let root_path = std::env::current_dir()
            .expect("Failed to get current directory")
            .join("../../../../");
        let root_path = std::fs::canonicalize(root_path).expect("Failed to canonicalize root path");

        let verify_constraints = VerifyConstraintsCmd {
            witness_lib: root_path.join("target/debug/libpermutation.so"),
            rom: None,
            public_inputs: None,
            proving_key: root_path.join("test/std/permutation/build/provingKey"),
            field: Field::Goldilocks,
            verbose: 0,
        };

        if let Err(e) = verify_constraints.run() {
            eprintln!("Failed to verify constraints: {:?}", e);
            std::process::exit(1);
        }
    }
}
