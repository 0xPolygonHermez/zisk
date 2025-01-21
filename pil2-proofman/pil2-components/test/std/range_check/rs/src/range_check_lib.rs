use std::sync::Arc;

use pil_std_lib::Std;
use witness::{witness_library, WitnessLibrary, WitnessManager};

use p3_field::PrimeField;
use p3_goldilocks::Goldilocks;
use rand::{distributions::Standard, prelude::Distribution};

use crate::{
    RangeCheckMix, RangeCheckDynamic1, RangeCheckDynamic2, MultiRangeCheck1, MultiRangeCheck2, RangeCheck1,
    RangeCheck2, RangeCheck3, RangeCheck4,
};

witness_library!(WitnessLib, Goldilocks);

impl<F: PrimeField> WitnessLibrary<F> for WitnessLib
where
    Standard: Distribution<F>,
{
    fn register_witness(&mut self, wcm: Arc<WitnessManager<F>>) {
        let std_lib = Std::new(wcm.clone());
        let range_check1 = RangeCheck1::new(std_lib.clone());
        let range_check2 = RangeCheck2::new(std_lib.clone());
        let range_check3 = RangeCheck3::new(std_lib.clone());
        let range_check4 = RangeCheck4::new(std_lib.clone());
        let multi_range_check1 = MultiRangeCheck1::new(std_lib.clone());
        let multi_range_check2 = MultiRangeCheck2::new(std_lib.clone());
        let range_check_dynamic1 = RangeCheckDynamic1::new(std_lib.clone());
        let range_check_dynamic2 = RangeCheckDynamic2::new(std_lib.clone());
        let range_check_mix = RangeCheckMix::new(std_lib.clone());

        wcm.register_component(range_check1.clone());
        wcm.register_component(range_check2.clone());
        wcm.register_component(range_check3.clone());
        wcm.register_component(range_check4.clone());
        wcm.register_component(multi_range_check1.clone());
        wcm.register_component(multi_range_check2.clone());
        wcm.register_component(range_check_dynamic1.clone());
        wcm.register_component(range_check_dynamic2.clone());
        wcm.register_component(range_check_mix.clone());
    }
}
