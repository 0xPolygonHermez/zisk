pub mod pilout {
    include!(concat!(env!("OUT_DIR"), "/pilout.rs"));
}

use pilout::PilOut;
use prost::Message;

use std::fs::File;
use std::io::Read;
use std::path::Path;

pub fn load_pilout(airout: &Path) -> PilOut {
    // Open the file
    let file_result = File::open(airout);
    let mut file = match file_result {
        Ok(f) => f,
        Err(e) => panic!("Failed to open file {}: {}", airout.display(), e),
    };

    // Read the file content into a Vec<u8>
    let mut file_content = Vec::new();
    if let Err(e) = file.read_to_end(&mut file_content) {
        panic!("Failed to read file content {}: {}", airout.display(), e);
    }

    // Parse the protobuf message
    match PilOut::decode(file_content.as_slice()) {
        Ok(decoded) => decoded,
        Err(e) => panic!("Failed to decode protobuf message from {}: {}", airout.display(), e),
    }
}
