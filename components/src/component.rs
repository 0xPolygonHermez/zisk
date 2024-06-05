pub trait Component<T> {
    type Output;

    fn get_default_id(&self) -> u16;
    fn calculate_free_input(&self, values: Vec<T>) -> Self::Output;
}