//! Parameterized Fibonacci-mod program. Reads `n` and `module` from stdin,
//! computes `fib(n) mod module`, and commits `(n, module, b)`.
//!
//! Replaces three near-identical sources previously in `examples/`:
//!   - aggregation/guest::guest (n: u16, mod=233)
//!   - multiple-programs/multiple-program-guest (n: u32, mod=233)
//!   - multiple-programs/multiple-program-guest-2 (n: u32, mod=253)

#![no_main]
ziskos::entrypoint!(main);

fn main() {
    let n: u32 = ziskos::io::read();
    let module: u32 = ziskos::io::read();
    ziskos::io::commit(&n);
    ziskos::io::commit(&module);

    let mut a: u32 = 0;
    let mut b: u32 = 1;
    for _ in 0..n {
        let c = (a + b) % module;
        a = b;
        b = c;
    }
    ziskos::io::commit(&b);
}
