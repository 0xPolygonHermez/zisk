use std::{cell::RefCell, collections::HashMap, sync::Arc};

use goldilocks::{AbstractField, DeserializeField, PrimeField64};
use proofman::trace::trace_pol::TracePol;

use crate::{
    memory::memory::Memory,
    register::{Register, RegisterN, Registerable, VirtualRegister, VirtualRegisterN},
    Component, RomProgram, RomProgramLine,
};

use super::basic_processor_trace::BasicProcessorTrace;

const CHUNKS: usize = 8;
const CHUNK_BITS: usize = 32;
const CHUNK_MASK: usize = (1 << CHUNK_BITS) - 1;

pub struct RomLink<T> {
    col: Arc<RefCell<TracePol<T>>>,
    binary: bool,
}

impl<T> RomLink<T> {
    pub fn new(col: Arc<RefCell<TracePol<T>>>, binary: bool) -> Self {
        RomLink { col, binary }
    }
}

pub struct BasicProcesssorComponent<'a> {
    id: Option<usize>,
    component: Box<dyn Component + 'a>,
}

pub struct BasicProcessorConfig {
    pub rom_json_path: String,
}

pub struct BasicProcessorRegisters<'a, T> {
    reg_a: RegisterN<T, CHUNKS>,
    reg_b: RegisterN<T, CHUNKS>,
    reg_c: Arc<RegisterN<T, CHUNKS>>,
    reg_d: RegisterN<T, CHUNKS>,
    reg_e: RegisterN<T, CHUNKS>,
    reg_sr: RegisterN<T, CHUNKS>,
    reg_free: Arc<RegisterN<T, CHUNKS>>,
    reg_sp: Register<T>,
    reg_pc: Register<T>,
    reg_rr: Register<T>,
    reg_ctx: Register<T>,
    reg_rcx: Register<T>,
    reg_step: VirtualRegister<'a, T>,
    reg_free0: VirtualRegister<'a, T>,
    reg_rotl_c: VirtualRegisterN<'a, T, CHUNKS>,
}

#[allow(dead_code)]
pub struct BasicProcessor<'a, T> {
    // TODO! config: BasicProcessorConfig,
    trace: BasicProcessorTrace<T>,
    // TODO! N: BasicProcessorConfig,
    // TODO! frFirst32BitsNegative: T,
    // TODO! frLast32BitsPositive: T,
    row: Arc<RefCell<usize>>,
    addr: usize,
    addr_rel: usize,

    rom: RomProgram,
    rom_line: Option<RomProgramLine>,
    rom_const: String,
    rom_constl: String,

    // TODO! context: BasicProcessorContext,
    registers: BasicProcessorRegisters<'a, T>,
    rom_links: HashMap<String, RomLink<T>>,
    components: HashMap<String, BasicProcesssorComponent<'a>>,
    // TODO! helpers: BasicProcessorHelpers,

    // TODO! command: BasicProcessorCommand,
}

