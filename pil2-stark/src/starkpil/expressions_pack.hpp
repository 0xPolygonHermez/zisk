#ifndef EXPRESSIONS_PACK_HPP
#define EXPRESSIONS_PACK_HPP
#include "expressions_ctx.hpp"

class ExpressionsPack : public ExpressionsCtx {
public:
    uint64_t nrowsPack;
    uint64_t nCols;
    vector<uint64_t> nColsStages;
    vector<uint64_t> nColsStagesAcc;
    vector<uint64_t> offsetsStages;
    ExpressionsPack(SetupCtx& setupCtx, uint64_t nrowsPack_ = 4) : ExpressionsCtx(setupCtx), nrowsPack(nrowsPack_) {};

    void setBufferTInfo(bool domainExtended, int64_t expId) {
        uint64_t nOpenings = setupCtx.starkInfo.openingPoints.size();
        offsetsStages.resize((setupCtx.starkInfo.nStages + 2)*nOpenings + 1);
        nColsStages.resize((setupCtx.starkInfo.nStages + 2)*nOpenings + 1);
        nColsStagesAcc.resize((setupCtx.starkInfo.nStages + 2)*nOpenings + 1);

        nCols = setupCtx.starkInfo.nConstants;
        uint64_t ns = setupCtx.starkInfo.nStages + 2;
        for(uint64_t o = 0; o < nOpenings; ++o) {
            for(uint64_t stage = 0; stage <= ns; ++stage) {
                std::string section = stage == 0 ? "const" : "cm" + to_string(stage);
                offsetsStages[(setupCtx.starkInfo.nStages + 2)*o + stage] = setupCtx.starkInfo.mapOffsets[std::make_pair(section, domainExtended)];
                nColsStages[(setupCtx.starkInfo.nStages + 2)*o + stage] = setupCtx.starkInfo.mapSectionsN[section];
                nColsStagesAcc[(setupCtx.starkInfo.nStages + 2)*o + stage] = stage == 0 && o == 0 ? 0 : nColsStagesAcc[(setupCtx.starkInfo.nStages + 2)*o + stage - 1] + nColsStages[stage - 1];
            }
        }

        nColsStagesAcc[(setupCtx.starkInfo.nStages + 2)*nOpenings] = nColsStagesAcc[(setupCtx.starkInfo.nStages + 2)*nOpenings - 1] + nColsStages[(setupCtx.starkInfo.nStages + 2)*nOpenings - 1];
        if(expId == int64_t(setupCtx.starkInfo.cExpId)) {
            nCols = nColsStagesAcc[(setupCtx.starkInfo.nStages + 2)*nOpenings] + setupCtx.starkInfo.boundaries.size() + 1;
        } else if(expId == int64_t(setupCtx.starkInfo.friExpId)) {
            nCols = nColsStagesAcc[(setupCtx.starkInfo.nStages + 2)*nOpenings] + nOpenings*FIELD_EXTENSION;
        } else {
            nCols = nColsStagesAcc[(setupCtx.starkInfo.nStages + 2)*nOpenings] + 1;
        }
    }

    inline void loadPolynomials(StepsParams& params, ParserArgs &parserArgs, std::vector<Dest> &dests, Goldilocks::Element *bufferT_, uint64_t row, uint64_t domainSize) {
        uint64_t nOpenings = setupCtx.starkInfo.openingPoints.size();
        bool domainExtended = domainSize == uint64_t(1 << setupCtx.starkInfo.starkStruct.nBitsExt) ? true : false;

        uint64_t extendBits = (setupCtx.starkInfo.starkStruct.nBitsExt - setupCtx.starkInfo.starkStruct.nBits);
        int64_t extend = domainExtended ? (1 << extendBits) : 1;
        uint64_t nextStrides[nOpenings];
        for(uint64_t i = 0; i < nOpenings; ++i) {
            uint64_t opening = setupCtx.starkInfo.openingPoints[i] < 0 ? setupCtx.starkInfo.openingPoints[i] + domainSize : setupCtx.starkInfo.openingPoints[i];
            nextStrides[i] = opening * extend;
        }

        Goldilocks::Element *constPols = domainExtended ? &params.pConstPolsExtendedTreeAddress[2] : params.pConstPolsAddress;

        std::vector<bool> constPolsUsed(setupCtx.starkInfo.constPolsMap.size(), false);
        std::vector<bool> cmPolsUsed(setupCtx.starkInfo.cmPolsMap.size(), false);

        for(uint64_t i = 0; i < dests.size(); ++i) {
            for(uint64_t j = 0; j < dests[i].params.size(); ++j) {
                if(dests[i].params[j].op == opType::cm) {
                    cmPolsUsed[dests[i].params[j].polsMapId] = true;
                } else if (dests[i].params[j].op == opType::const_) {
                    constPolsUsed[dests[i].params[j].polsMapId] = true;
                } else if(dests[i].params[j].op == opType::tmp) {
                    uint16_t* cmUsed = &parserArgs.cmPolsIds[dests[i].params[j].parserParams.cmPolsOffset];
                    uint16_t* constUsed = &parserArgs.constPolsIds[dests[i].params[j].parserParams.constPolsOffset];

                    for(uint64_t k = 0; k < dests[i].params[j].parserParams.nConstPolsUsed; ++k) {
                        constPolsUsed[constUsed[k]] = true;
                    }

                    for(uint64_t k = 0; k < dests[i].params[j].parserParams.nCmPolsUsed; ++k) {
                        cmPolsUsed[cmUsed[k]] = true;
                    }
                }
            }
        }
        for(uint64_t k = 0; k < constPolsUsed.size(); ++k) {
            if(!constPolsUsed[k]) continue;
            for(uint64_t o = 0; o < nOpenings; ++o) {
                for(uint64_t j = 0; j < nrowsPack; ++j) {
                    uint64_t l = (row + j + nextStrides[o]) % domainSize;
                    bufferT_[(nColsStagesAcc[(setupCtx.starkInfo.nStages + 2)*o] + k)*nrowsPack + j] = constPols[l * nColsStages[0] + k];
                }
            }
        }

        for(uint64_t k = 0; k < cmPolsUsed.size(); ++k) {
            if(!cmPolsUsed[k]) continue;
            PolMap polInfo = setupCtx.starkInfo.cmPolsMap[k];
            uint64_t stage = polInfo.stage;
            uint64_t stagePos = polInfo.stagePos;
            for(uint64_t d = 0; d < polInfo.dim; ++d) {
                for(uint64_t o = 0; o < nOpenings; ++o) {
                    for(uint64_t j = 0; j < nrowsPack; ++j) {
                        uint64_t l = (row + j + nextStrides[o]) % domainSize;
                        bufferT_[(nColsStagesAcc[(setupCtx.starkInfo.nStages + 2)*o + stage] + (stagePos + d))*nrowsPack + j] = params.pols[offsetsStages[stage] + l * nColsStages[stage] + stagePos + d];
                    }
                }
            }
        }

        if(dests[0].params[0].parserParams.expId == int64_t(setupCtx.starkInfo.cExpId)) {
            for(uint64_t d = 0; d < setupCtx.starkInfo.boundaries.size(); ++d) {
                for(uint64_t j = 0; j < nrowsPack; ++j) {
                    bufferT_[(nColsStagesAcc[(setupCtx.starkInfo.nStages + 2)*nOpenings] + d + 1)*nrowsPack + j] = setupCtx.proverHelpers.zi[row + j + d*domainSize];
                }
            }
            for(uint64_t j = 0; j < nrowsPack; ++j) {
                bufferT_[(nColsStagesAcc[(setupCtx.starkInfo.nStages + 2)*nOpenings])*nrowsPack + j] = setupCtx.proverHelpers.x_2ns[row + j];
            }
        } else if(dests[0].params[0].parserParams.expId == int64_t(setupCtx.starkInfo.friExpId)) {
            for(uint64_t d = 0; d < setupCtx.starkInfo.openingPoints.size(); ++d) {
               for(uint64_t k = 0; k < FIELD_EXTENSION; ++k) {
                    for(uint64_t j = 0; j < nrowsPack; ++j) {
                        bufferT_[(nColsStagesAcc[(setupCtx.starkInfo.nStages + 2)*nOpenings] + d*FIELD_EXTENSION + k)*nrowsPack + j] = params.xDivXSub[(row + j + d*domainSize)*FIELD_EXTENSION + k];
                    }
                }
            }
        } else {
            for(uint64_t j = 0; j < nrowsPack; ++j) {
                bufferT_[(nColsStagesAcc[(setupCtx.starkInfo.nStages + 2)*nOpenings])*nrowsPack + j] = setupCtx.proverHelpers.x[row + j];
            }
        }
    }

