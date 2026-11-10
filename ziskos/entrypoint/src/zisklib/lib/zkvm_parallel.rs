//! Fork-join regions: `zkvm_parallel` runs a list of tasks as if one after another, which is
//! what a proof sees, while the emulator may run them at the same time.
//!
//! On ZisK the function is the ZisK library routine `ziskasm_zkvm_parallel`
//! (ziskasm/zisklib/zkvm/parallel.zisk), reached through its zkvmcall. Natively it runs the
//! tasks one after another, on the caller's stack.
//!
//! The tasks must be independent: a task may only write the memory it owns (`mem_start..mem_end`,
//! which holds its stack and its `out`) and memory that nobody reads afterwards before writing it
//! again, since an emulator that runs the tasks apart keeps only the writes to the memory each
//! task owns; it must not read what another task of the region writes, nor read the input, commit
//! outputs or use other host state (randomness, the Keccak-f cache) that the tasks before it would
//! have moved on; it must not open a region of its own (regions do not nest; `ziskos::par` runs a
//! region inside a task inline); and it must not end the program: a guest that ends (exits or panics) or
//! faults inside a task fails the execution, even where its sequential execution would end
//! normally.
//! Every task starts with the FCSR the region started with, and its rounding-mode changes and
//! floating-point flags end with it: after the region, the FCSR is what the caller left.
//! The driver cannot check the rest; a parallel emulation that breaks it builds traces that do not
//! match the sequential execution, and the proof fails.

/// One task of a fork-join region, as `zkvm_parallel` reads it: six 64-bit words.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ZkvmTask {
    /// Runs the task; it must follow the calling convention, as any `extern "C"` function does
    pub entry: unsafe extern "C" fn(arg: *const u8, out: *mut u8),
    /// The task's argument
    pub arg: *const u8,
    /// Where the task writes its result
    pub out: *mut u8,
    /// The top of the task's own stack, 16-byte aligned, within `mem_start..mem_end`. Unused
    /// natively
    pub stack_top: u64,
    /// The memory the task owns, `mem_start..mem_end`: its stack, its `out` and anything else it
    /// writes for after the region. An emulator that runs the task apart from the others keeps
    /// only the writes in this range, which must be page-aligned for it to do so. Unused natively
    pub mem_start: u64,
    pub mem_end: u64,
}

/// Runs `n` tasks from `tasks`, in order, each on its own stack. `ret` must be
/// [`zkvm_parallel_ret`]: each task returns to it, and it leads back into the library routine.
///
/// # Safety
///
/// `tasks` must point to `n` valid tasks, and each task's entry must be safe to call with its
/// argument and out.
#[cfg(zisk_guest)]
#[cfg_attr(not(zisk_staticlib), no_mangle)]
#[unsafe(naked)]
pub unsafe extern "C" fn zkvm_parallel(
    tasks: *const ZkvmTask,
    n: u64,
    ret: unsafe extern "C" fn(),
) {
    // The transpiler turns the csrs into a jump to the library routine, which returns straight
    // to the caller; the `ret` is never reached
    core::arch::naked_asm!(
        "csrs {id}, x0",
        "ret",
        id = const zisk_definitions::zkvmcall_id("zkvm_parallel"),
    )
}

/// Where every task of a region returns. Its zkvmcall is a static jump back into the library
/// routine of [`zkvm_parallel`]; a task cannot return there directly, since the assembly executor
/// resolves the dynamic jumps of guest code (a `ret`) only within the guest. Only
/// `zkvm_parallel` may use it.
///
/// # Safety
///
/// Never call it: pass it to [`zkvm_parallel`].
#[cfg(zisk_guest)]
#[cfg_attr(not(zisk_staticlib), no_mangle)]
#[unsafe(naked)]
pub unsafe extern "C" fn zkvm_parallel_ret() {
    // The transpiler turns the csrs into a jump into the library routine; the `ret` is never
    // reached
    core::arch::naked_asm!(
        "csrs {id}, x0",
        "ret",
        id = const zisk_definitions::zkvmcall_id("zkvm_parallel_ret"),
    )
}

/// Runs `n` tasks from `tasks`, in order, on the caller's stack. `ret` is unused natively.
///
/// # Safety
///
/// `tasks` must point to `n` valid tasks, and each task's entry must be safe to call with its
/// argument and out.
#[cfg(not(zisk_guest))]
#[cfg_attr(all(not(feature = "hints"), not(zisk_staticlib)), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_zkvm_parallel")]
pub unsafe extern "C" fn zkvm_parallel(
    tasks: *const ZkvmTask,
    n: u64,
    ret: unsafe extern "C" fn(),
) {
    let _ = ret;
    for i in 0..n as usize {
        let task = unsafe { &*tasks.add(i) };
        unsafe { (task.entry)(task.arg, task.out) };
    }
}

/// Natively the tasks return to [`zkvm_parallel`] itself; this is never reached.
///
/// # Safety
///
/// Never call it: pass it to [`zkvm_parallel`].
#[cfg(not(zisk_guest))]
#[cfg_attr(all(not(feature = "hints"), not(zisk_staticlib)), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_zkvm_parallel_ret")]
pub unsafe extern "C" fn zkvm_parallel_ret() {
    unreachable!("zkvm_parallel_ret is only a return address for zkvm_parallel's tasks");
}

#[cfg(all(test, not(zisk_guest)))]
mod tests {
    use super::*;

    unsafe extern "C" fn add_seven(arg: *const u8, out: *mut u8) {
        unsafe { *(out as *mut u64) = *(arg as *const u64) + 7 };
    }

    #[test]
    fn runs_every_task_natively() {
        let args = [1u64, 2, 3];
        let mut outs = [0u64; 3];
        let tasks: Vec<ZkvmTask> = (0..3)
            .map(|i| ZkvmTask {
                entry: add_seven,
                arg: &args[i] as *const u64 as *const u8,
                out: &mut outs[i] as *mut u64 as *mut u8,
                stack_top: 0,
                mem_start: 0,
                mem_end: 0,
            })
            .collect();
        unsafe { zkvm_parallel(tasks.as_ptr(), tasks.len() as u64, zkvm_parallel_ret) };
        assert_eq!(outs, [8, 9, 10]);
    }
}
