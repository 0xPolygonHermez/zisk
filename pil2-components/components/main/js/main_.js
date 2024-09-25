

/**
 * This function creates an array of polynomials and a mapping that maps the reference name in pil to the polynomial
 * @param {Field} Fr - Field element
 * @param {Object} pols - polynomials
 * @param {Object} ctx - context
 */
function checkFinalState(Fr, pols, ctx) {

    if (
        (!Fr.isZero(pols.A0[0])) ||
        (!Fr.isZero(pols.A1[0])) ||
        (!Fr.isZero(pols.A2[0])) ||
        (!Fr.isZero(pols.A3[0])) ||
        (!Fr.isZero(pols.A4[0])) ||
        (!Fr.isZero(pols.A5[0])) ||
        (!Fr.isZero(pols.A6[0])) ||
        (!Fr.isZero(pols.A7[0])) ||
        (!Fr.isZero(pols.D0[0])) ||
        (!Fr.isZero(pols.D1[0])) ||
        (!Fr.isZero(pols.D2[0])) ||
        (!Fr.isZero(pols.D3[0])) ||
        (!Fr.isZero(pols.D4[0])) ||
        (!Fr.isZero(pols.D5[0])) ||
        (!Fr.isZero(pols.D6[0])) ||
        (!Fr.isZero(pols.D7[0])) ||
        (!Fr.isZero(pols.E0[0])) ||
        (!Fr.isZero(pols.E1[0])) ||
        (!Fr.isZero(pols.E2[0])) ||
        (!Fr.isZero(pols.E3[0])) ||
        (!Fr.isZero(pols.E4[0])) ||
        (!Fr.isZero(pols.E5[0])) ||
        (!Fr.isZero(pols.E6[0])) ||
        (!Fr.isZero(pols.E7[0])) ||
        (!Fr.isZero(pols.SR0[0])) ||
        (!Fr.isZero(pols.SR1[0])) ||
        (!Fr.isZero(pols.SR2[0])) ||
        (!Fr.isZero(pols.SR3[0])) ||
        (!Fr.isZero(pols.SR4[0])) ||
        (!Fr.isZero(pols.SR5[0])) ||
        (!Fr.isZero(pols.SR6[0])) ||
        (!Fr.isZero(pols.SR7[0])) ||
        (pols.PC[0]) ||
        (pols.HASHPOS[0]) ||
        (pols.RR[0]) ||
        (pols.RCX[0])
    ) {
        if(fullTracer) fullTracer.exportTrace();

        if(ctx.step >= (ctx.stepsN - 1)) console.log("Not enough steps to finalize execution (${ctx.step},${ctx.stepsN-1})\n");
        throw new Error("Program terminated with registers A, D, E, SR, PC, HASHPOS, RR, RCX, zkPC not set to zero");
    }

    const feaOldStateRoot = scalar2fea(ctx.Fr, Scalar.e(ctx.input.oldStateRoot));
    if (
        (!Fr.eq(pols.B0[0], feaOldStateRoot[0])) ||
        (!Fr.eq(pols.B1[0], feaOldStateRoot[1])) ||
        (!Fr.eq(pols.B2[0], feaOldStateRoot[2])) ||
        (!Fr.eq(pols.B3[0], feaOldStateRoot[3])) ||
        (!Fr.eq(pols.B4[0], feaOldStateRoot[4])) ||
        (!Fr.eq(pols.B5[0], feaOldStateRoot[5])) ||
        (!Fr.eq(pols.B6[0], feaOldStateRoot[6])) ||
        (!Fr.eq(pols.B7[0], feaOldStateRoot[7]))
    ) {
        if(fullTracer) fullTracer.exportTrace();
        throw new Error("Register B not terminetd equal as its initial value");
    }

    const feaOldAccInputHash = scalar2fea(ctx.Fr, Scalar.e(ctx.input.oldAccInputHash));
    if (
        (!Fr.eq(pols.C0[0], feaOldAccInputHash[0])) ||
        (!Fr.eq(pols.C1[0], feaOldAccInputHash[1])) ||
        (!Fr.eq(pols.C2[0], feaOldAccInputHash[2])) ||
        (!Fr.eq(pols.C3[0], feaOldAccInputHash[3])) ||
        (!Fr.eq(pols.C4[0], feaOldAccInputHash[4])) ||
        (!Fr.eq(pols.C5[0], feaOldAccInputHash[5])) ||
        (!Fr.eq(pols.C6[0], feaOldAccInputHash[6])) ||
        (!Fr.eq(pols.C7[0], feaOldAccInputHash[7]))
    ) {
        if(fullTracer) fullTracer.exportTrace();
        throw new Error("Register C not termined equal as its initial value");
    }

    if (!Fr.eq(pols.SP[0], ctx.Fr.e(ctx.input.oldNumBatch))){
        if(fullTracer) fullTracer.exportTrace();
        throw new Error("Register SP not termined equal as its initial value");
    }

    if (!Fr.eq(pols.GAS[0], ctx.Fr.e(ctx.input.chainID))){
        if(fullTracer) fullTracer.exportTrace();
        throw new Error("Register GAS not termined equal as its initial value");
    }

    if (!Fr.eq(pols.CTX[0], ctx.Fr.e(ctx.input.forkID))){
        if(fullTracer) fullTracer.exportTrace();
        throw new Error(`Register CTX not termined equal as its initial value CTX[0]:${pols.CTX[0]} forkID:${ctx.input.forkID}`);
    }
}

