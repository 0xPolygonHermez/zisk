const { WitnessCalculatorComponent } = require("pil2-proofman/src/witness_calculator_component.js");

const log = require("pil2-proofman/logger.js");

const { setPol, setSubproofValue } = require("pil2-stark-js/src/prover/prover_helpers.js");
const { getHintField } = require("pil2-stark-js/src/prover/hints_helpers.js");

module.exports = class Sum extends WitnessCalculatorComponent {
    constructor(wcManager, proofCtx) {
        super("STD Sum", wcManager, proofCtx);

        this.hint = undefined;
    }

    _witnessComputation(stageId, subproofId, airInstance, publics, hint) {
        if (!hint) {
            throw new Error(`[${this.name}]`, `Hint not found.`);
        }

        this.hint = hint;

        return this.witnessComputation(stageId, subproofId, airInstance, publics);
    }

    async witnessComputation(stageId, subproofId, airInstance, publics) {
        log.info(`[${this.name}]`, `Starting witness computation stage ${stageId}.`);

        // Here we compute the grand-sum column
        const gsumIdx = getHintField(airInstance.ctx, this.hint, "reference", true).id;
        const resultIdx = getHintField(airInstance.ctx, this.hint, "result", true).id;

        if (gsumIdx === -1) {
            throw new Error(`[${this.name}]`, `Grand-sum column not found in the columns map.`);
        } else if (resultIdx === -1) {
            throw new Error(`[${this.name}]`, `Subproof value not found.`);
        }

        const numerator = getHintField(airInstance.ctx, this.hint, "numerator");
        const denominator = getHintField(airInstance.ctx, this.hint, "denominator", false, true);
        console.log(this.hint);

        const numRows = airInstance.layout.numRows;
        const F = this.proofCtx.F;

        // Compute gsum as gsum_0 = num_0/den_0, gsum_i = gsum_{i-1} + num_i/den_i
        const tmpColIdx = airInstance.tmpPol.push(new Array(airInstance.layout.numRows)) - 1;
        const gsum = airInstance.tmpPol[tmpColIdx];
        console.log("NUM",numerator);
        console.log("DEN",denominator);
        console.log(0, Array.isArray(numerator) ? numerator[0] : numerator, Array.isArray(denominator) ? denominator[0] : denominator);
        gsum[0] = F.div(Array.isArray(numerator) ? numerator[0] : numerator, Array.isArray(denominator) ? denominator[0] : denominator);
        for (let i = 1; i < numRows; i++) {
            console.log(i, Array.isArray(numerator) ? numerator[i] : numerator, Array.isArray(denominator) ? denominator[i] : denominator);
            gsum[i] = F.add(
                gsum[i - 1],
                F.div(
                    Array.isArray(numerator) ? numerator[i] : numerator,
                    Array.isArray(denominator) ? denominator[i] : denominator
                )
            );
        }
        // extendWith;

        // TODO: Do the previous computation with the batch division component
        // await this.wcManager.addNotification(this.name, "divLib", "div_batch", { instanceId: airInstance.instanceId, tmpColIdx }, true);

        setSubproofValue(airInstance.ctx, resultIdx, gsum[numRows - 1]);
        setPol(airInstance.ctx, gsumIdx, gsum, "n");

        log.info(`[${this.name}       ]`, `Finishing witness computation stage ${stageId}.`);
        return;
    }
}