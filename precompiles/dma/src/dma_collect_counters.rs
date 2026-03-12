use zisk_common::CollectCounter;
use zisk_core::zisk_ops::ZiskOp;

#[derive(Debug, Clone, Copy)]
pub struct DmaCollectCounters {
    // This counters are for a specific instance, means that only need to know of collect
    // for each different operation, no for destination
    pub memcpy: CollectCounter,
    pub inputcpy: CollectCounter,
    pub memset: CollectCounter,
    pub memcmp: CollectCounter,
}

impl DmaCollectCounters {
    pub fn debug_assert_is_final_skip(&self) {
        debug_assert!(
            self.is_final_skip(),
            "pending to collect => memcpy: {}/{}|inputcpy: {}/{}|memset: {}/{}|memcmp: {}/{}",
            self.memcpy.collected,
            self.memcpy.collect_count,
            self.inputcpy.collected,
            self.inputcpy.collect_count,
            self.memset.collected,
            self.memset.collect_count,
            self.memcmp.collected,
            self.memcmp.collect_count
        );
    }
    pub fn is_final_skip(&self) -> bool {
        self.memcpy.is_final_skip()
            && self.inputcpy.is_final_skip()
            && self.memset.is_final_skip()
            && self.memcmp.is_final_skip()
    }
    pub fn should_collect(&mut self, rows: u64, op: u8) -> Option<(u32, u32)> {
        match op {
            ZiskOp::DMA_MEMCPY | ZiskOp::DMA_XMEMCPY => self.memcpy.should_process(rows as u32),
            ZiskOp::DMA_MEMCMP | ZiskOp::DMA_XMEMCMP => self.memcmp.should_process(rows as u32),
            ZiskOp::DMA_INPUTCPY => self.inputcpy.should_process(rows as u32),
            ZiskOp::DMA_XMEMSET => self.memset.should_process(rows as u32),
            _ => panic!("Invalid operation 0x{op:02X} for DmaCollectCounters"),
        }
    }
    #[inline(always)]
    pub fn should_collect_single_row(&mut self, op: u8) -> bool {
        self.should_collect(1, op).is_some()
    }

    #[cfg(feature = "save_dma_collectors")]
    pub fn get_full_debug_info(&self) -> String {
        format!(
            "CY:{}/{}|IC:{}/{}|MS:{}/{}|MC:{}/{}",
            self.memcpy.collected,
            self.memcpy.collect_count,
            self.inputcpy.collected,
            self.inputcpy.collect_count,
            self.memset.collected,
            self.memset.collect_count,
            self.memcmp.collected,
            self.memcmp.collect_count,
        )
    }
    #[cfg(any(feature = "save_dma_collectors", feature = "save_dma_plans"))]
    pub fn get_debug_info(&self) -> String {
        (if self.memcpy.initial_skip == 0 {
            format!("CY:{}|", self.memcpy.collect_count)
        } else {
            format!("CY:({}){}|", self.memcpy.collect_count, self.memcpy.initial_skip)
        }) + &(if self.inputcpy.initial_skip == 0 {
            format!("IC:{}|", self.inputcpy.collect_count)
        } else {
            format!("IC:({}){}|", self.inputcpy.collect_count, self.inputcpy.initial_skip)
        }) + &(if self.memset.initial_skip == 0 {
            format!("MS:{}|", self.memset.collect_count)
        } else {
            format!("MS:({}){}|", self.memset.collect_count, self.memset.initial_skip)
        }) + &(if self.memcmp.initial_skip == 0 {
            format!("MC:{}", self.memcmp.collect_count)
        } else {
            format!("MC:({}){}", self.memcmp.collect_count, self.memcmp.initial_skip)
        })
    }
}
