use executor::ZiskExecutor;
use pil_std_lib::Std;
use precomp_keccakf::KeccakfManager;
use std::sync::Arc;

use p3_field::PrimeField64;
use witness::WitnessManager;

pub fn register_state_machines_dev<F: PrimeField64>(
    executor: &mut ZiskExecutor<F>,
    wcm: Arc<WitnessManager<F>>,
) {
    let register_u8: bool = false;
    let register_u16: bool = false;
    let register_specified_ranges: bool = false;

    // DON'T REMOVE THIS LINE, NEEDED FOR COMPUTING STD_PROD AND STD_SUM
    let _std = Std::new_dev(wcm.clone(), register_u8, register_u16, register_specified_ranges);

    // executor.register_main_sm(std.clone());

    // let mem = Mem::new(std.clone());
    // executor.register_sm(mem);

    let keccakf_sm = KeccakfManager::new::<F>(executor.keccak_path.clone());
    executor.register_sm(keccakf_sm);
}
