pub struct Proof<T> {
    _inner: Vec<T>,
}

impl<T> Proof<T> {
    pub fn new() -> Self {
        Self { _inner: vec![] }
    }
}
