//! Error types for the Main State Machine crate.
//!
//! All fallible operations in `sm-main` return [`Result`], the crate-local alias for
//! `std::result::Result<T, MainSmError>`. [`MainSmError`] implements
//! `std::error::Error + Send + Sync + 'static`, so callers using `anyhow` or
//! `ProofmanResult` can propagate via `?` with the usual `From`/`map_err` bridges.

/// Crate-local `Result` alias bound to [`MainSmError`].
pub type Result<T> = std::result::Result<T, MainSmError>;

/// Errors produced by the Main State Machine planner and witness pipeline.
#[derive(Debug, thiserror::Error)]
pub enum MainSmError {
    /// The minimal trace size is not a power of two.
    #[error("min_traces_size ({size}) must be a power of two")]
    MinTraceSizeNotPowerOfTwo {
        /// The offending minimal trace size.
        size: u64,
    },

    /// The configured minimal trace size exceeds the row capacity of `MainTrace`.
    #[error("min_traces_size ({min_traces_size}) exceeds MainTrace::NUM_ROWS ({num_rows})")]
    MinTraceSizeTooBig {
        /// The offending minimal trace size.
        min_traces_size: u64,
        /// The fixed row count of `MainTrace`.
        num_rows: u64,
    },
    // /// The plan handed to the main instance has no `segment_id`.
    // #[error("plan is missing a segment_id")]
    // MissingSegmentId,

    // /// The plan metadata could not be downcast to the expected `bool`
    // /// (the `is_last_segment` flag set by the planner).
    // #[error("plan metadata is not the expected bool (is_last_segment)")]
    // InvalidSegmentMetadata,

    // /// `fill_trace_outputs` was empty — the segment had no minimal traces to process.
    // #[error("fill_trace_outputs is empty; segment has no minimal traces")]
    // EmptyFillTraceOutput,

    // /// `MemHelpers::mem_step_to_slot` returned a value outside the expected `0..=2` range.
    // #[error("mem_step_to_slot produced invalid slot {slot}")]
    // InvalidSlot {
    //     /// The offending slot value.
    //     slot: u8,
    // },

    // /// An internal error occurred.
    // #[error("{0}")]
    // Internal(String),
}

impl From<MainSmError> for proofman_common::ProofmanError {
    fn from(err: MainSmError) -> Self {
        proofman_common::ProofmanError::InvalidSetup(err.to_string())
    }
}
