#[doc(hidden)]
#[macro_export]
macro_rules! profile_start {
    ($name:ident) => {
        ziskos_syscall(
            zisk_definitions::SYSCALL_PROFILE_ID,
            zisk_definitions::PROFILE_START_COST_ID,
            stringify!($name),
        );
    };
}

#[macro_export]
macro_rules! profile_end {
    ($name:ident) => {
        ziskos_syscall(
            zisk_definitions::SYSCALL_PROFILE_ID,
            zisk_definitions::PROFILE_END_COST_ID,
            stringify!($name),
        );
    };
}

#[macro_export]
macro_rules! profile_steps_start {
    ($name:ident) => {
        ziskos_syscall(
            zisk_definitions::SYSCALL_PROFILE_ID,
            zisk_definitions::PROFILE_START_STEPS_ID,
            stringify!($name),
        );
    };
}

#[macro_export]
macro_rules! profile_steps_end {
    ($name:ident) => {
        ziskos_syscall(
            zisk_definitions::SYSCALL_PROFILE_ID,
            zisk_definitions::PROFILE_END_STEPS_ID,
            stringify!($name),
        );
    };
}
