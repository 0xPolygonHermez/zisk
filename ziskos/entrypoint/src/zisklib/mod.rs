mod fcalls;
pub use fcalls::*;
#[cfg(target_os = "ziskos")]
mod lib;
#[cfg(target_os = "ziskos")]
pub use lib::*;
#[cfg(not(target_os = "ziskos"))]
mod fcalls_impl;
#[cfg(not(target_os = "ziskos"))]
pub use fcalls_impl::*;
