use ziskos::zisklib::{
    FCALL_PAR_BOUNDARY_ID, FCALL_PAR_FORK_ID, FCALL_PAR_JOIN_ID, FCALL_PAR_TASK_END_ID,
};

use crate::{InstContext, CHUNK_SIZE, REGS_IN_MAIN_TOTAL_NUMBER};

// In the worst case, we divide a 16.384-bit number (8.192 * 2)
// by an 8.192-bit number. We must also include the length fields.
// This results in a total of 1 + 256 + 1 + 128 = 386 u64 parameters.
pub const FCALL_PARAMS_MAX_SIZE: usize = 386;
// In the worst case, we compute the binary decomposition of a 8192-bit number
// This results in 1 + 8192 = 8193 u64 results.
pub const FCALL_RESULT_MAX_SIZE: usize = 8193;

/// Whether `function_id` is one of the fork-join markers that return nothing: fork, boundary,
/// task end and join. The padding fcall (`FCALL_PAR_PAD_ID`) returns a word and is not one of them.
pub fn is_par_marker(function_id: u64) -> bool {
    [FCALL_PAR_FORK_ID, FCALL_PAR_BOUNDARY_ID, FCALL_PAR_TASK_END_ID, FCALL_PAR_JOIN_ID]
        .iter()
        .any(|&id| id as u64 == function_id)
}

/// The result of the padding fcall issued at step `step`: how many steps are left in its chunk
/// after it, 0 to `CHUNK_SIZE - 1`.
///
/// Chunks are counted on the `CHUNK_SIZE` grid from step 0, whatever chunk size an execution uses,
/// so that every emulator and every assembly service returns the same value for the same step: the
/// value decides how many steps the guest's padding takes, and they must all run the same
/// instructions. A smaller power-of-two chunk size still sees the padding end on one of its
/// boundaries.
pub fn par_pad_steps(step: u64) -> u64 {
    // CHUNK_SIZE is a power of two, so this is (CHUNK_SIZE - 1) - (step % CHUNK_SIZE)
    !step & (CHUNK_SIZE - 1)
}

/// The register that holds the task index in a fork-join region's loop (s11, x27): the only
/// one in which the passes through the loop differ (see ziskasm/zisklib/zkvm/parallel.zisk).
/// `PAR_TASK_INDEX_REG` in emulator-asm/src/par.c too.
const PAR_TASK_INDEX_REG: usize = 27;

/// The fork-join region an emulation is in, followed to check, at every pass through the
/// region's task loop, the invariant the parallel emulation relies on: each pass starts on a
/// chunk boundary and in the state of the first pass but for the task index, so that any task
/// can be started from the first pass's state.
#[derive(Debug, Clone)]
pub struct ParRegion {
    /// Number of tasks, from the fork
    tasks: u64,
    /// The state at the start of the first pass, once it has started
    first: Option<ParPassState>,
    /// Passes started so far
    passes: u64,
}

/// What a pass through a region's task loop starts with that a chunk's start records: the
/// registers, c and sp
#[derive(Debug, Clone, PartialEq)]
struct ParPassState {
    regs: [u64; REGS_IN_MAIN_TOTAL_NUMBER],
    c: u64,
    sp: u64,
}

