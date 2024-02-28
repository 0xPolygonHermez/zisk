// use serde_json::Value as JsonValue;
use std::collections::HashMap;
use util::{timer_start, timer_stop_and_log};
use log::debug;
use serde::Deserialize;

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct Boundary {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "offsetMin")]
    pub offset_min: Option<u64>,
    #[serde(rename = "offsetMax")]
    pub offset_max: Option<u64>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct StepStruct {
    #[serde(rename = "nBits")]
    pub n_bits: u64,
}

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct StarkStruct {
    #[serde(rename = "nBits")]
    pub n_bits: u64,
    #[serde(rename = "nBitsExt")]
    pub n_bits_ext: u64,
    #[serde(rename = "nQueries")]
    pub n_queries: u64,
    #[serde(rename = "verificationHashType")]
    pub verification_hash_type: String,
    #[serde(rename = "steps")]
    pub steps: Vec<StepStruct>,
}

#[allow(dead_code)]
#[allow(non_camel_case_types)]
#[derive(Deserialize)]
pub enum OpType {
    #[serde(rename = "const_")]
    Const = 0,
    #[serde(rename = "cm")]
    Cm = 1,
    #[serde(rename = "tmp")]
    Tmp = 2,
    #[serde(rename = "public_")]
    Public = 3,
    #[serde(rename = "subproofvalue")]
    SubproofValue = 4,
    #[serde(rename = "challenge")]
    Challenge = 5,
    #[serde(rename = "number")]
    Number = 6,
}

#[allow(dead_code)]
#[allow(non_camel_case_types)]
#[derive(Deserialize, Eq, PartialEq, Hash)]
pub enum ESection {
    #[serde(rename = "cm1_n")]
    Cm1_N = 0,
    #[serde(rename = "cm1_2ns")]
    Cm1_2Ns = 1,
    #[serde(rename = "cm2_n")]
    Cm2_N = 2,
    #[serde(rename = "cm2_2ns")]
    Cm2_2Ns = 3,
    #[serde(rename = "cm3_n")]
    Cm3_N = 4,
    #[serde(rename = "cm3_2ns")]
    Cm3_2Ns = 5,
    #[serde(rename = "cm4_n")]
    Cm4_N = 6,
    #[serde(rename = "cm4_2ns")]
    Cm4_2Ns = 7,
    #[serde(rename = "tmpExp_n")]
    TmpExp_N = 8,
    #[serde(rename = "q_2ns")]
    Q_2Ns = 9,
    #[serde(rename = "f_2ns")]
    F_2Ns = 10,
}

const ESECTION_VARIANTS: usize = 11;

#[allow(dead_code)]
#[allow(non_camel_case_types)]
#[derive(Deserialize)]
pub enum HintType {
    #[serde(rename = "h1h2")]
    H1H2 = 0,
    #[serde(rename = "gprod")]
    GProd = 1,
    #[serde(rename = "publicValue")]
    PublicValue = 2,
}

#[derive(Deserialize)]
pub struct PolsSections {
    pub section: [u64; ESECTION_VARIANTS],
}

#[derive(Deserialize)]
pub struct CmPolMap {
    pub stage: String,
    #[serde(rename = "stageNum")]
    pub stage_num: u64,
    pub name: String,
    pub dim: u64,
    #[serde(rename = "imPol")]
    pub im_pol: bool,
    #[serde(rename = "stagePos")]
    pub stage_pos: u64,
    #[serde(rename = "stageId")]
    pub stage_id: u64,
}

#[derive(Deserialize)]
pub struct Symbol {
    pub op: OpType,
    pub stage: Option<u64>,
    #[serde(rename = "stageId")]
    pub stage_id: Option<u64>,
    pub id: Option<u64>,
    pub value: Option<u64>,
}

