use std::any::Any;
use std::collections::HashMap;

use anyhow::Result;
use executor::unit_test_hooks::UnitTestHookBag;
use executor::unit_test_trace_override::TraceOverrideBag;
use fields::Goldilocks;
use zisk_common::UnitTestSm;

use crate::{UnitTestProver, VerifyConstraintsOutput};

/// Per-SM batch of type-erased inputs (each a boxed `S::Input`).
type ErasedInputs = HashMap<String, Vec<Box<dyn Any + Send + Sync>>>;

/// Fluent builder. Construct via [`UnitTestProver::verify_input`].
pub struct VerifyInput<'a> {
    prover: &'a UnitTestProver,
    inputs: ErasedInputs,
    hooks: UnitTestHookBag,
    trace_overrides: TraceOverrideBag,
    debug_info: Option<Option<String>>,
}

impl<'a> VerifyInput<'a> {
    pub(crate) fn new(prover: &'a UnitTestProver) -> Self {
        Self {
            prover,
            inputs: HashMap::new(),
            hooks: UnitTestHookBag::new(),
            trace_overrides: TraceOverrideBag::new(),
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
        self.hooks = std::mem::replace(&mut self.hooks, UnitTestHookBag::new()).with::<S>(hook);
        self
    }

    /// Author SM `S`'s trace directly, bypassing `compute_witness` for that
    /// AIR id. No `.input()` is needed: the executor plans one instance for
    /// the AIR from its fixed shape. The closure receives the freshly
    /// allocated, zeroed typed trace `&mut Trace` and the shared
    /// [`Std`](pil_std_lib::Std) handle — call into it (range checks,
    /// lookups) to emit the side-effects `compute_witness` would, so a trace
    /// can be authored as fully valid, or as invalid in exactly one chosen
    /// way. `Trace` is the SM's concrete trace type (e.g.
    /// `KeccakfTrace<KeccakfTraceRow<Goldilocks>>`). The SM must have a
    /// `unit_test_trace_override!` impl registered.
    ///
    /// A later call for the same SM replaces the earlier author closure.
    pub fn trace<S, Trace>(
        mut self,
        author: impl Fn(&mut Trace, &pil_std_lib::Std<Goldilocks>) -> proofman_common::ProofmanResult<()>
            + Send
            + Sync
            + 'static,
    ) -> Self
    where
        S: UnitTestSm<Goldilocks>,
        Trace: 'static,
    {
        self.trace_overrides =
            std::mem::replace(&mut self.trace_overrides, TraceOverrideBag::new())
                .with::<S, Trace>(author);
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
    /// not hooks are present. Returns `Ok` when all per-AIR constraints
    /// hold and `Err` otherwise (including the case where an injected hook
    /// breaks a constraint).
    pub fn run(self) -> Result<VerifyConstraintsOutput> {
        self.prover.run_verify_input(self.inputs, self.hooks, self.trace_overrides, self.debug_info)
    }
}
