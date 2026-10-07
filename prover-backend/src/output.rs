use anyhow::Result;
use std::path::Path;
use std::time::Duration;
use zisk_common::{
    ProgramVK, PublicValues, RandomSeed, StatsCostPerType, ZiskExecutorSummary, ZiskExecutorTime,
    ZiskVerifyBuilder, ZISK_PUBLICS,
};
use zisk_executor::{AirClassifier, PlanSummaryEntry};

pub use zisk_common::Proof;

fn pairs_to_publics(pairs: &[(u64, u32)]) -> PublicValues {
    let mut buf = vec![0u8; 32 + ZISK_PUBLICS * 8];
    for &(index, value) in pairs {
        let idx = index as usize;
        if idx < ZISK_PUBLICS {
            let off = 32 + idx * 8;
            buf[off..off + 4].copy_from_slice(&value.to_le_bytes());
        }
    }
    PublicValues::new(&buf)
}

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
            /// The program's raw public values.
            pub fn get_publics(&self) -> &PublicValues {
                &self.$field$(.$rest)*
            }

            /// Deserialize the public values into `T` (serde).
            pub fn get_public_values<T: serde::Serialize + serde::de::DeserializeOwned>(
                &self,
            ) -> Result<T> {
                Ok(self.$field$(.$rest)*.read()?)
            }

            /// Decode the public values into `T` using its Solidity ABI encoding.
            pub fn get_public_values_abi<T>(&self) -> Result<T>
            where
                T: alloy_sol_types::SolValue
                    + From<<T::SolType as alloy_sol_types::SolType>::RustType>,
            {
                Ok(self.$field$(.$rest)*.read_abi()?)
            }

            /// Copy the raw public-value bytes into `slice`.
            pub fn get_public_values_slice(&self, slice: &mut [u8]) {
                self.$field$(.$rest)*.read_slice(slice);
            }
        }
    };
}

/// Result of executing a program (without proving): step count, timing, and
/// public outputs, plus an optional per-AIR plan summary in standalone mode.
pub struct ExecuteOutput {
    steps: u64,
    time: u64,
    cost: Option<u64>,
    executor_time: ZiskExecutorTime,
    publics: PublicValues,
    plan: Option<Vec<PlanSummaryEntry>>,
    random_seed: Option<RandomSeed>,
}

impl ExecuteOutput {
    /// Construct from a full (proofman) execution: cost is derived from the
    /// summary's cost weights.
    pub fn new(
        execution_time: Duration,
        executor_summary: ZiskExecutorSummary,
        publics: &[u8],
    ) -> Self {
        Self {
            steps: executor_summary.steps,
            time: execution_time.as_millis() as u64,
            cost: Some(executor_summary.cost_per_type.total_cost()),
            executor_time: executor_summary.executor_time,
            publics: PublicValues::new(publics),
            plan: None,
            random_seed: executor_summary.random_seed,
        }
    }

    /// Constructor for the standalone path. No `SetupCtx` → no cost
    /// breakdown; carries the executor-produced plan summary and the
    /// raw `(index, value)` public-output pairs.
    pub fn new_standalone(
        execution_time: Duration,
        executor_summary: ZiskExecutorSummary,
        pub_outs: &[(u64, u32)],
        plan: Vec<PlanSummaryEntry>,
    ) -> Self {
        Self {
            steps: executor_summary.steps,
            time: execution_time.as_millis() as u64,
            cost: None,
            executor_time: executor_summary.executor_time,
            publics: pairs_to_publics(pub_outs),
            plan: Some(plan),
            random_seed: executor_summary.random_seed,
        }
    }

    /// Number of executed steps.
    pub fn get_execution_steps(&self) -> u64 {
        self.steps
    }

    /// Returns the total execution cost when available. `None` in
    /// standalone mode (no `SetupCtx` means no cost weights).
    pub fn get_execution_cost(&self) -> Option<u64> {
        self.cost
    }

