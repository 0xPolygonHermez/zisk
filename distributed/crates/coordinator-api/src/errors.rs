//! The error contract between the coordinator and its clients: stable numeric codes, and the
//! response metadata that carries them.
//!
//! Every API error status carries its code and name in [`CODE_KEY`] and [`NAME_KEY`], so a
//! client acts on the code instead of parsing the message. The gRPC status code says whether a
//! retry can help: only `UNAVAILABLE` ([`codes::CLUSTER_UNAVAILABLE`]) is worth retrying.

/// Response metadata key holding the error's numeric code (e.g. `2001`).
pub const CODE_KEY: &str = "zisk-error-code";

/// Response metadata key holding the error's name (e.g. `CLUSTER_UNAVAILABLE`).
pub const NAME_KEY: &str = "zisk-error-name";

/// Stable numeric error codes (matches `book/developer/coordinator_api.md`).
pub mod codes {
    /// The requested job does not exist.
    pub const JOB_NOT_FOUND: u32 = 1001;
    /// The requested program is not registered.
    pub const PROGRAM_NOT_FOUND: u32 = 1002;
    /// The program is registered but its setup is not complete.
    pub const PROGRAM_NOT_SETUP: u32 = 1003;
    /// The operation is invalid for the job's current state.
    pub const INVALID_JOB_STATE: u32 = 1004;
    /// The requested proof-kind conversion is not allowed.
    pub const INVALID_PROOF_CONVERSION: u32 = 1005;
    /// The request provided a random seed and the coordinator does not accept client random seeds.
    pub const CLIENT_RANDOM_SEED_NOT_ALLOWED: u32 = 1006;
    /// The request carried job metadata and the coordinator does not accept it.
    pub const JOB_METADATA_NOT_ALLOWED: u32 = 1007;
    /// The request asks for a final SNARK the workers cannot prove.
    pub const SNARK_UNAVAILABLE: u32 = 1008;
    /// The pushed input is larger than the guest's input region holds.
    pub const INPUT_TOO_LARGE: u32 = 1009;
    /// The cluster cannot take the job now (busy, setting up, or no ready workers); retry later.
    pub const CLUSTER_UNAVAILABLE: u32 = 2001;
    /// The cluster has less capacity than the job's minimum; retrying does not help until it
    /// grows.
    pub const CLUSTER_TOO_SMALL: u32 = 2002;
    /// An unexpected internal error.
    pub const INTERNAL: u32 = 3001;
}
