use rayon::Scope;

pub trait Provable<O, OR, F> {
    fn calculate(&self, operation: O) -> Result<OR, Box<dyn std::error::Error>>;

    fn prove(&self, operations: &[O], drain: bool, scope: &Scope);

    fn calculate_prove(
        &self,
        operation: O,
        drain: bool,
        scope: &Scope,
    ) -> Result<OR, Box<dyn std::error::Error>>;
}