impl<'a, T: AbstractField + DeserializeField + PrimeField64 + Copy + 'a> BasicProcessor<'a, T> {
    pub fn new(config: BasicProcessorConfig) -> Self {
        let mut trace = BasicProcessorTrace::<T>::new(16);
        let row = Arc::new(RefCell::new(0));

        let registers = Self::setup_registers(&mut trace, row.clone());

        let rom_links = Self::setup_rom_links(&mut trace);

        let components = Self::setup_components();

        let rom =
            RomProgram::from_json(&config.rom_json_path).unwrap_or_else(|_| panic!("Failed to parse ROM program"));

        Self {
            trace,
            row,
            addr: 0,
            addr_rel: 0,
            rom,
            rom_line: None,
            rom_const: "CONST".to_string(),
            rom_constl: "CONSTL".to_owned(),
            registers,
            rom_links,
            components,
        }
    }

    fn setup_registers(trace: &mut BasicProcessorTrace<T>, row: Arc<RefCell<usize>>) -> BasicProcessorRegisters<'a, T> {
        let reg_a = RegisterN::new("A", trace.A.clone(), trace.in_A.clone(), trace.set_A.clone(), "inA", "setA");
        let reg_b = RegisterN::new("B", trace.B.clone(), trace.in_B.clone(), trace.set_B.clone(), "inB", "setB");
        let reg_c =
            Arc::new(RegisterN::new("C", trace.C.clone(), trace.in_C.clone(), trace.set_C.clone(), "inC", "setC"));
        let reg_d = RegisterN::new("D", trace.D.clone(), trace.in_D.clone(), trace.set_D.clone(), "inD", "setD");
        let reg_e = RegisterN::new("E", trace.E.clone(), trace.in_E.clone(), trace.set_E.clone(), "inE", "setE");
        let reg_sr = RegisterN::new("SR", trace.SR.clone(), trace.in_SR.clone(), trace.set_SR.clone(), "inSR", "setSR");
        let reg_free = Arc::new(RegisterN::new_ro("FREE", trace.FREE.clone(), trace.in_FREE.clone(), "inFREE"));
        let reg_sp = Register::new("SP", trace.SP.clone(), trace.in_SP.clone(), trace.set_SP.clone(), "inSP", "setSP");
        let reg_pc = Register::new("PC", trace.PC.clone(), trace.in_PC.clone(), trace.set_PC.clone(), "inPC", "setPC");
        let reg_rr = Register::new("RR", trace.RR.clone(), trace.in_RR.clone(), trace.set_RR.clone(), "inRR", "setRR");
        let reg_ctx =
            Register::new("CTX", trace.CTX.clone(), trace.in_CTX.clone(), trace.set_CTX.clone(), "inCTX", "setCTX");
        let reg_rcx =
            Register::new("RCX", trace.RCX.clone(), trace.in_RCX.clone(), trace.set_RCX.clone(), "inRCX", "setRCX");

        let step_closure = {
            let row = row.clone();
            Box::new(move || T::from_canonical_usize(*row.borrow()))
        };
        let reg_step = VirtualRegister::new("STEP", step_closure, trace.in_STEP.clone(), "inSTEP");

        let free0_closure = {
            let reg_free_clone = reg_free.clone();
            Box::new(move || reg_free_clone.get_value()[0])
        };
        let reg_free0 = VirtualRegister::new("FREE0", free0_closure, trace.in_FREE0.clone(), "inFREE0");

        let rotl_c_closure = {
            let reg_c_clone = reg_c.clone();
            Box::new(move || reg_c_clone.rotate_left())
        };
        let reg_rotl_c = VirtualRegisterN::new("ROTL_C", rotl_c_closure, trace.in_ROTL_C.clone(), "inROTL_C");

        BasicProcessorRegisters {
            reg_a,
            reg_b,
            reg_c,
            reg_d,
            reg_e,
            reg_sr,
            reg_free,
            reg_sp,
            reg_pc,
            reg_rr,
            reg_ctx,
            reg_rcx,
            reg_step,
            reg_free0,
            reg_rotl_c,
        }
    }

    fn setup_rom_links(trace: &mut BasicProcessorTrace<T>) -> HashMap<String, RomLink<T>> {
        let mut rom_links = HashMap::new();

        rom_links.insert("isStack".to_string(), RomLink::new(trace.is_stack.clone(), true));
        rom_links.insert("isMem".to_string(), RomLink::new(trace.is_mem.clone(), true));
        rom_links.insert("mOp".to_string(), RomLink::new(trace.m_op.clone(), true));
        rom_links.insert("mWR".to_string(), RomLink::new(trace.m_wr.clone(), true));
        rom_links.insert("memUseAddrRel".to_string(), RomLink::new(trace.mem_use_addr_rel.clone(), true));
        rom_links.insert("useCTX".to_string(), RomLink::new(trace.use_ctx.clone(), true));

        rom_links.insert("incStack".to_string(), RomLink::new(trace.inc_stack.clone(), false));
        rom_links.insert("ind".to_string(), RomLink::new(trace.ind.clone(), false));
        rom_links.insert("indRR".to_string(), RomLink::new(trace.ind_rr.clone(), false));
        rom_links.insert("offset".to_string(), RomLink::new(trace.offset.clone(), false));

        rom_links.insert("doAssert".to_string(), RomLink::new(trace.do_assert.clone(), true));
        rom_links.insert("assumeFREE".to_string(), RomLink::new(trace.assume_free.clone(), true));

        rom_links.insert("JMP".to_string(), RomLink::new(trace.jmp.clone(), true));
        rom_links.insert("JMPN".to_string(), RomLink::new(trace.jmpn.clone(), true));
        rom_links.insert("JMPZ".to_string(), RomLink::new(trace.jmpz.clone(), true));
        rom_links.insert("call".to_string(), RomLink::new(trace.call.clone(), true));
        rom_links.insert("return".to_string(), RomLink::new(trace.return_jmp.clone(), true));

        rom_links.insert("jmpUseAddrRel".to_string(), RomLink::new(trace.jmp_use_addr_rel.clone(), true));
        rom_links.insert("elseUseAddrRel".to_string(), RomLink::new(trace.else_use_addr_rel.clone(), true));
        rom_links.insert("repeat".to_string(), RomLink::new(trace.repeat.clone(), true));

        rom_links.insert("condConst".to_string(), RomLink::new(trace.cond_const.clone(), false));
        rom_links.insert("jmpAddr".to_string(), RomLink::new(trace.jmp_addr.clone(), false));
        rom_links.insert("elseAddr".to_string(), RomLink::new(trace.else_addr.clone(), false));

        rom_links
    }

    fn setup_components() -> HashMap<String, BasicProcesssorComponent<'a>> {
        let mut components = HashMap::new();

        components.insert(
            "mOp".to_string(),
            BasicProcesssorComponent { id: None, component: Box::new(Memory::<'a, T>::build()) },
        );

        components
    }

    fn calculate_relative_address(&mut self) {
        self.addr_rel = 0;

        let program_line = &self.rom_line.as_ref().unwrap().program_line;

        if program_line.contains_key("ind") {
            self.addr_rel += self.registers.reg_e.get_value()[0].as_canonical_u64() as usize;
        }

        if program_line.contains_key("indRR") {
            self.addr_rel += self.registers.reg_rr.get_value().as_canonical_u64() as usize;
        }

        let max_ind = program_line.get("maxInd");

        if max_ind.is_none() {
            return;
        }

        let max_ind = max_ind.unwrap().as_u64().unwrap_or_else(|| panic!("Failed to parse maxInd"));

        if self.addr_rel > max_ind as usize {
            let offset = program_line.get("offset").unwrap().as_u64().unwrap_or(0) as usize;
            let base_label = program_line.get("baseLabel").unwrap().as_u64().unwrap_or(0) as usize;
            let index = offset - base_label + self.addr_rel;

            panic!(
                "Address out of bounds accessing index {} but {}[{}] ind:{}",
                index,
                program_line.get("offsetLabel").unwrap(),
                program_line.get("sizeLabel").unwrap(),
                self.addr_rel
            );
        }
    }

    fn calculate_memory_address(&mut self) {
        let program_line = &self.rom_line.as_ref().unwrap().program_line;

        self.addr = program_line.get("offset").unwrap().as_u64().unwrap_or(0) as usize;

        if program_line.contains_key("useCTX") {
            self.addr += self.registers.reg_ctx.get_value().as_canonical_u64() as usize * 0x40000;
        }

        if program_line.contains_key("isStack") {
            self.addr += 0x10000;
        }

        if program_line.contains_key("isMem") {
            self.addr += 0x20000;
        }

        if program_line.contains_key("memUseAddrRel") {
            self.addr += self.addr_rel;
        }
    }
}
