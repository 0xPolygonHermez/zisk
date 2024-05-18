const path = require('path');
const Processor = require('./processor/processor.js');
const { WitnessCalculatorComponent } = require("../../../node_modules/pil2-proofman/src/witness_calculator_component.js");
const log = require("../../../node_modules/pil2-proofman/logger.js");

module.exports = class BasicMain extends WitnessCalculatorComponent {
    constructor(wcManager, proofCtx) {
        super("Basic Main", wcManager, proofCtx);
    }

    async witnessComputation(stageId, subproofId, airId, instanceId, publics) {        
        console.log(`witnessComputation (Basic Main) STAGE(${stageId})`);
        if(stageId === 1) {            
            if(instanceId !== -1) {
                log.error(`[${this.name}]`, `Air instance id already existing in stageId 1.`);
                throw new Error(`[${this.name}]`, `Air instance id already existing in stageId 1.`);
            }

            await this.wcManager.writeData(this, "Rom.createInstances", {"airId": 0});
            await this.wcManager.writeData(this, "Mem.createInstances", {"airId": 0});
            airId = 0;

            console.log('**', subproofId, Object.keys(this.proofCtx.airout.subproofs[subproofId].airs));
            const air = this.proofCtx.airout.subproofs[subproofId].airs[airId];

            log.info(`[${this.name}]`, `Creating air instance for air '${air.name}' with N=${air.numRows} rows.`)
            let { result, airInstance } = this.proofCtx.addAirInstance(subproofId, airId, air.numRows);

            if (result === false) {
                log.error(`[${this.name}]`, `New air instance for air '${air.name}' with N=${air.numRows} rows failed.`);
                throw new Error(`[${this.name}]`, `New air instance for air '${air.name}' with N=${air.numRows} rows failed.`);
            }
        
            this.createPolynomialTraces(airInstance, publics);
        }

        return;
    }

    createPolynomialTraces(airInstance, publics) {
        console.log('createPolynomialTraces (Basic Main)');
        const cols = airInstance.wtnsPols.Main;
        const N = airInstance.layout.numRows;

        console.log('== MAIN ===');
        console.log(N, Object.keys(cols));
        const processor = new Processor(cols, {romFile: path.join(__dirname, '..', 'rom/rom.json'), proofCtx: this.proofCtx});
        processor.execute(publics);
    }   
}