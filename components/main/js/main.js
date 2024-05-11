const Register = require('./register.js');

/** needs a pil information about registers ==> hints can be **/

module.exports = class Processor 
{
    constructor(config = {}) {
        const longReg = {chunks: 8};
        const shortReg = {single: true};
        const shortReadOnlyReg = {single: true, set: false};
        // in in0 ...
        this.setupRegs({
            A: longReg,
            B: longReg,
            C: longReg,
            D: longReg,
            E: longReg,
            SR: longReg,
            CTX: shortReg,
            SP: shortReg,
            PC: shortReg,
            zkPC: {...shortReg, pc: true},
            RR: shortReg,
            RCX: shortReg,
            GAS: shortReg,
            HASPOS: shortReg,
            cntArith: shortReadOnlyReg,
            cntBinary: shortReadOnlyReg,
            cntKeccakF: shortReadOnlyReg,
            cntSha256F: shortReadOnlyReg,
            cntMemAlign: shortReadOnlyReg,
            cntPoseidonG: shortReadOnlyReg,
            cntPaddingPG: shortReadOnlyReg,
            op: {...longReg, set: false, get: false, op: true},
            // multiple FREE registers ?
            FREE: {...longReg, set: false, get: false, free: true},
            ROTL_C: {...longReg, virtual: true, set: false, get: (index) => this.regs.C[(index + 1) % 7]},
        });
        

        assert(this.pc instanceof Register, "Register used as pc must be defined (config.pc = true)");
    }
    setupRegs(config, incremental = false) {
        if (!incremental) {
            this.regs = {};
        }
        for (const name in config) {
            const reg = new Register(name, config[name]);
            this.regs[name];
            if (reg.pc) {
                this.pcReg = reg;
            }
        }
    }
    calculateRelativeAddress() {
        this.addrRel = 0;        
        if (this.romline.ind) {
            this.addrRel += fe2n(Fr, ctx.E[0], ctx);
        }
        if (this.romline.indRR) {
            this.addrRel += fe2n(Fr, ctx.RR, ctx);
        }
        if (typeof this.romline.maxInd !== 'undefined' && this.addrRel > this.romline.maxInd) {
            const index = this.romline.offset - this.romline.baseLabel + this.addrRel;
            throw new Error(`Address out of bounds accessing index ${index} but ${this.romline.offsetLabel}[${this.romline.sizeLabel}] ind:${this.addrRel}`);
        }   
    }
    initRegs() {
        for (const reg in this.regs) {
            reg.init();
        }
    }
    inRegs(op) {
        for (const reg in this.regs) {
            reg.in(op);
        }
    }
    execute(pols, input, rom) {
        const stepN = 100;
        this.initRegs();
        for (let step = 0; step < stepsN; step++) {
            this.romline = rom[this.pc];
            // get rom line (PC)
            this.resetOp();
            this.inRegs(); // add inRegs to opReg
            this.inConst(); // add CONST to opReg
            // init op with CONST;        
            this.calculateRelativeAddress(l);
            
            // selectors, component, mapping (lookup/multiset)

            if (this.romline.inFREE || this.romline.inFREE0) {
                this.calculateFreeInputFromComponents();
            }
            this.verifyComponents();
            this.setRegs();
            this.setCounters(); // REVIEW: asReg?, how link to components, pil is the key
        }
        this.componentsEnd();
    }
    calculateFreeInputFromComponents() {
        if (!this.romline.freeInTag) {
            throw new Error(`Instruction with freeIn without freeInTag ${sourceRef}`);
        }

        let fi;
        if (this.freeInTag.op=="") {
            let nHits = 0;
            this.calculateFreeInputFromComponents();

            if (nHits==0) {
                throw new Error(`Empty freeIn without a valid instruction ${sourceRef}`);
            }
            // if (nHits>1) {
            //     throw new Error(`Only one instruction that requires freeIn is allowed ${sourceRef}`);
            // }
        } else {
            fi = evalCommand(ctx, l.freeInTag);
            if (!Array.isArray(fi)) fi = scalar2fea(Fr, fi);
        }
    }

    // required filled when verifys
}
