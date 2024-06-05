pub trait Component {
    fn get_default_id(&self) -> u16;
}

pub struct BasicProcesssorComponent<'a> {
    pub id: Option<usize>,
    pub component: Box<dyn Component + 'a>,
}