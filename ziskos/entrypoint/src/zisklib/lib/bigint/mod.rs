//! Arbitrary-precision integer arithmetic over multi-limb `[u64]` slices.
//!
//! - Addition: [`add_agtb`], [`add_short`].
//! - Multiplication: [`mul_short`], [`mul_long`].
//! - Squaring: [`square_short`], [`square_long`].
//! - Division: [`div_short`], [`div_long`].
//! - Remainder: [`rem_short`], [`rem_long`].
//! - Modular exponentiation: [`modexp`].

mod add_agtb;
mod add_short;
mod common;
mod div_long;
mod div_short;
mod modexp;
mod mul_long;
mod mul_short;
mod rem_long;
mod rem_short;
mod square_long;
mod square_short;

pub use add_agtb::*;
pub use add_short::*;
pub use common::{LongScratch, RemLongScratch, ShortScratch, U256};
pub use div_long::*;
pub use div_short::*;
pub use modexp::*;
pub use mul_long::*;
pub use mul_short::*;
pub use rem_long::*;
pub use rem_short::*;
pub use square_long::*;
pub use square_short::*;
