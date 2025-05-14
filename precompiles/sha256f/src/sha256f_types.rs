use serde::Deserialize;

/// Gates for the Sha256f's circuit representation
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

/// Script for the Sha256f's circuit representation
#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct Script {
    pub program: Vec<ProgramLine>,
    pub sums: Sums,
    pub total: usize,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct ProgramLine {
    pub in1: InputType,
    pub in2: InputType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in3: Option<InputType>,
    pub op: String,
    #[serde(rename = "ref")]
    pub ref_: usize,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
#[allow(dead_code)]
pub enum InputType {
    #[serde(rename = "input")]
    Input { bit: usize, wire: usize },
    #[serde(rename = "inputState")]
    InputState { bit: usize, wire: usize },
    #[serde(rename = "wired")]
    Wired { gate: usize, pin: String, wire: usize },
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct Sums {
    pub xor: usize,
    pub add: usize,
    pub ch: usize,
    pub maj: usize,
}
