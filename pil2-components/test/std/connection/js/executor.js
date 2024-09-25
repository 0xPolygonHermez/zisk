const { WitnessCalculatorComponent } = require('pil2-proofman/src/witness_calculator_component.js');

const log = require("pil2-proofman/logger.js");

module.exports = class RangeCheckTest extends WitnessCalculatorComponent {
    constructor(wcManager, proofCtx) {
        super("Range Check Test", wcManager, proofCtx);
    }

    async witnessComputation(stageId, subproofId, airInstance, publics) {
        log.info(`[${this.name}       ]`, `Starting witness computation stage ${stageId}.`);
        if (stageId === 1) {
            const instanceId = airInstance.instanceId;

            if (instanceId !== -1) {
                log.error(`[${this.name}]`, `Air instance id already existing in stageId 1.`);
                throw new Error(`[${this.name}]`, `Air instance id already existing in stageId 1.`);
            }

            airInstance.airId = 0; // TODO: This should be updated automatically

            const air = this.proofCtx.airout.subproofs[subproofId].airs[airInstance.airId];

            log.info(`[${this.name}]`, `Creating air instance for air '${air.name}' with N=${air.numRows} rows.`);
            let result = this.proofCtx.addAirInstance(subproofId, airInstance, air.numRows);

            if (result === false) {
                log.error(`[${this.name}]`, `Air instance for air '${air.name}' with N=${air.numRows} rows failed.`);
                throw new Error(`[${this.name}]`, `Air instance for air '${air.name}' with N=${air.numRows} rows failed.`);
            }

            this.#createPolynomialTraces(stageId, airInstance, publics);
        }

        log.info(`[${this.name}       ]`, `Finishing witness computation stage ${stageId}.`);
        return;
    }

    #createPolynomialTraces(stageId, airInstance, publics) {
        log.info(`[${this.name}]`, `Computing column traces stage ${stageId}.`);

        const F = this.proofCtx.F;

        const N = airInstance.layout.numRows;
        if (airInstance.wtnsPols.Connection1) {
            const a = airInstance.wtnsPols.Connection1.a;
            const b = airInstance.wtnsPols.Connection1.b;
            const c = airInstance.wtnsPols.Connection1.c;

            if (BigInt(N) === 2n**3n) {
                for (let i = 0; i < N; i++) {
                    a[i] = BigInt(i);
                    b[i] = BigInt(i);
                    c[i] = BigInt(i);
                }
            } else {
                throw new Error(`N=${N} is not supported for this test`);
            }
        } else if (airInstance.wtnsPols.Connection2) {
            const a = airInstance.wtnsPols.Connection2.a;
            const b = airInstance.wtnsPols.Connection2.b;
            const c = airInstance.wtnsPols.Connection2.c;

            if (BigInt(N) === 2n**4n) {
                for (let i = 0; i < N; i++) {
                    a[i] = BigInt(i);
                    if (i === 1) a[i] = a[0];
                    b[i] = BigInt(i);
                    c[i] = BigInt(i);
                }
            } else {
                throw new Error(`N=${N} is not supported for this test`);
            }
        } else if (airInstance.wtnsPols.Connection3) {
            const a = airInstance.wtnsPols.Connection3.a;
            const b = airInstance.wtnsPols.Connection3.b;
            const c = airInstance.wtnsPols.Connection3.c;

            if (BigInt(N) === 2n**12n) {
                for (let i = 0; i < N; i++) {
                    a[i] = BigInt(i);
                    b[i] = BigInt(i);
                    c[i] = BigInt(i);
                }
            } else {
                throw new Error(`N=${N} is not supported for this test`);
            }
        } else if (airInstance.wtnsPols.ConnectionNew) {
            const len = airInstance.wtnsPols.ConnectionNew.a.length;
            let a = new Array(len).fill(0n);
            let b = new Array(len).fill(0n);
            let c = new Array(len).fill(0n);
            let d = new Array(len).fill(0n);
            for (let i = 0; i < len; i++) {
                a[i] = airInstance.wtnsPols.ConnectionNew.a[i];
                b[i] = airInstance.wtnsPols.ConnectionNew.b[i];
                c[i] = airInstance.wtnsPols.ConnectionNew.c[i];
                d[i] = airInstance.wtnsPols.ConnectionNew.d[i];
            }

            for (let i = 0; i < len; i++) {
                if (i == 0) {
                    for (let j = 0; j < N; j++) {
                        a[i][j] = F.random()[0];
                        b[i][j] = F.random()[0];
                        c[i][j] = F.random()[0];
                    }
                } else if (i == 1) {
                    let frame = 0;
                    for (let j = 0; j < N; j++) {
                        a[i][j] = F.random()[0];
                        b[i][j] = F.random()[0];
                        c[i][j] = F.random()[0];
                        if (j == 3 + frame) {
                            c[i][j-1] = c[i][j];
                            frame += N/2;
                        }
                    }
                } else if (i == 2) {
                    let frame = 0;
                    let conn_len = 0;
                    for (let j = 0; j < N; j++) {
                        a[i][j] = F.random()[0];
                        b[i][j] = F.random()[0];
                        c[i][j] = F.random()[0];
                        if (j == 2 + frame) {
                            c[i][j-1] = a[i][j];
                            conn_len++;
                        }

                        if (j == 3 + frame) {
                            c[i][0 + frame] = b[i][j];
                            a[i][1 + frame] = b[i][j];
                            conn_len += 2;
                        }

                        if (conn_len == 3) {
                            frame += N/2;
                            conn_len = 0;
                        }
                    }
                } else if (i == 3) {
                    let frame = 0;
                    let conn_len = 0;
                    for (let j = 0; j < N; j++) {
                        a[i][j] = F.random()[0];
                        b[i][j] = F.random()[0];
                        c[i][j] = F.random()[0];
                        d[i][j] = F.random()[0];
                        if (j == 2 + frame) {
                            d[i][j-1] = b[i][j-1];
                            a[i][j-1] = c[i][j];
                            conn_len += 2;
                        }
                        if (j == 3 + frame) {
                            b[i][j-1] = a[i][j];
                            c[i][j] = a[i][j];
                            conn_len += 2;
                        }

                        if (conn_len == 4) {
                            frame += N/2;
                            conn_len = 0;
                        }
                    }
                } else if (i == 4) {
                    let frame = 0;
                    let conn_len = 0;
                    for (let j = 0; j < N; j++) {
                        a[i][j] = F.random()[0];
                        b[i][j] = F.random()[0];
                        c[i][j] = F.random()[0];
                        d[i][j] = F.random()[0];
                        if ((j == 2 + frame) || (j == 3 + frame)) {
                            d[i][j] = b[i][0 + frame];
                            conn_len++;
                        }

                        if (conn_len == 2) {
                            frame += N/2;
                            conn_len = 0;
                        }
                    }
                }
            }
        } else {
            throw new Error(`Connection not found in air instance`);
        }
    }
}