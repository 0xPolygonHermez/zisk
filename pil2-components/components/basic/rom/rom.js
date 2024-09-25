const Component = require('../../../src/component.js');
const DEFAULT_ROM_ID = 2;
module.exports = class ROM extends Component {
    constructor(config = {}) {
        super(config);
    }

    getDefaultId() {
        return DEFAULT_ROM_ID;
    }

    calculateVerify(verify, values) {
        const [zkPC, fileName, line] = values;

        if (verify) {
            this.proves(zkPC, fileName, line);
            return true;
        }

        return this.onVerifyFails('ROM only operates in verify mode');
    }

    proves(zkPC, fileName, line) {
        if (!this.inputs[zkPC]) {
            this.inputs.push({count: 1, sourceRef: `${fileName}:${line}`});
        } else {
            this.inputs[zkPC].count++;
        }

        // Note: Since the trace Rom is injective, there is no need to normalize the multiplicity
    }

    execute(cols) {
        const F = this.fr;

        const N = cols.mul.length;
        for (let i = 0; i < N; i++) {
            cols.mul[i] = this.inputs[i] ? F.e(this.inputs[i].count) : F.zero;
        }
    }
}
