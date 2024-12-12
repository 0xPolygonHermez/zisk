use std::{collections::HashMap, sync::RwLock};

use p3_field::Field;

use crate::AirInstance;

// #[derive(Default)]
// pub struct InstancesInfo {
//     pub my_groups: Vec<Vec<usize>>,
//     pub my_air_groups: Vec<Vec<usize>>,
// }
pub struct AirInstancesRepository<F> {
    pub air_instances: RwLock<Vec<AirInstance<F>>>,
    pub air_instances_counts: RwLock<HashMap<(usize, usize), usize>>,
    // pub instances_info: RwLock<InstancesInfo>,
}

impl<F: Field> Default for AirInstancesRepository<F> {
    fn default() -> Self {
        Self::new()
    }
}

impl<F: Field> AirInstancesRepository<F> {
    pub fn new() -> Self {
        AirInstancesRepository {
            air_instances: RwLock::new(Vec::new()),
            air_instances_counts: RwLock::new(HashMap::new()),
            // instances_info: RwLock::new(InstancesInfo::default()),
        }
    }

    pub fn add_air_instance(&self, mut air_instance: AirInstance<F>, global_idx: Option<usize>) {
        let mut air_instances = self.air_instances.write().unwrap();
        let n_air_instances = air_instances.len();

        let mut air_instances_counts = self.air_instances_counts.write().unwrap();
        let instance_id = air_instances_counts.entry((air_instance.airgroup_id, air_instance.air_id)).or_insert(0);
        air_instance.set_air_instance_id(*instance_id, n_air_instances);
        if global_idx.is_some() {
            air_instance.global_idx = global_idx;
        } else {
            air_instance.global_idx = Some(n_air_instances);
        }
        *instance_id += 1;
        air_instances.push(air_instance);
    }

    pub fn find_airgroup_instances(&self, airgroup_id: usize) -> Vec<usize> {
        let air_instances = self.air_instances.read().unwrap();

        let mut indices = Vec::new();
        for (index, air_instance) in air_instances.iter().enumerate() {
            if air_instance.airgroup_id == airgroup_id {
                indices.push(index);
            }
        }
        indices
    }

    pub fn find_air_instances(&self, airgroup_id: usize, air_id: usize) -> Vec<usize> {
        let air_instances = self.air_instances.read().unwrap();

        let mut indices = Vec::new();
        for (index, air_instance) in air_instances.iter().enumerate() {
            if air_instance.airgroup_id == airgroup_id && air_instance.air_id == air_id {
                indices.push(index);
            }
        }

        indices
    }

    // pub fn calculate_my_groups(&self) {
    //     let mut group_indices: BTreeMap<usize, Vec<usize>> = BTreeMap::new();

    //     let air_instances = self.air_instances.read().unwrap();

    //     let mut instances_info = self.instances_info.write().unwrap();

    //     // Populate the HashMap based on group_id and buffer positions
    //     for (idx, instance) in air_instances.iter().enumerate() {
    //         #[cfg(feature = "distributed")]
    //         let pos_buffer =
    //             self.roots_gatherv_displ[self.instances_owner[idx].0] as usize + self.instances_owner[idx].1 * 4;
    //         #[cfg(not(feature = "distributed"))]
    //         let pos_buffer = idx * 4;
    //         group_indices.entry(instance.airgroup_id).or_default().push(pos_buffer);
    //     }

    //     for (_, indices) in group_indices {
    //         instances_info.my_groups.push(indices);
    //     }

    //     let mut my_air_groups_indices: HashMap<(usize, usize), Vec<usize>> = HashMap::new();
    //     for (loc_idx, air_instance) in air_instances.iter().enumerate() {
    //         my_air_groups_indices.entry((air_instance.airgroup_id, air_instance.air_id)).or_default().push(loc_idx);
    //     }

    //     for (_, indices) in my_air_groups_indices {
    //         instances_info.my_air_groups.push(indices);
    //     }

    //     println!("// MY AIR GROUPS {:?} // MY GROUPS {:?}", instances_info.my_air_groups, instances_info.my_groups);

    // }

    pub fn find_last_segment(&self, airgroup_id: usize, air_id: usize) -> Option<usize> {
        let air_instances = self.air_instances.read().unwrap();

        air_instances
            .iter()
            .filter(|air_instance| {
                air_instance.airgroup_id == airgroup_id
                    && air_instance.air_id == air_id
                    && air_instance.air_segment_id.is_some()
            })
            .map(|air_instance| air_instance.air_segment_id.unwrap_or(0))
            .max()
    }
}
