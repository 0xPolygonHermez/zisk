use std::sync::Arc;
use pil_std_lib::Std;
use witness::{witness_library, WitnessLibrary, WitnessManager};

use p3_field::PrimeField64;
use p3_goldilocks::Goldilocks;
use rand::{distributions::Standard, prelude::Distribution};

use crate::{SimpleLeft, SimpleRight};

witness_library!(WitnessLib, Goldilocks);

impl<F: PrimeField64> WitnessLibrary<F> for WitnessLib
where
    Standard: Distribution<F>,
{
    fn register_witness(&mut self, wcm: Arc<WitnessManager<F>>) {
        Std::new(wcm.clone());
        let simple_left = SimpleLeft::new();
        let simple_right = SimpleRight::new();

        wcm.register_component(simple_left.clone());
        wcm.register_component(simple_right.clone());
    }
}
