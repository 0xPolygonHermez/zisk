/// Converts a byte to 8 individual bits (LSB first)
pub fn byte_to_bits(byte: u8, bits: &mut [u8; 8]) {
    for (i, bit) in bits.iter_mut().enumerate() {
        *bit = (byte >> i) & 1;
    }
}

/// Converts 8 bits to a byte (LSB first)
pub fn bits_to_byte(bits: &[u8; 8], byte: &mut u8) {
    // bits.iter().rev().fold(0, |byte, &bit| (byte << 1) | (bit & 1))
    *byte = 0;
    for i in 0..8 {
        *byte = (*byte << 1) | (bits[7 - i] & 1);
    }
}

/// Prints bits in a formatted way
pub fn print_bits(bits: &[u8], name: &str) {
    let mut output = format!("{} = ", name);

    for k in 0..(bits.len() / 8) {
        let bits: &[u8; 8] = &bits[k * 8..(k + 1) * 8].try_into().unwrap();
        let mut byte = 0;
        bits_to_byte(bits, &mut byte);
        output += &format!("{}:", byte_to_string(byte));
    }

    println!("{}", output);
}

fn byte_to_string(b: u8) -> String {
    let mut result = String::with_capacity(2);
    result.push(byte_to_char(b >> 4));
    result.push(byte_to_char(b & 0x0F));
    result
}

fn byte_to_char(b: u8) -> char {
    match b {
        0..=9 => (b'0' + b) as char,
        10..=15 => (b'a' + b - 10) as char,
        _ => panic!("Invalid nibble value: {}", b),
    }
}
