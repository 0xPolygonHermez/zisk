//! Tests guest failure modes. Input value controls behavior:
//!   0 -> panic!
//!   1 -> assert! failure
//!   2 -> segfault
//!   _ -> normal exit

#![no_main]
ziskos::entrypoint!(main);

fn main() {
    let input: u64 = ziskos::io::read();

    match input {
        0 => panic!("Intentional panic triggered by guest program"),
        1 => assert!(false, "Intentional assert failure triggered by guest program"),
        2 => unsafe {
            let ptr = core::ptr::null_mut::<u64>();
            core::ptr::write_volatile(ptr, 0xDEAD);
        },
        _ => println!("No failure triggered with input value: {}", input),
    }
}
