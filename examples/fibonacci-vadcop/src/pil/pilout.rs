use common::{AirGroup, BasicAir};
use common::WitnessPilOut;

pub fn get_fibonacci_vadcop_pilout() -> WitnessPilOut {
    let fibo = BasicAir { name: Some("FibonacciSquare".to_string()), num_rows: 8 };

    let module = BasicAir { name: Some("Module".to_string()), num_rows: 8 };

    WitnessPilOut {
        name: "FibonacciVadcopPilOut".to_string(),
        air_groups: vec![AirGroup { name: Some("AirGroup_1".to_string()), airs: vec![fibo, module] }],
    }
}
