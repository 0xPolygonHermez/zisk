use std::{error::Error, path::PathBuf, sync::Arc};

use pil_std_lib::Std;
use proofman::{WitnessLibrary, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx, SetupCtx, WitnessPilout};

use p3_field::PrimeField;
use p3_goldilocks::Goldilocks;
use rand::{distributions::Standard, prelude::Distribution};

use crate::{Connection1, Connection2, ConnectionNew, Pilout};

pub struct ConnectionWitness<F: PrimeField> {
    pub wcm: Option<Arc<WitnessManager<F>>>,
    pub connection1: Option<Arc<Connection1<F>>>,
    pub connection2: Option<Arc<Connection2<F>>>,
    pub connection_new: Option<Arc<ConnectionNew<F>>>,
    pub std_lib: Option<Arc<Std<F>>>,
}

impl<F: PrimeField> Default for ConnectionWitness<F>
where
    Standard: Distribution<F>,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<F: PrimeField> ConnectionWitness<F>
where
    Standard: Distribution<F>,
{
    pub fn new() -> Self {
        ConnectionWitness { wcm: None, connection1: None, connection2: None, connection_new: None, std_lib: None }
    }

    pub fn initialize(&mut self, pctx: Arc<ProofCtx<F>>, ectx: Arc<ExecutionCtx>, sctx: Arc<SetupCtx>) {
        let wcm = Arc::new(WitnessManager::new(pctx, ectx, sctx));

        let std_lib = Std::new(wcm.clone(), None);
        let connection1 = Connection1::new(wcm.clone());
        let connection2 = Connection2::new(wcm.clone());
        let connection_new = ConnectionNew::new(wcm.clone());

        self.wcm = Some(wcm);
        self.connection1 = Some(connection1);
        self.connection2 = Some(connection2);
        self.connection_new = Some(connection_new);
        self.std_lib = Some(std_lib);
    }
}

impl<F: PrimeField> WitnessLibrary<F> for ConnectionWitness<F>
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
        self.connection1.as_ref().unwrap().execute(pctx.clone(), ectx.clone(), sctx.clone());
        self.connection2.as_ref().unwrap().execute(pctx.clone(), ectx.clone(), sctx.clone());
        self.connection_new.as_ref().unwrap().execute(pctx, ectx, sctx);
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
    _public_inputs_path: Option<PathBuf>,
) -> Result<Box<dyn WitnessLibrary<Goldilocks>>, Box<dyn Error>> {
    env_logger::builder()
        .format_timestamp(None)
        .format_level(true)
        .format_target(false)
        .filter_level(log::LevelFilter::Trace)
        .init();
    let connection_witness = ConnectionWitness::new();
    Ok(Box::new(connection_witness))
}

#[cfg(test)]
mod tests {
    use proofman_cli::commands::verify_constraints::{Field, VerifyConstraintsCmd};

    #[test]
    fn test_verify_constraints() {
        let root_path = std::env::current_dir().expect("Failed to get current directory").join("../../../../");
        let root_path = std::fs::canonicalize(root_path).expect("Failed to canonicalize root path");

        let verify_constraints = VerifyConstraintsCmd {
            witness_lib: root_path.join("target/debug/libconnection.so"),
            rom: None,
            public_inputs: None,
            proving_key: root_path.join("test/std/connection/build/provingKey"),
            field: Field::Goldilocks,
            verbose: 0,
        };

        if let Err(e) = verify_constraints.run() {
            eprintln!("Failed to verify constraints: {:?}", e);
            std::process::exit(1);
        }
    }
}
