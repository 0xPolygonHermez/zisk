const path = require('path');

const { generateSetupTest } = require("pil2-proofman/test/test_utils.js");

function getSettings() {
    return {
        name: "Intermediate-Sums-" + Date.now(),
        airout: {
            airoutFilename: path.join(__dirname, 'intermediate_sums.pilout'),
        },
        prover: {
            filename: "./src/lib/provers/stark_fri_prover.js",
            settings: {
                default: { blowupFactor: 1 },
            },
        },
    };

}

async function main() {
    let config = getSettings();
    await generateSetupTest(config);
}

if (require.main === module) {
    main();
}