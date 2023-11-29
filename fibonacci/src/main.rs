use proofman::executor::Executor;

mod executor1;
mod executor2;

fn main() {
    let modules: Vec<&'static dyn Executor> = vec![
        &executor1::Executor1,
        &executor2::Executor2,
    ];

    for module in modules {
        module.witness_computation(42);
    }   
}