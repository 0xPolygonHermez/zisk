mod fcalls;
pub use fcalls::*;
#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
mod lib;
#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
pub use lib::*;
#[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
mod fcalls_impl;
#[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
pub use fcalls_impl::*;
