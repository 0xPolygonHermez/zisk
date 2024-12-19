use crate::ArithCounter;
use sm_common::{
    plan, BusDeviceMetrics, ChunkId, InstCount, InstanceInfo, InstanceType, Plan, Planner,
    TableInfo,
};

#[derive(Default)]
pub struct ArithPlanner {
    instances_info: Vec<InstanceInfo>,
    tables_info: Vec<TableInfo>,
}

impl ArithPlanner {
    pub fn new() -> Self {
        Self { instances_info: Vec::new(), tables_info: Vec::new() }
    }

    pub fn add_instance(mut self, instance_info: InstanceInfo) -> Self {
        self.instances_info.push(instance_info);
        self
    }

    pub fn add_table_instance(mut self, table_info: TableInfo) -> Self {
        self.tables_info.push(table_info);
        self
    }
}

impl Planner for ArithPlanner {
    fn plan(&self, counters: Vec<(ChunkId, Box<dyn BusDeviceMetrics>)>) -> Vec<Plan> {
        // Prepare counts
        let mut count: Vec<Vec<InstCount>> = Vec::with_capacity(self.instances_info.len());

        for _ in 0..self.instances_info.len() {
            count.push(Vec::new());
        }

        counters.iter().for_each(|(chunk_id, counter)| {
            let reg_counter = counter.as_any().downcast_ref::<ArithCounter>().unwrap();

            // Iterate over `instances_info` and add `InstCount` objects to the correct vector
            for (index, instance_info) in self.instances_info.iter().enumerate() {
                let inst_count = InstCount::new(
                    *chunk_id,
                    reg_counter.inst_count(instance_info.op_type).unwrap(),
                );

                // Add the `InstCount` to the corresponding inner vector
                count[index].push(inst_count);
            }
        });

        let mut plan_result = Vec::new();

        for (idx, instance) in self.instances_info.iter().enumerate() {
            let plan: Vec<_> = plan(&count[idx], instance.num_rows as u64)
                .into_iter()
                .map(|check_point| {
                    Plan::new(
                        instance.airgroup_id,
                        instance.air_id,
                        None,
                        InstanceType::Instance,
                        Some(check_point),
                        None,
                    )
                })
                .collect();

            plan_result.extend(plan);
        }

        for table_instance in self.tables_info.iter() {
            plan_result.push(Plan::new(
                table_instance.airgroup_id,
                table_instance.air_id,
                None,
                InstanceType::Table,
                None,
                None,
            ));
        }

        plan_result
    }
}
