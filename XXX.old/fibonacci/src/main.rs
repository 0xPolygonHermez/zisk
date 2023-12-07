use math::fields::f64::BaseElement;
use proofman::executor::Executor;

use std::sync::{Arc, Mutex};

mod fibonacci;
mod module;

use proofman::proof_orchestrator::Options;
use proofman::proof_orchestrator::ProofOrchestrator;

use log::info;

fn main() {
    env_logger::builder()
        .format_timestamp(None)
        .format_target(false)
        // .filter_level(LevelFilter::Debug)
        .init();

    // Load witness calculators
    let executor1 = fibonacci::Fibonacci::new();
    let executor2 = module::Module::new();

    let witness_calculators: Vec<Box<dyn Executor>> =
        vec![Box::new(executor2), Box::new(executor1)];

    info!("[FullProve ] {}", "==> FULL PROVE TEST");

    let mut proof_orchestrator = ProofOrchestrator::<BaseElement>::new();

    proof_orchestrator.initialize(
        "config",
        Options { use_threads: false },
        Arc::new(Mutex::new(witness_calculators)),
    );

    proof_orchestrator.generate_proof(/*setup, publics*/);

    // const { proofs, challenges, challengesFRISteps, subproofValues } = await proveCmd(setup, publics, options);

    // const tmpPath =  path.join(__dirname, "..", "tmp");
    // if(!fs.existsSync(tmpPath)) fs.mkdirSync(tmpPath);

    // for(const proof of proofs) {
    //     const name = proof.subproofId === 1 ? "fibonacci" : "module"
    //     let proofZkinFilename = path.join(tmpPath, name + ".proof.zkin.json");

    //     let starkInfoFilename = path.join(tmpPath, name + ".starkinfo.json");

    //     let verKeyFilename = path.join(tmpPath, name + ".verkey.json");

    //     const starkInfo = setup.setup[proof.subproofId][proof.airId].starkInfo;

    //     const constRoot = {constRoot: setup.setup[proof.subproofId][proof.airId].constRoot};

    //     const zkin = proof2zkin(proof.proof, setup.setup[proof.subproofId][proof.airId].starkInfo);
    //     zkin.publics = proof.publics;
    //     zkin.challenges = challenges.flat();
    //     zkin.challengesFRISteps = challengesFRISteps;

    //     await fs.promises.writeFile(proofZkinFilename, JSONbig.stringify(zkin, (k, v) => {
    //         if (typeof(v) === "bigint") {
    //             return v.toString();
    //         } else {
    //             return v;
    //         }
    //     }, 1), "utf8");

    //     await fs.promises.writeFile(starkInfoFilename, JSON.stringify(starkInfo, null, 1), "utf8");

    //     await fs.promises.writeFile(verKeyFilename, JSONbig.stringify(constRoot, null, 1), "utf8");
    // }

    // const isValid = await verifyCmd(setup, proofs, challenges, challengesFRISteps, subproofValues, options);

    // assert(isValid == true, "PROOF NOT VALID");

    // if(executeCircom) await verifyCircomCmd(setup, proofs, challenges, challengesFRISteps);
}
