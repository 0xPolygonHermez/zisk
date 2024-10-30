#ifndef GLOBAL_CONSTRAINTS_HPP
#define GLOBAL_CONSTRAINTS_HPP
#include "timer.hpp"
#include "goldilocks_base_field.hpp"
#include <nlohmann/json.hpp>

using json = nlohmann::json;

void calculateGlobalExpression(Goldilocks::Element* dest, Goldilocks::Element* publics, Goldilocks::Element* proofValues, Goldilocks::Element** airgroupValues, ParserArgs &parserArgs, ParserParams &parserParams) {

    uint8_t* ops = &parserArgs.ops[parserParams.opsOffset];
    uint16_t* args = &parserArgs.args[parserParams.argsOffset];

    uint64_t i_args = 0;

    Goldilocks::Element tmp1[parserParams.nTemp1];
    Goldilocks::Element tmp3[parserParams.nTemp3*FIELD_EXTENSION];

    Goldilocks::Element numbers_[parserArgs.nNumbers];
    for(uint64_t i = 0; i < parserArgs.nNumbers; ++i) {
        numbers_[i] = Goldilocks::fromU64(parserArgs.numbers[i]);
    }

    for (uint64_t kk = 0; kk < parserParams.nOps; ++kk) {
        switch (ops[kk]) {
            case 0: {
                // OPERATION WITH DEST: tmp1 - SRC0: tmp1 - SRC1: tmp1
                Goldilocks::op_pack(1, args[i_args], &tmp1[args[i_args + 1]], &tmp1[args[i_args + 2]], &tmp1[args[i_args + 3]]);
                i_args += 4;
                break;
            }
            case 1: {
                // OPERATION WITH DEST: tmp1 - SRC0: tmp1 - SRC1: public
                Goldilocks::op_pack(1, args[i_args], &tmp1[args[i_args + 1]], &tmp1[args[i_args + 2]], &publics[args[i_args + 3]]);
                i_args += 4;
                break;
            }
            case 2: {
                // OPERATION WITH DEST: tmp1 - SRC0: tmp1 - SRC1: number
                Goldilocks::op_pack(1, args[i_args], &tmp1[args[i_args + 1]], &tmp1[args[i_args + 2]], &numbers_[args[i_args + 3]]);
                i_args += 4;
                break;
            }
            case 3: {
                // OPERATION WITH DEST: tmp1 - SRC0: public - SRC1: public
                Goldilocks::op_pack(1, args[i_args], &tmp1[args[i_args + 1]], &publics[args[i_args + 2]], &publics[args[i_args + 3]]);
                i_args += 4;
                break;
            }
            case 4: {
                // OPERATION WITH DEST: tmp1 - SRC0: public - SRC1: number
                Goldilocks::op_pack(1, args[i_args], &tmp1[args[i_args + 1]], &publics[args[i_args + 2]], &numbers_[args[i_args + 3]]);
                i_args += 4;
                break;
            }
            case 5: {
                // OPERATION WITH DEST: tmp1 - SRC0: number - SRC1: number
                Goldilocks::op_pack(1, args[i_args], &tmp1[args[i_args + 1]], &numbers_[args[i_args + 2]], &numbers_[args[i_args + 3]]);
                i_args += 4;
                break;
            }
            case 6: {
                // OPERATION WITH DEST: tmp3 - SRC0: tmp3 - SRC1: tmp1
                Goldilocks3::op_31_pack(1, args[i_args], &tmp3[args[i_args + 1] * FIELD_EXTENSION], &tmp3[args[i_args + 2] * FIELD_EXTENSION], &tmp1[args[i_args + 3]]);
                i_args += 4;
                break;
            }
            case 7: {
                // OPERATION WITH DEST: tmp3 - SRC0: tmp3 - SRC1: public
                Goldilocks3::op_31_pack(1, args[i_args], &tmp3[args[i_args + 1] * FIELD_EXTENSION], &tmp3[args[i_args + 2] * FIELD_EXTENSION], &publics[args[i_args + 3]]);
                i_args += 4;
                break;
            }
            case 8: {
                // OPERATION WITH DEST: tmp3 - SRC0: tmp3 - SRC1: number
                Goldilocks3::op_31_pack(1, args[i_args], &tmp3[args[i_args + 1] * FIELD_EXTENSION], &tmp3[args[i_args + 2] * FIELD_EXTENSION], &numbers_[args[i_args + 3]]);
                i_args += 4;
                break;
            }
            case 9: {
                // OPERATION WITH DEST: tmp3 - SRC0: airgroupvalue - SRC1: tmp1
                Goldilocks3::op_31_pack(1, args[i_args], &tmp3[args[i_args + 1] * FIELD_EXTENSION], &airgroupValues[args[i_args + 2]][args[i_args + 3] * FIELD_EXTENSION], &tmp1[args[i_args + 4]]);
                i_args += 5;
                break;
            }
            case 10: {
                // OPERATION WITH DEST: tmp3 - SRC0: airgroupvalue - SRC1: public
                Goldilocks3::op_31_pack(1, args[i_args], &tmp3[args[i_args + 1] * FIELD_EXTENSION], &airgroupValues[args[i_args + 2]][args[i_args + 3] * FIELD_EXTENSION], &publics[args[i_args + 4]]);
                i_args += 5;
                break;
            }
            case 11: {
                // OPERATION WITH DEST: tmp3 - SRC0: airgroupvalue - SRC1: number
                Goldilocks3::op_31_pack(1, args[i_args], &tmp3[args[i_args + 1] * FIELD_EXTENSION], &airgroupValues[args[i_args + 2]][args[i_args + 3] * FIELD_EXTENSION], &numbers_[args[i_args + 4]]);
                i_args += 5;
                break;
            }
            case 12: {
                // OPERATION WITH DEST: tmp3 - SRC0: proofvalue - SRC1: tmp1
                Goldilocks3::op_31_pack(1, args[i_args], &tmp3[args[i_args + 1] * FIELD_EXTENSION], &proofValues[args[i_args + 2] * FIELD_EXTENSION], &tmp1[args[i_args + 3]]);
                i_args += 4;
                break;
            }
            case 13: {
                // OPERATION WITH DEST: tmp3 - SRC0: proofvalue - SRC1: public
                Goldilocks3::op_31_pack(1, args[i_args], &tmp3[args[i_args + 1] * FIELD_EXTENSION], &proofValues[args[i_args + 2] * FIELD_EXTENSION], &publics[args[i_args + 3]]);
                i_args += 4;
                break;
            }
            case 14: {
                // OPERATION WITH DEST: tmp3 - SRC0: proofvalue - SRC1: number
                Goldilocks3::op_31_pack(1, args[i_args], &tmp3[args[i_args + 1] * FIELD_EXTENSION], &proofValues[args[i_args + 2] * FIELD_EXTENSION], &numbers_[args[i_args + 3]]);
                i_args += 4;
                break;
            }
            case 15: {
                // OPERATION WITH DEST: tmp3 - SRC0: tmp3 - SRC1: tmp3
                Goldilocks3::op_pack(1, args[i_args], &tmp3[args[i_args + 1] * FIELD_EXTENSION], &tmp3[args[i_args + 2] * FIELD_EXTENSION], &tmp3[args[i_args + 3] * FIELD_EXTENSION]);
                i_args += 4;
                break;
            }
            case 16: {
                // OPERATION WITH DEST: tmp3 - SRC0: tmp3 - SRC1: airgroupvalue
                Goldilocks3::op_pack(1, args[i_args], &tmp3[args[i_args + 1] * FIELD_EXTENSION], &tmp3[args[i_args + 2] * FIELD_EXTENSION], &airgroupValues[args[i_args + 3]][args[i_args + 4] * FIELD_EXTENSION]);
                i_args += 5;
                break;
            }
            case 17: {
                // OPERATION WITH DEST: tmp3 - SRC0: tmp3 - SRC1: proofvalue
                Goldilocks3::op_pack(1, args[i_args], &tmp3[args[i_args + 1] * FIELD_EXTENSION], &tmp3[args[i_args + 2] * FIELD_EXTENSION], &proofValues[args[i_args + 3] * FIELD_EXTENSION]);
                i_args += 4;
                break;
            }
            case 18: {
                // OPERATION WITH DEST: tmp3 - SRC0: airgroupvalue - SRC1: airgroupvalue
                Goldilocks3::op_pack(1, args[i_args], &tmp3[args[i_args + 1] * FIELD_EXTENSION], &airgroupValues[args[i_args + 2]][args[i_args + 3] * FIELD_EXTENSION], &airgroupValues[args[i_args + 4]][args[i_args + 5] * FIELD_EXTENSION]);
                i_args += 6;
                break;
            }
            case 19: {
                // OPERATION WITH DEST: tmp3 - SRC0: airgroupvalue - SRC1: proofvalue
                Goldilocks3::op_pack(1, args[i_args], &tmp3[args[i_args + 1] * FIELD_EXTENSION], &airgroupValues[args[i_args + 2]][args[i_args + 3] * FIELD_EXTENSION], &proofValues[args[i_args + 4] * FIELD_EXTENSION]);
                i_args += 5;
                break;
            }
            case 20: {
                // OPERATION WITH DEST: tmp3 - SRC0: proofvalue - SRC1: proofvalue
                Goldilocks3::op_pack(1, args[i_args], &tmp3[args[i_args + 1] * FIELD_EXTENSION], &proofValues[args[i_args + 2] * FIELD_EXTENSION], &proofValues[args[i_args + 3] * FIELD_EXTENSION]);
                i_args += 4;
                break;
            }
            default: {
                std::cout << " Wrong operation!" << std::endl;
                exit(1);
            }
        }
    }

    if (i_args != parserParams.nArgs) std::cout << " " << i_args << " - " << parserParams.nArgs << std::endl;
    assert(i_args == parserParams.nArgs);

    if(parserParams.destDim == 1) {
        std::memcpy(dest, &tmp1[parserParams.destId], sizeof(Goldilocks::Element));
    } else if(parserParams.destDim == 3) {
        std::memcpy(dest, &tmp3[parserParams.destId * FIELD_EXTENSION], parserParams.destDim * sizeof(Goldilocks::Element));
    }
}


