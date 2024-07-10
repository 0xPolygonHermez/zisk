const CHUNKS: usize = 8;

#[allow(dead_code)]
pub enum ComponentOutput<T> {
    Single(T),
    Array([T; CHUNKS]),
}

#[allow(dead_code)]
pub trait Component<F> {
    fn init(&mut self);
    fn finish(&mut self);
    fn get_default_id(&self) -> u16;

    fn calculate_free_input(&self, values: Vec<F>) -> ComponentOutput<F>;

    fn verify(&self, values: Vec<F>) -> bool;
}
