// Re-export profile constants from zisk_definitions to make them available
// when macros are expanded in guest code
#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
pub use zisk_definitions::{
    PROFILE_END_COST_ID, PROFILE_END_STEPS_ID, PROFILE_REPORT_END_COST_ID,
    PROFILE_REPORT_END_STEPS_ID, PROFILE_REPORT_START_COST_ID, PROFILE_REPORT_START_STEPS_ID,
    PROFILE_START_COST_ID, PROFILE_START_STEPS_ID, SYSCALL_PROFILE_ID,
};

#[macro_export]
#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
macro_rules! profile_call {
    ($name:ident, $profile_call_id:expr) => {{
        let name_str = stringify!($name);
        $crate::ziskos_syscall!(
            $crate::SYSCALL_PROFILE_ID,
            $profile_call_id,
            &name_str as *const _ as usize
        );
    }};
}
#[macro_export]
#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
macro_rules! profile_start {
    ($name:ident) => {
        $crate::profile_call!($name, $crate::PROFILE_START_COST_ID);
    };
}

#[macro_export]
#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
macro_rules! profile_end {
    ($name:ident) => {
        $crate::profile_call!($name, $crate::PROFILE_END_COST_ID);
    };
}

#[macro_export]
#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
macro_rules! profile_report_start {
    ($name:ident) => {
        $crate::profile_call!($name, $crate::PROFILE_REPORT_START_COST_ID);
    };
}

#[macro_export]
#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
macro_rules! profile_report_end {
    ($name:ident) => {
        $crate::profile_call!($name, $crate::PROFILE_REPORT_END_COST_ID);
    };
}

#[macro_export]
#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
macro_rules! profile_steps_start {
    ($name:ident) => {
        $crate::profile_call!($name, $crate::PROFILE_START_STEPS_ID);
    };
}

#[macro_export]
#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
macro_rules! profile_steps_end {
    ($name:ident) => {
        $crate::profile_call!($name, $crate::PROFILE_END_STEPS_ID);
    };
}

#[macro_export]
#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
macro_rules! profile_report_steps_start {
    ($name:ident) => {
        $crate::profile_call!($name, $crate::PROFILE_REPORT_START_STEPS_ID);
    };
}

#[macro_export]
#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
macro_rules! profile_report_steps_end {
    ($name:ident) => {
        $crate::profile_call!($name, $crate::PROFILE_REPORT_END_STEPS_ID);
    };
}

#[macro_export]
#[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
macro_rules! profile_start {
    ($name:ident) => {};
}

#[macro_export]
#[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
macro_rules! profile_end {
    ($name:ident) => {};
}

#[macro_export]
#[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
macro_rules! profile_steps_start {
    ($name:ident) => {};
}

#[macro_export]
#[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
macro_rules! profile_steps_end {
    ($name:ident) => {};
}

#[macro_export]
#[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
macro_rules! profile_report_start {
    ($name:ident) => {};
}

#[macro_export]
#[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
macro_rules! profile_report_end {
    ($name:ident) => {};
}

#[macro_export]
#[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
macro_rules! profile_report_steps_start {
    ($name:ident) => {};
}

#[macro_export]
#[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
macro_rules! profile_report_steps_end {
    ($name:ident) => {};
}
