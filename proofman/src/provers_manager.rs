use log::debug;
use crate::prover::Prover;

// PROVERS MANAGER
// ================================================================================================
pub struct ProversManager {
    prover: Box<dyn Prover>,
}

impl ProversManager {
    const MY_NAME: &'static str = "proversm";

    pub fn new(prover: Box<dyn Prover>) -> Self {
        debug!("{}> Initializing...", Self::MY_NAME);

        Self {
            prover
        }
    }
}