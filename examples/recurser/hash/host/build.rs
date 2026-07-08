// Builds the leaf guest ELF and resolves `guest/aggregations/hash.toml`
// (`build_program` processes any `aggregations/*.toml` in the target dir), wiring
// up both `load_program!("recurser_hash_guest")` and `load_aggregation_program!("hash")`.
fn main() {
    zisk_sdk::build_program("../guest");
}
