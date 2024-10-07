use rayon::Scope;

pub trait Provable<O, OR> {
    fn calculate(&self, _operation: O) -> Result<OR, Box<dyn std::error::Error>> {
        panic!("Provable::calculate() not implemented");
    }

    fn prove(&self, operations: &[O], drain: bool, scope: &Scope);

    fn calculate_prove(
        &self,
        _operation: O,
        _drain: bool,
        _scope: &Scope,
    ) -> Result<OR, Box<dyn std::error::Error>> {
        panic!("Provable::calculate_prove() not implemented");
    }
}
