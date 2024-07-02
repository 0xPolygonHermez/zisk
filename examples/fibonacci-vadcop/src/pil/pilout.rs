use common::{AirGroup, BasicAir};
use common::WitnessPilOut;

pub fn get_fibonacci_vadcop_pilout() -> WitnessPilOut {
    let fibo_10 = BasicAir { name: Some("Fibo_10".to_string()), num_rows: Some(10) };

    let module_10 = BasicAir { name: Some("Module_10".to_string()), num_rows: Some(10) };
    let module_12 = BasicAir { name: Some("Module_12".to_string()), num_rows: Some(12) };

    WitnessPilOut {
        name: "FibonacciVadcopPilOut".to_string(),
        air_groups: vec![AirGroup { name: Some("AirGroup_1".to_string()), airs: vec![fibo_10, module_10, module_12] }],
    }
}
