// use serde_json::Value as JsonValue;
use std::collections::HashMap;
use goldilocks::Goldilocks;

#[derive(Debug)]
struct StepStruct {
    n_bits: u64,
}

#[derive(Debug)]
struct StarkStruct {
    n_bits: u64,
    n_bits_ext: u64,
    n_queries: u64,
    verification_hash_type: String,
    steps: Vec<StepStruct>,
}

#[derive(Debug)]
#[allow(non_camel_case_types)]
enum ESection {
    Cm1_N = 0,
    Cm1_2Ns = 1,
    Cm2_N = 2,
    Cm2_2Ns = 3,
    Cm3_N = 4,
    Cm3_2Ns = 5,
    Cm4_N = 6,
    Cm4_2Ns = 7,
    TmpExp_N = 8,
    Q_2Ns = 9,
    F_2Ns = 10,
}

const ESECTION_VARIANTS: usize = 11;

fn string_to_section(s: &str) -> Option<ESection> {
    match s {
        "cm1_n" => Some(ESection::Cm1_N),
        "cm1_2ns" => Some(ESection::Cm1_2Ns),
        "cm2_n" => Some(ESection::Cm2_N),
        "cm2_2ns" => Some(ESection::Cm2_2Ns),
        "cm3_n" => Some(ESection::Cm3_N),
        "cm3_2ns" => Some(ESection::Cm3_2Ns),
        "cm4_n" => Some(ESection::Cm4_N),
        "cm4_2ns" => Some(ESection::Cm4_2Ns),
        "tmpExp_n" => Some(ESection::TmpExp_N),
        "q_2ns" => Some(ESection::Q_2Ns),
        "f_2ns" => Some(ESection::F_2Ns),
        _ => None,
    }
}

#[derive(Debug)]
struct PolsSections {
    section: [u64; ESECTION_VARIANTS],
}

#[derive(Debug)]
struct PolsSectionsVector {
    section: [Vec<u64>; ESECTION_VARIANTS],
}

#[derive(Debug)]
struct VarPolMap {
    section: ESection,
    dim: u64,
    section_pos: u64,
}

#[derive(Debug)]
struct PolInfo<T> {
    map: VarPolMap,
    n: u64,
    offset: u64,
    size: u64,
    p_address: *mut T,
}

impl<T> PolInfo<T> {
    fn get(&self, step: u64) -> *mut T {
        assert!(self.map.dim == 1);
        unsafe { self.p_address.offset((step * self.size * std::mem::size_of::<T>() as u64) as isize) }
    }

    fn get1(&self, step: u64) -> *mut T {
        assert!(self.map.dim == 3);
        unsafe { self.p_address.offset((step * self.size * std::mem::size_of::<T>() as u64) as isize) }
    }

    fn get2(&self, step: u64) -> *mut T {
        assert!(self.map.dim == 3);
        unsafe { self.p_address.offset((step * self.size * std::mem::size_of::<T>() as u64) as isize) }
    }

    fn get3(&self, step: u64) -> *mut T {
        assert!(self.map.dim == 3);
        unsafe { self.p_address.offset((step * self.size * std::mem::size_of::<T>() as u64) as isize) }
    }
}

#[derive(Debug)]
struct PeCtx {
    t_exp_id: u64,
    f_exp_id: u64,
    z_id: u64,
    c1_id: u64,
    num_id: u64,
    den_id: u64,
    c2_id: u64,
}

#[derive(Debug)]
struct PuCtx {
    t_exp_id: u64,
    f_exp_id: u64,
    h1_id: u64,
    h2_id: u64,
    z_id: u64,
    c1_id: u64,
    num_id: u64,
    den_id: u64,
    c2_id: u64,
}

#[derive(Debug)]
struct CiCtx {
    z_id: u64,
    num_id: u64,
    den_id: u64,
    c1_id: u64,
    c2_id: u64,
}

#[derive(Debug)]
enum EvMapEType {
    Cm,
    Const,
    Q,
}

#[derive(Debug)]
struct EvMap {
    type_: EvMapEType,
    id: u64,
    prime: bool,
}

impl EvMap {
    fn set_type(&mut self, s: &str) {
        match s {
            "cm" => self.type_ = EvMapEType::Cm,
            "const" => self.type_ = EvMapEType::Const,
            "q" => self.type_ = EvMapEType::Q,
            _ => {
                eprintln!("EvMap::set_type() found invalid type: {}", s);
                std::process::exit(1);
            }
        }
    }
}

