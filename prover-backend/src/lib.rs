//! Prover backend for the [ZisK] zkVM.
//!
//! ZisK lets you prove that a program ran and produced a given result, so
//! anyone can check the result without re-running the program. You compile
//! your program to a RISC-V ELF; this crate is the engine that runs it and,
//! when asked, produces that proof.
//!
//! # What you can do
//!
//! - **Execute** — just run the program and get its output. Fast; no proof.
//!   Handy during development.
//! - **Prove** — run the program *and* generate a proof of the result.
//!
//! # Picking a backend
//!
//! There are two ways to run the program:
//!
//! - [`EmuB`] — a portable, pure-Rust emulator. Works everywhere.
//! - [`AsmB`] — a faster native (assembly) runner for supported platforms.
//!
//! # Usage
//!
//! Start from [`ProverClientBuilder`], choose a backend and what to do, then
//! `build()` a client. Every combination gives you the same [`ExecuteClient`]
//! interface, so you can hold one as `Box<dyn ExecuteClient>` and switch
//! backends at runtime.
//!
//! ```ignore
//! use zisk_prover_backend::ProverClientBuilder;
//!
//! // Emulator backend, execute-only (no proving).
//! let client = ProverClientBuilder::new().emu().execute_only().build()?;
//!
//! client.setup(&program, false)?;
//! let output = client.execute(&program, stdin, None)?;
//! ```
//!
//! [ZisK]: https://github.com/0xPolygonHermez/zisk

#![warn(missing_docs)]
#![warn(rustdoc::all)]
#![deny(rustdoc::missing_crate_level_docs)]

mod builder;
mod circuit;
mod execute_client;
mod guest;
mod output;
mod prover;
mod utils;

pub use execute_client::ExecuteClient;

pub use proofman_common::VerboseMode;
pub use zisk_executor::PlanSummaryEntry;
pub use zisk_pil::get_packed_info;
pub use zisk_rom_setup::HashMode;

pub use builder::*;
pub use circuit::*;
pub use guest::*;
pub use output::*;
pub use prover::*;
pub use utils::*;
pub use zisk_program_macros::load_program;
