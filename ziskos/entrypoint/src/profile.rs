#![allow(dead_code)]
const PROFILE_PARAM1_FLAG: u16 = 0x100;
const PROFILE_PARAM2_FLAG: u16 = 0x200;
const PROFILE_PARAM3_FLAG: u16 = 0x300;
const PROFILE_VALUE_FLAG: u16 = 0x400;

#[doc(hidden)]
#[macro_export]
macro_rules! __ziskos_profile_export_name {
    ($id:expr, $name:ident) => {
        concat!("__ZISKOS_PROFILE_ID_", stringify!($id), "_", stringify!($name))
    };
}

#[macro_export]
macro_rules! ziskos_profile_start {
    ($name:ident = $id:expr) => {
        #[export_name = $crate::__ziskos_profile_export_name!($id, $name)]
        #[used]
        static $name: u16 = $id;

        ziskos_profile_start::<$name>()
    };
    ($name:ident) => {
        ziskos_profile_start::<$name>()
    };
}

#[macro_export]
macro_rules! ziskos_profile_end {
    ($name:ident) => {
        ziskos_profile_end::<$name>()
    };
}

#[macro_export]
macro_rules! ziskos_profile_absolute {
    ($name:ident = $id:expr) => {
        #[export_name = $crate::__ziskos_profile_export_name!($id, $name)]
        #[used]
        static $name: u16 = $id;

        ziskos_profile_absolute::<$name>()
    };
    ($name:ident) => {
        ziskos_profile_absolute::<$name>()
    };
}

#[macro_export]
macro_rules! ziskos_profile_relative {
    ($name:ident = $id:expr) => {
        #[export_name = $crate::__ziskos_profile_export_name!($id, $name)]
        #[used]
        static $name: u16 = $id;

        ziskos_profile_relative::<$name>()
    };
    ($name:ident) => {
        ziskos_profile_relative::<$name>()
    };
}

#[macro_export]
macro_rules! ziskos_profile_reset_relative {
    ($name:ident = $id:expr) => {
        #[export_name = $crate::__ziskos_profile_export_name!($id, $name)]
        #[used]
        static $name: u16 = $id;

        ziskos_profile_reset_relative::<$name>()
    };
    ($name:ident) => {
        ziskos_profile_reset_relative::<$name>()
    };
}

#[macro_export]
macro_rules! ziskos_profile_counter {
    ($name:ident = $id:expr) => {
        #[export_name = $crate::__ziskos_profile_export_name!($id, $name)]
        #[used]
        static $name: u16 = $id;

        ziskos_profile_counter::<$name>()
    };
    ($name:ident) => {
        ziskos_profile_counter::<$name>()
    };
}

#[macro_export]
macro_rules! ziskos_profile_value {
    ($name:ident = $id:expr, $value:expr) => {
        #[export_name = $crate::__ziskos_profile_export_name!($id, $name)]
        #[used]
        static $name: u16 = $id;

        ziskos_profile_value::<$name>($value)
    };
    ($name:ident) => {
        ziskos_profile_value::<$name>($value)
    };
}

#[macro_export]
macro_rules! ziskos_profile_arguments {
    ($name:ident = $id:expr, $arg1:expr) => {
        #[export_name = $crate::__ziskos_profile_export_name!($id, $name)]
        #[used]
        static $name: u16 = $id;

        ziskos_profile_argument::<$name>($arg1)
    };
    ($name:ident) => {
        ziskos_profile_argument::<$name>($arg1)
    };
    ($name:ident = $id:expr, $arg1:expr, $arg2: expr) => {
        #[export_name = $crate::__ziskos_profile_export_name!($id, $name)]
        #[used]
        static $name: u16 = $id;

        ziskos_profile_2_arguments::<$name>($arg1, $arg2)
    };
    ($name:ident) => {
        ziskos_profile_2_arguments::<$name>($arg1, $arg2)
    };
    ($name:ident = $id:expr, $arg1:expr, $arg2: expr, $arg3: expr) => {
        #[export_name = $crate::__ziskos_profile_export_name!($id, $name)]
        #[used]
        static $name: u16 = $id;

        ziskos_profile_3_arguments::<$name>($arg1, $arg2, $arg3)
    };
    ($name:ident) => {
        ziskos_profile_3_arguments::<$name>($arg1, $arg2, $arg3)
    };
}

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use core::arch::asm;

const MAX_TAG_ID: u16 = 256;

fn check_tag_id<const TAG_ID: u16>() {
    const { assert!(TAG_ID < MAX_TAG_ID, concat!("TAG_ID must be less than ", stringify!(MAX_TAG_ID))) };
}

/// Marks the start of a cost measurement region
///
/// # Arguments
/// * `TAG_ID` - Unique identifier for the cost region (must fit in 12-bit immediate)
#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
#[inline(always)]
pub fn ziskos_profile_start<const TAG_ID: u16>() {
    check_tag_id::<TAG_ID>();
    unsafe {
        asm!("addi x0, x1, {}", const TAG_ID, options(nomem, nostack));
    }
}

#[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
#[inline(always)]
pub fn ziskos_profile_start<const TAG_ID: u16>() {
    check_tag_id::<TAG_ID>();
}

