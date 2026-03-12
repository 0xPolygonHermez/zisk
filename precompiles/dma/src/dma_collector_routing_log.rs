use zisk_common::ChunkId;

#[cfg(feature = "save_dma_collectors")]
use zisk_common::STEP;

#[cfg(feature = "save_dma_collectors")]
#[derive(Debug)]
pub struct DmaCollectorRoutingLog {
    pub chunk_id: ChunkId,
    pub log: Vec<(u8, u64, u32, u32, u32)>,
}

#[cfg(not(feature = "save_dma_collectors"))]
#[derive(Debug)]
pub struct DmaCollectorRoutingLog {}

#[cfg(not(feature = "save_dma_collectors"))]
impl DmaCollectorRoutingLog {
    pub fn new(_chunk_id: ChunkId) -> Self {
        Self {}
    }
    #[inline(always)]
    pub fn log_collect(&mut self, _rows: u32, _data: &[u64], _skip: u32, _max_count: u32) {}
    #[inline(always)]
    pub fn log_discard(&mut self, _reason: u8, _data: &[u64]) {}
    #[inline(always)]
    pub fn log_discard_cond(
        &mut self,
        cond: bool,
        _reason: u8,
        _data: &[u64],
        _result: bool,
    ) -> bool {
        cond
    }
}

#[cfg(feature = "save_dma_collectors")]
impl DmaCollectorRoutingLog {
    pub fn new(chunk_id: ChunkId) -> Self {
        Self { chunk_id, log: Vec::new() }
    }

    pub fn get_debug_info(&self) -> String {
        self.log
            .iter()
            .map(|(reason, step, rows, skip, max_count)| {
                format!(
                    "{}|{reason}|@{}|C:{rows}|K:{skip}|M:{max_count}|S:{step}",
                    if *reason == 0 { "COLLECT" } else { "SKIP" },
                    self.chunk_id
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
            + "\n"
    }

    #[inline(always)]
    pub fn log_discard(&mut self, reason: u8, data: &[u64]) {
        self.log.push((reason, data[zisk_common::STEP], 0, 0, 0));
    }

    #[inline(always)]
    pub fn log_collect(&mut self, rows: u32, data: &[u64], skip: u32, max_count: u32) {
        self.log.push((0, data[zisk_common::STEP], rows, skip, max_count));
    }

    #[inline(always)]
    pub fn log_discard_cond(&mut self, cond: bool, reason: u8, data: &[u64], result: bool) -> bool {
        self.log.push((reason + cond as u8, data[STEP], 0, 0, 0));
        result
    }
}
