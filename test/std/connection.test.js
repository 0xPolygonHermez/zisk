const path = require('path');

const { F1Field } = require("ffjavascript");
const { AirOut } = require("pil2-proofman/src/airout.js");

const Fp = new F1Field("0xFFFFFFFF00000001");

// List of generators of groups of size power of 2
const Fp_gen = [
    1n,18446744069414584320n,281474976710656n,18446744069397807105n,17293822564807737345n,70368744161280n,
    549755813888n,17870292113338400769n,13797081185216407910n,1803076106186727246n,11353340290879379826n,
    455906449640507599n,17492915097719143606n,1532612707718625687n,16207902636198568418n,17776499369601055404n,
    6115771955107415310n,12380578893860276750n,9306717745644682924n,18146160046829613826n,3511170319078647661n,
    17654865857378133588n,5416168637041100469n,16905767614792059275n,9713644485405565297n,5456943929260765144n,
    17096174751763063430n,1213594585890690845n,6414415596519834757n,16116352524544190054n,9123114210336311365n,
    4614640910117430873n,1753635133440165772n,
];

// 7^(2^32) => Generator of the group of size m = 3·5·17·257·65537
const Fp_k = 12275445934081160404n;

describe("Basic Vadcop", async function () {
    this.timeout("10s");

    it("Verify connection constraints", async () => {
        const airout = new AirOut(path.join(__dirname, "../../tmp/connection.pilout"));

        // console.log(airout.getHintsBySubproofIdAirId(0,0));

        console.log(airout.subproofs[0].airs[0].fixedCols[1]);

        for (hints of airout.getHintsBySubproofIdAirId(0,0)) {
            if (hints.name === "connections") {
                for (field of hints.hintFields[0].hintFieldArray.hintFields) {
                    console.log(`Field ${field.name}`);
                    console.log(field.operand);
                }
            }
        }

        const Connection2 = airout.subproofs.find((subproof) => subproof.name === "Connection2");
        const Connection3 = airout.subproofs.find((subproof) => subproof.name === "Connection3");

        // TODO: It's hard to compare against an object without a specific reference defined in the pil
        // for (const air of Connection2.airs) {
        //     for (const fixed of air.fixedCols) {
        //         const values = fixed.values.map((value) => Fp.fromRprBE(value));
        //         console.log(values);
        //     }
        // }
    });
});