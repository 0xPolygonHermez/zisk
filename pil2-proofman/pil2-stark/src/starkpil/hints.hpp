#include "expressions_ctx.hpp"

typedef enum {
    Field = 0,
    FieldExtended = 1,
    Column = 2,
    ColumnExtended = 3,
    String = 4,
} HintFieldType;

struct HintFieldInfo {
    uint64_t size;
    uint64_t string_size;
    uint8_t offset;
    HintFieldType fieldType;
    Goldilocks::Element* values;
    uint8_t* stringValue;
    uint64_t matrix_size;
    uint64_t* pos;
    uint8_t* expression_line;
    uint64_t expression_line_size;
};

struct HintFieldArgs {
    std::string name;
    bool inverse = false;  
};

struct HintFieldOptions {
    bool dest = false;
    bool inverse = false;
    bool print_expression = false;
    bool initialize_zeros = false;
    bool compilation_time = false;
};


void getPolynomial(SetupCtx& setupCtx, Goldilocks::Element *buffer, Goldilocks::Element *dest, PolMap& polInfo, uint64_t rowOffsetIndex, string type) {
    uint64_t deg = 1 << setupCtx.starkInfo.starkStruct.nBits;
    uint64_t dim = polInfo.dim;
    std::string stage = type == "cm" ? "cm" + to_string(polInfo.stage) : type == "custom" ? setupCtx.starkInfo.customCommits[polInfo.commitId].name + "0" : "const";
    uint64_t nCols = setupCtx.starkInfo.mapSectionsN[stage];
    uint64_t offset = setupCtx.starkInfo.mapOffsets[std::make_pair(stage, false)];
    offset += polInfo.stagePos;
    Polinomial pol = Polinomial(&buffer[offset], deg, dim, nCols);
    uint64_t rowOffset = setupCtx.starkInfo.openingPoints[rowOffsetIndex];
#pragma omp parallel for
    for(uint64_t j = 0; j < deg; ++j) {
        std::memcpy(&dest[j*dim], pol[(j + rowOffset)%deg], dim * sizeof(Goldilocks::Element));
    }
}

void setPolynomial(SetupCtx& setupCtx, Goldilocks::Element *buffer, Goldilocks::Element *values, uint64_t idPol) {
    PolMap polInfo = setupCtx.starkInfo.cmPolsMap[idPol];
    uint64_t deg = 1 << setupCtx.starkInfo.starkStruct.nBits;
    uint64_t dim = polInfo.dim;
    std::string stage = "cm" + to_string(polInfo.stage);
    uint64_t nCols = setupCtx.starkInfo.mapSectionsN[stage];
    uint64_t offset = setupCtx.starkInfo.mapOffsets[std::make_pair(stage, false)];
    offset += polInfo.stagePos;
    Polinomial pol = Polinomial(&buffer[offset], deg, dim, nCols, std::to_string(idPol));
#pragma omp parallel for
    for(uint64_t j = 0; j < deg; ++j) {
        std::memcpy(pol[j], &values[j*dim], dim * sizeof(Goldilocks::Element));
    }
}

void printRow(SetupCtx& setupCtx, Goldilocks::Element* buffer, uint64_t stage, uint64_t row) {
    Goldilocks::Element *pol = &buffer[setupCtx.starkInfo.mapOffsets[std::make_pair("cm" + to_string(stage), false)] + setupCtx.starkInfo.mapSectionsN["cm" + to_string(stage)] * row];
    cout << "Values at row " << row << " = {" << endl;
    bool first = true;
    for(uint64_t i = 0; i < setupCtx.starkInfo.cmPolsMap.size(); ++i) {
        PolMap cmPol = setupCtx.starkInfo.cmPolsMap[i];
        if(cmPol.stage == stage) {
            if(first) {
                first = false;
            } else {
                cout << endl;
            }
            cout << "    " << cmPol.name;
            if(cmPol.lengths.size() > 0) {
                cout << "[";
                for(uint64_t i = 0; i < cmPol.lengths.size(); ++i) {
                    cout << cmPol.lengths[i];
                    if(i != cmPol.lengths.size() - 1) cout << ", ";
                }
                cout << "]";
            }
            cout << ": ";
            if(cmPol.dim == 1) {
                cout << Goldilocks::toString(pol[cmPol.stagePos]) << ",";
            } else {
                cout << "[" << Goldilocks::toString(pol[cmPol.stagePos]) << ", " << Goldilocks::toString(pol[cmPol.stagePos + 1]) << ", " << Goldilocks::toString(pol[cmPol.stagePos + 2]) << "],";
            }
        }
    }
    cout << endl;
    cout << "}" << endl;
}

