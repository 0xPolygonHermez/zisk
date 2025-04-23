/// Calculates the linear position of a bit in the Keccak state as per the specification.
///
/// The Keccak state is organized as a 3-dimensional array:
/// - x: 0..4 (5 lanes)
/// - y: 0..4 (5 lanes)
/// - z: 0..63 (64 bits per lane)
///
/// # Arguments
/// * `x` - Lane coordinate (0-4)
/// * `y` - Lane coordinate (0-4)
/// * `z` - Bit position within lane (0-63)
///
/// # Returns
/// Linear position in 0..1599 range
pub fn bit_position(x: usize, y: usize, z: usize) -> usize {
    assert!(x < 5, "x coordinate must be 0-4");
    assert!(y < 5, "y coordinate must be 0-4");
    assert!(z < 64, "z coordinate must be 0-63");

    64 * x + 320 * y + z
}
