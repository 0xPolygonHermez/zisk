use std::collections::HashMap;
use std::sync::{Arc, PoisonError, RwLock};

use zisk_asm_runner::{AsmRunnerRH, SegmentsClaim};
use zisk_common::LateValue;

use super::AsmResources;
use crate::state::ChunkCollector;

/// The executor's collector store, shared with the job that may have to drop it.
type Collectors = Arc<RwLock<HashMap<usize, Vec<Option<ChunkCollector>>>>>;

/// One job on the assembly backend, holding the shared segments from its first write to
/// them until dropped. Begun by [`ZiskExecutor::begin_asm_job`].
///
/// Every program a process sets up on a rank in a hints mode shares one set of segments,
/// so a job holds them for its whole length: register its streams and run its execution
/// while this is alive, and keep it until the job's results have been read.
///
/// Dropping it ends the job: it joins the ROM-histogram runner the job left parked, rewinds
/// the input and hints for the next job, and releases the segments. The runner goes first
/// for two reasons. The next job's switch waits for the outgoing program's services with a
/// ping, which queues behind a request already sent but not behind one the runner has yet
/// to send; and the rewind would strand a runner still reading. A runner is parked only
/// once the minimal trace has run, and only when its service is fed, so it finishes.
///
/// When the job's execution lent the memory-operations output to its collectors
/// ([`zisk_common::lent_memory`]), the drop also drops the collectors and takes the output back
/// before releasing the segments: the next job's run writes those pages. A buffer still alive
/// there stops the process (`collect_budget::take_back_lent_memory`).
///
/// [`ZiskExecutor::begin_asm_job`]: crate::ZiskExecutor::begin_asm_job
#[must_use = "dropping the job releases the shared segments at once"]
pub struct AsmJob {
    resources: Arc<AsmResources>,
    /// The cell the job's ROM-histogram runner is parked in; `None` for an executor that
    /// computes no witness, which runs none.
    rom_histogram: Option<Arc<LateValue<AsmRunnerRH>>>,
    /// The executor's collectors, dropped at the job's end when they may hold lent memory.
    collectors: Collectors,
    /// Released after the drop has ended the job: fields drop after `Drop::drop`.
    _claim: SegmentsClaim,
}

impl AsmJob {
    pub(crate) fn new(
        resources: Arc<AsmResources>,
        rom_histogram: Option<Arc<LateValue<AsmRunnerRH>>>,
        collectors: Collectors,
        claim: SegmentsClaim,
    ) -> Self {
        Self { resources, rom_histogram, collectors, _claim: claim }
    }
}

impl Drop for AsmJob {
    fn drop(&mut self) {
        if let Some(rom_histogram) = &self.rom_histogram {
            rom_histogram.drain();
        }
        // Before the segments are released: the next job's memory-operations run writes the pages
        // lent to this job's collectors, so none may outlive it.
        if zisk_common::lent_memory::is_lent() {
            self.collectors.write().unwrap_or_else(PoisonError::into_inner).clear();
            crate::witness::collect_budget::take_back_lent_memory();
        }
        self.resources.reset();
    }
}

impl std::fmt::Debug for AsmJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AsmJob").field("resources", &self.resources).finish_non_exhaustive()
    }
}
