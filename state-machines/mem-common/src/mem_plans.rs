use crate::{MemAlignCheckPoint, MemModuleSegmentCheckPoint};
use std::{collections::HashMap, env, fs};
use zisk_common::{CheckPoint, ChunkId, Plan, SegmentId};

pub fn save_plans(plans: &[Plan], filename: &str) {
    let path = env::var("BUS_DATA_DIR").unwrap_or("tmp/bus_data".to_string());

    let mut content = String::new();
    for plan in plans {
        if plan.meta.is_none() {
            continue;
        }
        if let Some(mem_align_cps) =
            plan.meta.as_ref().unwrap().downcast_ref::<HashMap<ChunkId, MemAlignCheckPoint>>()
        {
            let _chunks = match &plan.check_point {
                CheckPoint::Single(chunk_id) => format!("[{chunk_id}]"),
                CheckPoint::Multiple(chunks) => {
                    chunks.iter().map(|&id| id.to_string()).collect::<Vec<String>>().join(",")
                }
                _ => "[]".to_string(),
            };
            let chunks_count = match &plan.check_point {
                CheckPoint::Single(_chunk_id) => 1,
                CheckPoint::Multiple(chunks) => chunks.len(),
                _ => 0,
            };
            let segment_id = plan.segment_id.unwrap_or(SegmentId(0));
            let air_id = plan.air_id;
            let mut rows = 0;
            let mut count = 0;
            for (chunk_id, mem_align_cp) in mem_align_cps {
                count += mem_align_cp.full_5.count()
                    + mem_align_cp.full_3.count()
                    + mem_align_cp.full_2.count()
                    + mem_align_cp.write_byte.count()
                    + mem_align_cp.read_byte.count();
                rows += mem_align_cp.full_5.count() * 5
                    + mem_align_cp.full_3.count() * 3
                    + mem_align_cp.full_2.count() * 2
                    + mem_align_cp.write_byte.count()
                    + mem_align_cp.read_byte.count();
                content += &format!(
                    "MEM_ALIGN_{air_id} #{segment_id}@{chunk_id} full_5(+{},{}) full_3(+{},{}) full_2(+{},{}) write_byte:(+{},{}) read_byte:(+{},{})\n",
                    mem_align_cp.full_5.skip(), mem_align_cp.full_5.count(), mem_align_cp.full_3.skip(), mem_align_cp.full_3.count(),
                    mem_align_cp.full_2.skip(), mem_align_cp.full_2.count(), mem_align_cp.write_byte.skip(), mem_align_cp.write_byte.count(), mem_align_cp.read_byte.skip(), mem_align_cp.read_byte.count()
                );
            }
            content += &format!("MEM_ALIGN_TOT_{air_id} #{segment_id} count:{count} rows:{rows} chunks_count:({chunks_count},{})\n", mem_align_cps.len());
        } else if let Some(mem_cp) =
            plan.meta.as_ref().unwrap().downcast_ref::<MemModuleSegmentCheckPoint>()
        {
            content += &mem_cp.to_string(plan.segment_id.unwrap_or(SegmentId(0)).as_usize())
        }
    }
    fs::write(format!("{path}/{filename}"), content).expect("Unable to write plans to file");
}