    inline void copyPolynomial(Goldilocks::Element* destVals, bool inverse, uint64_t dim, Goldilocks::Element* tmp) {
        if(dim == 1) {
            if(inverse) {
                Goldilocks::batchInverse(&destVals[0], &tmp[0], nrowsPack);
            } else {
                Goldilocks::copy_pack(nrowsPack, &destVals[0], &tmp[0]);
            }
        } else if(dim == FIELD_EXTENSION) {
            if(inverse) {
                Goldilocks::Element buff[FIELD_EXTENSION*nrowsPack];
                Goldilocks::copy_pack(nrowsPack, &buff[0], uint64_t(FIELD_EXTENSION), &tmp[0]);
                Goldilocks::copy_pack(nrowsPack, &buff[1], uint64_t(FIELD_EXTENSION), &tmp[nrowsPack]);
                Goldilocks::copy_pack(nrowsPack, &buff[2], uint64_t(FIELD_EXTENSION), &tmp[2*nrowsPack]);
                Goldilocks3::batchInverse((Goldilocks3::Element *)buff, (Goldilocks3::Element *)buff, nrowsPack);
                Goldilocks::copy_pack(nrowsPack, &destVals[0], &buff[0], uint64_t(FIELD_EXTENSION));
                Goldilocks::copy_pack(nrowsPack, &destVals[nrowsPack], &buff[1], uint64_t(FIELD_EXTENSION));
                Goldilocks::copy_pack(nrowsPack, &destVals[2*nrowsPack], &buff[2], uint64_t(FIELD_EXTENSION));
            } else {
                Goldilocks::copy_pack(nrowsPack, &destVals[0], &tmp[0]);
                Goldilocks::copy_pack(nrowsPack, &destVals[nrowsPack], &tmp[nrowsPack]);
                Goldilocks::copy_pack(nrowsPack, &destVals[2*nrowsPack], &tmp[2*nrowsPack]);
            }
        }
    }

    inline void storePolynomial(std::vector<Dest> dests, Goldilocks::Element** destVals, uint64_t row) {
        for(uint64_t i = 0; i < dests.size(); ++i) {
            Goldilocks::Element vals[FIELD_EXTENSION*nrowsPack];
            uint64_t dim = 1;
            if(dests[i].params.size() == 1) {
                dim = dests[i].params[0].dim;
                if(dim == 1) {
                    Goldilocks::copy_pack(nrowsPack, &vals[0], &destVals[i][0]);
                } else {
                    Goldilocks::copy_pack(nrowsPack, &vals[0], &destVals[i][0]);
                    Goldilocks::copy_pack(nrowsPack, &vals[nrowsPack], &destVals[i][nrowsPack]);
                    Goldilocks::copy_pack(nrowsPack, &vals[2*nrowsPack], &destVals[i][2*nrowsPack]);
                }
            } else if(dests[i].params.size() == 2) {
                if(dests[i].params[0].dim == FIELD_EXTENSION && dests[i].params[1].dim == FIELD_EXTENSION) {
                    Goldilocks3::op_pack(nrowsPack, 2, &vals[0], &destVals[i][0], &destVals[i][FIELD_EXTENSION*nrowsPack]);
                    dim = FIELD_EXTENSION;
                } else if(dests[i].params[0].dim == FIELD_EXTENSION && dests[i].params[1].dim == 1) {
                    Goldilocks3::op_31_pack(nrowsPack, 2, &vals[0], &destVals[i][0], &destVals[i][FIELD_EXTENSION*nrowsPack]);
                    dim = FIELD_EXTENSION;
                } else if(dests[i].params[0].dim == 1 && dests[i].params[1].dim == FIELD_EXTENSION) {
                    Goldilocks3::op_31_pack(nrowsPack, 2, &vals[0], &destVals[i][FIELD_EXTENSION*nrowsPack], &destVals[i][0]);
                    dim = FIELD_EXTENSION;
                } else {
                    Goldilocks::op_pack(nrowsPack, 2, &vals[0], &destVals[i][0], &destVals[i][FIELD_EXTENSION*nrowsPack]);
                    dim = 1;
                }
            } else {
                zklog.error("Currently only length 1 and 2 are supported");
                exitProcess();
            }
            if(dim == 1) {
                uint64_t offset = dests[i].offset != 0 ? dests[i].offset : 1;
                Goldilocks::copy_pack(nrowsPack, &dests[i].dest[row*offset], uint64_t(offset), &vals[0]);
            } else {
                uint64_t offset = dests[i].offset != 0 ? dests[i].offset : FIELD_EXTENSION;
                Goldilocks::copy_pack(nrowsPack, &dests[i].dest[row*offset], uint64_t(offset), &vals[0]);
                Goldilocks::copy_pack(nrowsPack, &dests[i].dest[row*offset + 1], uint64_t(offset), &vals[nrowsPack]);
                Goldilocks::copy_pack(nrowsPack, &dests[i].dest[row*offset + 2], uint64_t(offset), &vals[2*nrowsPack]);
            }
        }
    }

    inline void printTmp1(uint64_t row, Goldilocks::Element* tmp) {
        Goldilocks::Element buff[nrowsPack];
        Goldilocks::copy_pack(nrowsPack, buff, tmp);
        for(uint64_t i = 0; i < nrowsPack; ++i) {
            cout << "Value at row " << row + i << " is " << Goldilocks::toString(buff[i]) << endl;
        }
    }

