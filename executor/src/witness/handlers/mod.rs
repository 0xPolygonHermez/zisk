//! Per-category witness-compute handlers.

pub mod main;
pub mod rom_rust;
pub mod secondary;
pub mod table;

pub use main::MainWitnessHandler;
pub use secondary::SecondaryWitnessHandler;
pub use table::TableWitnessHandler;

use std::collections::HashMap;

use zisk_common::Instance;

/// Map of secondary instances keyed by `global_id`.
pub(crate) type SecnInstanceMap<F> = HashMap<usize, Box<dyn Instance<F>>>;

/// Map of borrowed secondary instances.
pub(crate) type SecnInstanceMapRef<'a, F> = HashMap<usize, &'a dyn Instance<F>>;
