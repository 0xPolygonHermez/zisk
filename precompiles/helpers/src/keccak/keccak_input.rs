use circuit::byte_to_bits;

use super::{BITRATE, BYTERATE};

#[derive(Debug)]
pub struct KeccakInput<'a> {
    input: &'a [u8],
    offset: usize,
    padding: [u8; BYTERATE],
    padding_size: usize,
}

impl<'a> KeccakInput<'a> {
    /// Initializes a new KeccakInput with the given data
    pub fn new(input: &'a [u8]) -> Self {
        let mut padding = [0u8; BYTERATE];
        let padding_size = BYTERATE - (input.len() % BYTERATE);

        // Keccak makes use of multi-rate padding:
        // First bit = 1, last bit = 1, others = 0
        padding[0] = 0b00000001;
        padding[padding_size - 1] |= 0b10000000;

        Self { input, offset: 0, padding, padding_size }
    }

    /// Reads next BYTERATE bytes of input
    pub fn get_next(&mut self, buffer: &mut [u8; BYTERATE]) -> bool {
        if self.offset + BYTERATE <= self.input.len() {
            // Full block available
            buffer.copy_from_slice(&self.input[self.offset..self.offset + BYTERATE]);
            self.offset += BYTERATE;
            true
        } else if self.offset <= self.input.len() {
            // Partial block needs padding
            let remaining = self.input.len() - self.offset;

            if remaining > 0 {
                buffer[..remaining].copy_from_slice(&self.input[self.offset..]);
            }

            if remaining < BYTERATE {
                buffer[remaining..].copy_from_slice(&self.padding[..self.padding_size]);
            }

            self.offset += BYTERATE;
            true
        } else {
            // No more data available
            false
        }
    }

    /// Reads next BITRATE bits of input
    pub fn get_next_bits(&mut self, bit_buffer: &mut [u8; BITRATE]) -> bool {
        let mut byte_buffer = [0u8; BYTERATE];
        if !self.get_next(&mut byte_buffer) {
            return false;
        }

        for (i, &byte) in byte_buffer.iter().enumerate() {
            let bit_slice = &mut bit_buffer[i * 8..(i + 1) * 8];
            byte_to_bits(byte, bit_slice.try_into().unwrap());
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_input() {
        let data = vec![];
        let mut input = KeccakInput::new(&data);
        let mut buffer = [0u8; BYTERATE];
        assert!(input.padding_size == BYTERATE);

        assert!(input.get_next(&mut buffer));
        assert_eq!(buffer[0], 0x01);
        for &byte in buffer.iter().take(BYTERATE - 1).skip(1) {
            assert_eq!(byte, 0x00);
        }
        assert_eq!(buffer[BYTERATE - 1], 0x80);
        assert!(!input.get_next(&mut buffer));
    }

    #[test]
    fn test_small_input() {
        // Test with input smaller than block size
        let data = vec![0xFFu8; 100];
        let mut input = KeccakInput::new(&data);
        let mut buffer = [0u8; 136];
        assert!(input.padding_size == 36);

        assert!(input.get_next(&mut buffer));
        for &byte in buffer.iter().take(100) {
            assert_eq!(byte, 0xFF); // Original data
        }
        assert_eq!(buffer[100], 0x01); // From padding
        for &byte in buffer.iter().take(135).skip(101) {
            assert_eq!(byte, 0x00); // Padding
        }
        assert_eq!(buffer[135], 0x80); // Last padding byte

        // Second read should fail
        assert!(!input.get_next(&mut buffer));
    }

    #[test]
    fn test_full_input() {
        // Test with input equal to block size
        let data = vec![0xFFu8; BYTERATE];
        let mut input = KeccakInput::new(&data);
        let mut buffer = [0u8; BYTERATE];
        assert!(input.padding_size == 136);

        assert!(input.get_next(&mut buffer));
        for &byte in buffer.iter().take(BYTERATE) {
            assert_eq!(byte, 0xFF); // Original data
        }

        // Second read should be padded
        assert!(input.get_next(&mut buffer));
        assert_eq!(buffer[0], 0x01); // From padding
        for &byte in buffer.iter().take(BYTERATE - 1).skip(1) {
            assert_eq!(byte, 0x00); // Padding
        }
        assert_eq!(buffer[BYTERATE - 1], 0x80); // Last padding byte

        // Third read should fail
        assert!(!input.get_next(&mut buffer));
    }

    #[test]
    fn test_big_input() {
        // Test with input larger than block size
        let data = vec![0xFFu8; 200];
        let mut input = KeccakInput::new(&data);
        let mut buffer = [0u8; BYTERATE];
        assert!(input.padding_size == 72);

        assert!(input.get_next(&mut buffer));
        for &byte in buffer.iter().take(BYTERATE) {
            assert_eq!(byte, 0xFF); // Original data
        }

        // Second read should be padded
        assert!(input.get_next(&mut buffer));
        for &byte in buffer.iter().take(64) {
            assert_eq!(byte, 0xFF); // Original data
        }
        assert_eq!(buffer[64], 0x01); // From padding
        for &byte in buffer.iter().take(BYTERATE - 1).skip(65) {
            assert_eq!(byte, 0x00); // Padding
        }
        assert_eq!(buffer[BYTERATE - 1], 0x80); // Last padding byte

        // Third read should fail
        assert!(!input.get_next(&mut buffer));
    }
}
