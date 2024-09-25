const path = require('path');
const { WitnessCalculatorComponent } = require('pil2-proofman/src/witness_calculator_component.js');
const defaultWitnessComputation = require(path.join(__dirname, '..', '..', '..', 'lib/proofman/witness_calculator_component.js'));
const log = require("pil2-proofman/logger.js");

const CLIMB_KEY_CLOCKS = 4;
const LAST_CLOCK = CLIMB_KEY_CLOCKS - 1;
const RESULT_CLOCK = CLIMB_KEY_CLOCKS - 2;

const GL_CHUNKS = [0x00001, 0x3C000, 0x3FFFF, 0x003FF];
const CHUNK_FACTORS = [1n, 2n ** 18n, 2n ** 36n, 2n ** 54n];
const CHUNK_MASKS = [0x3FFFFn, 0x3FFFFn, 0x3FFFFn, 0x3FFn];

const DEBUG = false;

module.exports = class ClimbKey extends WitnessCalculatorComponent {
    constructor(wcManager, proofCtx) {
        super("ClimbKey Exe", wcManager, proofCtx);
    }

    async witnessComputation(stageId, subproofId, airId, instanceId, publics) {     
        defaultWitnessComputation.apply(this, ['ClimbKey', stageId, subproofId, airId, instanceId, publics]);
    }

    createPolynomialTraces(airInstance, publics) {
        const N = airInstance.layout.numRows;
        const cols = airInstance.wtnsPols.Mem;
        console.log('createPolynomialTraces (ClimbKey)');


        // INPUT FORMAT {key: [4], level, bit}

        for (let i = 0; i < input.length; i++) {
            const key = input[i].key.map(x => BigInt(x));
            const level = BigInt(input[i].level);
            const zlevel = Number(input[i].level) % 4;
            const bit = BigInt(input[i].bit);
            let value = key[zlevel];

            if (DEBUG) {
                console.log(`INPUT #${i}: level:${level} bit:${bit} key:${key.join(',')} value:${value}/0x${value.toString(16)}`);
            }
            let carry = bit;
            let lt = 0n;
            for (let clock = 0; clock < CLIMB_KEY_CLOCKS; ++clock) {
                const row = i * CLIMB_KEY_CLOCKS + clock;
                const chunkValue = value & 0x3FFFFn;
                let chunkValueClimbed = chunkValue * 2n + carry;

                value = value >> 18n;

                if (clock == LAST_CLOCK) {
                    key[zlevel] = key[zlevel] * 2n + bit;
                }
                cols.key0[row] = key[0];
                cols.key1[row] = key[1];
                cols.key2[row] = key[2];
                cols.key3[row] = key[3];
                cols.level[row] = level;
                cols.keyInChunk[row] = chunkValue;
                cols.keyIn[row] = (clock === 0 ? 0n : cols.keyIn[row - 1]) + cols.keyInChunk[row] * CHUNK_FACTORS[clock];

                cols.bit[row] = bit;
                cols.carryLt[row] = carry + 2n * lt;

                // CHUNK_MASK has same values than carry limit.
                carry = chunkValueClimbed > CHUNK_MASKS[clock] ? 1n : 0n;

                // to compare with GL only use bits of CHUNK
                const croppedChunkValueClimbed = chunkValueClimbed & CHUNK_MASKS[clock];
                lt = croppedChunkValueClimbed < GL_CHUNKS[clock] ? 1n : (croppedChunkValueClimbed == GL_CHUNKS[clock] ? lt : 0n);

                const keySelLevel = clock == LAST_CLOCK ? zlevel : 0xFFFF;
                cols.keySel0[row] = (keySelLevel === 0) ? 1n : 0n;
                cols.keySel1[row] = (keySelLevel === 1) ? 1n : 0n;
                cols.keySel2[row] = (keySelLevel === 2) ? 1n : 0n;
                cols.keySel3[row] = (keySelLevel === 3) ? 1n : 0n;
                cols.result[row] = clock === RESULT_CLOCK ? 1n : 0n;
                if (DEBUG) {
                    console.log(`TRACE w=${row} key:${cols.key0[row]},${cols.key1[row]},${cols.key2[row]},${cols.key3[row]} level:${cols.level[row]} value:0x${value.toString(16)} keyInChunk:0x${cols.keyInChunk[row].toString(16)} keyIn:0x${cols.keyIn[row].toString(16)}`+
                                ` bit:${cols.bit[row]} carryLt:${cols.carryLt[row]} keySel:${cols.keySel0[row]},${cols.keySel1[row]},${cols.keySel2[row]},${cols.keySel3[row]} result:${cols.result[row]}`);
                }
            }
        }
        // filling the rest of trace to pass the constraints

        const usedRows = input.length * CLIMB_KEY_CLOCKS;
        let row = input.length * 4;
        this.completePolynomialTraces(cols, row, N);

        console.log(`ClimbKeyExecutor successfully processed ${input.length} climbkey actions (${(input.length * CLIMB_KEY_CLOCKS * 100) / N}%)`);
    }
    completePolynomialTraces(cols, row, N) {
        const _keySel0 = [0n, 0n, 0n, 1n];
        const _carryLt = [0n, 2n, 2n, 2n];
        while (row < N) {
            const step = row % 4;
            cols.key0[row] = 0n;
            cols.key1[row] = 0n;
            cols.key2[row] = 0n;
            cols.key3[row] = 0n;
            cols.level[row] = 0n;
            cols.keyIn[row] = 0n;
            cols.keyInChunk[row] = 0n;
            cols.bit[row] = 0n;
            cols.keySel0[row] = _keySel0[step];
            cols.keySel1[row] = 0n;
            cols.keySel2[row] = 0n;
            cols.keySel3[row] = 0n;
            cols.result[row] = 0n;
            cols.carryLt[row] = _carryLt[step];
            ++row;
        }
    }
}