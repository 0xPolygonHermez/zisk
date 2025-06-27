pub struct MpiContext {
    #[cfg(distributed)]
    pub universe: mpi::environment::Universe,
    pub world_rank: i32,
    pub local_rank: i32,
}
