//! [`MtChunkProcessor`] — per-chunk MT replay state + post-processing.
//!
//! `EmulatorAsm::run_mt_assembly` previously inlined ~120 lines that:
//!   1. spawned a rayon task per chunk that built a per-chunk
//!      [`crate::StaticDataBus`], replayed the chunk's `EmuTrace`
//!      against it, closed it, and pushed the bus onto a shared `Vec`;
//!   2. collected errors from any failed task into a side `Vec`;
//!   3. after the scope ended, sorted the per-chunk results by
//!      `ChunkId`, drained `pub_outs` from each bus, and flattened
//!      `into_devices` into `CountersChunkMetrics`.
//!
//! This module pulls steps 1–3 out so:
//!   * the MT entry point in [`crate::EmulatorAsm`] becomes a thin
//!     orchestrator over an `AsmRunnerSupervisor`, an
//!     `AsmRunnerMT::run_and_count` call, and this processor;
//!   * the per-chunk dispatch and the post-process aggregation can be
//!     reasoned about — and partially tested — without bringing up
//!     shmem.
//!
//! See `.claude/executor_refactor_plan.md` step 2.4 for context.

#![cfg_attr(not(all(target_os = "linux", target_arch = "x86_64")), allow(dead_code))]

use std::collections::HashMap;
use std::sync::Mutex;

use data_bus::DataBusTrait;
use fields::PrimeField64;
use zisk_common::{stats_begin, stats_end, ChunkId, EmuTrace, ExecutorStatsHandle, PayloadType};
use zisk_core::ZiskRom;
use ziskemu::ZiskEmulator;

use crate::{
    error::{ExecutorError, ExecutorResult},
    pub_outs_collector::PubOutsCollector,
    CountersChunkMetrics, StaticDataBus,
};

/// Stateful accumulator for the per-chunk MT replay phase.
///
/// One `MtChunkProcessor<F>` covers a single
/// `AsmRunnerMT::run_and_count` invocation:
///   * during the scope, rayon tasks call
///     [`Self::process_chunk`] once per chunk;
///   * after the scope, the caller calls [`Self::finalize`] to drain
///     the collected per-chunk buses into chunk-indexed counters and
///     accumulated `pub_outs`.
pub struct MtChunkProcessor<F: PrimeField64> {
    /// Per-chunk databuses, pushed in arbitrary order. `finalize` sorts
    /// by `ChunkId` before draining.
    results: Mutex<Vec<(ChunkId, StaticDataBus<PayloadType, F>)>>,
    /// Pre-formatted error messages collected from any failed chunk
    /// task. `finalize` returns `Err` if non-empty.
    errors: Mutex<Vec<String>>,
}

impl<F: PrimeField64> MtChunkProcessor<F> {
    /// Fresh processor with no recorded chunks or errors.
    pub fn new() -> Self {
        Self { results: Mutex::new(Vec::new()), errors: Mutex::new(Vec::new()) }
    }

    /// Process a single chunk: build a per-chunk bus, replay the
    /// chunk's `EmuTrace` against it, close it, and record the bus.
    ///
    /// Errors are captured in the internal error log and do not panic
    /// or propagate — the rayon scope must complete so the caller can
    /// drain via [`Self::finalize`]. The `mt_scope_id` is the parent
    /// `MT_ASSEMBLY` stats scope; per-chunk stats nest under it.
    #[allow(unused_variables)] // `stats` / `mt_scope_id` only used when the `stats` feature is on
    pub fn process_chunk(
        &self,
        chunk_id: ChunkId,
        emu_trace: &EmuTrace,
        zisk_rom: &ZiskRom,
        stats: &ExecutorStatsHandle,
        mt_scope_id: u64,
    ) {
        stats_begin!(stats, mt_scope_id, _chunk_scope, "MT_CHUNK_PLAYER", 0);

        let mut data_bus = StaticDataBus::<_, F>::build(true);

        ZiskEmulator::process_emu_trace::<F, _, _>(zisk_rom, emu_trace, &mut data_bus, false);
        data_bus.on_close();

        stats_end!(stats, &_chunk_scope);

        match self.results.lock() {
            Ok(mut guard) => guard.push((chunk_id, data_bus)),
            Err(e) => {
                self.record_error(format!("results lock poisoned for chunk {}: {e}", chunk_id.0))
            }
        }
    }

