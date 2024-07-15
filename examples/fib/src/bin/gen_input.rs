use std::fs::File;
use std::io::{self, Read, Write};

extern crate prost;
extern crate prost_types;

use prost::Message;

mod input {
    include!(concat!(env!("OUT_DIR"), "/inputs.rs"));
}

use input::Input;

const FILE_SIZE: usize = 0x2000;
const FILENAME: &str = "input.bin";

// Function to serialize the `Input` object to a binary file
fn serialize_input(input: &Input) -> io::Result<()> {
    let mut buf = Vec::new();
    buf.reserve(input.encoded_len());
    input.encode(&mut buf)?;

    // Write the vector to a binary file
    let mut file = File::create(FILENAME)?;
    file.write_all(&buf)?;

    Ok(())
}


// Function to deserialize the `Input` object from a binary file
fn deserialize_input() -> io::Result<Input> {
    let mut file = File::open(FILENAME)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;

    // Deserialize the data
    let input = Input::decode(&*buf).unwrap_or_else(|_| Input {
        msg: "".to_string(),
        n: 0,
        a: 0,
        b: 0,
    });

    Ok(input)
}

fn main() -> io::Result<()> {
    let input = Input {
        msg: "Hello, Zisk!! by edu".to_string(),
        n: 0,
        a: 0,
        b: 1,
    };

    // Serialize the `Input` object to a binary file with fixed size
    serialize_input(&input)?;

    // Deserialize the `Input` object from the binary file
    let input_read = deserialize_input()?;

    // Print the deserialized data
    println!("Input: {:?}", input_read);

    Ok(())
}