std::string getExpressionDebug(SetupCtx& setupCtx, uint64_t hintId, std::string hintFieldName, HintFieldValue hintFieldVal) {
    std::string debug = "Hint name " + hintFieldName + " for hint id " + to_string(hintId) + " is ";
    
    if(hintFieldVal.operand == opType::cm) {
        debug += "witness col " + setupCtx.starkInfo.cmPolsMap[hintFieldVal.id].name;
        if(setupCtx.starkInfo.cmPolsMap[hintFieldVal.id].lengths.size() > 0) {
            debug +=  "[";
            for(uint64_t i = 0; i < setupCtx.starkInfo.cmPolsMap[hintFieldVal.id].lengths.size(); ++i) {
                debug += to_string(setupCtx.starkInfo.cmPolsMap[hintFieldVal.id].lengths[i]);
                if(i != setupCtx.starkInfo.cmPolsMap[hintFieldVal.id].lengths.size() - 1) debug += ", ";
            }
            debug += "]";
        }
    } else if(hintFieldVal.operand == opType::custom) {
        debug += "custom col " + setupCtx.starkInfo.customCommitsMap[hintFieldVal.commitId][hintFieldVal.id].name;
        if(setupCtx.starkInfo.customCommitsMap[hintFieldVal.commitId][hintFieldVal.id].lengths.size() > 0) {
            debug += "[";
            for(uint64_t i = 0; i < setupCtx.starkInfo.customCommitsMap[hintFieldVal.commitId][hintFieldVal.id].lengths.size(); ++i) {
                debug += to_string(setupCtx.starkInfo.customCommitsMap[hintFieldVal.commitId][hintFieldVal.id].lengths[i]);
                if(i != setupCtx.starkInfo.customCommitsMap[hintFieldVal.commitId][hintFieldVal.id].lengths.size() - 1) debug += ", ";
            }
            debug += "]";
        }
    } else if(hintFieldVal.operand == opType::const_) {
        debug += "fixed col" + setupCtx.starkInfo.constPolsMap[hintFieldVal.id].name;
        if(setupCtx.starkInfo.constPolsMap[hintFieldVal.id].lengths.size() > 0) {
            debug += "[";
            for(uint64_t i = 0; i < setupCtx.starkInfo.constPolsMap[hintFieldVal.id].lengths.size(); ++i) {
                debug += to_string(setupCtx.starkInfo.constPolsMap[hintFieldVal.id].lengths[i]);
                if(i != setupCtx.starkInfo.constPolsMap[hintFieldVal.id].lengths.size() - 1) debug += ", ";
            }
            debug += "]";
        }
    } else if (hintFieldVal.operand == opType::tmp) {
        debug += "the expression with id: ";
        if(setupCtx.expressionsBin.expressionsInfo[hintFieldVal.id].line != "") {
            debug += " " + setupCtx.expressionsBin.expressionsInfo[hintFieldVal.id].line;
        }
    } else if (hintFieldVal.operand == opType::public_) {
        debug += "public input " + setupCtx.starkInfo.publicsMap[hintFieldVal.id].name;
    } else if (hintFieldVal.operand == opType::proofvalue) {
        debug += "proof value  " + setupCtx.starkInfo.proofValuesMap[hintFieldVal.id].name;
    } else if (hintFieldVal.operand == opType::number) {
        debug += "number " + to_string(hintFieldVal.value);
    } else if (hintFieldVal.operand == opType::airgroupvalue) {
        debug += "airgroupValue " + setupCtx.starkInfo.airgroupValuesMap[hintFieldVal.id].name;
    } else if (hintFieldVal.operand == opType::airvalue) {
        debug += "airValue " + setupCtx.starkInfo.airValuesMap[hintFieldVal.id].name;
    } else if (hintFieldVal.operand == opType::challenge) {
        debug += "challenge " + setupCtx.starkInfo.challengesMap[hintFieldVal.id].name;
    } else if (hintFieldVal.operand == opType::string_) {
        debug += "string " + hintFieldVal.stringValue;
    } else {
        zklog.error("Unknown HintFieldType");
        exitProcess();
        exit(-1);
    }

    return debug;
}