/**
 * get output registers and assert them against outputs provided
 * @param {Object} ctx - context
 */
function assertOutputs(ctx){
    const feaNewStateRoot = scalar2fea(ctx.Fr, Scalar.e(ctx.input.newStateRoot));

    if (
        (!ctx.Fr.eq(ctx.SR[0], feaNewStateRoot[0])) ||
        (!ctx.Fr.eq(ctx.SR[1], feaNewStateRoot[1])) ||
        (!ctx.Fr.eq(ctx.SR[2], feaNewStateRoot[2])) ||
        (!ctx.Fr.eq(ctx.SR[3], feaNewStateRoot[3])) ||
        (!ctx.Fr.eq(ctx.SR[4], feaNewStateRoot[4])) ||
        (!ctx.Fr.eq(ctx.SR[5], feaNewStateRoot[5])) ||
        (!ctx.Fr.eq(ctx.SR[6], feaNewStateRoot[6])) ||
        (!ctx.Fr.eq(ctx.SR[7], feaNewStateRoot[7]))
    ) {
        let errorMsg = "Assert Error: newStateRoot does not match\n";
        errorMsg += `   State root computed: ${fea2String(ctx.Fr, ctx.SR)}\n`;
        errorMsg += `   State root expected: ${ctx.input.newStateRoot}\n`;
        errorMsg += `Errors: ${nameRomErrors.toString()}`;
        throw new Error(errorMsg);
    }

    const feaNewAccInputHash = scalar2fea(ctx.Fr, Scalar.e(ctx.input.newAccInputHash));

    if (
        (!ctx.Fr.eq(ctx.D[0], feaNewAccInputHash[0])) ||
        (!ctx.Fr.eq(ctx.D[1], feaNewAccInputHash[1])) ||
        (!ctx.Fr.eq(ctx.D[2], feaNewAccInputHash[2])) ||
        (!ctx.Fr.eq(ctx.D[3], feaNewAccInputHash[3])) ||
        (!ctx.Fr.eq(ctx.D[4], feaNewAccInputHash[4])) ||
        (!ctx.Fr.eq(ctx.D[5], feaNewAccInputHash[5])) ||
        (!ctx.Fr.eq(ctx.D[6], feaNewAccInputHash[6])) ||
        (!ctx.Fr.eq(ctx.D[7], feaNewAccInputHash[7]))
    ) {
        let errorMsg = "Assert Error: AccInputHash does not match\n";
        errorMsg += `   AccInputHash computed: ${fea2String(ctx.Fr, ctx.D)}\n`;
        errorMsg += `   AccInputHash expected: ${ctx.input.newAccInputHash}\n`;
        errorMsg += `Errors: ${nameRomErrors.toString()}`;
        throw new Error(errorMsg);
    }

    const feaNewLocalExitRoot = scalar2fea(ctx.Fr, Scalar.e(ctx.input.newLocalExitRoot));

    if (
        (!ctx.Fr.eq(ctx.E[0], feaNewLocalExitRoot[0])) ||
        (!ctx.Fr.eq(ctx.E[1], feaNewLocalExitRoot[1])) ||
        (!ctx.Fr.eq(ctx.E[2], feaNewLocalExitRoot[2])) ||
        (!ctx.Fr.eq(ctx.E[3], feaNewLocalExitRoot[3])) ||
        (!ctx.Fr.eq(ctx.E[4], feaNewLocalExitRoot[4])) ||
        (!ctx.Fr.eq(ctx.E[5], feaNewLocalExitRoot[5])) ||
        (!ctx.Fr.eq(ctx.E[6], feaNewLocalExitRoot[6])) ||
        (!ctx.Fr.eq(ctx.E[7], feaNewLocalExitRoot[7]))
    ) {
        let errorMsg = "Assert Error: NewLocalExitRoot does not match\n";
        errorMsg += `   NewLocalExitRoot computed: ${fea2String(ctx.Fr, ctx.E)}\n`;
        errorMsg += `   NewLocalExitRoot expected: ${ctx.input.newLocalExitRoot}\n`;
        errorMsg += `Errors: ${nameRomErrors.toString()}`;
        throw new Error(errorMsg);
    }

    if (!ctx.Fr.eq(ctx.PC, ctx.Fr.e(ctx.input.newNumBatch))){
        let errorMsg = "Assert Error: NewNumBatch does not match\n";
        errorMsg += `   NewNumBatch computed: ${Number(ctx.PC)}\n`;
        errorMsg += `   NewNumBatch expected: ${ctx.input.newNumBatch}\n`;
        errorMsg += `Errors: ${nameRomErrors.toString()}`;
        throw new Error(errorMsg);
    }

    console.log("Assert outputs run succesfully");
}

