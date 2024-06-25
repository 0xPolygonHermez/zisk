const path = require('path');

const { executeFullProveTest, checkConstraintsTest, generateSetupTest } = require("../test_utils.js");

const publicInputs = [5n, 1n, 1n, undefined];

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
                Fibonacci_2: { starkStruct: path.join(__dirname, 'stark_struct_2_8.json') },
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

    let setup;

    let config;

    before(async () => {
        config = getSettings();
        setup = await generateSetupTest(config);
    });

    it("Verify a Fibonacci Square constraints", async () => {
        await checkConstraintsTest(setup, publicInputs, optionsVerifyConstraints);
    });

    // it.only("Generate a Fibonacci Square proof", async () => {
    //     await executeFullProveTest(setup, publicInputs, options, config.aggregation?.genProof);
    // });
});