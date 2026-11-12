use super::services::AsmServicesInner;

use anyhow::Result;

use std::collections::BTreeMap;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError, Weak};

/// Every live [`SharedSegments`] in the process, by shm prefix.
///
/// Weak, so an entry never keeps a set alive; its owners do. It exists because the
/// segments' names carry only the pid, rank and hints mode: everything in the process that
/// sets up a program on a prefix must get the same set.
static REGISTRY: Mutex<BTreeMap<String, Weak<SharedSegments>>> = Mutex::new(BTreeMap::new());

/// The services' shared-memory segments named by one shm prefix, shared by every program
/// set up on it.
///
/// Owned through `Arc` by each program's services and by the parent's mappings of the
/// segments, and unlinked when the last owner drops it. Created at most once per prefix
/// while owned: creating the segments again would unlink them under the services mapping
/// them, and the old ones would stay resident with nothing able to reach them.
///
/// It also records which program the guest RAM and ROM serve ([`AsmServices::activate`]),
/// and holds the lock behind [`SegmentsClaim`].
///
/// [`AsmServices::activate`]: super::AsmServices::activate
pub struct SharedSegments {
    shm_prefix: String,
    /// Taken before anything writes the guest RAM and ROM, and set only once the
    /// incoming program's reset has completed, so a failure in between leaves no
    /// program recorded rather than the wrong one.
    active: Mutex<Option<ActiveProgram>>,
    /// Whether a [`SegmentsClaim`] is held. Not a guard: a job holds its claim for minutes,
    /// across calls.
    held: Mutex<bool>,
    released: Condvar,
}

impl SharedSegments {
    /// The set named by `shm_prefix`, created with `create` if nothing in the process owns
    /// one.
    ///
    /// Decided under the registry's lock, so two setups cannot both create the set, and none
    /// can take one whose last owner is unlinking it.
    pub(super) fn acquire(
        shm_prefix: &str,
        create: impl FnOnce() -> Result<()>,
    ) -> Result<Arc<Self>> {
        let mut registry = lock(&REGISTRY);
        if let Some(segments) = registry.get(shm_prefix).and_then(Weak::upgrade) {
            tracing::debug!("Reusing the shared segments of {shm_prefix}");
            return Ok(segments);
        }
        create()?;
        let segments = Arc::new(Self {
            shm_prefix: shm_prefix.to_string(),
            active: Mutex::default(),
            held: Mutex::default(),
            released: Condvar::new(),
        });
        registry.insert(shm_prefix.to_string(), Arc::downgrade(&segments));
        Ok(segments)
    }

    /// The prefix naming the segments, `ZISK_{pid}_{rank}_h{0,1}`.
    pub fn shm_prefix(&self) -> &str {
        &self.shm_prefix
    }

    /// Wait until no setup or job is using the segments, and hold them until the returned
    /// claim is dropped.
    pub(super) fn claim(self: &Arc<Self>) -> SegmentsClaim {
        let mut held = lock(&self.held);
        while *held {
            held = self.released.wait(held).unwrap_or_else(PoisonError::into_inner);
        }
        *held = true;
        SegmentsClaim { segments: Arc::clone(self) }
    }

    /// Whether the guest RAM and ROM serve the program `services` runs.
    pub(super) fn is_active(&self, services: &Arc<AsmServicesInner>) -> bool {
        lock(&self.active)
            .as_ref()
            .is_some_and(|active| std::ptr::eq(active.services.as_ptr(), Arc::as_ptr(services)))
    }

    /// Record `program` as the one the guest RAM and ROM serve.
    pub(super) fn set_active(&self, program: ActiveProgram) {
        *lock(&self.active) = Some(program);
    }

    /// Forget which program the guest RAM and ROM serve, and return it.
    pub(super) fn take_active(&self) -> Option<ActiveProgram> {
        lock(&self.active).take()
    }
}

impl Drop for SharedSegments {
    fn drop(&mut self) {
        let mut registry = lock(&REGISTRY);
        // Between this set's last owner going and this drop taking the lock, a setup may
        // have found the set unreachable and created a new one under the same names, which
        // unlinks these. The names are then the new set's.
        let ours = registry
            .get(&self.shm_prefix)
            .is_some_and(|entry| std::ptr::eq(entry.as_ptr(), self as *const Self));
        if !ours {
            return;
        }
        registry.remove(&self.shm_prefix);
        tracing::debug!(
            "Last owner of the shared segments {} gone; unlinking them",
            self.shm_prefix
        );
        super::janitor::cleanup_shm_prefix(&self.shm_prefix);
    }
}

impl std::fmt::Debug for SharedSegments {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SharedSegments").field("shm_prefix", &self.shm_prefix).finish()
    }
}

/// The use of a set of shared segments, by one setup or one job, until dropped.
///
/// Every program on a set shares its guest RAM and ROM, its input and its outputs, so only
/// one may use them at a time: from the handoff that makes its program active
/// ([`AsmServices::activate`], which requires a claim) through the last read of its job's
/// results. Starting a program's services takes one too, since starting writes the guest RAM
/// and ROM, and [`AsmServices::new`] returns it so the setup keeps it until its program is
/// active.
///
/// Not reentrant: a thread holding one that asks for another on the same set, including by
/// setting up a program on it, waits for ever.
///
/// [`AsmServices::activate`]: super::AsmServices::activate
/// [`AsmServices::new`]: super::AsmServices::new
#[must_use = "dropping the claim releases the shared segments at once"]
pub struct SegmentsClaim {
    segments: Arc<SharedSegments>,
}

impl SegmentsClaim {
    /// The set this claim holds.
    pub(super) fn segments(&self) -> &Arc<SharedSegments> {
        &self.segments
    }
}

