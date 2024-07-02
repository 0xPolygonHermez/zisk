pub struct WitnessPilOut {
    pub name: String,
    pub air_groups: Vec<AirGroup>,
}

pub struct AirGroup {
    pub name: Option<String>,
    pub airs: Vec<BasicAir>,
}

pub struct BasicAir {
    pub name: Option<String>,
    /// log2(n), where n is the number of rows
    pub num_rows: Option<u32>,
}
