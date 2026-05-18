use anyhow::Result;
use std::path::Path;
use std::time::Duration;
use zisk_common::{
    ProgramVK, PublicValues, StatsCostPerType, ZiskExecutorSummary, ZiskExecutorTime,
    ZiskVerifyBuilder,
};

pub use zisk_common::Proof;

/// Shared execution statistics captured after any run (execute, prove, verify_constraints).
pub(crate) struct ExecutionSummary {
    time: u64,
    steps: u64,
    cost: u64,
}

impl ExecutionSummary {
    pub fn new(time: Duration, executor_summary: &ZiskExecutorSummary) -> Self {
        Self {
            time: time.as_millis() as u64,
            steps: executor_summary.steps,
            cost: executor_summary.cost_per_type.total_cost(),
        }
    }

    pub fn from_remote(time: Duration, steps: u64, cost_per_type: &StatsCostPerType) -> Self {
        Self { time: time.as_millis() as u64, steps, cost: cost_per_type.total_cost() }
    }
}

macro_rules! impl_public_outputs {
    ($type:ty, $field:ident $(. $rest:ident)*) => {
        impl $type {
            pub fn get_publics(&self) -> &PublicValues {
                &self.$field$(.$rest)*
            }

            pub fn get_public_values<T: serde::Serialize + serde::de::DeserializeOwned>(
                &self,
            ) -> Result<T> {
                self.$field$(.$rest)*.read()
            }

            pub fn get_public_values_abi<T>(&self) -> Result<T>
            where
                T: alloy_sol_types::SolValue
                    + From<<T::SolType as alloy_sol_types::SolType>::RustType>,
            {
                self.$field$(.$rest)*.read_abi()
            }

            pub fn get_public_values_slice(&self, slice: &mut [u8]) {
                self.$field$(.$rest)*.read_slice(slice);
            }
        }
    };
}

pub struct ExecuteOutput {
    summary: ExecutionSummary,
    publics: PublicValues,
}

impl ExecuteOutput {
    pub fn new(
        execution_time: Duration,
        executor_summary: ZiskExecutorSummary,
        publics: &[u8],
    ) -> Self {
        Self {
            summary: ExecutionSummary::new(execution_time, &executor_summary),
            publics: PublicValues::new(publics),
        }
    }

    pub fn get_execution_steps(&self) -> u64 {
        self.summary.steps
    }

    pub fn get_execution_cost(&self) -> u64 {
        self.summary.cost
    }

    pub fn get_execution_time(&self) -> u64 {
        self.summary.time
    }

    /// Construct a result from a remote coordinator response.
    pub fn from_remote(
        steps: u64,
        execution_time: Duration,
        cost_per_type: StatsCostPerType,
        publics: &[u8],
    ) -> Self {
        Self {
            summary: ExecutionSummary::from_remote(execution_time, steps, &cost_per_type),
            publics: PublicValues::new(publics),
        }
    }
}

impl_public_outputs!(ExecuteOutput, publics);

pub struct ProveOutput {
    summary: ExecutionSummary,
    proof: Proof,
}

impl ProveOutput {
    pub fn new(execution: ZiskExecutorSummary, proving_time: Duration, proof: Proof) -> Self {
        Self { summary: ExecutionSummary::new(proving_time, &execution), proof }
    }

    pub fn new_null(execution: ZiskExecutorSummary, proving_time: Duration) -> Self {
        Self { summary: ExecutionSummary::new(proving_time, &execution), proof: Proof::default() }
    }

    /// Construct a result from a remote coordinator response (no ExecutorStatsHandle).
    pub fn from_remote(
        proof: Proof,
        steps: u64,
        proving_time: Duration,
        cost_per_type: StatsCostPerType,
    ) -> Self {
        Self { summary: ExecutionSummary::from_remote(proving_time, steps, &cost_per_type), proof }
    }

    pub fn get_proving_time(&self) -> u64 {
        self.summary.time
    }

    pub fn get_execution_steps(&self) -> u64 {
        self.summary.steps
    }

    pub fn get_execution_cost(&self) -> u64 {
        self.summary.cost
    }

    pub fn get_proof(&self) -> &Proof {
        &self.proof
    }

    pub fn get_proof_u64(&self) -> Result<Vec<u64>> {
        self.proof.get_proof_u64()
    }

    pub fn get_proof_bytes(&self) -> Result<Vec<u8>> {
        self.proof.get_proof_bytes()
    }

    pub fn get_program_vk(&self) -> &ProgramVK {
        &self.proof.program_vk
    }

    pub fn save_proof(&self, path: impl AsRef<Path>) -> Result<()> {
        self.proof.save(path)
    }

    pub fn verify(&self) -> Result<()> {
        self.proof.verify()
    }

    pub fn with_publics<'a>(&'a self, publics: &'a PublicValues) -> ZiskVerifyBuilder<'a> {
        self.proof.with_publics(publics)
    }

    pub fn with_program_vk<'a>(&'a self, program_vk: &'a ProgramVK) -> ZiskVerifyBuilder<'a> {
        self.proof.with_program_vk(program_vk)
    }
}

impl_public_outputs!(ProveOutput, proof.publics);

pub struct VerifyConstraintsOutput {
    summary: ExecutionSummary,
    executor_time: ZiskExecutorTime,
    cost_per_type: StatsCostPerType,
    publics: PublicValues,
}

impl VerifyConstraintsOutput {
    pub fn new(executor_summary: ZiskExecutorSummary, duration: u64, publics: &[u8]) -> Self {
        let summary = ExecutionSummary::new(Duration::from_millis(duration), &executor_summary);
        Self {
            summary,
            executor_time: executor_summary.executor_time,
            cost_per_type: executor_summary.cost_per_type,
            publics: PublicValues::new(publics),
        }
    }

    pub fn get_execution_steps(&self) -> u64 {
        self.summary.steps
    }

    pub fn get_execution_total_cost(&self) -> u64 {
        self.summary.cost
    }

    pub fn get_execution_cost_per_type(&self) -> &StatsCostPerType {
        &self.cost_per_type
    }

    pub fn get_executor_time(&self) -> &ZiskExecutorTime {
        &self.executor_time
    }

    pub fn get_duration(&self) -> u64 {
        self.summary.time
    }
}

impl_public_outputs!(VerifyConstraintsOutput, publics);
