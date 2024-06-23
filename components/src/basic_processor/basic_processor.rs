use std::{cell::RefCell, collections::HashMap, rc::Rc};

use goldilocks::{AbstractField, DeserializeField, PrimeField64};

use crate::{
    memory::memory::Memory,
    register::{Register, RegisterN, RegisterN2, Registerable, VirtualRegister, VirtualRegisterN},
    Component, Context, RomProgram,
};

use super::{
    BasicProcessorConfig, BasicProcessorRegisters, BasicProcessorTrace, BasicProcessorTrace2, RegistersEnum, TracePolEnum, RomLink, CHUNKS,
};

use log::info;

struct BasicProcessorComponent<'a, T> {
    pub id: Option<usize>,
    pub component: Box<dyn Component<T, Output = Option<RegistersEnum<T>>> + 'a>,
}

#[allow(dead_code)]
pub struct BasicProcessor<'a, T> {
    // Configuration for the basic processor
    config: BasicProcessorConfig,
    // Trace for the basic processor
    trace: &'a BasicProcessorTrace<T>,
    trace2: &'a BasicProcessorTrace2<T>,
    // Number of rows in the trace
    n: Rc<RefCell<usize>>,

    // Current row
    row: Rc<RefCell<usize>>,
    // Next row
    row_next: usize,
    // Current row
    step: Rc<RefCell<usize>>,
    // Address
    addr: usize,
    // Relative address
    addr_rel: usize,

    zk_pc: usize,

    rom: RomProgram,
    rom_line: usize,
    rom_const: String,
    rom_constl: String,

    op_value: RegistersEnum<T>,

    context: Rc<RefCell<Context<'a, T>>>,
    registers: Rc<RefCell<BasicProcessorRegisters<'a, T>>>,
    rom_links: HashMap<String, RomLink<T>>,
    components: HashMap<String, BasicProcessorComponent<'a, T>>,
}

