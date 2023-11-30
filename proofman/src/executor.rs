pub trait Executor {
    fn witness_computation(&self, stage_id: u32, subproof_id: u32, instance_id: i32/*, publics*/);
}
