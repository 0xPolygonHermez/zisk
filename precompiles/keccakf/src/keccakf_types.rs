use std::collections::HashMap;

use serde::Deserialize;

/// Connections for the Keccakf's circuit representation
#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct Connections(pub Vec<Connection>);

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct Connection(pub HashMap<String, Vec<Peer>>);

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct Peer(pub String, pub usize);

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