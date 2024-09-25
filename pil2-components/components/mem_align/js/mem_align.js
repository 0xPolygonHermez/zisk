const Component = require(__dirname + '../src/component.js');

module.exports = class Binary extends Component {
    constructor (config = {}) {
        super(config);
    }
    calculateFreeInput(values) {
        return false;
    }
    verify(values) {
    }
    verify_RD(values) {
        // TODO
        TODO
        const m0 = safeFea2scalar(Fr, ctx.A);
        const m1 = safeFea2scalar(Fr, ctx.B);
        const P2_256 = 2n ** 256n;
        const MASK_256 = P2_256 - 1n;
        const offset = safeFea2scalar(Fr, ctx.C);
        if (offset < 0 || offset > 32) {
            throw new Error(`MemAlign out of range (${offset})  ${sourceRef}`);
        }
        const leftV = Scalar.band(Scalar.shl(m0, offset * 8n), MASK_256);
        const rightV = Scalar.band(Scalar.shr(m1, 256n - (offset * 8n)), MASK_256 >> (256n - (offset * 8n)));
        const _V = Scalar.bor(leftV, rightV);
        fi = scalar2fea(Fr, _V);
        nHits ++;
    }
}