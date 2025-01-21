use std::sync::Arc;

use pil_std_lib::Std;
use witness::{witness_library, WitnessLibrary, WitnessManager};

use p3_field::PrimeField;
use p3_goldilocks::Goldilocks;
use rand::{distributions::Standard, prelude::Distribution};

use crate::{Lookup0, Lookup1, Lookup2_12, Lookup2_13, Lookup2_15, Lookup3};

witness_library!(WitnessLib, Goldilocks);

impl<F: PrimeField> WitnessLibrary<F> for WitnessLib
where
    Standard: Distribution<F>,
{
    fn register_witness(&mut self, wcm: Arc<WitnessManager<F>>) {
        Std::new(wcm.clone());
        let lookup0 = Lookup0::new();
        let lookup1 = Lookup1::new();
        let lookup2_12 = Lookup2_12::new();
        let lookup2_13 = Lookup2_13::new();
        let lookup2_15 = Lookup2_15::new();
        let lookup3 = Lookup3::new();

        wcm.register_component(lookup0.clone());
        wcm.register_component(lookup1.clone());
        wcm.register_component(lookup2_12.clone());
        wcm.register_component(lookup2_13.clone());
        wcm.register_component(lookup2_15.clone());
        wcm.register_component(lookup3.clone());
    }
}
