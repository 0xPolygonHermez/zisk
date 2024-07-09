const { WitnessCalculatorComponent } = require('pil2-proofman/src/witness_calculator_component.js');

const log = require("pil2-proofman/logger.js");

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
        const N = airInstance.layout.numRows;

        const STD = this.wcManager.wc.find(wc => wc.name === "STD");
        const [range1, range2] = STD.setupRange(airInstance);

        const a1 = airInstance.wtnsPols.RangeCheck1.a1;
        const a2 = airInstance.wtnsPols.RangeCheck1.a2;

        const sel1 = airInstance.wtnsPols.RangeCheck1.sel1;
        const sel2 = airInstance.wtnsPols.RangeCheck1.sel2;

        for (let i = 0; i < N; i++) {
            a1[i] = getRandom(0, 2**8-1);
            a2[i] = getRandom(0, 2**4-1);

            sel1[i] = getRandom(0, 1);
            sel2[i] = getRandom(0, 1);

            if (sel1[i]) STD.rangeCheck(range1, a1[i]);
            if (sel2[i]) STD.rangeCheck(range2, a2[i]);
        }
    }
}

// Note: It works as expected for number up to Number.MAX_SAFE_INTEGER=2^53-1
function getRandom(min, max) {
    return BigInt(Math.floor(Math.random()*(max-min+1)+min));
}