/// Marks the end of a cost measurement region
///
/// # Arguments
/// * `TAG_ID` - Unique identifier for the cost region (must match the start tag)
#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
#[inline(always)]
pub fn ziskos_profile_end<const TAG_ID: u16>() {
    check_tag_id::<TAG_ID>();
    unsafe {
        asm!("addi x0, x2, {}", const TAG_ID, options(nomem, nostack));
    }
}

#[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
#[inline(always)]
pub fn ziskos_profile_end<const TAG_ID: u16>() {
    check_tag_id::<TAG_ID>();
}

/// Records an absolute cost measurement
#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
#[inline(always)]
pub fn ziskos_profile_absolute<const TAG_ID: u16>() {
    unsafe {
        asm!("addi x0, x3, {}", const TAG_ID, options(nomem, nostack));
    }
}

#[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
#[inline(always)]
pub fn ziskos_profile_absolute<const TAG_ID: u16>() {
    check_tag_id::<TAG_ID>();
}

/// Records a relative cost measurement
#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
#[inline(always)]
pub fn ziskos_profile_relative<const TAG_ID: u16>() {
    check_tag_id::<TAG_ID>();
    unsafe {
        asm!("addi x0, x4, {}", const TAG_ID, options(nomem, nostack));
    }
}

#[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
#[inline(always)]
pub fn ziskos_profile_relative<const TAG_ID: u16>() {
    check_tag_id::<TAG_ID>();
}

/// Reset relative cost measurement
#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
#[inline(always)]
pub fn ziskos_profile_reset_relative<const TAG_ID: u16>() {
    check_tag_id::<TAG_ID>();
    unsafe {
        asm!("addi x0, x5, {}", const TAG_ID, options(nomem, nostack));
    }
}

#[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
#[inline(always)]
pub fn ziskos_profile_reset_relative<const TAG_ID: u16>() {
    check_tag_id::<TAG_ID>();
}

/// Counter of executions
#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
#[inline(always)]
pub fn ziskos_profile_counter<const TAG_ID: u16>() {
    check_tag_id::<TAG_ID>();
    unsafe {
        asm!("addi x0, x6, {}", const TAG_ID, options(nomem, nostack));
    }
}

/// Counter of executions
#[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
#[inline(always)]
pub fn ziskos_profile_counter<const TAG_ID: u16>() {
    check_tag_id::<TAG_ID>();
}

/// Cost arguments
#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
#[inline(always)]
pub fn ziskos_profile_value<const TAG_ID: u16>(a: u64) {
    check_tag_id::<TAG_ID>();
    unsafe {
        asm!("addi x0, {}, {}",  in(reg) a, const TAG_ID + PROFILE_VALUE_FLAG, options(nomem, nostack));
    }
}

#[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
#[inline(always)]
pub fn ziskos_profile_value<const TAG_ID: u16>(_a: u64) {
    check_tag_id::<TAG_ID>();
}

/// Cost arguments
#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
#[inline(always)]
pub fn ziskos_profile_argument<const TAG_ID: u16>(a: u64) {
    check_tag_id::<TAG_ID>();
    unsafe {
        asm!("addi x0, {}, {}", in(reg) a, const TAG_ID + PROFILE_PARAM1_FLAG, options(nomem, nostack));
    }
}

#[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
#[inline(always)]
pub fn ziskos_profile_argument<const TAG_ID: u16>(_a: u64) {
    check_tag_id::<TAG_ID>();
}

/// Cost arguments
#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
#[inline(always)]
pub fn ziskos_profile_2_arguments<const TAG_ID: u16>(a: u64, b: u64) {
    check_tag_id::<TAG_ID>();
    unsafe {
        asm!(
            "addi x0, {a}, {flag1}",
            "addi x0, {b}, {flag2}",
            a = in(reg) a,
            b = in(reg) b,
            flag1 = const TAG_ID + PROFILE_PARAM1_FLAG,
            flag2 = const TAG_ID + PROFILE_PARAM2_FLAG,
            options(nomem, nostack)
        );
    }
}

#[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
#[inline(always)]
pub fn ziskos_profile_2_arguments<const TAG_ID: u16>(_a: u64, _b: u64) {
    check_tag_id::<TAG_ID>();
}

/// Cost arguments
#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
#[inline(always)]
pub fn ziskos_profile_3_arguments<const TAG_ID: u16>(a: u64, b: u64, c: u64) {
    check_tag_id::<TAG_ID>();
    unsafe {
        asm!(
            "addi x0, {a}, {flag1}",
            "addi x0, {b}, {flag2}",
            "addi x0, {c}, {flag3}",
            a = in(reg) a,
            b = in(reg) b,
            c = in(reg) c,
            flag1 = const TAG_ID + PROFILE_PARAM1_FLAG,
            flag2 = const TAG_ID + PROFILE_PARAM2_FLAG,
            flag3 = const TAG_ID + PROFILE_PARAM3_FLAG,
            options(nomem, nostack)
        );
    }
}

#[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
#[inline(always)]
pub fn ziskos_profile_3_arguments<const TAG_ID: u16>(_a: u64, _b: u64, _c: u64) {
    check_tag_id::<TAG_ID>();
}
