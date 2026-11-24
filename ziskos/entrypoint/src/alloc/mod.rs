mod alloc;
pub use alloc::*;

mod kernel_heap;

#[cfg(feature = "alloc-marks")]
mod marks;
#[cfg(feature = "alloc-marks")]
pub use marks::AllocMarks;

#[cfg(all(
    not(feature = "zisk-embedded-alloc"),
    not(feature = "zisk-custom-alloc"),
    not(feature = "zisk-embedded-dlmalloc-alloc"),
    not(feature = "zisk-embedded-talc-alloc"),
    not(feature = "zisk-embedded-tlfs-alloc"),
    not(feature = "zisk-precompile-alloc")
))]
pub mod bump;

#[cfg(any(feature = "zisk-embedded-alloc", feature = "zisk-embedded-dlmalloc-alloc"))]
pub mod embedded_dlmalloc;

#[cfg(feature = "zisk-embedded-talc-alloc")]
pub mod embedded_talc;

#[cfg(feature = "zisk-embedded-tlfs-alloc")]
pub mod embedded_tlfs;

#[cfg(any(feature = "zisk-embedded-alloc", feature = "zisk-embedded-dlmalloc-alloc"))]
pub use embedded_dlmalloc as embedded;

#[cfg(feature = "zisk-embedded-talc-alloc")]
pub use embedded_talc as embedded;

#[cfg(feature = "zisk-embedded-tlfs-alloc")]
pub use embedded_tlfs as embedded;

#[cfg(feature = "zisk-precompile-alloc")]
pub mod precompile_alloc;

#[cfg(feature = "zisk-precompile-alloc")]
pub use precompile_alloc as embedded;

// disabled, worse performance
// pub mod embedded_lla;
// pub mod embedded_llff;
// pub use embedded_llff as embedded;
// pub use embedded_lla as embedded;
