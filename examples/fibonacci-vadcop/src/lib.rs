mod pil;
use pil::{Fibonacci, Module};

use wcmanager::{WitnessModule, HasSubcomponents, WitnessManagerAPI};

const FIBONACCI_VADCOP_HASH: &[u8] = b"fibonacci-vadcop-hash";

pub struct FibonacciVadcop<'a, F> {
    _marker: core::marker::PhantomData<&'a F>,
}

impl<'a, F> FibonacciVadcop<'a, F> {
    pub fn new() -> Self {
        Self { _marker: core::marker::PhantomData }
    }
}

impl<'a, F> WitnessManagerAPI<'a, F> for FibonacciVadcop<'a, F> {
    fn build_wcmanager(&self) -> Box<dyn WitnessModule<'a, F> + 'a> {
        let mut fibonacci = Box::new(Fibonacci::<'a, F>::new());
        let module = Box::new(Module::<'a, F>::new());

        fibonacci.add_subcomponent(module);

        fibonacci
    }

    fn get_pilout_hash(&self) -> &[u8] {
        FIBONACCI_VADCOP_HASH
    }
}

#[no_mangle]
pub extern "Rust" fn create_plugin<'a>() -> Box<dyn WitnessManagerAPI<'a, goldilocks::Goldilocks> + 'a> {
    env_logger::builder().format_timestamp(None).format_target(false).filter_level(log::LevelFilter::Trace).init();
    Box::new(FibonacciVadcop::<'a, goldilocks::Goldilocks>::new())
}
