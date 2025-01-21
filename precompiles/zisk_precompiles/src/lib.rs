use std::{collections::HashMap, sync::Arc};

use precomp_keccakf::KECCAK_OPCODE;
use precompiles_common::{PrecompileCall, PrecompileCode};

pub fn precompiles_map() -> HashMap<PrecompileCode, Arc<dyn PrecompileCall>> {
    let mut registry = HashMap::new();

    registry.insert(KECCAK_OPCODE.into(), Arc::new(Keccak256Precompile::new()));

    registry
}
