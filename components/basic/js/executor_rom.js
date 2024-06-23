const { WitnessCalculatorComponent } = require('pil2-proofman/src/witness_calculator_component.js');
const log = require("pil2-proofman/logger.js");

module.exports = class BasicRom extends WitnessCalculatorComponent {
    constructor(wcManager, proofCtx) {
        super("Basic Rom", wcManager, proofCtx);
    }

    async witnessComputation(stageId, subproofId, airInstance, publics) {
        log.info(`[${this.name}]`, `Starting witness computation stage ${stageId}.`);
        if(stageId === 1) {
            const instanceId = airInstance.instanceId;

            if(instanceId !== -1) {
                log.error(`[${this.name}]`, `Air instance id already existing in stageId 1.`);
                throw new Error(`[${this.name}]`, `Air instance id already existing in stageId 1.`);
            }

            const instanceData = await this.wcManager.receiveData(this.inboxId);
            airInstance.airId = 0; // TODO: This should be updated automatically

            const air = this.proofCtx.airout.subproofs[subproofId].airs[instanceData[0].airId]; // TODO: Should 0 be hardcoded?

            log.info(`[${this.name}]`, `Creating air instance for air '${air.name}' with N=${air.numRows} rows.`);
            let result = this.proofCtx.addAirInstance(subproofId, airInstance, air.numRows)

            if (result === false) {
                log.error(`[${this.name}]`, `Air instance for air '${air.name}' with N=${air.numRows} rows failed.`);
                throw new Error(`[${this.name}]`, `Air instance for air '${air.name}' with N=${air.numRows} rows failed.`);
            }

            this.createPolynomialTraces(stageId, airInstance, publics);
        }

        return;
    }

    createPolynomialTraces(stageId, airInstance, publics) {
        log.info(`[${this.name}]`, `Computing column traces stage ${stageId}.`);

        // rom has not witness cols
    }
}