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

use anyhow::Result;
use data_bus::DataBusTrait;
use fields::PrimeField64;
use zisk_common::{stats_begin, stats_end, ChunkId, EmuTrace, ExecutorStatsHandle, PayloadType};
use zisk_core::ZiskRom;
use ziskemu::ZiskEmulator;

use crate::{
    pub_outs_collector::PubOutsCollector, CountersChunkMetrics, StaticDataBus, StaticSMBundle,
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
    /// Errors collected from any failed chunk task. `finalize` returns
    /// `Err` if non-empty.
    errors: Mutex<Vec<anyhow::Error>>,
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
        sm_bundle: &StaticSMBundle<F>,
        stats: &ExecutorStatsHandle,
        mt_scope_id: u64,
    ) {
        stats_begin!(stats, mt_scope_id, _chunk_scope, "MT_CHUNK_PLAYER", 0);

        let mut data_bus = match StaticDataBus::from_bundle(sm_bundle, true) {
            Ok(db) => db,
            Err(e) => {
                self.record_error(anyhow::anyhow!(
                    "StaticDataBus::from_bundle failed for chunk {}: {e}",
                    chunk_id.0
                ));
                return;
            }
        };

        ZiskEmulator::process_emu_trace::<F, _, _>(zisk_rom, emu_trace, &mut data_bus, false);
        data_bus.on_close();

        stats_end!(stats, &_chunk_scope);

        match self.results.lock() {
            Ok(mut guard) => guard.push((chunk_id, data_bus)),
            Err(e) => self.record_error(anyhow::anyhow!(
                "results lock poisoned for chunk {}: {e}",
                chunk_id.0
            )),
        }
    }

    /// Drain recorded results into chunk-indexed counters + accumulated
    /// `pub_outs`. Consumes `self`.
    ///
    /// Returns the combined error log if any chunk task recorded a
    /// failure; otherwise returns sorted-by-chunk counters and the
    /// concatenated `pub_outs`.
    pub fn finalize(self) -> Result<(CountersChunkMetrics, PubOutsCollector)> {
        let err_vec =
            self.errors.into_inner().map_err(|e| anyhow::anyhow!("errors mutex poisoned: {e}"))?;
        if !err_vec.is_empty() {
            let combined = err_vec
                .iter()
                .enumerate()
                .map(|(i, e)| format!("[Error {}] {:#}", i + 1, e))
                .collect::<Vec<_>>()
                .join("\n");
            return Err(anyhow::anyhow!(
                "MT assembly chunk processing failed ({} errors):\n{}",
                err_vec.len(),
                combined
            ));
        }

        let mut data_buses = self
            .results
            .into_inner()
            .map_err(|e| anyhow::anyhow!("results mutex poisoned: {e}"))?;

        data_buses.sort_by_key(|(chunk_id, _)| chunk_id.0);

        let mut counters: CountersChunkMetrics = HashMap::new();
        let mut pub_outs = PubOutsCollector::new();

        for (chunk_id, mut data_bus) in data_buses {
            pub_outs.0.extend(data_bus.take_pub_outs().0);
            let databus_counters = data_bus.into_devices(false);

            for (idx, counter) in databus_counters.into_iter() {
                let idx = idx.ok_or_else(|| {
                    anyhow::anyhow!("unexpected unindexed counter for chunk {}", chunk_id.0)
                })?;
                let counter = counter.ok_or_else(|| {
                    anyhow::anyhow!("secondary counter is None for chunk {} idx {idx}", chunk_id.0)
                })?;
                counters.entry(idx).or_default().push((chunk_id, counter));
            }
        }

        Ok((counters, pub_outs))
    }

    /// Helper: append an error to the internal log, silently dropping
    /// it if the mutex is poisoned (already in a failure mode).
    fn record_error(&self, err: anyhow::Error) {
        let _ = self.errors.lock().map(|mut errs| errs.push(err));
    }

    /// Test-only: directly record an error without going through the
    /// chunk-processing path. Used by `finalize` tests to verify the
    /// error-aggregation branch without needing a real bundle.
    #[cfg(test)]
    pub(crate) fn push_error_for_test(&self, err: anyhow::Error) {
        self.record_error(err);
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
        p.push_error_for_test(anyhow::anyhow!("boom-chunk-42"));
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
        p.push_error_for_test(anyhow::anyhow!("first-fail"));
        p.push_error_for_test(anyhow::anyhow!("second-fail"));
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
