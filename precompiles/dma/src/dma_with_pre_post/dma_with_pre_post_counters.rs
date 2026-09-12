use zisk_common::CollectCounter;

/// Operations that take a single row: the DMA row, carrying a PRE, a POST, or nothing.
pub const DMA_WPP_CLASS_SINGLE: usize = 0;

/// Operations that take two rows: the DMA + POST row, plus the extra PRE row after it.
pub const DMA_WPP_CLASS_DOUBLE: usize = 1;

/// Number of row-cost classes.
pub const DMA_WPP_CLASSES: usize = 2;

/// Rows each class takes.
pub const DMA_WPP_CLASS_ROWS: [usize; DMA_WPP_CLASSES] = [1, 2];

/// How many operations of each row-cost class an instance collects from a chunk.
///
/// `Dma` and `DmaPrePost` count *rows* per opcode, and let an operation be split between two
/// instances. The fused air cannot split an operation, so it counts *operations* instead, and it
/// splits them by row cost rather than by opcode: within a class every operation costs the same
/// number of rows, so a cut between two operations of a class is exact in rows too — which is what
/// lets the planner hand out rows without ever knowing where the double operations sit inside the
/// chunk.
///
/// Splitting by opcode instead would not help here: this air is not parameterizable, so every
/// opcode goes to it anyway.
#[derive(Debug, Clone, Copy)]
pub struct DmaWithPrePostCollectCounters {
    pub classes: [CollectCounter; DMA_WPP_CLASSES],
}

impl DmaWithPrePostCollectCounters {
    pub fn new(single: CollectCounter, double: CollectCounter) -> Self {
        Self { classes: [single, double] }
    }

    /// `true` when the operation must be collected by this instance.
    #[inline(always)]
    pub fn should_collect(&mut self, is_double: bool) -> bool {
        let class = if is_double { DMA_WPP_CLASS_DOUBLE } else { DMA_WPP_CLASS_SINGLE };
        !self.classes[class].should_skip()
    }

    pub fn is_final_skip(&self) -> bool {
        self.classes.iter().all(|c| c.is_final_skip())
    }

    pub fn debug_assert_is_final_skip(&self) {
        debug_assert!(
            self.is_final_skip(),
            "pending to collect => single: {}/{}|double: {}/{}",
            self.classes[DMA_WPP_CLASS_SINGLE].collected,
            self.classes[DMA_WPP_CLASS_SINGLE].collect_count,
            self.classes[DMA_WPP_CLASS_DOUBLE].collected,
            self.classes[DMA_WPP_CLASS_DOUBLE].collect_count,
        );
    }

    #[cfg(any(feature = "save_dma_collectors", feature = "save_dma_plans"))]
    pub fn get_debug_info(&self) -> String {
        let one = &self.classes[DMA_WPP_CLASS_SINGLE];
        let two = &self.classes[DMA_WPP_CLASS_DOUBLE];
        (if one.initial_skip == 0 {
            format!("S1:{}|", one.collect_count)
        } else {
            format!("S1:({}){}|", one.collect_count, one.initial_skip)
        }) + &(if two.initial_skip == 0 {
            format!("S2:{}", two.collect_count)
        } else {
            format!("S2:({}){}", two.collect_count, two.initial_skip)
        })
    }
}