impl Drop for SegmentsClaim {
    fn drop(&mut self) {
        *lock(&self.segments.held) = false;
        self.segments.released.notify_one();
    }
}

/// The program the guest RAM and ROM serve, as [`SharedSegments`] records it.
///
/// Weak, so that recording a program does not keep it alive. With the latch its teardown
/// sets, so that an activation finding it already dropped can still wait for its services:
/// the `Weak` stops upgrading as soon as the last reference goes, before `Drop` has stopped
/// them.
pub(super) struct ActiveProgram {
    pub(super) services: Weak<AsmServicesInner>,
    pub(super) stopped: Arc<StoppedLatch>,
}

/// Sets its latch when dropped. A field placed after the services it reports on, so it
/// is set once they are gone, however their owner's teardown ended.
#[derive(Default)]
pub(super) struct SetOnDrop(pub(super) Arc<StoppedLatch>);

impl Drop for SetOnDrop {
    fn drop(&mut self) {
        self.0.set();
    }
}

/// Set once a program's services have stopped.
#[derive(Default)]
pub(super) struct StoppedLatch {
    stopped: Mutex<bool>,
    set: Condvar,
}

impl StoppedLatch {
    pub(super) fn set(&self) {
        *lock(&self.stopped) = true;
        self.set.notify_all();
    }

    pub(super) fn wait(&self) {
        let mut stopped = lock(&self.stopped);
        while !*stopped {
            stopped = self.set.wait(stopped).unwrap_or_else(PoisonError::into_inner);
        }
    }
}

/// Lock `mutex` even if poisoned: each state behind these locks is a single value, written
/// whole, so a panic cannot leave it half-updated.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::mpsc;
    use std::time::Duration;

    /// A set as the first program's setup creates it, without the C helpers.
    fn first_owner(shm_prefix: &str) -> Arc<SharedSegments> {
        SharedSegments::acquire(shm_prefix, || Ok(())).unwrap()
    }

    /// Whether the segment `name` exists in `/dev/shm`.
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    fn segment_exists(name: &str) -> bool {
        let c = std::ffi::CString::new(name).unwrap();
        let fd = unsafe { libc::shm_open(c.as_ptr(), libc::O_RDONLY, 0) };
        if fd >= 0 {
            unsafe { libc::close(fd) };
        }
        fd >= 0
    }

    /// Create the stand-in segment `name` in `/dev/shm`.
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    fn create_segment(name: &str) {
        let c = std::ffi::CString::new(name).unwrap();
        let fd = unsafe { libc::shm_open(c.as_ptr(), libc::O_CREAT | libc::O_RDWR, 0o600) };
        assert!(fd >= 0, "could not create the stand-in segment {name}");
        unsafe { libc::close(fd) };
    }

    /// The segments are shared by every owner of a set, so one owner going must not take
    /// them with it, but the last one must, or the process leaves ~15 GiB of `/dev/shm`
    /// behind.
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn segments_are_unlinked_only_when_the_last_owner_drops_them() {
        let prefix = format!("ZISK_unittest_lease_{}", std::process::id());
        let segment = format!("{prefix}_MT_output_0");
        create_segment(&segment);

        let first = first_owner(&prefix);
        let second =
            SharedSegments::acquire(&prefix, || panic!("an owned set must not be created again"))
                .unwrap();
        assert!(Arc::ptr_eq(&first, &second), "a prefix must have one set in the process");

        drop(first);
        assert!(segment_exists(&segment), "one owner going must not unlink the shared segments");

        drop(second);
        assert!(!segment_exists(&segment), "the last owner must unlink them");

        let mut created = false;
        let _third = SharedSegments::acquire(&prefix, || {
            created = true;
            Ok(())
        })
        .unwrap();
        assert!(created, "the prefix must be forgotten, so a later setup creates it again");
    }

    /// A setup that finds a set unreachable before its drop has run creates a new one under
    /// the same names; the old set's drop must then leave those names to the new one.
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn a_replaced_set_leaves_its_successors_segments_alone() {
        let prefix = format!("ZISK_unittest_replaced_{}", std::process::id());
        let segment = format!("{prefix}_MT_output_0");
        create_segment(&segment);

        let old = first_owner(&prefix);
        let successor = Arc::new(SharedSegments {
            shm_prefix: prefix.clone(),
            active: Mutex::default(),
            held: Mutex::default(),
            released: Condvar::new(),
        });
        lock(&REGISTRY).insert(prefix.clone(), Arc::downgrade(&successor));

        drop(old);
        assert!(
            segment_exists(&segment),
            "a replaced set must not unlink its successor's segments"
        );
        assert!(
            lock(&REGISTRY).get(&prefix).is_some_and(|e| std::ptr::eq(e.as_ptr(), &*successor)),
            "a replaced set must not forget its successor"
        );

        drop(successor);
        assert!(!segment_exists(&segment), "the successor's last owner must unlink them");
    }

    /// A job holds the segments from its handoff to its last read, so a second claim waits
    /// for the first to be dropped, and then gets in.
    #[test]
    fn a_claim_waits_until_the_one_before_it_is_dropped() {
        let segments = first_owner(&format!("ZISK_unittest_claim_{}", std::process::id()));
        let first = segments.claim();

        let (claimed, rx) = mpsc::channel();
        let second = {
            let segments = Arc::clone(&segments);
            std::thread::spawn(move || {
                let _claim = segments.claim();
                claimed.send(()).unwrap();
            })
        };
        assert!(
            rx.recv_timeout(Duration::from_millis(200)).is_err(),
            "a second claim must wait while the first is held"
        );

        drop(first);
        rx.recv_timeout(Duration::from_secs(10)).expect("dropping a claim must let the next in");
        second.join().unwrap();
        assert!(!*lock(&segments.held), "the last claim dropped must release the segments");
    }
}