    inline void printTmp3(uint64_t row, Goldilocks::Element* tmp) {
        Goldilocks::Element buff[FIELD_EXTENSION*nrowsPack];
        Goldilocks::copy_pack(nrowsPack,&buff[0], uint64_t(FIELD_EXTENSION), &tmp[0]);
        Goldilocks::copy_pack(nrowsPack,&buff[1], uint64_t(FIELD_EXTENSION), &tmp[1]);
        Goldilocks::copy_pack(nrowsPack,&buff[2], uint64_t(FIELD_EXTENSION), &tmp[2]);
        for(uint64_t i = 0; i < 1; ++i) {
            cout << "Value at row " << row + i << " is [" << Goldilocks::toString(buff[FIELD_EXTENSION*i]) << ", " << Goldilocks::toString(buff[FIELD_EXTENSION*i + 1]) << ", " << Goldilocks::toString(buff[FIELD_EXTENSION*i + 2]) << "]" << endl;
        }
    }

    inline void printCommit(uint64_t row, Goldilocks::Element* bufferT, bool extended) {
        if(extended) {
            Goldilocks::Element buff[FIELD_EXTENSION*nrowsPack];
            Goldilocks::copy_pack(nrowsPack, &buff[0], uint64_t(FIELD_EXTENSION), &bufferT[0]);
            Goldilocks::copy_pack(nrowsPack, &buff[1], uint64_t(FIELD_EXTENSION), &bufferT[setupCtx.starkInfo.openingPoints.size()]);
            Goldilocks::copy_pack(nrowsPack, &buff[2], uint64_t(FIELD_EXTENSION), &bufferT[2*setupCtx.starkInfo.openingPoints.size()]);
            for(uint64_t i = 0; i < 1; ++i) {
                cout << "Value at row " << row + i << " is [" << Goldilocks::toString(buff[FIELD_EXTENSION*i]) << ", " << Goldilocks::toString(buff[FIELD_EXTENSION*i + 1]) << ", " << Goldilocks::toString(buff[FIELD_EXTENSION*i + 2]) << "]" << endl;
            }
        } else {
            Goldilocks::Element buff[nrowsPack];
            Goldilocks::copy_pack(nrowsPack, &buff[0], &bufferT[0]);
            for(uint64_t i = 0; i < nrowsPack; ++i) {
                cout << "Value at row " << row + i << " is " << Goldilocks::toString(buff[i]) << endl;
            }
        }
    }

