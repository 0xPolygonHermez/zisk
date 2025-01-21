use std::sync::Arc;

use pil_std_lib::Std;
use witness::{witness_library, WitnessLibrary, WitnessManager};

use p3_field::PrimeField;
use p3_goldilocks::Goldilocks;
use rand::{distributions::Standard, prelude::Distribution};

use crate::{DirectUpdateProdLocal, DirectUpdateProdGlobal, DirectUpdateSumLocal, DirectUpdateSumGlobal};

witness_library!(WitnessLib, Goldilocks);

impl<F: PrimeField> WitnessLibrary<F> for WitnessLib
where
    Standard: Distribution<F>,
{
    fn register_witness(&mut self, wcm: Arc<WitnessManager<F>>) {
        Std::new(wcm.clone());
        let direct_update_prod_local = DirectUpdateProdLocal::new();
        let direct_update_prod_global = DirectUpdateProdGlobal::new();
        let direct_update_sum_local = DirectUpdateSumLocal::new();
        let direct_update_sum_global = DirectUpdateSumGlobal::new();

        wcm.register_component(direct_update_prod_local.clone());
        wcm.register_component(direct_update_prod_global.clone());
        wcm.register_component(direct_update_sum_local.clone());
        wcm.register_component(direct_update_sum_global.clone());
    }
}
