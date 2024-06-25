const { WitnessCalculatorComponent } = require("pil2-proofman/src/witness_calculator_component.js");

const log = require("pil2-proofman/logger.js");

const Sum = require("./std_sum.js");
const Prod = require("./std_prod.js");

module.exports = class Std extends WitnessCalculatorComponent {
    constructor(wcManager, proofCtx) {
        super("STD", wcManager, proofCtx);
    }

    // This function should decide what to call in the std depending on what the pilout specifies
    async witnessComputation(stageId, subproofId, airInstance, publics) {
        log.info(`[${this.name}       ]`, `Starting witness computation stage ${stageId}.`);

        // The following waits until an executor finishes
        const instancesData = await this.receiveData();
        for (let i = 0; i < instancesData.length; i++) {
            const info = instancesData[i];
            if (info.sender === this.name) return; // TODO: This filters own messages

            if (info.type === 'notification' && info.payload.data === 'finished') {
                await this.decider(info.payload.stageId, info.payload.subproofId, info.payload.airId, info.payload.instanceId, publics);
            }
        }
    }

    async decider(stageId, subproofId, airId, instanceId, publics) {
        // We need to be able to know at this point which component of the std we need to call
        const instanceToProcess = this.proofCtx.getAirInstancesBySubproofIdAirId(subproofId,airId)[instanceId];
        const hints = this.proofCtx.setup.setup[subproofId][airId].expressionsInfo.hintsInfo;
        const hints_sum = hints.filter(h => h.name === 'gsum_col' || h.name === 'sum_assumes' || h.name === 'sum_proves');
        const gprod_hint = hints.find(h => h.name === 'gprod_col');

        if (hints_sum.length > 0) {
            const sum = new Sum(this.wcManager, this.proofCtx, hints_sum); // this should only be called once (it is called once per stage)
            await sum.witnessComputation(stageId, subproofId, instanceToProcess, publics);
        }

        if (gprod_hint.length > 0 && stageId === 2) {
            const prod = new Prod(this.wcManager, this.proofCtx, gprod_hint);
            await prod.witnessComputation(2, subproofId, instanceToProcess, publics);
        }
    }
}
