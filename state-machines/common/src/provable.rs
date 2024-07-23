pub trait Provable<O, OR> {
    fn calculate(&self, operation: O) -> Result<OR, Box<dyn std::error::Error>>;

    fn prove(&self, operations: &[O]);

    fn calculate_prove(&self, operation: O) -> Result<OR, Box<dyn std::error::Error>>;
}
