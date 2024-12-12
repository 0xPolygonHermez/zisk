#ifndef EXPRESSIONS_AVX512_HPP
#define EXPRESSIONS_AVX512_HPP
#include "expressions_ctx.hpp"

#ifdef __AVX512__

class ExpressionsAvx512 : public ExpressionsCtx {
public:
    uint64_t nrowsPack = 8;
    uint64_t nCols;
    vector<uint64_t> nColsStages;
    vector<uint64_t> nColsStagesAcc;
    vector<uint64_t> offsetsStages;
    ExpressionsAvx512(SetupCtx& setupCtx) : ExpressionsCtx(setupCtx) {};

    void setBufferTInfo(bool domainExtended, int64_t expId) {
        uint64_t nOpenings = setupCtx.starkInfo.openingPoints.size();
        uint64_t ns = 2 + setupCtx.starkInfo.nStages + setupCtx.starkInfo.customCommits.size();
        offsetsStages.resize(ns*nOpenings + 1);
        nColsStages.resize(ns*nOpenings + 1);
        nColsStagesAcc.resize(ns*nOpenings + 1);

        nCols = setupCtx.starkInfo.nConstants;

        for(uint64_t o = 0; o < nOpenings; ++o) {
            for(uint64_t stage = 0; stage < ns; ++stage) {
                if(stage == 0) {
                    offsetsStages[ns*o] = 0;
                    nColsStages[ns*o] = setupCtx.starkInfo.mapSectionsN["const"];
                    nColsStagesAcc[ns*o] = o == 0 ? 0 : nColsStagesAcc[ns*o + stage - 1] + nColsStages[stage - 1];
                } else if(stage < 2 + setupCtx.starkInfo.nStages) {
                    std::string section = "cm" + to_string(stage);
                    offsetsStages[ns*o + stage] = setupCtx.starkInfo.mapOffsets[std::make_pair(section, domainExtended)];
                    nColsStages[ns*o + stage] = setupCtx.starkInfo.mapSectionsN[section];
                    nColsStagesAcc[ns*o + stage] = nColsStagesAcc[ns*o + stage - 1] + nColsStages[stage - 1];
                } else {
                    uint64_t index = stage - setupCtx.starkInfo.nStages - 2;
                    std::string section = setupCtx.starkInfo.customCommits[index].name + "0";
                    offsetsStages[ns*o + stage] = setupCtx.starkInfo.mapOffsets[std::make_pair(section, domainExtended)];
                    nColsStages[ns*o + stage] = setupCtx.starkInfo.mapSectionsN[section];
                    nColsStagesAcc[ns*o + stage] = nColsStagesAcc[ns*o + stage - 1] + nColsStages[stage - 1];
                }
            }
        }

        nColsStagesAcc[ns*nOpenings] = nColsStagesAcc[ns*nOpenings - 1] + nColsStages[ns*nOpenings - 1];
        if(expId == int64_t(setupCtx.starkInfo.cExpId)) {
            nCols = nColsStagesAcc[ns*nOpenings] + setupCtx.starkInfo.boundaries.size() + 1;
        } else if(expId == int64_t(setupCtx.starkInfo.friExpId)) {
            nCols = nColsStagesAcc[ns*nOpenings] + nOpenings*FIELD_EXTENSION;
        } else {
            nCols = nColsStagesAcc[ns*nOpenings] + 1;
        }
    }