uint64_t getHintFieldValues(SetupCtx& setupCtx, uint64_t hintId, std::string hintFieldName) {
     Hint hint = setupCtx.expressionsBin.hints[hintId];
    
    auto hintField = std::find_if(hint.fields.begin(), hint.fields.end(), [hintFieldName](const HintField& hintField) {
        return hintField.name == hintFieldName;
    });

    if(hintField == hint.fields.end()) {
        zklog.error("Hint field " + hintFieldName + " not found in hint " + hint.name + ".");
        exitProcess();
        exit(-1);
    }

    return hintField->values.size();
}

void getHintFieldSizes(
    SetupCtx& setupCtx, 
    HintFieldInfo *hintFieldValues,
    uint64_t hintId, 
    std::string hintFieldName,
    HintFieldOptions& hintOptions
) {

    uint64_t deg = 1 << setupCtx.starkInfo.starkStruct.nBits;

    if(setupCtx.expressionsBin.hints.size() == 0) {
        zklog.error("No hints were found.");
        exitProcess();
        exit(-1);
    }

    Hint hint = setupCtx.expressionsBin.hints[hintId];
    
    auto hintField = std::find_if(hint.fields.begin(), hint.fields.end(), [hintFieldName](const HintField& hintField) {
        return hintField.name == hintFieldName;
    });

    if(hintField == hint.fields.end()) {
        zklog.error("Hint field " + hintFieldName + " not found in hint " + hint.name + ".");
        exitProcess();
        exit(-1);
    }

    for(uint64_t i = 0; i < hintField->values.size(); ++i) {
        HintFieldValue hintFieldVal = hintField->values[i];

        if(hintOptions.print_expression) {
            std::string expression_line = getExpressionDebug(setupCtx, hintId, hintFieldName, hintFieldVal);
            hintFieldValues[i].expression_line_size = expression_line.size();
        }
        if(hintFieldVal.operand == opType::cm) {
            uint64_t dim = setupCtx.starkInfo.cmPolsMap[hintFieldVal.id].dim;
            hintFieldValues[i].size = deg*dim;
            hintFieldValues[i].fieldType = dim == 1 ? HintFieldType::Column : HintFieldType::ColumnExtended;
            hintFieldValues[i].offset = dim;
        } else if(hintFieldVal.operand == opType::custom) {
            uint64_t dim = setupCtx.starkInfo.customCommitsMap[hintFieldVal.commitId][hintFieldVal.id].dim;
            hintFieldValues[i].size = deg*dim;
            hintFieldValues[i].fieldType = dim == 1 ? HintFieldType::Column : HintFieldType::ColumnExtended;
            hintFieldValues[i].offset = dim;
        } else if(hintFieldVal.operand == opType::const_) {
            uint64_t dim = setupCtx.starkInfo.constPolsMap[hintFieldVal.id].dim;
            hintFieldValues[i].size = deg*dim;
            hintFieldValues[i].fieldType = dim == 1 ? HintFieldType::Column : HintFieldType::ColumnExtended;
            hintFieldValues[i].offset = dim;
        } else if (hintFieldVal.operand == opType::tmp) {
            if(hintOptions.compilation_time) {
                hintFieldValues[i].size = 1;
                hintFieldValues[i].fieldType = HintFieldType::Field;
                hintFieldValues[i].offset = 1;
            } else {
                uint64_t dim = setupCtx.expressionsBin.expressionsInfo[hintFieldVal.id].destDim;
                hintFieldValues[i].size = deg*dim;
                hintFieldValues[i].fieldType = dim == 1 ? HintFieldType::Column : HintFieldType::ColumnExtended;
                hintFieldValues[i].offset = dim;
            }
        } else if (hintFieldVal.operand == opType::public_) {
            hintFieldValues[i].size = 1;
            hintFieldValues[i].fieldType = HintFieldType::Field;
            hintFieldValues[i].offset = 1;
        } else if (hintFieldVal.operand == opType::number) {
            hintFieldValues[i].size = 1;
            hintFieldValues[i].fieldType = HintFieldType::Field;
            hintFieldValues[i].offset = 1;
        } else if (hintFieldVal.operand == opType::airgroupvalue) {
            uint64_t dim = setupCtx.starkInfo.airgroupValuesMap[hintFieldVal.id].stage == 1 ? 1 : FIELD_EXTENSION;
            hintFieldValues[i].size = dim;
            hintFieldValues[i].fieldType = dim == 1 ? HintFieldType::Field : HintFieldType::FieldExtended;
            hintFieldValues[i].offset = dim;
        } else if (hintFieldVal.operand == opType::airvalue) {
            uint64_t dim = setupCtx.starkInfo.airValuesMap[hintFieldVal.id].stage == 1 ? 1 : FIELD_EXTENSION;
            hintFieldValues[i].size = dim;
            hintFieldValues[i].fieldType = dim == 1 ? HintFieldType::Field : HintFieldType::FieldExtended;
            hintFieldValues[i].offset = dim;
        } else if (hintFieldVal.operand == opType::proofvalue) {
            uint64_t dim = setupCtx.starkInfo.proofValuesMap[hintFieldVal.id].stage == 1 ? 1 : FIELD_EXTENSION;
            hintFieldValues[i].size = dim;
            hintFieldValues[i].fieldType = dim == 1 ? HintFieldType::Field : HintFieldType::FieldExtended;
            hintFieldValues[i].offset = dim;
        } else if (hintFieldVal.operand == opType::challenge) {
            hintFieldValues[i].size = FIELD_EXTENSION;
            hintFieldValues[i].fieldType = HintFieldType::FieldExtended;
            hintFieldValues[i].offset = FIELD_EXTENSION;
        } else if (hintFieldVal.operand == opType::string_) {
            hintFieldValues[i].string_size = hintFieldVal.stringValue.size();
            hintFieldValues[i].fieldType = HintFieldType::String;
            hintFieldValues[i].offset = 0;
        } else {
            zklog.error("Unknown HintFieldType");
            exitProcess();
            exit(-1);
        }

        hintFieldValues[i].matrix_size = hintFieldVal.pos.size();
    }
    
    return;
}

