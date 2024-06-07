use goldilocks::AbstractField;

use crate::{basic_processor::CallbackReturnType, Component};

pub struct Memory<'a, T> {
    phantom: std::marker::PhantomData<&'a T>,
}

impl<'a, T> Memory<'a, T> {
    const DEFAULT_ID: u16 = 4;

    pub fn build() -> Self {
        Self { phantom: std::marker::PhantomData }
    }
}

impl<'a, T> Component<T> for Memory<'a, T>
where
    T: AbstractField + Copy,
{
    type Output = Option<CallbackReturnType<T>>;

    fn get_default_id(&self) -> u16 {
        Self::DEFAULT_ID
    }

    fn calculate_free_input(&self, values: Vec<T>) -> Self::Output {
        Some(CallbackReturnType::Array([T::default(); 8]))
    }

    fn verify(&self, values: Vec<T>) -> bool {
        unimplemented!()
    }
}
