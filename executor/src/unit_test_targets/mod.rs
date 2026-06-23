use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use fields::Goldilocks;
use zisk_common::{DynTraceOverride, DynUnitTestSm};

use crate::{
    register_builtin_unit_tests, StateMachines, StaticSMBundle, BUILTIN_UNIT_TEST_MARKERS,
    BUILTIN_UNIT_TEST_OVERRIDES, PRECOMPILE_UNIT_TEST_MARKERS, PRECOMPILE_UNIT_TEST_OVERRIDES,
};

/// Coerce `&Arc<T>` to `Arc<dyn Any + Send + Sync>` (the intermediate binding
/// keeps `Arc::clone`'s generic from resolving to the trait object). Shared by
/// the built-in (`builtin_unit_tests!`) and precompile (`register_precompiles!`)
/// generated registry code.
pub(crate) fn erase<T: Any + Send + Sync + 'static>(arc: &Arc<T>) -> Arc<dyn Any + Send + Sync> {
    let cloned: Arc<T> = arc.clone();
    cloned
}

/// Every unit-test SM marker, built-ins followed by precompiles. Both halves
/// are generated from the declarative SM lists — see
/// [`crate::sm::builtins::unit_tests`] and `register_precompiles!`. Order
/// doesn't matter; the executor looks SMs up by AIR id or name.
fn registry() -> impl Iterator<Item = &'static dyn DynUnitTestSm<Goldilocks>> {
    BUILTIN_UNIT_TEST_MARKERS.iter().chain(PRECOMPILE_UNIT_TEST_MARKERS).copied()
}

/// Every raw trace-authoring override builder, built-ins followed by
/// precompiles. One per marker (see [`crate::unit_test_trace_override`]).
fn override_registry() -> impl Iterator<Item = &'static dyn DynTraceOverride<Goldilocks>> {
    BUILTIN_UNIT_TEST_OVERRIDES.iter().chain(PRECOMPILE_UNIT_TEST_OVERRIDES).copied()
}

/// Look up an SM in the registry by AIR id.
pub fn lookup_by_air_id(air_id: usize) -> Option<&'static dyn DynUnitTestSm<Goldilocks>> {
    registry().find(|s| s.air_id() == air_id)
}

/// Look up an SM in the registry by name.
pub fn lookup_by_name(name: &str) -> Option<&'static dyn DynUnitTestSm<Goldilocks>> {
    registry().find(|s| s.name() == name)
}

/// Look up a trace-override builder by AIR id. `None` means the SM has no
/// override support, so the executor takes the normal `compute_witness` path.
pub fn lookup_override_by_air_id(
    air_id: usize,
) -> Option<&'static dyn DynTraceOverride<Goldilocks>> {
    override_registry().find(|s| s.air_id() == air_id)
}

/// Build the AIR-id → erased-inner-SM map from a `StaticSMBundle`. Each AIR id
/// maps to its specific inner SM (the actual witness producer), not to the
/// orchestrator that bundled them at construction time.
///
/// The per-SM wiring lives in the two declarative lists: built-ins in
/// [`crate::sm::builtins::unit_tests`] (`builtin_unit_tests!`) and precompiles
/// in `register_precompiles!`. Each emits a `register_*_unit_tests` arm that
/// inserts its inner SMs here; this function just walks the bundle and
/// dispatches.
pub fn build_manager_registry(
    bundle: &StaticSMBundle<Goldilocks>,
) -> HashMap<usize, Arc<dyn Any + Send + Sync>> {
    let mut map: HashMap<usize, Arc<dyn Any + Send + Sync>> = HashMap::new();

    for (_, sm) in bundle.iter_sms() {
        match sm {
            StateMachines::Builtin(b) => register_builtin_unit_tests(&mut map, b),
            StateMachines::Precompile(p) => p.register_unit_tests(&mut map),
        }
    }

    map
}
