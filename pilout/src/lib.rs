pub mod pilout {
    include!(concat!(env!("OUT_DIR"), "/pilout.rs"));
}

use pilout::PilOut;
use prost::Message;

use std::fs::File;
use std::io::Read;

pub fn load_pilout(pilout_filename: &str) -> PilOut {
    // Open the file
    let mut file = File::open(pilout_filename).unwrap_or_else(|error| {
        panic!("Failed to open file {}: {}", pilout_filename, error);
    });

    // Read the file content into a Vec<u8>
    let mut file_content = Vec::new();
    if let Err(e) = file.read_to_end(&mut file_content) {
        panic!("Failed to read file content {}: {}", pilout_filename, e);
    }

    // Parse the protobuf message
    match PilOut::decode(file_content.as_slice()) {
        Ok(decoded) => decoded,
        Err(e) => panic!("Failed to decode protobuf message from {}: {}", pilout_filename, e),
    }
}

pub fn find_subproof_id_by_name(pilout: &PilOut, name: &str) -> Option<usize> {
    pilout.subproofs.iter().position(|x| x.name.as_deref() == Some(name))
}
