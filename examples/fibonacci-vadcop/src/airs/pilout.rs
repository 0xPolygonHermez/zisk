use common::WCPilOut;

pub struct FibonacciVadcopPilout;

impl FibonacciVadcopPilout {
    pub fn get_fibonacci_vadcop_pilout() -> WCPilOut {
        //TODO: This should not be harcoded
        let mut pilout = WCPilOut::new("FibonacciVadcopPilOut", 1, b"fibonacci-vadcop-hash".to_vec());

        let air_group = pilout.add_air_group(Some("FibonacciSquare"));
        air_group.add_air(Some("FibonacciSquare"), 10);

        let air_group = pilout.add_air_group(Some("Module"));
        air_group.add_air(Some("Module"), 10);

        //let air_group = pilout.add_air_group(Some("U8Air"));
        //air_group.add_air(Some("U8Air"), 8);

        pilout
    }
}
