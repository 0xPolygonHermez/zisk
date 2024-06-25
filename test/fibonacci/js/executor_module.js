const { WitnessCalculatorComponent } = require("../../src/witness_calculator_component.js");

const log = require("../../logger.js");

class FibonacciVadcopModule extends WitnessCalculatorComponent {
  constructor(wcManager, proofCtx) {
    super("Module", wcManager, proofCtx);
    this.terminate = false;
  }

  async witnessComputation(stageId, subproofId, airId, instanceId, publics) {
    if (stageId === 1) {
      if (instanceId !== -1) {
        log.error(`[${this.name}]`, `Air instance id already existing in stageId 1.`);
        throw new Error(`[${this.name}]`, `Air instance id already existing in stageId 1.`);
      }

      while (!this.terminate) {
        let instanceData = await this.wcManager.receiveData(this, "Module");
        for (let i = 0; i < instanceData.length; i++) {
          this.processMessage(stageId, subproofId, airId, instanceId, publics, instanceData[i]);
        }
      }
    }

    return;
  }

  processMessage(stageId, subproofId, airId, instanceId, publics, instanceData) {
    if (instanceData.command && instanceData.command === "createInstances") {
      const air = this.proofCtx.airout.subproofs[subproofId].airs[instanceData.airId];

      log.info(
        `[${this.name}]`,
        `Creating air instance for air '${air.name}' with N=${air.numRows} rows.`
      );
      let { result, airInstance } = this.proofCtx.addAirInstance(
        subproofId,
        instanceData.airId,
        air.numRows
      );

      if (result === false) {
        log.error(
          `[${this.name}]`,
          `New air instance for air '${air.name}' with N=${air.numRows} rows failed.`
        );
        throw new Error(
          `[${this.name}]`,
          `New air instance for air '${air.name}' with N=${air.numRows} rows failed.`
        );
      }

      this.createPolynomialTraces(airInstance, publics);
      this.terminate = true;
    }
  }

  createPolynomialTraces(airInstance, publics) {
    const N = airInstance.layout.numRows;

    const polX = airInstance.wtnsPols.Module.x;
    const polQ = airInstance.wtnsPols.Module.q;
    const polX_mod = airInstance.wtnsPols.Module.x_mod;

    const mod = publics[0];
    let a = publics[1];
    let b = publics[2];

    for (let i = 0; i < N; i++) {
      polX[i] = a * a + b * b;

      polQ[i] = polX[i] / mod;
      polX_mod[i] = polX[i] % mod;

      b = a;
      a = polX_mod[i];
    }
  }
}

module.exports = FibonacciVadcopModule;