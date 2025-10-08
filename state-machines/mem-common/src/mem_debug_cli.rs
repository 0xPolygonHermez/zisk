use clap::Parser;
use fields::Goldilocks;
use mem_common::MemHelpers;
use mem_common::{MemCounters, MemDebug};
use std::path::PathBuf;
use zisk_common::ChunkId;
use zisk_common::MemBusData;
use zisk_pil::MemTrace;

/// Inspect mem_{chunk}.bin files produced with feature save_mem_bus_data
#[derive(Parser, Debug)]
#[command(name = "mem-chunk-view", version, about = "View decoded memory bus chunk records")]
struct Opts {
    /// First chunk id
    #[arg(long, default_value_t = 0)]
    from: u32,
    /// Last chunk id (inclusive). If omitted uses --from only.
    #[arg(long)]
    to: Option<u32>,
    /// Base directory (current load_from_file uses fixed path; optional alt root)
    #[arg(long)]
    dir: Option<PathBuf>,
}

const COSTS: [usize; 5] = [22, 28, 33, 38, 43];
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts = Opts::parse();

    if opts.dir.is_some() {
        eprintln!("Warning: --dir ignored (load_from_file has fixed path).");
    }

    let mut total = 0usize;

    let mut debug = MemDebug::new(0xA000_0000, 0xFFFF_FFFF, true);
    let mut chunk_id = 0;
    loop {
        let Ok(bus) = MemCounters::load_from_file(ChunkId(chunk_id as usize)) else {
            break;
        };
        println!("Loading chunk {chunk_id} ..... *");
        for data in &bus {
            total += 1;
            let addr = MemBusData::get_addr(data);
            let op = MemBusData::get_op(data);
            let is_write = MemHelpers::is_write(op);

            let step = MemBusData::get_step(data);
            let bytes = MemBusData::get_bytes(data);
            if addr >= 0xC000_0000 {
                println!("ADDR 0x{addr:08X}");
            }
            if data[5] != 0 {
                println!("DATA {data:?}");
            }
            debug.log(addr, step, bytes, is_write, false, data);
        }
        chunk_id += 1;
    }
    println!("Preparing data (sort) ....");
    debug.prepare();
    println!("Preparation data is done");
    let count = debug.get_total();
    let area_wo_duals = count * COSTS[0];
    for (dual, cost) in COSTS.iter().enumerate().skip(1) {
        let (dual_rows, dual_count) = debug.count_n_dual(dual);
        let rows_w_duals = count - dual_count as usize + dual_rows as usize;
        let area_w_duals = rows_w_duals * cost;
        println!("area_w_duals:{area_w_duals} area_wo_duals:{area_wo_duals}");
        println!(
            "count_dual({dual})=({dual_rows},{dual_count})  (reduction area:{:.2}%  rows:{:.2}%)",
            ((area_wo_duals as i32 - area_w_duals as i32) as f32 * 100.0) / area_wo_duals as f32,
            ((dual_count - dual_rows) as f32 * 100.0) / count as f32
        );
    }
    println!("Preparing dual data ....");
    debug.apply_dual();
    println!("Preparation dual data is done");
    let direct = debug.get_direct();
    let indirect = debug.get_indirect();
    let dual = debug.get_dual();
    let area_w_duals = (count - dual) * COSTS[1];
    println!("{area_wo_duals} {area_w_duals}");
    println!(
        "Total records processed: {total}. Total operations: {count}, Direct: {direct}, Indirect: {indirect} Dual: {dual} (reduction area:{:.2}%  rows:{:.2}%)",
        ((area_wo_duals - area_w_duals) as f32 * 100.0) / area_wo_duals as f32,
            (dual as f32 * 100.0) / count as f32
    );
    let num_rows = MemTrace::<Goldilocks>::NUM_ROWS;
    debug.info_instances(num_rows);
    debug.info_chunks(num_rows);
    debug.dump_to_file(num_rows, "tmp/mem_debug_cli_ops.txt");
    // debug.save_to_file(num_rows, "tmp/debug_mem.txt");
    Ok(())
}
