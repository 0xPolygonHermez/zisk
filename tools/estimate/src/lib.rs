//! Proof-time estimator of ZisK: predicts how long a block takes to prove as a coordinator job on
//! W workers, from one execution of the block (its instance plan and how long the execution
//! took). `model` is the coefficient table and the placement of the instances on the workers;
//! `cluster` is the model; `fit` turns measurements into its coefficients. None of them executes
//! anything: the binaries do that.

pub mod cluster;
pub mod files;
pub mod fit;
pub mod model;
