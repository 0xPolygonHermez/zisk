const { WitnessCalculatorComponent, ModuleTypeEnum } = require("pil2-proofman/src/witness_calculator_component.js");

const log = require("pil2-proofman/logger.js");

module.exports = class DivModule extends WitnessCalculatorComponent {
    constructor(wcManager, proofCtx) {
        super("divLib", wcManager, proofCtx, ModuleTypeEnum.DEFERRED);
    }

    async witnessComputation(stageId, subproofId, airInstance, publics) {
        console.log(`################################ div_lib.js STAGE ${stageId} SUBPROOF ${subproofId} AIR ${airId} INSTANCEID ${instanceId}`);
        const payloads = this.wcManager.airBus.getPendingPayloadsByRecipient(this.name);

        // TODO remove this is, it's only a temporal hack to do the wc_manager tests work
        if(stageId === 2) {
            let den = [];
            for(const payload of payloads) {
                const data = payload.data;
                const instance = this.proofCtx.airInstances[data.instanceId];
                den = den.concat(instance.tmpPol[data.tmpPolIdx]);
            }

            den = this.proofCtx.F.batchInverse(den);

            // Copy result to each array
            let idx = 0;
            for(const payload of payloads) {
                const data = payload.data;
                const instance = this.proofCtx.airInstances[data.instanceId];
                instance.tmpPol[data.tmpPolIdx] = den.slice(idx, idx + instance.layout.numRows);
                idx += instance.layout.numRows;
            }
        }
        
        for(const payload of payloads) {
            log.info(`[${this.name}]`, ` Resolving payload: ${payload.payloadId}`);
            this.wcManager.resolveBusPayload(payload.payloadId);
        }
    }
}