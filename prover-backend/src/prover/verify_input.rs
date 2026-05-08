use std::collections::HashMap;

use anyhow::Result;
use executor::unit_test_hooks::UnitTestHookBag;
use fields::Goldilocks;
use zisk_common::UnitTestSm;

use crate::{UnitTestProver, VerifyConstraintsOutput};

/// Fluent builder. Construct via [`UnitTestProver::verify_input`].
pub struct VerifyInput<'a> {
    prover: &'a UnitTestProver,
    inputs: HashMap<String, Vec<serde_json::Value>>,
    hooks: UnitTestHookBag,
    debug_info: Option<Option<String>>,
}

impl<'a> VerifyInput<'a> {
    pub(crate) fn new(prover: &'a UnitTestProver) -> Self {
        Self { prover, inputs: HashMap::new(), hooks: UnitTestHookBag::new(), debug_info: None }
    }

    /// Add one input for state-machine `S`. Repeated calls accumulate.
    ///
    /// Internally serialises the typed input with `Serialize` so it shares
    /// the same wire format as the CLI JSON path (which keeps the trait
    /// registry as the single source of truth for parsing semantics).
    pub fn input<S>(mut self, input: S::Input) -> Self
    where
        S: UnitTestSm<Goldilocks>,
    {
        let value = serde_json::to_value(input).expect("UnitTestSm::Input must Serialize cleanly");
        self.inputs.entry(S::name().to_string()).or_default().push(value);
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
            let value =
                serde_json::to_value(inp).expect("UnitTestSm::Input must Serialize cleanly");
            bucket.push(value);
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
        self.prover.run_verify_input(self.inputs, self.hooks, self.debug_info)
    }
}