void getHintField(
    SetupCtx& setupCtx, 
    StepsParams &params,
    HintFieldInfo *hintFieldValues,
    uint64_t hintId, 
    std::string hintFieldName, 
    HintFieldOptions& hintOptions
) {

    if(setupCtx.expressionsBin.hints.size() == 0) {
        zklog.error("No hints were found.");
        exitProcess();
        exit(-1);
    }

    Hint hint = setupCtx.expressionsBin.hints[hintId];
    
    auto hintField = std::find_if(hint.fields.begin(), hint.fields.end(), [hintFieldName](const HintField& hintField) {
        return hintField.name == hintFieldName;
    });

    if(hintField == hint.fields.end()) {
        zklog.error("Hint field " + hintFieldName + " not found in hint " + hint.name + ".");
        exitProcess();
        exit(-1);
    }

    for(uint64_t i = 0; i < hintField->values.size(); ++i) {
        HintFieldValue hintFieldVal = hintField->values[i];
        if(hintOptions.dest && (hintFieldVal.operand != opType::cm && hintFieldVal.operand != opType::airgroupvalue && hintFieldVal.operand != opType::airvalue)) {
            zklog.error("Invalid destination.");
            exitProcess();
            exit(-1);
        }

        HintFieldInfo hintFieldInfo = hintFieldValues[i];

        if(hintOptions.print_expression) {
            std::string expression_line = getExpressionDebug(setupCtx, hintId, hintFieldName, hintFieldVal);
            std::memcpy(hintFieldInfo.expression_line, expression_line.data(), expression_line.size());
            hintFieldInfo.expression_line_size = expression_line.size();
        }
        if(hintFieldVal.operand == opType::cm) {
            if(!hintOptions.dest) {
                Goldilocks::Element *pAddress = setupCtx.starkInfo.cmPolsMap[hintFieldVal.id].stage == 1 ? params.trace : params.aux_trace;
                getPolynomial(setupCtx, pAddress, hintFieldInfo.values, setupCtx.starkInfo.cmPolsMap[hintFieldVal.id], hintFieldVal.rowOffsetIndex, "cm");
                if(hintOptions.inverse) {
                    zklog.error("Inverse not supported still for polynomials");
                    exitProcess();
                }
            } else if(hintOptions.initialize_zeros) {
                memset((uint8_t *)hintFieldInfo.values, 0, hintFieldInfo.size * sizeof(Goldilocks::Element));
            }
        } else if(hintFieldVal.operand == opType::custom) {
            getPolynomial(setupCtx, params.pCustomCommits[hintFieldVal.commitId], hintFieldInfo.values, setupCtx.starkInfo.customCommitsMap[hintFieldVal.commitId][hintFieldVal.id], hintFieldVal.rowOffsetIndex, "custom");
            if(hintOptions.inverse) {
                zklog.error("Inverse not supported still for polynomials");
                exitProcess();
            }
        } else if(hintFieldVal.operand == opType::const_) {
            getPolynomial(setupCtx, params.pConstPolsAddress, hintFieldInfo.values, setupCtx.starkInfo.constPolsMap[hintFieldVal.id], hintFieldVal.rowOffsetIndex, "const");
            if(hintOptions.inverse) {
                zklog.error("Inverse not supported still for polynomials");
                exitProcess();
            }
        } else if (hintFieldVal.operand == opType::tmp) {
            if(hintOptions.compilation_time) {
                ExpressionsPack expressionsCtx(setupCtx, 1);
                expressionsCtx.calculateExpression(params, hintFieldInfo.values, hintFieldVal.id, hintOptions.inverse, true);
            } else {
#ifdef __AVX512__
    ExpressionsAvx512 expressionsCtx(setupCtx);
#elif defined(__AVX2__)
    ExpressionsAvx expressionsCtx(setupCtx);
#else
    ExpressionsPack expressionsCtx(setupCtx);
#endif
                expressionsCtx.calculateExpression(params, hintFieldInfo.values, hintFieldVal.id, hintOptions.inverse, false);
            }
        } else if (hintFieldVal.operand == opType::public_) {
            hintFieldInfo.values[0] = hintOptions.inverse ? Goldilocks::inv(params.publicInputs[hintFieldVal.id]) : params.publicInputs[hintFieldVal.id];
        } else if (hintFieldVal.operand == opType::number) {
            hintFieldInfo.values[0] = hintOptions.inverse ? Goldilocks::inv(Goldilocks::fromU64(hintFieldVal.value)) : Goldilocks::fromU64(hintFieldVal.value);
        } else if (hintFieldVal.operand == opType::airgroupvalue) {
            if(!hintOptions.dest) {
                uint64_t pos = 0;
                for(uint64_t i = 0; i < hintFieldVal.id; ++i) {
                    pos += setupCtx.starkInfo.airgroupValuesMap[i].stage == 1 ? 1 : FIELD_EXTENSION;
                }
                if(hintOptions.inverse)  {
                    Goldilocks3::inv((Goldilocks3::Element *)hintFieldInfo.values, (Goldilocks3::Element *)&params.airgroupValues[pos]);
                } else {
                    std::memcpy(hintFieldInfo.values, &params.airgroupValues[pos], hintFieldInfo.size * sizeof(Goldilocks::Element));
                }
            }
        } else if (hintFieldVal.operand == opType::airvalue) {
            if(!hintOptions.dest) {
                uint64_t pos = 0;
                for(uint64_t i = 0; i < hintFieldVal.id; ++i) {
                    pos += setupCtx.starkInfo.airValuesMap[i].stage == 1 ? 1 : FIELD_EXTENSION;
                }
                if(hintOptions.inverse)  {
                    Goldilocks3::inv((Goldilocks3::Element *)hintFieldInfo.values, (Goldilocks3::Element *)&params.airValues[pos]);
                } else {
                    std::memcpy(hintFieldInfo.values, &params.airValues[pos], hintFieldInfo.size * sizeof(Goldilocks::Element));
                }
            }
        } else if (hintFieldVal.operand == opType::proofvalue) {
            if(!hintOptions.dest) {
                uint64_t pos = 0;
                for(uint64_t i = 0; i < hintFieldVal.id; ++i) {
                    pos += setupCtx.starkInfo.proofValuesMap[i].stage == 1 ? 1 : FIELD_EXTENSION;
                }
                if(hintOptions.inverse)  {
                    Goldilocks3::inv((Goldilocks3::Element *)hintFieldInfo.values, (Goldilocks3::Element *)&params.proofValues[pos]);
                } else {
                    std::memcpy(hintFieldInfo.values, &params.proofValues[pos], hintFieldInfo.size * sizeof(Goldilocks::Element));
                }
            }
        } else if (hintFieldVal.operand == opType::challenge) {
            if(hintOptions.inverse) {
                Goldilocks3::inv((Goldilocks3::Element *)hintFieldInfo.values, (Goldilocks3::Element *)&params.challenges[FIELD_EXTENSION*hintFieldVal.id]);
            } else {
                std::memcpy(hintFieldInfo.values, &params.challenges[FIELD_EXTENSION*hintFieldVal.id], hintFieldInfo.size * sizeof(Goldilocks::Element));
            }
        } else if (hintFieldVal.operand == opType::string_) {
            std::memcpy(hintFieldInfo.stringValue, hintFieldVal.stringValue.data(), hintFieldVal.stringValue.size()); 
        } else {
            zklog.error("Unknown HintFieldType");
            exitProcess();
            exit(-1);
        }

        for(uint64_t i = 0; i < hintFieldInfo.matrix_size; ++i) {
            hintFieldInfo.pos[i] =  hintFieldVal.pos[i];
        }
    }
    
    return;
}

