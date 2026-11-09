mod common;
use common::*;

#[test]
fn compiled_programs_load_with_sbpf_v3_disabled() {
    let mut svm = v2_runtime();
    for (id, name) in [(ID, "zisk_solana_program"), (CONSUMER, "zisk_consumer_example")] {
        svm.add_program(id, &elf(name)).expect("program must load without SBPF v3");
    }
}