#[derive(Debug)]
enum StepTypeEType {
    Tmp,
    Exp,
    Eval,
    Challenge,
    Tree1,
    Tree2,
    Tree3,
    Tree4,
    Number,
    X,
    Z,
    Public,
    XDivXSubXi,
    XDivXSubWXi,
    Cm,
    Const,
    Q,
    Zi,
    TmpExp,
    F,
}

#[derive(Debug)]
struct StepType {
    type_: StepTypeEType,
    id: u64,
    prime: bool,
    p: u64,
    value: String,
}

impl StepType {
    fn set_type(&mut self, s: &str) {
        match s {
            "tmp" => self.type_ = StepTypeEType::Tmp,
            "exp" => self.type_ = StepTypeEType::Exp,
            "eval" => self.type_ = StepTypeEType::Eval,
            "challenge" => self.type_ = StepTypeEType::Challenge,
            "tree1" => self.type_ = StepTypeEType::Tree1,
            "tree2" => self.type_ = StepTypeEType::Tree2,
            "tree3" => self.type_ = StepTypeEType::Tree3,
            "tree4" => self.type_ = StepTypeEType::Tree4,
            "number" => self.type_ = StepTypeEType::Number,
            "x" => self.type_ = StepTypeEType::X,
            "Z" => self.type_ = StepTypeEType::Z,
            "public" => self.type_ = StepTypeEType::Public,
            "xDivXSubXi" => self.type_ = StepTypeEType::XDivXSubXi,
            "xDivXSubWXi" => self.type_ = StepTypeEType::XDivXSubWXi,
            "cm" => self.type_ = StepTypeEType::Cm,
            "const" => self.type_ = StepTypeEType::Const,
            "q" => self.type_ = StepTypeEType::Q,
            "Zi" => self.type_ = StepTypeEType::Zi,
            "tmpExp" => self.type_ = StepTypeEType::TmpExp,
            "f" => self.type_ = StepTypeEType::F,
            _ => {
                eprintln!("StepType::set_type() found invalid type: {}", s);
                std::process::exit(1);
            }
        }
    }
}

#[derive(Debug)]
enum StepOperationEOperation {
    Add,
    Sub,
    Mul,
    Copy,
}

#[derive(Debug)]
struct StepOperation {
    op: StepOperationEOperation,
    dest: StepType,
    src: Vec<StepType>,
}

impl StepOperation {
    fn set_operation(&mut self, s: &str) {
        match s {
            "add" => self.op = StepOperationEOperation::Add,
            "sub" => self.op = StepOperationEOperation::Sub,
            "mul" => self.op = StepOperationEOperation::Mul,
            "copy" => self.op = StepOperationEOperation::Copy,
            _ => {
                eprintln!("StepOperation::set_operation() found invalid type: {}", s);
                std::process::exit(1);
            }
        }
    }
}

#[derive(Debug)]
struct Step {
    first: Vec<StepOperation>,
    i: Vec<StepOperation>,
    last: Vec<StepOperation>,
    tmp_used: u64,
}

#[derive(Debug)]
struct StarkInfo {
    pub stark_struct: StarkStruct,
    pub map_total_n: u64,
    pub n_constants: u64,
    pub n_publics: u64,
    pub n_cm1: u64,
    pub n_cm2: u64,
    pub n_cm3: u64,
    pub n_cm4: u64,
    pub q_deg: u64,
    pub q_dim: u64,
    pub fri_exp_id: u64,
    pub n_exps: u64,
    pub map_deg: PolsSections,
    pub map_offsets: PolsSections,
    pub map_sections: PolsSectionsVector,
    pub map_sections_n: PolsSections,
    pub map_sections_n1: PolsSections,
    pub map_sections_n3: PolsSections,
    pub var_pol_map: Vec<VarPolMap>,
    pub qs: Vec<u64>,
    pub cm_n: Vec<u64>,
    pub cm_2ns: Vec<u64>,
    pub pe_ctx: Vec<PeCtx>,
    pub pu_ctx: Vec<PuCtx>,
    pub ci_ctx: Vec<CiCtx>,
    pub ev_map: Vec<EvMap>,
    pub step2_prev: Step,
    pub step3_prev: Step,
    pub step3: Step,
    pub step4_2ns: Step,
    pub step5_2ns: Step,
    pub exps_n: Vec<u64>,
    pub q_2ns_vector: Vec<u64>,
    pub cm4_n_vector: Vec<u64>,
    pub cm4_2ns_vector: Vec<u64>,
    pub tmp_exp_n: Vec<u64>,
    pub exp2pol: HashMap<String, u64>,
}

impl StarkInfo {
    //TODO!
}