    /// Wall-clock execution time, in milliseconds.
    pub fn get_execution_time(&self) -> u64 {
        self.time
    }

    /// Per-phase executor timing breakdown.
    pub fn get_executor_time(&self) -> &ZiskExecutorTime {
        &self.executor_time
    }

    /// Per-AIR plan summary. `Some` only in standalone mode; the full
    /// proofman path emits its own plan via tracing.
    pub fn get_plan(&self) -> Option<&[PlanSummaryEntry]> {
        self.plan.as_deref()
    }

    /// The host randomness seed the execution ran with, which decided the values the guest got from
    /// `zkvm_random_u64`. Passing it to `.random_seed()` on a new request reproduces the run.
    /// `None` only when a remote coordinator that predates this field ran it.
    pub fn get_random_seed(&self) -> Option<RandomSeed> {
        self.random_seed
    }

    /// Construct a result from a remote coordinator response.
    ///
    /// `plan` carries raw `(airgroup_id, air_id, count)` triples from the wire; the
    /// AIR display name is resolved here via [`AirClassifier`] so remote output matches
    /// the embedded plan. `seed` is the one the coordinator reported, if any.
    #[allow(clippy::too_many_arguments)]
    pub fn from_remote(
        steps: u64,
        execution_time: Duration,
        cost_per_type: StatsCostPerType,
        executor_time: ZiskExecutorTime,
        plan: &[(usize, usize, u64)],
        publics: &[u8],
        seed: Option<RandomSeed>,
    ) -> Self {
        let plan: Vec<PlanSummaryEntry> = plan
            .iter()
            .map(|&(airgroup_id, air_id, count)| PlanSummaryEntry {
                airgroup_id,
                air_id,
                name: AirClassifier::name(airgroup_id, air_id),
                count: count as usize,
            })
            .collect();
        Self {
            steps,
            time: execution_time.as_millis() as u64,
            cost: Some(cost_per_type.total_cost()),
            executor_time,
            publics: PublicValues::new(publics),
            plan: (!plan.is_empty()).then_some(plan),
            random_seed: seed,
        }
    }
}

impl_public_outputs!(ExecuteOutput, publics);

/// Result of proving a program: the generated [`Proof`] plus execution
/// statistics (steps, cost, proving time).
pub struct ProveOutput {
    summary: ExecutionSummary,
    proof: Proof,
    /// Key of the setup that produced `proof`, when that was this process.
    setup_vk: Option<Vec<u64>>,
}

impl ProveOutput {
    /// Construct from a completed proof and the execution it attests to, whose seed the proof
    /// takes.
    pub fn new(execution: ZiskExecutorSummary, proving_time: Duration, proof: Proof) -> Self {
        Self {
            summary: ExecutionSummary::new(proving_time, &execution),
            proof: proof.with_random_seed(execution.random_seed),
            setup_vk: None,
        }
    }

    /// Set the key `verify` uses: that of the setup which produced the proof.
    pub fn with_produced_under(mut self, setup_vk: Option<Vec<u64>>) -> Self {
        self.setup_vk = setup_vk;
        self
    }

    /// Construct with an empty (default) proof — used when only the execution
    /// statistics are wanted, not a real proof. The proof still carries the execution's seed.
    pub fn new_null(execution: ZiskExecutorSummary, proving_time: Duration) -> Self {
        Self::new(execution, proving_time, Proof::default())
    }

    /// Construct a result from a remote coordinator response (no ExecutorStatsHandle).
    pub fn from_remote(
        proof: Proof,
        steps: u64,
        proving_time: Duration,
        cost_per_type: StatsCostPerType,
    ) -> Self {
        Self {
            summary: ExecutionSummary::from_remote(proving_time, steps, &cost_per_type),
            proof,
            setup_vk: None,
        }
    }

    /// Proving time, in milliseconds.
    pub fn get_proving_time(&self) -> u64 {
        self.summary.time
    }

    /// Number of executed steps.
    pub fn get_execution_steps(&self) -> u64 {
        self.summary.steps
    }

