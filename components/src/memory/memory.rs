use crate::Component;

pub struct Memory<'a, T> {
    phantom: std::marker::PhantomData<&'a T>,
}

impl<'a, T> Memory<'a, T> {
    const DEFAULT_ID: u16 = 4;

    pub fn build() -> Self {
        Self { phantom: std::marker::PhantomData }
    }
}

impl<'a, T> Component for Memory<'a, T> {
    fn get_default_id(&self) -> u16 {
        Self::DEFAULT_ID
    }
}
