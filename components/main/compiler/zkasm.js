#!/usr/bin/env node

const path = require("path");
const fs = require("fs");
const version = require("../package").version;
const compile = require("./compiler.js");

const argv = require("yargs")
    .version(version)
    .usage("zkasm <source.zkasm> -o <output.json> [-D name=value]")
    .alias("o", "output")
    .option('D', {
        alias: 'define',
        array: true
    })
    .options('t', { alias: 'set', type: 'array' })
    .argv;

async function run() {
    let inputFile;
    if (argv._.length > 1) {
        console.log("Only one source file at a time is permitted");
        process.exit(1);
    } else if (argv._.length == 1) {
        inputFile = argv._[0];
    } else  {
        console.log("You need to specify a source file");
        process.exit(1);
    }
    const fullFileName = path.resolve(process.cwd(), inputFile);
    const fileName = path.basename(fullFileName, ".zkasm");

    const outputFile = typeof(argv.output) === "string" ?  argv.output.trim() : fileName + ".json";

    const defines = [];
    if (argv.define) {
        argv.define.forEach((define) => {
            const [name, value] = define.trim().split('=');
            if (value.length > 1 && value.charAt(value.length-1) == 'n') {
                defines[name] = {value: BigInt(value.substr(0, -1)), type: 'CONSTL', line: false, fileName: false};
            } else {
                defines[name] = {value: BigInt(value), type: 'CONST', line: false, fileName: false};
            }
        });
    }
    let config = {defines};
    for (set of (argv.set ?? [])) {
        const index = set.indexOf('=');
        const name = index < 0 ? set : set.substr(0, index);
        let value = index < 0 ? true : set.substr(index+1);
        if (!isNaN(value) && value !== true) {
            const numValue = parseInt(value);
            const bigValue = BigInt(value);
            if ( bigValue === BigInt(numValue)) value = numValue;
            else value = BigInt(value);
        }
        config[name] = value;
    }
    const out = await compile(fullFileName, null, config);

    await fs.promises.writeFile(outputFile, JSON.stringify(out, null, 1) + "\n");
}

run().then(()=> {
    process.exit(0);
}, (err) => {
    console.error(err.message);
    process.exit(1);
});

