mod fcalls;
pub use fcalls::*;
#[cfg(not(target_os = "ziskos"))]
mod fcalls_impl;
#[cfg(not(target_os = "ziskos"))]
pub use fcalls_impl::*;
