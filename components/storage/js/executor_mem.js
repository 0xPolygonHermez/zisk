const { WitnessCalculatorComponent } = require('pil2-proofman/src/witness_calculator_component.js');
const log = require("pil2-proofman/logger.js");

module.exports = class BasicMem extends WitnessCalculatorComponent {
    constructor(wcManager, proofCtx) {
        super("Basic Mem Exe", wcManager, proofCtx);
    }

    async witnessComputation(stageId, subproofId, airId, instanceId, publics) {     
        console.log('witnessComputation (Basic Mem)');
        if(stageId === 1) {            
            if(instanceId !== -1) {
                log.error(`[${this.name}]`, `Air instance id already existing in stageId 1.`);
                throw new Error(`[${this.name}]`, `Air instance id already existing in stageId 1.`);
            }

            const instanceData = await this.wcManager.readData(this, "Mem.createInstances");
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
        const cols = airInstance.wtnsPols.Mem;
        console.log('createPolynomialTraces (Basic Mem)');

        console.log('== MEM ===');
        const memory = this.proofCtx.memory;
        memory.execute(cols);
/*
        console.log(this.proofCtx.memory);
        EXIT_HERE;
        console.log(N, Object.keys(cols));

        for (let row = 0; row < N; ++row) {
            cols.addr[row] = 0n;
            cols.step[row] = 0n;
            cols.sel[row] = 0n;
            for (let index = 0; index < 8; ++index) {
                cols.value[index][row] = 0n;
            }
            cols.wr[row] = 0n;
            cols.lastAccess[row] = 0n;
        }
        console.log(airInstance.wtnsPols.Mem.step[0]); */
    }   
}