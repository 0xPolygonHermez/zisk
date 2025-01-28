/// Converts a signed 64-bit integer (`i64`) to an unsigned 64-bit integer (`u64`)
/// in a finite field representation.
///
/// This function handles the conversion of negative values to their field representation
/// using modular arithmetic. The field is assumed to have a prime characteristic
/// of `2^64 - 2^32`, and the conversion adjusts the value to fit within this finite field.
///
/// # Arguments
/// * `value` - The signed 64-bit integer to convert.
///
/// # Returns
/// The unsigned 64-bit integer in the finite field representation.
///
/// # Example
/// ```
/// use sm_common::i64_to_u64_field;
///
/// let pos_value: i64 = 1;
/// let neg_value: i64 = -1;
/// assert_eq!(i64_to_u64_field(pos_value), 1);
/// assert_eq!(i64_to_u64_field(neg_value), 0xFFFF_FFFF_0000_0000);
/// let pos_value: i64 = 2;
/// let neg_value: i64 = -2;
/// assert_eq!(i64_to_u64_field(pos_value), 2);
/// assert_eq!(i64_to_u64_field(neg_value), 0xFFFF_FFFE_FFFF_FFFF);
/// ```
pub fn i64_to_u64_field(value: i64) -> u64 {
    const PRIME_MINUS_ONE: u64 = 0xFFFF_FFFF_0000_0000; // 2^64 - 2^32 - 1
    if value >= 0 {
        value as u64
    } else {
        PRIME_MINUS_ONE - (0xFFFF_FFFF_FFFF_FFFF - value as u64)
    }
}
