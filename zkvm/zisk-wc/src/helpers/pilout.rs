use common::WCPilOut;

pub struct FibonacciVadcopPilout;

impl FibonacciVadcopPilout {
    pub fn get_fibonacci_vadcop_pilout() -> WCPilOut {
        let mut pilout = WCPilOut::new("Zisk", 2, b"zisk-hash".to_vec());

        let air_group = pilout.add_air_group(Some("Main"));
        air_group.add_air(Some("FibonacciSquare"), 10);

        let air_group = pilout.add_air_group(Some("Mem"));
        air_group.add_air(Some("Module"), 10);

        pilout
    }
}
