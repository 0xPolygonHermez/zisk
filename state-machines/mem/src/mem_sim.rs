pub mod mem_constants;
pub mod mem_counters;
mod mem_helpers;
pub use mem_constants::*;
use mem_counters::*;
pub use mem_helpers::*;
use zisk_common::MEM_BUS_DATA_SIZE;
mod mem_inputs;
pub use mem_module::*;
mod mem_module;
pub use mem_inputs::*;
mod mem_sm;
pub use mem_sm::*;
mod rom_data_sm;
pub use rom_data_sm::*;
mod input_data_sm;
pub use input_data_sm::*;
mod mem_align_sm;
pub use mem_align_sm::*;
mod mem_align_rom_sm;
pub use mem_align_rom_sm::*;
mod mem_planner;
pub use mem_planner::*;
mod mem_align_planner;
pub use mem_align_planner::*;
mod mem_module_planner;
pub use mem_module_planner::*;
mod mem_counters_cursor;
pub use mem_counters_cursor::*;
// mod mem_module_check_point;
// pub use mem_module_check_point::*;
// mod mem_ops;
// pub use mem_ops::*;
use zisk_common::BusDeviceMetrics;
use zisk_common::ChunkId;

// cargo run --release --features="test_data" --bin arith_eq_test_generator

fn main() {
    let mut data: Vec<Vec<[u64; MEM_BUS_DATA_SIZE]>> = Vec::new();
    loop {
        let _chunk_id = data.len();
        println!("Loading bus data chunk {_chunk_id} ...");
        if let Ok(bus_data) = MemCounters::load_from_file(ChunkId(_chunk_id)) {
            data.push(bus_data);
        } else {
            println!("No more bus data to load.");
            break;
        }
    }
    let mut metrics: Vec<(ChunkId, Box<dyn BusDeviceMetrics>)> = Vec::new();

    for (i, data) in data.iter().enumerate() {
        println!("Executing bus data chunk {i} ...");
        let mut counter = MemCounters::new();
        counter.execute_from_vector(data);
        counter.close();
        metrics.push((ChunkId(i), Box::new(counter)));
    }
    let planner = MemPlanner::new();
    let start = std::time::Instant::now();
    let plans = planner.generate_plans(metrics);
    let elapsed = std::time::Instant::now() - start;
    println!("Elapsed time: {:?} ms", elapsed.as_millis());
    println!("Plans: {}", plans.len());
}
