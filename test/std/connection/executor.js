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
        if (airInstance.wtnsPols.Connection1) {
            const a = airInstance.wtnsPols.Connection1.a;
            const b = airInstance.wtnsPols.Connection1.b;
            const c = airInstance.wtnsPols.Connection1.c;

            if (BigInt(N) === 2n**3n) {
                for (let i = 0; i < N; i++) {
                    a[i] = BigInt(i);
                    b[i] = BigInt(i);
                    c[i] = BigInt(i);
                }
            } else {
                throw new Error(`N=${N} is not supported for this test`);
            }
        } else if (airInstance.wtnsPols.Connection2) {
            const a = airInstance.wtnsPols.Connection2.a;
            const b = airInstance.wtnsPols.Connection2.b;
            const c = airInstance.wtnsPols.Connection2.c;

            if (BigInt(N) === 2n**4n) {
                for (let i = 0; i < N; i++) {
                    a[i] = BigInt(i);
                    if (i === 1) a[i] = a[0];
                    b[i] = BigInt(i);
                    c[i] = BigInt(i);
                }
            } else {
                throw new Error(`N=${N} is not supported for this test`);
            }
        } else if (airInstance.wtnsPols.Connection3) {
            const a = airInstance.wtnsPols.Connection3.a;
            const b = airInstance.wtnsPols.Connection3.b;
            const c = airInstance.wtnsPols.Connection3.c;

            if (BigInt(N) === 2n**12n) {
                for (let i = 0; i < N; i++) {
                    a[i] = BigInt(i);
                    b[i] = BigInt(i);
                    c[i] = BigInt(i);
                }
            } else {
                throw new Error(`N=${N} is not supported for this test`);
            }
        } else if (airInstance.wtnsPols.ConnectionNew) {
            const a = airInstance.wtnsPols.ConnectionNew.a;
            const b = airInstance.wtnsPols.ConnectionNew.b;
            const c = airInstance.wtnsPols.ConnectionNew.c;
            const d = airInstance.wtnsPols.ConnectionNew.d;

            console.log("HEYYY");

            for (let i = 0; i < N; i++) {
                a[i] = BigInt(i);
                b[i] = BigInt(i);
                c[i] = BigInt(i);
                d[i] = BigInt(i);
            }
        } else {
            throw new Error(`Connection not found in air instance`);
        }
    }
}

// Note: It works as expected for number up to Number.MAX_SAFE_INTEGER=2^53-1
function getRandom(min, max) {
    min = BigInt(min);
    max = BigInt(max);

    if (min > max) {
        throw new Error("min must be less than or equal to max");
    }

    const range = max - min + 1n;

    const rand = BigInt(Math.floor(Math.random() * Number(range)));

    return rand + min;
}