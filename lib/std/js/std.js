const { WitnessCalculatorComponent } = require("pil2-proofman/src/witness_calculator_component.js");

const log = require("pil2-proofman/logger.js");

const Sum = require("./std_sum.js");
const Prod = require("./std_prod.js");

module.exports = class Std extends WitnessCalculatorComponent {
    constructor(wcManager, proofCtx) {
        super("STD", wcManager, proofCtx);

        this.components = {};

        this.#initializeComponents();
    }

    #initializeComponents() {
        this.components['Sum'] = new Sum(this.wcManager, this.proofCtx);
        this.components['Prod'] = new Prod(this.wcManager, this.proofCtx);
    }

    async witnessComputation(stageId, subproofId, airInstance, publics) {
        log.info(`[${this.name}       ]`, `Starting witness computation stage ${stageId}.`);

        // The following waits until an executor finishes
        const instancesData = await this.receiveData();
        for (let i = 0; i < instancesData.length; i++) {
            const info = instancesData[i];
            if (info.type === 'notification' && info.payload.data === 'finished') {
                await this.decider(info.payload.stageId, info.payload.subproofId, info.payload.airId, info.payload.instanceId, publics);
            }
        }

        log.info(`[${this.name}       ]`, `Finishing witness computation stage ${stageId}.`);
        return;
    }

    // This function should decide what to call in the std depending on what the pilout specifies
    async decider(stageId, subproofId, airId, instanceId, publics) {
        if (stageId === 2) {
            const instanceToProcess = this.proofCtx.getAirInstancesBySubproofIdAirId(subproofId,airId)[instanceId];
            const hints = this.proofCtx.setup.setup[subproofId][airId].expressionsInfo.hintsInfo;

            const hint_gsum = hints.find(h => h.name === 'gsum_col');
            const hint_gprod = hints.find(h => h.name === 'gprod_col');

            if (hint_gsum) {
                await this.components['Sum']._witnessComputation(stageId, subproofId, instanceToProcess, publics, hint_gsum);
            }

            if (hint_gprod) {
                await this.components['Prod']._witnessComputation(stageId, subproofId, instanceToProcess, publics, hint_gprod);
            }
        }
    }
}
