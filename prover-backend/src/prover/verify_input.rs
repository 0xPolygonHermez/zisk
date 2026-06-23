use std::any::Any;
use std::collections::HashMap;

use anyhow::Result;
use executor::unit_test_hooks::UnitTestHookBag;
use executor::unit_test_trace_override::TraceOverrideBag;
use executor::{AirValuesFn, ProofValuesFn};
use fields::Goldilocks;
use proofman_common::ConstraintsVerificationResult;
use zisk_common::UnitTestSm;
use zisk_pil::ZiskProofValues;

use crate::UnitTestProver;

/// Per-SM batch of type-erased inputs (each a boxed `S::Input`).
type ErasedInputs = HashMap<String, Vec<Box<dyn Any + Send + Sync>>>;

/// Fluent builder. Construct via [`UnitTestProver::input`],
/// [`UnitTestProver::inputs`], [`UnitTestProver::hook`] or
/// [`UnitTestProver::trace`], then chain further calls and finish with
/// [`run`](Self::run).
pub struct VerifyInput<'a> {
    prover: &'a UnitTestProver,
    inputs: ErasedInputs,
    hooks: UnitTestHookBag,
    trace_overrides: TraceOverrideBag,
    air_values: HashMap<usize, AirValuesFn>,
    proof_values: Option<ProofValuesFn>,
    global_constraints: Vec<usize>,
    debug_info: Option<Option<String>>,
}

impl<'a> VerifyInput<'a> {
    pub(crate) fn new(prover: &'a UnitTestProver) -> Self {
        Self {
            prover,
            inputs: HashMap::new(),
            hooks: UnitTestHookBag::new(),
            trace_overrides: TraceOverrideBag::new(),
            air_values: HashMap::new(),
            proof_values: None,
            global_constraints: Vec::new(),
            debug_info: None,
        }
    }

    /// Add one input for state-machine `S`. Repeated calls accumulate.
    ///
    /// The typed input is boxed (type-erased) under the SM's name; the
    /// executor downcasts it back to `S::Input` via the trait registry — no
    /// serialization round-trip.
    pub fn input<S>(mut self, input: S::Input) -> Self
    where
        S: UnitTestSm<Goldilocks>,
    {
        let boxed = Box::new(input) as Box<dyn Any + Send + Sync>;
        self.inputs.entry(S::name().to_string()).or_default().push(boxed);
        self
    }

    /// Add many inputs for SM `S` in one call.
    pub fn inputs<S, I>(mut self, inputs: I) -> Self
    where
        S: UnitTestSm<Goldilocks>,
        I: IntoIterator<Item = S::Input>,
    {
        let key = S::name().to_string();
        let bucket = self.inputs.entry(key).or_default();
        for inp in inputs {
            bucket.push(Box::new(inp) as Box<dyn Any + Send + Sync>);
        }
        self
    }

    /// Register a typed trace-row hook for SM `S`. Repeated calls for the
    /// same SM **stack** — each hook runs in registration order; the next
    /// one sees mutations from the previous. The closure receives
    /// `(input_idx, clock, &mut S::Row)` and is invoked once per row of
    /// every AIR instance produced for `S`.
    pub fn hook<S>(
        mut self,
        hook: impl Fn(usize, usize, &mut S::Row) + Send + Sync + 'static,
    ) -> Self
    where
        S: UnitTestSm<Goldilocks>,
    {
        self.hooks.register::<S>(hook);
        self
    }

    /// Author SM `S`'s trace directly, bypassing `compute_witness` for that
    /// AIR id. No `.input()` is needed: the executor plans one instance for
    /// the AIR from its fixed shape. The closure receives the freshly
    /// allocated, zeroed typed trace `&mut S::Trace` and the shared
    /// [`Std`](pil_std_lib::Std) handle — call into it (range checks,
    /// lookups) to emit the side-effects `compute_witness` would, so a trace
    /// can be authored as fully valid, or as invalid in exactly one chosen
    /// way. The SM must have a `unit_test_trace_override!` impl registered.
    ///
    /// A later call for the same SM replaces the earlier author closure.
    pub fn trace<S>(
        mut self,
        author: impl Fn(&mut S::Trace, &pil_std_lib::Std<Goldilocks>) -> proofman_common::ProofmanResult<()>
            + Send
            + Sync
            + 'static,
    ) -> Self
    where
        S: UnitTestSm<Goldilocks>,
    {
        self.trace_overrides.register::<S>(1, move |_, trace, std| author(trace, std));
        self
    }

