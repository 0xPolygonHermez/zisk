use std::sync::RwLock;

use crate::AirInstance;

pub struct AirInstancesRepository<F> {
    pub air_instances: RwLock<Vec<AirInstance<F>>>,
}

impl Default for AirInstancesRepository<usize> {
    fn default() -> Self {
        Self::new()
    }
}

impl<F> AirInstancesRepository<F> {
    pub fn new() -> Self {
        AirInstancesRepository { air_instances: RwLock::new(Vec::new()) }
    }

    pub fn add_air_instance(&self, air_instance: AirInstance<F>) {
        self.air_instances.write().unwrap().push(air_instance);
    }

    pub fn find_airgroup_instances(&self, airgroup_id: usize) -> Vec<usize> {
        let air_instances = self.air_instances.read().unwrap();

        let mut indices = Vec::new();
        #[cfg(feature = "proofman/distributed")]
        let mut segment_ids = Vec::new();
        for (index, air_instance) in air_instances.iter().enumerate() {
            if air_instance.airgroup_id == airgroup_id {
                indices.push(index);
                #[cfg(feature = "proofman/distributed")]
                segment_ids.push(air_instance.air_segment_id.unwrap_or(0)); 
            }
        }
        #[cfg(feature = "proofman/distributed")]
        indices.sort_by(|a, b| segment_ids[*a].cmp(&segment_ids[*b]));
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
