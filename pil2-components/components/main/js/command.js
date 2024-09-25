module.exports = class Command {
    constructor(parent) {
        this.vars = [];
        this.parent = parent;
    }
    eval_cmds(cmds) {
        if (!Array.isArray(cmds)) {
            cmds = [cmds];
        }
        let res = false;
        for (let j=0; j< cmds.length; j++) {
            res = this.eval_cmd(cmds[j]);
        }
        return res;
    }
    eval_cmd(cmd) {
        const direct_method = this[`eval_${cmd.op}`];
        if (typeof direct_method !== 'function') {
            const method = this[`eval__${cmd.op}`];
            if (typeof method !== 'function') {
                throw new Error(`Invalid operation ${cmd.op} ${this.ctx.sourceRef}`);
            }
            return direct_method.apply(this, cmd);    
        }
        return direct_method.apply(this, cmd.values.map(x => this.eval_cmd(x)));
    }
    eval_add(a, b) {
        return Scalar.add(a,b);
    }
    eval_sub(a, b) {
        return Scalar.sub(a,b);
    }
    eval_neg(a) {
        return Scalar.neg(a);
    }
    eval_mul(a, b) {
        return Scalar.mul(a,b);
    }
    eval_div(a,b) {
        return Scalar.div(a,b);
    }
    eval_mod(a,b) {
        return Scalar.mod(a,b);
    }
    eval_bitnot(a) {
        // TODO: use Scalar
        return ~a;
    }
    eval_bitor(a, b) {
        return Scalar.bor(a,b);
    }
    eval_bitand(a, b) {
        return Scalar.band(a,b);
    }
    eval_bitxor(a,b) {
        return Scalar.bxor(a,b);
    }
    eval_shl(a,b) {
        return Scalar.shl(a,b);
    }
    eval_shr(a,b) {
        return Scalar.shr(a,b);
    }
    eval__if(cmd) {
        const a = this.eval_cmd(cmd.values[0]);
        return this.eval_cmd(cmd.values[ a ? 1:2]);
    }
    eval_not(a) {
        return a ? 0 : 1;
    }
    eval_or(a,b) {
        return a || b ? 1: 0;
    }
    eval_and(a,b) {
        return a && b ? 1: 0;
    }
    eval_eq(a,b) {
        return a == b ? 1: 0;
    }
    eval_ne(a,b) {
        return a != b ? 1: 0;
    }
    eval_gt(a,b) {
        return a > b ? 1: 0;
    }
    eval_ge(a,b) {
        return a >= b ? 1: 0;
    }
    eval_lt(a,b) {
        return a < b ? 1: 0;
    }
    eval_le(a,b) {
        return a <= b ? 1: 0;
    }
    eval_getReg(reg) {
        return this.parent.getReg(reg);
    }
    eval__number(cmd) {
        return Scalar.e(cmd.num);
    }
    eval__setVar(cmd) {
        const op = cmd.op;
        if (op !== 'declareVar' || op !== 'getVar') {
            throw new Error(`Invalid left expression (${tag.op}) ${this.parent.sourceRef}`);
        }
        const name = cmd.varName;
        if (op === 'declareVar') {
            this.eval__declareVar(cmd);
        }
        if (typeof this.vars[name] == "undefined") {
            throw new Error(`Variable ${name} not defined ${this.parent.sourceRef}`);
        }
        return this.vars[name] = this.eval_cmd(cmd.values[1]);
    }
    eval__declareVar(cmd) {
        // local variables, redeclared must start with _
        const name = cmd.varName;
        if (name[0] !== '_' && typeof this.vars[name] != "undefined") {
            throw new Error(`Variable ${name} already declared ${this.parent.sourceRef}`);
        }
        return this.vars[name] = Scalar.e(0);
    }
    eval__getVar(cmd) {
        const name = cmd.varName;
        if (typeof this.vars[name] == "undefined") {
            throw new Error(`Variable ${name} not defined ${this.parent.sourceRef}`);
        }
        return this.vars[name];
    }
}