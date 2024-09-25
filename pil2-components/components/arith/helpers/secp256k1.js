
const Helper = require(__dirname + '../../src/helper.js');

// TODO: multiple helpers, one of each operation, here load all of them using options

module.exports = class extends Helper {
    constructor (options = {}) {
        super(options);
        this.fec = false;
        this.cache = {
                x1: false, 
                y1: false, 
                x2: false, 
                y2: false, 
                dbl: false, 
                x3: false, 
                y3: false
            };
    }
    eval_xAddPointEc(cmd) {
        return this.addPointEc(cmd, false)[0];
    }

    eval_yAddPointEc(cmd) {
        return this.addPointEc(cmd, false)[1];
    }

    eval_xDblPointEc(cmd) {
        return this.addPointEc(cmd, true)[0];
    }

    eval_yDblPointEc(cmd) {
        return this.addPointEc(cmd, true)[1];
    }

    addPointEc(cmd, dbl = false) {
        const x1 = this.fec.e(this.eval_cmd(cmd.params[0]));
        const y1 = this.fec.e(this.eval_cmd(cmd.params[1]));
        const x2 = this.fec.e(this.eval_cmd(cmd.params[dbl ? 0 : 2]));
        const y2 = this.fec.e(this.eval_cmd(cmd.params[dbl ? 1 : 3]));
        const cache = this.cache;
        if (cache.dbl !== dbl || cache.x1 !== x1 || cache.y1 !== y1 || cache.x2 !== x2 || cache.y2 !== y2) {
            let s;
            if (dbl) {
                const divisor = this.fec.add(y1, y1)
                if (this.fec.isZero(divisor)) {
                    throw new Error(`Invalid AddPointEc (divisionByZero) ${this.sourceRef}`);
                }
                s = this.fec.div(this.fec.mul(3n, this.fec.mul(x1, x1)), divisor);
            }
            else {
                const deltaX = this.fec.sub(x2, x1)
                if (this.fec.isZero(deltaX)) {
                    throw new Error(`Invalid AddPointEc (divisionByZero) ${this.sourceRef}`);
                }
                s = this.fec.div(this.fec.sub(y2, y1), deltaX);
            }
            cache.x1 = x1;
            cache.y1 = y1;
            cache.x2 = x2;
            cache.y2 = y2;
            cache.dbl = dbl;
            cache.x3 = this.fec.sub(this.fec.mul(s, s), this.fec.add(x1, x2));
            cache.y3 = this.fec.sub(this.fec.mul(s, this.fec.sub(x1,x3)), y1);
        }
        return [cache.x3, cache.y3];
    }
}