function initCounterControls(counterControls, rom) {
    Object.values(counterControls).forEach(cc => {
        cc.limit = rom.constants[cc.limitConstant] ? BigInt(rom.constants[cc.limitConstant].value) : false;
        cc.reserved = false;
        cc.sourceRef = false});
}


/**
 * Set input parameters to initial registers
 * @param {Field} Fr - field element
 * @param {Object} pols - polynomials
 * @param {Object} ctx - context
 */
function initState(Fr, pols, ctx) {
    // Set oldStateRoot to register B
    [
        pols.B0[0],
        pols.B1[0],
        pols.B2[0],
        pols.B3[0],
        pols.B4[0],
        pols.B5[0],
        pols.B6[0],
        pols.B7[0]
    ] = scalar2fea(ctx.Fr, Scalar.e(ctx.input.oldStateRoot));

    // Set oldAccInputHash to register C
    [
        pols.C0[0],
        pols.C1[0],
        pols.C2[0],
        pols.C3[0],
        pols.C4[0],
        pols.C5[0],
        pols.C6[0],
        pols.C7[0]
    ] = scalar2fea(ctx.Fr, Scalar.e(ctx.input.oldAccInputHash));

    // Set oldNumBatch to SP register
    pols.SP[0] = ctx.Fr.e(ctx.input.oldNumBatch)

    // Set chainID to GAS register
    pols.GAS[0] = ctx.Fr.e(ctx.input.chainID)

    // Set forkID to CTX register
    pols.CTX[0] = ctx.Fr.e(ctx.input.forkID)

    pols.A0[0] = Fr.zero;
    pols.A1[0] = Fr.zero;
    pols.A2[0] = Fr.zero;
    pols.A3[0] = Fr.zero;
    pols.A4[0] = Fr.zero;
    pols.A5[0] = Fr.zero;
    pols.A6[0] = Fr.zero;
    pols.A7[0] = Fr.zero;
    pols.D0[0] = Fr.zero;
    pols.D1[0] = Fr.zero;
    pols.D2[0] = Fr.zero;
    pols.D3[0] = Fr.zero;
    pols.D4[0] = Fr.zero;
    pols.D5[0] = Fr.zero;
    pols.D6[0] = Fr.zero;
    pols.D7[0] = Fr.zero;
    pols.E0[0] = Fr.zero;
    pols.E1[0] = Fr.zero;
    pols.E2[0] = Fr.zero;
    pols.E3[0] = Fr.zero;
    pols.E4[0] = Fr.zero;
    pols.E5[0] = Fr.zero;
    pols.E6[0] = Fr.zero;
    pols.E7[0] = Fr.zero;
    pols.SR0[0] = Fr.zero;
    pols.SR1[0] = Fr.zero;
    pols.SR2[0] = Fr.zero;
    pols.SR3[0] = Fr.zero;
    pols.SR4[0] = Fr.zero;
    pols.SR5[0] = Fr.zero;
    pols.SR6[0] = Fr.zero;
    pols.SR7[0] = Fr.zero;
    pols.PC[0] = 0n;
    pols.HASHPOS[0] = 0n;
    pols.RR[0] = 0n;
    pols.zkPC[0] = 0n;
    pols.cntArith[0] = 0n;
    pols.cntBinary[0] = 0n;
    pols.cntKeccakF[0] = 0n;
    pols.cntSha256F[0] = 0n;
    pols.cntMemAlign[0] = 0n;
    pols.cntPaddingPG[0] = 0n;
    pols.cntPoseidonG[0] = 0n;
    pols.RCX[0] = 0n;
    pols.RCXInv[0] = 0n;
    pols.op0Inv[0] = 0n;
}

