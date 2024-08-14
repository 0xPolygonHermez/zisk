use rayon::Scope;

pub trait Provable<O, OR> {
    fn calculate(&self, operation: O) -> Result<OR, Box<dyn std::error::Error>>;

    fn prove(&self, operations: &[O], is_last: bool, scope: &Scope);

    fn calculate_prove(
        &self,
        operation: O,
        is_last: bool,
        scope: &Scope,
    ) -> Result<OR, Box<dyn std::error::Error>>;
}
