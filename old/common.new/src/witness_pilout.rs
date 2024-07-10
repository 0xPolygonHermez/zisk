pub struct WitnessPilOut {
    pub name: String,
    pub air_groups: Vec<AirGroup>,
}

impl WitnessPilOut {
    pub fn get_air(&self, airgroup_name: &str, air_name: &str) -> Option<&BasicAir> {
        for airgroup in &self.air_groups {
            if let Some(name) = &airgroup.name {
                if name == airgroup_name {
                    for air in &airgroup.airs {
                        if let Some(name) = &air.name {
                            if name == air_name {
                                return Some(air);
                            }
                        }
                    }
                }
            }
        }
        None
    }
}

pub struct AirGroup {
    pub name: Option<String>,
    pub airs: Vec<BasicAir>,
}

pub struct BasicAir {
    pub name: Option<String>,
    /// log2(n), where n is the number of rows
    pub num_rows: usize,
}
