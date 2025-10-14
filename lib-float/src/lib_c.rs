#![allow(unused_variables)]

include!("../bindings.rs");

#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
macro_rules! run_on_linux {
    ($body:expr) => {
        0
    };
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
macro_rules! run_on_linux {
    ($body:expr) => {
        unsafe { $body }
    };
}
