use fields::Goldilocks;
use rayon::prelude::*;
#[cfg(feature = "save_mem_counters")]
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use zisk_common::{BusDeviceMetrics, ChunkId, Metrics, Plan, Planner};

use zisk_pil::{
    InputDataTrace, MemTrace, RomDataTrace, INPUT_DATA_AIR_IDS, MEM_AIR_IDS, ROM_DATA_AIR_IDS,
    ZISK_AIRGROUP_ID,
};

#[cfg(any(feature = "save_mem_plans", feature = "save_mem_bus_data"))]
use mem_common::save_plans;

use crate::{
    MemModulePlanner, MemModulePlannerConfig, INPUT_DATA_W_ADDR_INIT, ROM_DATA_W_ADDR_INIT,
};

use mem_common::{MemAlignPlanner, MemCounters, RAM_W_ADDR_INIT};

#[cfg(feature = "save_mem_counters")]
use mem_common::MemAlignCounters;

#[cfg(feature = "save_mem_counters")]
#[derive(Clone)]
pub struct SerializableMemCounters {
    pub addr: HashMap<u32, u32>,
    pub addr_sorted: [Vec<(u32, u32)>; 3],
    pub mem_align_counters: MemAlignCounters,
}

#[cfg(feature = "save_mem_counters")]
impl From<&MemCounters> for SerializableMemCounters {
    fn from(counters: &MemCounters) -> Self {
        Self {
            addr: counters.addr.clone(),
            addr_sorted: counters.addr_sorted.clone(),
            mem_align_counters: counters.mem_align_counters,
        }
    }
}

#[cfg(feature = "save_mem_counters")]
impl From<SerializableMemCounters> for MemCounters {
    fn from(serializable: SerializableMemCounters) -> Self {
        Self {
            addr: serializable.addr,
            addr_sorted: serializable.addr_sorted,
            mem_align_counters: serializable.mem_align_counters,
            file: None,
        }
    }
}

pub trait MemPlanCalculator {
    fn plan(&mut self);
    fn collect_plans(&mut self) -> Vec<Plan>;
}

#[derive(Default)]
pub struct DummyMemPlanner {}

impl DummyMemPlanner {
    pub fn new() -> Self {
        Self {}
    }
}

impl Planner for DummyMemPlanner {
    fn plan(&self, _metrics: Vec<(ChunkId, Box<dyn BusDeviceMetrics>)>) -> Vec<Plan> {
        vec![]
    }
}

#[derive(Default)]
pub struct MemPlanner {}

impl MemPlanner {
    pub fn new() -> Self {
        Self {}
    }

    #[cfg(feature = "save_mem_counters")]
    pub fn save_counters_to_file(
        &self,
        counters_data: Arc<Vec<(ChunkId, &MemCounters)>>,
        filename: &str,
    ) -> std::io::Result<()> {
        use std::io::Write;

        let path = std::env::var("BUS_DATA_DIR").unwrap_or("tmp/bus_data".to_string());
        std::fs::create_dir_all(&path)?;
        let full_path = format!("{}/{}", path, filename);

        let mut file = std::fs::File::create(&full_path)?;

        // Save number of chunks
        let chunk_count = counters_data.len() as u32;
        file.write_all(&chunk_count.to_le_bytes())?;

        let mut counters_to_save = Vec::new();

        for (chunk_id, counters) in counters_data.iter() {
            // Save chunk_id
            file.write_all(&(chunk_id.0 as u32).to_le_bytes())?;

            // Check if addr HashMap is empty (data moved to addr_sorted) or still has data
            let has_addr_data = !counters.addr.is_empty();
            file.write_all(&(has_addr_data as u8).to_le_bytes())?;

            if has_addr_data {
                // Save addr HashMap (original data)
                let addr_len = counters.addr.len() as u32;
                file.write_all(&addr_len.to_le_bytes())?;
                for (k, v) in &counters.addr {
                    file.write_all(&k.to_le_bytes())?;
                    file.write_all(&v.to_le_bytes())?;
                }

                // Save empty addr_sorted arrays for consistency
                for _ in 0..3 {
                    file.write_all(&0u32.to_le_bytes())?;
                }
            } else {
                // Save empty addr HashMap
                file.write_all(&0u32.to_le_bytes())?;

                // Save addr_sorted arrays (processed data)
                for sorted_array in &counters.addr_sorted {
                    let len = sorted_array.len() as u32;
                    file.write_all(&len.to_le_bytes())?;
                    for (k, v) in sorted_array {
                        file.write_all(&k.to_le_bytes())?;
                        file.write_all(&v.to_le_bytes())?;
                    }
                }
            }

            // Save mem_align_counters
            file.write_all(&counters.mem_align_counters.chunk_id.to_le_bytes())?;
            file.write_all(&counters.mem_align_counters.full_2.to_le_bytes())?;
            file.write_all(&counters.mem_align_counters.full_3.to_le_bytes())?;
            file.write_all(&counters.mem_align_counters.full_5.to_le_bytes())?;
            file.write_all(&counters.mem_align_counters.read_byte.to_le_bytes())?;
            file.write_all(&counters.mem_align_counters.write_byte.to_le_bytes())?;

            // Store a copy for potential reuse (create serializable version)
            let serializable_counters = SerializableMemCounters::from(*counters);
            counters_to_save.push((*chunk_id, serializable_counters));
        }

        println!("Saved {} memory counters to {}", chunk_count, full_path);
        Ok(())
    }

