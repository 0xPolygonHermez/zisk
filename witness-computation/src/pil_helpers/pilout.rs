use common::WCPilout;

pub struct ZiskPilout;

impl ZiskPilout {
    pub fn get_pilout() -> WCPilout {
        let mut pilout = WCPilout::new("Zisk", 2, b"zisk-hash".to_vec());

        let air_group = pilout.add_air_group(Some("Main"));
        air_group.add_air(Some("FibonacciSquare"), 10);

        let air_group = pilout.add_air_group(Some("Mem"));
        air_group.add_air(Some("Module"), 10);

        pilout
    }
}
