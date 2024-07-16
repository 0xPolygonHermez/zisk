use indexmap::IndexMap;
use serde::{Deserialize, Deserializer};
use serde_json::Value;


use crate::ZiskOperator;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct RomProgram {
    #[serde(rename = "nextInitInstAddr")]
    pub next_init_inst_addr: usize,
    #[serde(rename = "insts")]
    #[serde(deserialize_with = "deserialize_insts")]
    pub insts: IndexMap<String, RomInstruction>,
    #[serde(rename = "roData")]
    pub ro_data: Vec<RomRoData>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct RomInstruction {
    pub paddr: u64,
    #[serde(deserialize_with = "deserialize_bool")]
    pub store_ra: bool,
    #[serde(deserialize_with = "deserialize_bool")]
    pub store_use_sp: bool,
    pub store: RomStore,
    pub store_offset: i64,
    #[serde(deserialize_with = "deserialize_bool")]
    pub set_pc: bool,
    #[serde(deserialize_with = "deserialize_bool")]
    pub set_sp: bool,
    pub ind_width: u64,
    pub inc_sp: i64,
    #[serde(deserialize_with = "deserialize_bool")]
    pub end: bool,
    pub a_src: RomSrc,
    pub a_use_sp_imm1: isize,
    pub a_offset_imm0: isize,
    pub b_src: RomSrc,
    pub b_use_sp_imm1: isize,
    pub b_offset_imm0: isize,
    pub jmp_offset1: isize,
    pub jmp_offset2: isize,
    #[serde(deserialize_with = "deserialize_bool")]
    pub is_external_op: bool,
    pub op: ZiskOperator,
    #[serde(rename = "opStr")]
    pub op_str: String,
    pub verbose: String,
}

#[derive(Debug)]
pub enum RomStore {
    StoreNone,
    StoreMem,
    StoreInd,
}

impl<'de> Deserialize<'de> for RomStore {
    fn deserialize<D>(deserializer: D) -> Result<RomStore, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value: u64 = Deserialize::deserialize(deserializer)?;
        match value {
            0 => Ok(RomStore::StoreNone),
            1 => Ok(RomStore::StoreMem),
            2 => Ok(RomStore::StoreInd),
            _ => Err(serde::de::Error::custom("Invalid value for RomStore")),
        }
    }
}

#[derive(Debug)]
pub enum RomSrc {
    SrcC,
    SrcMem,
    SrcImm,
    SrcStep,
    SrcSp,
    SrcInd,
}

impl<'de> Deserialize<'de> for RomSrc {
    fn deserialize<D>(deserializer: D) -> Result<RomSrc, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value: u64 = Deserialize::deserialize(deserializer)?;
        match value {
            0 => Ok(RomSrc::SrcC),
            1 => Ok(RomSrc::SrcMem),
            2 => Ok(RomSrc::SrcImm),
            3 => Ok(RomSrc::SrcStep),
            4 => Ok(RomSrc::SrcSp),
            5 => Ok(RomSrc::SrcInd),
            _ => Err(serde::de::Error::custom("Invalid value for RomSrc")),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct RomRoData {
    pub start: usize,
    pub data: RomRoData2,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct RomRoData2 {
    #[serde(rename = "type")]
    pub type_: RomRoDataType,
    pub data: Vec<usize>,
}

#[derive(Debug, Deserialize)]
pub enum RomRoDataType {
    #[serde(rename = "Buffer")]
    Buffer,
}

fn deserialize_insts<'de, D>(deserializer: D) -> Result<IndexMap<String, RomInstruction>, D::Error>
where
    D: Deserializer<'de>,
{
    let value: IndexMap<String, Value> = Deserialize::deserialize(deserializer)?;
    value
        .into_iter()
        .map(|(k, v)| serde_json::from_value(v).map(|inst| (k, inst)).map_err(serde::de::Error::custom))
        .collect()
}

fn deserialize_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let value: u8 = Deserialize::deserialize(deserializer)?;
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(serde::de::Error::custom("expected 0 or 1")),
    }
}

#[allow(dead_code)]
impl RomProgram {
    pub fn from_file(file_path: &str) -> Result<RomProgram, std::io::Error> {
        let path = std::path::Path::new(file_path);
        if !path.exists() {
            println!("File {} does not exist", file_path);
            return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "File not found"));
        } else {
            println!("File exists");
        }
        let file_contents = std::fs::read_to_string(file_path)?;

        let parsed_json: RomProgram = serde_json::from_str(&file_contents)?;

        Ok(parsed_json)
    }

    pub fn from_json(input_json: &str) -> Result<RomProgram, serde_json::Error> {
        let parsed_json: RomProgram = serde_json::from_str(input_json)?;

        Ok(parsed_json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test deserialization and parsing of JSON input
    #[test]
    fn test_parse_rom_json() {
        let rom_program_json = RomProgram::from_file("./data/rom.json");
        assert!(rom_program_json.is_ok());

        let rom_program = rom_program_json.unwrap();
        println!("{:?}", rom_program.insts);
    }

    // Test deserialization and parsing of JSON input with wrong fields
    #[test]
    fn test_parse_input_json_wrong_fields() {
        let input_json = r#"
            {
                "wrong": "fields"
            }
        "#;

        let rom_program_json = RomProgram::from_json(input_json);
        assert!(rom_program_json.is_err());
    }
}
