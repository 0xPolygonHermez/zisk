const { WitnessCalculatorComponent } = require("pil2-proofman/src/witness_calculator_component.js");

const log = require("pil2-proofman/logger.js");

const { getHintField } = require("pil2-stark-js/src/prover/hints_helpers.js");

const BYTE = 255n;
const TWOBYTES = 65535n;

module.exports = class RangeCheck extends WitnessCalculatorComponent {
    constructor(wcManager, proofCtx) {
        super("STD Range Check", wcManager, proofCtx);

        this.subproofs = ["U8Air", "U16Air", "SpecifiedRanges"];

        this.inputs = {};
    }

    setup(airInstance) {
        const hints = this.proofCtx.setup.setup[airInstance.subproofId][airInstance.airId].expressionsInfo.hintsInfo;

        const hints_rc = hints.filter(h => h.name === 'range_check');

        let ranges = [];
        for (const hint of hints_rc) {
            const predefined = getHintField(airInstance.ctx, hint, "predefined");
            let min = getHintField(airInstance.ctx, hint, "min");
            const max = getHintField(airInstance.ctx, hint, "max");

            if (min > max) {
                // Min was negative (and max non-negative) but mapped to positive when the moduli was applied
                min = min - this.proofCtx.F.p;
            }

            let range = {min, max};
            if (predefined && min >= 0 && max <= TWOBYTES) {
                if (min == 0 && (max == BYTE || max == TWOBYTES)) {
                    if (max == BYTE) {
                        range = {...range, type: "U8Air"};
                    } else {
                        range = {...range, type: "U16Air"};
                    }
                } else if (max <= BYTE) {
                    range = {...range, type: "U8AirDouble"};
                } else if (max <= TWOBYTES) {
                    range = {...range, type: "U16AirDouble"};
                } else {
                    throw new Error(`[${this.name}]`, `Range check values not supported.`);
                }
            } else {
                this.proves("SpecifiedRanges", null, min, max); // To create the range in a specific order

                range = {...range, type: "SpecifiedRanges"};
            }

            ranges.push(range);
        }
        return ranges;
    }

    assignValues(range, values) {
        switch (range.type) {
            case "U8Air":
                this.proves("U8Air", values);
                break;
            case "U16Air":
                this.proves("U16Air", values);
                break;
            case "U8AirDouble":
                this.proves("U8Air", values-range.min);
                this.proves("U8Air", range.max-values);
                break;
            case "U16AirDouble":
                this.proves("U16Air", values-range.min);
                this.proves("U16Air", range.max-values);
                break;
            case "SpecifiedRanges":
                this.proves("SpecifiedRanges", values, range.min, range.max);
                break;
            default:
                log.error(`[${this.name}]`, `Range check values not supported.`);
                throw new Error(`[${this.name}]`);
        }
    }

    proves(subproof, val, min, max) {
        this.inputs[subproof] = this.inputs[subproof] ?? {};
        if (subproof !== "SpecifiedRanges") {
            this.inputs[subproof][val] = (this.inputs[subproof][val] ?? 0n) + 1n;
        } else {
            const range = `${min}:${max}`;
            this.inputs[subproof][range] = this.inputs[subproof][range] ?? {};
            if (val === null) return;
            if (val > max) {
                // This only happens when min is negative and max is positive
                val -= this.proofCtx.F.p;
            }
            this.inputs[subproof][range][val] = (this.inputs[subproof][range][val] ?? 0n) + 1n;
        }
    }

    async witnessComputation(stageId, subproofId, airInstance, publics) {
        log.info(`[${this.name}]`, `Starting witness computation stage ${stageId}.`);

        if (stageId === 1) {
            const N = airInstance.layout.numRows;

            const subproof = this.subproofs.find(subproof => airInstance.wtnsPols[subproof]);
            if (!subproof) {
                throw new Error(`[${this.name}] Subproof not found.`);
            }

            const mul = airInstance.wtnsPols[subproof].mul;
            if (Array.isArray(mul[0])) {
                const keys = Object.keys(this.inputs[subproof]);
                if (keys.length === mul[0].length) throw new Error(`[${this.name}]`, `Too many ranges.`);
                for (let k = 0; k < mul.length; k++) {
                    const key = keys[k].split(':');
                    const min = BigInt(key[0]);
                    const max = BigInt(key[1]);
                    // Ranges doesn't necessarily have to be a power of two
                    // so we must adjust the multiplicity to that case
                    for (let i = 0; i < N; i++) {
                        if (BigInt(i) >= max - min + 1n) {
                            mul[k][i] = 0n;
                        } else {
                            mul[k][i] = this.inputs[subproof][keys[k]][BigInt(i)+min] ?? 0n;
                        }
                    }
                }
            } else {
                for (let i = 0; i < N; i++) {
                    mul[i] = this.inputs[subproof][i] ?? 0n;
                }
            }
        }

        if (stageId === 2) {
            const instanceToProcess = this.proofCtx.getAirInstancesBySubproofIdAirId(subproofId,airInstance.airId)[airInstance.instanceId];

            const hints = this.proofCtx.setup.setup[subproofId][airInstance.airId].expressionsInfo.hintsInfo;

            const hint_gsum = hints.find(h => h.name === 'gsum_col');

            if (hint_gsum) {
                await this.components['Sum']._witnessComputation(stageId, subproofId, instanceToProcess, publics, hint_gsum);
            } else {
                throw new Error(`[${this.name}]`, `Hint not found.`);
            }
        }

        this.sendData("STD", {sender: this.name, command: "continueSubsequentSubproof"}); // Read TODO (1) in std
        log.info(`[${this.name}       ]`, `Finishing witness computation stage ${stageId}.`);
        return;
    }
}