uint64_t setHintField(SetupCtx& setupCtx, StepsParams& params, Goldilocks::Element* values, uint64_t hintId, std::string hintFieldName) {
    Hint hint = setupCtx.expressionsBin.hints[hintId];

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
    if(hintFieldVal.operand == opType::cm) {
        Goldilocks::Element *pAddress = setupCtx.starkInfo.cmPolsMap[hintFieldVal.id].stage > 1 ? params.aux_trace : params.trace;
        setPolynomial(setupCtx, pAddress, values, hintFieldVal.id);
    } else if(hintFieldVal.operand == opType::airgroupvalue) {
        if(setupCtx.starkInfo.airgroupValuesMap[hintFieldVal.id].stage > 1) {
            std::memcpy(&params.airgroupValues[FIELD_EXTENSION*hintFieldVal.id], values, FIELD_EXTENSION * sizeof(Goldilocks::Element));
        } else {
           params.airgroupValues[FIELD_EXTENSION*hintFieldVal.id] = values[0]; 
        }
    } else if(hintFieldVal.operand == opType::airvalue) {
        if(setupCtx.starkInfo.airValuesMap[hintFieldVal.id].stage > 1) {
            std::memcpy(&params.airValues[FIELD_EXTENSION*hintFieldVal.id], values, FIELD_EXTENSION * sizeof(Goldilocks::Element));
        } else {
           params.airValues[FIELD_EXTENSION*hintFieldVal.id] = values[0]; 
        }
    } else {
        zklog.error("Only committed pols and airgroupvalues can be set");
        exitProcess();
        exit(-1);  
    }

    return hintFieldVal.id;
}