function eval_getReg(ctx, tag) {
    if (tag.regName == "A") {
        return ctx.fullFe ? fea2scalar(ctx.Fr, ctx.A) : safeFea2scalar(ctx.Fr, ctx.A);
    } else if (tag.regName == "B") {
        return ctx.fullFe ? fea2scalar(ctx.Fr, ctx.B) : safeFea2scalar(ctx.Fr, ctx.B);
    } else if (tag.regName == "C") {
        return ctx.fullFe ? fea2scalar(ctx.Fr, ctx.C) : safeFea2scalar(ctx.Fr, ctx.C);
    } else if (tag.regName == "D") {
        return ctx.fullFe ? fea2scalar(ctx.Fr, ctx.D) : safeFea2scalar(ctx.Fr, ctx.D);
    } else if (tag.regName == "E") {
        return ctx.fullFe ? fea2scalar(ctx.Fr, ctx.E) : safeFea2scalar(ctx.Fr, ctx.E);
    } else if (tag.regName == "SR") {
        return ctx.fullFe ? fea2scalar(ctx.Fr, ctx.SR) : safeFea2scalar(ctx.Fr, ctx.SR);
    } else if (tag.regName == "CTX") {
        return Scalar.e(ctx.CTX);
    } else if (tag.regName == "SP") {
        return Scalar.e(ctx.SP);
    } else if (tag.regName == "PC") {
        return Scalar.e(ctx.PC);
    } else if (tag.regName == "GAS") {
        return Scalar.e(ctx.GAS);
    } else if (tag.regName == "zkPC") {
        return Scalar.e(ctx.zkPC);
    } else if (tag.regName == "RR") {
        return Scalar.e(ctx.RR);
    } else if (tag.regName == "CNT_ARITH") {
        return Scalar.e(ctx.cntArith);
    } else if (tag.regName == "CNT_BINARY") {
        return Scalar.e(ctx.cntBinary);
    } else if (tag.regName == "CNT_KECCAK_F") {
        return Scalar.e(ctx.cntKeccakF);
    } else if (tag.regName == 'CNT_SHA256_F') {
        return Scalar.e(ctx.cntSha256F);
    } else if (tag.regName == "CNT_MEM_ALIGN") {
        return Scalar.e(ctx.cntMemAlign);
    } else if (tag.regName == "CNT_PADDING_PG") {
        return Scalar.e(ctx.cntPaddingPG);
    } else if (tag.regName == "CNT_POSEIDON_G") {
        return Scalar.e(ctx.cntPoseidonG);
    } else if (tag.regName == "STEP") {
        return Scalar.e(ctx.step);
    } else if (tag.regName == "HASHPOS") {
        return Scalar.e(ctx.HASHPOS);
    } else if (tag.regName == "RCX") {
        return Scalar.e(ctx.RCX);
    } else {
        throw new Error(`Invalid register ${tag.regName} ${ctx.sourceRef}`);
    }
}


function eval_getMemValue(ctx, tag) {
    let addr = tag.offset;

    if (tag.useCTX === 1) {
        addr += Number(ctx.CTX) * 0x40000;
    }

    if (ctx.fullFe) {
        return fea2scalar(ctx.Fr, ctx.mem[addr]);
    }

    return safeFea2scalar(ctx.Fr, ctx.mem[addr]);
}

function eval_functionCall(ctx, tag) {
    if (ctx.helpers) {
        const method = 'eval_'+ tag.funcName;
        for (const helper of ctx.helpers) {
            if (typeof helper[method] !== 'function') continue;
            const res = helper[method](ctx, tag);
            if (res !== null) {
                return res;
            }
        }
    }
/*
    } else if (tag.funcName.includes("comp") && tag.funcName.split('_')[0] === "comp") {
        return eval_comp(ctx, tag);
    } else if (tag.funcName.includes("precompiled") && tag.funcName.split('_')[0] === "precompiled") {
        return eval_precompiled(ctx, tag);
*/
    throw new Error(`function ${tag.funcName} not defined ${ctx.sourceRef}`);
}


function eval_cond(ctx, tag) {
    if (tag.params.length != 1) throw new Error(`Invalid number of parameters (1 != ${tag.params.length}) function ${tag.funcName} ${ctx.sourceRef}`);
    const result = Number(evalCommand(ctx,tag.params[0]));
    if (result) {
        return [ctx.Fr.e(-1), ctx.Fr.zero, ctx.Fr.zero, ctx.Fr.zero, ctx.Fr.zero, ctx.Fr.zero, ctx.Fr.zero, ctx.Fr.zero];
    }
    return [ctx.Fr.zero, ctx.Fr.zero, ctx.Fr.zero, ctx.Fr.zero, ctx.Fr.zero, ctx.Fr.zero, ctx.Fr.zero, ctx.Fr.zero];
}

