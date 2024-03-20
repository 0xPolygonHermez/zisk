use crate::pilout::PilOut;
use prost::{DecodeError, Message};

use std::fs::File;
use std::io::Read;
use std::ops::Deref;

use log::{debug, info};
use util::{timer_start, timer_stop_and_log};

#[derive(Debug)]
pub struct PilOutProxy {
    pub pilout: PilOut,
}

impl PilOutProxy {
    const MY_NAME: &'static str = "piloutPx";

    pub fn new(pilout_filename: &str) -> Result<PilOutProxy, Box<dyn std::error::Error>> {
        let pilout = Self::load_pilout(pilout_filename)?;
        Ok(PilOutProxy { pilout })
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

    pub fn num_stages(&self) -> u32 {
        self.pilout.num_challenges.len() as u32
    }

    pub fn print_pilout_info(&self) {
        // Print PilOut subproofs and airs names and degrees
        info!("{}: ··· '{}' PilOut info", Self::MY_NAME, self.name.as_ref().unwrap());

        let base_field: &Vec<u8> = self.pilout.base_field.as_ref();
        let mut hex_string = "0x".to_owned();
        for &byte in base_field {
            hex_string.push_str(&format!("{:02X}", byte));
        }
        info!("{}:     Base field: {}", Self::MY_NAME, hex_string);

        info!("{}:     Subproofs:", Self::MY_NAME);
        for (subproof_index, subproof) in self.pilout.subproofs.iter().enumerate() {
            info!(
                "{}:     + [{}] {} (aggregable: {}, subproof values: {})",
                Self::MY_NAME,
                subproof_index,
                subproof.name.as_ref().unwrap(),
                subproof.aggregable,
                subproof.subproofvalues.len()
            );

            for (air_index, air) in self.pilout.subproofs[subproof_index].airs.iter().enumerate() {
                info!(
                    "{}:       [{}][{}] {} (rows: {}, fixed cols: {}, periodic cols: {}, stage widths: {}, expressions: {}, constraints: {})",
                    Self::MY_NAME,
                    subproof_index,
                    air_index,
                    air.name.as_ref().unwrap(),
                    air.num_rows.unwrap(),
                    air.fixed_cols.len(),
                    air.periodic_cols.len(),
                    air.stage_widths.len(),
                    air.expressions.len(),
                    air.constraints.len()
                );
            }
        }

        info!("{}:     Challenges: {}", Self::MY_NAME, self.pilout.num_challenges.len());
        for i in 0..self.pilout.num_challenges.len() {
            info!("{}:       stage {}: {}", Self::MY_NAME, i, self.pilout.num_challenges[i]);
        }

        info!(
            "{}:     #Proof values: {}, #Public values: {}, #Global expressions: {}, #Global constraints: {}",
            Self::MY_NAME,
            self.pilout.num_proof_values,
            self.pilout.num_public_values,
            self.pilout.expressions.len(),
            self.pilout.constraints.len()
        );
        info!("{}:     #Hints: {}, #Symbols: {}", Self::MY_NAME, self.pilout.hints.len(), self.pilout.symbols.len());
        info!("{}:     Public tables: {}", Self::MY_NAME, self.pilout.public_tables.len());
    }
}

impl Deref for PilOutProxy {
    type Target = PilOut;

    fn deref(&self) -> &Self::Target {
        &self.pilout
    }
}

impl Default for PilOutProxy {
    fn default() -> Self {
        PilOutProxy { pilout: PilOut::default() }
    }
}