    #[cfg(feature = "save_mem_counters")]
    pub fn load_counters_from_file(
        filename: &str,
    ) -> std::io::Result<Arc<Vec<(ChunkId, MemCounters)>>> {
        use std::io::Read;

        let path = std::env::var("BUS_DATA_DIR").unwrap_or("tmp/bus_data".to_string());
        let full_path = format!("{}/{}", path, filename);

        let mut file = std::fs::File::open(&full_path)?;
        let mut buf = [0u8; 4];

        // Read number of chunks
        file.read_exact(&mut buf)?;
        let chunk_count = u32::from_le_bytes(buf);

        let mut result = Vec::new();

        for _ in 0..chunk_count {
            // Read chunk_id
            file.read_exact(&mut buf)?;
            let chunk_id = ChunkId(u32::from_le_bytes(buf) as usize);

            // Read whether addr HashMap has data
            let mut bool_buf = [0u8; 1];
            file.read_exact(&mut bool_buf)?;
            let has_addr_data = bool_buf[0] != 0;

            let (addr, addr_sorted) = if has_addr_data {
                // Read addr HashMap (original data)
                file.read_exact(&mut buf)?;
                let addr_len = u32::from_le_bytes(buf);
                let mut addr = HashMap::new();
                for _ in 0..addr_len {
                    file.read_exact(&mut buf)?;
                    let k = u32::from_le_bytes(buf);
                    file.read_exact(&mut buf)?;
                    let v = u32::from_le_bytes(buf);
                    addr.insert(k, v);
                }

                // Read empty addr_sorted arrays
                for _ in 0..3 {
                    file.read_exact(&mut buf)?;
                    let _len = u32::from_le_bytes(buf);
                }

                (addr, [Vec::new(), Vec::new(), Vec::new()])
            } else {
                // Read empty addr HashMap
                file.read_exact(&mut buf)?;
                let _addr_len = u32::from_le_bytes(buf);

                // Read addr_sorted arrays (processed data)
                let mut addr_sorted = [Vec::new(), Vec::new(), Vec::new()];
                for i in 0..3 {
                    file.read_exact(&mut buf)?;
                    let len = u32::from_le_bytes(buf);
                    for _ in 0..len {
                        file.read_exact(&mut buf)?;
                        let k = u32::from_le_bytes(buf);
                        file.read_exact(&mut buf)?;
                        let v = u32::from_le_bytes(buf);
                        addr_sorted[i].push((k, v));
                    }
                }

                (HashMap::new(), addr_sorted)
            };

            // Read mem_align_counters
            let mut read_u32 = || -> std::io::Result<u32> {
                file.read_exact(&mut buf)?;
                Ok(u32::from_le_bytes(buf))
            };

            let mem_align_counters = MemAlignCounters {
                chunk_id: read_u32()?,
                full_2: read_u32()?,
                full_3: read_u32()?,
                full_5: read_u32()?,
                read_byte: read_u32()?,
                write_byte: read_u32()?,
            };

            let serializable_counters =
                SerializableMemCounters { addr, addr_sorted, mem_align_counters };

            let counters = MemCounters::from(serializable_counters);

            result.push((chunk_id, counters));
        }

        println!("Loaded {} memory counters from {}", chunk_count, full_path);
        Ok(Arc::new(result))
    }

    #[cfg(feature = "save_mem_counters")]
    fn generate_plans_from_serializable_counters(
        &self,
        saved_counters: &[(ChunkId, SerializableMemCounters)],
    ) -> Vec<Plan> {
        // Convert SerializableMemCounters to owned MemCounters for processing
        let owned_counters: Vec<(ChunkId, MemCounters)> = saved_counters
            .iter()
            .map(|(chunk_id, serializable_counters)| {
                let counters = MemCounters::from(serializable_counters.clone());
                (*chunk_id, counters)
            })
            .collect();

        // Convert to references for the existing planning logic
        let counters_refs: Vec<(ChunkId, &MemCounters)> =
            owned_counters.iter().map(|(chunk_id, counters)| (*chunk_id, counters)).collect();

        let counters = Arc::new(counters_refs);
        self.generate_plans_from_counters(counters)
    }

