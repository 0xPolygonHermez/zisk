pub mod pilout {
    include!(concat!(env!("OUT_DIR"), "/pilout.rs"));
}

use pilout::PilOut;
use prost::Message;

use std::fs::File;
use std::io::Read;

pub fn load_pilout(airout: &str) -> PilOut {
    // Open the file
    let mut file = File::open(airout).unwrap_or_else(|error| {
        panic!("Failed to open file {}: {}", airout, error);
    });

    // Read the file content into a Vec<u8>
    let mut file_content = Vec::new();
    if let Err(e) = file.read_to_end(&mut file_content) {
        panic!("Failed to read file content {}: {}", airout, e);
    }

    // Parse the protobuf message
    match PilOut::decode(file_content.as_slice()) {
        Ok(decoded) => decoded,
        Err(e) => panic!("Failed to decode protobuf message from {}: {}", airout, e),
    }
}

pub fn find_subproof_id_by_name(pilout: &PilOut, name: &str) -> Option<usize> {
    pilout
        .subproofs
        .iter()
        .position(|x| x.name.as_deref() == Some(name))
}