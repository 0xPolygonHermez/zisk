mod client;
pub(crate) mod remote_execute;
pub(crate) mod remote_prove;
pub(crate) mod remote_setup;
pub(crate) mod remote_upload;
pub(crate) mod remote_wrap;

pub(crate) use client::{
    duration_to_proto_timestamp, proof_with_publics_to_proto, stdin_to_input_kind,
};
pub use client::{RemoteClient, RemoteClientBuilder};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct JobId(String);

impl From<String> for JobId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for JobId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<JobId> for String {
    fn from(id: JobId) -> Self {
        id.0
    }
}
