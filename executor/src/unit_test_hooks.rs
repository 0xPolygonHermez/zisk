use std::collections::HashMap;

use fields::Goldilocks;
use proofman_common::trace::TraceRow;

use zisk_common::UnitTestSm;

type ErasedHook = Box<dyn Fn(usize, &mut [Goldilocks]) + Send + Sync + 'static>;

/// The registered hooks for one AIR id, plus the geometry the dispatcher
/// needs to slice the flat `Vec<F>` buffer into row-sized chunks.
pub(crate) struct HookSlot {
    pub(crate) row_size: usize,
    pub(crate) hooks: Vec<ErasedHook>,
}

/// A bag of per-SM trace-row hooks keyed by AIR id. Register hooks with
/// [`UnitTestHookBag::register`]; hooks for the same SM run in registration
/// order (each sees the previous one's mutations).
#[derive(Default)]
pub struct UnitTestHookBag {
    pub(crate) hooks: HashMap<usize /* air_id */, HookSlot>,
}

impl UnitTestHookBag {
    /// Create an empty hook bag (no hooks registered).
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a hook for SM `S`.
    ///
    /// The closure operates on the typed row struct `S::Row` and can call
    /// any of the macro-generated `set_<col>` / `get_<col>` methods.
    /// `(input_idx, clock)` is computed from the absolute row index using
    /// `S::rows_per_input()` — for variable-row SMs this just gives a
    /// uniform partition; the closure can recompute from `clock` if needed.
    pub fn register<S>(&mut self, hook: impl Fn(usize, usize, &mut S::Row) + Send + Sync + 'static)
    where
        S: UnitTestSm<Goldilocks>,
    {
        let row_size = <S::Row as TraceRow>::ROW_SIZE;
        let rpi = S::rows_per_input().max(1);
        let erased: ErasedHook = Box::new(move |row_idx, raw| {
            // SAFETY:
            //   - `raw.len() == row_size`, guaranteed by the dispatcher
            //     which slices the AIR-instance trace by `row_size`.
            //   - The trace was just written by the SM using the same
            //     `<X>TraceRow<Goldilocks>` layout we cast back to.
            //   - `Goldilocks` is `repr(transparent)` over `u64`, alignment
            //     8, matching the row struct's alignment.
            let row: &mut S::Row = unsafe { &mut *(raw.as_mut_ptr() as *mut S::Row) };
            hook(row_idx / rpi, row_idx % rpi, row);
        });

        let slot = self
            .hooks
            .entry(S::air_id())
            .or_insert_with(|| HookSlot { row_size, hooks: Vec::new() });
        debug_assert_eq!(
            slot.row_size,
            row_size,
            "UnitTestHookBag: row_size mismatch for air_id {}",
            S::air_id(),
        );
        slot.hooks.push(erased);
    }

    /// True if no hooks are registered. Used by the executor to skip the
    /// dispatcher cleanly when no injection is requested.
    pub fn is_empty(&self) -> bool {
        self.hooks.is_empty()
    }
}
