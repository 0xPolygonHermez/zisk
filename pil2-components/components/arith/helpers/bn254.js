
const Helper = require(__dirname + '../../../src/helper.js');

// REVIEW: Generic class with fe by option and prefix like 'ARITH_BN254_' as option

module.exports = class extends Helper {
    constructor (options = {}) {
        super(options);
        this.fpBN254 = false;
    }
    eval_ARITH_BN254_MULFP2_X(cmd) {
        // const ctxFullFe = {...ctx, fullFe: true};
        const [x1,y1,x2,y2] = cmd.params.map(x => this.fpBN254.e(x));
        return this.fpBN254.sub(this.fpBN254.mul(x1,x2), this.fpBN254.mul(y1, y2));
    }
    eval_ARITH_BN254_MULFP2_Y(cmd)  {
        // const ctxFullFe = {...ctx, fullFe: true};
        const [x1,y1,x2,y2] = cmd.params.map(x => this.fpBN254.e(x));
        return this.fpBN254.add(this.fpBN254.mul(x1,y2), this.fpBN254.mul(x2, y1));
    }
    eval_ARITH_BN254_ADDFP2(cmd) {
        // const ctxFullFe = {...ctx, fullFe: true};
        const [x1,x2] = cmd.params.map(x => this.fpBN254.e(x));
        return this.fpBN254.add(x1,x2);
    }
    eval_ARITH_BN254_SUBFP2(cmd) {
        // const ctxFullFe = {...ctx, fullFe: true};
        const [x1,x2] = cmd.params.map(x => this.fpBN254.e(x));
        return this.fpBN254.sub(x1,x2);
    }
}