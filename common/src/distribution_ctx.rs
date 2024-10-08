#[cfg(feature = "distributed")]
use mpi::traits::Communicator;
#[cfg(feature = "distributed")]
use mpi::collective::CommunicatorCollectives;

/// Represents the context of distributed computing
#[derive(Clone, Default)]
pub struct DistributionCtx {
    pub rank: i32,
    pub n_processes: i32,
    #[cfg(feature = "distributed")]
    pub world: mpi::topology::SimpleCommunicator,

    #[cfg(not(feature = "distributed"))]
    pub world: i32,
}

impl DistributionCtx {
    pub fn new() -> Self {
        let mut ctx = DistributionCtx {
            rank: 0,
            n_processes: 1,
            #[cfg(feature = "distributed")]
            world: mpi::topology::SimpleCommunicator::null(),

            #[cfg(not(feature = "distributed"))]
            world: -1,
        };
        ctx.init();
        ctx
    }

    pub fn init(&mut self) {
        #[cfg(feature = "distributed")]
        {
            let (universe, _threading) = mpi::initialize_with_threading(mpi::Threading::Multiple).unwrap();
            self.world = universe.world();
            self.rank = self.world.rank();
            self.n_processes = self.world.size();
        }
        #[cfg(not(feature = "distributed"))]
        {
            self.rank = 0;
            self.n_processes = 1;
            self.world = -1;
        }
    }

    pub fn barrier(&self) {
        #[cfg(feature = "distributed")]
        {
            self.world.barrier();
        }
    }

    pub fn is_master(&self) -> bool {
        self.rank == 0
    }

    pub fn is_distributed(&self) -> bool {
        self.n_processes > 1
    }
}