bool verifyGlobalConstraint(Goldilocks::Element* publics, Goldilocks::Element* proofValues, Goldilocks::Element** airgroupValues, ParserArgs &parserArgs, ParserParams &parserParams) {

    Goldilocks::Element dest[parserParams.destDim];

    calculateGlobalExpression(dest, publics, proofValues, airgroupValues, parserArgs, parserParams);

    bool isValidConstraint = true;
    if(parserParams.destDim == 1) {
        if(!Goldilocks::isZero(dest[0])) {
            cout << "Global constraint check failed with value: " << Goldilocks::toString(dest[0]) << endl;
            isValidConstraint = false;
        }
    } else {
        for(uint64_t i = 0; i < FIELD_EXTENSION; ++i) {
            if(!Goldilocks::isZero(dest[i])) {
                cout << "Global constraint check failed with value: [" << Goldilocks::toString(dest[0]) << ", " << Goldilocks::toString(dest[1]) << ", " << Goldilocks::toString(dest[2]) << "]" << endl;
                isValidConstraint = false;
                break;
            }
        }
    }

    if(isValidConstraint) {
        TimerLog(VALID_CONSTRAINT);
        return true;
    } else {
        TimerLog(INVALID_CONSTRAINT);
        return false;
    }
}

  
bool verifyGlobalConstraints(ExpressionsBin &globalConstraintsBin, Goldilocks::Element* publicInputs, Goldilocks::Element* proofValues, Goldilocks::Element** airgroupValues)
{

    std::vector<ParserParams> globalConstraintsInfo = globalConstraintsBin.constraintsInfoDebug;

    bool validGlobalConstraints = true;
    for(uint64_t i = 0; i < globalConstraintsInfo.size(); ++i) {
        TimerLog(CHECKING_CONSTRAINT);
        cout << "--------------------------------------------------------" << endl;
        cout << globalConstraintsInfo[i].line << endl;
        cout << "--------------------------------------------------------" << endl;
        if(!verifyGlobalConstraint(publicInputs, proofValues, airgroupValues, globalConstraintsBin.expressionsBinArgsConstraints, globalConstraintsInfo[i])) {
            validGlobalConstraints = false;
        };
    }

    return validGlobalConstraints;
}


