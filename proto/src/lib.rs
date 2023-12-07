pub mod pilout;

use pilout::PilOut;
use prost::Message;

use std::fs::File;
use std::io::Read;

use std::path::Path;

pub fn get_pilout(airout: &Path) -> PilOut {
    // Read the file content into a Vec<u8>
    let mut file = File::open(airout).expect("Failed to open file");

    let mut file_content = Vec::new();
    file.read_to_end(&mut file_content).expect("Failed to read file content");

    // Parse the protobuf message
    PilOut::decode(file_content.as_slice()).expect("Failed to decode protobuf message")
}