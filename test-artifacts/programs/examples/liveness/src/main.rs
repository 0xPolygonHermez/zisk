//! Liveness test guest: reads two u64 records and either runs short-mode
//! (return immediately) or long-mode (busy-loop ~10s on prod hardware). Used
//! by the host driver to exercise worker post-failure recovery:
//!   - short-mode: baseline successful execute.
//!   - long-mode:  long enough that the host can cancel mid-flight.
//!   - malformed second record: the second `read::<u64>()` panics — the JIT
//!                              signal handler reports a task failure.

#![no_main]
ziskos::entrypoint!(main);

fn main() {
    let mode: u64 = ziskos::io::read();
    let value: u64 = ziskos::io::read();

    if mode == 1 {
        const ITER: u64 = 200_000_000;
        let mut acc: u64 = value;
        let mut i: u64 = 0;
        while i < ITER {
            acc = acc.wrapping_add(i.wrapping_mul(0x9E37_79B9_7F4A_7C15));
            i = i.wrapping_add(1);
        }
        println!("liveness-guest: long-mode finished, acc={acc}");
    } else {
        println!("liveness-guest: short-mode mode={mode} value={value}");
    }
}
