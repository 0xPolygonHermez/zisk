// use serde_json::Value as JsonValue;
use std::collections::HashMap;
use util::{timer_start, timer_stop_and_log};
use log::debug;
use serde::Deserialize;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct StepStruct {
    #[serde(rename = "nBits")]
    pub n_bits: u64,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
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
#[derive(Debug, Deserialize)]
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
    #[serde(rename = "cm5_2ns")]
    Cm4_2Ns = 7,
    #[serde(rename = "tmpExp_n")]
    TmpExp_N = 8,
    #[serde(rename = "q_2ns")]
    Q_2Ns = 9,
    #[serde(rename = "f_2ns")]
    F_2Ns = 10,
}

const ESECTION_VARIANTS: usize = 11;

#[derive(Debug, Deserialize)]
pub struct PolsSections {
    pub section: [u64; ESECTION_VARIANTS],
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct PolsSectionsVector {
    section: [Vec<u64>; ESECTION_VARIANTS],
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct VarPolMap {
    section: ESection,
    dim: u64,
    section_pos: u64,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct PolInfo<T> {
    map: VarPolMap,
    n: u64,
    offset: u64,
    size: u64,
    p_address: *mut T,
}

impl<T> PolInfo<T> {
    fn _get(&self, step: u64) -> *mut T {
        assert!(self.map.dim == 1);
        unsafe { self.p_address.offset((step * self.size * std::mem::size_of::<T>() as u64) as isize) }
    }

    fn _get1(&self, step: u64) -> *mut T {
        assert!(self.map.dim == 3);
        unsafe { self.p_address.offset((step * self.size * std::mem::size_of::<T>() as u64) as isize) }
    }

    fn _get2(&self, step: u64) -> *mut T {
        assert!(self.map.dim == 3);
        unsafe { self.p_address.offset((step * self.size * std::mem::size_of::<T>() as u64) as isize) }
    }

    fn _get3(&self, step: u64) -> *mut T {
        assert!(self.map.dim == 3);
        unsafe { self.p_address.offset((step * self.size * std::mem::size_of::<T>() as u64) as isize) }
    }
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct PeCtx {
    #[serde(rename = "tExpId")]
    t_exp_id: u64,
    #[serde(rename = "fExpId")]
    f_exp_id: u64,
    #[serde(rename = "zId")]
    z_id: u64,
    #[serde(rename = "c1Id")]
    c1_id: u64,
    #[serde(rename = "numId")]
    num_id: u64,
    #[serde(rename = "denId")]
    den_id: u64,
    #[serde(rename = "c2Id")]
    c2_id: u64,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct PuCtx {
    #[serde(rename = "tExpId")]
    t_exp_id: u64,
    #[serde(rename = "fExpId")]
    f_exp_id: u64,
    #[serde(rename = "h1Id")]
    h1_id: u64,
    #[serde(rename = "h2Id")]
    h2_id: u64,
    #[serde(rename = "zId")]
    z_id: u64,
    #[serde(rename = "c1Id")]
    c1_id: u64,
    #[serde(rename = "numId")]
    num_id: u64,
    #[serde(rename = "denId")]
    den_id: u64,
    #[serde(rename = "c2Id")]
    c2_id: u64,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct CiCtx {
    #[serde(rename = "zId")]
    z_id: u64,
    #[serde(rename = "numId")]
    num_id: u64,
    #[serde(rename = "denId")]
    den_id: u64,
    #[serde(rename = "c1Id")]
    c1_id: u64,
    #[serde(rename = "c2Id")]
    c2_id: u64,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
enum EvMapEType {
    #[serde(rename = "cm")]
    Cm,
    #[serde(rename = "const")]
    Const,
    #[serde(rename = "q")]
    Q,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct EvMap {
    #[serde(rename = "type")]
    type_: EvMapEType,
    #[serde(rename = "id")]
    id: u64,
    #[serde(rename = "prime", default)]
    prime: bool,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
enum StepTypeEType {
    #[serde(rename = "tmp")]
    Tmp,
    #[serde(rename = "exp")]
    Exp,
    #[serde(rename = "eval")]
    Eval,
    #[serde(rename = "challenge")]
    Challenge,
    #[serde(rename = "tree1")]
    Tree1,
    #[serde(rename = "tree2")]
    Tree2,
    #[serde(rename = "tree3")]
    Tree3,
    #[serde(rename = "tree4")]
    Tree4,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "x")]
    X,
    #[serde(rename = "Z")]
    Z,
    #[serde(rename = "public")]
    Public,
    #[serde(rename = "xDivXSubXi")]
    XDivXSubXi,
    #[serde(rename = "xDivXSubWXi")]
    XDivXSubWXi,
    #[serde(rename = "cm")]
    Cm,
    #[serde(rename = "const")]
    Const,
    #[serde(rename = "q")]
    Q,
    #[serde(rename = "Zi")]
    Zi,
    #[serde(rename = "tmpExp")]
    TmpExp,
    #[serde(rename = "f")]
    F,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct StepType {
    #[serde(rename = "type")]
    type_: StepTypeEType,
    #[serde(rename = "id", default)]
    id: u64,
    #[serde(rename = "prime", default)]
    prime: bool,
    #[serde(rename = "p", default)]
    p: u64,
    #[serde(rename = "value", default)]
    value: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
enum StepOperationEOperation {
    #[serde(rename = "add")]
    Add,
    #[serde(rename = "sub")]
    Sub,
    #[serde(rename = "mul")]
    Mul,
    #[serde(rename = "copy")]
    Copy,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct StepOperation {
    #[serde(rename = "op")]
    op: StepOperationEOperation,
    #[serde(rename = "dest")]
    dest: StepType,
    #[serde(rename = "src")]
    src: Vec<StepType>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct Step {
    #[serde(rename = "first")]
    first: Vec<StepOperation>,
    #[serde(rename = "i")]
    i: Vec<StepOperation>,
    #[serde(rename = "last")]
    last: Vec<StepOperation>,
    #[serde(rename = "tmpUsed")]
    tmp_used: u64,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct StarkInfo {
    #[serde(rename = "starkStruct")]
    pub stark_struct: StarkStruct,

    #[serde(rename = "mapTotalN")]
    pub map_total_n: u64,
    #[serde(rename = "nConstants")]
    pub n_constants: u64,
    #[serde(rename = "nPublics")]
    pub n_publics: u64,
    #[serde(rename = "nCm1")]
    pub n_cm1: u64,
    #[serde(rename = "nCm2")]
    pub n_cm2: u64,
    #[serde(rename = "nCm3")]
    pub n_cm3: u64,
    #[serde(rename = "nCm4")]
    pub n_cm4: u64,
    #[serde(rename = "qDeg")]
    pub q_deg: u64,
    #[serde(rename = "qDim")]
    pub q_dim: u64,
    #[serde(rename = "friExpId")]
    pub fri_exp_id: u64,
    #[serde(rename = "nExps")]
    pub n_exps: u64,
    #[serde(rename = "mapDeg")]
    pub map_deg: HashMap<String, u64>,
    // pub map_offsets: PolsSections,
    // pub map_sections: PolsSectionsVector,
    #[serde(rename = "mapSectionsN")]
    pub map_sections_n: HashMap<String, u64>,
    // pub map_sections_n1: PolsSections,
    // pub map_sections_n3: PolsSections,
    // pub var_pol_map: Vec<VarPolMap>,
    #[serde(rename = "qs")]
    pub qs: Vec<u64>,
    #[serde(rename = "cm_n")]
    pub cm_n: Vec<u64>,
    #[serde(rename = "cm_2ns")]
    pub cm_2ns: Vec<u64>,
    #[serde(rename = "peCtx")]
    pub pe_ctx: Vec<PeCtx>,
    #[serde(rename = "puCtx")]
    pub pu_ctx: Vec<PuCtx>,
    #[serde(rename = "ciCtx")]
    pub ci_ctx: Vec<CiCtx>,
    #[serde(rename = "evMap")]
    pub ev_map: Vec<EvMap>,
    #[serde(rename = "step2prev")]
    pub step2_prev: Step,
    #[serde(rename = "step3prev")]
    pub step3_prev: Step,
    #[serde(rename = "step3")]
    pub step3: Step,
    #[serde(rename = "step42ns")]
    pub step4_2ns: Step,
    #[serde(rename = "step52ns")]
    pub step5_2ns: Step,
    #[serde(rename = "exps_n", default)]
    pub exps_n: Vec<u64>,
    #[serde(rename = "q_2ns")]
    pub q_2ns_vector: Vec<u64>,
    #[serde(rename = "cm4_n", default)]
    pub cm4_n_vector: Vec<u64>,
    #[serde(rename = "cm4_2ns", default)]
    pub cm4_2ns_vector: Vec<u64>,
    #[serde(rename = "tmpExp_n")]
    pub tmp_exp_n: Vec<u64>,
    #[serde(rename = "exp2pol")]
    pub exp2pol: HashMap<String, u64>,
    #[serde(default)]
    pub opening_points: Vec<u64>,
    #[serde(default)]
    pub num_challenges: Vec<u64>,
    #[serde(default)]
    pub n_stages: u64,
    #[serde(default)]
    pub n_challenges: u64,
}

impl StarkInfo {
    pub fn from_json(stark_info_json: &str) -> Self {
        timer_start!(STARK_INFO_LOAD);
        debug!("starkinf: ··· Loading StarkInfo JSON");
        let mut stark_info: StarkInfo = serde_json::from_str(&stark_info_json).expect("Failed to parse JSON file");

        // TODO: THIS SHOULD NOT BE HARDCODED

        stark_info.opening_points.push(0);
        stark_info.opening_points.push(1);

        stark_info.num_challenges.push(0);
        stark_info.num_challenges.push(2);
        stark_info.num_challenges.push(2);

        stark_info.n_stages = stark_info.num_challenges.len() as u64;
        stark_info.n_challenges = stark_info.num_challenges.iter().fold(4, |acc, &x| acc + x);
        //std::accumulate(numChallenges.begin(), numChallenges.end(), 4);

        timer_stop_and_log!(STARK_INFO_LOAD);
        stark_info
    }

    // TODO!
    // /* Returns information about a polynomial specified by its ID */
    // void getPol(void * pAddress, uint64_t idPol, PolInfo &polInfo);

    // TODO!
    // /* Returns the size of a polynomial specified by its ID */
    // uint64_t getPolSize(uint64_t polId);

    // TODO!
    // /* Returns a polynomial specified by its ID */
    // Polinomial getPolinomial(Goldilocks::Element *pAddress, uint64_t idPol);

    // TODO!
    // /* Returns the size of the constant tree data/file */
    // uint64_t getConstTreeSizeInBytes (void) const
    // {
    //     uint64_t NExtended = 1 << starkStruct.nBitsExt;
    //     uint64_t constTreeSize = nConstants * NExtended + NExtended * HASH_SIZE + (NExtended - 1) * HASH_SIZE + MERKLEHASHGOLDILOCKS_HEADER_SIZE;
    //     uint64_t constTreeSizeBytes = constTreeSize * sizeof(Goldilocks::Element);
    //     return constTreeSizeBytes;
    // }
}
