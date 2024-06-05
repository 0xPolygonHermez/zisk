use std::{cell::RefCell, collections::HashMap, rc::Rc};

use goldilocks::{AbstractField, DeserializeField, PrimeField64};

use crate::{
    memory::memory::Memory,
    register::{Register, RegisterN, Registerable, VirtualRegister, VirtualRegisterN},
    Component, RomProgram,
};

use super::{BasicProcessorConfig, BasicProcessorRegisters, BasicProcessorTrace, CallbackReturnType, RomLink};

use log::info;

struct BasicProcessorComponent<'a, T> {
    pub id: Option<usize>,
    pub component: Box<dyn Component<T, Output = Option<CallbackReturnType<T>>> + 'a>,
}

#[allow(dead_code)]
pub struct BasicProcessor<'a, T> {
    trace: BasicProcessorTrace<T>,
    n: usize,

    row: Rc<RefCell<usize>>,
    row_next: usize,
    addr: usize,
    addr_rel: usize,

    zk_pc: usize,

    rom: RomProgram,
    rom_line: usize,
    rom_const: String,
    rom_constl: String,

    registers: BasicProcessorRegisters<'a, T>,
    rom_links: HashMap<String, RomLink<T>>,
    components: HashMap<String, BasicProcessorComponent<'a, T>>,
}

