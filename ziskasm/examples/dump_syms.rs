// Prints the zisklib symbol map (address, name), sorted by address. Used by
// ziskasm/zisklib/scripts/benchmark/profile.sh.
fn main() {
    let lib = ziskasm::assemble_zisk_library().unwrap();
    let mut v: Vec<_> = lib.symbols.iter().collect();
    v.sort_by_key(|(_, a)| **a);
    for (n, a) in v {
        println!("{a:#x} {n}");
    }
}