    /// Drain recorded results into chunk-indexed counters + accumulated
    /// `pub_outs`. Consumes `self`.
    ///
    /// Returns the combined error log if any chunk task recorded a
    /// failure; otherwise returns sorted-by-chunk counters and the
    /// concatenated `pub_outs`.
    pub fn finalize(self) -> ExecutorResult<(CountersChunkMetrics, PubOutsCollector)> {
        let err_vec = self
            .errors
            .into_inner()
            .map_err(|_| ExecutorError::mutex_poisoned("mt_chunk_errors"))?;
        if !err_vec.is_empty() {
            let message = err_vec
                .iter()
                .enumerate()
                .map(|(i, e)| format!("[Error {}] {e}", i + 1))
                .collect::<Vec<_>>()
                .join("\n");
            return Err(ExecutorError::MtChunkProcessing { count: err_vec.len(), message });
        }

        let mut data_buses = self
            .results
            .into_inner()
            .map_err(|_| ExecutorError::mutex_poisoned("mt_chunk_results"))?;

        data_buses.sort_by_key(|(chunk_id, _)| chunk_id.0);

        let mut counters: CountersChunkMetrics = HashMap::new();
        let mut pub_outs = PubOutsCollector::new();

        for (chunk_id, mut data_bus) in data_buses {
            pub_outs.0.extend(data_bus.take_pub_outs().0);
            let databus_counters = data_bus.into_devices(false);

            for (idx, counter) in databus_counters.into_iter() {
                counters.entry(idx).or_default().push((chunk_id, counter));
            }
        }

        Ok((counters, pub_outs))
    }

    /// Helper: append a pre-formatted error message to the internal
    /// log, silently dropping it if the mutex is poisoned (already in
    /// a failure mode).
    fn record_error(&self, message: String) {
        let _ = self.errors.lock().map(|mut errs| errs.push(message));
    }

    /// Test-only: directly record an error without going through the
    /// chunk-processing path. Used by `finalize` tests to verify the
    /// error-aggregation branch without needing a real bundle.
    #[cfg(test)]
    pub(crate) fn push_error_for_test(&self, message: String) {
        self.record_error(message);
    }
}

impl<F: PrimeField64> Default for MtChunkProcessor<F> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fields::Goldilocks;

    /// Concrete F used for tests that need a placeholder field type.
    /// MtChunkProcessor uses F only through the `StaticDataBus`
    /// generic argument; the empty-state and error-state tests don't
    /// actually exercise databus construction.
    type F = Goldilocks;

    #[test]
    fn new_processor_finalizes_to_empty() {
        let p: MtChunkProcessor<F> = MtChunkProcessor::new();
        let (counters, pub_outs) = p.finalize().expect("empty processor finalizes Ok");
        assert!(counters.is_empty(), "no chunks recorded → no counters");
        assert!(pub_outs.0.is_empty(), "no chunks recorded → no pub_outs");
    }

    #[test]
    fn default_matches_new() {
        let p: MtChunkProcessor<F> = MtChunkProcessor::default();
        let (counters, pub_outs) = p.finalize().expect("default ok");
        assert!(counters.is_empty());
        assert!(pub_outs.0.is_empty());
    }

    #[test]
    fn single_error_propagates_through_finalize() {
        let p: MtChunkProcessor<F> = MtChunkProcessor::new();
        p.push_error_for_test("boom-chunk-42".to_string());
        // The Ok variant of `finalize` contains `Box<dyn BusDeviceMetrics>`
        // which doesn't implement `Debug`, so we can't use `expect_err`.
        match p.finalize() {
            Ok(_) => panic!("single error must surface"),
            Err(err) => {
                let msg = err.to_string();
                assert!(msg.contains("MT assembly chunk processing failed (1 errors)"));
                assert!(msg.contains("boom-chunk-42"));
            }
        }
    }

    #[test]
    fn multiple_errors_combine_with_indexed_prefixes() {
        let p: MtChunkProcessor<F> = MtChunkProcessor::new();
        p.push_error_for_test("first-fail".to_string());
        p.push_error_for_test("second-fail".to_string());
        match p.finalize() {
            Ok(_) => panic!("multiple errors must surface"),
            Err(err) => {
                let msg = err.to_string();
                assert!(msg.contains("(2 errors)"));
                assert!(msg.contains("[Error 1] first-fail"));
                assert!(msg.contains("[Error 2] second-fail"));
            }
        }
    }

    // Note: process_chunk's happy path (build databus, replay,
    // collect counters) requires a real StaticSMBundle, which in turn
    // requires Std::new(ProofCtx, SetupCtx, ...). That setup is
    // integration-test territory; we cover it there.
}
