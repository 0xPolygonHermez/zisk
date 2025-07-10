use crate::{MemAlignCheckPoint, MemModuleSegmentCheckPoint};
use std::{env, fs};
use zisk_common::{CheckPoint, Plan, SegmentId};

pub fn save_plans(plans: &[Plan], filename: &str) {
    let path = env::var("BUS_DATA_DIR").unwrap_or("tmp/bus_data".to_string());

    let mut content = String::new();
    for plan in plans {
        if plan.meta.is_none() {
            continue;
        }
        if let Some(mem_align_cps) =
            plan.meta.as_ref().unwrap().downcast_ref::<Vec<MemAlignCheckPoint>>()
        {
            let chunks = match &plan.check_point {
                CheckPoint::Single(chunk_id) => format!("[{}]", chunk_id),
                CheckPoint::Multiple(chunks) => {
                    chunks.iter().map(|&id| id.to_string()).collect::<Vec<String>>().join(",")
                }
                _ => "[]".to_string(),
            };
            let segment_id = plan.segment_id.unwrap_or(SegmentId(0));
            for mem_align_cp in mem_align_cps {
                content += &format!(
                    "MEM_ALIGN segment_id:{} skip:{} count:{} rows:{} chuns:{}\n",
                    segment_id, mem_align_cp.skip, mem_align_cp.count, mem_align_cp.rows, chunks
                );
            }
        } else if let Some(mem_cp) =
            plan.meta.as_ref().unwrap().downcast_ref::<MemModuleSegmentCheckPoint>()
        {
            content += &mem_cp.to_string(plan.segment_id.unwrap_or(SegmentId(0)).as_usize())
        }
    }
    fs::write(format!("{}/{}", path, filename), content).expect("Unable to write plans to file");
}
