use std::collections::HashMap;
use std::{mem::MaybeUninit, sync::RwLock};
use std::path::PathBuf;

use p3_field::Field;

use crate::{AirInstancesRepository, GlobalInfo, VerboseMode, WitnessPilout};

pub struct PublicInputs {
    pub inputs: RwLock<Vec<u8>>,
    pub inputs_set: RwLock<Vec<bool>>,
}

impl Default for PublicInputs {
    fn default() -> Self {
        Self { inputs: RwLock::new(Vec::new()), inputs_set: RwLock::new(Vec::new()) }
    }
}

impl PublicInputs {
    pub fn new(n_publics: usize) -> Self {
        Self {
            inputs: RwLock::new(vec![0; n_publics * std::mem::size_of::<u64>()]),
            inputs_set: RwLock::new(vec![false; n_publics]),
        }
    }
}

pub struct ProofValues<F> {
    pub values: RwLock<Vec<F>>,
    pub values_set: RwLock<HashMap<usize, bool>>,
}

impl<F> Default for ProofValues<F> {
    fn default() -> Self {
        Self { values: RwLock::new(Vec::new()), values_set: RwLock::new(HashMap::new()) }
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
    pub proof_values: ProofValues<F>,
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

        let proof_values = ProofValues {
            values: RwLock::new(vec![F::zero(); global_info.n_proof_values * 3]),
            values_set: RwLock::new(HashMap::new()),
        };
        let n_publics = global_info.n_publics;
        Self {
            pilout,
            global_info,
            public_inputs: PublicInputs::new(n_publics),
            proof_values,
            challenges: Challenges::default(),
            buff_helper: BuffHelper::default(),
            air_instance_repo: AirInstancesRepository::new(),
        }
    }

    pub fn set_proof_value(&self, name: &str, value: F) {
        let id = (0..self.global_info.n_proof_values)
            .find(|&i| {
                if let Some(proof_value) = self
                    .global_info
                    .proof_values_map
                    .as_ref()
                    .expect("global_info.proof_values_map is not initialized")
                    .get(i)
                {
                    proof_value.name == name
                } else {
                    false
                }
            })
            .unwrap_or_else(|| panic!("No proof value found with name {}", name));

        self.proof_values.values.write().unwrap()[3 * id] = value;
        self.proof_values.values.write().unwrap()[3 * id + 1] = F::zero();
        self.proof_values.values.write().unwrap()[3 * id + 2] = F::zero();

        self.set_proof_value_calculated(id);
    }

    pub fn set_proof_value_ext(&self, name: &str, value: Vec<F>) {
        let id = (0..self.global_info.n_proof_values)
            .find(|&i| {
                if let Some(proof_value) = self
                    .global_info
                    .proof_values_map
                    .as_ref()
                    .expect("global_info.proof_values_map is not initialized")
                    .get(i)
                {
                    proof_value.name == name
                } else {
                    false
                }
            })
            .unwrap_or_else(|| panic!("No proof value found with name {}", name));

        self.proof_values.values.write().unwrap()[3 * id] = value[0];
        self.proof_values.values.write().unwrap()[3 * id + 1] = value[1];
        self.proof_values.values.write().unwrap()[3 * id + 2] = value[2];

        self.set_proof_value_calculated(id);
    }

    pub fn set_proof_value_calculated(&self, id: usize) {
        self.proof_values.values_set.write().unwrap().insert(id, true);
    }

    pub fn set_public_value(&self, value: u64, public_id: u64) {
        self.public_inputs.inputs.write().unwrap()[(public_id as usize) * 8..(public_id as usize + 1) * 8]
            .copy_from_slice(&value.to_le_bytes());

        self.public_inputs.inputs_set.write().unwrap()[public_id as usize] = true;
    }

    pub fn set_public_value_by_name(&self, value: u64, public_name: &str, lengths: Option<Vec<u64>>) {
        let n_publics: usize = self.global_info.publics_map.as_ref().expect("REASON").len();
        let public_id = (0..n_publics)
            .find(|&i| {
                let public = self.global_info.publics_map.as_ref().expect("REASON").get(i).unwrap();

                // Check if name matches
                let name_matches = public.name == public_name;

                // If lengths is provided, check that it matches public.lengths
                let lengths_match = if let Some(ref provided_lengths) = lengths {
                    Some(&public.lengths) == Some(provided_lengths)
                } else {
                    true // If lengths is None, skip the lengths check
                };

                name_matches && lengths_match
            })
            .unwrap_or_else(|| {
                panic!("Name {} with specified lengths {:?} not found in publics_map", public_name, lengths)
            });

        self.set_public_value(value, public_id as u64);
    }

    pub fn get_public_value(&self, public_name: &str) -> u64 {
        let n_publics: usize = self.global_info.publics_map.as_ref().expect("REASON").len();
        let public_id = (0..n_publics)
            .find(|&i| {
                let public = self.global_info.publics_map.as_ref().expect("REASON").get(i).unwrap();
                public.name == public_name
            })
            .unwrap_or_else(|| panic!("Name {} not found in publics_map", public_name));

        u64::from_le_bytes(
            self.public_inputs.inputs.read().unwrap()[public_id * 8..(public_id + 1) * 8]
                .try_into()
                .expect("Expected 8 bytes for u64"),
        )
    }
}
