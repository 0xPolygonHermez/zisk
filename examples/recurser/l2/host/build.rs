// Builds the leaf guest ELF and resolves the aggregation definitions under
// `guest/aggregations/` — `build_program` does both (it processes any
// `aggregations/*.toml` in the target program dir), so this single call wires
// up both `load_program!("recurser_l2_guest")` and `load_aggregation_program!("l2")`.
fn main() {
    zisk_sdk::build_program("../guest");
    // A different guest (different programVK) used to show the allow-list
    // rejecting a foreign leaf. It has no `aggregations/`, so this only builds
    // the ELF.
    zisk_sdk::build_program("../foreign");
}
