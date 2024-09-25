const { WitnessCalculatorComponent } = require('pil2-proofman/src/witness_calculator_component.js');

const log = require("pil2-proofman/logger.js");

const { getRandom } = require("../../utils.js");

module.exports = class LookupTest extends WitnessCalculatorComponent {
    constructor(wcManager, proofCtx) {
        super("Lookup Test", wcManager, proofCtx);
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

        const F = this.proofCtx.F;

        const N = airInstance.layout.numRows;
        if (airInstance.wtnsPols.Lookup0) {
            const lenCols = airInstance.wtnsPols.Lookup0.f.length;
            const lenSels = airInstance.wtnsPols.Lookup0.sel.length;
            let f = new Array(lenCols).fill(0n);
            let t = new Array(lenCols).fill(0n);
            for (let i = 0; i < lenCols; i++) {
                f[i] = airInstance.wtnsPols.Lookup0.f[i];
                t[i] = airInstance.wtnsPols.Lookup0.t[i];
            }

            let sel = new Array(lenSels).fill(0n);
            let mul = new Array(lenSels).fill(0n);
            for (let i = 0; i < lenSels; i++) {
                sel[i] = airInstance.wtnsPols.Lookup0.sel[i];
                mul[i] = airInstance.wtnsPols.Lookup0.mul[i];
            }

            for (let i = 0; i < N; i++) {
                for (let j = 0; j < lenCols; j++) {
                    f[j][i] = F.random()[0];
                    t[j][i] = f[j][i];
                }

                for (let j = 0; j < lenSels; j++) {
                    sel[j][i] = getRandom(0, 1);
                    mul[j][i] = sel[j][i];
                }
            }
        } else {
            throw new Error(`Connection not found in air instance`);
        }
    }
}