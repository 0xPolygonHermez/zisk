//! Fork-join regions for Rust guests: [`run`] runs closures as if one after another, which is
//! what a proof sees, while the emulator may run them at the same time (see
//! [`zkvm_parallel`](crate::zisklib::zkvm_parallel)).
//!
//! Each task gets an arena of its own, carved out of the caller's heap before the region: a slot
//! with its closure and its result, then its heap, then a canary, then its stack. A task allocates
//! only in its arena, so the tasks are independent as far as the allocator goes. The closures must
//! be too. A task:
//! - reads anything built before the region;
//! - must not read what another task of the region writes;
//! - must not write memory outside its arena that anything after it reads, a later task or the
//!   code after the region: run apart, a task keeps only the writes to its own arena. That
//!   includes changing in place data built before the region, such as a `Vec` moved into its
//!   closure or a slice of a shared buffer. Return what the task computes instead;
//! - must not read the input or commit outputs (`crate::io`), which panics inside a task, nor use
//!   host randomness or the Keccak-f cache;
//! - must not end the program: a closure that panics (or exits) fails the execution.
//!
//! Closures and results must be `Send`, which rules out sharing state through `Cell`, `RefCell` or
//! `Rc`. A [`run`] or [`join`] inside a task runs its jobs inline, one after another: regions do
//! not nest.
//!
//! A result may own memory the task allocated (a `Vec`, a `Box`): the arenas are never reused,
//! so it stays valid after the region.
//!
//! Natively, and on ZisK with an allocator other than the default bump allocator, the closures
//! simply run one after another.

/// The size of each task's arena
#[derive(Clone, Copy, Debug)]
pub struct Config {
    /// Bytes each task can allocate
    pub heap_bytes: usize,
    /// Bytes of each task's stack: enough for the task's deepest call chain (see [`run`])
    pub stack_bytes: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self { heap_bytes: 16 << 20, stack_bytes: 256 << 10 }
    }
}

/// Whether [`run`] runs its closures as a fork-join region on ZisK: only with the default bump
/// allocator, whose state a task can point at its own arena
#[cfg(zisk_guest)]
const FORK_JOIN_REGION: bool = cfg!(not(any(
    feature = "zisk-embedded-alloc",
    feature = "zisk-custom-alloc",
    feature = "zisk-embedded-dlmalloc-alloc",
    feature = "zisk-embedded-talc-alloc",
    feature = "zisk-embedded-tlfs-alloc"
)));

#[cfg(zisk_guest)]
use crate::alloc_extern::vec::Vec;

/// Whether the code runs inside a task of a fork-join region. Set by the caller of a region before
/// it opens and cleared after it closes, so tasks only ever read it: run apart or in order,
/// every task sees it set
#[cfg(zisk_guest)]
static IN_TASK: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

#[cfg(zisk_guest)]
fn in_task() -> bool {
    IN_TASK.load(core::sync::atomic::Ordering::Relaxed)
}

#[cfg(zisk_guest)]
fn set_in_task(value: bool) {
    IN_TASK.store(value, core::sync::atomic::Ordering::Relaxed);
}

#[cfg(not(zisk_guest))]
std::thread_local! {
    static IN_TASK: core::cell::Cell<bool> = const { core::cell::Cell::new(false) };
}

#[cfg(not(zisk_guest))]
fn in_task() -> bool {
    IN_TASK.with(|in_task| in_task.get())
}

#[cfg(not(zisk_guest))]
fn set_in_task(value: bool) {
    IN_TASK.with(|in_task| in_task.set(value));
}

/// Clears the flag when a region closes, or unwinds natively
struct TaskScope;

impl TaskScope {
    fn enter() -> Self {
        set_in_task(true);
        TaskScope
    }
}

impl Drop for TaskScope {
    fn drop(&mut self) {
        set_in_task(false);
    }
}

/// Panics if it is called inside a task: for what a task must not do, `what` being its name
pub(crate) fn assert_outside_task(what: &str) {
    if in_task() {
        panic!("{what} inside a fork-join task: do it before or after the region");
    }
}

