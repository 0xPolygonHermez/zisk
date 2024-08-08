const path = require('path');

const { executeFullProveTest, checkConstraintsTest, generateSetupTest } = require("pil2-proofman/test/test_utils.js");

const basePath = path.join(__dirname, '.');
const libPath = path.join(basePath, '..', '..', '..', 'lib');

function getSettings() {
    return {
        name: "Lookup-" + Date.now(),
        airout: {
            airoutFilename: path.join(basePath, 'lookup.pilout'),
        },
        witnessCalculators: [
            { filename: path.join(basePath, 'executor.js'), settings: {}, sm: "Lookup" },
            { filename: path.join(libPath, 'std/js/std.js'), settings: {} },
        ],
        prover: {
            filename: "./src/lib/provers/stark_fri_prover.js",
        },
        setup: {
            settings: {
                default: { blowupFactor: 1, nQueries: 10, foldingFactor: 2, finalDegree: 2 },
            },
        },
        verifier: { filename: "./src/lib/provers/stark_fri_verifier.js", settings: {} },
    };

}

describe("Lookup tests", async function () {
    this.timeout(10000000);

    const options = {
        parallelExec: false,
        useThreads: true,
        vadcop: true,
    };

    const optionsVerifyConstraints = {...options, onlyCheck: true};

    let publics;
    let setup;
    let config;

    before(async () => {
        config = getSettings();
        setup = await generateSetupTest(config);
    });

    it("Verify the Lookup versatility", async () => {
        await checkConstraintsTest(setup, publics, optionsVerifyConstraints);
    });

    it.only("Generate a Lookup proof", async () => {
        await executeFullProveTest(setup, publics, options, config.aggregation?.genProof);
    });
});