const { WitnessCalculatorComponent } = require('pil2-proofman/src/witness_calculator_component.js');

const log = require("pil2-proofman/logger.js");

module.exports = class FibonacciSquare extends WitnessCalculatorComponent {
    constructor(wcManager, proofCtx) {
        super("FibonacciSq", wcManager, proofCtx);
    }

    async witnessComputation(stageId, subproofId, airInstance, publics) {
        log.info(`[${this.name}       ]`, `Starting witness computation stage ${stageId}.`);
        if(stageId === 1) {
            const instanceId = airInstance.instanceId;

            if(instanceId !== -1) {
                log.error(`[${this.name}]`, `Air instance id already existing in stageId 1.`);
                throw new Error(`[${this.name}]`, `Air instance id already existing in stageId 1.`);
            }

            /// NOTE: Here we decide for test purposes to create a fibonacci 2**4 and a module 2**4
            await this.sendData("Module", {command: "createInstances", airId: 0});
            airInstance.airId = 0; // TODO: This should be updated automatically

            const air = this.proofCtx.airout.subproofs[subproofId].airs[airInstance.airId];

            log.info(`[${this.name}]`, `Creating air instance for air '${air.name}' with N=${air.numRows} rows.`);
            let result = this.proofCtx.addAirInstance(subproofId, airInstance, air.numRows);

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
        const N = airInstance.layout.numRows;

        const polA = airInstance.wtnsPols.FibonacciSquare.a;
        const polB = airInstance.wtnsPols.FibonacciSquare.b;

        const mod = publics[0];

        polB[0] = publics[1];
        polA[0] = publics[2];


        for (let i = 1; i < N; i++) {
            polA[i] = (polA[i - 1]*polA[i - 1] + polB[i - 1]*polB[i - 1]) % mod;
            polB[i] = polA[i-1];
        }

        publics[3] = polA[N - 1];
    }
}