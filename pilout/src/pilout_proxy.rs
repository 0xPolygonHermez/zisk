use crate::pilout::PilOut;
use prost::{DecodeError, Message};

use std::fs::File;
use std::io::Read;
use std::ops::Deref;

use log::debug;
use util::{timer_start, timer_stop_and_log};
use std::time::Instant;

#[derive(Debug)]
pub struct PilOutProxy {
    pub pilout: PilOut,
}

impl PilOutProxy {
    const MY_NAME: &'static str = "piloutPx";

    pub fn new(pilout_filename: &str) -> Self {
        let pilout = Self::load_pilout(pilout_filename);
        let pilout = match pilout {
            Ok(pilout) => pilout,
            Err(e) => panic!("Failed to load pilout: {}", e),
        };
        PilOutProxy { pilout }
    }

    fn load_pilout(pilout_filename: &str) -> Result<PilOut, DecodeError> {
        timer_start!(LOADING_PILOUT);
        debug!("{}: ··· Loading pilout", Self::MY_NAME);

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
        let result = PilOut::decode(file_content.as_slice());
        timer_stop_and_log!(LOADING_PILOUT);

        result
    }

    pub fn find_subproof_id_by_name(&self, name: &str) -> Option<usize> {
        self.pilout.subproofs.iter().position(|x| x.name.as_deref() == Some(name))
    }

    pub fn print_pilout_info(&self) {
        // Print PilOut subproofs and airs names and degrees
        debug!("{}: ··· '{}' PilOut info", Self::MY_NAME, self.name.as_ref().unwrap());

        let base_field: &Vec<u8> = self.pilout.base_field.as_ref();
        let mut hex_string = "0x".to_owned();
        for &byte in base_field {
            hex_string.push_str(&format!("{:02X}", byte));
        }
        debug!("{}:     Base field: {}", Self::MY_NAME, hex_string);

        debug!("{}:     Subproofs:", Self::MY_NAME);
        for (subproof_index, subproof) in self.pilout.subproofs.iter().enumerate() {
            debug!(
                "{}:     [{}] {} (aggregable: {}, subproof values: {})",
                Self::MY_NAME,
                subproof_index,
                subproof.name.as_ref().unwrap(),
                subproof.aggregable,
                subproof.subproofvalues.len()
            );
            for (air_index, air) in self.pilout.subproofs[subproof_index].airs.iter().enumerate() {
                debug!("{}:       [{}] {}", Self::MY_NAME, air_index, air.name.as_ref().unwrap());
                debug!("{}:         rows: {}, fixed cols: {}, periodic cols: {}, stage widths: {}, expressions: {}, constraints: {}", Self::MY_NAME,
                    air.num_rows.unwrap(), air.fixed_cols.len(), air.periodic_cols.len(), air.stage_widths.len(), air.expressions.len(), air.constraints.len());
            }
        }

        debug!("{}:     Challenges: {}", Self::MY_NAME, self.pilout.num_challenges.len());
        for i in 0..self.pilout.num_challenges.len() {
            debug!("{}:       stage {}: {}", Self::MY_NAME, i, self.pilout.num_challenges[i]);
        }


        debug!("{}:     Number of proof values: {}", Self::MY_NAME, self.pilout.num_proof_values);
        debug!("{}:     Number of public values: {}", Self::MY_NAME, self.pilout.num_public_values);
        debug!("{}:     Public tables: {}", Self::MY_NAME, self.pilout.public_tables.len());
        debug!("{}:     Global expressions: {}", Self::MY_NAME, self.pilout.expressions.len());
        debug!("{}:     Global constraints: {}", Self::MY_NAME, self.pilout.constraints.len());
        debug!("{}:     Hints: {}", Self::MY_NAME, self.pilout.hints.len());
        debug!("{}:     Symbols: {}", Self::MY_NAME, self.pilout.symbols.len());
    }
}

impl Deref for PilOutProxy {
    type Target = PilOut;

    fn deref(&self) -> &Self::Target {
        &self.pilout
    }
}
