use crate::Plan;

/// The `InstanceCtx` struct encapsulates the context of an execution instance,
/// including its associated execution plan and global identifier.
///
/// This struct is primarily used to manage metadata and configurations for instances
/// that are part of a larger execution pipeline.
///
/// # Fields
/// * `plan` - The `Plan` associated with this instance, defining its execution strategy.
/// * `global_id` - A unique global identifier for the instance, useful for tracking its position
///   within the execution pipeline.
pub struct InstanceCtx {
    /// Plan for the current instance.
    pub plan: Plan,

    /// Global ID of the current instance.
    pub global_id: usize,
}

impl InstanceCtx {
    /// Creates a new `InstanceCtx`.
    ///
    /// # Arguments
    /// * `global_id` - A unique global identifier for the instance.
    /// * `plan` - The execution `Plan` for this instance.
    ///
    /// # Returns
    /// A new instance of `InstanceCtx` initialized with the given plan and global ID.
    pub fn new(global_id: usize, plan: Plan) -> Self {
        Self { plan, global_id }
    }
}

/// # Safety
/// This struct is marked as `Send` because its fields are safe to transfer across threads,
/// assuming that the `Plan` type and its associated data are also thread-safe.
unsafe impl Send for InstanceCtx {}
