pub trait WCOpCalculator {
    fn calculate_verify(&self, verify: bool, values: Vec<u64>) -> Result<Vec<u64>, Box<dyn std::error::Error>>;

    fn codes(&self) -> Vec<&str> {
        vec![]
    }
}
