// This example program processes large u64 input data (250MB - 1GB+)
// Input size is controlled by INPUT_SIZE_MB environment variable in host build.rs

#![no_main]
ziskos::entrypoint!(main);

fn main() {
    let panic: u64 = ziskos::io::read();

    if panic == 0 {
        panic!("Intentional panic triggered by guest program");
    } else {
        println!("Panic not triggered triggered with input value: {}", panic);
        
    }
}