/// Follows a fork-join marker (fork, boundary, task end or join) in an emulation that runs the
/// tasks one after another, checking the region's invariant. `c` is the value of c before the
/// marker's instruction.
///
/// Panics, saying what broke, when the guest's region does not keep it: a parallel emulation
/// would stitch traces that do not match the sequential one, and the proof would fail.
pub fn par_marker(ctx: &mut InstContext, function_id: u64, c: u64) {
    let step = ctx.step;
    if function_id == FCALL_PAR_FORK_ID as u64 {
        assert!(ctx.par.is_none(), "fork-join: a region starts at step {step} inside another");
        assert_eq!(
            ctx.fcall.parameters_size, 2,
            "fork-join: the fork at step {step} takes 2 parameters (tasks, n)"
        );
        ctx.par = Some(ParRegion { tasks: ctx.fcall.parameters[1], first: None, passes: 0 });
    } else if function_id == FCALL_PAR_BOUNDARY_ID as u64 {
        let state = ParPassState { regs: ctx.regs, c, sp: ctx.sp };
        let Some(region) = ctx.par.as_mut() else {
            panic!("fork-join: a pass starts at step {step} outside a region");
        };
        let pass = region.passes;
        assert_eq!(
            step % CHUNK_SIZE,
            0,
            "fork-join: pass {pass} starts at step {step}, not on a chunk boundary"
        );
        assert!(
            pass <= region.tasks,
            "fork-join: pass {pass} starts at step {step}, but there are {} tasks",
            region.tasks
        );
        match &region.first {
            None => {
                assert_eq!(
                    state.regs[PAR_TASK_INDEX_REG], 0,
                    "fork-join: the first pass starts at step {step} with task index {}",
                    state.regs[PAR_TASK_INDEX_REG]
                );
                region.first = Some(state);
            }
            Some(first) => {
                let mut expected = first.clone();
                expected.regs[PAR_TASK_INDEX_REG] = pass;
                if state != expected {
                    let mut diffs: Vec<String> = (0..REGS_IN_MAIN_TOTAL_NUMBER)
                        .filter(|&r| state.regs[r] != expected.regs[r])
                        .map(|r| {
                            format!("r{r} = {:#x}, not {:#x}", state.regs[r], expected.regs[r])
                        })
                        .collect();
                    if state.c != expected.c {
                        diffs.push(format!("c = {:#x}, not {:#x}", state.c, expected.c));
                    }
                    if state.sp != expected.sp {
                        diffs.push(format!("sp = {:#x}, not {:#x}", state.sp, expected.sp));
                    }
                    panic!(
                        "fork-join: pass {pass} starts at step {step} in another state than the \
                         first pass: {}",
                        diffs.join(", ")
                    );
                }
            }
        }
        region.passes += 1;
    } else if function_id == FCALL_PAR_TASK_END_ID as u64 {
        assert!(ctx.par.is_some(), "fork-join: a task ends at step {step} outside a region");
    } else if function_id == FCALL_PAR_JOIN_ID as u64 {
        let Some(region) = ctx.par.take() else {
            panic!("fork-join: a region ends at step {step} without starting");
        };
        assert_eq!(
            region.passes,
            region.tasks + 1,
            "fork-join: the region ending at step {step} made {} passes, not one per task and \
             the last",
            region.passes
        );
    } else {
        panic!("fork-join: {function_id} is not a fork-join marker");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ziskos::zisklib::{FCALL_INPUT_READY_ID, FCALL_PAR_PAD_ID};

    #[test]
    fn pad_steps_counts_the_steps_left_in_the_chunk() {
        assert_eq!(par_pad_steps(0), CHUNK_SIZE - 1, "the first step of a chunk");
        assert_eq!(par_pad_steps(5), CHUNK_SIZE - 6);
        assert_eq!(par_pad_steps(CHUNK_SIZE - 1), 0, "the last step of a chunk");
        assert_eq!(par_pad_steps(CHUNK_SIZE), CHUNK_SIZE - 1, "the first step of the next chunk");
        assert_eq!(par_pad_steps(7 * CHUNK_SIZE + 9), CHUNK_SIZE - 10);
        // Padding by the returned amount always lands the next step on a chunk boundary
        for step in [0, 1, 1234, CHUNK_SIZE - 2, CHUNK_SIZE - 1, 3 * CHUNK_SIZE + 17] {
            assert_eq!((step + 1 + par_pad_steps(step)) % CHUNK_SIZE, 0, "step {step}");
        }
    }

    #[test]
    fn the_markers_are_exactly_the_four_that_return_nothing() {
        for id in
            [FCALL_PAR_FORK_ID, FCALL_PAR_BOUNDARY_ID, FCALL_PAR_TASK_END_ID, FCALL_PAR_JOIN_ID]
        {
            assert!(is_par_marker(id as u64), "id {id}");
        }
        assert!(!is_par_marker(FCALL_PAR_PAD_ID as u64), "the padding fcall returns a word");
        assert!(!is_par_marker(FCALL_INPUT_READY_ID as u64));
        assert!(!is_par_marker(0));
    }
}