HintFieldValues getHintFieldGlobalConstraint(ExpressionsBin &globalConstraintsBin, Goldilocks::Element* publicInputs, Goldilocks::Element* proofValues, Goldilocks::Element** airgroupValues, uint64_t hintId, std::string hintFieldName, bool print_expression) {
   

    if(globalConstraintsBin.hints.size() == 0) {
        zklog.error("No hints were found.");
        exitProcess();
        exit(-1);
    }

    Hint hint = globalConstraintsBin.hints[hintId];
    
    auto hintField = std::find_if(hint.fields.begin(), hint.fields.end(), [hintFieldName](const HintField& hintField) {
        return hintField.name == hintFieldName;
    });

    if(hintField == hint.fields.end()) {
        zklog.error("Hint field " + hintFieldName + " not found in hint " + hint.name + ".");
        exitProcess();
        exit(-1);
    }

    HintFieldValues hintFieldValues;
    hintFieldValues.nValues = hintField->values.size();
    hintFieldValues.values = new HintFieldInfo[hintField->values.size()];

    for(uint64_t i = 0; i < hintField->values.size(); ++i) {
        HintFieldValue hintFieldVal = hintField->values[i];
       
        HintFieldInfo hintFieldInfo;

        if(print_expression) {
            cout << "--------------------------------------------------------" << endl;
            cout << "Hint name " << hintFieldName << " for hint id " << hintId << " is ";
        }
        if (hintFieldVal.operand == opType::tmp) {
            uint64_t dim = globalConstraintsBin.expressionsInfo[hintFieldVal.id].destDim;
            hintFieldInfo.size = dim;
            hintFieldInfo.values = new Goldilocks::Element[hintFieldInfo.size];
            hintFieldInfo.fieldType = dim == 1 ? HintFieldType::Column : HintFieldType::ColumnExtended;
            hintFieldInfo.offset = dim;
            if(print_expression && globalConstraintsBin.expressionsInfo[hintFieldVal.id].line != "") {
                cout << "the expression with id: " << hintFieldVal.id << " " << globalConstraintsBin.expressionsInfo[hintFieldVal.id].line << endl;
            }
           
            calculateGlobalExpression(hintFieldInfo.values, publicInputs, proofValues, airgroupValues, globalConstraintsBin.expressionsBinArgsExpressions, globalConstraintsBin.expressionsInfo[hintFieldVal.id]);
        } else if (hintFieldVal.operand == opType::public_) {
            hintFieldInfo.size = 1;
            hintFieldInfo.values = new Goldilocks::Element[hintFieldInfo.size];
            hintFieldInfo.values[0] = publicInputs[hintFieldVal.id];
            hintFieldInfo.fieldType = HintFieldType::Field;
            hintFieldInfo.offset = 1;
        } else if (hintFieldVal.operand == opType::number) {
            hintFieldInfo.size = 1;
            hintFieldInfo.values = new Goldilocks::Element[hintFieldInfo.size];
            hintFieldInfo.values[0] = Goldilocks::fromU64(hintFieldVal.value);
            hintFieldInfo.fieldType = HintFieldType::Field;
            hintFieldInfo.offset = 1;
            if(print_expression) cout << "number " << hintFieldVal.value << endl;
        } else if (hintFieldVal.operand == opType::airgroupvalue) {
            hintFieldInfo.size = FIELD_EXTENSION;
            hintFieldInfo.values = new Goldilocks::Element[hintFieldInfo.size];
            hintFieldInfo.fieldType = HintFieldType::FieldExtended;
            hintFieldInfo.offset = FIELD_EXTENSION;
            std::memcpy(hintFieldInfo.values, &airgroupValues[hintFieldVal.dim][FIELD_EXTENSION*hintFieldVal.id], FIELD_EXTENSION * sizeof(Goldilocks::Element));
        } else if (hintFieldVal.operand == opType::proofvalue) {
            hintFieldInfo.size = FIELD_EXTENSION;
            hintFieldInfo.values = new Goldilocks::Element[hintFieldInfo.size];
            hintFieldInfo.values = new Goldilocks::Element[hintFieldInfo.size];
            hintFieldInfo.fieldType = HintFieldType::FieldExtended;
            hintFieldInfo.offset = FIELD_EXTENSION;
            std::memcpy(hintFieldInfo.values, &proofValues[FIELD_EXTENSION*hintFieldVal.id], FIELD_EXTENSION * sizeof(Goldilocks::Element));
        } else if (hintFieldVal.operand == opType::string_) {
            hintFieldInfo.values = nullptr;
            hintFieldInfo.fieldType = HintFieldType::String;
            hintFieldInfo.size = hintFieldVal.stringValue.size();
            hintFieldInfo.stringValue = new uint8_t[hintFieldVal.stringValue.size()];
            std::memcpy(hintFieldInfo.stringValue, hintFieldVal.stringValue.data(), hintFieldVal.stringValue.size());
            hintFieldInfo.offset = 0;
            if(print_expression) cout << "string " << hintFieldVal.stringValue << endl;
        } else {
            zklog.error("Unknown HintFieldType");
            exitProcess();
            exit(-1);
        }

        if(print_expression) cout << "--------------------------------------------------------" << endl;

        hintFieldInfo.matrix_size = hintFieldVal.pos.size();
        hintFieldInfo.pos = new uint64_t[hintFieldInfo.matrix_size];
        for(uint64_t i = 0; i < hintFieldInfo.matrix_size; ++i) {
            hintFieldInfo.pos[i] =  hintFieldVal.pos[i];
        }
        hintFieldValues.values[i] = hintFieldInfo;
    }
    
    return hintFieldValues;
}


uint64_t setHintFieldGlobalConstraint(ExpressionsBin &globalConstraintsBin, Goldilocks::Element* proofValues, Goldilocks::Element* values, uint64_t hintId, std::string hintFieldName) {
    Hint hint = globalConstraintsBin.hints[hintId];

    auto hintField = std::find_if(hint.fields.begin(), hint.fields.end(), [hintFieldName](const HintField& hintField) {
        return hintField.name == hintFieldName;
    });

    if(hintField == hint.fields.end()) {
        zklog.error("Hint field " + hintFieldName + " not found in hint " + hint.name + ".");
        exitProcess();
        exit(-1);
    }

    if(hintField->values.size() != 1) {
        zklog.error("Hint field " + hintFieldName + " in " + hint.name + "has more than one destination.");
        exitProcess();
        exit(-1);
    }

    auto hintFieldVal = hintField->values[0];
    if(hintFieldVal.operand == opType::proofvalue) {
        std::memcpy(&proofValues[FIELD_EXTENSION*hintFieldVal.id], values, FIELD_EXTENSION * sizeof(Goldilocks::Element));
    } else {
        zklog.error("Only committed pols and airgroupvalues can be set");
        exitProcess();
        exit(-1);  
    }

    return hintFieldVal.id;
}

#endif