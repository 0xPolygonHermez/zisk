//! Target enums that let `upload()` and `setup()` accept either a
//! [`GuestProgram`] or a [`Recurser`].

use zisk_prover_backend::GuestProgram;

use crate::recurser::Recurser;

/// What `client.upload()` is targeting.
pub enum UploadTarget<'a> {
    Program(&'a GuestProgram),
    Recurser(&'a Recurser),
}

impl<'a> From<&'a GuestProgram> for UploadTarget<'a> {
    fn from(p: &'a GuestProgram) -> Self {
        UploadTarget::Program(p)
    }
}

impl<'a> From<&'a Recurser> for UploadTarget<'a> {
    fn from(a: &'a Recurser) -> Self {
        UploadTarget::Recurser(a)
    }
}

/// What `client.setup()` is targeting.
pub enum SetupTarget<'a> {
    Program(&'a GuestProgram),
    Recurser(&'a Recurser),
}

impl<'a> From<&'a GuestProgram> for SetupTarget<'a> {
    fn from(p: &'a GuestProgram) -> Self {
        SetupTarget::Program(p)
    }
}

impl<'a> From<&'a Recurser> for SetupTarget<'a> {
    fn from(a: &'a Recurser) -> Self {
        SetupTarget::Recurser(a)
    }
}
