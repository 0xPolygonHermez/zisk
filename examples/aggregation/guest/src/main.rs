// This example program takes a number `n` as input and computes the SHA-256 hash `n` times sequentially.

// Mark the main function as the entry point for ZisK
#![no_main]
ziskos::entrypoint!(main);

fn main() {
    // Read the input data
    let n: u32 = ziskos::io::read();

    let module: u32 = 233;

    ziskos::io::write_output(&n.to_le_bytes());
    ziskos::io::write_output(&module.to_le_bytes());

    let mut a: u32 = 0;
    let mut b: u32 = 1;
    for _ in 0..n {
        let mut c = a + b;
        c %= module;
        a = b;
        b = c;
    }

    ziskos::io::write_output(&b.to_le_bytes());
}