    /// Total execution cost.
    pub fn get_execution_cost(&self) -> u64 {
        self.summary.cost
    }

    /// The host randomness seed the proven execution ran with (see
    /// [`ExecuteOutput::get_random_seed`]). `None` for proofs that do not come from a single
    /// execution (aggregates), or when a remote coordinator that predates this field ran it.
    pub fn get_random_seed(&self) -> Option<RandomSeed> {
        self.proof.random_seed
    }

    /// Stamp the seed of the execution the proof attests to. Wrapping a proof re-proves its
    /// statement without running the guest, so the result keeps the source proof's seed.
    pub(crate) fn with_random_seed(self, seed: Option<RandomSeed>) -> Self {
        Self { proof: self.proof.with_random_seed(seed), ..self }
    }

    /// The generated proof.
    pub fn get_proof(&self) -> &Proof {
        &self.proof
    }

    /// The proof serialized as a `u64` field-element vector.
    pub fn get_proof_u64(&self) -> Result<Vec<u64>> {
        Ok(self.proof.get_proof_u64()?)
    }

    /// The proof serialized as raw bytes.
    pub fn get_proof_bytes(&self) -> Result<Vec<u8>> {
        Ok(self.proof.get_proof_bytes()?)
    }

    /// The program verification key committed by this proof (derived from the
    /// committed publics, not the untrusted stored copy).
    pub fn get_program_vk(&self) -> ProgramVK {
        self.proof.get_program_vk()
    }

    /// Write the proof to `path`.
    pub fn save_proof(&self, path: impl AsRef<Path>) -> Result<()> {
        Ok(self.proof.save(path)?)
    }

    /// Verify under the producing setup's key, else the release key.
    pub fn verify(&self) -> Result<()> {
        Ok(self.with_produced_setup_vk(self.proof.verify_builder()).verify()?)
    }

    /// Start a verification with the given public values (overriding the ones
    /// embedded in the proof).
    pub fn with_publics<'a>(&'a self, publics: &'a PublicValues) -> ZiskVerifyBuilder<'a> {
        self.with_produced_setup_vk(self.proof.with_publics(publics))
    }

    /// Start a verification against the given program verification key.
    pub fn with_program_vk<'a>(&'a self, program_vk: &'a ProgramVK) -> ZiskVerifyBuilder<'a> {
        self.with_produced_setup_vk(self.proof.with_program_vk(program_vk))
    }

    fn with_produced_setup_vk<'a>(
        &'a self,
        builder: ZiskVerifyBuilder<'a>,
    ) -> ZiskVerifyBuilder<'a> {
        match &self.setup_vk {
            Some(vk) => builder.with_setup_vk(vk),
            None => builder,
        }
    }
}

// ProveOutput's publics are derived from the proof body (the u32 view of a
// Vadcop proof's full publics, or a Plonk proof's stored u32 publics), so
// `get_publics` returns an owned value rather than a borrow.
impl ProveOutput {
    /// The public values, derived from the proof body (returned by value).
    pub fn get_publics(&self) -> PublicValues {
        self.proof.publics()
    }

    /// Deserialize the public values into `T` (serde).
    pub fn get_public_values<T: serde::Serialize + serde::de::DeserializeOwned>(
        &self,
    ) -> Result<T> {
        Ok(self.proof.publics().read()?)
    }

    /// Decode the public values into `T` using its Solidity ABI encoding.
    pub fn get_public_values_abi<T>(&self) -> Result<T>
    where
        T: alloy_sol_types::SolValue + From<<T::SolType as alloy_sol_types::SolType>::RustType>,
    {
        Ok(self.proof.publics().read_abi()?)
    }

    /// Copy the raw public-value bytes into `slice`.
    pub fn get_public_values_slice(&self, slice: &mut [u8]) {
        self.proof.publics().read_slice(slice);
    }
}

