const { WitnessCalculatorComponent } = require("pil2-proofman/src/witness_calculator_component.js");

const log = require("pil2-proofman/logger.js");

const { setPol, getPol, getFixedPol, setSubproofValue } = require("pil2-stark-js/src/prover/prover_helpers.js");
const { getHintField } = require("pil2-stark-js/src/prover/hints_helpers.js");

module.exports = class Sum extends WitnessCalculatorComponent {
    constructor(wcManager, proofCtx) {
        super("STD Sum", wcManager, proofCtx);

        this.hints = undefined;
    }

    _witnessComputation(stageId, subproofId, airInstance, publics, hints) {
        if (!hints) {
            throw new Error(`[${this.name}]`, `Hints not found.`);
        }

        this.hints = hints;

        return this.witnessComputation(stageId, subproofId, airInstance, publics);
    }

    async witnessComputation(stageId, subproofId, airInstance, publics) {
        log.info(`[${this.name}]`, `Starting witness computation stage ${stageId}.`);

        if (stageId === 2) {
            const N = airInstance.layout.numRows;
            const F = this.proofCtx.F;

            for (let i = 0; i < this.hints.length; i++) {
                if (this.hints[i].name === "im_col") {
                    const imHint = this.hints[i];

                    const imIdx = getHintField(airInstance.ctx, imHint, "reference", true).id;
                    if (imIdx === -1) {
                        throw new Error(`[${this.name}]`, `Intermediate column ${i} not found in the columns map.`);
                    }

                    const numerator = getHintField(airInstance.ctx, imHint, "numerator");
                    const denominator = getHintField(airInstance.ctx, imHint, "denominator");

                    // Compute the intermediate columns
                    const tmpColIdx = airInstance.tmpPol.push(new Array(N)) - 1;
                    const im = airInstance.tmpPol[tmpColIdx];
                    for (let i = 0; i < N; i++) {
                        const num = Array.isArray(numerator) ? numerator[i] : numerator;
                        const den = Array.isArray(denominator) ? denominator[i] : denominator;
                        im[i] = F.div(num, den);
                    }

                    // TODO: Do the previous computation with the batch division component
                    // await this.wcManager.addNotification(this.name, "divLib", "div_batch", { instanceId: airInstance.instanceId, tmpColIdx }, true);

                    setPol(airInstance.ctx, imIdx, im, "n");

                } else if (this.hints[i].name === "gsum_col") {
                    const gsumHint = this.hints[i];

                    const gsumIdx = getHintField(airInstance.ctx, gsumHint, "reference", true).id;
                    const resultIdx = getHintField(airInstance.ctx, gsumHint, "result", true).id;

                    if (gsumIdx === -1) {
                        throw new Error(`[${this.name}]`, `Grand-sum column not found in the columns map.`);
                    } else if (resultIdx === -1) {
                        throw new Error(`[${this.name}]`, `Subproof value not found.`);
                    }

                    const expression = getHintField(airInstance.ctx, gsumHint, "expression");

                    // Compute gsum as gsum_0 = expr_0, gsum_i = gsum_{i-1} + expr_i
                    const tmpColIdx = airInstance.tmpPol.push(new Array(N)) - 1;
                    const gsum = airInstance.tmpPol[tmpColIdx];
                    gsum[0] = Array.isArray(expression) ? expression[0] : expression;
                    for (let i = 1; i < N; i++) {
                        gsum[i] = F.add(gsum[i - 1], Array.isArray(expression) ? expression[i] : expression);
                    }

                    // TODO: Do the previous computation with the batch division component
                    // await this.wcManager.addNotification(this.name, "divLib", "div_batch", { instanceId: airInstance.instanceId, tmpColIdx }, true);

                    setSubproofValue(airInstance.ctx, resultIdx, gsum[N - 1]);
                    setPol(airInstance.ctx, gsumIdx, gsum, "n");
                } else {
                    throw new Error(`[${this.name}]`, `Unknown hint ${this.hints[i].name}.`);
                }
            }

        }

        log.info(`[${this.name}       ]`, `Finishing witness computation stage ${stageId}.`);
        return;
    }
}