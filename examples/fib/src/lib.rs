use prost::Message;
use tiny_keccak::Hasher;
use tiny_keccak::Keccak;

mod input {
    include!(concat!(env!("OUT_DIR"), "/inputs.rs"));
}

use input::Input;

pub fn main(input: &[u8], _output: &mut [u8]) {
    // Deserializar el vector a un objeto `Input`
    let input_read = Input::decode(input).unwrap();

    println!("Input: {:?}", input.len());
    

   
    println!("Input: {:?}", input_read);

    let mut a: u64 = input_read.a;
    let mut b: u64 = input_read.b;

    for _ in 0..input_read.n {
        let c: u64 = a + b;
        a = b;
        b = c;
    }

    println!("fib({}) = {}", input_read.n, a);
  
    let hash = __compute_keccak(input_read.msg.as_str());
    println!("keccak256({}) = {:?}", input_read.msg, hash);
  
}

#[no_mangle]
fn __compute_keccak(str: &str) -> [u8; 32] {
    let mut keccak = Keccak::v256();
    let mut output = [0; 32];

    keccak.update(str.as_bytes());
    keccak.finalize(&mut output);
    output
}
