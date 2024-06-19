const { WitnessCalculatorComponent } = require("pil2-proofman/src/witness_calculator_component.js");

const log = require("pil2-proofman/logger.js");

const { setPol } = require("pil2-stark-js/src/prover/prover_helpers.js");

const { getHintField } = require("./utils.js");

module.exports = class Prod extends WitnessCalculatorComponent {
    constructor(wcManager, proofCtx, hint) {
        super("prod", wcManager, proofCtx);

        this.hint = hint;
    }

    async witnessComputation(stageId, subproofId, airId, instanceId, publics) {
        console.log(`witnessComputation (STD Prod) STAGE(${stageId})`);

        const airInstance = this.proofCtx.getAirInstancesBySubproofIdAirId(subproofId,airId)[instanceId];
        const gprodIdx = getHintField(this.proofCtx, this.hint, "reference");

        console.log("HEY",airInstance.ctx.expressionsInfo.hintsInfo[0].fields);
        EXIT;
        // console.log("HEY",airInstance.ctx.tmp);
        // console.log(airInstance);

        if(gprodIdx === -1) {
            throw new Error(`[${this.name}]`, `Grand-product column not found in the columns map.`);
        }

        // Calculate the grand-product column
        const numIdx = getHintField(this.proofCtx, this.hint, "numerator");
        const denIdx = getHintField(this.proofCtx, this.hint, "denominator");

        const numRows = airInstance.layout.numRows;

        const F = this.proofCtx.F;

        // Compute gprod as gprod_0 = num_0/den_0, gprod_i = gprod_{i-1} * num_i/den_i
        const tmpColIdx = airInstance.tmpPol.push(new Array(airInstance.layout.numRows)) - 1;
        const gprod = airInstance.tmpPol[tmpColIdx];
        gprod[0] = F.div(this.calculateExpAtRow(airInstance, numIdx,0),this.calculateExpAtRow(airInstance, denIdx,0));
        for (let i = 1; i < numRows; i++) {
            gprod[i] = F.mul(
                gprod[i - 1],
                F.div(
                    this.calculateExpAtRow(airInstance, numIdx, i),
                    this.calculateExpAtRow(airInstance, denIdx, i)
                )
            );
        }

        // TODO: Do the previous computation with the batch division component
        // await this.wcManager.addNotification(this.name, "divLib", "div_batch", { instanceId: airInstance.instanceId, tmpColIdx }, true);

        airInstance.ctx.subAirValues.push(gprod[numRows - 1]);

        setPol(airInstance.ctx, gprodIdx, gprod, "n");
    }
}