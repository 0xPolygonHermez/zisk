//! Minimal L2 "block segment" leaf: the host ABI-encodes a BlocksInfoStruct and
//! the guest commits those bytes verbatim as its publics. No computation — the
//! example is about the fold, not the leaf.
#![no_main]
ziskos::entrypoint!(main);

fn main() {
    let abi_bytes: &[u8] = ziskos::io::read_slice();
    ziskos::io::commit_slice(abi_bytes);
}