    /// Author `instances` instances of SM `S`'s AIR, e.g. a segment chain for
    /// continuation testing. Like [`trace`](Self::trace), but the closure
    /// additionally receives the 0-based instance index and is invoked once
    /// per instance. Combine with [`air_values`](Self::air_values) for the
    /// per-segment air values and [`global_constraints`](Self::global_constraints)
    /// to verify the cross-instance linkage.
    pub fn traces<S>(
        mut self,
        instances: usize,
        author: impl Fn(
                usize,
                &mut S::Trace,
                &pil_std_lib::Std<Goldilocks>,
            ) -> proofman_common::ProofmanResult<()>
            + Send
            + Sync
            + 'static,
    ) -> Self
    where
        S: UnitTestSm<Goldilocks>,
    {
        self.trace_overrides.register::<S>(instances, author);
        self
    }

    /// Mutate SM `S`'s air values. The closure receives the 0-based per-AIR
    /// instance index and the flat `airvalues` buffer of every instance of
    /// `S`, after `compute_witness` (or the trace-authoring override) and any
    /// row hooks have run. Map it with the AIR's generated values struct,
    /// e.g. for a two-segment Mem chain:
    ///
    /// ```ignore
    /// .air_values::<MemSm>(|instance_idx, vals| {
    ///     let mut av = MemAirValues::new();
    ///     av.segment_id = Goldilocks::from_usize(instance_idx);
    ///     av.is_first_segment = Goldilocks::from_bool(instance_idx == 0);
    ///     // … previous_segment_* from the prior segment …
    ///     *vals = av.get_buffer();
    /// })
    /// ```
    ///
    /// Authored traces start with an *empty* buffer (the override path sets
    /// no air values), so for AIRs that have them this is how a fully valid
    /// authored instance supplies its values. A later call for the same SM
    /// replaces the earlier closure.
    pub fn air_values<S>(
        mut self,
        f: impl Fn(usize, &mut Vec<Goldilocks>) + Send + Sync + 'static,
    ) -> Self
    where
        S: UnitTestSm<Goldilocks>,
    {
        self.air_values.insert(S::air_id(), Box::new(f));
        self
    }

    /// Mutate the proof-wide values through the typed [`ZiskProofValues`]
    /// view, once per run (they are zeroed by the proofman before every
    /// run). E.g. `.proof_values(|pv| pv.enable_rom_data = Goldilocks::ONE)`.
    pub fn proof_values(
        mut self,
        f: impl for<'b> Fn(&mut ZiskProofValues<'b, Goldilocks>) + Send + Sync + 'static,
    ) -> Self {
        self.proof_values = Some(Box::new(f));
        self
    }

    /// Verify the listed global constraints (by id) in addition to the
    /// per-AIR constraints. Global constraints are skipped by default in
    /// unit-test mode because cross-SM multiplicities can never balance with
    /// only a subset of SMs fed; this opts back in for a chosen subset —
    /// e.g. the memory-continuity constraint when testing a segment chain.
    /// Use `cargo-zisk-dev get-constraints` to list the global-constraint
    /// ids. Repeated calls accumulate.
    pub fn global_constraints(mut self, ids: impl IntoIterator<Item = usize>) -> Self {
        self.global_constraints.extend(ids);
        self
    }

    /// Override the prover-default debug-info value (rarely needed). The
    /// outer `Option` selects "no debug" / "use debug"; the inner
    /// `Option<String>` is an optional path to a debug-instances JSON.
    pub fn debug(mut self, debug: Option<Option<String>>) -> Self {
        self.debug_info = debug;
        self
    }

    /// Run constraint verification. Single terminal — same path whether or
    /// not hooks are present. Returns the typed
    /// [`ConstraintsVerificationResult`]: constraint violations are data
    /// (`result.valid == false`, per-instance failures in
    /// `result.instances`), not an `Err`. `Err` means the run itself failed
    /// (setup, planning, witness computation).
    pub fn run(self) -> Result<ConstraintsVerificationResult> {
        self.prover.run_verify_input(
            self.inputs,
            self.hooks,
            self.trace_overrides,
            self.air_values,
            self.proof_values,
            self.global_constraints,
            self.debug_info,
        )
    }
}
