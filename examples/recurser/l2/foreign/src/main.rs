//! A "foreign" guest for the recurser `l2` example — a *different* program, so
//! it has a different programVK. The aggregation's allow-list only permits
//! `recurser_l2_guest`, so a proof from this guest is rejected when the host
//! tries to fold it (the circuit becomes unsatisfiable at witness generation).
//!
//! It commits arbitrary bytes; the publics don't matter — the fold fails on the
//! programVK check before anything else.
#![no_main]
ziskos::entrypoint!(main);

fn main() {
    ziskos::io::commit_slice(&[0xAAu8; 32]);
}
