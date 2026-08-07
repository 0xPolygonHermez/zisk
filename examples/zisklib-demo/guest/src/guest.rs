//! Demo guest: calls a function implemented in the ZisK library (`ziskasm/lib/`)
//! rather than in Rust. The `ziskos_add` stub (see `ziskos.rs`) is redirected by
//! the transpiler to the hand-written `zisklib_add` routine, which runs as ZisK
//! instructions in the guest's place. The committed result (7) proves the
//! redirect happened — the stub's own body would return 0xBAD.

#![no_main]

ziskos::entrypoint!(main);

#[path = "ziskos.rs"]
mod stubs;

use stubs::ziskos_add;

fn main() {
    // `black_box` keeps the arguments opaque so the call to `ziskos_add` is a real
    // call (not const-folded), leaving a call site for the transpiler to redirect.
    let a = core::hint::black_box(3u64);
    let b = core::hint::black_box(4u64);
    let sum = ziskos_add(a, b);
    ziskos::io::commit(&sum);
    println!("ziskos_add({a}, {b}) = 0x{sum:x}");
}
