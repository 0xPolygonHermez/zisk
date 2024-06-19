function getHintField(ctx, hint, field, dest = false) {
    // Helper function to search through hint fields recursively
    function searchFields(fields) {
        for (const f of fields) {
            if (f.name && f.name === field) {
                return f.operand;
            }
            if (f.hintFieldArray) {
                const nestedOperand = searchFields(f.hintFieldArray.hintFields);
                if (nestedOperand !== null) {
                    return nestedOperand;
                }
            }
        }
        return null;
    }

    const result = searchFields(hint.hintFields);

    if (result.expression) {
        return result.expression.idx;
    } else {
        throw new Error("Case not considered");
    }

    // oneof operand {
    //     Constant constant = 1;
    //     Challenge challenge = 2;
    //     ProofValue proofValue = 3;
    //     SubproofValue subproofValue = 4;
    //     PublicValue publicValue = 5;
    //     PeriodicCol periodicCol = 6;
    //     FixedCol fixedCol = 7;
    //     WitnessCol witnessCol = 8;
    //     Expression expression = 9;
    // }

    console.log(hintField);

    if (!hintField) throw new Error(`Field "${field}" is missing in hint ${hint.name}`);

    if (hintField.operand === "expression") {

    }

    // TODO: Old code, correct
    // if (hintField.operand === "cm" && dest) return hintField;

    // if(["cm", "tmp"].includes(hintField.operand)) {
    //     return getPol(ctx, hintField.id, "n");
    // }

    // if (["number"].includes(hintField.operand)) return BigInt(hintField.value);

    // if (["subproofValue", "public"].includes(hintField.operand)) return hintField;

    throw new Error("Case not considered");
}

module.exports = { getHintField };