    inline void loadPolynomials(StepsParams& params, ParserArgs &parserArgs, std::vector<Dest> &dests, __m512i *bufferT_, uint64_t row, uint64_t domainSize) {
        uint64_t nOpenings = setupCtx.starkInfo.openingPoints.size();
        uint64_t ns = 2 + setupCtx.starkInfo.nStages + setupCtx.starkInfo.customCommits.size();
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
        std::vector<std::vector<bool>> customCommitsUsed(setupCtx.starkInfo.customCommits.size());
        for(uint64_t i = 0; i < setupCtx.starkInfo.customCommits.size(); ++i) {
            customCommitsUsed[i] = std::vector<bool>(setupCtx.starkInfo.customCommits[i].stageWidths[0], false);
        }

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

                    for(uint64_t k = 0; k < setupCtx.starkInfo.customCommits.size(); ++k) {
                        uint16_t* customCmUsed = &parserArgs.customCommitsPolsIds[dests[i].params[j].parserParams.customCommitsOffset[k]];
                        for(uint64_t l = 0; l < dests[i].params[j].parserParams.nCustomCommitsPolsUsed[k]; ++l) {
                            customCommitsUsed[k][customCmUsed[l]] = true;
                        }
                    }
                }
            }
        }
        Goldilocks::Element bufferT[nOpenings*nrowsPack];

        for(uint64_t k = 0; k < constPolsUsed.size(); ++k) {
            if(!constPolsUsed[k]) continue;
            for(uint64_t o = 0; o < nOpenings; ++o) {
                for(uint64_t j = 0; j < nrowsPack; ++j) {
                    uint64_t l = (row + j + nextStrides[o]) % domainSize;
                    bufferT[nrowsPack*o + j] = constPols[l * nColsStages[0] + k];
                }
                Goldilocks::load_avx512(bufferT_[nColsStagesAcc[ns*o] + k], &bufferT[nrowsPack*o]);
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
                        if(stage == 1 && !domainExtended) {
                            bufferT[nrowsPack*o + j] = params.trace[l * nColsStages[stage] + stagePos + d];
                        } else {
                            bufferT[nrowsPack*o + j] = params.pols[offsetsStages[stage] + l * nColsStages[stage] + stagePos + d];
                        }
                    }
                    Goldilocks::load_avx512(bufferT_[nColsStagesAcc[ns*o + stage] + (stagePos + d)], &bufferT[nrowsPack*o]);
                }
            }
        }

        for(uint64_t i = 0; i < setupCtx.starkInfo.customCommits.size(); ++i) {
            for(uint64_t j = 0; j < setupCtx.starkInfo.customCommits[i].stageWidths[0]; ++j) {
                if(!customCommitsUsed[i][j]) continue;
                PolMap polInfo = setupCtx.starkInfo.customCommitsMap[i][j];
                uint64_t stage = setupCtx.starkInfo.nStages + 2 + i;
                uint64_t stagePos = polInfo.stagePos;
                for(uint64_t d = 0; d < polInfo.dim; ++d) {
                    for(uint64_t o = 0; o < nOpenings; ++o) {
                        for(uint64_t j = 0; j < nrowsPack; ++j) {
                            uint64_t l = (row + j + nextStrides[o]) % domainSize;
                            bufferT[nrowsPack*o + j] = params.customCommits[i][offsetsStages[stage] + l * nColsStages[stage] + stagePos + d];
                        }
                        Goldilocks::load_avx(bufferT_[nColsStagesAcc[ns*o + stage] + (stagePos + d)], &bufferT[nrowsPack*o]);
                    }
                }
            }
        }

        if(dests[0].params[0].parserParams.expId == int64_t(setupCtx.starkInfo.cExpId)) {
            for(uint64_t j = 0; j < nrowsPack; ++j) {
                bufferT[j] = setupCtx.proverHelpers.x_2ns[row + j];
            }
            Goldilocks::load_avx512(bufferT_[nColsStagesAcc[ns*nOpenings]], &bufferT[0]);
            for(uint64_t d = 0; d < setupCtx.starkInfo.boundaries.size(); ++d) {
                for(uint64_t j = 0; j < nrowsPack; ++j) {
                    bufferT[j] = setupCtx.proverHelpers.zi[row + j + d*domainSize];
                }
                Goldilocks::load_avx512(bufferT_[nColsStagesAcc[ns*nOpenings] + 1 + d], &bufferT[0]);
            }
        } else if(dests[0].params[0].parserParams.expId == int64_t(setupCtx.starkInfo.friExpId)) {
            for(uint64_t d = 0; d < setupCtx.starkInfo.openingPoints.size(); ++d) {
               for(uint64_t k = 0; k < FIELD_EXTENSION; ++k) {
                    for(uint64_t j = 0; j < nrowsPack; ++j) {
                        bufferT[j] = params.xDivXSub[(row + j + d*domainSize)*FIELD_EXTENSION + k];
                    }
                    Goldilocks::load_avx512(bufferT_[nColsStagesAcc[ns*nOpenings] + d*FIELD_EXTENSION + k], &bufferT[0]);
                }
            }
        } else {
            for(uint64_t j = 0; j < nrowsPack; ++j) {
                bufferT[j] = setupCtx.proverHelpers.x_n[row + j];
            }
            Goldilocks::load_avx512(bufferT_[nColsStagesAcc[ns*nOpenings]], &bufferT[0]);
        }
    }

    inline void copyPolynomial(__m512i* destVals, bool inverse, uint64_t dim, __m512i* tmp) {
        if(dim == 1) {
            if(inverse) {
                Goldilocks::Element buff[nrowsPack];
                Goldilocks::store_avx512(buff, tmp[0]);
                Goldilocks::batchInverse(buff, buff, nrowsPack);
                Goldilocks::load_avx512(destVals[0], buff);
            } else {
                Goldilocks::copy_avx512(destVals[0],tmp[0]);
            }
        } else if(dim == FIELD_EXTENSION) {
            if(inverse) {
                Goldilocks::Element buff[FIELD_EXTENSION*nrowsPack];
                Goldilocks::store_avx512( &buff[0], uint64_t(FIELD_EXTENSION), tmp[0]);
                Goldilocks::store_avx512( &buff[1], uint64_t(FIELD_EXTENSION), tmp[1]);
                Goldilocks::store_avx512( &buff[2], uint64_t(FIELD_EXTENSION), tmp[2]);
                Goldilocks3::batchInverse((Goldilocks3::Element *)buff, (Goldilocks3::Element *)buff, nrowsPack);
                Goldilocks::load_avx512(destVals[0], &buff[0], uint64_t(FIELD_EXTENSION));
                Goldilocks::load_avx512(destVals[1], &buff[1], uint64_t(FIELD_EXTENSION));
                Goldilocks::load_avx512(destVals[2], &buff[2], uint64_t(FIELD_EXTENSION));
            } else {
                Goldilocks::copy_avx512(destVals[0], tmp[0]);
                Goldilocks::copy_avx512(destVals[1],tmp[1]);
                Goldilocks::copy_avx512(destVals[2],tmp[2]);
            }
        }
    }

    inline void multiplyPolynomials(Dest &dest, __m512i* destVals) {
        if(dest.dim == 1) {
            Goldilocks::op_avx512(2, destVals[0], destVals[0], destVals[FIELD_EXTENSION]);
        } else {
            __m512i vals3[FIELD_EXTENSION];
            if(dest.params[0].dim == FIELD_EXTENSION && dest.params[1].dim == FIELD_EXTENSION) {
                Goldilocks3::op_avx512(2, (Goldilocks3::Element_avx512 &)vals3, (Goldilocks3::Element_avx512 &)destVals[0], (Goldilocks3::Element_avx512 &)destVals[FIELD_EXTENSION]);
            } else if(dest.params[0].dim == FIELD_EXTENSION && dest.params[1].dim == 1) {
                Goldilocks3::op_31_avx512(2, (Goldilocks3::Element_avx512 &)vals3, (Goldilocks3::Element_avx512 &)destVals[0], destVals[FIELD_EXTENSION]);
            } else {
                Goldilocks3::op_31_avx512(2, (Goldilocks3::Element_avx512 &)vals3, (Goldilocks3::Element_avx512 &)destVals[FIELD_EXTENSION], destVals[0]);
            }
            Goldilocks::copy_avx512(destVals[0], vals3[0]);
            Goldilocks::copy_avx512(destVals[1], vals3[1]);
            Goldilocks::copy_avx512(destVals[2], vals3[2]);
        }
    }

    inline void storePolynomial(std::vector<Dest> dests, __m512i** destVals, uint64_t row) {
        for(uint64_t i = 0; i < dests.size(); ++i) {
            if(dests[i].dim == 1) {
                uint64_t offset = dests[i].offset != 0 ? dests[i].offset : 1;
                Goldilocks::store_avx512(&dests[i].dest[row*offset], uint64_t(offset), destVals[i][0]);
            } else {
                uint64_t offset = dests[i].offset != 0 ? dests[i].offset : FIELD_EXTENSION;
                Goldilocks::store_avx512(&dests[i].dest[row*offset], uint64_t(offset), destVals[i][0]);
                Goldilocks::store_avx512(&dests[i].dest[row*offset + 1], uint64_t(offset),destVals[i][1]);
                Goldilocks::store_avx512(&dests[i].dest[row*offset + 2], uint64_t(offset), destVals[i][2]);
            }
        }
    }

    inline void printTmp1(uint64_t row, __m512i tmp) {
        Goldilocks::Element buff[nrowsPack];
        Goldilocks::store_avx512(buff, tmp);
        for(uint64_t i = 0; i < nrowsPack; ++i) {
            cout << "Value at row " << row + i << " is " << Goldilocks::toString(buff[i]) << endl;
        }
    }

    inline void printTmp3(uint64_t row, Goldilocks3::Element_avx512 tmp) {
        Goldilocks::Element buff[FIELD_EXTENSION*nrowsPack];
        Goldilocks::store_avx512(&buff[0], uint64_t(FIELD_EXTENSION), tmp[0]);
        Goldilocks::store_avx512(&buff[1], uint64_t(FIELD_EXTENSION), tmp[1]);
        Goldilocks::store_avx512(&buff[2], uint64_t(FIELD_EXTENSION), tmp[2]);
        for(uint64_t i = 0; i < 1; ++i) {
            cout << "Value at row " << row + i << " is [" << Goldilocks::toString(buff[FIELD_EXTENSION*i]) << ", " << Goldilocks::toString(buff[FIELD_EXTENSION*i + 1]) << ", " << Goldilocks::toString(buff[FIELD_EXTENSION*i + 2]) << "]" << endl;
        }
    }

    inline void printCommit(uint64_t row, __m512i* bufferT, bool extended) {
        if(extended) {
            Goldilocks::Element buff[FIELD_EXTENSION*nrowsPack];
            Goldilocks::store_avx512(&buff[0], uint64_t(FIELD_EXTENSION), bufferT[0]);
            Goldilocks::store_avx512(&buff[1], uint64_t(FIELD_EXTENSION), bufferT[setupCtx.starkInfo.openingPoints.size()]);
            Goldilocks::store_avx512(&buff[2], uint64_t(FIELD_EXTENSION), bufferT[2*setupCtx.starkInfo.openingPoints.size()]);
            for(uint64_t i = 0; i < 1; ++i) {
                cout << "Value at row " << row + i << " is [" << Goldilocks::toString(buff[FIELD_EXTENSION*i]) << ", " << Goldilocks::toString(buff[FIELD_EXTENSION*i + 1]) << ", " << Goldilocks::toString(buff[FIELD_EXTENSION*i + 2]) << "]" << endl;
            }
        } else {
            Goldilocks::Element buff[nrowsPack];
            Goldilocks::store_avx512(&buff[0], bufferT[0]);
            for(uint64_t i = 0; i < nrowsPack; ++i) {
                cout << "Value at row " << row + i << " is " << Goldilocks::toString(buff[i]) << endl;
            }
        }
    }

    void calculateExpressions(StepsParams& params, ParserArgs &parserArgs, std::vector<Dest> dests, uint64_t domainSize, bool compilation_time) override {
        uint64_t nOpenings = setupCtx.starkInfo.openingPoints.size();
        uint64_t ns = 2 + setupCtx.starkInfo.nStages + setupCtx.starkInfo.customCommits.size();
        bool domainExtended = domainSize == uint64_t(1 << setupCtx.starkInfo.starkStruct.nBitsExt) ? true : false;

        uint64_t expId = dests[0].params[0].op == opType::tmp ? dests[0].params[0].parserParams.destDim : 0;
        setBufferTInfo(domainExtended, expId);

        Goldilocks3::Element_avx512 challenges[setupCtx.starkInfo.challengesMap.size()];
        for(uint64_t i = 0; i < setupCtx.starkInfo.challengesMap.size(); ++i) {
            challenges[i][0] = _mm512_set1_epi64(params.challenges[i * FIELD_EXTENSION].fe);
            challenges[i][1] = _mm512_set1_epi64(params.challenges[i * FIELD_EXTENSION + 1].fe);
            challenges[i][2] = _mm512_set1_epi64(params.challenges[i * FIELD_EXTENSION + 2].fe);

        }

        __m512i numbers_[parserArgs.nNumbers];
        for(uint64_t i = 0; i < parserArgs.nNumbers; ++i) {
            numbers_[i] = _mm512_set1_epi64(parserArgs.numbers[i]);
        }

        __m512i publics[setupCtx.starkInfo.nPublics];
        for(uint64_t i = 0; i < setupCtx.starkInfo.nPublics; ++i) {
            publics[i] = _mm512_set1_epi64(params.publicInputs[i].fe);
        }

        Goldilocks3::Element_avx512 proofValues[setupCtx.starkInfo.proofValuesMap.size()];
        for(uint64_t i = 0; i < setupCtx.starkInfo.proofValuesMap.size(); ++i) {
            proofValues[i][0] = _mm512_set1_epi64(params.proofValues[i * FIELD_EXTENSION].fe);
            proofValues[i][1] = _mm512_set1_epi64(params.proofValues[i * FIELD_EXTENSION + 1].fe);
            proofValues[i][2] = _mm512_set1_epi64(params.proofValues[i * FIELD_EXTENSION + 2].fe);
        }

        Goldilocks3::Element_avx512 airgroupValues[setupCtx.starkInfo.airgroupValuesMap.size()];
        for(uint64_t i = 0; i < setupCtx.starkInfo.airgroupValuesMap.size(); ++i) {
            airgroupValues[i][0] = _mm512_set1_epi64(params.airgroupValues[i * FIELD_EXTENSION].fe);
            airgroupValues[i][1] = _mm512_set1_epi64(params.airgroupValues[i * FIELD_EXTENSION + 1].fe);
            airgroupValues[i][2] = _mm512_set1_epi64(params.airgroupValues[i * FIELD_EXTENSION + 2].fe);
        }

        Goldilocks3::Element_avx512 airValues[setupCtx.starkInfo.airValuesMap.size()];
        for(uint64_t i = 0; i < setupCtx.starkInfo.airValuesMap.size(); ++i) {
            airValues[i][0] = _mm512_set1_epi64(params.airValues[i * FIELD_EXTENSION].fe);
            airValues[i][1] = _mm512_set1_epi64(params.airValues[i * FIELD_EXTENSION + 1].fe);
            airValues[i][2] = _mm512_set1_epi64(params.airValues[i * FIELD_EXTENSION + 2].fe);
        }

        Goldilocks3::Element_avx512 evals[setupCtx.starkInfo.evMap.size()];
        for(uint64_t i = 0; i < setupCtx.starkInfo.evMap.size(); ++i) {
            evals[i][0] = _mm512_set1_epi64(params.evals[i * FIELD_EXTENSION].fe);
            evals[i][1] = _mm512_set1_epi64(params.evals[i * FIELD_EXTENSION + 1].fe);
            evals[i][2] = _mm512_set1_epi64(params.evals[i * FIELD_EXTENSION + 2].fe);
        }

    #pragma omp parallel for
        for (uint64_t i = 0; i < domainSize; i+= nrowsPack) {
            __m512i bufferT_[nOpenings*nCols];

            loadPolynomials(params, parserArgs, dests, bufferT_, i, domainSize);

            __m512i** destVals = new __m512i*[dests.size()];

            for(uint64_t j = 0; j < dests.size(); ++j) {
                destVals[j] = new __m512i[dests[j].params.size() * FIELD_EXTENSION];
                for(uint64_t k = 0; k < dests[j].params.size(); ++k) {
                    uint64_t i_args = 0;

                    if(dests[j].params[k].op == opType::cm || dests[j].params[k].op == opType::const_) {
                        uint64_t openingPointIndex = dests[j].params[k].rowOffsetIndex;
                        uint64_t buffPos = ns*openingPointIndex + dests[j].params[k].stage;
                        uint64_t stagePos = dests[j].params[k].stagePos;
                        copyPolynomial(&destVals[j][k*FIELD_EXTENSION], dests[j].params[k].inverse,dests[j].params[k].dim, &bufferT_[nColsStagesAcc[buffPos] + stagePos]);
                        continue;
                    } else if(dests[j].params[k].op == opType::number) {
                        destVals[j][k*FIELD_EXTENSION] = _mm512_set1_epi64(dests[j].params[k].value);
                        continue;
                    }

                    uint8_t* ops = &parserArgs.ops[dests[j].params[k].parserParams.opsOffset];
                    uint16_t* args = &parserArgs.args[dests[j].params[k].parserParams.argsOffset];
                    __m512i tmp1[dests[j].params[k].parserParams.nTemp1];
                    Goldilocks3::Element_avx512 tmp3[dests[j].params[k].parserParams.nTemp3];

                    for (uint64_t kk = 0; kk < dests[j].params[k].parserParams.nOps; ++kk) {
                        switch (ops[kk]) {
                            case 0: {
                                // COPY commit1 to tmp1
                                Goldilocks::copy_avx512(tmp1[args[i_args]], bufferT_[nColsStagesAcc[args[i_args + 1]] + args[i_args + 2]]);
                                i_args += 3;
                                break;
                            }
                            case 1: {
                                // OPERATION WITH DEST: tmp1 - SRC0: commit1 - SRC1: commit1
                                Goldilocks::op_avx512(args[i_args], tmp1[args[i_args + 1]], bufferT_[nColsStagesAcc[args[i_args + 2]] + args[i_args + 3]], bufferT_[nColsStagesAcc[args[i_args + 4]] + args[i_args + 5]]);
                                i_args += 6;
                                break;
                            }
                            case 2: {
                                // OPERATION WITH DEST: tmp1 - SRC0: commit1 - SRC1: tmp1
                                Goldilocks::op_avx512(args[i_args], tmp1[args[i_args + 1]], bufferT_[nColsStagesAcc[args[i_args + 2]] + args[i_args + 3]], tmp1[args[i_args + 4]]);
                                i_args += 5;
                                break;
                            }
                            case 3: {
                                // OPERATION WITH DEST: tmp1 - SRC0: commit1 - SRC1: public
                                Goldilocks::op_avx512(args[i_args], tmp1[args[i_args + 1]], bufferT_[nColsStagesAcc[args[i_args + 2]] + args[i_args + 3]], publics[args[i_args + 4]]);
                                i_args += 5;
                                break;
                            }
                            case 4: {
                                // OPERATION WITH DEST: tmp1 - SRC0: commit1 - SRC1: number
                                Goldilocks::op_avx512(args[i_args], tmp1[args[i_args + 1]], bufferT_[nColsStagesAcc[args[i_args + 2]] + args[i_args + 3]], numbers_[args[i_args + 4]]);
                                i_args += 5;
                                break;
                            }
                            case 5: {
                                // OPERATION WITH DEST: tmp1 - SRC0: commit1 - SRC1: airvalue1
                                Goldilocks::op_avx512(args[i_args], tmp1[args[i_args + 1]], bufferT_[nColsStagesAcc[args[i_args + 2]] + args[i_args + 3]], airValues[args[i_args + 4]][0]);
                                i_args += 5;
                                break;
                            }
                            case 6: {
                                // COPY tmp1 to tmp1
                                Goldilocks::copy_avx512(tmp1[args[i_args]], tmp1[args[i_args + 1]]);
                                i_args += 2;
                                break;
                            }
                            case 7: {
                                // OPERATION WITH DEST: tmp1 - SRC0: tmp1 - SRC1: tmp1
                                Goldilocks::op_avx512(args[i_args], tmp1[args[i_args + 1]], tmp1[args[i_args + 2]], tmp1[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 8: {
                                // OPERATION WITH DEST: tmp1 - SRC0: tmp1 - SRC1: public
                                Goldilocks::op_avx512(args[i_args], tmp1[args[i_args + 1]], tmp1[args[i_args + 2]], publics[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 9: {
                                // OPERATION WITH DEST: tmp1 - SRC0: tmp1 - SRC1: number
                                Goldilocks::op_avx512(args[i_args], tmp1[args[i_args + 1]], tmp1[args[i_args + 2]], numbers_[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 10: {
                                // OPERATION WITH DEST: tmp1 - SRC0: tmp1 - SRC1: airvalue1
                                Goldilocks::op_avx512(args[i_args], tmp1[args[i_args + 1]], tmp1[args[i_args + 2]], airValues[args[i_args + 3]][0]);
                                i_args += 4;
                                break;
                            }
                            case 11: {
                                // COPY public to tmp1
                                Goldilocks::copy_avx512(tmp1[args[i_args]], publics[args[i_args + 1]]);
                                i_args += 2;
                                break;
                            }
                            case 12: {
                                // OPERATION WITH DEST: tmp1 - SRC0: public - SRC1: public
                                Goldilocks::op_avx512(args[i_args], tmp1[args[i_args + 1]], publics[args[i_args + 2]], publics[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 13: {
                                // OPERATION WITH DEST: tmp1 - SRC0: public - SRC1: number
                                Goldilocks::op_avx512(args[i_args], tmp1[args[i_args + 1]], publics[args[i_args + 2]], numbers_[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 14: {
                                // OPERATION WITH DEST: tmp1 - SRC0: public - SRC1: airvalue1
                                Goldilocks::op_avx512(args[i_args], tmp1[args[i_args + 1]], publics[args[i_args + 2]], airValues[args[i_args + 3]][0]);
                                i_args += 4;
                                break;
                            }
                            case 15: {
                                // COPY number to tmp1
                                Goldilocks::copy_avx512(tmp1[args[i_args]], numbers_[args[i_args + 1]]);
                                i_args += 2;
                                break;
                            }
                            case 16: {
                                // OPERATION WITH DEST: tmp1 - SRC0: number - SRC1: number
                                Goldilocks::op_avx512(args[i_args], tmp1[args[i_args + 1]], numbers_[args[i_args + 2]], numbers_[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 17: {
                                // OPERATION WITH DEST: tmp1 - SRC0: number - SRC1: airvalue1
                                Goldilocks::op_avx512(args[i_args], tmp1[args[i_args + 1]], numbers_[args[i_args + 2]], airValues[args[i_args + 3]][0]);
                                i_args += 4;
                                break;
                            }
                            case 18: {
                                // COPY airvalue1 to tmp1
                                Goldilocks::copy_avx512(tmp1[args[i_args]], airValues[args[i_args + 1]][0]);
                                i_args += 2;
                                break;
                            }
                            case 19: {
                                // OPERATION WITH DEST: tmp1 - SRC0: airvalue1 - SRC1: airvalue1
                                Goldilocks::op_avx512(args[i_args], tmp1[args[i_args + 1]], airValues[args[i_args + 2]][0], airValues[args[i_args + 3]][0]);
                                i_args += 4;
                                break;
                            }
                            case 20: {
                                // OPERATION WITH DEST: tmp3 - SRC0: commit3 - SRC1: commit1
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], (Goldilocks3::Element_avx512 &)bufferT_[nColsStagesAcc[args[i_args + 2]] + args[i_args + 3]], bufferT_[nColsStagesAcc[args[i_args + 4]] + args[i_args + 5]]);
                                i_args += 6;
                                break;
                            }
                            case 21: {
                                // OPERATION WITH DEST: tmp3 - SRC0: commit3 - SRC1: tmp1
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], (Goldilocks3::Element_avx512 &)bufferT_[nColsStagesAcc[args[i_args + 2]] + args[i_args + 3]], tmp1[args[i_args + 4]]);
                                i_args += 5;
                                break;
                            }
                            case 22: {
                                // OPERATION WITH DEST: tmp3 - SRC0: commit3 - SRC1: public
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], (Goldilocks3::Element_avx512 &)bufferT_[nColsStagesAcc[args[i_args + 2]] + args[i_args + 3]], publics[args[i_args + 4]]);
                                i_args += 5;
                                break;
                            }
                            case 23: {
                                // OPERATION WITH DEST: tmp3 - SRC0: commit3 - SRC1: number
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], (Goldilocks3::Element_avx512 &)bufferT_[nColsStagesAcc[args[i_args + 2]] + args[i_args + 3]], numbers_[args[i_args + 4]]);
                                i_args += 5;
                                break;
                            }
                            case 24: {
                                // OPERATION WITH DEST: tmp3 - SRC0: commit3 - SRC1: airvalue1
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], (Goldilocks3::Element_avx512 &)bufferT_[nColsStagesAcc[args[i_args + 2]] + args[i_args + 3]], airValues[args[i_args + 4]][0]);
                                i_args += 5;
                                break;
                            }
                            case 25: {
                                // OPERATION WITH DEST: tmp3 - SRC0: tmp3 - SRC1: commit1
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], tmp3[args[i_args + 2]], bufferT_[nColsStagesAcc[args[i_args + 3]] + args[i_args + 4]]);
                                i_args += 5;
                                break;
                            }
                            case 26: {
                                // OPERATION WITH DEST: tmp3 - SRC0: tmp3 - SRC1: tmp1
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], tmp3[args[i_args + 2]], tmp1[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 27: {
                                // OPERATION WITH DEST: tmp3 - SRC0: tmp3 - SRC1: public
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], tmp3[args[i_args + 2]], publics[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 28: {
                                // OPERATION WITH DEST: tmp3 - SRC0: tmp3 - SRC1: number
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], tmp3[args[i_args + 2]], numbers_[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 29: {
                                // OPERATION WITH DEST: tmp3 - SRC0: tmp3 - SRC1: airvalue1
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], tmp3[args[i_args + 2]], airValues[args[i_args + 3]][0]);
                                i_args += 4;
                                break;
                            }
                            case 30: {
                                // OPERATION WITH DEST: tmp3 - SRC0: challenge - SRC1: commit1
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], challenges[args[i_args + 2]], bufferT_[nColsStagesAcc[args[i_args + 3]] + args[i_args + 4]]);
                                i_args += 5;
                                break;
                            }
                            case 31: {
                                // OPERATION WITH DEST: tmp3 - SRC0: challenge - SRC1: tmp1
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], challenges[args[i_args + 2]], tmp1[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 32: {
                                // OPERATION WITH DEST: tmp3 - SRC0: challenge - SRC1: public
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], challenges[args[i_args + 2]], publics[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 33: {
                                // OPERATION WITH DEST: tmp3 - SRC0: challenge - SRC1: number
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], challenges[args[i_args + 2]], numbers_[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 34: {
                                // OPERATION WITH DEST: tmp3 - SRC0: challenge - SRC1: airvalue1
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], challenges[args[i_args + 2]], airValues[args[i_args + 3]][0]);
                                i_args += 4;
                                break;
                            }
                            case 35: {
                                // OPERATION WITH DEST: tmp3 - SRC0: airgroupvalue - SRC1: commit1
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], airgroupValues[args[i_args + 2]], bufferT_[nColsStagesAcc[args[i_args + 3]] + args[i_args + 4]]);
                                i_args += 5;
                                break;
                            }
                            case 36: {
                                // OPERATION WITH DEST: tmp3 - SRC0: airgroupvalue - SRC1: tmp1
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], airgroupValues[args[i_args + 2]], tmp1[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 37: {
                                // OPERATION WITH DEST: tmp3 - SRC0: airgroupvalue - SRC1: public
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], airgroupValues[args[i_args + 2]], publics[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 38: {
                                // OPERATION WITH DEST: tmp3 - SRC0: airgroupvalue - SRC1: number
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], airgroupValues[args[i_args + 2]], numbers_[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 39: {
                                // OPERATION WITH DEST: tmp3 - SRC0: airgroupvalue - SRC1: airvalue1
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], airgroupValues[args[i_args + 2]], airValues[args[i_args + 3]][0]);
                                i_args += 4;
                                break;
                            }
                            case 40: {
                                // OPERATION WITH DEST: tmp3 - SRC0: airvalue3 - SRC1: commit1
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], airValues[args[i_args + 2]], bufferT_[nColsStagesAcc[args[i_args + 3]] + args[i_args + 4]]);
                                i_args += 5;
                                break;
                            }
                            case 41: {
                                // OPERATION WITH DEST: tmp3 - SRC0: airvalue3 - SRC1: tmp1
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], airValues[args[i_args + 2]], tmp1[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 42: {
                                // OPERATION WITH DEST: tmp3 - SRC0: airvalue3 - SRC1: public
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], airValues[args[i_args + 2]], publics[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 43: {
                                // OPERATION WITH DEST: tmp3 - SRC0: airvalue3 - SRC1: number
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], airValues[args[i_args + 2]], numbers_[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 44: {
                                // OPERATION WITH DEST: tmp3 - SRC0: airvalue3 - SRC1: airvalue1
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], airValues[args[i_args + 2]], airValues[args[i_args + 3]][0]);
                                i_args += 4;
                                break;
                            }
                            case 45: {
                                // OPERATION WITH DEST: tmp3 - SRC0: proofvalue - SRC1: commit1
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], proofValues[args[i_args + 2]], bufferT_[nColsStagesAcc[args[i_args + 3]] + args[i_args + 4]]);
                                i_args += 5;
                                break;
                            }
                            case 46: {
                                // OPERATION WITH DEST: tmp3 - SRC0: proofvalue - SRC1: tmp1
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], proofValues[args[i_args + 2]], tmp1[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 47: {
                                // OPERATION WITH DEST: tmp3 - SRC0: proofvalue - SRC1: public
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], proofValues[args[i_args + 2]], publics[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 48: {
                                // OPERATION WITH DEST: tmp3 - SRC0: proofvalue - SRC1: number
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], proofValues[args[i_args + 2]], numbers_[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 49: {
                                // OPERATION WITH DEST: tmp3 - SRC0: proofvalue - SRC1: airvalue1
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], proofValues[args[i_args + 2]], airValues[args[i_args + 3]][0]);
                                i_args += 4;
                                break;
                            }
                            case 50: {
                                // COPY commit3 to tmp3
                                Goldilocks3::copy_avx512(tmp3[args[i_args]], (Goldilocks3::Element_avx512 &)bufferT_[nColsStagesAcc[args[i_args + 1]] + args[i_args + 2]]);
                                i_args += 3;
                                break;
                            }
                            case 51: {
                                // OPERATION WITH DEST: tmp3 - SRC0: commit3 - SRC1: commit3
                                Goldilocks3::op_avx512(args[i_args], tmp3[args[i_args + 1]], (Goldilocks3::Element_avx512 &)bufferT_[nColsStagesAcc[args[i_args + 2]] + args[i_args + 3]], (Goldilocks3::Element_avx512 &)bufferT_[nColsStagesAcc[args[i_args + 4]] + args[i_args + 5]]);
                                i_args += 6;
                                break;
                            }
                            case 52: {
                                // OPERATION WITH DEST: tmp3 - SRC0: commit3 - SRC1: tmp3
                                Goldilocks3::op_avx512(args[i_args], tmp3[args[i_args + 1]], (Goldilocks3::Element_avx512 &)bufferT_[nColsStagesAcc[args[i_args + 2]] + args[i_args + 3]], tmp3[args[i_args + 4]]);
                                i_args += 5;
                                break;
                            }
                            case 53: {
                                // OPERATION WITH DEST: tmp3 - SRC0: commit3 - SRC1: challenge
                                Goldilocks3::op_avx512(args[i_args], tmp3[args[i_args + 1]], (Goldilocks3::Element_avx512 &)bufferT_[nColsStagesAcc[args[i_args + 2]] + args[i_args + 3]], challenges[args[i_args + 4]]);
                                i_args += 5;
                                break;
                            }
                            case 54: {
                                // OPERATION WITH DEST: tmp3 - SRC0: commit3 - SRC1: airgroupvalue
                                Goldilocks3::op_avx512(args[i_args], tmp3[args[i_args + 1]], (Goldilocks3::Element_avx512 &)bufferT_[nColsStagesAcc[args[i_args + 2]] + args[i_args + 3]], airgroupValues[args[i_args + 4]]);
                                i_args += 5;
                                break;
                            }
                            case 55: {
                                // OPERATION WITH DEST: tmp3 - SRC0: commit3 - SRC1: airvalue3
                                Goldilocks3::op_avx512(args[i_args], tmp3[args[i_args + 1]], (Goldilocks3::Element_avx512 &)bufferT_[nColsStagesAcc[args[i_args + 2]] + args[i_args + 3]], airValues[args[i_args + 4]]);
                                i_args += 5;
                                break;
                            }
                            case 56: {
                                // OPERATION WITH DEST: tmp3 - SRC0: commit3 - SRC1: proofvalue
                                Goldilocks3::op_avx512(args[i_args], tmp3[args[i_args + 1]], (Goldilocks3::Element_avx512 &)bufferT_[nColsStagesAcc[args[i_args + 2]] + args[i_args + 3]], proofValues[args[i_args + 4]]);
                                i_args += 5;
                                break;
                            }
                            case 57: {
                                // COPY tmp3 to tmp3
                                Goldilocks3::copy_avx512(tmp3[args[i_args]], tmp3[args[i_args + 1]]);
                                i_args += 2;
                                break;
                            }
                            case 58: {
                                // OPERATION WITH DEST: tmp3 - SRC0: tmp3 - SRC1: tmp3
                                Goldilocks3::op_avx512(args[i_args], tmp3[args[i_args + 1]], tmp3[args[i_args + 2]], tmp3[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 59: {
                                // OPERATION WITH DEST: tmp3 - SRC0: tmp3 - SRC1: challenge
                                Goldilocks3::op_avx512(args[i_args], tmp3[args[i_args + 1]], tmp3[args[i_args + 2]], challenges[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 60: {
                                // OPERATION WITH DEST: tmp3 - SRC0: tmp3 - SRC1: airgroupvalue
                                Goldilocks3::op_avx512(args[i_args], tmp3[args[i_args + 1]], tmp3[args[i_args + 2]], airgroupValues[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 61: {
                                // OPERATION WITH DEST: tmp3 - SRC0: tmp3 - SRC1: airvalue3
                                Goldilocks3::op_avx512(args[i_args], tmp3[args[i_args + 1]], tmp3[args[i_args + 2]], airValues[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 62: {
                                // OPERATION WITH DEST: tmp3 - SRC0: tmp3 - SRC1: proofvalue
                                Goldilocks3::op_avx512(args[i_args], tmp3[args[i_args + 1]], tmp3[args[i_args + 2]], proofValues[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 63: {
                                // OPERATION WITH DEST: tmp3 - SRC0: challenge - SRC1: challenge
                                Goldilocks3::op_avx512(args[i_args], tmp3[args[i_args + 1]], challenges[args[i_args + 2]], challenges[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 64: {
                                // OPERATION WITH DEST: tmp3 - SRC0: challenge - SRC1: airgroupvalue
                                Goldilocks3::op_avx512(args[i_args], tmp3[args[i_args + 1]], challenges[args[i_args + 2]], airgroupValues[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 65: {
                                // OPERATION WITH DEST: tmp3 - SRC0: challenge - SRC1: airvalue3
                                Goldilocks3::op_avx512(args[i_args], tmp3[args[i_args + 1]], challenges[args[i_args + 2]], airValues[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 66: {
                                // OPERATION WITH DEST: tmp3 - SRC0: challenge - SRC1: proofvalue
                                Goldilocks3::op_avx512(args[i_args], tmp3[args[i_args + 1]], challenges[args[i_args + 2]], proofValues[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 67: {
                                // OPERATION WITH DEST: tmp3 - SRC0: airgroupvalue - SRC1: airgroupvalue
                                Goldilocks3::op_avx512(args[i_args], tmp3[args[i_args + 1]], airgroupValues[args[i_args + 2]], airgroupValues[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 68: {
                                // OPERATION WITH DEST: tmp3 - SRC0: airgroupvalue - SRC1: airvalue3
                                Goldilocks3::op_avx512(args[i_args], tmp3[args[i_args + 1]], airgroupValues[args[i_args + 2]], airValues[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 69: {
                                // OPERATION WITH DEST: tmp3 - SRC0: airgroupvalue - SRC1: proofvalue
                                Goldilocks3::op_avx512(args[i_args], tmp3[args[i_args + 1]], airgroupValues[args[i_args + 2]], proofValues[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 70: {
                                // OPERATION WITH DEST: tmp3 - SRC0: airvalue3 - SRC1: airvalue3
                                Goldilocks3::op_avx512(args[i_args], tmp3[args[i_args + 1]], airValues[args[i_args + 2]], airValues[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 71: {
                                // OPERATION WITH DEST: tmp3 - SRC0: airvalue3 - SRC1: proofvalue
                                Goldilocks3::op_avx512(args[i_args], tmp3[args[i_args + 1]], airValues[args[i_args + 2]], proofValues[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 72: {
                                // OPERATION WITH DEST: tmp3 - SRC0: proofvalue - SRC1: proofvalue
                                Goldilocks3::op_avx512(args[i_args], tmp3[args[i_args + 1]], proofValues[args[i_args + 2]], proofValues[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 73: {
                                // COPY eval to tmp3
                                Goldilocks3::copy_avx512(tmp3[args[i_args]], evals[args[i_args + 1]]);
                                i_args += 2;
                                break;
                            }
                            case 74: {
                                // OPERATION WITH DEST: tmp3 - SRC0: challenge - SRC1: eval
                                Goldilocks3::op_avx512(args[i_args], tmp3[args[i_args + 1]], challenges[args[i_args + 2]], evals[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 75: {
                                // OPERATION WITH DEST: tmp3 - SRC0: tmp3 - SRC1: eval
                                Goldilocks3::op_avx512(args[i_args], tmp3[args[i_args + 1]], tmp3[args[i_args + 2]], evals[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 76: {
                                // OPERATION WITH DEST: tmp3 - SRC0: eval - SRC1: commit1
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], evals[args[i_args + 2]], bufferT_[nColsStagesAcc[args[i_args + 3]] + args[i_args + 4]]);
                                i_args += 5;
                                break;
                            }
                            case 77: {
                                // OPERATION WITH DEST: tmp3 - SRC0: commit3 - SRC1: eval
                                Goldilocks3::op_avx512(args[i_args], tmp3[args[i_args + 1]], (Goldilocks3::Element_avx512 &)bufferT_[nColsStagesAcc[args[i_args + 2]] + args[i_args + 3]], evals[args[i_args + 4]]);
                                i_args += 5;
                                break;
                            }
                            case 78: {
                                // OPERATION WITH DEST: tmp3 - SRC0: eval - SRC1: eval
                                Goldilocks3::op_avx512(args[i_args], tmp3[args[i_args + 1]], evals[args[i_args + 2]], evals[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 79: {
                                // OPERATION WITH DEST: tmp3 - SRC0: eval - SRC1: public
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], evals[args[i_args + 2]], publics[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 80: {
                                // OPERATION WITH DEST: tmp3 - SRC0: eval - SRC1: number
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], evals[args[i_args + 2]], numbers_[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 81: {
                                // OPERATION WITH DEST: tmp3 - SRC0: airgroupvalue - SRC1: eval
                                Goldilocks3::op_avx512(args[i_args], tmp3[args[i_args + 1]], airgroupValues[args[i_args + 2]], evals[args[i_args + 3]]);
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
                        copyPolynomial(&destVals[j][k*FIELD_EXTENSION], dests[j].params[k].inverse, dests[j].params[k].parserParams.destDim, &tmp1[dests[j].params[k].parserParams.destId]);
                    } else {
                        copyPolynomial(&destVals[j][k*FIELD_EXTENSION], dests[j].params[k].inverse, dests[j].params[k].parserParams.destDim, tmp3[dests[j].params[k].parserParams.destId]);
                    }
                }
                if(dests[j].params.size() == 2) {
                    multiplyPolynomials(dests[j], destVals[j]);
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
#endif