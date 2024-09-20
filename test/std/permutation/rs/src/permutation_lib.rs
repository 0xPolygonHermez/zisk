use std::{error::Error, path::PathBuf, sync::Arc};

use pil_std_lib::Std;
use proofman::{WitnessLibrary, WitnessManager};
use proofman_cli::commands::pil_helpers::PilHelpersCmd;
use proofman_common::{ExecutionCtx, ProofCtx, SetupCtx, WitnessPilout};

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

    pub fn initialize(
        &mut self,
        pctx: Arc<ProofCtx<F>>,
        ectx: Arc<ExecutionCtx>,
        sctx: Arc<SetupCtx>,
    ) {
        let mut wcm = Arc::new(WitnessManager::new(pctx, ectx, sctx));

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
    fn start_proof(
        &mut self,
        pctx: Arc<ProofCtx<F>>,
        ectx: Arc<ExecutionCtx>,
        sctx: Arc<SetupCtx>,
    ) {
        self.initialize(pctx.clone(), ectx.clone(), sctx.clone());

        self.wcm.as_ref().unwrap().start_proof(pctx, ectx, sctx);
    }

    fn end_proof(&mut self) {
        self.wcm.as_ref().unwrap().end_proof();
    }

    fn execute(&self, pctx: Arc<ProofCtx<F>>, ectx: Arc<ExecutionCtx>, sctx: Arc<SetupCtx>) {
        // Execute those components that need to be executed
        self.permutation1_6
            .as_ref()
            .unwrap()
            .execute(pctx.clone(), ectx.clone(), sctx.clone());
        self.permutation1_7
            .as_ref()
            .unwrap()
            .execute(pctx.clone(), ectx.clone(), sctx.clone());
        self.permutation1_8
            .as_ref()
            .unwrap()
            .execute(pctx.clone(), ectx.clone(), sctx.clone());
        self.permutation2
            .as_ref()
            .unwrap()
            .execute(pctx, ectx, sctx);
    }

    fn calculate_witness(
        &mut self,
        stage: u32,
        pctx: Arc<ProofCtx<F>>,
        ectx: Arc<ExecutionCtx>,
        sctx: Arc<SetupCtx>,
    ) {
        self.wcm
            .as_ref()
            .unwrap()
            .calculate_witness(stage, pctx, ectx, sctx);
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

mod tests {
    use proofman_cli::commands::pil_helpers::PilHelpersCmd;

    #[test]
    fn test_multiple_bash_commands() {
        // TODO: Do it without commands.
        //  TODO: Make it path independent.

        let root_path = std::env::current_dir().unwrap().join("../../../../");
        let root_path = std::fs::canonicalize(root_path).unwrap();

        let build_dir = root_path.join("test/std/permutation/build/");
        if !build_dir.exists() {
            std::fs::create_dir_all(build_dir).unwrap();
        }

        // Compile the pil file
        let _compilation = std::process::Command::new("node")
            .arg(root_path.join("../pil2-compiler/src/pil.js"))
            .arg("-I")
            .arg(root_path.join("lib/std/pil"))
            .arg(root_path.join("test/std/permutation/permutation.pil"))
            .arg("-o")
            .arg(root_path.join("test/std/permutation/build/permutation.pilout"))
            .status()
            .expect("Failed to execute command");

        let proofman_dir = root_path.join("../pil2-proofman");

        let pil_helpers = PilHelpersCmd {
            pilout: root_path.join("test/std/permutation/build/permutation.pilout"),
            path: root_path.join("test/std/permutation/rs/src"),
            overide: true,
        };

        pil_helpers.run().expect("Failed to generate pil_helpers");

        // let status = std::process::Command::new("cargo")
        //     .arg("run")
        //     .arg("--bin")
        //     .arg("proofman-cli")
        //     .arg("pil-helpers")
        //     .arg("--pilout")
        //     .arg(root_path.join("test/std/permutation/build/permutation.pilout"))
        //     .arg("--path")
        //     .arg(root_path.join("test/std/permutation/rs/src"))
        //     .arg("-o")
        //     .current_dir(proofman_dir)
        //     .status()
        //     .expect("Failed to execute command");

        // if status.success() {
        //     println!("Command executed successfully");
        // } else {
        //     println!("Command failed");
        // }
    }
}
