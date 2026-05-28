#![cfg_attr(zisk_guest, no_std)]
#![cfg_attr(zisk_guest, feature(core_intrinsics))]
#![cfg_attr(zisk_guest, allow(internal_features))]

// This crate produces libziskos.a for linking by C programs.
// Re-exporting the public interface ensures those symbols are bundled into the
// archive.  The #[panic_handler] is required by staticlib but not rlib targets.

pub use ziskos::zisklib::zkvm_accelerators::*;
pub use ziskos::zisklib::zkvm_io::read_input;
pub use ziskos::zisklib::zkvm_io::write_output;
pub use ziskos::zkvm_deinit;
pub use ziskos::zkvm_init;

#[cfg(all(feature = "panic-handler", zisk_guest))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    core::intrinsics::abort()
}

// ── Forbidden global allocator ────────────────────────────────────────────────
// `zisk-custom-alloc` disables the bump allocator inside `ziskos`, leaving no
// `#[global_allocator]` registered.  Rust requires one whenever the `alloc`
// crate is linked (even if it is never called), so we satisfy the compiler with
// a stub whose alloc/dealloc bodies call non-existent extern symbols.
//
// Effect: if any code path actually invokes the global allocator at link time,
// the C consumer's linker fails with a clear "undefined symbol" error that names
// the forbidden operation.  Code that allocates exclusively through BumpScratch
// (ScratchVec, etc.) never calls these functions, so the link succeeds.
#[cfg(zisk_guest)]
mod forbidden_alloc {
    use core::alloc::{GlobalAlloc, Layout};

    unsafe extern "Rust" {
        safe fn __ziskos_forbidden_alloc(size: usize, align: usize) -> *mut u8;
        safe fn __ziskos_forbidden_dealloc(ptr: *mut u8, size: usize, align: usize);
    }

    struct ForbiddenAlloc;

    unsafe impl GlobalAlloc for ForbiddenAlloc {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            __ziskos_forbidden_alloc(layout.size(), layout.align())
        }
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            __ziskos_forbidden_dealloc(ptr, layout.size(), layout.align())
        }
    }

    #[global_allocator]
    static ALLOC: ForbiddenAlloc = ForbiddenAlloc;
}
