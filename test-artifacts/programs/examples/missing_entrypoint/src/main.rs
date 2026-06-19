//! Negative-test fixture: a guest program intentionally missing
//! `#![no_main]` and `ziskos::entrypoint!(main);`. `elf2rom` must reject
//! the resulting ELF with an actionable error instead of letting the
//! emulator panic at PC=0.

fn main() {}
