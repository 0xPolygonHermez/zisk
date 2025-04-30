use circuit::byte_to_bits_msb;

use super::{SHA256_BLOCK_SIZE_BITS, SHA256_BLOCK_SIZE_BYTES};

#[derive(Debug)]
pub struct Sha256Input<'a> {
    input: &'a [u8],
    offset: usize,
    padding: [u8; SHA256_BLOCK_SIZE_BYTES + 8],
    padding_size: usize,
    padding_offset: usize,
    state: State,
}

#[derive(Debug, PartialEq)]
enum State {
    ReadingInput,
    ReadingPadding,
    Done,
}

impl<'a> Sha256Input<'a> {
    /// Initializes a new Sha256Input with the given input
    pub fn new(input: &'a [u8]) -> Self {
        // Sha256 makes use of the padding:
        //  · Say original message M is of length L bits.
        //  · The padded message is:
        //          M || 1 || <K 0's> || <L as 64 bit integer>,
        //    where K is such that L + 1 + K = 448 (mod 512).
        //  · The length of the padded message is a multiple of 512 bits.

        // This is the same as performing:
        //          M || <0x80> || (K/8)*<0x00> || <L as 64 bit integer>

        // Create padding buffer
        let input_size = input.len();
        let remaining_bytes = input_size % SHA256_BLOCK_SIZE_BYTES;
        let mut padding = [0u8; SHA256_BLOCK_SIZE_BYTES + 8];
        let mut padding_size = SHA256_BLOCK_SIZE_BYTES - remaining_bytes;
        if remaining_bytes >= 56 {
            // If remaining bytes are equal or bigger than 56 bytes, we need to pad to the next block
            padding_size += SHA256_BLOCK_SIZE_BYTES;
        }

        // Set the first byte to 0x80
        padding[0] = 0x80;

        // Store message length in (big-endian) bits in the last 8 bytes
        let input_len: [u8; 8] = (input_size as u64 * 8).to_be_bytes();
        padding[padding_size - 8..padding_size].copy_from_slice(&input_len);

        Self {
            input,
            offset: 0,
            padding,
            padding_size,
            padding_offset: 0,
            state: State::ReadingInput,
        }
    }

    /// Reads next SHA256_BLOCK_SIZE_BYTES bytes of input
    pub fn get_next(&mut self, buffer: &mut [u8; SHA256_BLOCK_SIZE_BYTES]) -> bool {
        match self.state {
            State::ReadingInput => {
                if self.offset + SHA256_BLOCK_SIZE_BYTES <= self.input.len() {
                    // Full block of input
                    buffer.copy_from_slice(
                        &self.input[self.offset..self.offset + SHA256_BLOCK_SIZE_BYTES],
                    );
                    self.offset += SHA256_BLOCK_SIZE_BYTES;
                    true
                } else {
                    // Last partial block from input + padding
                    let remaining = self.input.len() - self.offset;
                    if remaining > 0 {
                        buffer[..remaining].copy_from_slice(&self.input[self.offset..]);
                    }

                    let pad_to_copy = SHA256_BLOCK_SIZE_BYTES - remaining;
                    buffer[remaining..].copy_from_slice(&self.padding[..pad_to_copy]);
                    self.padding_offset += pad_to_copy;
                    self.state = State::ReadingPadding;
                    true
                }
            }
            State::ReadingPadding => {
                // Full block of padding
                if self.padding_offset < self.padding_size {
                    buffer.copy_from_slice(
                        &self.padding
                            [self.padding_offset..self.padding_offset + SHA256_BLOCK_SIZE_BYTES],
                    );
                    self.state = State::Done;
                    true
                } else {
                    self.state = State::Done;
                    false
                }
            }
            State::Done => false,
        }
    }