/// Runs `jobs` as one fork-join region and returns their results, in order.
///
/// Every job's arena gets `config`'s heap and stack. A job that allocates more than its heap
/// panics, which fails the execution. Nothing stops a job that runs deeper than its stack: it
/// writes over the top of its heap. A canary below the stack catches that when the job returns,
/// and the job panics; but a single stack frame larger than the canary can step over it, so size
/// the stack for the job's deepest call chain.
///
/// Called inside a task, it runs `jobs` inline, one after another, in that task: regions do not
/// nest. Closures and results must be `Send`, so tasks cannot share state through `Cell`,
/// `RefCell` or `Rc`:
///
/// ```compile_fail,E0277
/// let counter = std::cell::Cell::new(0);
/// let jobs: Vec<_> = (0..2).map(|i| { let counter = &counter; move || counter.set(i) }).collect();
/// ziskos::par::run(jobs, &ziskos::par::Config::default());
/// ```
pub fn run<T: Send, F: FnOnce() -> T + Send>(jobs: Vec<F>, config: &Config) -> Vec<T> {
    if in_task() {
        return jobs.into_iter().map(|job| job()).collect();
    }
    let _scope = TaskScope::enter();
    #[cfg(zisk_guest)]
    if FORK_JOIN_REGION {
        return region::run(jobs, config);
    }
    let _ = config;
    jobs.into_iter().map(|job| job()).collect()
}

/// Runs `a` and `b` as one fork-join region of two tasks and returns both results: [`run`] for
/// two closures of different types.
///
/// Both tasks get `config`'s heap and stack, as in [`run`].
pub fn join<A, B, RA, RB>(a: A, b: B, config: &Config) -> (RA, RB)
where
    A: FnOnce() -> RA + Send,
    B: FnOnce() -> RB + Send,
    RA: Send,
    RB: Send,
{
    enum Job<A, B> {
        A(A),
        B(B),
    }
    enum Done<RA, RB> {
        A(RA),
        B(RB),
    }
    // One closure expression, so both tasks have the same type
    let task = |job: Job<A, B>| {
        move || match job {
            Job::A(a) => Done::A(a()),
            Job::B(b) => Done::B(b()),
        }
    };
    let jobs = Vec::from([task(Job::A(a)), task(Job::B(b))]);
    let mut done = run(jobs, config).into_iter();
    match (done.next(), done.next()) {
        (Some(Done::A(ra)), Some(Done::B(rb))) => (ra, rb),
        _ => unreachable!("run returns its jobs' results in order"),
    }
}

#[cfg(any(zisk_guest, test))]
/// The page every arena starts on, so that no two tasks share a page
const ARENA_PAGE: usize = 4096;

#[cfg(any(zisk_guest, test))]
/// Bytes of the canary between a task's heap and its stack
const ARENA_CANARY_BYTES: usize = 8 * 8;

#[cfg(any(zisk_guest, test))]
/// How a region lays out its arenas: each `stride` bytes long, starting at a multiple of `align`,
/// the first `slot_bytes` holding the task's slot and the next `heap_bytes` its heap
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ArenaLayout {
    slot_bytes: usize,
    heap_bytes: usize,
    stride: usize,
    align: usize,
    total: usize,
}

#[cfg(any(zisk_guest, test))]
/// The layout of `n` arenas for a slot of `slot_size` bytes aligned to `slot_align`, and
/// `config`'s heap and stack; `None` when it does not fit in the address space. The arenas are
/// aligned to the page, or to the slot's alignment where that is larger, so that every slot is
/// aligned; every size is checked, since a `Config` is the guest's to choose
fn arena_layout(
    n: usize,
    slot_size: usize,
    slot_align: usize,
    config: &Config,
) -> Option<ArenaLayout> {
    let align = slot_align.max(ARENA_PAGE);
    let slot_bytes = slot_size.checked_next_multiple_of(16)?;
    let heap_bytes = config.heap_bytes.checked_next_multiple_of(16)?;
    let stride = slot_bytes
        .checked_add(heap_bytes)?
        .checked_add(ARENA_CANARY_BYTES)?
        .checked_add(config.stack_bytes)?
        .checked_next_multiple_of(align)?;
    let total = stride.checked_mul(n)?;
    // What a `Layout` accepts: a size that, rounded up to the alignment, does not overflow isize
    (total <= isize::MAX as usize - (align - 1)).then_some(ArenaLayout {
        slot_bytes,
        heap_bytes,
        stride,
        align,
        total,
    })
}

