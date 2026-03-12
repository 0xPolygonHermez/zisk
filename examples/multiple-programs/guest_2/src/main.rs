// This example program takes a number `n` as input and computes the SHA-256 hash `n` times sequentially.

// Mark the main function as the entry point for ZisK
#![no_main]
ziskos::entrypoint!(main);

fn main() {
    // Read the input data
    let n: u32 = ziskos::io::read();

    let module = 253;

    ziskos::io::commit(&n);
    ziskos::io::commit(&module);

    let mut a = 0;
    let mut b = 1;
    for _ in 0..n {
        let mut c = a + b;
        c %= module;
        a = b;
        b = c;
    }

    ziskos::io::commit(&b);
}
