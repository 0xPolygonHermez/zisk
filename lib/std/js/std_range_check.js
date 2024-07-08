const { WitnessCalculatorComponent } = require("pil2-proofman/src/witness_calculator_component.js");

const log = require("pil2-proofman/logger.js");

const { setPol, setSubproofValue } = require("pil2-stark-js/src/prover/prover_helpers.js");
const { getHintField } = require("pil2-stark-js/src/prover/hints_helpers.js");

module.exports = class RangeCheck extends WitnessCalculatorComponent {
    constructor(wcManager, proofCtx) {
        super("STD Range Check", wcManager, proofCtx);

        this.inputs = {};
    }

    proves(val) {
        if (!this.inputs[val]) {
            this.inputs[val] = 1n;
        } else {
            this.inputs[val]++;
        }
    }

    async witnessComputation(stageId, subproofId, airInstance, publics) {
        log.info(`[${this.name}]`, `Starting witness computation stage ${stageId}.`);

        // Do this for every subproof of the range check

        if (stageId === 1) {

            const N = airInstance.layout.numRows;

            const mul = airInstance.wtnsPols.U8Air.mul;

            for (let i = 0; i < N; i++) {
                mul[i] = this.inputs[i] ?? 0n;
            }
        }

        if (stageId === 2) {
            const instanceToProcess = this.proofCtx.getAirInstancesBySubproofIdAirId(subproofId,airInstance.airId)[airInstance.instanceId];

            const hints = this.proofCtx.setup.setup[subproofId][airInstance.airId].expressionsInfo.hintsInfo;

            const hint_gsum = hints.find(h => h.name === 'gsum_col');
            const hint_gprod = hints.find(h => h.name === 'gprod_col');

            if (hint_gsum) {
                await this.components['Sum']._witnessComputation(stageId, subproofId, instanceToProcess, publics, hint_gsum);
            }

            if (hint_gprod) {
                await this.components['Prod']._witnessComputation(stageId, subproofId, instanceToProcess, publics, hint_gprod);
            }
        }

        log.info(`[${this.name}       ]`, `Finishing witness computation stage ${stageId}.`);
        return;
    }
}