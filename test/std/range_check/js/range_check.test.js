const path = require('path');

const { executeFullProveTest, checkConstraintsTest, generateSetupTest } = require("pil2-proofman/test/test_utils.js");

const basePath = path.join(__dirname, '.');
const libPath = path.join(basePath, '..', '..', '..', '..', 'lib');

function getSettings() {
    return {
        name: "Range-Check-" + Date.now(),
        airout: {
            airoutFilename: path.join(basePath, 'range_check.pilout'),
        },
        witnessCalculators: [
            { filename: path.join(basePath, 'executor.js'), settings: {}, sm: "RangeCheck1" },
            { filename: path.join(basePath, 'executor.js'), settings: {}, sm: "RangeCheck2" },
            { filename: path.join(basePath, 'executor.js'), settings: {}, sm: "RangeCheck3" },
            { filename: path.join(basePath, 'executor.js'), settings: {}, sm: "RangeCheck4" },
            { filename: path.join(libPath, 'std/js/std.js'), settings: {} },
        ],
        setup: {
            settings: {
                default: { blowupFactor: 2, nQueries: 10, foldingFactor: 2, finalDegree: 2 },
            },
        },
        prover: {
            filename: "./src/lib/provers/stark_fri_prover.js",
        },
        verifier: { filename: "./src/lib/provers/stark_fri_verifier.js", settings: {} },
    };

}

describe("Range Check tests", async function () {
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

    it("Verify the Range Check versatility", async () => {
        await checkConstraintsTest(setup, publics, optionsVerifyConstraints);
    });

    it.only("Generate a Range Check proof", async () => {
        await executeFullProveTest(setup, publics, options, config.aggregation?.genProof);
    });
});