    /// Reads next SHA256_BLOCK_SIZE_BITS bits of input
    pub fn get_next_bits(&mut self, bit_buffer: &mut [u8; SHA256_BLOCK_SIZE_BITS]) -> bool {
        let mut byte_buffer = [0u8; SHA256_BLOCK_SIZE_BYTES];
        if !self.get_next(&mut byte_buffer) {
            return false;
        }

        for (i, &byte) in byte_buffer.iter().enumerate() {
            let bit_slice = &mut bit_buffer[i * 8..(i + 1) * 8];
            byte_to_bits_msb(byte, bit_slice.try_into().unwrap());
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
        let mut input = Sha256Input::new(&data);
        let mut buffer = [0u8; SHA256_BLOCK_SIZE_BYTES];
        assert!(input.padding_size == SHA256_BLOCK_SIZE_BYTES);

        assert!(input.get_next(&mut buffer));
        assert_eq!(buffer[0], 0x80);
        for i in 1..SHA256_BLOCK_SIZE_BYTES {
            assert_eq!(buffer[i], 0x00);
        }
        assert!(!input.get_next(&mut buffer));
    }

    #[test]
    fn test_small_input() {
        // Test with input smaller than block size
        let data = vec![0xFFu8; 55];
        let mut input = Sha256Input::new(&data);
        let mut buffer = [0u8; SHA256_BLOCK_SIZE_BYTES];
        assert!(input.padding_size == 9);

        assert!(input.get_next(&mut buffer));
        for i in 0..55 {
            assert_eq!(buffer[i], 0xFF); // Original data
        }
        assert_eq!(buffer[55], 0x80); // From padding

        // 55*8 in binary is 0b00000001 0b10111000 == 0x01 || 0xB8
        for i in 56..SHA256_BLOCK_SIZE_BYTES - 2 {
            assert_eq!(buffer[i], 0x00); // Padding
        }
        assert_eq!(buffer[SHA256_BLOCK_SIZE_BYTES - 2], 0x01);
        assert_eq!(buffer[SHA256_BLOCK_SIZE_BYTES - 1], 0xB8);

        // Second read should fail
        assert!(!input.get_next(&mut buffer));
    }

    #[test]
    fn test_small_input2() {
        // Test with input smaller than block size but bigger than 55
        let data = vec![0xFFu8; 56];
        let mut input = Sha256Input::new(&data);
        let mut buffer = [0u8; SHA256_BLOCK_SIZE_BYTES];
        assert!(input.padding_size == 72);

        assert!(input.get_next(&mut buffer));
        for i in 0..56 {
            assert_eq!(buffer[i], 0xFF); // Original data
        }
        assert_eq!(buffer[56], 0x80); // From padding
        for i in 57..SHA256_BLOCK_SIZE_BYTES {
            assert_eq!(buffer[i], 0x00); // Original data
        }

        assert!(input.get_next(&mut buffer));
        // 56*8 in binary is 0b00000001 0b11000000 == 0x01 || 0xC0
        for i in 0..SHA256_BLOCK_SIZE_BYTES - 2 {
            assert_eq!(buffer[i], 0x00); // Padding
        }
        assert_eq!(buffer[SHA256_BLOCK_SIZE_BYTES - 2], 0x01);
        assert_eq!(buffer[SHA256_BLOCK_SIZE_BYTES - 1], 0xC0);

        // Second read should fail
        assert!(!input.get_next(&mut buffer));
    }

    #[test]
    fn test_full_input() {
        // Test with input equal to block size
        let data = vec![0xFFu8; SHA256_BLOCK_SIZE_BYTES];
        let mut input = Sha256Input::new(&data);
        let mut buffer = [0u8; SHA256_BLOCK_SIZE_BYTES];
        assert!(input.padding_size == SHA256_BLOCK_SIZE_BYTES);

        assert!(input.get_next(&mut buffer));
        for i in 0..SHA256_BLOCK_SIZE_BYTES {
            assert_eq!(buffer[i], 0xFF); // Original data
        }

        // Second read should be padded
        assert!(input.get_next(&mut buffer));
        assert_eq!(buffer[0], 0x80); // From padding

        // 64*8 in binary is 0b00000010 0b00000000 == 0x02 || 0x00
        for i in 1..SHA256_BLOCK_SIZE_BYTES - 2 {
            assert_eq!(buffer[i], 0x00); // Padding
        }
        assert_eq!(buffer[SHA256_BLOCK_SIZE_BYTES - 2], 0x02);
        assert_eq!(buffer[SHA256_BLOCK_SIZE_BYTES - 1], 0x00);

        // Third read should fail
        assert!(!input.get_next(&mut buffer));
    }

    #[test]
    fn test_big_input() {
        // Test with input larger than (2) block size
        let data = vec![0xFFu8; SHA256_BLOCK_SIZE_BYTES + 57];
        let mut input = Sha256Input::new(&data);
        let mut buffer = [0u8; SHA256_BLOCK_SIZE_BYTES];
        assert!(input.padding_size == 71);

        assert!(input.get_next(&mut buffer));
        for i in 0..SHA256_BLOCK_SIZE_BYTES {
            assert_eq!(buffer[i], 0xFF); // Original data
        }

        // Second read should be padded
        assert!(input.get_next(&mut buffer));
        for i in 0..57 {
            assert_eq!(buffer[i], 0xFF); // Original data
        }
        assert_eq!(buffer[57], 0x80); // From padding

        // 121*8 in binary is 0b00000011 0b11001000 == 0x03 || 0xC8
        for i in 58..SHA256_BLOCK_SIZE_BYTES {
            assert_eq!(buffer[i], 0x00); // Padding
        }

        // Third read should be padded
        assert!(input.get_next(&mut buffer));
        for i in 0..SHA256_BLOCK_SIZE_BYTES - 2 {
            assert_eq!(buffer[i], 0x00);
        }
        assert_eq!(buffer[SHA256_BLOCK_SIZE_BYTES - 2], 0x03);
        assert_eq!(buffer[SHA256_BLOCK_SIZE_BYTES - 1], 0xC8);

        // Fourth read should fail
        assert!(!input.get_next(&mut buffer));
    }
}
