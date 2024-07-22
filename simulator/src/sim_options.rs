/// ZisK simulator options structure
pub struct SimOptions {
    pub to: i64,
    pub max_steps: u64,
    pub print_step: u64,
}

/// Default constructor for SimOptions structure
impl Default for SimOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// ZisK simulator options structure implementation
impl SimOptions {
    /// Zisk simulator options constructor
    pub fn new() -> SimOptions {
        SimOptions { to: -1, max_steps: 1_000_000, print_step: 0 }
    }
}