    #[cfg(feature = "save_mem_counters")]
    pub fn generate_plans_from_file(&self, filename: &str) -> std::io::Result<Vec<Plan>> {
        let loaded_counters = Self::load_counters_from_file(filename)?;

        // Convert owned MemCounters to references for planning
        let counters_refs: Vec<(ChunkId, &MemCounters)> =
            loaded_counters.iter().map(|(chunk_id, counters)| (*chunk_id, counters)).collect();

        let counters = Arc::new(counters_refs);
        Ok(self.generate_plans_from_counters(counters))
    }

    pub fn generate_plans(&self, metrics: Vec<(ChunkId, Box<dyn BusDeviceMetrics>)>) -> Vec<Plan> {
        // convert generic information to specific information
        let mut counters: Vec<(ChunkId, &MemCounters)> = metrics
            .iter()
            .map(|(chunk_id, metric)| {
                (*chunk_id, Metrics::as_any(&**metric).downcast_ref::<MemCounters>().unwrap())
            })
            .collect();
        counters.par_sort_by_key(|(chunk_id, _)| *chunk_id);
        let counters = Arc::new(counters);
        #[cfg(feature = "save_mem_counters")]
        self.save_counters_to_file(counters.clone(), "mem_counters.bin");
        self.generate_plans_from_counters(counters)
    }

    pub fn generate_plans_from_counters(
        &self,
        counters: Arc<Vec<(ChunkId, &MemCounters)>>,
    ) -> Vec<Plan> {
        let mem_planner = Arc::new(Mutex::new(MemModulePlanner::new(
            MemModulePlannerConfig {
                airgroup_id: ZISK_AIRGROUP_ID,
                air_id: MEM_AIR_IDS[0],
                addr_index: 2,
                from_addr: RAM_W_ADDR_INIT,
                last_addr: RAM_W_ADDR_INIT,
                rows: MemTrace::<Goldilocks>::NUM_ROWS as u32,
                consecutive_addr: false,
            },
            counters.clone(),
        )));

        let rom_data_planner = Arc::new(Mutex::new(MemModulePlanner::new(
            MemModulePlannerConfig {
                airgroup_id: ZISK_AIRGROUP_ID,
                air_id: ROM_DATA_AIR_IDS[0],
                addr_index: 0,
                from_addr: ROM_DATA_W_ADDR_INIT,
                last_addr: ROM_DATA_W_ADDR_INIT - 1,
                rows: RomDataTrace::<Goldilocks>::NUM_ROWS as u32,
                consecutive_addr: true,
            },
            counters.clone(),
        )));

        let input_data_planner = Arc::new(Mutex::new(MemModulePlanner::new(
            MemModulePlannerConfig {
                airgroup_id: ZISK_AIRGROUP_ID,
                air_id: INPUT_DATA_AIR_IDS[0],
                addr_index: 1,
                from_addr: INPUT_DATA_W_ADDR_INIT,
                last_addr: INPUT_DATA_W_ADDR_INIT,
                rows: InputDataTrace::<Goldilocks>::NUM_ROWS as u32,
                consecutive_addr: true,
            },
            counters.clone(),
        )));
        let mut mem_align_planner = MemAlignPlanner::new(counters.clone());

        let planners = vec![
            Arc::clone(&mem_planner),
            Arc::clone(&rom_data_planner),
            Arc::clone(&input_data_planner),
        ];

        planners.par_iter().for_each(|plan| {
            let mut locked_plan = plan.lock().unwrap();
            locked_plan.plan();
        });
        mem_align_planner.plan();

        let mut plans: Vec<Plan> = Vec::new();
        plans.append(&mut mem_planner.lock().unwrap().collect_plans());
        plans.append(&mut rom_data_planner.lock().unwrap().collect_plans());
        plans.append(&mut input_data_planner.lock().unwrap().collect_plans());
        plans.append(&mut mem_align_planner.collect_plans());

        #[cfg(any(feature = "save_mem_plans", feature = "save_mem_bus_data"))]
        save_plans(&plans, "plans.txt");
        plans
    }
}

impl Planner for MemPlanner {
    fn plan(&self, metrics: Vec<(ChunkId, Box<dyn BusDeviceMetrics>)>) -> Vec<Plan> {
        self.generate_plans(metrics)
    }
}