#[cfg(zisk_guest)]
mod region {
    use core::alloc::Layout;
    use core::mem::{align_of, size_of};

    use crate::alloc::{bump_heap, set_bump_heap};
    use crate::alloc_extern::{alloc::alloc as allocate, vec::Vec};
    use crate::zisklib::zkvm_parallel::{zkvm_parallel, zkvm_parallel_ret, ZkvmTask};

    /// The canary between a task's heap and its stack: what a stack that grew past its bottom
    /// writes over first
    const CANARY_WORDS: usize = super::ARENA_CANARY_BYTES / size_of::<u64>();
    const CANARY: u64 = 0x5354_4143_4b5f_454e; // "STACK_EN"

    /// What a task finds at the start of its arena
    struct Slot<F, T> {
        job: Option<F>,
        result: Option<T>,
        heap_bottom: usize,
        heap_top: usize,
        /// Where the canary starts, between the heap and the stack
        canary: usize,
        stack_bytes: usize,
    }

    /// A task's entry: points the allocator at the task's heap, runs its job, and checks that its
    /// stack did not grow past its bottom
    unsafe extern "C" fn enter<T, F: FnOnce() -> T>(arg: *const u8, _out: *mut u8) {
        let slot = unsafe { &mut *(arg as *mut Slot<F, T>) };
        // Before anything else touches the allocator: it only writes its state, so the task does
        // not depend on what the task before it left there
        unsafe { set_bump_heap(slot.heap_bottom, slot.heap_top) };
        let job = slot.job.take().expect("a fork-join task runs once");
        slot.result = Some(job());
        let canary = slot.canary as *const u64;
        // SAFETY: the canary words are inside the task's own arena, written before the region
        if (0..CANARY_WORDS).any(|k| unsafe { canary.add(k).read_volatile() } != CANARY) {
            panic!(
                "a fork-join task ran deeper than its {}-byte stack: raise par::Config::stack_bytes",
                slot.stack_bytes
            );
        }
    }

    pub(super) fn run<T, F: FnOnce() -> T>(jobs: Vec<F>, config: &super::Config) -> Vec<T> {
        let n = jobs.len();
        if n == 0 {
            return Vec::new();
        }

        // Each arena: the slot, the heap, the canary and the stack, aligned so the slot is, the
        // stack top is 16-byte aligned and no two tasks share a page
        let arenas =
            super::arena_layout(n, size_of::<Slot<F, T>>(), align_of::<Slot<F, T>>(), config)
                .unwrap_or_else(|| {
                    panic!(
                "{n} fork-join arenas of a {}-byte heap and a {}-byte stack do not fit in memory",
                config.heap_bytes, config.stack_bytes
            )
                });
        let (slot_bytes, heap_bytes, arena_bytes) =
            (arenas.slot_bytes, arenas.heap_bytes, arenas.stride);
        let layout = Layout::from_size_align(arenas.total, arenas.align).expect("fork-join arenas");
        // Fresh bump memory, never used before, so the heaps start zeroed as the allocator
        // assumes
        let base = unsafe { allocate(layout) } as usize;
        let slot = |i: usize| (base + i * arena_bytes) as *mut Slot<F, T>;

        let mut tasks = Vec::with_capacity(n);
        for (i, job) in jobs.into_iter().enumerate() {
            let heap_bottom = slot(i) as usize + slot_bytes;
            let heap_top = heap_bottom + heap_bytes;
            unsafe {
                slot(i).write(Slot {
                    job: Some(job),
                    result: None,
                    heap_bottom,
                    heap_top,
                    canary: heap_top,
                    stack_bytes: config.stack_bytes,
                });
                let canary = heap_top as *mut u64;
                for k in 0..CANARY_WORDS {
                    canary.add(k).write_volatile(CANARY);
                }
            };
            tasks.push(ZkvmTask {
                entry: enter::<T, F>,
                arg: slot(i) as *const u8,
                out: slot(i) as *mut u8,
                stack_top: (slot(i) as usize + arena_bytes) as u64,
                mem_start: slot(i) as u64,
                mem_end: (slot(i) as usize + arena_bytes) as u64,
            });
        }

        // The tasks point the allocator at their own heaps; the caller's comes back after
        let heap = bump_heap();
        unsafe { zkvm_parallel(tasks.as_ptr(), n as u64, zkvm_parallel_ret) };
        unsafe { set_bump_heap(heap.0, heap.1) };

        (0..n)
            .map(|i| unsafe { (*slot(i)).result.take() }.expect("every fork-join task ran"))
            .collect()
    }
}

