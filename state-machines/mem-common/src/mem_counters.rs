use rayon::prelude::*;
#[cfg(feature = "save_mem_bus_data")]
use std::{env, io::Write, slice};

use std::{
    collections::{HashMap, VecDeque},
    fs::File,
    io::Read,
};
use zisk_common::ChunkId;

use crate::{MemAlignCounters, MemHelpers};
use std::fmt;
use zisk_common::{BusDevice, BusId, MemBusData, Metrics, MEM_BUS_DATA_SIZE, MEM_BUS_ID};

const ST_BITS_OFFSET: u32 = 30;
const ST_INI: u8 = 0;
const ST_READ: u8 = 1;
const ST_WRITE: u8 = ST_READ + 1;

const ST_INI_TO_READ: u32 = (ST_READ as u32) << ST_BITS_OFFSET;
const ST_INI_TO_WRITE: u32 = (ST_WRITE as u32) << ST_BITS_OFFSET;
const ST_READ_TO_WRITE: u32 = ((ST_WRITE - ST_READ) as u32) << ST_BITS_OFFSET;
const ST_X_TO_INI_MASK: u32 = 0xFFFF_FFFF >> (32 - ST_BITS_OFFSET);

#[derive(Default)]
pub struct MemCounters {
    pub addr: HashMap<u32, u32>,
    pub addr_sorted: [Vec<(u32, u32)>; 3],
    pub mem_align_counters: MemAlignCounters,
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
            file: None,
            mem_align_counters: MemAlignCounters::default(),
        }
    }
    pub fn to_array(&self) -> [u32; 5] {
        self.mem_align_counters.to_array()
    }
    pub fn close(&mut self) {
        // address must be ordered
        let mut addr_vector: Vec<(u32, u32)> = std::mem::take(&mut self.addr)
            .into_par_iter()
            .map(|(k, counter)| (k, Self::close_st_counter(counter)))
            .collect();
        addr_vector.par_sort_by_key(|(key, _)| *key);

        // Divide the original vector into three parts
        let point = addr_vector.partition_point(|x| x.0 < (0xA000_0000 / 8));
        self.addr_sorted[2] = addr_vector.split_off(point);

        let point = addr_vector.partition_point(|x| x.0 < (0x9000_0000 / 8));
        self.addr_sorted[1] = addr_vector.split_off(point);

        self.addr_sorted[0] = addr_vector;
    }
    #[inline(always)]
    fn incr_st_counter_aligned(count: u32, is_write: bool) -> u32 {
        match (count >> ST_BITS_OFFSET) as u8 {
            ST_INI => {
                if is_write {
                    // this write could be compacted on dual operation write-read
                    // don't increase the count, just change the state
                    count + ST_INI_TO_WRITE
                } else {
                    // this read could be compacted on dual operation read-read
                    // don't increase the count, just change the state
                    count + ST_INI_TO_READ
                }
            }
            ST_READ => {
                if is_write {
                    // this write means that the previous read cannot be compacted, increase
                    // the counter by this previous read and change state to write with
                    // hope that this write could be compacted on dual operation write-read
                    (count & ST_X_TO_INI_MASK) + 1 + ST_INI_TO_WRITE
                } else {
                    // this read was compacted on dual operation read-read
                    // increase the count for this dual operation and reset the state
                    (count & ST_X_TO_INI_MASK) + 1
                }
            }
            ST_WRITE => {
                if is_write {
                    // this write means that the previous write cannot be compacted, increase
                    // the counter by this previous write and continue in same state
                    // hope that this write could be compacted on dual operation write-read
                    count + 1
                } else {
                    // this read was compacted on dual operation read-read
                    // increase the count for this dual operation and reset the state
                    (count & ST_X_TO_INI_MASK) + 1
                }
            }
            _ => {
                panic!("incr_st_counter_aligned::Invalid count state 0x{count:08X}")
            }
        }
    }
    #[inline(always)]
    fn incr_st_counter_unaligned(count: u32, is_write: bool) -> u32 {
        // in this case the operation are:
        //  - is_write = false => READ
        //  - is_write = true  => READ + WRITE
        match (count >> ST_BITS_OFFSET) as u8 {
            ST_INI => {
                if is_write {
                    // [read + write], the first read operation cannot be compacted, increase the
                    // counter by first read, and change state to write by second write.
                    count + 1 + ST_INI_TO_WRITE
                } else {
                    // this read could be compacted on dual operation read-read
                    // don't increase the count, just change the state
                    count + ST_INI_TO_READ
                }
            }
            ST_READ => {
                if is_write {
                    // [read + write], means that the previous read could be compacted, increase
                    // the counter by this read-read operation, change state to write by second write.
                    count + 1 + ST_READ_TO_WRITE
                } else {
                    // this read was compacted on dual operation read-read
                    // increase the count for this dual operation and reset the state
                    (count & ST_X_TO_INI_MASK) + 1
                }
            }
            ST_WRITE => {
                if is_write {
                    // [read + write], this write means that the previous write cannot be compacted,
                    // increase the counter by this previous write and continue in same state
                    // hope that this write could be compacted on dual operation write-read
                    count + 1
                } else {
                    // this read was compacted on dual operation read-read
                    // increase the count for this dual operation and reset the state
                    (count & ST_X_TO_INI_MASK) + 1
                }
            }
            _ => {
                panic!("incr_st_counter_unaligned::Invalid count state 0x{count:08X}")
            }
        }
    }
    fn close_st_counter(count: u32) -> u32 {
        match (count >> ST_BITS_OFFSET) as u8 {
            ST_INI => count,
            ST_READ | ST_WRITE => (count & ST_X_TO_INI_MASK) + 1,
            _ => panic!("Invalid count state"),
        }
    }
    fn mem_measure(&mut self, data: &[u64]) {
        let addr = MemBusData::get_addr(data);
        let bytes = MemBusData::get_bytes(data);
        let addr_w = MemHelpers::get_addr_w(addr);
        let is_dual = MemHelpers::is_dual(addr);

        if MemHelpers::is_aligned(addr, bytes) {
            if is_dual {
                let is_write = MemHelpers::is_write(MemBusData::get_op(data));
                self.addr
                    .entry(addr_w)
                    .and_modify(|count| {
                        *count = Self::incr_st_counter_aligned(*count, is_write);
                    })
                    .or_insert(if is_write { ST_INI_TO_WRITE } else { ST_INI_TO_READ });
            } else {
                self.addr.entry(addr_w).and_modify(|count| *count += 1).or_insert(1);
            }
        } else {
            let is_write = MemHelpers::is_write(MemBusData::get_op(data));
            let addr_count = if MemHelpers::is_double(addr, bytes) { 2 } else { 1 };
            let is_full = bytes != 1
                || if is_write {
                    if (MemBusData::get_value(data) & 0xFFFF_FFFF_FFFF_FF00) == 0 {
                        self.mem_align_counters.write_byte += 1;
                        false
                    } else {
                        true
                    }
                } else {
                    self.mem_align_counters.read_byte += 1;
                    false
                };

            let ops_by_addr = if is_write { 2 } else { 1 };

            if is_dual {
                let initial_value = if is_write { ST_INI_TO_READ + 1 } else { ST_INI_TO_READ };
                for index in 0..addr_count {
                    self.addr
                        .entry(addr_w + index)
                        .and_modify(|count| {
                            *count = Self::incr_st_counter_unaligned(*count, is_write)
                        })
                        .or_insert(initial_value);
                }
            } else {
                for index in 0..addr_count {
                    self.addr
                        .entry(addr_w + index)
                        .and_modify(|count| *count += ops_by_addr)
                        .or_insert(ops_by_addr);
                }
            }

            let mem_align_op_rows = 1 + addr_count * ops_by_addr;

            if is_full {
                match mem_align_op_rows {
                    2 => self.mem_align_counters.full_2 += 1,
                    3 => self.mem_align_counters.full_3 += 1,
                    5 => self.mem_align_counters.full_5 += 1,
                    _ => panic!("Invalid mem_align_op_rows"),
                }
            }
        }
    }

    #[cfg(feature = "save_mem_bus_data")]
    #[allow(dead_code)]
    pub fn save_to_file(&mut self, chunk_id: ChunkId, data: &[u64]) {
        if self.file.is_none() {
            let path = env::var("BUS_DATA_DIR").unwrap_or("tmp/bus_data".to_string());
            self.file =
                Some(File::create(format!("{path}/mem_{:04}.bin", usize::from(chunk_id))).unwrap());
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
            self.file = Some(
                File::create(format!("{path}/mem_count_data_{:04}.bin", usize::from(chunk_id)))
                    .unwrap(),
            );
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
        let mut file = File::open(format!("tmp/bus_data/mem_{:04}.bin", usize::from(chunk_id)))?;
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
            // TODO: dynamic chunk_size
            let chunk_id =
                MemHelpers::static_mem_step_to_chunk(MemBusData::get_step(data), 1 << 18);
            // self.save_to_compact_file(chunk_id, data);
            self.save_to_file(chunk_id, data);
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