function eval_exp(ctx, tag) {
    if (tag.params.length != 2) throw new Error(`Invalid number of parameters (2 != ${tag.params.length}) function ${tag.funcName} ${ctx.sourceRef}`)
    const a = evalCommand(ctx, tag.params[0]);
    const b = evalCommand(ctx, tag.params[1])
    return scalar2fea(ctx.Fr, Scalar.exp(a, b));;
}

function eval_bitwise(ctx, tag) {
    const func = tag.funcName.split('_')[1];
    const a = evalCommand(ctx, tag.params[0]);
    let b;

    switch (func) {
        case 'and':
            checkParams(ctx, tag, 2);
            b = evalCommand(ctx, tag.params[1]);
            return Scalar.band(a, b);
        case 'or':
            checkParams(ctx, tag, 2);
            b = evalCommand(ctx, tag.params[1]);
            return Scalar.bor(a, b);
        case 'xor':
            checkParams(ctx, tag, 2);
            b = evalCommand(ctx, tag.params[1]);
            return Scalar.bxor(a, b);
        case 'not':
            checkParams(ctx, tag, 1);
            return Scalar.bxor(a, Mask256);
        default:
            throw new Error(`Invalid bitwise operation ${func} (${tag.funcName}) ${ctx.sourceRef}`)
    }
}

function eval_beforeLast(ctx) {
    if (ctx.step >= ctx.stepsN-2) {
        return [0n, 0n, 0n, 0n, 0n, 0n, 0n, 0n];
    } else {
        return [ctx.Fr.negone, 0n, 0n, 0n, 0n, 0n, 0n, 0n];
    }
}

function eval_comp(ctx, tag){
    checkParams(ctx, tag, 2);

    const func = tag.funcName.split('_')[1];
    const a = evalCommand(ctx,tag.params[0]);
    const b = evalCommand(ctx,tag.params[1]);

    switch (func){
        case 'lt':
            return Scalar.lt(a, b) ? 1 : 0;
        case 'gt':
            return Scalar.gt(a, b) ? 1 : 0;
        case 'eq':
            return Scalar.eq(a, b) ? 1 : 0;
        default:
            throw new Error(`Invalid bitwise operation ${func} (${tag.funcName}) ${ctx.sourceRef}`)
    }
}

function eval_loadScalar(ctx, tag){
    checkParams(ctx, tag, 1);
    return evalCommand(ctx,tag.params[0]);
}

function eval_storeLog(ctx, tag){
    checkParams(ctx, tag, 3);

    const indexLog = evalCommand(ctx, tag.params[0]);
    const isTopic = evalCommand(ctx, tag.params[1]);
    const data = evalCommand(ctx, tag.params[2]);

    if (typeof ctx.outLogs[indexLog] === "undefined"){
        ctx.outLogs[indexLog] = {
            data: [],
            topics: []
        }
    }

    if (isTopic) {
        ctx.outLogs[indexLog].topics.push(data.toString(16));
    } else {
        ctx.outLogs[indexLog].data.push(data.toString(16));
    }
    if (fullTracer)
        fullTracer.handleEvent(ctx, tag);
}

function eval_log(ctx, tag) {
    const frLog = ctx[tag.params[0].regName];
    const label = typeof tag.params[1] === "undefined" ? "notset" : tag.params[1].varName;
    if(typeof(frLog) == "number") {
        console.log(frLog)
    } else {
        let scalarLog;
        let hexLog;
        if (tag.params[0].regName !== "HASHPOS" && tag.params[0].regName !== "GAS"){
            scalarLog = safeFea2scalar(ctx.Fr, frLog);
            hexLog = `0x${scalarLog.toString(16)}`;
        } else {
            scalarLog = Scalar.e(frLog);
            hexLog = `0x${scalarLog.toString(16)}`;
        }

        console.log(`Log regname ${tag.params[0].regName} ${ctx.sourceRef}`);
        if (label !== "notset")
            console.log("       Label: ", label);
        console.log("       Scalar: ", scalarLog);
        console.log("       Hex:    ", hexLog);
        console.log("--------------------------");
    }
    return scalar2fea(ctx.Fr, Scalar.e(0));
}

function eval_breakPoint(ctx, tag) {
    console.log(`Breakpoint: ${ctx.sourceRef}`);
    return scalar2fea(ctx.Fr, Scalar.e(0));
}

// Helpers MemAlign


function checkParams(ctx, tag, expectedParams){
    if (tag.params.length != expectedParams) throw new Error(`Invalid number of parameters (${expectedParams} != ${tag.params.length}) function ${tag.funcName} ${ctx.sourceRef}`);
}


