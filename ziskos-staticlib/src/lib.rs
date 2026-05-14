#![cfg_attr(all(target_os = "zkvm", target_vendor = "zisk"), no_std)]
#![cfg_attr(all(target_os = "zkvm", target_vendor = "zisk"), feature(core_intrinsics))]
#![cfg_attr(all(target_os = "zkvm", target_vendor = "zisk"), allow(internal_features))]

// This crate produces libziskos.a for linking by C programs.
// Re-exporting the public interface ensures those symbols are bundled into the
// archive.  The #[panic_handler] is required by staticlib but not rlib targets.

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
pub use ziskos::zkvm_init;
#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
pub use ziskos::zkvm_deinit;
#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
pub use ziskos::zisklib::zkvm_io::read_input;
#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
pub use ziskos::zisklib::zkvm_io::write_output;
#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
pub use ziskos::zisklib::zkvm_accelerators::*;

#[cfg(all(target_os = "zkvm", target_vendor = "zisk", feature = "panic-handler"))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    core::intrinsics::abort()
}
