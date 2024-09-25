const Helpers = require(__dirname + '../../../src/helper.js');
const { Scalar, F1Field } = require("ffjavascript");

const MASK_256 = Scalar.sub(Scalar.shl(Scalar.e(1), 256), 1);

module.exports = class BinaryHelper {
    constructor(config = {}) {
        super(config);
    }
    eval_memAlignWR_W0(cmd) {
        // parameters: M0, value, offset
        const [m0,value,offset] = cmd.params.map(param => eval_cmd(param));

        return scalar2fea(this.fr, Scalar.bor(  Scalar.band(m0, Scalar.shl(MASK_256, (32n - offset) * 8n)),
                            Scalar.band(Mask256, Scalar.shr(value, offset * 8n))));
    }
    eval_memAlignWR_W1(cmd) {
        // parameters: M1, value, offset
        const [m1,value,offset] = cmd.params.map(param => eval_cmd(param));

        return scalar2fea(this.Fr, Scalar.bor(  Scalar.band(m1, Scalar.shr(MASK_256, offset * 8n)),
                            Scalar.band(Mask256, Scalar.shl(value, (32n - offset) * 8n))));
    }

    eval_memAlignWR8_W0(ctx, tag) {
        // parameters: M0, value, offset
        const [m0,value,offset] = cmd.params.map(param => eval_cmd(param));
        const bits = (31n - offset) * 8n;

        return scalar2fea(this.Fr, Scalar.bor(  Scalar.band(m0, Scalar.sub(MASK_256, Scalar.shl(0xFFn, bits))),
                            Scalar.shl(Scalar.band(0xFFn, value), bits)));
    }
}