void addHintField(SetupCtx& setupCtx, StepsParams& params, uint64_t hintId, Dest &destStruct, std::string hintFieldName, HintFieldOptions hintFieldOptions) {
    Hint hint = setupCtx.expressionsBin.hints[hintId];
    
    auto hintField = std::find_if(hint.fields.begin(), hint.fields.end(), [hintFieldName](const HintField& hintField) {
        return hintField.name == hintFieldName;
    });
    HintFieldValue hintFieldVal = hintField->values[0];

    if(hintField == hint.fields.end()) {
        zklog.error("Hint field " + hintFieldName + " not found in hint " + hint.name + ".");
        exitProcess();
        exit(-1);
    }

    if(hintFieldOptions.print_expression) {
        std::string expression_line = getExpressionDebug(setupCtx, hintId, hintFieldName, hintFieldVal);
    }
    if(hintFieldVal.operand == opType::cm) {
        destStruct.addCmPol(setupCtx.starkInfo.cmPolsMap[hintFieldVal.id], hintFieldVal.rowOffsetIndex, hintFieldOptions.inverse);
    } else if(hintFieldVal.operand == opType::const_) {
        destStruct.addConstPol(setupCtx.starkInfo.constPolsMap[hintFieldVal.id], hintFieldVal.rowOffsetIndex, hintFieldOptions.inverse);
    } else if(hintFieldVal.operand == opType::number) {
        destStruct.addNumber(hintFieldVal.value, hintFieldOptions.inverse);
    } else if(hintFieldVal.operand == opType::tmp) {
        destStruct.addParams(setupCtx.expressionsBin.expressionsInfo[hintFieldVal.id], hintFieldOptions.inverse);
    } else {
        zklog.error("Op type " + to_string(hintFieldVal.operand) + "is not considered yet.");
        exitProcess();
        exit(-1);
    }
}

void opHintFields(SetupCtx& setupCtx, StepsParams& params, std::vector<Dest> &dests) {
#ifdef __AVX512__
    ExpressionsAvx512 expressionsCtx(setupCtx);
#elif defined(__AVX2__)
    ExpressionsAvx expressionsCtx(setupCtx);
#else
    ExpressionsPack expressionsCtx(setupCtx);
#endif

    uint64_t domainSize = 1 << setupCtx.starkInfo.starkStruct.nBits;
    expressionsCtx.calculateExpressions(params, setupCtx.expressionsBin.expressionsBinArgsExpressions, dests, domainSize, false);
}

