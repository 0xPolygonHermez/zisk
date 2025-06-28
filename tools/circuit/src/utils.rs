/// Converts a byte to 8 individual bits (LSB first)
pub fn byte_to_bits(byte: u8, bits: &mut [u8; 8]) {
    for (i, bit) in bits.iter_mut().enumerate() {
        *bit = (byte >> i) & 1;
    }
}

pub fn byte_to_bits_msb(byte: u8) -> [u8; 8] {
    let mut bits = [0u8; 8];
    for (i, bit) in bits.iter_mut().enumerate() {
        *bit = (byte >> (7 - i)) & 1;
    }
    bits
}

/// Converts 8 bits to a byte (LSB first)
pub fn bits_to_byte(bits: &[u8; 8], byte: &mut u8) {
    *byte = 0;
    for i in 0..8 {
        *byte = (*byte << 1) | (bits[7 - i] & 1);
    }
}

pub fn bits_to_byte_msb(bits: &[u8; 8], byte: &mut u8) {
    *byte = 0;
    for bit in bits.iter() {
        *byte = (*byte << 1) | (*bit & 1);
    }
}

/// Prints bits in a formatted way
pub fn print_bits(bits: &[u8], name: &str) {
    let mut output = format!("{name} = ");

    for k in 0..(bits.len() / 8) {
        let bits: &[u8; 8] = &bits[k * 8..(k + 1) * 8].try_into().unwrap();
        let mut byte = 0;
        bits_to_byte(bits, &mut byte);
        output += &format!("{}:", byte_to_string(byte));
    }

    if output.ends_with(':') {
        output.pop();
    }
    println!("{output}");
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
        _ => panic!("Invalid nibble value: {b}"),
    }
}

/// Converts u32 to bits (LSB first)
pub fn u32_to_bits(value: u32) -> [u8; 32] {
    let mut bits = [0u8; 32];
    for (i, bit) in bits.iter_mut().enumerate() {
        *bit = ((value >> i) as u8) & 1;
    }
    bits
}

pub fn u32_to_bits_msb(value: u32) -> [u8; 32] {
    let mut bits = [0u8; 32];
    for (i, bit) in bits.iter_mut().enumerate() {
        *bit = ((value >> (31 - i)) as u8) & 1;
    }
    bits
}

/// Converts bits to u32 (LSB first)
pub fn bits_to_u32(bits: &[u8; 32]) -> u32 {
    let mut value = 0u32;
    for i in (0..32).rev() {
        value = (value << 1) | (bits[i] as u32);
    }
    value
}

pub fn bits_to_u32_msb(bits: &[u8; 32]) -> u32 {
    let mut value = 0u32;
    for bit in bits.iter() {
        value = (value << 1) | (*bit as u32);
    }
    value
}

pub fn bytes_to_u32_msb(bytes: &[u8; 4]) -> u32 {
    let mut value = 0u32;
    for (i, &byte) in bytes.iter().enumerate() {
        value |= (byte as u32) << (24 - i * 8);
    }
    value
}

/// Converts u64 to bits (LSB first)
pub fn u64_to_bits(value: u64) -> [u8; 64] {
    // Divide into two of u32
    let lo = (value & 0xFFFF_FFFF) as u32;
    let hi = (value >> 32) as u32;

    let lo_bits = u32_to_bits(lo);
    let hi_bits = u32_to_bits(hi);

    // Combine into a single array
    let mut result = [0u8; 64];
    result[..32].copy_from_slice(&hi_bits);
    result[32..].copy_from_slice(&lo_bits);
    result
}

pub fn u64_to_bits_msb(value: u64) -> [u8; 64] {
    // Divide into two of u32
    let lo = (value & 0xFFFF_FFFF) as u32;
    let hi = (value >> 32) as u32;

    let lo_bits = u32_to_bits_msb(lo);
    let hi_bits = u32_to_bits_msb(hi);

    // Combine into a single array
    let mut result = [0u8; 64];
    result[..32].copy_from_slice(&hi_bits);
    result[32..].copy_from_slice(&lo_bits);
    result
}

pub fn bits_to_u64(bits: &[u8; 64]) -> u64 {
    let mut value = 0u64;
    for i in (0..64).rev() {
        value = (value << 1) | (bits[i] as u64);
    }
    value
}
