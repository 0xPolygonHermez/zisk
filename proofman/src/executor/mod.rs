pub mod executors_manager;
pub mod executors_manager_thread;
use crate::proof_ctx::ProofCtx;

pub trait Executor<T> {
    fn witness_computation(&self, stage_id: u32, proof_ctx: &mut ProofCtx<T>);
}

#[macro_export]
macro_rules! executor {
    ($executor_name:ident) => {
            executor!($executor_name {});
    };

    ($executor_name:ident { $( $field:ident : $field_type:ty ),* $(,)? }) => {
        pub struct $executor_name {
            ptr: std::cell::UnsafeCell<*mut u8>,
            $( $field : $field_type ),*
        }

        impl $executor_name {
            fn get_name(&self) -> String {
                stringify!($executor_name).to_string()
            }

            pub fn new($( $field : $field_type ),*) -> Self {
                $executor_name { ptr: std::cell::UnsafeCell::new(std::ptr::null_mut()), $( $field ),* }
            }

            pub fn from_ptr(ptr: *mut u8, $( $field : $field_type ),*) -> Self {
                $executor_name { ptr: std::cell::UnsafeCell::new(ptr), $( $field ),* }
            }
        }
    };
}
