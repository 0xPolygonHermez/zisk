use crate::{
    create_debug_info, prover::verify_input::VerifyInput, BackendProverOpts,
    VerifyConstraintsOutput,
};
use anyhow::Result;
use colored::Colorize;
use executor::{
    initialize_executor_test, unit_test_hooks::UnitTestHookBag,
    unit_test_trace_override::TraceOverrideBag, ZiskExecutorTest,
};
use fields::Goldilocks;
use proofman::ProofMan;
use proofman_common::initialize_logger;
use std::any::Any;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use zisk_common::{PublicValues, ZiskExecutorSummary, ZiskPaths};
//
// Note: `proofman.get_publics()` returns `Vec<u8>`; we wrap with `PublicValues::new` when we
// need the typed view (mirrors `prover-backend/src/prover/backend.rs`).

/// Unit-test prover. Constructed via [`UnitTestProver::new`]; runs constraint verification
/// against a JSON input file via [`UnitTestProver::verify_constraints`].
pub struct UnitTestProver {
    proofman: ProofMan<Goldilocks>,
    executor: Arc<ZiskExecutorTest>,
    proving_key_path: PathBuf,
}

impl UnitTestProver {
    /// Build the unit-test backend from `BackendProverOpts`. Only the proving-key path and
    /// verbosity are honored; SNARK / GPU / aggregation settings are ignored because they don't
    /// apply to constraint verification.
    pub fn new(opts: &BackendProverOpts) -> Result<Self> {
        let proving_key = ZiskPaths::get_proving_key(opts.get_proving_key());

        Self::print_command_info(&proving_key);

        let mut options = opts.build_proofman_options();
        options.verify_constraints = true;
        options.aggregation = false;

        if !proving_key.exists() {
            anyhow::bail!("Proving key not found at {}", proving_key.display());
        }

        let proofman = ProofMan::new(proving_key.clone(), options.clone())
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        let rank_info = proofman.get_rank_info();
        initialize_logger(options.verbose_mode, Some(&rank_info));

        proofman.set_barrier();

        let executor = initialize_executor_test(options.verbose_mode, false, &proofman.get_wcm())?;

        // Mirror EmuProver/AsmProver: when proofman is in packed/GPU mode, witness rows
        // must be the packed layout. `BackendProverOpts::build_proofman_options` flips
        // `packed` on whenever `gpu` is set, so we just propagate that bit.
        executor.set_packed(options.packed);

        Ok(Self { proofman, executor, proving_key_path: proving_key })
    }

    /// Begin a typed verify-input builder for in-memory test workflows.
    ///
    /// ```ignore
    /// let result = prover.verify_input()
    ///     .input::<BinarySm>(BinaryInput { op: 15, a: 5, b: 3 })
    ///     .hook::<BinarySm>(|input_idx, _clock, row| { /* mutate row */ })
    ///     .run()?;
    /// ```
    pub fn verify_input(&self) -> VerifyInput<'_> {
        VerifyInput::new(self)
    }

    /// Internal entry used by the [`VerifyInput`] builder.
    pub(crate) fn run_verify_input(
        &self,
        inputs: HashMap<String, Vec<Box<dyn Any + Send + Sync>>>,
        hooks: UnitTestHookBag,
        trace_overrides: TraceOverrideBag,
        debug_info: Option<Option<String>>,
    ) -> Result<VerifyConstraintsOutput> {
        let start = std::time::Instant::now();
        self.executor.set_input_data_value(inputs);
        self.executor.set_hooks(hooks);
        self.executor.set_trace_overrides(trace_overrides);
        let result = self.run_verify(start, debug_info);
        // Always clear hooks and overrides for the next call.
        self.executor.set_hooks(UnitTestHookBag::new());
        self.executor.set_trace_overrides(TraceOverrideBag::new());
        result
    }

    fn run_verify(
        &self,
        start: std::time::Instant,
        debug_info: Option<Option<String>>,
    ) -> Result<VerifyConstraintsOutput> {
        let mut debug_info = create_debug_info(debug_info, self.proving_key_path.clone())?;

        // Skip global-constraint verification: in unit-test mode only a
        // subset of state machines is fed inputs, so cross-SM range-check
        // multiplicities can never balance. Sentinel entry in
        // `debug_instances` flips the proofman's global-constraint check
        // to false without disturbing per-AIR verification.
        debug_info.debug_instances.insert(0, HashMap::new());

        self.proofman
            .verify_proof_constraints_from_lib(&debug_info)
            .map_err(|e| anyhow::anyhow!("Constraint verification failed: {}", e))?;

        let publics = self.proofman.get_publics();
        let elapsed = start.elapsed();

        Ok(VerifyConstraintsOutput::new(
            ZiskExecutorSummary::default(),
            elapsed.as_millis() as u64,
            &publics,
        ))
    }

    /// Reference to the inner `ZiskExecutorTest` for advanced callers.
    pub fn executor(&self) -> &Arc<ZiskExecutorTest> {
        &self.executor
    }

    /// Cancel any in-flight work managed by the underlying `ProofMan`.
    pub fn cancel(&self) {
        self.proofman.cancel();
    }

    /// Reference to the inner `ProofMan` for advanced callers; primarily for parity with the
    /// production backend's introspection helpers. Most callers should not need this.
    pub fn proofman(&self) -> &ProofMan<Goldilocks> {
        &self.proofman
    }

    /// Strip down to a `PublicValues` view of the underlying proofman state.
    pub fn publics(&self) -> PublicValues {
        PublicValues::new(&self.proofman.get_publics())
    }

    fn print_command_info(proving_key: &Path) {
        println!(
            "{: >12} {}",
            "Unit Test".bright_green().bold(),
            "Verifying constraints from JSON inputs".bright_yellow()
        );
        println!("{: >12} {}", "Proving Key".bright_green().bold(), proving_key.display());
        println!();
    }
}
