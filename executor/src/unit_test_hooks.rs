use std::collections::HashMap;

use fields::Goldilocks;
use proofman_common::trace::TraceRow;

use zisk_common::UnitTestSm;

type ErasedHook = Box<dyn Fn(usize, &mut [Goldilocks]) + Send + Sync + 'static>;

/// One registered hook plus the geometry the dispatcher needs to slice
/// the flat `Vec<F>` buffer into row-sized chunks.
pub(crate) struct HookSlot {
    pub(crate) row_size: usize,
    pub(crate) apply: ErasedHook,
}

/// A bag of per-SM trace-row hooks keyed by AIR id.
///
/// Built via the chainable [`UnitTestHookBag::with`] method, e.g.:
///
/// ```ignore
/// UnitTestHookBag::new()
///     .with::<BinarySm>(|input_idx, clock, row| {
///         if input_idx == 0 { row.set_a_op(F::from_u64(99)); }
///     })
/// ```
pub struct UnitTestHookBag {
    pub(crate) hooks: HashMap<usize /* air_id */, HookSlot>,
}

impl Default for UnitTestHookBag {
    fn default() -> Self {
        Self::new()
    }
}

impl UnitTestHookBag {
    /// Create an empty hook bag (no hooks registered).
    pub fn new() -> Self {
        Self { hooks: HashMap::new() }
    }

    /// Register a hook for SM `S`.
    ///
    /// The closure operates on the typed row struct `S::Row` and can call
    /// any of the macro-generated `set_<col>` / `get_<col>` methods.
    /// `(input_idx, clock)` is computed from the absolute row index using
    /// `S::rows_per_input()` — for variable-row SMs this just gives a
    /// uniform partition; the closure can recompute from `clock` if needed.
    ///
    /// **Repeated calls for the same SM stack** in registration order: an
    /// earlier hook runs first and writes to the row, the next hook sees
    /// the post-mutation state. Cross-SM hooks are independent (each AIR
    /// id has its own composed chain).
    ///
    /// Named `with` (not `add`) so clippy doesn't confuse it with
    /// `std::ops::Add::add`.
    pub fn with<S>(
        mut self,
        hook: impl Fn(usize, usize, &mut S::Row) + Send + Sync + 'static,
    ) -> Self
    where
        S: UnitTestSm<Goldilocks>,
    {
        let row_size = <S::Row as TraceRow>::ROW_SIZE;
        let rpi = S::rows_per_input().max(1);
        let new_hook: ErasedHook = Box::new(move |row_idx, raw| {
            // SAFETY:
            //   - `raw.len() == row_size`, guaranteed by the dispatcher
            //     which slices the AIR-instance trace by `row_size`.
            //   - The trace was just written by the SM using the same
            //     `<X>TraceRow<Goldilocks>` layout we cast back to.
            //   - `Goldilocks` is `repr(transparent)` over `u64`, alignment
            //     8, matching the row struct's alignment.
            let row: &mut S::Row = unsafe { &mut *(raw.as_mut_ptr() as *mut S::Row) };
            let input_idx = row_idx / rpi;
            let clock = row_idx % rpi;
            hook(input_idx, clock, row);
        });

        let air_id = S::air_id();
        let combined = match self.hooks.remove(&air_id) {
            Some(prev_slot) => {
                debug_assert_eq!(
                    prev_slot.row_size, row_size,
                    "UnitTestHookBag: row_size mismatch for air_id {air_id}",
                );
                let prev_apply = prev_slot.apply;
                Box::new(move |row_idx, raw: &mut [Goldilocks]| {
                    prev_apply(row_idx, raw);
                    new_hook(row_idx, raw);
                }) as ErasedHook
            }
            None => new_hook,
        };
        self.hooks.insert(air_id, HookSlot { row_size, apply: combined });
        self
    }

    /// True if no hooks are registered. Used by the executor to skip the
    /// dispatcher cleanly when no injection is requested.
    pub fn is_empty(&self) -> bool {
        self.hooks.is_empty()
    }
}
