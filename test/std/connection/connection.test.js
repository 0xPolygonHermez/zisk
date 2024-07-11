const path = require('path');

const { executeFullProveTest, checkConstraintsTest, generateSetupTest } = require("pil2-proofman/test/test_utils.js");

const basePath = path.join(__dirname, '.');
const libPath = path.join(basePath, '..', '..', '..', 'lib');

function getSettings() {
    return {
        name: "Fibonacci-Square-" + Date.now(),
        airout: {
            airoutFilename: path.join(basePath, 'connection.pilout'),
        },
        witnessCalculators: [
            { filename: path.join(basePath, 'executor.js'), settings: {}, sm: "Connection1" },
            { filename: path.join(basePath, 'executor.js'), settings: {}, sm: "Connection2" },
            { filename: path.join(basePath, 'executor.js'), settings: {}, sm: "Connection3" },
            { filename: path.join(basePath, 'executor.js'), settings: {}, sm: "ConnectionNew" },
            { filename: path.join(libPath, 'std/js/std.js'), settings: {} },
        ],
        prover: {
            filename: "./src/lib/provers/stark_fri_prover.js",
            settings: {
                default: { blowupFactor: 2, nQueries: 10, foldingFactor: 2, finalDegree: 2 },
            },
        },
        aggregation: {
            settings: {
                recursive: { starkStruct: "./src/recursion/configs/recursive.starkstruct.json" },
                final: { starkStruct: "./src/recursion/configs/final.starkstruct.json" }
            },
            genProof: false,
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

    it("Verify the Connection versatility", async () => {
        await checkConstraintsTest(setup, publics, optionsVerifyConstraints);
    });

    it.only("Generate a Connection proof", async () => {
        await executeFullProveTest(setup, publics, options, config.aggregation?.genProof);
    });
});