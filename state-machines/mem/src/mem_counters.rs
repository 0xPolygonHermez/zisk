use rayon::prelude::*;
#[cfg(feature = "save_mem_bus_data")]
use std::{env, io::Write, slice};

use std::{
    collections::{HashMap, VecDeque},
    fs::File,
    io::Read,
};
use zisk_common::ChunkId;

use crate::MemHelpers;
use std::fmt;
use zisk_common::{BusDevice, BusId, MemBusData, Metrics, MEM_BUS_DATA_SIZE, MEM_BUS_ID};

// inside a chunk no more than 2^32 access by one address

#[derive(Default)]
pub struct MemCounters {
    pub addr: HashMap<u32, u32>,
    pub addr_sorted: [Vec<(u32, u32)>; 3],
    pub mem_align: Vec<u8>,
    pub mem_align_rows: u32,
    pub file: Option<File>,
}

impl fmt::Debug for MemCounters {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.addr.is_empty() {
            for (mem_index, mem_area) in self.addr_sorted.iter().enumerate() {
                write!(f, "[MEM_{},#:{} =>", mem_index, mem_area.len())?;
                for (addr, count) in mem_area {
                    write!(f, " 0x{:08X}:{}", addr * 8, count)?;
                }
                write!(f, "]")?;
            }
        } else {
            write!(f, "[HASH,#:{} =>", self.addr.len())?;
            for (addr, count) in self.addr.iter() {
                write!(f, " 0x{:08X}:{}", addr * 8, count)?;
            }
            write!(f, "]")?;
        }
        Ok(())
    }
}

impl MemCounters {
    pub fn new() -> Self {
        Self {
            addr: HashMap::new(),
            addr_sorted: [Vec::new(), Vec::new(), Vec::new()],
            mem_align: Vec::new(),
            mem_align_rows: 0,
            file: None,
        }
    }

    pub fn close(&mut self) {
        // address must be ordered
        let mut addr_vector: Vec<(u32, u32)> = std::mem::take(&mut self.addr).into_iter().collect();
        addr_vector.par_sort_by_key(|(key, _)| *key);

        // Divideix el vector original en tres parts
        let point = addr_vector.partition_point(|x| x.0 < (0xA000_0000 / 8));
        self.addr_sorted[2] = addr_vector.split_off(point);

        let point = addr_vector.partition_point(|x| x.0 < (0x9000_0000 / 8));
        self.addr_sorted[1] = addr_vector.split_off(point);

        self.addr_sorted[0] = addr_vector;
    }
    #[inline(always)]
    fn mem_measure(&mut self, data: &[u64]) {
        let addr = MemBusData::get_addr(data);
        let addr_w = MemHelpers::get_addr_w(addr);
        let bytes = MemBusData::get_bytes(data);

        if MemHelpers::is_aligned(addr, bytes) {
            self.addr.entry(addr_w).and_modify(|count| *count += 1).or_insert(1);
        } else {
            let op = MemBusData::get_op(data);
            let addr_count = if MemHelpers::is_double(addr, bytes) { 2 } else { 1 };
            let ops_by_addr = if MemHelpers::is_write(op) { 2 } else { 1 };

            for index in 0..addr_count {
                self.addr
                    .entry(addr_w + index)
                    .and_modify(|count| *count += ops_by_addr)
                    .or_insert(ops_by_addr);
            }
            let mem_align_op_rows = 1 + addr_count * ops_by_addr;
            self.mem_align.push(mem_align_op_rows as u8);
            self.mem_align_rows += mem_align_op_rows;
        }
    }
    #[cfg(feature = "save_mem_bus_data")]
    #[allow(dead_code)]
    pub fn save_to_file(&mut self, chunk_id: ChunkId, data: &[u64]) {
        if self.file.is_none() {
            let path = env::var("BUS_DATA_DIR").unwrap_or("tmp/bus_data".to_string());
            self.file = Some(File::create(format!("{}/mem_{:04}.bin", path, chunk_id)).unwrap());
        }
        let bytes = unsafe {
            slice::from_raw_parts(data.as_ptr() as *const u8, MEM_BUS_DATA_SIZE * size_of::<u64>())
        };
        self.file.as_mut().unwrap().write_all(bytes).unwrap();
    }

    #[cfg(feature = "save_mem_bus_data")]
    #[allow(dead_code)]
    pub fn save_to_compact_file(&mut self, chunk_id: ChunkId, data: &[u64]) {
        if self.file.is_none() {
            let path = env::var("BUS_DATA_DIR").unwrap_or("tmp/bus_data".to_string());
            self.file =
                Some(File::create(format!("{}/mem_count_data_{:04}.bin", path, chunk_id)).unwrap());
        }
        let values: [u32; 2] = [
            MemBusData::get_addr(data),
            // ((MemBusData::get_bytes(data) as u32) << 28)
            MemBusData::get_bytes(data) as u32
                + ((MemHelpers::is_write(MemBusData::get_op(data)) as u32) << 16),
        ];
        let bytes =
            unsafe { slice::from_raw_parts(values.as_ptr() as *const u8, 2 * size_of::<u32>()) };
        self.file.as_mut().unwrap().write_all(bytes).unwrap();
    }
    #[allow(dead_code)]
    pub fn load_from_file(
        chunk_id: ChunkId,
    ) -> Result<Vec<[u64; MEM_BUS_DATA_SIZE]>, std::io::Error> {
        let mut file = File::open(format!("tmp/bus_data/mem_{chunk_id}.bin"))?;
        const BUS_DATA_BYTES: usize = MEM_BUS_DATA_SIZE * size_of::<u64>();
        let count = file.metadata().unwrap().len() as usize / BUS_DATA_BYTES;
        let mut buffer = [0u8; BUS_DATA_BYTES];
        let mut data: Vec<[u64; MEM_BUS_DATA_SIZE]> = Vec::new();
        let mut counters: Vec<MemCounters> = Vec::new();
        for _ in 0..count {
            file.read_exact(&mut buffer)?;
            let values = unsafe {
                std::mem::transmute::<
                    [u8; MEM_BUS_DATA_SIZE * size_of::<u64>()],
                    [u64; MEM_BUS_DATA_SIZE],
                >(buffer)
            };
            counters.push(MemCounters::new());
            data.push(values);
        }
        Ok(data)
    }
    #[allow(dead_code)]
    pub fn execute_from_vector(&mut self, data_bus: &Vec<[u64; MEM_BUS_DATA_SIZE]>) {
        for data in data_bus {
            self.mem_measure(data);
        }
    }
}

impl Metrics for MemCounters {
    #[inline(always)]
    fn measure(&mut self, data: &[u64]) {
        self.mem_measure(data);
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl BusDevice<u64> for MemCounters {
    #[inline(always)]
    fn process_data(
        &mut self,
        bus_id: &BusId,
        data: &[u64],
        _pending: &mut VecDeque<(BusId, Vec<u64>)>,
    ) -> bool {
        debug_assert!(bus_id == &MEM_BUS_ID);

        #[cfg(feature = "save_mem_bus_data")]
        {
            let chunk_id = MemHelpers::mem_step_to_chunk(MemBusData::get_step(data));
            self.save_to_compact_file(chunk_id, data);
        }

        self.measure(data);

        true
    }

    fn bus_id(&self) -> Vec<BusId> {
        vec![MEM_BUS_ID]
    }

    /// Provides a dynamic reference for downcasting purposes.
    fn as_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }

    fn on_close(&mut self) {
        self.close();
    }
}