/// Result of a constraint-verification run: execution statistics, per-phase
/// timing, per-type cost breakdown, and public outputs. No proof is produced.
pub struct VerifyConstraintsOutput {
    summary: ExecutionSummary,
    executor_time: ZiskExecutorTime,
    cost_per_type: StatsCostPerType,
    publics: PublicValues,
    random_seed: Option<RandomSeed>,
}

impl VerifyConstraintsOutput {
    /// Construct from an execution summary and the run `duration` (ms).
    pub fn new(executor_summary: ZiskExecutorSummary, duration: u64, publics: &[u8]) -> Self {
        let summary = ExecutionSummary::new(Duration::from_millis(duration), &executor_summary);
        Self {
            summary,
            executor_time: executor_summary.executor_time,
            cost_per_type: executor_summary.cost_per_type,
            publics: PublicValues::new(publics),
            random_seed: executor_summary.random_seed,
        }
    }

    /// Number of executed steps.
    pub fn get_execution_steps(&self) -> u64 {
        self.summary.steps
    }

    /// Total execution cost across all types.
    pub fn get_execution_total_cost(&self) -> u64 {
        self.summary.cost
    }

    /// Per-type cost breakdown.
    pub fn get_execution_cost_per_type(&self) -> &StatsCostPerType {
        &self.cost_per_type
    }

    /// Per-phase executor timing breakdown.
    pub fn get_executor_time(&self) -> &ZiskExecutorTime {
        &self.executor_time
    }

    /// Run duration, in milliseconds.
    pub fn get_duration(&self) -> u64 {
        self.summary.time
    }

    /// Host randomness seed the execution ran with. Always set for outputs built from an
    /// execution.
    pub fn get_random_seed(&self) -> Option<RandomSeed> {
        self.random_seed
    }
}

impl_public_outputs!(VerifyConstraintsOutput, publics);

#[cfg(test)]
mod tests {
    use super::*;

    const SEED: RandomSeed = RandomSeed::new([4; RandomSeed::LEN]);

    fn executed() -> ZiskExecutorSummary {
        ZiskExecutorSummary { random_seed: Some(SEED), ..Default::default() }
    }

    fn publics() -> Vec<u8> {
        vec![0u8; 32 + ZISK_PUBLICS * 8]
    }

    #[test]
    fn prove_output_takes_the_seed_of_the_execution_it_attests() {
        let other = RandomSeed::new([5; RandomSeed::LEN]);
        let output =
            ProveOutput::new(executed(), Duration::ZERO, Proof::default().with_random_seed(other));
        assert_eq!(output.get_random_seed(), Some(SEED));
        assert_eq!(output.get_proof().random_seed, Some(SEED));
    }

    /// Ranks that end a prove without the final proof still report what they ran with.
    #[test]
    fn prove_output_without_a_final_proof_reports_the_seed() {
        let output = ProveOutput::new_null(executed(), Duration::ZERO);
        assert_eq!(output.get_random_seed(), Some(SEED));
    }

    /// A wrap runs no execution, so its output keeps the source proof's seed.
    #[test]
    fn wrap_output_keeps_the_source_seed() {
        let output =
            ProveOutput::new(ZiskExecutorSummary::default(), Duration::ZERO, Proof::default())
                .with_random_seed(Some(SEED));
        assert_eq!(output.get_random_seed(), Some(SEED));
        assert_eq!(output.get_proof().random_seed, Some(SEED));
    }

    #[test]
    fn execute_and_verify_constraints_outputs_report_the_random_seed() {
        let execute = ExecuteOutput::new(Duration::ZERO, executed(), &publics());
        assert_eq!(execute.get_random_seed(), Some(SEED));

        let standalone = ExecuteOutput::new_standalone(Duration::ZERO, executed(), &[], Vec::new());
        assert_eq!(standalone.get_random_seed(), Some(SEED));

        let verify = VerifyConstraintsOutput::new(executed(), 0, &publics());
        assert_eq!(verify.get_random_seed(), Some(SEED));
    }
}
