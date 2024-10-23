use std::{mem::MaybeUninit, sync::RwLock};
use std::path::PathBuf;

use p3_field::Field;

use crate::{AirInstancesRepository, GlobalInfo, VerboseMode, WitnessPilout};

pub struct PublicInputs {
    pub inputs: RwLock<Vec<u8>>,
}

impl Default for PublicInputs {
    fn default() -> Self {
        Self { inputs: RwLock::new(Vec::new()) }
    }
}

pub struct Challenges<F> {
    pub challenges: RwLock<Vec<F>>,
}

impl<F> Default for Challenges<F> {
    fn default() -> Self {
        Self { challenges: RwLock::new(Vec::new()) }
    }
}

pub struct BuffHelper<F> {
    pub buff_helper: RwLock<Vec<MaybeUninit<F>>>,
}

impl<F> Default for BuffHelper<F> {
    fn default() -> Self {
        Self { buff_helper: RwLock::new(Vec::new()) }
    }
}

pub struct ProofOptions {
    pub verify_constraints: bool,
    pub verbose_mode: VerboseMode,
    pub aggregation: bool,
    pub verify_proof: bool,
}

impl ProofOptions {
    pub fn new(verify_constraints: bool, verbose_mode: VerboseMode, aggregation: bool, verify_proof: bool) -> Self {
        Self { verify_constraints, verbose_mode, aggregation, verify_proof }
    }
}

#[allow(dead_code)]
pub struct ProofCtx<F> {
    pub pilout: WitnessPilout,
    pub public_inputs: PublicInputs,
    pub challenges: Challenges<F>,
    pub buff_helper: BuffHelper<F>,
    pub global_info: GlobalInfo,
    pub air_instance_repo: AirInstancesRepository<F>,
}

impl<F: Field> ProofCtx<F> {
    const MY_NAME: &'static str = "ProofCtx";

    pub fn create_ctx(pilout: WitnessPilout, proving_key_path: PathBuf) -> Self {
        log::info!("{}: Creating proof context", Self::MY_NAME);

        let global_info: GlobalInfo = GlobalInfo::new(&proving_key_path);

        Self {
            pilout,
            global_info,
            public_inputs: PublicInputs::default(),
            challenges: Challenges::default(),
            buff_helper: BuffHelper::default(),
            air_instance_repo: AirInstancesRepository::new(),
        }
    }
}
