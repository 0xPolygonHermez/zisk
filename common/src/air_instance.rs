use crate::AirInstanceCtx;

#[derive(Debug)]
pub struct AirInstance {
    pub air_group_id: usize,
    pub air_id: usize,
    pub inputs_interval: Option<(usize, usize)>,
    pub wc_component_idx: Option<usize>,
}

impl AirInstance {
    pub fn new(air_group_id: usize, air_id: usize, inputs_interval: Option<(usize, usize)>) -> Self {
        AirInstance { air_group_id, air_id, inputs_interval, wc_component_idx: None }
    }

    pub fn set_wc_component_idx(&mut self, idx: usize) {
        self.wc_component_idx = Some(idx);
    }

    pub fn get_wc_component_idx(&self) -> Option<usize> {
        self.wc_component_idx
    }
}

impl Into<AirInstanceCtx> for &AirInstance {
    fn into(self) -> AirInstanceCtx {
        AirInstanceCtx::new(self.air_group_id, self.air_id)
    }
}
