use std::sync::{
    atomic::{AtomicU32, Ordering},
    Condvar, Mutex,
};

/// A struct to track the number of active threads and allows waiting for all threads to complete
/// before proceeding with further operations.
pub struct ThreadController {
    // An atomic counter for tracking the number of active working threads.
    working_threads: AtomicU32,

    // A mutex used to synchronize access to the condition variable.
    mutex: Mutex<()>,

    // A condition variable used to notify waiting threads when all working threads are done.
    condvar: Condvar,
}

impl ThreadController {
    pub fn new() -> ThreadController {
        ThreadController {
            working_threads: AtomicU32::new(0),
            mutex: Mutex::new(()),
            condvar: Condvar::new(),
        }
    }

    /// Blocks the calling thread until all working threads have completed.
    /// This is done by waiting on a condition variable, which will be signaled when
    /// the number of working threads reaches zero.
    pub fn wait_for_threads(&self) {
        let mut guard = self.mutex.lock().unwrap();

        while self.working_threads.load(Ordering::Acquire) > 0 {
            guard = self.condvar.wait(guard).unwrap();
        }
    }

    /// Increments the count of active working threads. This is typically called
    /// when a new thread is started and begins performing work.
    pub fn add_working_thread(&self) {
        self.working_threads.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrements the count of active working threads. If this brings the count
    /// to zero, it notifies any threads waiting for all work to complete.
    pub fn remove_working_thread(&self) {
        if self.working_threads.fetch_sub(1, Ordering::Relaxed) == 1 {
            let _guard = self.mutex.lock().unwrap();
            self.condvar.notify_all();
        }
    }
}
