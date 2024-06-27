
pub struct Command<T> {
    pub name: String,
    pub callback: Box<dyn Fn(&mut T)>,
}