// HINTS
// =================================================================================================
#[derive(Deserialize)]
#[serde(tag = "name")]
pub enum Hint {
    #[serde(rename = "gprod")]
    GProd(GProdHint),
    #[serde(rename = "h1h2")]
    H1H2(H1H2Hint),
    #[serde(rename = "public")]
    Public(PublicHint),
}

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct GProdHint {
    numerator: Symbol,
    denominator: Symbol,
    dest: Vec<Symbol>,
    fields: Vec<String>,
    symbols: Vec<Symbol>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct H1H2Hint {
    f: Symbol,
    t: Symbol,
    dest: Vec<Symbol>,
    fields: Vec<String>,
    symbols: Vec<Symbol>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct PublicHint {
    // row_index: XXX,
    // expression: YYY,
    dest: Vec<Symbol>,
    fields: Vec<String>,
    symbols: Vec<Symbol>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct VarPolMap {
    section: ESection,
    dim: u64,
    section_pos: Option<u64>,
    deg: Option<u64>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
enum EvMapEType {
    #[serde(rename = "cm")]
    Cm,
    #[serde(rename = "const")]
    Const,
    #[serde(rename = "q")]
    Q,
}

#[allow(dead_code)]
#[allow(non_camel_case_types)]
#[derive(Deserialize, Eq, PartialEq, Hash)]
pub enum EMapSectionsN {
    #[serde(rename = "cm1_n")]
    Cm1_N = 0,
    #[serde(rename = "cm1_ext")]
    Cm1_Ext = 1,
    #[serde(rename = "cm2_n")]
    Cm2_N = 2,
    #[serde(rename = "cm2_ext")]
    Cm2_Ext = 3,
    #[serde(rename = "cm3_n")]
    Cm3_N = 4,
    #[serde(rename = "cm3_ext")]
    Cm3_Ext = 5,
    #[serde(rename = "cmQ_n")]
    CmQ_N = 6,
    #[serde(rename = "cmQ_ext")]
    CmQ_Ext = 7,
    #[serde(rename = "tmpExp_n")]
    TmpExp_N = 8,
    #[serde(rename = "const_n")]
    Const_N = 9,
    #[serde(rename = "const_ext")]
    Const_Ext = 10,
    #[serde(rename = "q_ext")]
    Q_Ext = 11,
    #[serde(rename = "f_ext")]
    F_Ext = 12,
}

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct EvMap {
    #[serde(rename = "type")]
    type_: EvMapEType,
    name: Option<String>,
    id: u64,
    prime: Option<bool>,
    stage: Option<u64>,
    dim: Option<u64>,
    #[serde(rename = "subproofId")]
    subproof_id: Option<u64>,
    #[serde(rename = "airId")]
    air_id: Option<u64>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct ExpressionsCode {
    #[serde(rename = "expId", default)]
    exp_id: u64,
    stage: u64,
    symbols: Vec<Symbol>,
    // code: Vec<XXX>,
    // dest: Vec<XXX>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct CodeStage {
    #[serde(rename = "tmpUsed", default)]
    tmp_used: u64,
    // code: Vec<XXX>,
    // #[serde(rename = "symbolsCalculated", default)]
    // symbols_calculated: Vec<XXX>,
    // #[serde(rename = "symbolsUsed", default)]
    // symbols_used: Vec<XXX>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct StarkInfo {
    #[serde(rename = "starkStruct")]
    pub stark_struct: StarkStruct,

    pub pil2: Option<bool>,

    #[serde(rename = "nCm1")]
    pub n_cm1: u64,
    #[serde(rename = "nConstants")]
    pub n_constants: u64,
    #[serde(rename = "nPublics")]
    pub n_publics: u64,

    #[serde(rename = "numChallenges")]
    pub num_challenges: Option<Vec<u64>>,

    #[serde(rename = "nSubAirValues")]
    pub n_subair_values: Option<u64>,

    #[serde(rename = "openingPoints")]
    pub opening_points: Option<Vec<u64>>,

    pub boundaries: Option<Vec<Boundary>>,

    #[serde(rename = "qDeg")]
    pub q_deg: u64,
    #[serde(rename = "qDim")]
    pub q_dim: u64,
    pub qs: Vec<u64>,

    #[serde(rename = "mapTotalN")]
    pub map_total_n: u64,
    #[serde(rename = "mapSectionsN")]
    pub map_sections_n: HashMap<ESection, u64>,
    #[serde(rename = "mapOffsets")]
    pub map_offsets: HashMap<ESection, u64>,

    // pil2-stark-js specific
    #[serde(rename = "cmPolsMap")]
    pub cm_pols_map: Option<Vec<CmPolMap>>,
    #[serde(rename = "symbolsStage")]
    pub symbols_stage: Option<Vec<Vec<Symbol>>>,

    pub code: Option<HashMap<String, CodeStage>>,

    #[serde(rename = "expressionsCode")]
    pub expressions_code: Option<HashMap<u64, ExpressionsCode>>,

    pub hints: Option<Vec<Hint>>,

    //Exclusius de PIL1
    #[serde(rename = "varPolMap")]
    pub var_pol_map: Option<Vec<VarPolMap>>,
    #[serde(rename = "cm_n")]
    pub cm_n: Option<Vec<u64>>,
    #[serde(rename = "cm_2ns")]
    pub cm_2ns: Option<Vec<u64>>,

    #[serde(rename = "evMap")]
    pub ev_map: Vec<EvMap>,

    #[serde(rename = "exp2pol")]
    pub exp2pol: Option<HashMap<String, u64>>,

    // Computed fields:
    pub n_stages: Option<u64>,
    pub n_challenges: Option<u64>,
}

impl StarkInfo {
    pub fn from_json(stark_info_json: &str) -> Self {
        timer_start!(STARK_INFO_LOAD);

        debug!("starkinf: ··· Loading StarkInfo JSON");
        let mut stark_info: StarkInfo = serde_json::from_str(&stark_info_json).expect("Failed to parse JSON file");

        if stark_info.n_subair_values.is_none() {
            stark_info.n_subair_values = Some(0);
        }

        if stark_info.num_challenges.is_none() {
            stark_info.num_challenges = Some(vec![0, 2, 2]);
        }

        stark_info.n_stages = Some(stark_info.num_challenges.as_ref().unwrap().len() as u64);

        stark_info.n_challenges = Some(stark_info.num_challenges.as_ref().unwrap().iter().sum::<u64>() + 4);

        if stark_info.opening_points.is_none() {
            stark_info.opening_points = Some(vec![0, 1]);
        }

        if stark_info.boundaries.is_none() {
            stark_info.boundaries =
                Some(vec![Boundary { name: "everyRow".to_string(), offset_min: Some(0), offset_max: Some(0) }]);
        }

        timer_stop_and_log!(STARK_INFO_LOAD);
        stark_info
    }
}
