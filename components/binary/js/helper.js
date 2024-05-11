const Helpers = require(__dirname + '../../src/helper.js');
module.exports = class BinaryHelper {
    constructor(config = {}) {
        super(config);
    }

    /**
    * Computes the comparison of 256-bit values a,b by dividing them in 4 chunks of 64 bits
    * and comparing the chunks one-to-one.
    * lt4 = (a[0] < b[0]) && (a[1] < b[1]) && (a[2] < b[2]) && (a[3] < b[3]).
    * @param a - Scalar
    * @param b - Scalar
    * @returns 1 if ALL chunks of a are less than those of b, 0 otherwise.
    */
    lt4(a, b) {
        const MASK64 = 0xFFFFFFFFFFFFFFFFn;
        for (let index = 0; index < 4; ++index) {
            if (Scalar.lt(Scalar.band(Scalar.shr(a, 64 * index), MASK64), Scalar.band(Scalar.shr(b, 64 * index), MASK64)) == false) {
                return 0n;
            }
        }
        return 1n;
    }
}