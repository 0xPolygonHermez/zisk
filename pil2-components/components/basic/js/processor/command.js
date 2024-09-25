const Context = require('./context.js');
const {assert} = require('chai');

module.exports = class Command {
    constructor () {
        this.operations = {};
        this.helpers = {};
        this.setup();
    }
    registerHelperCall(funcname, object, method) {
        this.helpers[funcname] = {object, method};
    }   
    evalComands(cmds) {
        for (const cmd of cmds) {
            this.evalCommand(cmd);
        }
    }
    evalCommand(cmd) {      
        const def = this.operations[cmd.op];
        if (typeof def === 'undefined') {
            throw new Error(`Invalid operation ${cmd.op} ${Context.sourceRef}`);
        }
        if (!def.autoScalarParams) {
            return def.method.apply(this,[cmd]);
        }
        return def.method.apply(this, cmd.values.map(x => this.evalCommand(x)));
    }
    setup() {
        this.defineOperation('number', (cmd) => BigInt(cmd.num),false);
        this.defineOperation('declareVar', this.declareVariable, false);
        this.defineOperation('setVar', this.declareSetVariable, false);
        this.defineOperation('getVar', this.getVariable, false);
        this.defineOperation('getReg', this.getReg, false);
        this.defineOperation('functionCall', this.functionCall, false);
        this.defineOperation('add', (a,b) => a + b); 
        this.defineOperation('sub', (a,b) => a - b);
        this.defineOperation('neg', (a) => -a);
        this.defineOperation('mul', (a,b) => a * b);
        this.defineOperation('div', (a,b) => a / b);
        this.defineOperation('mod', (a,b) => a % b);
        this.defineOperation('exp', (a,b) => a ** b);
        this.defineOperation('if', this.evalIf, false);
        this.defineOperation('getMemAddr', this.getMemAddr, false);
        this.defineOperation('getMemValue', this.getMemValue, false);
        this.defineOperation('or', (a,b) => (a || b) ? 1n : 0n);
        this.defineOperation('and', (a,b) => (a && b) ? 1n : 0n);
        this.defineOperation('gt', (a,b) => a > b ? 1n : 0n);
        this.defineOperation('ge', (a,b) => a >= b ? 1n : 0n);
        this.defineOperation('lt', (a,b) => a < b ? 1n : 0n);
        this.defineOperation('le', (a,b) => a <= b ? 1n : 0n);
        this.defineOperation('eq', (a,b) => a === b ? 1n : 0n);
        this.defineOperation('ne', (a,b) => a !== b ? 1n : 0n);
        this.defineOperation('not', (a) => a ? 0n : 1n);
        this.defineOperation('bitand', (a,b) => a & b);
        this.defineOperation('bitor', (a,b) => a | b);
        this.defineOperation('bitxor', (a,b) => a ^ b);
        this.defineOperation('bitnot', (a) => ~a);
        this.defineOperation('shl', (a,b) => a << b);
        this.defineOperation('shr', (a,b) => a >> b);

        this.registerHelperCall('beforeLast', this, this.beforeLast);
    }  
    defineOperation(operation, method, autoScalarParams = true) {
        assert(typeof this.operations[operation] === 'undefined');
        this.operations[operation] = {method, autoScalarParams};
    }

    setVar(cmd) {
        const name = this.evalLeft(cmd.values[0]);
        if (typeof this.vars[name] === 'undefined') {
            throw new Error(`Variable ${name} not defined ${Context.sourceRef}`);
        }
        return this.vars[name] = this.evalCommand(cmd.values[1]);
    }

    evalLeft(ctx, tag) {
        if (tag.op == "declareVar") {
            eval_declareVar(ctx, tag);
            return tag.varName;
        } else if (tag.op == "getVar") {
            return tag.varName;
        } else {
            throw new Error(`Invalid left expression (${tag.op}) ${Context.sourceRef}`);
        }
    }

    declareVar(cmd) {
        // local variables, redeclared must start with _
        const name = cmd.VarName;
        if (name.startsWith('_') && typeof this.vars[name] !== 'undefined') {
            throw new Error(`Variable ${name} already declared ${Context.sourceRef}`);
        }
        return this.vars[name] = 0n;
    }

    getVar(cmd) {
        const name = cmd.VarName;
        if (typeof ctx.vars[name] == 'undefined') {
            throw new Error(`Variable ${name} not defined ${Context.sourceRef}`);
        }
        return this.vars[name];
    }

    getReg(cmd) {
        const value = Context.registers.getValue(cmd.regName);
        if (value === false) {
            throw new Error(`Invalid register ${cmd.regName} ${Context.sourceRef}`);
        }
        if (!Array.isArray(value)) {
            return BigInt(value);
        }
        return this.feaToScalar(value);
    }

    evalIf(cmd) {
        const condRes = this.evalCommand(cmd.values[0]);
        return this.evalCommand(cmd.values[ condRes ? 1:2]);
    }

    getMemAddr(cmd) {
        const addr = BigInt(evalCommand(cmd.params[0]));
        return addr + (cmd.useCTX ? this.getReg({regName: 'CTX'}) * 0x40000n : 0n);
    }

    getMemValue(cmd) {
        const offset = this.getMemAddr(cmd);
        return this.feaToScalar(Context.memory.get(offset));
    }

    functionCall(cmd) {
        const helper = this.helpers[cmd.funcName];
        const res = helper ? helper.method.apply(helper.object, [cmd]) : null;
        if (res !== null) {
            return res;
        }
        throw new Error(`function ${cmd.funcName} not defined ${Context.sourceRef}`);
    }
    beforeLast(cmd) {
        // console.log({step: Context.step, N: Context.N});
        let res = Context.zeroValue;
        if (Context.step < (Context.N-2)) {
            res[0] = Context.fr.negone;
        }
        return res;
    }
}