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

        if (airInstance.wtnsPols.RangeCheck1) {
            const N = airInstance.layout.numRows;

            STD.setupRangeCheck(airInstance);

            const a1 = airInstance.wtnsPols.RangeCheck1.a1;
            const a2 = airInstance.wtnsPols.RangeCheck1.a2;
            const a3 = airInstance.wtnsPols.RangeCheck1.a3;
            const a4 = airInstance.wtnsPols.RangeCheck1.a4;
            const a5 = airInstance.wtnsPols.RangeCheck1.a5;

            const sel1 = airInstance.wtnsPols.RangeCheck1.sel1;
            const sel2 = airInstance.wtnsPols.RangeCheck1.sel2;
            const sel3 = airInstance.wtnsPols.RangeCheck1.sel3;

            for (let i = 0; i < N; i++) {
                a1[i] = getRandom(0, 2**8-1);
                a2[i] = getRandom(0, 2**4-1);
                a3[i] = getRandom(60, 2**16-1);
                a4[i] = getRandom(8228, 17400);
                a5[i] = getRandom(0, 2**8-1);

                sel1[i] = getRandom(0, 1);
                sel2[i] = getRandom(0, 1);
                sel3[i] = getRandom(0, 1);

                if (sel1[i]) {
                    STD.rangeCheck(a1[i], 0n, 2n**8n-1n);
                    STD.rangeCheck(a3[i], 60n, 2n**16n-1n);
                }
                if (sel2[i]) {
                    STD.rangeCheck(a2[i], 0n, 2n**4n-1n);
                    STD.rangeCheck(a4[i], 8228n, 17400n);
                }
                if (sel3[i]) {
                    STD.rangeCheck(a5[i], 0n, 2n**8n-1n);
                }
            }
        } else if (airInstance.wtnsPols.RangeCheck2) {
            const N = airInstance.layout.numRows;

            STD.setupRangeCheck(airInstance);

            const b1 = airInstance.wtnsPols.RangeCheck2.b1;
            const b2 = airInstance.wtnsPols.RangeCheck2.b2;
            const b3 = airInstance.wtnsPols.RangeCheck2.b3;

            for (let i = 0; i < N; i++) {
                b1[i] = getRandom(0, 2**8-1);
                b2[i] = getRandom(0, 2**9-1);
                b3[i] = getRandom(0, 2**10-1);

                STD.rangeCheck(b1[i], 0n, 2n**8n-1n);
                STD.rangeCheck(b2[i], 0n, 2n**9n-1n);
                STD.rangeCheck(b3[i], 0n, 2n**10n-1n);
            }
        } else if (airInstance.wtnsPols.RangeCheck3) {
            const N = airInstance.layout.numRows;

            STD.setupRangeCheck(airInstance);

            const c1 = airInstance.wtnsPols.RangeCheck3.c1;
            const c2 = airInstance.wtnsPols.RangeCheck3.c2;

            for (let i = 0; i < N; i++) {
                c1[i] = getRandom(0, 2**4-1);
                c2[i] = getRandom(0, 2**8-1);

                STD.rangeCheck(c1[i], 0n, 2n**4n-1n);
                STD.rangeCheck(c2[i], 0n, 2n**8n-1n);
            }
        } else if (airInstance.wtnsPols.RangeCheck4) {
            const N = airInstance.layout.numRows;

            STD.setupRangeCheck(airInstance);

            const a1 = airInstance.wtnsPols.RangeCheck4.a1;
            const a2 = airInstance.wtnsPols.RangeCheck4.a2;
            const a3 = airInstance.wtnsPols.RangeCheck4.a3;
            const a4 = airInstance.wtnsPols.RangeCheck4.a4;
            const a5 = airInstance.wtnsPols.RangeCheck4.a5;
            const a6 = airInstance.wtnsPols.RangeCheck4.a6;
            const a7 = airInstance.wtnsPols.RangeCheck4.a7;
            const a8 = airInstance.wtnsPols.RangeCheck4.a8;

            const sel1 = airInstance.wtnsPols.RangeCheck4.sel1;
            const sel2 = airInstance.wtnsPols.RangeCheck4.sel2;

            for (let i = 0; i < N; i++) {
                a1[i] = getRandom(0, 2**8-1);
                a2[i] = getRandom(50, 2**7-1);
                a3[i] = getRandom(127, 2**8);
                a4[i] = getRandom(1, 2**16+1);
                a5[i] = getRandom(127, 2**16);
                a6[i] = getRandom(-1, 2**3);
                a7[i] = getRandom(-(2**7)+1, -50);
                a8[i] = getRandom(-(2**8)+1, -127);

                sel1[i] = getRandom(0, 1);
                sel2[i] = getRandom(0, 1);

                if (sel1[i]) {
                    STD.rangeCheck(a1[i], 0n, 2n**16n-1n);
                    STD.rangeCheck(a5[i], 127n, 2n**16n);
                    STD.rangeCheck(a6[i], -1n, 2n**3n);
                }
                if (sel2[i]) {
                    STD.rangeCheck(a1[i], 0n, 2n**8n-1n);
                    STD.rangeCheck(a2[i], 50n, 2n**7n-1n);
                    STD.rangeCheck(a3[i], 127n, 2n**8n);
                    STD.rangeCheck(a4[i], 1n, 2n**16n+1n);
                }

                STD.rangeCheck(a7[i], -(2n**7n)+1n, -50n);
                STD.rangeCheck(a8[i], -(2n**8n)+1n, -127n);
            }
        }
    }
}