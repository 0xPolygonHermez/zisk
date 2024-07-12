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

        const a = airInstance.wtnsPols.MultiRangeCheck.a;

        const sel = airInstance.wtnsPols.MultiRangeCheck.sel;
        const range_sel = airInstance.wtnsPols.MultiRangeCheck.range_sel;

        for (let i = 0; i < N; i++) {
            sel[i] = 1n;
            range_sel[i] = i % 2 === 0 ? 1n : 0n;

            if (sel[i]) {
                if (range_sel[i] === 1n) {
                    a[i] = getRandom(0, 2**7-1);
                    STD.rangeCheck(a[i], 0n, 2n**7n-1n);
                } else {
                    a[i] = getRandom(0, 2**8-1);
                    STD.rangeCheck(a[i], 0n, 2n**8n-1n);
                }
            }
        }
    }
}