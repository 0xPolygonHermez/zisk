//! Re-exports the upstream `proofman-starks-lib-c` sources copied by `build.rs`.

include!(concat!(env!("OUT_DIR"), "/upstream/src/lib.rs"));

pub const LINK_PROVENANCE_JSON: &str = include_str!(concat!(env!("OUT_DIR"), "/link-provenance.json"));
