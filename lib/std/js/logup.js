const { WitnessCalculatorComponent } = require("pil2-proofman/src/witness_calculator_component.js");

const log = require("pil2-proofman/logger.js");

const { setPol } = require("pil2-stark-js/src/prover/prover_helpers.js");

module.exports = class LogUp extends WitnessCalculatorComponent {
    constructor(wcManager, proofCtx) {
        super("LogUp WC  ", wcManager, proofCtx);
    }

    async witnessComputation(stageId, subproofId, airId, instanceId, publics) {
        console.log(`################################ logup.js STAGE ${stageId} SUBPROOF ${subproofId} AIR ${airId} INSTANCEID ${instanceId}`);
        if(stageId === 2) {
            const airInstance = this.proofCtx.airInstances[instanceId];
            const subproof = this.proofCtx.airout.subproofs[subproofId];
            const gsumPolName = subproof.name + ".gsum";
            const polIdx = airInstance.ctx.pilInfo.cmPolsMap.findIndex(c => c.name === gsumPolName);

            if(polIdx === -1) {
                log.error(`[${this.name}]`, `Polynomial ${gsumPolName} not found in the polynomials map.`);
                throw new Error(`[${this.name}]`, `Polynomial ${gsumPolName} not found in the polynomials map.`);
            }

            // Calculate the gsum polynomial
            const gsum = await this.createGsumTrace(airInstance);

            setPol(airInstance.ctx, polIdx, gsum, "n");
        }
    }

    async createGsumTrace(airInstance) {
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

        const tmpPolIdx = airInstance.tmpPol.push(new Array(airInstance.layout.numRows)) - 1;

        const assumes_or_proves = subproof.name === "Module" ? F.one : F.negone;

        // TODO: Replace this with evaluate expression
        if(subproof.name === "Module") {
            const polX = airInstance.wtnsPols.Module.x;
            const polX_mod = airInstance.wtnsPols.Module.x_mod;

            for (let i = 0; i < numRows; i++) {
                airInstance.tmpPol[tmpPolIdx][i] = gsumitemModule(polX[i], polX_mod[i], std_alpha, std_beta, MODULE_ID);
            }
        } else if(subproof.name === "Fibonacci"){    
            const polA = airInstance.wtnsPols.Fibonacci.a;
            const polB = airInstance.wtnsPols.Fibonacci.b;
    
            for (let i = 0; i < numRows; i++) {
                const isLast = i === numRows - 1;
                const nextIsLast = i + 1 === numRows - 1;
                const iPrime = isLast ? 0 : i + 1;
    
                airInstance.tmpPol[tmpPolIdx][i] = gsumitemFibo(polA[i], polA[iPrime], polB[i], this.proofCtx.publics.out, std_alpha, std_beta, MODULE_ID, nextIsLast);
            }    
        }

        await this.wcManager.addNotification(this.name, "divLib", "div_batch", { instanceId: airInstance.instanceId, tmpPolIdx }, true);
        
        const result = airInstance.tmpPol[tmpPolIdx];
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
