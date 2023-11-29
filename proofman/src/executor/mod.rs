// Define the API trait

pub trait Executor {
    fn witness_computation(&self, stage_id: u32);
}
