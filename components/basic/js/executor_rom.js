const { WitnessCalculatorComponent } = require("../../../node_modules/pil2-proofman/src/witness_calculator_component.js");
const log = require("../../../node_modules/pil2-proofman/logger.js");

module.exports = class BasicRom extends WitnessCalculatorComponent {
    constructor(wcManager, proofCtx) {
        super("Basic Rom Exe", wcManager, proofCtx);
    }

    async witnessComputation(stageId, subproofId, airId, instanceId, publics) {      
        console.log('witnessComputation (Basic Rom)');
        if(stageId === 1) {            
            if(instanceId !== -1) {
                log.error(`[${this.name}]`, `Air instance id already existing in stageId 1.`);
                throw new Error(`[${this.name}]`, `Air instance id already existing in stageId 1.`);
            }

            const instanceData = await this.wcManager.readData(this, "Rom.createInstances");

            const air = this.proofCtx.airout.subproofs[subproofId].airs[instanceData.airId];

            log.info(`[${this.name}]`, `Creating air instance for air '${air.name}' with N=${air.numRows} rows.`)
            let { result, airInstance } = this.proofCtx.addAirInstance(subproofId, instanceData.airId, air.numRows);

            if (result === false) {
                log.error(`[${this.name}]`, `New air instance for air '${air.name}' with N=${air.numRows} rows failed.`);
                throw new Error(`[${this.name}]`, `New air instance for air '${air.name}' with N=${air.numRows} rows failed.`);
            }
        
            this.createPolynomialTraces(airInstance, publics);
        }

        return;
    }

    createPolynomialTraces(airInstance, publics) {
        const N = airInstance.layout.numRows;
        console.log('createPolynomialTraces (Basic Rom)');
        // rom has not pools
    }   
}