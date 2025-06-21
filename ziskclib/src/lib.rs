use generic_array::{typenum::U64, GenericArray};
use sha2::compress256;
use tiny_keccak::keccakf;

#[no_mangle]
pub extern "C" fn zisk_keccakf(data: &mut [u64; 25]) {
    //println!("zisk_keccakf() starting...");
    keccakf(data);
    //println!("zisk_keccakf() ...done");
}

#[no_mangle]
pub extern "C" fn zisk_sha256(state: &mut [u64; 4], input: &[u64; 8]) {
    //println!("zisk_sha256() starting...");
    let mut state_u32 = convert_u64_to_u32_be_words(state);
    let block: GenericArray<u8, U64> = u64s_to_generic_array_be(input);
    let blocks = &[block];
    compress256(&mut state_u32, blocks);

    let state_output = convert_u32s_back_to_u64_be(&state_u32);
    state[0] = state_output[0];
    state[1] = state_output[1];
    state[2] = state_output[2];
    state[3] = state_output[3];
    //println!("zisk_sha256() ...done");
}

pub fn convert_u64_to_u32_be_words(input: &[u64; 4]) -> [u32; 8] {
    let mut out = [0u32; 8];
    for (i, &word) in input.iter().enumerate() {
        let bytes = word.to_be_bytes();
        out[2 * i] = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        out[2 * i + 1] = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    }
    out
}

pub fn u64s_to_generic_array_be(input: &[u64; 8]) -> GenericArray<u8, U64> {
    let mut out = [0u8; 64];
    for (i, word) in input.iter().enumerate() {
        let bytes = word.to_be_bytes();
        out[i * 8..(i + 1) * 8].copy_from_slice(&bytes);
    }
    GenericArray::<u8, U64>::clone_from_slice(&out)
}

pub fn convert_u32s_back_to_u64_be(words: &[u32; 8]) -> [u64; 4] {
    let mut out = [0u64; 4];
    for i in 0..4 {
        let high = words[2 * i].to_be_bytes();
        let low = words[2 * i + 1].to_be_bytes();
        out[i] = u64::from_be_bytes([
            high[0], high[1], high[2], high[3], low[0], low[1], low[2], low[3],
        ]);
    }
    out
}
