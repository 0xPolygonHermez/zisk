mod bigint256;
mod bls318;
mod bn254;
mod modexp;
mod secp256k1;

pub use bigint256::*;
pub use bls318::*;
pub use bn254::*;
pub use modexp::*;
pub use secp256k1::*;

/// Macro to generate size, offset, and expected length constants for hint data fields.
///
/// # Example
/// ```ignore
/// hint_fields![A: 4, B: 4, M: 4]
/// ```
/// Generates:
/// - `A_SIZE`, `B_SIZE`, `M_SIZE` constants
/// - `A_OFFSET`, `B_OFFSET`, `M_OFFSET` constants (cumulative offsets)
/// - `EXPECTED_LEN` constant (sum of all sizes)
#[macro_export]
macro_rules! hint_fields {
    ($($name:ident: $size:expr),+ $(,)?) => {
        paste::paste! {
            $(
                #[allow(dead_code)]
                const [<$name _SIZE>]: usize = $size;
            )+
        }

        hint_fields!(@offsets 0, $($name: $size),+);

        const EXPECTED_LEN: usize = hint_fields!(@sum $($size),+);
    };

    (@offsets $offset:expr, $name:ident: $size:expr) => {
        paste::paste! {
            const [<$name _OFFSET>]: usize = $offset;
        }
    };

    (@offsets $offset:expr, $name:ident: $size:expr, $($rest_name:ident: $rest_size:expr),+) => {
        paste::paste! {
            const [<$name _OFFSET>]: usize = $offset;
        }
        hint_fields!(@offsets $offset + $size, $($rest_name: $rest_size),+);
    };

    (@sum $size:expr) => { $size };
    (@sum $size:expr, $($rest:expr),+) => {
        $size + hint_fields!(@sum $($rest),+)
    };
}

/// Validates that the hint data has the expected length.
///
/// # Arguments
///
/// * `data` - The hint data to validate
/// * `expected_len` - The expected number of u64 values
/// * `hint_name` - The name of the hint type for error messages
///
/// # Returns
///
/// * `Ok(())` - If the length is correct
/// * `Err(String)` - If the length is incorrect
#[inline]
fn validate_hint_length(data: &[u64], expected_len: usize, hint_name: &str) -> Result<(), String> {
    if data.len() != expected_len {
        return Err(format!(
            "Invalid {} hint length: expected {}, got {}",
            hint_name,
            expected_len,
            data.len()
        ));
    }
    Ok(())
}
