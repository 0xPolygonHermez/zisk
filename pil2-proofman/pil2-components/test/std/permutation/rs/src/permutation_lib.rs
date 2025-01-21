use std::sync::Arc;

use pil_std_lib::Std;
use witness::{witness_library, WitnessLibrary, WitnessManager};

use p3_field::PrimeField;
use p3_goldilocks::Goldilocks;
use rand::{distributions::Standard, prelude::Distribution};

use crate::{Permutation1_6, Permutation1_7, Permutation1_8, Permutation2};

witness_library!(WitnessLib, Goldilocks);

impl<F: PrimeField> WitnessLibrary<F> for WitnessLib
where
    Standard: Distribution<F>,
{
    fn register_witness(&mut self, wcm: Arc<WitnessManager<F>>) {
        Std::new(wcm.clone());
        wcm.register_component(Permutation1_6::new());
        wcm.register_component(Permutation1_7::new());
        wcm.register_component(Permutation1_8::new());
        wcm.register_component(Permutation2::new());
    }
}
