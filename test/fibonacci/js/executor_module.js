const { WitnessCalculatorComponent } = require('pil2-proofman/src/witness_calculator_component.js');

const log = require("pil2-proofman/logger.js");

module.exports = class Module extends WitnessCalculatorComponent {
    constructor(wcManager, proofCtx) {
        super("Module", wcManager, proofCtx);

        this.inputs = [];
    }

    computeVerify(verify, values) {
        const x = values[0];
        const x_mod = values[1];
        if (!verify) {
            return this.#calculate(x);
        }
        const _x_mod = this.#calculate(x);

        if (x_mod !== _x_mod) {
            throw new Error(`[${this.name}]`, `Verification failed for x=${x} and x_mod=${x_mod}.`);
        }

        this.inputs.push([x, x_mod]);
    }

    #calculate(x) {
        const mod = this.proofCtx.publics[0];

        return x % mod;
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

            let mailbox = await this.receiveData();
            for (let i = 0; i < mailbox.length; i++) {
                await this.#processMessage(stageId, subproofId, airInstance, publics, mailbox[i]);
            }
        }

        log.info(`[${this.name}       ]`, `Finishing witness computation stage ${stageId}.`);
        return;
    }

    async #processMessage(stageId, subproofId, airInstance, publics, msg) {
        if ((msg.sender && msg.payload.data) && (msg.sender === "FibonacciSq" && msg.payload.data === "finished")) {
            const air = this.proofCtx.airout.subproofs[subproofId].airs[airInstance.airId];

            log.info(`[${this.name}]`, `Creating air instance for air '${air.name}' with N=${air.numRows} rows.`);
            let result = this.proofCtx.addAirInstance(subproofId, airInstance, air.numRows);

            if (result === false) {
                log.error( `[${this.name}]`,`New air instance for air '${air.name}' with N=${air.numRows} rows failed.`);
                throw new Error(`[${this.name}]`,`New air instance for air '${air.name}' with N=${air.numRows} rows failed.`);
            }

            this.#createPolynomialTraces(stageId, airInstance, publics);
        }
    }

    #createPolynomialTraces(stageId, airInstance, publics) {
        log.info(`[${this.name}]`, `Computing column traces stage ${stageId}.`);

        const N = airInstance.layout.numRows;

        const STD = this.wcManager.wc.find(wc => wc.name === "STD");
        const [range] = STD.setupRange(airInstance);

        const polX = airInstance.wtnsPols.Module.x;
        const polQ = airInstance.wtnsPols.Module.q;
        const polX_mod = airInstance.wtnsPols.Module.x_mod;

        const mod = publics[0];

        for (let i = 0; i < this.inputs.length; i++) {
            const [x, x_mod] = this.inputs[i];

            polX[i] = x;

            polQ[i] = polX[i] / mod;
            polX_mod[i] = x_mod;

            STD.rangeCheck(range, mod - polX_mod[i]);
        }

        for (let i = this.inputs.length; i < N; i++) {
            polX[i] = 0n;
            polQ[i] = 0n;
            polX_mod[i] = 0n;

            STD.rangeCheck(range, mod - polX_mod[i]); // TODO: is this necessary at all?
        }
    }
}