    void calculateExpressions(StepsParams& params, ParserArgs &parserArgs, std::vector<Dest> dests, uint64_t domainSize) override {
        uint64_t nOpenings = setupCtx.starkInfo.openingPoints.size();
        bool domainExtended = domainSize == uint64_t(1 << setupCtx.starkInfo.starkStruct.nBitsExt) ? true : false;

        uint64_t expId = dests[0].params[0].op == opType::tmp ? dests[0].params[0].parserParams.destDim : 0;
        setBufferTInfo(domainExtended, expId);

        Goldilocks::Element challenges[setupCtx.starkInfo.challengesMap.size()*FIELD_EXTENSION*nrowsPack];
        for(uint64_t i = 0; i < setupCtx.starkInfo.challengesMap.size(); ++i) {
            for(uint64_t j = 0; j < nrowsPack; ++j) {
                challenges[(i*FIELD_EXTENSION)*nrowsPack + j] = params.challenges[i * FIELD_EXTENSION];
                challenges[(i*FIELD_EXTENSION + 1)*nrowsPack + j] = params.challenges[i * FIELD_EXTENSION + 1];
                challenges[(i*FIELD_EXTENSION + 2)*nrowsPack + j] = params.challenges[i * FIELD_EXTENSION + 2];
            }
        }

        Goldilocks::Element numbers_[parserArgs.nNumbers*nrowsPack];
        for(uint64_t i = 0; i < parserArgs.nNumbers; ++i) {
            for(uint64_t k = 0; k < nrowsPack; ++k) {
                numbers_[i*nrowsPack + k] = Goldilocks::fromU64(parserArgs.numbers[i]);
            }
        }

        Goldilocks::Element publics[setupCtx.starkInfo.nPublics*nrowsPack];
        for(uint64_t i = 0; i < setupCtx.starkInfo.nPublics; ++i) {
            for(uint64_t j = 0; j < nrowsPack; ++j) {
                publics[i*nrowsPack + j] = params.publicInputs[i];
            }
        }

        Goldilocks::Element evals[setupCtx.starkInfo.evMap.size()*FIELD_EXTENSION*nrowsPack];
        for(uint64_t i = 0; i < setupCtx.starkInfo.evMap.size(); ++i) {
            for(uint64_t j = 0; j < nrowsPack; ++j) {
                evals[(i*FIELD_EXTENSION)*nrowsPack + j] = params.evals[i * FIELD_EXTENSION];
                evals[(i*FIELD_EXTENSION + 1)*nrowsPack + j] = params.evals[i * FIELD_EXTENSION + 1];
                evals[(i*FIELD_EXTENSION + 2)*nrowsPack + j] = params.evals[i * FIELD_EXTENSION + 2];
            }
        }

        Goldilocks::Element airgroupValues[setupCtx.starkInfo.airgroupValuesMap.size()*FIELD_EXTENSION*nrowsPack];
        for(uint64_t i = 0; i < setupCtx.starkInfo.airgroupValuesMap.size(); ++i) {
            for(uint64_t j = 0; j < nrowsPack; ++j) {
                airgroupValues[(i*FIELD_EXTENSION)*nrowsPack + j] = params.airgroupValues[i * FIELD_EXTENSION];
                airgroupValues[(i*FIELD_EXTENSION + 1)*nrowsPack + j] = params.airgroupValues[i * FIELD_EXTENSION + 1];
                airgroupValues[(i*FIELD_EXTENSION + 2)*nrowsPack + j] = params.airgroupValues[i * FIELD_EXTENSION + 2];
            }
        }

        Goldilocks::Element airValues[setupCtx.starkInfo.airValuesMap.size()*FIELD_EXTENSION*nrowsPack];
        for(uint64_t i = 0; i < setupCtx.starkInfo.airValuesMap.size(); ++i) {
            for(uint64_t j = 0; j < nrowsPack; ++j) {
                airValues[(i*FIELD_EXTENSION)*nrowsPack + j] = params.airValues[i * FIELD_EXTENSION];
                airValues[(i*FIELD_EXTENSION + 1)*nrowsPack + j] = params.airValues[i * FIELD_EXTENSION + 1];
                airValues[(i*FIELD_EXTENSION + 2)*nrowsPack + j] = params.airValues[i * FIELD_EXTENSION + 2];
            }
        }

    #pragma omp parallel for
        for (uint64_t i = 0; i < domainSize; i+= nrowsPack) {
            Goldilocks::Element bufferT_[nOpenings*nCols*nrowsPack];

            loadPolynomials(params, parserArgs, dests, bufferT_, i, domainSize);

            Goldilocks::Element **destVals = new Goldilocks::Element*[dests.size()];

            for(uint64_t j = 0; j < dests.size(); ++j) {
                destVals[j] = new Goldilocks::Element[dests[j].params.size() * FIELD_EXTENSION* nrowsPack];
                for(uint64_t k = 0; k < dests[j].params.size(); ++k) {
                    uint64_t i_args = 0;

                    if(dests[j].params[k].op == opType::cm || dests[j].params[k].op == opType::const_) {
                        auto openingPointZero = std::find_if(setupCtx.starkInfo.openingPoints.begin(), setupCtx.starkInfo.openingPoints.end(), [](int p) { return p == 0; });
                        auto openingPointZeroIndex = std::distance(setupCtx.starkInfo.openingPoints.begin(), openingPointZero);

                        uint64_t buffPos = (setupCtx.starkInfo.nStages + 2)*openingPointZeroIndex + dests[j].params[k].stage;
                        uint64_t stagePos = dests[j].params[k].stagePos;
                        copyPolynomial(&destVals[j][k*FIELD_EXTENSION], dests[j].params[k].inverse, dests[j].params[k].dim, &bufferT_[nColsStagesAcc[buffPos] + stagePos]);
                        continue;
                    } else if(dests[j].params[k].op == opType::number) {
                        uint64_t val = dests[j].params[k].inverse ? Goldilocks::inv(Goldilocks::fromU64(dests[j].params[k].value)).fe : dests[j].params[k].value;
                        for(uint64_t r = 0; r < nrowsPack; ++r) {
                            destVals[j][k*FIELD_EXTENSION*nrowsPack + r] = Goldilocks::fromU64(val);
                        }
                        continue;
                    }

                    uint8_t* ops = &parserArgs.ops[dests[j].params[k].parserParams.opsOffset];
                    uint16_t* args = &parserArgs.args[dests[j].params[k].parserParams.argsOffset];
                    Goldilocks::Element tmp1[dests[j].params[k].parserParams.nTemp1*nrowsPack];
                    Goldilocks::Element tmp3[dests[j].params[k].parserParams.nTemp3*nrowsPack*FIELD_EXTENSION];

                    for (uint64_t kk = 0; kk < dests[j].params[k].parserParams.nOps; ++kk) {
                        switch (ops[kk]) {
                            case 0: {
                                // COPY commit1 to tmp1
                                Goldilocks::copy_pack(nrowsPack, &tmp1[args[i_args] * nrowsPack], &bufferT_[(nColsStagesAcc[args[i_args + 1]] + args[i_args + 2]) * nrowsPack]);
                                i_args += 3;
                                break;
                            }
                            case 1: {
                                // OPERATION WITH DEST: tmp1 - SRC0: commit1 - SRC1: commit1
                                Goldilocks::op_pack(nrowsPack, args[i_args], &tmp1[args[i_args + 1] * nrowsPack], &bufferT_[(nColsStagesAcc[args[i_args + 2]] + args[i_args + 3]) * nrowsPack], &bufferT_[(nColsStagesAcc[args[i_args + 4]] + args[i_args + 5]) * nrowsPack]);
                                i_args += 6;
                                break;
                            }
                            case 2: {
                                // OPERATION WITH DEST: tmp1 - SRC0: commit1 - SRC1: tmp1
                                Goldilocks::op_pack(nrowsPack, args[i_args], &tmp1[args[i_args + 1] * nrowsPack], &bufferT_[(nColsStagesAcc[args[i_args + 2]] + args[i_args + 3]) * nrowsPack], &tmp1[args[i_args + 4] * nrowsPack]);
                                i_args += 5;
                                break;
                            }
                            case 3: {
                                // OPERATION WITH DEST: tmp1 - SRC0: commit1 - SRC1: public
                                Goldilocks::op_pack(nrowsPack, args[i_args], &tmp1[args[i_args + 1] * nrowsPack], &bufferT_[(nColsStagesAcc[args[i_args + 2]] + args[i_args + 3]) * nrowsPack], &publics[args[i_args + 4] * nrowsPack]);
                                i_args += 5;
                                break;
                            }
                            case 4: {
                                // OPERATION WITH DEST: tmp1 - SRC0: commit1 - SRC1: number
                                Goldilocks::op_pack(nrowsPack, args[i_args], &tmp1[args[i_args + 1] * nrowsPack], &bufferT_[(nColsStagesAcc[args[i_args + 2]] + args[i_args + 3]) * nrowsPack], &numbers_[args[i_args + 4]*nrowsPack]);
                                i_args += 5;
                                break;
                            }
                            case 5: {
                                // OPERATION WITH DEST: tmp1 - SRC0: commit1 - SRC1: airvalue1
                                Goldilocks::op_pack(nrowsPack, args[i_args], &tmp1[args[i_args + 1] * nrowsPack], &bufferT_[(nColsStagesAcc[args[i_args + 2]] + args[i_args + 3]) * nrowsPack], &airValues[args[i_args + 4]*FIELD_EXTENSION*nrowsPack]);
                                i_args += 5;
                                break;
                            }
                            case 6: {
                                // COPY tmp1 to tmp1
                                Goldilocks::copy_pack(nrowsPack, &tmp1[args[i_args] * nrowsPack], &tmp1[args[i_args + 1] * nrowsPack]);
                                i_args += 2;
                                break;
                            }
                            case 7: {
                                // OPERATION WITH DEST: tmp1 - SRC0: tmp1 - SRC1: tmp1
                                Goldilocks::op_pack(nrowsPack, args[i_args], &tmp1[args[i_args + 1] * nrowsPack], &tmp1[args[i_args + 2] * nrowsPack], &tmp1[args[i_args + 3] * nrowsPack]);
                                i_args += 4;
                                break;
                            }
                            case 8: {
                                // OPERATION WITH DEST: tmp1 - SRC0: tmp1 - SRC1: public
                                Goldilocks::op_pack(nrowsPack, args[i_args], &tmp1[args[i_args + 1] * nrowsPack], &tmp1[args[i_args + 2] * nrowsPack], &publics[args[i_args + 3] * nrowsPack]);
                                i_args += 4;
                                break;
                            }
                            case 9: {
                                // OPERATION WITH DEST: tmp1 - SRC0: tmp1 - SRC1: number
                                Goldilocks::op_pack(nrowsPack, args[i_args], &tmp1[args[i_args + 1] * nrowsPack], &tmp1[args[i_args + 2] * nrowsPack], &numbers_[args[i_args + 3]*nrowsPack]);
                                i_args += 4;
                                break;
                            }
                            case 10: {
                                // OPERATION WITH DEST: tmp1 - SRC0: tmp1 - SRC1: airvalue1
                                Goldilocks::op_pack(nrowsPack, args[i_args], &tmp1[args[i_args + 1] * nrowsPack], &tmp1[args[i_args + 2] * nrowsPack], &airValues[args[i_args + 3]*FIELD_EXTENSION*nrowsPack]);
                                i_args += 4;
                                break;
                            }
                            case 11: {
                                // COPY public to tmp1
                                Goldilocks::copy_pack(nrowsPack, &tmp1[args[i_args] * nrowsPack], &publics[args[i_args + 1] * nrowsPack]);
                                i_args += 2;
                                break;
                            }
                            case 12: {
                                // OPERATION WITH DEST: tmp1 - SRC0: public - SRC1: public
                                Goldilocks::op_pack(nrowsPack, args[i_args], &tmp1[args[i_args + 1] * nrowsPack], &publics[args[i_args + 2] * nrowsPack], &publics[args[i_args + 3] * nrowsPack]);
                                i_args += 4;
                                break;
                            }
                            case 13: {
                                // OPERATION WITH DEST: tmp1 - SRC0: public - SRC1: number
                                Goldilocks::op_pack(nrowsPack, args[i_args], &tmp1[args[i_args + 1] * nrowsPack], &publics[args[i_args + 2] * nrowsPack], &numbers_[args[i_args + 3]*nrowsPack]);
                                i_args += 4;
                                break;
                            }
                            case 14: {
                                // OPERATION WITH DEST: tmp1 - SRC0: public - SRC1: airvalue1
                                Goldilocks::op_pack(nrowsPack, args[i_args], &tmp1[args[i_args + 1] * nrowsPack], &publics[args[i_args + 2] * nrowsPack], &airValues[args[i_args + 3]*FIELD_EXTENSION*nrowsPack]);
                                i_args += 4;
                                break;
                            }
                            case 15: {
                                // COPY number to tmp1
                                Goldilocks::copy_pack(nrowsPack, &tmp1[args[i_args] * nrowsPack], &numbers_[args[i_args + 1]*nrowsPack]);
                                i_args += 2;
                                break;
                            }
                            case 16: {
                                // OPERATION WITH DEST: tmp1 - SRC0: number - SRC1: number
                                Goldilocks::op_pack(nrowsPack, args[i_args], &tmp1[args[i_args + 1] * nrowsPack], &numbers_[args[i_args + 2]*nrowsPack], &numbers_[args[i_args + 3]*nrowsPack]);
                                i_args += 4;
                                break;
                            }
                            case 17: {
                                // OPERATION WITH DEST: tmp1 - SRC0: number - SRC1: airvalue1
                                Goldilocks::op_pack(nrowsPack, args[i_args], &tmp1[args[i_args + 1] * nrowsPack], &numbers_[args[i_args + 2]*nrowsPack], &airValues[args[i_args + 3]*FIELD_EXTENSION*nrowsPack]);
                                i_args += 4;
                                break;
                            }
                            case 18: {
                                // COPY airvalue1 to tmp1
                                Goldilocks::copy_pack(nrowsPack, &tmp1[args[i_args] * nrowsPack], &airValues[args[i_args + 1]*FIELD_EXTENSION*nrowsPack]);
                                i_args += 2;
                                break;
                            }
                            case 19: {
                                // OPERATION WITH DEST: tmp1 - SRC0: airvalue1 - SRC1: airvalue1
                                Goldilocks::op_pack(nrowsPack, args[i_args], &tmp1[args[i_args + 1] * nrowsPack], &airValues[args[i_args + 2]*FIELD_EXTENSION*nrowsPack], &airValues[args[i_args + 3]*FIELD_EXTENSION*nrowsPack]);
                                i_args += 4;
                                break;
                            }
                            case 20: {
                                // OPERATION WITH DEST: tmp3 - SRC0: commit3 - SRC1: commit1
                                Goldilocks3::op_31_pack(nrowsPack, args[i_args], &tmp3[args[i_args + 1] * nrowsPack * FIELD_EXTENSION], &bufferT_[(nColsStagesAcc[args[i_args + 2]] + args[i_args + 3]) * nrowsPack], &bufferT_[(nColsStagesAcc[args[i_args + 4]] + args[i_args + 5]) * nrowsPack]);
                                i_args += 6;
                                break;
                            }
                            case 21: {
                                // OPERATION WITH DEST: tmp3 - SRC0: commit3 - SRC1: tmp1
                                Goldilocks3::op_31_pack(nrowsPack, args[i_args], &tmp3[args[i_args + 1] * nrowsPack * FIELD_EXTENSION], &bufferT_[(nColsStagesAcc[args[i_args + 2]] + args[i_args + 3]) * nrowsPack], &tmp1[args[i_args + 4] * nrowsPack]);
                                i_args += 5;
                                break;
                            }
                            case 22: {
                                // OPERATION WITH DEST: tmp3 - SRC0: commit3 - SRC1: public
                                Goldilocks3::op_31_pack(nrowsPack, args[i_args], &tmp3[args[i_args + 1] * nrowsPack * FIELD_EXTENSION], &bufferT_[(nColsStagesAcc[args[i_args + 2]] + args[i_args + 3]) * nrowsPack], &publics[args[i_args + 4] * nrowsPack]);
                                i_args += 5;
                                break;
                            }
                            case 23: {
                                // OPERATION WITH DEST: tmp3 - SRC0: commit3 - SRC1: number
                                Goldilocks3::op_31_pack(nrowsPack, args[i_args], &tmp3[args[i_args + 1] * nrowsPack * FIELD_EXTENSION], &bufferT_[(nColsStagesAcc[args[i_args + 2]] + args[i_args + 3]) * nrowsPack], &numbers_[args[i_args + 4]*nrowsPack]);
                                i_args += 5;
                                break;
                            }
                            case 24: {
                                // OPERATION WITH DEST: tmp3 - SRC0: commit3 - SRC1: airvalue1
                                Goldilocks3::op_31_pack(nrowsPack, args[i_args], &tmp3[args[i_args + 1] * nrowsPack * FIELD_EXTENSION], &bufferT_[(nColsStagesAcc[args[i_args + 2]] + args[i_args + 3]) * nrowsPack], &airValues[args[i_args + 4]*FIELD_EXTENSION*nrowsPack]);
                                i_args += 5;
                                break;
                            }
                            case 25: {
                                // OPERATION WITH DEST: tmp3 - SRC0: tmp3 - SRC1: commit1
                                Goldilocks3::op_31_pack(nrowsPack, args[i_args], &tmp3[args[i_args + 1] * nrowsPack * FIELD_EXTENSION], &tmp3[args[i_args + 2] * nrowsPack * FIELD_EXTENSION], &bufferT_[(nColsStagesAcc[args[i_args + 3]] + args[i_args + 4]) * nrowsPack]);
                                i_args += 5;
                                break;
                            }
                            case 26: {
                                // OPERATION WITH DEST: tmp3 - SRC0: tmp3 - SRC1: tmp1
                                Goldilocks3::op_31_pack(nrowsPack, args[i_args], &tmp3[args[i_args + 1] * nrowsPack * FIELD_EXTENSION], &tmp3[args[i_args + 2] * nrowsPack * FIELD_EXTENSION], &tmp1[args[i_args + 3] * nrowsPack]);
                                i_args += 4;
                                break;
                            }
                            case 27: {
                                // OPERATION WITH DEST: tmp3 - SRC0: tmp3 - SRC1: public
                                Goldilocks3::op_31_pack(nrowsPack, args[i_args], &tmp3[args[i_args + 1] * nrowsPack * FIELD_EXTENSION], &tmp3[args[i_args + 2] * nrowsPack * FIELD_EXTENSION], &publics[args[i_args + 3] * nrowsPack]);
                                i_args += 4;
                                break;
                            }
                            case 28: {
                                // OPERATION WITH DEST: tmp3 - SRC0: tmp3 - SRC1: number
                                Goldilocks3::op_31_pack(nrowsPack, args[i_args], &tmp3[args[i_args + 1] * nrowsPack * FIELD_EXTENSION], &tmp3[args[i_args + 2] * nrowsPack * FIELD_EXTENSION], &numbers_[args[i_args + 3]*nrowsPack]);
                                i_args += 4;
                                break;
                            }
                            case 29: {
                                // OPERATION WITH DEST: tmp3 - SRC0: tmp3 - SRC1: airvalue1
                                Goldilocks3::op_31_pack(nrowsPack, args[i_args], &tmp3[args[i_args + 1] * nrowsPack * FIELD_EXTENSION], &tmp3[args[i_args + 2] * nrowsPack * FIELD_EXTENSION], &airValues[args[i_args + 3]*FIELD_EXTENSION*nrowsPack]);
                                i_args += 4;
                                break;
                            }
                            case 30: {
                                // OPERATION WITH DEST: tmp3 - SRC0: challenge - SRC1: commit1
                                Goldilocks3::op_31_pack(nrowsPack, args[i_args], &tmp3[args[i_args + 1] * nrowsPack * FIELD_EXTENSION], &challenges[args[i_args + 2]*FIELD_EXTENSION*nrowsPack], &bufferT_[(nColsStagesAcc[args[i_args + 3]] + args[i_args + 4]) * nrowsPack]);
                                i_args += 5;
                                break;
                            }
                            case 31: {
                                // OPERATION WITH DEST: tmp3 - SRC0: challenge - SRC1: tmp1
                                Goldilocks3::op_31_pack(nrowsPack, args[i_args], &tmp3[args[i_args + 1] * nrowsPack * FIELD_EXTENSION], &challenges[args[i_args + 2]*FIELD_EXTENSION*nrowsPack], &tmp1[args[i_args + 3] * nrowsPack]);
                                i_args += 4;
                                break;
                            }
                            case 32: {
                                // OPERATION WITH DEST: tmp3 - SRC0: challenge - SRC1: public
                                Goldilocks3::op_31_pack(nrowsPack, args[i_args], &tmp3[args[i_args + 1] * nrowsPack * FIELD_EXTENSION], &challenges[args[i_args + 2]*FIELD_EXTENSION*nrowsPack], &publics[args[i_args + 3] * nrowsPack]);
                                i_args += 4;
                                break;
                            }
                            case 33: {
                                // OPERATION WITH DEST: tmp3 - SRC0: challenge - SRC1: number
                                Goldilocks3::op_31_pack(nrowsPack, args[i_args], &tmp3[args[i_args + 1] * nrowsPack * FIELD_EXTENSION], &challenges[args[i_args + 2]*FIELD_EXTENSION*nrowsPack], &numbers_[args[i_args + 3]*nrowsPack]);
                                i_args += 4;
                                break;
                            }
                            case 34: {
                                // OPERATION WITH DEST: tmp3 - SRC0: challenge - SRC1: airvalue1
                                Goldilocks3::op_31_pack(nrowsPack, args[i_args], &tmp3[args[i_args + 1] * nrowsPack * FIELD_EXTENSION], &challenges[args[i_args + 2]*FIELD_EXTENSION*nrowsPack], &airValues[args[i_args + 3]*FIELD_EXTENSION*nrowsPack]);
                                i_args += 4;
                                break;
                            }
                            case 35: {
                                // OPERATION WITH DEST: tmp3 - SRC0: airgroupvalue - SRC1: commit1
                                Goldilocks3::op_31_pack(nrowsPack, args[i_args], &tmp3[args[i_args + 1] * nrowsPack * FIELD_EXTENSION], &airgroupValues[args[i_args + 2]*FIELD_EXTENSION*nrowsPack], &bufferT_[(nColsStagesAcc[args[i_args + 3]] + args[i_args + 4]) * nrowsPack]);
                                i_args += 5;
                                break;
                            }
                            case 36: {
                                // OPERATION WITH DEST: tmp3 - SRC0: airgroupvalue - SRC1: tmp1
                                Goldilocks3::op_31_pack(nrowsPack, args[i_args], &tmp3[args[i_args + 1] * nrowsPack * FIELD_EXTENSION], &airgroupValues[args[i_args + 2]*FIELD_EXTENSION*nrowsPack], &tmp1[args[i_args + 3] * nrowsPack]);
                                i_args += 4;
                                break;
                            }
                            case 37: {
                                // OPERATION WITH DEST: tmp3 - SRC0: airgroupvalue - SRC1: public
                                Goldilocks3::op_31_pack(nrowsPack, args[i_args], &tmp3[args[i_args + 1] * nrowsPack * FIELD_EXTENSION], &airgroupValues[args[i_args + 2]*FIELD_EXTENSION*nrowsPack], &publics[args[i_args + 3] * nrowsPack]);
                                i_args += 4;
                                break;
                            }
                            case 38: {
                                // OPERATION WITH DEST: tmp3 - SRC0: airgroupvalue - SRC1: number
                                Goldilocks3::op_31_pack(nrowsPack, args[i_args], &tmp3[args[i_args + 1] * nrowsPack * FIELD_EXTENSION], &airgroupValues[args[i_args + 2]*FIELD_EXTENSION*nrowsPack], &numbers_[args[i_args + 3]*nrowsPack]);
                                i_args += 4;
                                break;
                            }
                            case 39: {
                                // OPERATION WITH DEST: tmp3 - SRC0: airgroupvalue - SRC1: airvalue1
                                Goldilocks3::op_31_pack(nrowsPack, args[i_args], &tmp3[args[i_args + 1] * nrowsPack * FIELD_EXTENSION], &airgroupValues[args[i_args + 2]*FIELD_EXTENSION*nrowsPack], &airValues[args[i_args + 3]*FIELD_EXTENSION*nrowsPack]);
                                i_args += 4;
                                break;
                            }
                            case 40: {
                                // OPERATION WITH DEST: tmp3 - SRC0: airvalue3 - SRC1: commit1
                                Goldilocks3::op_31_pack(nrowsPack, args[i_args], &tmp3[args[i_args + 1] * nrowsPack * FIELD_EXTENSION], &airValues[args[i_args + 2]*FIELD_EXTENSION*nrowsPack], &bufferT_[(nColsStagesAcc[args[i_args + 3]] + args[i_args + 4]) * nrowsPack]);
                                i_args += 5;
                                break;
                            }
                            case 41: {
                                // OPERATION WITH DEST: tmp3 - SRC0: airvalue3 - SRC1: tmp1
                                Goldilocks3::op_31_pack(nrowsPack, args[i_args], &tmp3[args[i_args + 1] * nrowsPack * FIELD_EXTENSION], &airValues[args[i_args + 2]*FIELD_EXTENSION*nrowsPack], &tmp1[args[i_args + 3] * nrowsPack]);
                                i_args += 4;
                                break;
                            }
                            case 42: {
                                // OPERATION WITH DEST: tmp3 - SRC0: airvalue3 - SRC1: public
                                Goldilocks3::op_31_pack(nrowsPack, args[i_args], &tmp3[args[i_args + 1] * nrowsPack * FIELD_EXTENSION], &airValues[args[i_args + 2]*FIELD_EXTENSION*nrowsPack], &publics[args[i_args + 3] * nrowsPack]);
                                i_args += 4;
                                break;
                            }
                            case 43: {
                                // OPERATION WITH DEST: tmp3 - SRC0: airvalue3 - SRC1: number
                                Goldilocks3::op_31_pack(nrowsPack, args[i_args], &tmp3[args[i_args + 1] * nrowsPack * FIELD_EXTENSION], &airValues[args[i_args + 2]*FIELD_EXTENSION*nrowsPack], &numbers_[args[i_args + 3]*nrowsPack]);
                                i_args += 4;
                                break;
                            }
                            case 44: {
                                // OPERATION WITH DEST: tmp3 - SRC0: airvalue3 - SRC1: airvalue1
                                Goldilocks3::op_31_pack(nrowsPack, args[i_args], &tmp3[args[i_args + 1] * nrowsPack * FIELD_EXTENSION], &airValues[args[i_args + 2]*FIELD_EXTENSION*nrowsPack], &airValues[args[i_args + 3]*FIELD_EXTENSION*nrowsPack]);
                                i_args += 4;
                                break;
                            }
                            case 45: {
                                // COPY commit3 to tmp3
                                Goldilocks3::copy_pack(nrowsPack, &tmp3[args[i_args] * nrowsPack * FIELD_EXTENSION], &bufferT_[(nColsStagesAcc[args[i_args + 1]] + args[i_args + 2]) * nrowsPack]);
                                i_args += 3;
                                break;
                            }
                            case 46: {
                                // OPERATION WITH DEST: tmp3 - SRC0: commit3 - SRC1: commit3
                                Goldilocks3::op_pack(nrowsPack, args[i_args], &tmp3[args[i_args + 1] * nrowsPack * FIELD_EXTENSION], &bufferT_[(nColsStagesAcc[args[i_args + 2]] + args[i_args + 3]) * nrowsPack], &bufferT_[(nColsStagesAcc[args[i_args + 4]] + args[i_args + 5]) * nrowsPack]);
                                i_args += 6;
                                break;
                            }
                            case 47: {
                                // OPERATION WITH DEST: tmp3 - SRC0: commit3 - SRC1: tmp3
                                Goldilocks3::op_pack(nrowsPack, args[i_args], &tmp3[args[i_args + 1] * nrowsPack * FIELD_EXTENSION], &bufferT_[(nColsStagesAcc[args[i_args + 2]] + args[i_args + 3]) * nrowsPack], &tmp3[args[i_args + 4] * nrowsPack * FIELD_EXTENSION]);
                                i_args += 5;
                                break;
                            }
                            case 48: {
                                // OPERATION WITH DEST: tmp3 - SRC0: commit3 - SRC1: challenge
                                Goldilocks3::op_pack(nrowsPack, args[i_args], &tmp3[args[i_args + 1] * nrowsPack * FIELD_EXTENSION], &bufferT_[(nColsStagesAcc[args[i_args + 2]] + args[i_args + 3]) * nrowsPack], &challenges[args[i_args + 4]*FIELD_EXTENSION*nrowsPack]);
                                i_args += 5;
                                break;
                            }
                            case 49: {
                                // OPERATION WITH DEST: tmp3 - SRC0: commit3 - SRC1: airgroupvalue
                                Goldilocks3::op_pack(nrowsPack, args[i_args], &tmp3[args[i_args + 1] * nrowsPack * FIELD_EXTENSION], &bufferT_[(nColsStagesAcc[args[i_args + 2]] + args[i_args + 3]) * nrowsPack], &airgroupValues[args[i_args + 4]*FIELD_EXTENSION*nrowsPack]);
                                i_args += 5;
                                break;
                            }
                            case 50: {
                                // OPERATION WITH DEST: tmp3 - SRC0: commit3 - SRC1: airvalue3
                                Goldilocks3::op_pack(nrowsPack, args[i_args], &tmp3[args[i_args + 1] * nrowsPack * FIELD_EXTENSION], &bufferT_[(nColsStagesAcc[args[i_args + 2]] + args[i_args + 3]) * nrowsPack], &airValues[args[i_args + 4]*FIELD_EXTENSION*nrowsPack]);
                                i_args += 5;
                                break;
                            }
                            case 51: {
                                // COPY tmp3 to tmp3
                                Goldilocks3::copy_pack(nrowsPack, &tmp3[args[i_args] * nrowsPack * FIELD_EXTENSION], &tmp3[args[i_args + 1] * nrowsPack * FIELD_EXTENSION]);
                                i_args += 2;
                                break;
                            }
                            case 52: {
                                // OPERATION WITH DEST: tmp3 - SRC0: tmp3 - SRC1: tmp3
                                Goldilocks3::op_pack(nrowsPack, args[i_args], &tmp3[args[i_args + 1] * nrowsPack * FIELD_EXTENSION], &tmp3[args[i_args + 2] * nrowsPack * FIELD_EXTENSION], &tmp3[args[i_args + 3] * nrowsPack * FIELD_EXTENSION]);
                                i_args += 4;
                                break;
                            }
                            case 53: {
                                // OPERATION WITH DEST: tmp3 - SRC0: tmp3 - SRC1: challenge
                                Goldilocks3::op_pack(nrowsPack, args[i_args], &tmp3[args[i_args + 1] * nrowsPack * FIELD_EXTENSION], &tmp3[args[i_args + 2] * nrowsPack * FIELD_EXTENSION], &challenges[args[i_args + 3]*FIELD_EXTENSION*nrowsPack]);
                                i_args += 4;
                                break;
                            }
                            case 54: {
                                // OPERATION WITH DEST: tmp3 - SRC0: tmp3 - SRC1: airgroupvalue
                                Goldilocks3::op_pack(nrowsPack, args[i_args], &tmp3[args[i_args + 1] * nrowsPack * FIELD_EXTENSION], &tmp3[args[i_args + 2] * nrowsPack * FIELD_EXTENSION], &airgroupValues[args[i_args + 3]*FIELD_EXTENSION*nrowsPack]);
                                i_args += 4;
                                break;
                            }
                            case 55: {
                                // OPERATION WITH DEST: tmp3 - SRC0: tmp3 - SRC1: airvalue3
                                Goldilocks3::op_pack(nrowsPack, args[i_args], &tmp3[args[i_args + 1] * nrowsPack * FIELD_EXTENSION], &tmp3[args[i_args + 2] * nrowsPack * FIELD_EXTENSION], &airValues[args[i_args + 3]*FIELD_EXTENSION*nrowsPack]);
                                i_args += 4;
                                break;
                            }
                            case 56: {
                                // OPERATION WITH DEST: tmp3 - SRC0: challenge - SRC1: challenge
                                Goldilocks3::op_pack(nrowsPack, args[i_args], &tmp3[args[i_args + 1] * nrowsPack * FIELD_EXTENSION], &challenges[args[i_args + 2]*FIELD_EXTENSION*nrowsPack], &challenges[args[i_args + 3]*FIELD_EXTENSION*nrowsPack]);
                                i_args += 4;
                                break;
                            }
                            case 57: {
                                // OPERATION WITH DEST: tmp3 - SRC0: challenge - SRC1: airgroupvalue
                                Goldilocks3::op_pack(nrowsPack, args[i_args], &tmp3[args[i_args + 1] * nrowsPack * FIELD_EXTENSION], &challenges[args[i_args + 2]*FIELD_EXTENSION*nrowsPack], &airgroupValues[args[i_args + 3]*FIELD_EXTENSION*nrowsPack]);
                                i_args += 4;
                                break;
                            }
                            case 58: {
                                // OPERATION WITH DEST: tmp3 - SRC0: challenge - SRC1: airvalue3
                                Goldilocks3::op_pack(nrowsPack, args[i_args], &tmp3[args[i_args + 1] * nrowsPack * FIELD_EXTENSION], &challenges[args[i_args + 2]*FIELD_EXTENSION*nrowsPack], &airValues[args[i_args + 3]*FIELD_EXTENSION*nrowsPack]);
                                i_args += 4;
                                break;
                            }
                            case 59: {
                                // OPERATION WITH DEST: tmp3 - SRC0: airgroupvalue - SRC1: airgroupvalue
                                Goldilocks3::op_pack(nrowsPack, args[i_args], &tmp3[args[i_args + 1] * nrowsPack * FIELD_EXTENSION], &airgroupValues[args[i_args + 2]*FIELD_EXTENSION*nrowsPack], &airgroupValues[args[i_args + 3]*FIELD_EXTENSION*nrowsPack]);
                                i_args += 4;
                                break;
                            }
                            case 60: {
                                // OPERATION WITH DEST: tmp3 - SRC0: airgroupvalue - SRC1: airvalue3
                                Goldilocks3::op_pack(nrowsPack, args[i_args], &tmp3[args[i_args + 1] * nrowsPack * FIELD_EXTENSION], &airgroupValues[args[i_args + 2]*FIELD_EXTENSION*nrowsPack], &airValues[args[i_args + 3]*FIELD_EXTENSION*nrowsPack]);
                                i_args += 4;
                                break;
                            }
                            case 61: {
                                // OPERATION WITH DEST: tmp3 - SRC0: airvalue3 - SRC1: airvalue3
                                Goldilocks3::op_pack(nrowsPack, args[i_args], &tmp3[args[i_args + 1] * nrowsPack * FIELD_EXTENSION], &airValues[args[i_args + 2]*FIELD_EXTENSION*nrowsPack], &airValues[args[i_args + 3]*FIELD_EXTENSION*nrowsPack]);
                                i_args += 4;
                                break;
                            }
                            case 62: {
                                // COPY eval to tmp3
                                Goldilocks3::copy_pack(nrowsPack, &tmp3[args[i_args] * nrowsPack * FIELD_EXTENSION], &evals[args[i_args + 1]*FIELD_EXTENSION*nrowsPack]);
                                i_args += 2;
                                break;
                            }
                            case 63: {
                                // OPERATION WITH DEST: tmp3 - SRC0: challenge - SRC1: eval
                                Goldilocks3::op_pack(nrowsPack, args[i_args], &tmp3[args[i_args + 1] * nrowsPack * FIELD_EXTENSION], &challenges[args[i_args + 2]*FIELD_EXTENSION*nrowsPack], &evals[args[i_args + 3]*FIELD_EXTENSION*nrowsPack]);
                                i_args += 4;
                                break;
                            }
                            case 64: {
                                // OPERATION WITH DEST: tmp3 - SRC0: tmp3 - SRC1: eval
                                Goldilocks3::op_pack(nrowsPack, args[i_args], &tmp3[args[i_args + 1] * nrowsPack * FIELD_EXTENSION], &tmp3[args[i_args + 2] * nrowsPack * FIELD_EXTENSION], &evals[args[i_args + 3]*FIELD_EXTENSION*nrowsPack]);
                                i_args += 4;
                                break;
                            }
                            case 65: {
                                // OPERATION WITH DEST: tmp3 - SRC0: eval - SRC1: commit1
                                Goldilocks3::op_31_pack(nrowsPack, args[i_args], &tmp3[args[i_args + 1] * nrowsPack * FIELD_EXTENSION], &evals[args[i_args + 2]*FIELD_EXTENSION*nrowsPack], &bufferT_[(nColsStagesAcc[args[i_args + 3]] + args[i_args + 4]) * nrowsPack]);
                                i_args += 5;
                                break;
                            }
                            case 66: {
                                // OPERATION WITH DEST: tmp3 - SRC0: commit3 - SRC1: eval
                                Goldilocks3::op_pack(nrowsPack, args[i_args], &tmp3[args[i_args + 1] * nrowsPack * FIELD_EXTENSION], &bufferT_[(nColsStagesAcc[args[i_args + 2]] + args[i_args + 3]) * nrowsPack], &evals[args[i_args + 4]*FIELD_EXTENSION*nrowsPack]);
                                i_args += 5;
                                break;
                            }
                            case 67: {
                                // OPERATION WITH DEST: tmp3 - SRC0: eval - SRC1: eval
                                Goldilocks3::op_pack(nrowsPack, args[i_args], &tmp3[args[i_args + 1] * nrowsPack * FIELD_EXTENSION], &evals[args[i_args + 2]*FIELD_EXTENSION*nrowsPack], &evals[args[i_args + 3]*FIELD_EXTENSION*nrowsPack]);
                                i_args += 4;
                                break;
                            }
                            case 68: {
                                // OPERATION WITH DEST: tmp3 - SRC0: eval - SRC1: public
                                Goldilocks3::op_31_pack(nrowsPack, args[i_args], &tmp3[args[i_args + 1] * nrowsPack * FIELD_EXTENSION], &evals[args[i_args + 2]*FIELD_EXTENSION*nrowsPack], &publics[args[i_args + 3] * nrowsPack]);
                                i_args += 4;
                                break;
                            }
                            case 69: {
                                // OPERATION WITH DEST: tmp3 - SRC0: eval - SRC1: number
                                Goldilocks3::op_31_pack(nrowsPack, args[i_args], &tmp3[args[i_args + 1] * nrowsPack * FIELD_EXTENSION], &evals[args[i_args + 2]*FIELD_EXTENSION*nrowsPack], &numbers_[args[i_args + 3]*nrowsPack]);
                                i_args += 4;
                                break;
                            }
                            case 70: {
                                // OPERATION WITH DEST: tmp3 - SRC0: airgroupvalue - SRC1: eval
                                Goldilocks3::op_pack(nrowsPack, args[i_args], &tmp3[args[i_args + 1] * nrowsPack * FIELD_EXTENSION], &airgroupValues[args[i_args + 2]*FIELD_EXTENSION*nrowsPack], &evals[args[i_args + 3]*FIELD_EXTENSION*nrowsPack]);
                                i_args += 4;
                                break;
                            }
                            default: {
                                std::cout << " Wrong operation!" << std::endl;
                                exit(1);
                            }
                        }
                    }

                    if (i_args != dests[j].params[k].parserParams.nArgs) std::cout << " " << i_args << " - " << dests[j].params[k].parserParams.nArgs << std::endl;
                    assert(i_args == dests[j].params[k].parserParams.nArgs);

                    if(dests[j].params[k].parserParams.destDim == 1) {
                        copyPolynomial(&destVals[j][k*FIELD_EXTENSION*nrowsPack], dests[j].params[k].inverse, dests[j].params[k].parserParams.destDim, &tmp1[dests[j].params[k].parserParams.destId*nrowsPack]);
                    } else {
                        copyPolynomial(&destVals[j][k*FIELD_EXTENSION*nrowsPack], dests[j].params[k].inverse, dests[j].params[k].parserParams.destDim, &tmp3[dests[j].params[k].parserParams.destId*FIELD_EXTENSION*nrowsPack]);
                    }
                }
            }
            storePolynomial(dests, destVals, i);

            for(uint64_t j = 0; j < dests.size(); ++j) {
                delete destVals[j];
            }
            delete[] destVals;
        }
    }
};

#endif