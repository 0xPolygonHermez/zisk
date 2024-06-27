const { executeFullProveTest, checkConstraintsTest, generateSetupTest } = require("../../node_modules/pil2-proofman/test/test_utils.js");

const publicInputs = [5n, 1n, 1n, undefined];

const path = require('path');
const basePath = path.join(__dirname, '..', '..');
const componentsPath = path.join(basePath, 'components');
const libPath = path.join(basePath, 'lib');


const originalMethod = console.log
const maxSourceRefLen = 20;
console.log = (...args) => {
    let initiator = false;
    try {
        throw new Error();
    } catch (e) {
    if (typeof e.stack === 'string') {
        let isFirst = true;
        for (const line of e.stack.split('\n')) {
        const matches = line.match(/^\s+at\s+.*\/([^\/:]*:[0-9]+:[0-9]+)\)/);
        if (matches) {
            if (!isFirst) { // first line - current function
                            // second line - caller (what we are looking for)
            initiator = matches[1];
            break;
            }
            isFirst = false;
        }
        }
    }
    }
    if (initiator === false) {
        originalMethod.apply(console, args);
    } else {
        initiator = initiator.split(':').slice(0,2).join(':').replace('.js','');
        initiator = initiator.length > maxSourceRefLen ? ('...' + initiator.substring(-maxSourceRefLen+3)) : initiator.padEnd(maxSourceRefLen);
        originalMethod.apply(console, [`\x1B[30;104m${initiator} \x1B[0m`, ...args]);
    }
}

function getSettings() {
    return {
        name: "Basic-vadcop-" + Date.now(),
        airout: {
            airoutFilename: path.join(componentsPath, 'basic/pil/basic.pilout'),
        },
        witnessCalculators: [ // TODO: The order seems important
            { filename: path.join(componentsPath, 'basic/js/executor_main.js'), settings: {}, sm: "Main" },
            { filename: path.join(componentsPath, 'basic/js/executor_rom.js'), settings: {}, sm: "Rom" },
            { filename: path.join(componentsPath, 'basic/js/executor_mem.js'), settings: {}, sm: "Mem" },
            { filename: path.join(libPath, 'std/js/std.js'), settings: {} },
            // { filename: path.join(libPath, 'std/js/div_lib.js'), settings: {}, }, // TODO: This one should not be imported
        ],
        prover: {
            filename: "./src/lib/provers/stark_fri_prover.js",
            settings: {
                default: { starkStruct: path.join(__dirname,'stark_struct_2_4.json') },
                Rom: {starkStruct: path.join(__dirname, 'stark_struct_2_8.json') },
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

describe("Basic Vadcop", async function () {
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

    it("Verify a Basic Vadcop constraints", async () => {
        await checkConstraintsTest(setup, publicInputs, optionsVerifyConstraints);
    });

    // it.only("Generate a Basic Vadcop proof", async () => {
    //     await executeFullProveTest(setup, publicInputs, options, config.aggregation?.genProof);
    // });
});
