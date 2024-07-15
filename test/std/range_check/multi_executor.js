const { WitnessCalculatorComponent } = require('pil2-proofman/src/witness_calculator_component.js');

const log = require("pil2-proofman/logger.js");

const { getRandom } = require("../../utils.js");

module.exports = class RangeCheckTest extends WitnessCalculatorComponent {
    constructor(wcManager, proofCtx) {
        super("Range Check Test", wcManager, proofCtx);
    }

    async witnessComputation(stageId, subproofId, airInstance, publics) {
        log.info(`[${this.name}       ]`, `Starting witness computation stage ${stageId}.`);
        if (stageId === 1) {
            const instanceId = airInstance.instanceId;

            if (instanceId !== -1) {
                log.error(`[${this.name}]`, `Air instance id already existing in stageId 1.`);
                throw new Error(`[${this.name}]`, `Air instance id already existing in stageId 1.`);
            }

            airInstance.airId = 0; // TODO: This should be updated automatically

            const air = this.proofCtx.airout.subproofs[subproofId].airs[airInstance.airId];

            log.info(`[${this.name}]`, `Creating air instance for air '${air.name}' with N=${air.numRows} rows.`);
            let result = this.proofCtx.addAirInstance(subproofId, airInstance, air.numRows);

            if (result === false) {
                log.error(`[${this.name}]`, `Air instance for air '${air.name}' with N=${air.numRows} rows failed.`);
                throw new Error(`[${this.name}]`, `Air instance for air '${air.name}' with N=${air.numRows} rows failed.`);
            }

            this.#createPolynomialTraces(stageId, airInstance, publics);
        }

        log.info(`[${this.name}       ]`, `Finishing witness computation stage ${stageId}.`);
        return;
    }

    #createPolynomialTraces(stageId, airInstance, publics) {
        log.info(`[${this.name}]`, `Computing column traces stage ${stageId}.`);

        const STD = this.wcManager.wc.find(wc => wc.name === "STD");

        const N = airInstance.layout.numRows;

        STD.setupRangeCheck(airInstance);

        const len = airInstance.wtnsPols.MultiRangeCheck.a.length;
        let a = new Array(len).fill(0n);
        let sel = new Array(len).fill(0n);
        let range_sel = new Array(len).fill(0n);
        for (let i = 0; i < len; i++) {
            a[i] = airInstance.wtnsPols.MultiRangeCheck.a[i];
            sel[i] = airInstance.wtnsPols.MultiRangeCheck.sel[i];
            range_sel[i] = airInstance.wtnsPols.MultiRangeCheck.range_sel[i];
        }

        for (let i = 0; i < len; i++) {
            if (i === 0) {
                for (let j = 0; j < N; j++) {
                    sel[i][j] = getRandom(0, 1);
                    range_sel[i][j] = j % 2 === 0 ? 1n : 0n;

                    if (sel[i][j]) {
                        if (range_sel[i][j] === 1n) {
                            a[i][j] = getRandom(0, 2**7-1);
                            STD.rangeCheck(a[i][j], 0n, 2n**7n-1n);
                        } else {
                            a[i][j] = getRandom(0, 2**8-1);
                            STD.rangeCheck(a[i][j], 0n, 2n**8n-1n);
                        }
                    }
                }
            } else if (i === 1) {
                for (let j = 0; j < N; j++) {
                    sel[i][j] = getRandom(0, 1);
                    range_sel[i][j] = j % 2 === 0 ? 1n : 0n;

                    if (sel[i][j]) {
                        if (range_sel[i][j] === 1n) {
                            a[i][j] = getRandom(0, 2**7-1);
                            STD.rangeCheck(a[i][j], 0n, 2n**7n-1n);
                        } else {
                            a[i][j] = getRandom(0, 2**6-1);
                            STD.rangeCheck(a[i][j], 0n, 2n**6n-1n);
                        }
                    }
                }
            } else if (i === 2) {
                for (let j = 0; j < N; j++) {
                    sel[i][j] = getRandom(0, 1);
                    range_sel[i][j] = j % 2 === 0 ? 1n : 0n;

                    if (sel[i][j]) {
                        if (range_sel[i][j] === 1n) {
                            a[i][j] = getRandom(2**5, 2**8-1);
                            STD.rangeCheck(a[i][j], 2n**5n, 2n**8n-1n);
                        } else {
                            a[i][j] = getRandom(2**8, 2**9-1);
                            STD.rangeCheck(a[i][j], 2n**8n, 2n**9n-1n);
                        }
                    }
                }
            } else {
                throw new Error(`Invalid index ${i}`);
            }
        }
    }
}