uint64_t multiplyHintFields(SetupCtx& setupCtx, StepsParams &params, uint64_t hintId, std::string hintFieldNameDest, std::string hintFieldName1, std::string hintFieldName2,  HintFieldOptions &hintOptions1, HintFieldOptions &hintOptions2) {
    if(setupCtx.expressionsBin.hints.size() == 0) {
        zklog.error("No hints were found.");
        exitProcess();
        exit(-1);
    }

    Hint hint = setupCtx.expressionsBin.hints[hintId];

    auto hintFieldDest = std::find_if(hint.fields.begin(), hint.fields.end(), [hintFieldNameDest](const HintField& hintField) {
        return hintField.name == hintFieldNameDest;
    });
    HintFieldValue hintFieldDestVal = hintFieldDest->values[0];

    uint64_t offset = setupCtx.starkInfo.mapSectionsN["cm" + to_string(setupCtx.starkInfo.cmPolsMap[hintFieldDestVal.id].stage)];
    Goldilocks::Element *buff = &params.aux_trace[setupCtx.starkInfo.mapOffsets[std::make_pair("cm" + to_string(setupCtx.starkInfo.cmPolsMap[hintFieldDestVal.id].stage), false)] + setupCtx.starkInfo.cmPolsMap[hintFieldDestVal.id].stagePos];
    
    Dest destStruct(buff, offset);

    addHintField(setupCtx, params, hintId, destStruct, hintFieldName1, hintOptions1);
    addHintField(setupCtx, params, hintId, destStruct, hintFieldName2, hintOptions2);

    std::vector<Dest> dests = {destStruct};

    opHintFields(setupCtx, params, dests);

    return hintFieldDestVal.id;
}

void accHintField(SetupCtx& setupCtx, StepsParams &params, uint64_t hintId, std::string hintFieldNameDest, std::string hintFieldNameAirgroupVal, std::string hintFieldName, bool add) {
    uint64_t N = 1 << setupCtx.starkInfo.starkStruct.nBits;

    Hint hint = setupCtx.expressionsBin.hints[hintId];

    auto hintFieldDest = std::find_if(hint.fields.begin(), hint.fields.end(), [hintFieldNameDest](const HintField& hintField) {
        return hintField.name == hintFieldNameDest;
    });

    HintFieldOptions hintOptions;
    HintFieldValue hintFieldDestVal = hintFieldDest->values[0];

    uint64_t dim = setupCtx.starkInfo.cmPolsMap[hintFieldDestVal.id].dim;
    Goldilocks::Element *vals = &params.aux_trace[setupCtx.starkInfo.mapOffsets[std::make_pair("q", true)]];
    
    Dest destStruct(vals, 0);
    addHintField(setupCtx, params, hintId, destStruct, hintFieldName, hintOptions);

    std::vector<Dest> dests = {destStruct};

    opHintFields(setupCtx, params, dests);

    for(uint64_t i = 1; i < N; ++i) {
        if(add) {
            if(dim == 1) {
                Goldilocks::add(vals[i], vals[i], vals[(i - 1)]);
            } else {
                Goldilocks3::add((Goldilocks3::Element &)vals[i * FIELD_EXTENSION], (Goldilocks3::Element &)vals[i * FIELD_EXTENSION], (Goldilocks3::Element &)vals[(i - 1) * FIELD_EXTENSION]);
            }
        } else {
            if(dim == 1) {
                Goldilocks::mul(vals[i], vals[i], vals[(i - 1)]);
            } else {
                Goldilocks3::mul((Goldilocks3::Element &)vals[i * FIELD_EXTENSION], (Goldilocks3::Element &)vals[i * FIELD_EXTENSION], (Goldilocks3::Element &)vals[(i - 1) * FIELD_EXTENSION]);
            }
        }
    }

    setHintField(setupCtx, params, vals, hintId, hintFieldNameDest);
    setHintField(setupCtx, params, &vals[(N - 1)*FIELD_EXTENSION], hintId, hintFieldNameAirgroupVal);
}

uint64_t getHintId(SetupCtx& setupCtx, uint64_t hintId, std::string name) {
    Hint hint = setupCtx.expressionsBin.hints[hintId];

    auto hintField = std::find_if(hint.fields.begin(), hint.fields.end(), [name](const HintField& hintField) {
        return hintField.name == name;
    });
    return hintField->values[0].id;
}

