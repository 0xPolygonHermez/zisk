//! Low-level 256-bit unsigned integer arithmetic.
//!
//! All values are represented as `[u64; 4]` in little-endian limb order.
//!
//! - [`add`] — Addition and subtraction with carry/borrow.
//! - [`mul`] — Full 512-bit multiplication and squaring.
//! - [`div`] — Division and remainder via hint-and-verify.
//! - [`modular`] — Modular addition, subtraction, and reduction.
//! - [`pow`] — Exponentiation by squaring.

mod add;
mod div;
mod modular;
mod mul;
mod pow;

pub use add::*;
pub use div::*;
pub use modular::*;
pub use mul::*;
pub use pow::*;
