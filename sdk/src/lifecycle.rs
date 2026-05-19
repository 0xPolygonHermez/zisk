//! Target enums that let `upload()` and `setup()` accept either a
//! [`GuestProgram`] or a [`RecurserAggregator`].

use zisk_prover_backend::GuestProgram;

use crate::aggregator::RecurserAggregator;

/// What `client.upload()` is targeting.
pub enum UploadTarget<'a> {
    Program(&'a GuestProgram),
    Aggregator(&'a RecurserAggregator),
}

impl<'a> From<&'a GuestProgram> for UploadTarget<'a> {
    fn from(p: &'a GuestProgram) -> Self {
        UploadTarget::Program(p)
    }
}

impl<'a> From<&'a RecurserAggregator> for UploadTarget<'a> {
    fn from(a: &'a RecurserAggregator) -> Self {
        UploadTarget::Aggregator(a)
    }
}

/// What `client.setup()` is targeting.
pub enum SetupTarget<'a> {
    Program(&'a GuestProgram),
    Aggregator(&'a RecurserAggregator),
}

impl<'a> From<&'a GuestProgram> for SetupTarget<'a> {
    fn from(p: &'a GuestProgram) -> Self {
        SetupTarget::Program(p)
    }
}

impl<'a> From<&'a RecurserAggregator> for SetupTarget<'a> {
    fn from(a: &'a RecurserAggregator) -> Self {
        SetupTarget::Aggregator(a)
    }
}
