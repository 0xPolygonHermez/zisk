use common::WitnessPilOut;

pub fn get_fibonacci_vadcop_pilout() -> WitnessPilOut {
    let mut pilout = WitnessPilOut::new("FibonacciVadcopPilOut", 2, b"fibonacci-vadcop-hash".to_vec());

    let air_group = pilout.add_air_group(Some("FibonacciSquare"));
    println!("air_group: {:?}", air_group);
    air_group.add_air(Some("FibonacciSquare"), 10);

    let air_group = pilout.add_air_group(Some("Module"));
    air_group.add_air(Some("Module"), 10);

    let air_group = pilout.add_air_group(Some("U8Air"));
    air_group.add_air(Some("U8Air"), 8);

    pilout
}
