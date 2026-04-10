use zisk_core::InstContext;

/// Monitors heap/RAM usage during emulation.
///
/// Tracks the heap region boundaries and the address of the heap position pointer,
/// allowing dynamic reads of current heap usage from the emulated memory.
#[derive(Debug, Default)]
pub struct RamMonitor {
    /// Bottom (start) address of the heap region
    heap_bottom: u64,
    /// Top (end) address of the heap region
    heap_top: u64,
    /// Address in emulated memory where the current heap position is stored
    heap_pos_address: u64,
    /// Total size of the RAM region (heap_top - heap_bottom), updated on finish
    pub ram_size: u64,
    /// Bytes of RAM used, updated on finish
    pub ram_used: u64,
}

impl RamMonitor {
    pub fn new() -> Self {
        Self::default()
    }

    /// Configure heap boundaries and the address of the heap position pointer.
    pub fn set_heap_address(&mut self, heap_bottom: u64, heap_top: u64, heap_pos_address: u64) {
        self.heap_bottom = heap_bottom;
        self.heap_top = heap_top;
        self.heap_pos_address = heap_pos_address;
    }

    /// Returns the number of bytes currently used in the heap, reading from emulated memory.
    /// Returns 0 if heap monitoring has not been configured.
    pub fn get_usage(&self, inst_ctx: &InstContext) -> u64 {
        if self.heap_pos_address == 0 {
            0
        } else {
            let heap_pos = inst_ctx.mem.read(self.heap_pos_address, 8);
            if heap_pos == 0 {
                0
            } else {
                heap_pos - self.heap_bottom
            }
        }
    }

    /// Returns the total heap size (heap_top - heap_bottom).
    pub fn total_size(&self) -> u64 {
        self.heap_top - self.heap_bottom
    }

    /// Returns true if heap monitoring has been configured.
    pub fn is_configured(&self) -> bool {
        self.heap_pos_address != 0
    }

    /// Called at the end of emulation to snapshot current usage into ram_size/ram_used.
    pub fn on_finish(&mut self, inst_ctx: &InstContext) {
        let used = self.get_usage(inst_ctx);
        self.ram_size = self.total_size();
        self.ram_used = used;
    }
}
