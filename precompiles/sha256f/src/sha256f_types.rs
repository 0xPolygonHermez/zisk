use serde::Deserialize;

/// Connections for the Keccakf's circuit representation
#[derive(Deserialize, Debug)]
pub struct Gate {
    #[serde(rename = "type")]
    pub op: GateOp,
    pub connections: [usize; 4],
}

#[derive(Deserialize, Debug)]
#[allow(non_camel_case_types)]
pub enum GateOp {
    xor,
    ch,
    maj,
    add,
}

/// Script for the Keccakf's circuit representation
#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct Script {
    pub xors: usize,
    pub andps: usize,
    #[serde(rename = "maxRef")]
    pub maxref: usize,
    pub program: Vec<ProgramLine>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct ProgramLine {
    pub a: ValueType,
    pub b: ValueType,
    pub op: String,
    #[serde(rename = "ref")]
    pub ref_: usize,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
#[allow(dead_code)]
pub enum ValueType {
    Input(InputData),
    Wired(WiredData),
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct InputData {
    pub bit: usize,
    #[serde(rename = "type")]
    type_: String,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct WiredData {
    pub gate: usize,
    pub pin: String,
    #[serde(rename = "type")]
    type_: String,
}