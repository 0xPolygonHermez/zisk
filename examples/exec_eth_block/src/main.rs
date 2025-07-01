#![no_main]
ziskos::entrypoint!(main);

extern crate alloc;

use alloc::sync::Arc;
use reth_chainspec::ChainSpec;
use reth_evm_ethereum::EthEvmConfig;
use reth_stateless::{fork_spec::ForkSpec, validation::stateless_validation, StatelessInput};

fn main() {
    let (input, fork_spec): (StatelessInput, ForkSpec) =
        bincode::deserialize(&ziskos::read_input()).unwrap();
    let chain_spec: Arc<ChainSpec> = Arc::new(fork_spec.into());
    let evm_config = EthEvmConfig::new(chain_spec.clone());

    stateless_validation(input.block, input.witness, chain_spec, evm_config).unwrap();
    
    println!("Validation successful!");
}
