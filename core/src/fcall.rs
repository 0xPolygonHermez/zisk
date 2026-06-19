// In the worst case, we divide a 16.384-bit number (8.192 * 2)
// by an 8.192-bit number. We must also include the length fields.
// This results in a total of 1 + 256 + 1 + 128 = 386 u64 parameters.
pub const FCALL_PARAMS_MAX_SIZE: usize = 386;
// In the worst case, we compute the binary decomposition of a 8192-bit number
// This results in 1 + 8192 = 8193 u64 results.
pub const FCALL_RESULT_MAX_SIZE: usize = 8193;
