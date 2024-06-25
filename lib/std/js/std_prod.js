const { WitnessCalculatorComponent } = require("pil2-proofman/src/witness_calculator_component.js");

const log = require("pil2-proofman/logger.js");

const { getPol, getPolRef, setPol, setSubproofValue } = require("pil2-stark-js/src/prover/prover_helpers.js");

module.exports = class Prod extends WitnessCalculatorComponent {
    constructor(wcManager, proofCtx, hint) {
        super("STD Prod", wcManager, proofCtx);

        this.hint = hint;
    }

    async witnessComputation(stageId, subproofId, airInstance, publics) {
        log.info(`[${this.name}]`, `Starting witness computation stage ${stageId}.`);

        const gprodIdx = getHintField(airInstance.ctx, this.hint, "reference");
        const numIdx = getHintField(airInstance.ctx, this.hint, "numerator");
        const denIdx = getHintField(airInstance.ctx, this.hint, "denominator");
        const resultIdx = getHintField(airInstance.ctx, this.hint, "result");

        if (gprodIdx === -1) {
            throw new Error(`[${this.name}]`, `Grand-product column not found in the columns map.`);
        } else if (numIdx === -1) {
            throw new Error(`[${this.name}]`, `Numerator not specified.`);
        } else if (denIdx === -1) {
            throw new Error(`[${this.name}]`, `Denominator not specified.`);
        }

        // Calculate the grand-product column
        const numRows = airInstance.layout.numRows;

        const F = this.proofCtx.F;

        // Compute gprod as gprod_0 = num_0/den_0, gprod_i = gprod_{i-1} * num_i/den_i
        const tmpColIdx = airInstance.tmpPol.push(new Array(airInstance.layout.numRows)) - 1;
        const gprod = airInstance.tmpPol[tmpColIdx];
        gprod[0] = F.div(evaluateExpAtRow.call(this, airInstance, numIdx,0),evaluateExpAtRow.call(this, airInstance, denIdx,0));
        for (let i = 1; i < numRows; i++) {
            gprod[i] = F.mul(
                gprod[i - 1],
                F.div(
                    evaluateExpAtRow.call(this, airInstance, numIdx, i),
                    evaluateExpAtRow.call(this, airInstance, denIdx, i)
                )
            );
        }

        // TODO: Do the previous computation with the batch division component
        // await this.wcManager.addNotification(this.name, "divLib", "div_batch", { instanceId: airInstance.instanceId, tmpColIdx }, true);

        setSubproofValue(airInstance.ctx, resultIdx, gprod[numRows - 1]);
        setPol(airInstance.ctx, gprodIdx, gprod, "n");

        function evaluateExpAtRow(airInstance, expId, row) {
            if (typeof expId === "bigint") {
                return expId;
            }

            return this.calculateExpAtRow(airInstance, expId, row);
        }

        function getHintField(ctx, hint, field, dest = false) {
            const hintField = hint.fields.find(f => f.name === field);
            if(!hintField) throw new Error(`${field} field is missing`);

            if((hintField.op === "cm")) {
                if (dest) getPol(ctx, hintField.id, "n");
                return hintField.id;
            }

            if (hintField.op === "tmp") {
                if (dest) calculateExpression(ctx, hintField.expId);
                return hintField.expId;
            }
            if ((hintField.op === "number")) return BigInt(hintField.value);

            if (["subproofValue", "public"].includes(hintField.op)) return hintField.id;

            throw new Error("Case not considered");
        }
    }
}