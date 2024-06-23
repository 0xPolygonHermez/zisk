pub mod executors_manager;
pub mod executors_manager_thread;
use crate::proof_ctx::ProofCtx;

pub trait BufferManager<T> {
    fn get_buffer(&self, name: &str) -> Option<(Vec<u8>, usize)>;
}

pub trait Executor<T> {
    fn witness_computation(
        &self,
        stage_id: u32,
        proof_ctx: &mut ProofCtx<T>,
        buffer_manager: Option<&Box<dyn BufferManager<T>>>,
    );
}



pub struct ProofLayout {
    pub name: String,
    pub subprood_id: usize,
    pub instances: Vec<InstanceLayout>,
}

pub struct InstanceLayout {
    pub name: String,
    pub subproof_id: usize,
    pub air_id: usize,
    // We need a meta field to store how to "cut" each segment.
}

pub trait Executor2<T> {
    fn witness_computation(
        &self,
        stage_id: u32,
        proof_ctx: &mut ProofCtx<T>,
        buffer_manager: Option<&Box<dyn BufferManager<T>>>,
    );

    /// Pre_execute executes alll the witness computation in fast mode. I returns a list of ProofLayout.
    /// The number of returned ProofLayouts is equal to the number of subproofs in the pilout. Each ProofLayout
    /// contains the name of the subproof, the subproof_id and a list of InstanceLayouts.
    fn pre_execute(
        &self,
        proof_ctx: &mut ProofCtx<T>,
    ) -> Vec<ProofLayout>;

    fn execute_fast(
        &self,
        stage_id: u32,
        proof_ctx: &mut ProofCtx<T>,
    );
}

#[macro_export]
macro_rules! executor {
    ($executor_name:ident) => {
            executor!($executor_name {});
    };

    ($executor_name:ident { $( $field:ident : $field_type:ty ),* $(,)? }) => {
        pub struct $executor_name {
            $( $field : $field_type ),*
        }

        impl $executor_name {
            fn get_name(&self) -> String {
                stringify!($executor_name).to_string()
            }

            pub fn new($( $field : $field_type ),*) -> Self {
                $executor_name { $( $field ),* }
            }
        }
    };
}
