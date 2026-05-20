#[cfg(feature = "profile")]
macro_rules! profile_block {
    ($tag:ident, $body:block) => {{
        ziskos::profile_report_start!($tag);
        ziskos::profile_report_steps_start!($tag);
        let __profile_result = $body;
        ziskos::profile_report_steps_end!($tag);
        ziskos::profile_report_end!($tag);
        __profile_result
    }};
}

#[cfg(not(feature = "profile"))]
macro_rules! profile_block {
    ($tag:ident, $body:block) => {
        $body
    };
}

pub(crate) use profile_block;

pub(crate) const ZERO: [u64; 4] = [0, 0, 0, 0];

pub(crate) const ONE: [u64; 4] = [1, 0, 0, 0];

pub(crate) const TWO: [u64; 4] = [2, 0, 0, 0];

pub(crate) const MAX: [u64; 4] = [u64::MAX; 4];
pub(crate) const MAX_MINUS_ONE: [u64; 4] = [u64::MAX - 1, u64::MAX, u64::MAX, u64::MAX];

pub(crate) const POW2_64: [u64; 4] = [0, 1, 0, 0]; // 2^64
pub(crate) const POW2_128: [u64; 4] = [0, 0, 1, 0]; // 2^128

#[cfg(any(not(all(target_os = "zkvm", target_vendor = "zisk")), feature = "ruint-fallback"))]
pub(crate) type RU256 = ruint::Uint<256, 4>;