function sr8to4(F, SR) {
    const r=[];
    r[0] = F.add(SR[0], F.mul(SR[1], F.e("0x100000000")));
    r[1] = F.add(SR[2], F.mul(SR[3], F.e("0x100000000")));
    r[2] = F.add(SR[4], F.mul(SR[5], F.e("0x100000000")));
    r[3] = F.add(SR[6], F.mul(SR[7], F.e("0x100000000")));
    return r;
}

function sr4to8(F, r) {
    const sr=[];
    sr[0] = r[0] & 0xFFFFFFFFn;
    sr[1] = r[0] >> 32n;
    sr[2] = r[1] & 0xFFFFFFFFn;
    sr[3] = r[1] >> 32n;
    sr[4] = r[2] & 0xFFFFFFFFn;
    sr[5] = r[2] >> 32n;
    sr[6] = r[3] & 0xFFFFFFFFn;
    sr[7] = r[3] >> 32n;
    return sr;
}

function safeFea2scalar(Fr, arr) {
    for (let index = 0; index < 8; ++index) {
        const value = Fr.toObject(arr[index]);
        if (value > 0xFFFFFFFFn) {
            throw new Error(`Invalid value 0x${value.toString(16)} to convert to scalar on index ${index}: ${sourceRef}`);
        }
    }
    return fea2scalar(Fr, arr);
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
        // if (l.offset) addrRel += l.offset;
        // if (l.isStack == 1) addrRel += Number(ctx.SP);
        // if (!skipAddrRelControl) {
        //     if (addrRel >= 0x20000 || (!l.isMem && addrRel >= 0x10000)) throw new Error(`Address too big ${sourceRef}`);
        //     if (addrRel <0 ) throw new Error(`Address can not be negative ${sourceRef}`);
        // }
        // addr = addrRel;
/*
        let addrRel = 0;
        let addr = 0;
        if (l.mOp || l.JMP || l.JMPN || l.JMPC || l.JMPZ || l.call ||
            l.hashP || l.hashP1 || l.hashPLen || l.hashPDigest ||
            l.hashK || l.hashK1 || l.hashKLen || l.hashKDigest ||
            l.hashS || l.hashS1 || l.hashSLen || l.hashSDigest) {
        }
        if (l.useCTX==1) {
            addr += Number(ctx.CTX)*0x40000;
            pols.useCTX[i] = 1n;
        } else {
            pols.useCTX[i] = 0n;
        }
        if (l.isStack==1) {
            addr += 0x10000;
            pols.isStack[i] = 1n;
        } else {
            pols.isStack[i] = 0n;
        }
        if (l.isMem==1) {
            addr += 0x20000;
            pols.isMem[i] = 1n;
        } else {
            pols.isMem[i] = 0n;
        }
        if (l.incStack) {
            pols.incStack[i] = BigInt(l.incStack);
        } else {
            pols.incStack[i] = 0n;
        }
        if (l.ind) {
            pols.ind[i] = 1n;
        } else {
            pols.ind[i] = 0n;
        }
        if (l.indRR) {
            pols.indRR[i] = 1n;
        } else {
            pols.indRR[i] = 0n;
        }
        if (l.offset) {
            pols.offset[i] = BigInt(l.offset);
        } else {
            pols.offset[i] = 0n;
        }
    */
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
            this.inRegs(l, op); // add inRegs to opReg
            this.inConst(l, op); // add CONST to opReg
            // init op with CONST;
            this.calculateRelativeAddress(l);
            
            // selectors, component, mapping (lookup/multiset)

            if (this.romline.inFREE || this.romline.inFREE0) {
                if (!this.romline.freeInTag) {
                    throw new Error(`Instruction with freeIn without freeInTag ${sourceRef}`);
                }

                let fi;
                if (l.freeInTag.op=="") {
                    let nHits = 0;
                    if (l.mOp == 1 && l.mWR == 0) {
                        if (typeof ctx.mem[addr] != "undefined") {
                            fi = ctx.mem[addr];
                        } else {
                            fi = [Fr.zero, Fr.zero, Fr.zero, Fr.zero, Fr.zero, Fr.zero, Fr.zero, Fr.zero];
                        }
                        nHits++;
                    }
                    this.calculateFreeInputFromComponents();
                    this.components.calculateFreeInput()

                    if (nHits==0) {
                        throw new Error(`Empty freeIn without a valid instruction ${sourceRef}`);
                    }
                    if (nHits>1) {
                        throw new Error(`Only one instruction that requires freeIn is allowed ${sourceRef}`);
                    }
                } else {
                    fi = evalCommand(ctx, l.freeInTag);
                    if (!Array.isArray(fi)) fi = scalar2fea(Fr, fi);
                }
            [pols.FREE0[i], pols.FREE1[i], pols.FREE2[i], pols.FREE3[i], pols.FREE4[i], pols.FREE5[i], pols.FREE6[i], pols.FREE7[i]] = fi;
            [op0, op1, op2, op3, op4, op5, op6, op7] =
                [Fr.add( Fr.mul(Fr.add(Fr.e(l.inFREE), Fr.e(l.inFREE0)), fi[0]), op0 ),
                 Fr.add( Fr.mul(Fr.e(l.inFREE), fi[1]), op1 ),
                 Fr.add( Fr.mul(Fr.e(l.inFREE), fi[2]), op2 ),
                 Fr.add( Fr.mul(Fr.e(l.inFREE), fi[3]), op3 ),
                 Fr.add( Fr.mul(Fr.e(l.inFREE), fi[4]), op4 ),
                 Fr.add( Fr.mul(Fr.e(l.inFREE), fi[5]), op5 ),
                 Fr.add( Fr.mul(Fr.e(l.inFREE), fi[6]), op6 ),
                 Fr.add( Fr.mul(Fr.e(l.inFREE), fi[7]), op7 )
                ];
            pols.inFREE[i] = Fr.e(l.inFREE);
            pols.inFREE0[i] = Fr.e(l.inFREE0);
        } else {
            [pols.FREE0[i], pols.FREE1[i], pols.FREE2[i], pols.FREE3[i], pols.FREE4[i], pols.FREE5[i], pols.FREE6[i], pols.FREE7[i]] = [Fr.zero, Fr.zero, Fr.zero, Fr.zero, Fr.zero, Fr.zero, Fr.zero, Fr.zero];
            pols.inFREE[i] = Fr.zero;
            pols.inFREE0[i] = Fr.zero;
        }

        if (Fr.isZero(op0)) {
            pols.op0Inv[i] = 0n;
        } else {
            pols.op0Inv[i] = Fr.inv(op0);
        }

//////////
// PROCESS INSTRUCTIONS
////////// 
        this.verifyComponents();

        if (l.assert) {
            if ((Number(ctx.zkPC) === rom.labels.assertNewStateRoot) && skipAsserts){
                console.log("Skip assert newStateRoot");
            } else if ((Number(ctx.zkPC) === rom.labels.assertNewLocalExitRoot) && skipAsserts){
                console.log("Skip assert newLocalExitRoot");
            } else if (
                    (!Fr.eq(ctx.A[0], op0)) ||
                    (!Fr.eq(ctx.A[1], op1)) ||
                    (!Fr.eq(ctx.A[2], op2)) ||
                    (!Fr.eq(ctx.A[3], op3)) ||
                    (!Fr.eq(ctx.A[4], op4)) ||
                    (!Fr.eq(ctx.A[5], op5)) ||
                    (!Fr.eq(ctx.A[6], op6)) ||
                    (!Fr.eq(ctx.A[7], op7))
            ) {
                throw new Error(`Assert does not match ${sourceRef} (op:${fea2scalar(Fr, [op0, op1, op2, op3, op4, op5, op6, op7])} A:${fea2scalar(Fr, ctx.A)})`);
            }
            pols.assert[i] = 1n;
        } else {
            pols.assert[i] = 0n;
        }

        if (l.repeat) {
            pols.repeat[i] = 1n;
        } else {
            pols.repeat[i] = 0n;
        }

    //////////
    // SET NEXT REGISTERS
    //////////
        this.setRegs();
        this.setCounters(); // REVIEW: asReg?, how link to components, pil is the key
/*
        if (l.setRCX == 1) {
            pols.setRCX[i] = 1n;
            pols.RCX[nexti] = BigInt(fe2n(Fr, op0, ctx));
        } else {
            pols.setRCX[i] = 0n;
            if (!Fr.isZero(pols.RCX[i]) && l.repeat == 1) {
                pols.RCX[nexti] = Fr.add(pols.RCX[i], Fr.negone);
            } else {
                pols.RCX[nexti] = pols.RCX[i];
            }
        }

        if (Fr.isZero(pols.RCX[nexti])) {
            pols.RCXInv[nexti] = 0n;
        } else {
            if (!Fr.eq(previousRCX,pols.RCX[nexti])) {
                previousRCX = pols.RCX[nexti];
                previousRCXInv = Fr.inv(Fr.e(previousRCX));
            }
            pols.RCXInv[nexti] = previousRCXInv;
        }

        pols.JMP[i] = 0n;
        pols.JMPN[i] = 0n;
        pols.JMPC[i] = 0n;
        pols.JMPZ[i] = 0n;
        pols.return[i] = 0n;
        pols.call[i] = 0n;

        pols.jmpAddr[i] = l.jmpAddr ? BigInt(l.jmpAddr) : 0n;
        pols.useJmpAddr[i] = l.useJmpAddr ? 1n: 0n;

        const finalJmpAddr = l.useJmpAddr ? l.jmpAddr : addr;
        const nextNoJmpZkPC = pols.zkPC[i] + ((l.repeat && !Fr.isZero(ctx.RCX)) ? 0n:1n);

        let elseAddr = l.useElseAddr ? BigInt(l.elseAddr) : nextNoJmpZkPC;
        // modify JMP 'elseAddr' to continue execution in case of an unsigned transaction
        if (config.unsigned && l.elseAddrLabel === 'invalidIntrinsicTxSenderCode') {
            elseAddr = BigInt(finalJmpAddr);
        }

        pols.elseAddr[i] = l.elseAddr ? BigInt(l.elseAddr) : 0n;
        pols.useElseAddr[i] = l.useElseAddr ? 1n: 0n;

        if (l.JMPN) {            
            const o = Fr.toObject(op0);
            if (calculateReservedCounters) {
                const counterControl = counterControls[l.jmpAddrLabel] ?? false;
                if (counterControl !== false && counterControl.limit !== false) {
                    const reserv = counterControl.limit - (o < FrFirst32Negative ? o : o - (FrFirst32Negative + 0xFFFFFFFF));
                    if (typeof counterControl.reserved === 'undefined' || counterControl.reserved < reserv) {
                        counterControl.reserved = reserv;
                        counterControl.sourceRef = sourceRef;
                    }
                }
            }
            let jmpnCondValue = o;
            if (o > 0 && o >= FrFirst32Negative) {
                pols.isNeg[i]=1n;
                jmpnCondValue = Fr.toObject(Fr.e(jmpnCondValue + 2n**32n));
                pols.zkPC[nexti] = BigInt(finalJmpAddr);
            } else if (o >= 0 && o <= FrLast32Positive) {
                pols.isNeg[i]=0n;
                pols.zkPC[nexti] = elseAddr;
            } else {
                throw new Error(`On JMPN value ${o} not a valid 32bit value ${sourceRef}`);
            }
            pols.lJmpnCondValue[i] = jmpnCondValue & 0x7FFFFFn;
            jmpnCondValue = jmpnCondValue >> 23n;
            for (let index = 0; index < 9; ++index) {
                pols.hJmpnCondValueBit[index][i] = jmpnCondValue & 0x01n;
                jmpnCondValue = jmpnCondValue >> 1n;
            }
            pols.JMPN[i] = 1n;
        } else {
            pols.isNeg[i] = 0n;
            pols.lJmpnCondValue[i] = 0n;
            for (let index = 0; index < 9; ++index) {
                pols.hJmpnCondValueBit[index][i] = 0n;
            }
            if (l.JMPC) {
                if (pols.carry[i]) {
                    pols.zkPC[nexti] = BigInt(finalJmpAddr);
                } else {
                    pols.zkPC[nexti] = elseAddr;
                }
                pols.JMPC[i] = 1n;
            } else if (l.JMPZ) {
                if (Fr.isZero(op0)) {
                    pols.zkPC[nexti] = BigInt(finalJmpAddr);
                } else {
                    pols.zkPC[nexti] = elseAddr;
                }
                pols.JMPZ[i] = 1n;
                const o = Fr.toObject(op0);
                if (o > 0 && o >= FrFirst32Negative) {
                    // console.log(`WARNING: JMPZ with negative value ${sourceRef}`);
                }
            } else if (l.JMP) {
                pols.zkPC[nexti] = BigInt(finalJmpAddr);
                pols.JMP[i] = 1n;
            } else if (l.call) {
                pols.zkPC[nexti] = BigInt(finalJmpAddr);
                pols.call[i] = 1n;
            } else if (l.return) {
                pols.zkPC[nexti] = ctx.RR;
                pols.return[i] = 1n;
            } else {
                pols.zkPC[nexti] = nextNoJmpZkPC;
            }
        }

        if (pols.zkPC[nexti] == (pols.zkPC[i] + 1n)) {
            pendingCmds = l.cmdAfter;
        }
        if (checkJmpZero && pols.zkPC[nexti] === 0n && nexti !== 0) {
            if (checkJmpZero === ErrorCheck) {
                throw new Error(`ERROR: Not final JMP to 0 (N=${N}) ${sourceRef}`);
            }
            console.log(`WARNING: Not final JMP to 0 (N=${N}) ${sourceRef}`);
        }
    }
    } catch (error) {
        if (!error.message.includes(sourceRef)) {
            error.message += ' '+sourceRef;
        }
        throw error;
    }*/
    this.componentsEnd();
    // required filled when verifys
}


