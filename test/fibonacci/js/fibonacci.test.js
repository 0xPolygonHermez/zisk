const path = require('path');
const fs = require('fs').promises;

const { executeFullProveTest, checkConstraintsTest, generateSetupTest } = require("../../../node_modules/pil2-proofman/test/test_utils.js");

const basePath = path.join(__dirname, '..');
const libPath = path.join(basePath, '..', '..', 'lib');

function getSettings() {
    return {
        name: "Fibonacci-Square-" + Date.now(),
        airout: {
            airoutFilename: path.join(basePath, 'pil/build.pilout'),
        },
        witnessCalculators: [
            { filename: path.join(basePath, 'js/executor_fibonaccisq.js'), settings: {}, sm: "FibonacciSquare" },
            { filename: path.join(basePath, 'js/executor_module.js'), settings: {}, sm: "Module" },
            { filename: path.join(libPath, 'std/js/std.js'), settings: {} },
        ],
        prover: {
            filename: "./src/lib/provers/stark_fri_prover.js",
            settings: {
                default: { starkStruct: path.join(__dirname,'stark_struct_2_10.json') },
                FibonacciSquare_4: { starkStruct: path.join(__dirname, 'stark_struct_2_4.json') },
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

describe("Fibonacci Vadcop", async function () {
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
        const publicsJSON = await fs.readFile(path.join(__dirname, 'publics.json'), 'utf8');
        publics = JSON.parse(publicsJSON);

        // We only need in1, in2 for this SM
        // The output is initially set to undefined and computed during the execution
        // TODO: They have to be in the same order as in the pilout, fix it!
        publics = [BigInt(publics.mod), BigInt(publics.in1), BigInt(publics.in2), undefined];

        config = getSettings();
        setup = await generateSetupTest(config);
    });

    it("Verify a Fibonacci Square constraints", async () => {
        await checkConstraintsTest(setup, publics, optionsVerifyConstraints);
    });

    it.only("Generate a Fibonacci Square proof", async () => {
        await executeFullProveTest(setup, publics, options, config.aggregation?.genProof);
    });
});