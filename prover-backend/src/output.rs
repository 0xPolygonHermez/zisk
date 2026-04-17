use anyhow::Result;
use std::path::Path;
use std::time::Duration;
use zisk_common::{
    ProgramVK, PublicValues, StatsCostPerType, ZiskExecutorSummary, ZiskVerifyBuilder,
};

pub use zisk_common::Proof;

pub(crate) struct ExecutionSummary {
    pub steps: u64,
    pub cost: u64,
}

pub struct ExecuteOutput {
    execution_time: Duration,
    execution_summary: ExecutionSummary,
    publics: PublicValues,
}

impl ExecuteOutput {
    pub fn new(
        execution_time: Duration,
        executor_summary: ZiskExecutorSummary,
        publics: &[u8],
    ) -> Self {
        Self {
            execution_time,
            execution_summary: ExecutionSummary {
                steps: executor_summary.steps,
                cost: executor_summary.cost_per_type.total_cost(),
            },
            publics: PublicValues::new(publics),
        }
    }

    pub fn get_publics(&self) -> &PublicValues {
        &self.publics
    }

    pub fn get_public_values<T: serde::Serialize + serde::de::DeserializeOwned>(
        &self,
    ) -> Result<T> {
        self.publics.read()
    }

    pub fn get_public_values_abi<T>(&self) -> Result<T>
    where
        T: alloy_sol_types::SolValue + From<<T::SolType as alloy_sol_types::SolType>::RustType>,
    {
        self.publics.read_abi()
    }

    pub fn get_execution_steps(&self) -> u64 {
        self.execution_summary.steps
    }

    pub fn get_execution_cost(&self) -> u64 {
        self.execution_summary.cost
    }

    pub fn get_execution_time(&self) -> Duration {
        self.execution_time
    }

    /// Construct a result from a remote gateway response.
    pub fn from_remote(
        steps: u64,
        execution_time: Duration,
        cost_per_type: StatsCostPerType,
        publics: &[u8],
    ) -> Self {
        Self {
            execution_time,
            execution_summary: ExecutionSummary { steps, cost: cost_per_type.total_cost() },
            publics: PublicValues::new(publics),
        }
    }
}

pub struct ProveOutput {
    execution_summary: ExecutionSummary,
    proving_time: Duration,
    proof: Proof,
}

impl ProveOutput {
    pub fn new(execution: ZiskExecutorSummary, proving_time: Duration, proof: Proof) -> Self {
        Self {
            execution_summary: ExecutionSummary {
                steps: execution.steps,
                cost: execution.cost_per_type.total_cost(),
            },
            proving_time,
            proof,
        }
    }

    pub fn new_null(execution: ZiskExecutorSummary, proving_time: Duration) -> Self {
        Self {
            execution_summary: ExecutionSummary {
                steps: execution.steps,
                cost: execution.cost_per_type.total_cost(),
            },
            proving_time,
            proof: Proof::default(),
        }
    }

    /// Construct a result from a remote gateway response (no ExecutorStatsHandle).
    pub fn from_remote(
        proof: Proof,
        steps: u64,
        proving_time: Duration,
        cost_per_type: StatsCostPerType,
    ) -> Self {
        Self {
            execution_summary: ExecutionSummary { steps, cost: cost_per_type.total_cost() },
            proving_time,
            proof,
        }
    }

    pub fn get_proving_time(&self) -> Duration {
        self.proving_time
    }

    pub fn get_execution_steps(&self) -> u64 {
        self.execution_summary.steps
    }

    pub fn get_execution_cost(&self) -> u64 {
        self.execution_summary.cost
    }

    pub fn get_proof(&self) -> &Proof {
        &self.proof
    }

    pub fn get_proof_bytes(&self) -> Vec<u8> {
        self.proof.get_proof_bytes()
    }

    pub fn get_publics(&self) -> &PublicValues {
        &self.proof.publics
    }

    pub fn get_program_vk(&self) -> &ProgramVK {
        &self.proof.program_vk
    }

    pub fn save_proof(&self, path: impl AsRef<Path>) -> Result<()> {
        self.proof.save(path)
    }

    /// Deserialize a value from public outputs.
    /// The value must have been previously written with bincode serialization using `commit()`.
    pub fn get_public_values<T: serde::Serialize + serde::de::DeserializeOwned>(
        &self,
    ) -> Result<T> {
        self.proof.publics.read()
    }

    /// Decode an ABI-encoded value from public outputs.
    /// The value must have been previously written with ABI encoding using `write_abi()`.
    pub fn get_public_values_abi<T>(&self) -> Result<T>
    where
        T: alloy_sol_types::SolValue + From<<T::SolType as alloy_sol_types::SolType>::RustType>,
    {
        self.proof.publics.read_abi()
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
