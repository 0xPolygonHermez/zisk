use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// A token for cooperative cancellation of in-flight operations.
///
/// Obtain via [`CancellationToken::new`]. Clone to share across threads.
/// Call [`cancel`](Self::cancel) to signal cancellation; the operation
/// will poll [`is_cancelled`](Self::is_cancelled) at checkpoints and bail out.
///
/// This type is cheap to clone (internally reference-counted).
#[derive(Clone, Default)]
pub struct CancellationToken {
    flag: Arc<AtomicBool>,
}

impl CancellationToken {
    /// Create a new cancellation token in the non-cancelled state.
    #[must_use]
    pub fn new() -> Self {
        Self { flag: Arc::new(AtomicBool::new(false)) }
    }

    /// Signal cancellation.
    ///
    /// This is idempotent: calling it multiple times has no additional effect.
    /// Operations polling this token will see the cancellation at their next checkpoint.
    pub fn cancel(&self) {
        self.flag.store(true, Ordering::Release);
    }

    /// Check if cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::Acquire)
    }

    /// Create a callback that, when invoked, cancels this token.
    ///
    /// Useful for integrating with APIs that expect `Arc<dyn Fn() + Send + Sync>`.
    #[must_use]
    pub fn as_cancel_fn(&self) -> Arc<dyn Fn() + Send + Sync> {
        let flag = Arc::clone(&self.flag);
        Arc::new(move || flag.store(true, Ordering::Release))
    }
}

impl std::fmt::Debug for CancellationToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CancellationToken").field("cancelled", &self.is_cancelled()).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initially_not_cancelled() {
        let token = CancellationToken::new();
        assert!(!token.is_cancelled());
    }

    #[test]
    fn cancel_sets_flag() {
        let token = CancellationToken::new();
        token.cancel();
        assert!(token.is_cancelled());
    }

    #[test]
    fn clone_shares_state() {
        let token = CancellationToken::new();
        let clone = token.clone();
        token.cancel();
        assert!(clone.is_cancelled());
    }

    #[test]
    fn cancel_fn_works() {
        let token = CancellationToken::new();
        let f = token.as_cancel_fn();
        f();
        assert!(token.is_cancelled());
    }
}