impl<'a, T: AbstractField + DeserializeField + PrimeField64 + Copy + 'a> BasicProcessor<'a, T> {
    pub fn new(config: BasicProcessorConfig) -> Self {
        let n = 16;
        let mut trace = BasicProcessorTrace::<T>::new(n);
        let row: Rc<RefCell<usize>> = Rc::new(RefCell::new(0));

        let registers = Self::setup_registers(&mut trace, row.clone());

        let rom_links = Self::setup_rom_links(&mut trace);

        let components = Self::register_components();

        let rom =
            RomProgram::from_json(&config.rom_json_path).unwrap_or_else(|_| panic!("Failed to parse ROM program"));

        Self {
            trace,
            n,
            row,
            row_next: 1,
            addr: 0,
            addr_rel: 0,
            zk_pc: 0,
            rom,
            rom_line: 0,
            rom_const: "CONST".to_string(),
            rom_constl: "CONSTL".to_owned(),
            registers,
            rom_links,
            components,
        }
    }

    fn setup_registers(trace: &mut BasicProcessorTrace<T>, row: Rc<RefCell<usize>>) -> BasicProcessorRegisters<'a, T> {
        let reg_a = RegisterN::new("A", trace.A.clone(), trace.in_A.clone(), trace.set_A.clone(), "inA", "setA");
        let reg_b = RegisterN::new("B", trace.B.clone(), trace.in_B.clone(), trace.set_B.clone(), "inB", "setB");
        let reg_c = Rc::new(RefCell::new(RegisterN::new(
            "C",
            trace.C.clone(),
            trace.in_C.clone(),
            trace.set_C.clone(),
            "inC",
            "setC",
        )));
        let reg_d = RegisterN::new("D", trace.D.clone(), trace.in_D.clone(), trace.set_D.clone(), "inD", "setD");
        let reg_e = RegisterN::new("E", trace.E.clone(), trace.in_E.clone(), trace.set_E.clone(), "inE", "setE");
        let reg_sr = RegisterN::new("SR", trace.SR.clone(), trace.in_SR.clone(), trace.set_SR.clone(), "inSR", "setSR");
        let reg_free =
            Rc::new(RefCell::new(RegisterN::new_ro("FREE", trace.FREE.clone(), trace.in_FREE.clone(), "inFREE")));
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

    fn register_components() -> HashMap<String, BasicProcessorComponent<'a, T>> {
        let mut components = HashMap::new();

        let component = Memory::<'a, T>::build();
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
        let rom_line = self.rom.get_line(self.rom_line).unwrap_or_else(|| panic!("Failed to get ROM line"));
        let program_line = &rom_line.program_line;

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

    fn execute(&mut self) {
        // execute(input) {
        self.init_registers();
        // this.initComponents();

        for step in 0..self.n {
            self.set_step(step);
            self.set_rom_line_and_zk_pc();

            // selectors, component, mapping (lookup/multiset)
            //     this.evalPreCommands();
            //     this.calculateFreeInput();
            //     this.opValue = this.addInValues(this.getConstValue());
            //     // console.log({opValue: this.opValue});
            //     this.calculateRelativeAddress();
            //     this.updateRomToMainLinkedCols();
            //     this.verifyComponents();
            //     this.manageFlowControl();
            //     this.applySetValues();
            //     // this.registers.dump();
            //     this.evalPostCommands();
            // }
            // this.finishComponents();
        }
    }

    fn init_registers(&mut self) {
        // this.opValue = Context.zeroValue;
        // TODO! initalize publics

        self.registers.reg_a.reset(0);
        self.registers.reg_b.reset(0);
        self.registers.reg_c.borrow_mut().reset(0);
        self.registers.reg_d.reset(0);
        self.registers.reg_e.reset(0);
        self.registers.reg_sr.reset(0);
        self.registers.reg_free.borrow_mut().reset(0);
        self.registers.reg_sp.reset(0);
        self.registers.reg_pc.reset(0);
        self.registers.reg_rr.reset(0);
        self.registers.reg_ctx.reset(0);
        self.registers.reg_rcx.reset(0);
        self.registers.reg_step.reset(0);
        self.registers.reg_free0.reset(0);
        self.registers.reg_rotl_c.reset(0);

        self.zk_pc = 0;
    }

    fn set_step(&mut self, step: usize) {
        *self.row.borrow_mut() = step;
        self.row_next = (step + 1) % self.n;
        // this.context.row = this.row;
        // this.context.step = step;
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

    fn calculate_free_input(&mut self) {
        // let fi = 0usize;

        // let rom_line = self.rom.get_line(self.rom_line).unwrap_or_else(|| panic!("Failed to get ROM line"));
        // let program_line = &rom_line.program_line;

        // if program_line.contains_key("inFREE") || program_line.contains_key("inFREE0") {
        //     if !program_line.contains_key("freeInTag") {
        //         panic!("Instruction with freeIn without freeInTag"); //TODO! Add COntext srcRef
        //     }

        //     let free_in_tag = program_line.get("freeInTag").unwrap();

        //     if free_in_tag.contains_key("op") {
        //         // fi = this.command.evalCommand(freeInTag);
        //     } else {
        //         let mut n_hits = 0;

        //         for (rom_flag, component_info) in &self.components {
        //             if !program_line.contains_key(rom_flag) {
        //                 continue;
        //             }

        //             let res = component_info.component.apply(false, component_info.id, component_info.helper);
        //             if res == false {
        //                 continue;
        //             }

        //             // fi = res;
        //             n_hits += 1;
        //         }

        //         if n_hits == 0 {
        //             panic!("Empty freeIn without a valid instruction"); //TODO! Add COntext srcRef
        //         } else if n_hits > 1 {
        //             panic!("Only one instruction that requires freeIn is allowed"); //TODO! Add COntext srcRef
        //         }
        //     }
        // }

        // if (this.romline.inFREE || this.romline.inFREE0) {
        //     if (!this.romline.freeInTag) {
        //         throw new Error(`Instruction with freeIn without freeInTag ${Context.sourceRef}`);
        //     }

        //     const freeInTag = this.romline.freeInTag;
        //     if (freeInTag.op !== '') {
        //         fi = this.command.evalCommand(freeInTag);
        //     } else {
        //         let nHits = 0;
        //         for (const romFlag in this.components) {
        //             if (!this.romline[romFlag]) continue;
        //             const componentInfo = this.components[romFlag];
        //             const res = componentInfo.method.apply(this, [false, componentInfo.id,  componentInfo.helper]);
        //             if (res === false) continue;
        //             fi = res;
        //             ++nHits;
        //         }
        //         if (nHits==0) {
        //            throw new Error(`Empty freeIn without a valid instruction ${Context.sourceRef}`);
        //         } else if (nHits>1) {
        //            throw new Error(`Only one instruction that requires freeIn is allowed ${Context.sourceRef}`);
        //         }
        //     }
        // }
        // if (!Array.isArray(fi)) {
        //     fi = this.scalarToFea(fi);
        // }
        // this.registers.setValue('FREE', fi, this.row);
    }
}