void accMulHintFields(SetupCtx& setupCtx, StepsParams &params, uint64_t hintId, std::string hintFieldNameDest, std::string hintFieldNameAirgroupVal, std::string hintFieldName1, std::string hintFieldName2, HintFieldOptions &hintOptions1, HintFieldOptions &hintOptions2, bool add) {
    uint64_t N = 1 << setupCtx.starkInfo.starkStruct.nBits;

    Hint hint = setupCtx.expressionsBin.hints[hintId];

    auto hintFieldDest = std::find_if(hint.fields.begin(), hint.fields.end(), [hintFieldNameDest](const HintField& hintField) {
        return hintField.name == hintFieldNameDest;
    });
    HintFieldValue hintFieldDestVal = hintFieldDest->values[0];

    uint64_t dim = setupCtx.starkInfo.cmPolsMap[hintFieldDestVal.id].dim;
    Goldilocks::Element *vals = &params.aux_trace[setupCtx.starkInfo.mapOffsets[std::make_pair("q", true)]];
    
    Dest destStruct(vals, 0);
    addHintField(setupCtx, params, hintId, destStruct, hintFieldName1, hintOptions1);
    addHintField(setupCtx, params, hintId, destStruct, hintFieldName2, hintOptions2);

    std::vector<Dest> dests = {destStruct};

    opHintFields(setupCtx, params, dests);

    for(uint64_t i = 1; i < N; ++i) {
        if(add) {
            if(dim == 1) {
                Goldilocks::add(vals[i], vals[i], vals[(i - 1)]);
            } else {
                Goldilocks3::add((Goldilocks3::Element &)vals[i * FIELD_EXTENSION], (Goldilocks3::Element &)vals[i * FIELD_EXTENSION], (Goldilocks3::Element &)vals[(i - 1) * FIELD_EXTENSION]);
            }
        } else {
            if(dim == 1) {
                Goldilocks::mul(vals[i], vals[i], vals[(i - 1)]);
            } else {
                Goldilocks3::mul((Goldilocks3::Element &)vals[i * FIELD_EXTENSION], (Goldilocks3::Element &)vals[i * FIELD_EXTENSION], (Goldilocks3::Element &)vals[(i - 1) * FIELD_EXTENSION]);
            }
        }
    }

    setHintField(setupCtx, params, vals, hintId, hintFieldNameDest);
    setHintField(setupCtx, params, &vals[(N - 1)*FIELD_EXTENSION], hintId, hintFieldNameAirgroupVal);
}

uint64_t updateAirgroupValue(SetupCtx& setupCtx, StepsParams &params, uint64_t hintId, std::string hintFieldNameAirgroupVal, std::string hintFieldName1, std::string hintFieldName2, HintFieldOptions &hintOptions1, HintFieldOptions &hintOptions2, bool add) {
    
    Hint hint = setupCtx.expressionsBin.hints[hintId];

    auto hintFieldAirgroup = std::find_if(hint.fields.begin(), hint.fields.end(), [hintFieldNameAirgroupVal](const HintField& hintField) {
        return hintField.name == hintFieldNameAirgroupVal;
    });
    HintFieldValue hintFieldAirgroupVal = hintFieldAirgroup->values[0];

    Goldilocks::Element vals[3];
    
    Dest destStruct(vals, 0);
    addHintField(setupCtx, params, hintId, destStruct, hintFieldName1, hintOptions1);
    addHintField(setupCtx, params, hintId, destStruct, hintFieldName2, hintOptions2);

    std::vector<Dest> dests = {destStruct};

    ExpressionsPack expressionsCtx(setupCtx, 1);
    expressionsCtx.calculateExpressions(params, setupCtx.expressionsBin.expressionsBinArgsExpressions, dests, 1, false);

    Goldilocks::Element *airgroupValue = &params.airgroupValues[FIELD_EXTENSION*hintFieldAirgroupVal.id];
    if(add) {
        if(destStruct.dim == 1) {
            Goldilocks::add(airgroupValue[0], airgroupValue[0], vals[0]);
        } else {
            Goldilocks3::add((Goldilocks3::Element &)airgroupValue[0], (Goldilocks3::Element &)airgroupValue[0], (Goldilocks3::Element &)vals[0]);
        }
    } else {
        if(destStruct.dim == 1) {
            Goldilocks::mul(airgroupValue[0], airgroupValue[0], vals[0]);
        } else {
            Goldilocks3::mul((Goldilocks3::Element &)airgroupValue[0], (Goldilocks3::Element &)airgroupValue[0], (Goldilocks3::Element &)vals[0]);
        }
    }

    return hintFieldAirgroupVal.id;
}
