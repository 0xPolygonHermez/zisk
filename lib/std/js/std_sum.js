const { WitnessCalculatorComponent } = require("pil2-proofman/src/witness_calculator_component.js");

const log = require("pil2-proofman/logger.js");

const { setPol } = require("pil2-stark-js/src/prover/prover_helpers.js");

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
        const gsumIdx = getHintField(airInstance.ctx, this.hint, "reference");
        const numIdx = getHintField(airInstance.ctx, this.hint, "numerator");
        const denIdx = getHintField(airInstance.ctx, this.hint, "denominator");
        const resultIdx = getHintField(airInstance.ctx, this.hint, "result");

        if (gsumIdx === -1) {
            throw new Error(`[${this.name}]`, `Grand-sum column not found in the columns map.`);
        } else if (numIdx === -1) {
            throw new Error(`[${this.name}]`, `Numerator not specified.`);
        } else if (denIdx === -1) {
            throw new Error(`[${this.name}]`, `Denominator not specified.`);
        }

        const numRows = airInstance.layout.numRows;
        const F = this.proofCtx.F;

        // Compute gsum as gsum_0 = num_0/den_0, gprod_i = gprod_{i-1} * num_i/den_i
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

        // Calculate the grand-sum column
        const gsum = await this.computeGrandSumCol(airInstance);

        setPol(airInstance.ctx, colIdx, gsum, "n");
    }

    async computeMultiplicityCol(airInstance) {
        const subproof = this.proofCtx.airout.subproofs[airInstance.subproofId];
        const numRows = airInstance.layout.numRows;

        const tmpColIdx = airInstance.tmpPol.push(new Array(numRows)) - 1;
        for (let i = 0; i < numRows; i++) {
            airInstance.tmpPol[tmpColIdx][i] = 0; // TODO: Compute the multiplicity column
        }

        return airInstance.tmpPol[tmpColIdx];
    }

    async computeGrandSumCol(airInstance) {
        const subproof = this.proofCtx.airout.subproofs[airInstance.subproofId];
        const numRows = airInstance.layout.numRows;

        const MODULE_ID = 1n;
        const stageId = 2;
        const F = this.proofCtx.F;

        const std_alpha_airout = this.proofCtx.airout.getSymbolByName("std_alpha");
        const std_beta_airout = this.proofCtx.airout.getSymbolByName("std_beta");

        if(std_alpha_airout.stage !== stageId || std_beta_airout.stage !== stageId) {
            log.error(`[${this.name}]`, `std_alpha or std_beta not in stage ${stageId}.`);
            throw new Error(`[${this.name}]`, `std_alpha or std_beta not in stage ${stageId}.`);
        }

        const std_alpha = this.proofCtx.challenges[stageId - 1][std_alpha_airout.id];
        const std_beta = this.proofCtx.challenges[stageId - 1][std_beta_airout.id];

        const tmpColIdx = airInstance.tmpPol.push(new Array(airInstance.layout.numRows)) - 1;

        const assumes_or_proves = subproof.name === "Module" ? F.one : F.negone;

        // TODO: Replace this with evaluate expression
        if(subproof.name === "Module") {
            const polX = airInstance.wtnsPols.Module.x;
            const polX_mod = airInstance.wtnsPols.Module.x_mod;

            for (let i = 0; i < numRows; i++) {
                airInstance.tmpPol[tmpColIdx][i] = gsumitemModule(polX[i], polX_mod[i], std_alpha, std_beta, MODULE_ID);
            }
        } else if(subproof.name === "Fibonacci"){
            const polA = airInstance.wtnsPols.Fibonacci.a;
            const polB = airInstance.wtnsPols.Fibonacci.b;

            for (let i = 0; i < numRows; i++) {
                const isLast = i === numRows - 1;
                const nextIsLast = i + 1 === numRows - 1;
                const iPrime = isLast ? 0 : i + 1;

                airInstance.tmpPol[tmpColIdx][i] = gsumitemFibo(polA[i], polA[iPrime], polB[i], this.proofCtx.publics.out, std_alpha, std_beta, MODULE_ID, nextIsLast);
            }
        }

        await this.wcManager.addNotification(this.name, "divLib", "div_batch", { instanceId: airInstance.instanceId, tmpColIdx }, true);

        const result = airInstance.tmpPol[tmpColIdx];
        for (let i = 0; i < numRows; i++) {
            result[i] = F.mul(assumes_or_proves, result[i]);
            if(i!==0) result[i] = F.add(result[i], result[i-1]);
        }

        // TODO: Replace this with hint
        airInstance.ctx.subAirValues.push(result[numRows - 1]);

        return result;

        function gsumitemFibo(a, aprime, b, out, alpha, beta, MODULE_ID, isLast) {
            const t1 = MODULE_ID;
            const t2 = beta;
            const t3 = F.mul(alpha, F.add(F.square(a), F.square(b)));
            const t4 = F.mul(F.square(alpha), isLast ? out : aprime);

            return F.add(F.add(F.add(t1, t2), t3), t4);
        }

        function gsumitemModule(x, x_mod, alpha, beta, MODULE_ID) {
            const t1 = MODULE_ID;
            const t2 = beta;
            const t3 = F.mul(alpha, x);
            const t4 = F.mul(F.square(alpha), x_mod);

            return F.add(F.add(F.add(t1, t2), t3), t4);
        }
    }
}