impl<'a, T: AbstractField + DeserializeField + PrimeField64 + Copy + 'a> BasicProcessor<'a, T> {
    pub fn new(config: BasicProcessorConfig, trace: &'a mut BasicProcessorTrace<T>, trace2: &'a mut BasicProcessorTrace2<T>) -> Self {
        let n = Rc::new(RefCell::new(trace.zk_pc.borrow().num_rows()));
        let row = Rc::new(RefCell::new(0));
        let step = Rc::new(RefCell::new(0));

        let registers = Self::setup_registers(trace, row.clone());

        let reg_xxx = RegisterN2::new("A", trace2.A, trace.in_A, trace.set_A, "inA", "setA");


        let registers = Rc::new(RefCell::new(registers));

        let context = Context::new(n.clone(), row.clone(), step.clone(), registers.clone());
        let context = Rc::new(RefCell::new(context));

        // self.setup_command();

        let rom_links = Self::setup_rom_links(trace);

        let components = Self::register_components();

        // self.register_helpers();

        let rom =
            RomProgram::from_json(&config.rom_json_path).unwrap_or_else(|_| panic!("Failed to parse ROM program"));

        Self {
            config,
            trace,
            trace2,
            n,
            row,
            row_next: 1,
            step,
            addr: 0,
            addr_rel: 0,
            zk_pc: 0,
            rom,
            rom_line: 0,
            rom_const: "CONST".to_string(),
            rom_constl: "CONSTL".to_owned(),
            op_value: RegistersEnum::Array([T::zero(); CHUNKS]),
            context,
            registers,
            rom_links,
            components,
        }
    }

    fn setup_registers(trace: &mut BasicProcessorTrace<T>, row: Rc<RefCell<usize>>) -> BasicProcessorRegisters<'a, T> {
        let reg_a = RegisterN::new("A", trace.A.clone(), trace.in_A.clone(), trace.set_A.clone(), "inA", "setA");
        let reg_b = RegisterN::new("B", trace.B.clone(), trace.in_B.clone(), trace.set_B.clone(), "inB", "setB");
        let reg_c = RegisterN::new("C", trace.C.clone(), trace.in_C.clone(), trace.set_C.clone(), "inC", "setC");
        let reg_c = Rc::new(RefCell::new(reg_c));
        let reg_d = RegisterN::new("D", trace.D.clone(), trace.in_D.clone(), trace.set_D.clone(), "inD", "setD");
        let reg_e = RegisterN::new("E", trace.E.clone(), trace.in_E.clone(), trace.set_E.clone(), "inE", "setE");
        let reg_sr = RegisterN::new("SR", trace.SR.clone(), trace.in_SR.clone(), trace.set_SR.clone(), "inSR", "setSR");
        let reg_free = RegisterN::new_ro("FREE", trace.FREE.clone(), trace.in_FREE.clone(), "inFREE");
        let reg_free = Rc::new(RefCell::new(reg_free));
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
            Box::new(move || reg_free_clone.borrow().get_value()[0])
        };
        let reg_free0 = VirtualRegister::new("FREE0", free0_closure, trace.in_FREE0.clone(), "inFREE0");

        let rotl_c_closure = {
            let reg_c_clone = reg_c.clone();
            Box::new(move || reg_c_clone.borrow_mut().rotate_left())
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

    fn setup_fr(&self) {
        unimplemented!();
    }

    fn setup_command(&self) {
        unimplemented!();
    }

    fn setup_rom_links(trace: &mut BasicProcessorTrace<T>) -> HashMap<String, RomLink<T>> {
        let mut rom_links: HashMap<String, RomLink<T>> = HashMap::new();

        rom_links.insert("isStack".to_string(), RomLink::new(TracePolEnum::Single(trace.is_stack.clone()), true));
        rom_links.insert("isMem".to_string(), RomLink::new(TracePolEnum::Single(trace.is_mem.clone()), true));
        rom_links.insert("mOp".to_string(), RomLink::new(TracePolEnum::Single(trace.m_op.clone()), true));
        rom_links.insert("mWR".to_string(), RomLink::new(TracePolEnum::Single(trace.m_wr.clone()), true));
        rom_links.insert(
            "memUseAddrRel".to_string(),
            RomLink::new(TracePolEnum::Single(trace.mem_use_addr_rel.clone()), true),
        );
        rom_links.insert("useCTX".to_string(), RomLink::new(TracePolEnum::Single(trace.use_ctx.clone()), true));

        rom_links.insert("incStack".to_string(), RomLink::new(TracePolEnum::Single(trace.inc_stack.clone()), false));
        rom_links.insert("ind".to_string(), RomLink::new(TracePolEnum::Single(trace.ind.clone()), false));
        rom_links.insert("indRR".to_string(), RomLink::new(TracePolEnum::Single(trace.ind_rr.clone()), false));
        rom_links.insert("offset".to_string(), RomLink::new(TracePolEnum::Single(trace.offset.clone()), false));

        rom_links.insert("doAssert".to_string(), RomLink::new(TracePolEnum::Single(trace.do_assert.clone()), true));
        rom_links.insert("assumeFREE".to_string(), RomLink::new(TracePolEnum::Single(trace.assume_free.clone()), true));

        rom_links.insert("JMP".to_string(), RomLink::new(TracePolEnum::Single(trace.jmp.clone()), true));
        rom_links.insert("JMPN".to_string(), RomLink::new(TracePolEnum::Single(trace.jmpn.clone()), true));
        rom_links.insert("JMPZ".to_string(), RomLink::new(TracePolEnum::Single(trace.jmpz.clone()), true));
        rom_links.insert("call".to_string(), RomLink::new(TracePolEnum::Single(trace.call.clone()), true));
        rom_links.insert("return".to_string(), RomLink::new(TracePolEnum::Single(trace.return_jmp.clone()), true));

        rom_links.insert(
            "jmpUseAddrRel".to_string(),
            RomLink::new(TracePolEnum::Single(trace.jmp_use_addr_rel.clone()), true),
        );
        rom_links.insert(
            "elseUseAddrRel".to_string(),
            RomLink::new(TracePolEnum::Single(trace.else_use_addr_rel.clone()), true),
        );
        rom_links.insert("repeat".to_string(), RomLink::new(TracePolEnum::Single(trace.repeat.clone()), true));

        rom_links.insert("condConst".to_string(), RomLink::new(TracePolEnum::Single(trace.cond_const.clone()), false));
        rom_links.insert("jmpAddr".to_string(), RomLink::new(TracePolEnum::Single(trace.jmp_addr.clone()), false));
        rom_links.insert("elseAddr".to_string(), RomLink::new(TracePolEnum::Single(trace.else_addr.clone()), false));

        rom_links
    }

    fn register_components() -> HashMap<String, BasicProcessorComponent<'a, T>> {
        let mut components = HashMap::new();

        components.insert(
            "mOp".to_string(),
            BasicProcessorComponent { id: None, component: Box::new(Memory::<'a, T>::build()) },
        );

        components
    }

    fn calculate_relative_address(&mut self) {
        self.addr_rel = 0;

        let rom_line = self.rom.get_line(self.rom_line).unwrap_or_else(|| panic!("Failed to get ROM line"));
        let program_line = &rom_line.program_line;

        if program_line.contains_key("ind") {
            self.addr_rel += self.registers.borrow().reg_e.get_value()[0].as_canonical_u64() as usize;
        }

        if program_line.contains_key("indRR") {
            self.addr_rel += self.registers.borrow().reg_rr.get_value().as_canonical_u64() as usize;
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

    /*         Memory Region                    Size   isMem  isStack   Content
       ctxBase + [0x000000 - 0x00FFFF]   2MiB      0        0   Context specific variables
       ctxBase + [0x010000 - 0x000000]   2MiB      0        1   EVM Stack
       ctxBase + [0x020000 - 0x03FFFF]   4MiB      1        0   EVM Memory
    */
    fn calculate_memory_address(&mut self) {
        const CTX_SPECIFIC: usize = 0x00000;
        const EVM_STACK: usize = 0x10000;
        const EVM_MEMORY: usize = 0x20000;
        const CTX_MEMORY: usize = 0x40000;

        let rom_line = self.rom.get_line(self.rom_line).unwrap_or_else(|| panic!("Failed to get ROM line"));
        let program_line = &rom_line.program_line;

        self.addr = program_line.get("offset").unwrap().as_u64().unwrap_or(0) as usize;

        if program_line.contains_key("useCTX") {
            self.addr += self.registers.borrow().reg_ctx.get_value().as_canonical_u64() as usize * CTX_MEMORY;
        }

        if program_line.contains_key("isStack") {
            self.addr += EVM_STACK;
        }

        if program_line.contains_key("isMem") {
            self.addr += EVM_MEMORY;
        }

        if program_line.contains_key("memUseAddrRel") {
            self.addr += self.addr_rel;
        }
    }

    pub fn execute(&mut self) {
        self.init_registers();
        self.init_components();

        let n = *self.n.borrow();
        for step in 0..n {
            self.set_step(step);
            self.set_rom_line_and_zk_pc();

            // selectors, component, mapping (lookup/multiset)

            self.eval_pre_commands();
            self.calculate_free_input();
            //     this.opValue = this.addInValues(this.getConstValue());
            self.calculate_relative_address();
            self.update_rom_to_main_linked_cols();
            self.verify_components();
            self.manage_flow_control();
            self.apply_set_values();

            // this.registers.dump();
            self.eval_post_commands();
        }

        self.finish_components();
    }

    fn init_registers(&mut self) {
        self.op_value = RegistersEnum::Array([T::zero(); CHUNKS]);
        // TODO! initalize publics

        self.registers.borrow_mut().reg_a.reset(0);
        self.registers.borrow_mut().reg_b.reset(0);
        self.registers.borrow_mut().reg_c.borrow_mut().reset(0);
        self.registers.borrow_mut().reg_d.reset(0);
        self.registers.borrow_mut().reg_e.reset(0);
        self.registers.borrow_mut().reg_sr.reset(0);
        self.registers.borrow_mut().reg_free.borrow_mut().reset(0);
        self.registers.borrow_mut().reg_sp.reset(0);
        self.registers.borrow_mut().reg_pc.reset(0);
        self.registers.borrow_mut().reg_rr.reset(0);
        self.registers.borrow_mut().reg_ctx.reset(0);
        self.registers.borrow_mut().reg_rcx.reset(0);
        self.registers.borrow_mut().reg_step.reset(0);
        self.registers.borrow_mut().reg_free0.reset(0);
        self.registers.borrow_mut().reg_rotl_c.reset(0);

        self.zk_pc = 0;
    }

    fn init_components(&mut self) {
        for (_, component_info) in &mut self.components {
            component_info.component.init();
        }
    }

    fn finish_components(&mut self) {
        for (_, component_info) in &mut self.components {
            component_info.component.finish();
        }
    }

    fn verify_components(&mut self) {
        for (rom_flag, component_info) in &self.components {
            let rom_line = self.rom.get_line(self.rom_line).unwrap_or_else(|| panic!("Failed to get ROM line"));
            let program_line = &rom_line.program_line;

            if !program_line.contains_key(rom_flag) {
                continue;
            }

            unimplemented!();
        }
    }

    fn set_step(&mut self, step: usize) {
        *self.row.borrow_mut() = step;
        self.row_next = (step + 1) % *self.n.borrow();
        *self.context.borrow().row.borrow_mut() = step;
        *self.context.borrow().step.borrow_mut() = step;
    }

    fn set_rom_line_and_zk_pc(&mut self) {
        let row = *self.row.borrow();
        self.trace.zk_pc.borrow_mut()[row] = T::from_canonical_u64(self.zk_pc as u64);

        self.rom_line = self.zk_pc;
        assert!(self.rom_line < self.rom.program_lines.len());

        // this.context.sourceRef = `${this.romline.fileName}:${this.romline.line} (zkPC:${this.zkPC} row:${this.row})`;

        let rom_line = self.rom.get_line(self.rom_line).unwrap_or_else(|| panic!("Failed to get ROM line"));

        info!("#{:0>8} ROM{} {:0>6}", self.row.borrow().to_string(), self.zk_pc.to_string(), rom_line.line_str);
    }

    fn eval_pre_commands(&mut self) {
        unimplemented!();
    }

    fn eval_post_commands(&mut self) {
        unimplemented!();
    }

    fn calculate_free_input(&mut self) {
        let mut free_input = RegistersEnum::Single(T::default());

        let rom_line = self.rom.get_line(self.rom_line).unwrap_or_else(|| panic!("Failed to get ROM line"));
        let program_line = &rom_line.program_line;

        if program_line.contains_key("inFREE") || program_line.contains_key("inFREE0") {
            if !program_line.contains_key("freeInTag") {
                panic!("Instruction with freeIn without freeInTag"); //TODO! Add Context srcRef
            }

            let free_in_tag = program_line.get("freeInTag").unwrap();

            let op = free_in_tag.get("op");

            if op.is_some() {
                // fi = self.eval_command(free_in_tag);
            } else {
                let mut n_hits: isize = 0;

                for (rom_flag, component_info) in &self.components {
                    if !program_line.contains_key(rom_flag) {
                        continue;
                    }

                    let res = component_info.component.calculate_free_input(vec![T::one()]);
                    if res.is_none() {
                        continue;
                    }

                    free_input = res.unwrap();
                    n_hits += 1;
                }

                if n_hits == 0 {
                    panic!("Empty freeIn without a valid instruction"); //TODO! Add COntext srcRef
                } else if n_hits > 1 {
                    panic!("Only one instruction that requires freeIn is allowed");
                    //TODO! Add COntext srcRef
                }
            }
        }

        let free_input: [T; CHUNKS] = match free_input {
            RegistersEnum::Single(_) => [T::one(); 8],
            RegistersEnum::Array(arr) => arr,
        };

        self.registers.borrow().reg_free.borrow_mut().update_value(*self.row.borrow(), free_input);
    }

    fn update_rom_to_main_linked_cols(&mut self) {
        let rom_line = self.rom.get_line(self.rom_line).unwrap_or_else(|| panic!("Failed to get ROM line"));
        let program_line = &rom_line.program_line;

        for (rom_flag, rom_link) in &self.rom_links {
            match &rom_link.col {
                TracePolEnum::Single(col) => {
                    let mut col = col.borrow_mut();

                    // single binary links
                    if rom_link.binary {
                        col[*self.row.borrow()] =
                            if program_line.contains_key(rom_flag) { T::one() } else { T::zero() };
                        continue;
                    } else {
                        let value = program_line[rom_flag].as_str().unwrap_or_else(|| panic!("Failed to parse value"));
                        col[*self.row.borrow()] = T::from_string(value, 10);
                    }
                }
                TracePolEnum::Array(col) => {
                    let mut col = col.borrow_mut();

                    // multi-chunk non-binary links
                    let array =
                        program_line[rom_flag].as_array().unwrap_or_else(|| panic!("Failed to parse array value"));
                    for index in 0..CHUNKS {
                        let value = array[index].as_str().unwrap_or_else(|| panic!("Failed to parse value"));
                        col[*self.row.borrow()][index] = T::from_string(value, 10);
                    }
                }
            };
        }
    }

    fn manage_flow_control(&mut self) {
        unimplemented!();
    }

    fn apply_set_values(&mut self) {
        unimplemented!();
    }
}
