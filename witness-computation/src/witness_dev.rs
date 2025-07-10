use executor::DynSMBundle;
use pil_std_lib::Std;
use precomp_keccakf::KeccakfManager;
use proofman::register_std_dev;
use sm_mem::Mem;
use std::{path::PathBuf, sync::Arc};
use zisk_core::ZiskRom;

use fields::PrimeField64;
use witness::WitnessManager;

pub fn register_state_machines_dev<F: PrimeField64>(
    wcm: Arc<WitnessManager<F>>,
    std: Arc<Std<F>>,
    _zisk_rom: Arc<ZiskRom>,
    _asm_path: Option<PathBuf>,
    _sha256f_script_path: PathBuf,
) -> (DynSMBundle<F>, bool) {
    let register_u8: bool = false;
    let register_u16: bool = false;
    let register_specified_ranges: bool = false;
    register_std_dev(&wcm, &std, register_u8, register_u16, register_specified_ranges);

    let keccakf_sm = KeccakfManager::new(wcm.get_sctx());

    let mem_sm = Mem::new(std.clone());

    let mut bundle = DynSMBundle::default();
    bundle.add_secn_sm(keccakf_sm);
    bundle.add_secn_sm(mem_sm);
    (bundle, false)
}
