#![no_main]
ziskos::entrypoint!(main);

mod accelerators;

fn main() {
    accelerators::diagnostic_accelerators();
}
