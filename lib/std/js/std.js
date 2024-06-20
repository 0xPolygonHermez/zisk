const { WitnessCalculatorComponent } = require("pil2-proofman/src/witness_calculator_component.js");

const log = require("pil2-proofman/logger.js");

const { setPol } = require("pil2-stark-js/src/prover/prover_helpers.js");

// const Sum = require("./std_sum.js");
const Prod = require("./std_prod.js");

module.exports = class Std extends WitnessCalculatorComponent {
    constructor(wcManager, proofCtx) {
        super("std", wcManager, proofCtx);
    }

    // This function should decide what to call in the std depending on what the pilout specifies
    async witnessComputation(stageId, subproofId, airInstance, publics) {
        console.log(`### STD stage:${stageId} subproof:${subproofId} air:${airInstance.airId} instance:${airInstance.instanceId}`);

        // The following waits until an executor finishes
        const instancesData = await this.wcManager.receiveData(this, "std");
        // TODO: Find a generalization of this?
        const instancesIds = {};
        for (let i = 0; i < instancesData.length; i++) {
            const info = instancesData[i];
            if (info.type === 'notification' && info.payload.data === 'finished') {
                instancesIds.stageId = info.payload.stageId;
                instancesIds.subproofId = info.payload.subproofId;
                instancesIds.airId = info.payload.airId;
                instancesIds.instanceId = info.payload.instanceId;
                break;
            }
        }

        // We need to be able to know at this point which component of the std we need to call
        // const hints = this.proofCtx.airout.hints.filter(h => h.subproofId === subproofId && h.airId === airId);
        const instanceToProcess = this.proofCtx.getAirInstancesBySubproofIdAirId(instancesIds.subproofId,instancesIds.airId)[instancesIds.instanceId];
        const hints = this.proofCtx.setup.setup[instancesIds.subproofId][instancesIds.airId].expressionsInfo.hintsInfo;
        const hints_sum = [];
        let gprod_hint = 0;
        for (const h of hints) {
            if (h.name === 'gsum_col' || h.name === 'sum_assumes' || h.name === 'sum_proves') {
                hints_sum.push(h);
            } else if (h.name === 'gprod_col') {
                gprod_hint = h;
            } else {
                throw new Error(`Unknown hint type ${h.name} for std`);
            }
        }

        if (gprod_hint && instancesIds.stageId === 2) {
            const prod = new Prod(this.wcManager, this.proofCtx, gprod_hint);
            await prod.witnessComputation(2, instancesIds.subproofId, instanceToProcess, {});
        }

        // TODO
        // if (hints_sum) {
        //     const sum = new Sum(this.wcManager, this.proofCtx, hints_sum);
        //     await sum.witnessComputation(stageId, subproofId, airId, instanceId, publics);
        // }
    }
}
