use std::{collections::HashMap, sync::RwLock};

use p3_field::Field;

use crate::AirInstance;

pub struct AirInstancesRepository<F> {
    pub air_instances: RwLock<Vec<AirInstance<F>>>,
    pub air_instances_counts: RwLock<HashMap<(usize, usize), usize>>,
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
        }
    }

    pub fn add_air_instance(&self, mut air_instance: AirInstance<F>, global_idx: Option<usize>) {
        let mut air_instances = self.air_instances.write().unwrap();
        let n_air_instances = air_instances.len();

        let mut air_instances_counts = self.air_instances_counts.write().unwrap();
        let instance_id = air_instances_counts.entry((air_instance.airgroup_id, air_instance.air_id)).or_insert(0);
        air_instance.set_air_instance_id(*instance_id, n_air_instances);
        air_instance.global_idx = global_idx;
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