#[cfg(all(test, not(zisk_guest)))]
mod tests {
    use super::*;

    #[test]
    fn arenas_are_page_aligned_and_hold_slot_heap_canary_and_stack() {
        let config = Config { heap_bytes: 1000, stack_bytes: 3000 };
        let arenas = arena_layout(3, 40, 8, &config).unwrap();
        assert_eq!((arenas.slot_bytes, arenas.heap_bytes, arenas.align), (48, 1008, ARENA_PAGE));
        assert_eq!(arenas.stride, 2 * ARENA_PAGE); // 48 + 1008 + 64 + 3000 rounded up to a page
        assert_eq!(arenas.total, 3 * arenas.stride);
    }

    #[test]
    fn an_over_aligned_slot_aligns_every_arena() {
        let arenas = arena_layout(2, 8192, 8192, &Config::default()).unwrap();
        assert_eq!(arenas.align, 8192);
        assert_eq!(arenas.stride % 8192, 0);
    }

    #[test]
    fn arenas_that_overflow_the_address_space_are_rejected() {
        let huge = |heap_bytes, stack_bytes| Config { heap_bytes, stack_bytes };
        assert_eq!(arena_layout(1, 64, 8, &huge(usize::MAX, 0)), None);
        assert_eq!(arena_layout(1, 64, 8, &huge(0, usize::MAX)), None);
        assert_eq!(arena_layout(1, 64, 8, &huge(usize::MAX / 2, usize::MAX / 2)), None);
        // Each arena fits, but not all of them together
        assert_eq!(arena_layout(3, 64, 8, &huge(isize::MAX as usize / 2, 0)), None);
        assert_eq!(arena_layout(1, usize::MAX - 4, 8, &Config::default()), None);
    }

    #[test]
    fn runs_every_job_in_order() {
        let shared = [1u64, 2, 3, 4];
        let jobs: Vec<_> = (0..4).map(|i| move || shared[i] * 10).collect();
        assert_eq!(run(jobs, &Config::default()), [10, 20, 30, 40]);
    }

    #[test]
    fn joins_two_jobs_of_different_types() {
        let words = ["fork", "join"];
        let (len, upper) = join(
            || words.iter().map(|w| w.len()).sum::<usize>(),
            || words[1].to_uppercase(),
            &Config::default(),
        );
        assert_eq!((len, upper.as_str()), (8, "JOIN"));
    }

    #[test]
    fn a_region_inside_a_task_runs_inline() {
        let config = Config::default();
        let jobs: Vec<_> = (1..=3u64)
            .map(|i| {
                let config = &config;
                move || run((0..i).map(|k| move || k * 10).collect(), config).iter().sum::<u64>()
            })
            .collect();
        assert_eq!(run(jobs, &config), [0, 10, 30]);
        // And the flag is cleared once the outer region closes
        assert!(!in_task());
    }

    #[test]
    #[should_panic(expected = "reading the input inside a fork-join task")]
    fn reading_the_input_inside_a_task_panics() {
        run(vec![|| crate::io::read_slice().len()], &Config::default());
    }

    #[test]
    #[should_panic(expected = "committing outputs inside a fork-join task")]
    fn committing_outputs_inside_a_task_panics() {
        run(vec![|| crate::io::commit_slice(&[1, 2, 3])], &Config::default());
    }

    #[test]
    fn the_input_and_outputs_are_usable_around_a_region() {
        assert_outside_task("before");
        run(vec![|| 1], &Config::default());
        assert_outside_task("after");
    }
}
