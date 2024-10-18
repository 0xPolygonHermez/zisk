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

    inline void loadPolynomials(StepsParams& params, ParserArgs &parserArgs, std::vector<Dest> &dests, __m512i *bufferT_, uint64_t row, uint64_t domainSize) {
        uint64_t nOpenings = setupCtx.starkInfo.openingPoints.size();
        bool domainExtended = domainSize == uint64_t(1 << setupCtx.starkInfo.starkStruct.nBitsExt) ? true : false;

        uint64_t extendBits = (setupCtx.starkInfo.starkStruct.nBitsExt - setupCtx.starkInfo.starkStruct.nBits);
        int64_t extend = domainExtended ? (1 << extendBits) : 1;
        uint64_t nextStrides[nOpenings];
        for(uint64_t i = 0; i < nOpenings; ++i) {
            uint64_t opening = setupCtx.starkInfo.openingPoints[i] < 0 ? setupCtx.starkInfo.openingPoints[i] + domainSize : setupCtx.starkInfo.openingPoints[i];
            nextStrides[i] = opening * extend;
        }

        Goldilocks::Element *constPols = domainExtended ? setupCtx.constPols.pConstPolsAddressExtended : setupCtx.constPols.pConstPolsAddress;

        std::vector<bool> constPolsUsed(setupCtx.starkInfo.constPolsMap.size(), false);
        std::vector<bool> cmPolsUsed(setupCtx.starkInfo.cmPolsMap.size(), false);

        for(uint64_t i = 0; i < dests.size(); ++i) {
            for(uint64_t j = 0; j < dests[i].params.size(); ++j) {
                if(dests[i].params[j].op == opType::cm) {
                    cmPolsUsed[dests[i].params[j].polMap.polsMapId] = true;
                }
                if(dests[i].params[j].op == opType::tmp) {
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
        Goldilocks::Element bufferT[nOpenings*nrowsPack];

        for(uint64_t k = 0; k < constPolsUsed.size(); ++k) {
            if(!constPolsUsed[k]) continue;
            for(uint64_t o = 0; o < nOpenings; ++o) {
                for(uint64_t j = 0; j < nrowsPack; ++j) {
                    uint64_t l = (row + j + nextStrides[o]) % domainSize;
                    bufferT[nrowsPack*o + j] = constPols[l * nColsStages[0] + k];
                }
                Goldilocks::load_avx512(bufferT_[nColsStagesAcc[(setupCtx.starkInfo.nStages + 2)*o] + k], &bufferT[nrowsPack*o]);
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
                        bufferT[nrowsPack*o + j] = params.pols[offsetsStages[stage] + l * nColsStages[stage] + stagePos + d];
                    }
                    Goldilocks::load_avx512(bufferT_[nColsStagesAcc[(setupCtx.starkInfo.nStages + 2)*o + stage] + (stagePos + d)], &bufferT[nrowsPack*o]);
                }
            }
        }

        if(dests[0].params[0].parserParams.expId == int64_t(setupCtx.starkInfo.cExpId)) {
            for(uint64_t j = 0; j < nrowsPack; ++j) {
                bufferT[j] = setupCtx.constPols.x_2ns[row + j];
            }
            Goldilocks::load_avx512(bufferT_[nColsStagesAcc[(setupCtx.starkInfo.nStages + 2)*nOpenings]], &bufferT[0]);
            for(uint64_t d = 0; d < setupCtx.starkInfo.boundaries.size(); ++d) {
                for(uint64_t j = 0; j < nrowsPack; ++j) {
                    bufferT[j] = setupCtx.constPols.zi[row + j + d*domainSize];
                }
                Goldilocks::load_avx512(bufferT_[nColsStagesAcc[(setupCtx.starkInfo.nStages + 2)*nOpenings] + 1 + d], &bufferT[0]);
            }
        } else if(dests[0].params[0].parserParams.expId == int64_t(setupCtx.starkInfo.friExpId)) {
            for(uint64_t d = 0; d < setupCtx.starkInfo.openingPoints.size(); ++d) {
               for(uint64_t k = 0; k < FIELD_EXTENSION; ++k) {
                    for(uint64_t j = 0; j < nrowsPack; ++j) {
                        bufferT[j] = params.xDivXSub[(row + j + d*domainSize)*FIELD_EXTENSION + k];
                    }
                    Goldilocks::load_avx512(bufferT_[nColsStagesAcc[(setupCtx.starkInfo.nStages + 2)*nOpenings] + d*FIELD_EXTENSION + k], &bufferT[0]);
                }
            }
        } else {
            for(uint64_t j = 0; j < nrowsPack; ++j) {
                bufferT[j] = setupCtx.constPols.x_n[row + j];
            }
            Goldilocks::load_avx512(bufferT_[nColsStagesAcc[(setupCtx.starkInfo.nStages + 2)*nOpenings]], &bufferT[0]);
        }
    }

    inline void copyPolynomial(__m512i* destVals, bool inverse, uint64_t dim, __m512i* tmp) {
        if(dim == 1) {
            if(inverse) {
                Goldilocks::Element buff[nrowsPack];
                Goldilocks::store_avx(buff, tmp[0]);
                Goldilocks::batchInverse(buff, buff, nrowsPack);
                Goldilocks::load_avx(destVals[0], buff);
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

    inline void storePolynomial(std::vector<Dest> dests, __m512i** destVals, uint64_t row) {
        for(uint64_t i = 0; i < dests.size(); ++i) {
            __m512i vals1;
            __m512i vals3[FIELD_EXTENSION];
            uint64_t dim = 1;
            if(dests[i].params.size() == 1) {
                if(dests[i].params[0].dim == 1) {
                    Goldilocks::copy_avx512(vals1, destVals[i][0]);
                    dim = 1;
                } else {
                    Goldilocks::copy_avx512(vals3[0], destVals[i][0]);
                    Goldilocks::copy_avx512(vals3[1], destVals[i][1]);
                    Goldilocks::copy_avx512(vals3[2], destVals[i][2]);
                    dim = FIELD_EXTENSION;
                }
            } else if(dests[i].params.size() == 2) {
                if(dests[i].params[0].dim == FIELD_EXTENSION && dests[i].params[1].dim == FIELD_EXTENSION) {
                    Goldilocks3::op_avx512(2, (Goldilocks3::Element_avx512 &)vals3, (Goldilocks3::Element_avx512 &)destVals[i][0], (Goldilocks3::Element_avx512 &)destVals[i][FIELD_EXTENSION]);
                    dim = FIELD_EXTENSION;
                } else if(dests[i].params[0].dim == FIELD_EXTENSION && dests[i].params[1].dim == 1) {
                    Goldilocks3::op_31_avx512(2, (Goldilocks3::Element_avx512 &)vals3, (Goldilocks3::Element_avx512 &)destVals[i][0], destVals[i][FIELD_EXTENSION]);
                    dim = FIELD_EXTENSION;
                } else if(dests[i].params[0].dim == 1 && dests[i].params[1].dim == FIELD_EXTENSION) {
                    Goldilocks3::op_31_avx512(2, (Goldilocks3::Element_avx512 &)vals3, (Goldilocks3::Element_avx512 &)destVals[i][FIELD_EXTENSION], destVals[i][0]);
                    dim = FIELD_EXTENSION;
                } else {
                    Goldilocks::op_avx512(2, vals1, destVals[i][0], destVals[i][FIELD_EXTENSION]);
                    dim = 1;
                }
            } else {
                zklog.error("Currently only length 1 and 2 are supported");
                exitProcess();
            }
            if(dim == 1) {
                uint64_t offset = dests[i].offset != 0 ? dests[i].offset : 1;
                Goldilocks::store_avx512(&dests[i].dest[row*offset], uint64_t(offset), vals1);
            } else {
                uint64_t offset = dests[i].offset != 0 ? dests[i].offset : FIELD_EXTENSION;
                Goldilocks::store_avx512(&dests[i].dest[row*offset], uint64_t(offset), vals3[0]);
                Goldilocks::store_avx512(&dests[i].dest[row*offset + 1], uint64_t(offset),vals3[1]);
                Goldilocks::store_avx512(&dests[i].dest[row*offset + 2], uint64_t(offset), vals3[2]);
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

    void calculateExpressions(StepsParams& params, ParserArgs &parserArgs, std::vector<Dest> dests, uint64_t domainSize) override {
        uint64_t nOpenings = setupCtx.starkInfo.openingPoints.size();
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

        Goldilocks3::Element_avx512 subproofValues[setupCtx.starkInfo.nSubProofValues];
        for(uint64_t i = 0; i < setupCtx.starkInfo.nSubProofValues; ++i) {
            subproofValues[i][0] = _mm512_set1_epi64(params.subproofValues[i * FIELD_EXTENSION].fe);
            subproofValues[i][1] = _mm512_set1_epi64(params.subproofValues[i * FIELD_EXTENSION + 1].fe);
            subproofValues[i][2] = _mm512_set1_epi64(params.subproofValues[i * FIELD_EXTENSION + 2].fe);
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

                    if(dests[j].params[k].op == opType::cm) {
                        auto openingPointZero = std::find_if(setupCtx.starkInfo.openingPoints.begin(), setupCtx.starkInfo.openingPoints.end(), [](int p) { return p == 0; });
                        auto openingPointZeroIndex = std::distance(setupCtx.starkInfo.openingPoints.begin(), openingPointZero);

                        uint64_t buffPos = (setupCtx.starkInfo.nStages + 2)*openingPointZeroIndex + dests[j].params[k].polMap.stage;
                        uint64_t stagePos = dests[j].params[k].polMap.stagePos;
                        copyPolynomial(&destVals[j][k*FIELD_EXTENSION], dests[j].params[k].inverse, dests[j].params[k].polMap.dim, &bufferT_[nColsStagesAcc[buffPos] + stagePos]);
                        continue;
                    } else if(dests[j].params[k].op == opType::number) {
                        uint64_t val = dests[j].params[k].inverse ? Goldilocks::inv(Goldilocks::fromU64(dests[j].params[k].value)).fe : dests[j].params[k].value;
                        destVals[j][k*FIELD_EXTENSION] = _mm512_set1_epi64(val);
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
                                // COPY tmp1 to tmp1
                                Goldilocks::copy_avx512(tmp1[args[i_args]], tmp1[args[i_args + 1]]);
                                i_args += 2;
                                break;
                            }
                            case 6: {
                                // OPERATION WITH DEST: tmp1 - SRC0: tmp1 - SRC1: tmp1
                                Goldilocks::op_avx512(args[i_args], tmp1[args[i_args + 1]], tmp1[args[i_args + 2]], tmp1[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 7: {
                                // OPERATION WITH DEST: tmp1 - SRC0: tmp1 - SRC1: public
                                Goldilocks::op_avx512(args[i_args], tmp1[args[i_args + 1]], tmp1[args[i_args + 2]], publics[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 8: {
                                // OPERATION WITH DEST: tmp1 - SRC0: tmp1 - SRC1: number
                                Goldilocks::op_avx512(args[i_args], tmp1[args[i_args + 1]], tmp1[args[i_args + 2]], numbers_[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 9: {
                                // COPY public to tmp1
                                Goldilocks::copy_avx512(tmp1[args[i_args]], publics[args[i_args + 1]]);
                                i_args += 2;
                                break;
                            }
                            case 10: {
                                // OPERATION WITH DEST: tmp1 - SRC0: public - SRC1: public
                                Goldilocks::op_avx512(args[i_args], tmp1[args[i_args + 1]], publics[args[i_args + 2]], publics[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 11: {
                                // OPERATION WITH DEST: tmp1 - SRC0: public - SRC1: number
                                Goldilocks::op_avx512(args[i_args], tmp1[args[i_args + 1]], publics[args[i_args + 2]], numbers_[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 12: {
                                // COPY number to tmp1
                                Goldilocks::copy_avx512(tmp1[args[i_args]], numbers_[args[i_args + 1]]);
                                i_args += 2;
                                break;
                            }
                            case 13: {
                                // OPERATION WITH DEST: tmp1 - SRC0: number - SRC1: number
                                Goldilocks::op_avx512(args[i_args], tmp1[args[i_args + 1]], numbers_[args[i_args + 2]], numbers_[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 14: {
                                // OPERATION WITH DEST: tmp3 - SRC0: commit3 - SRC1: commit1
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], (Goldilocks3::Element_avx512 &)bufferT_[nColsStagesAcc[args[i_args + 2]] + args[i_args + 3]], bufferT_[nColsStagesAcc[args[i_args + 4]] + args[i_args + 5]]);
                                i_args += 6;
                                break;
                            }
                            case 15: {
                                // OPERATION WITH DEST: tmp3 - SRC0: commit3 - SRC1: tmp1
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], (Goldilocks3::Element_avx512 &)bufferT_[nColsStagesAcc[args[i_args + 2]] + args[i_args + 3]], tmp1[args[i_args + 4]]);
                                i_args += 5;
                                break;
                            }
                            case 16: {
                                // OPERATION WITH DEST: tmp3 - SRC0: commit3 - SRC1: public
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], (Goldilocks3::Element_avx512 &)bufferT_[nColsStagesAcc[args[i_args + 2]] + args[i_args + 3]], publics[args[i_args + 4]]);
                                i_args += 5;
                                break;
                            }
                            case 17: {
                                // OPERATION WITH DEST: tmp3 - SRC0: commit3 - SRC1: number
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], (Goldilocks3::Element_avx512 &)bufferT_[nColsStagesAcc[args[i_args + 2]] + args[i_args + 3]], numbers_[args[i_args + 4]]);
                                i_args += 5;
                                break;
                            }
                            case 18: {
                                // OPERATION WITH DEST: tmp3 - SRC0: tmp3 - SRC1: commit1
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], tmp3[args[i_args + 2]], bufferT_[nColsStagesAcc[args[i_args + 3]] + args[i_args + 4]]);
                                i_args += 5;
                                break;
                            }
                            case 19: {
                                // OPERATION WITH DEST: tmp3 - SRC0: tmp3 - SRC1: tmp1
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], tmp3[args[i_args + 2]], tmp1[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 20: {
                                // OPERATION WITH DEST: tmp3 - SRC0: tmp3 - SRC1: public
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], tmp3[args[i_args + 2]], publics[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 21: {
                                // OPERATION WITH DEST: tmp3 - SRC0: tmp3 - SRC1: number
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], tmp3[args[i_args + 2]], numbers_[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 22: {
                                // OPERATION WITH DEST: tmp3 - SRC0: challenge - SRC1: commit1
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], challenges[args[i_args + 2]], bufferT_[nColsStagesAcc[args[i_args + 3]] + args[i_args + 4]]);
                                i_args += 5;
                                break;
                            }
                            case 23: {
                                // OPERATION WITH DEST: tmp3 - SRC0: challenge - SRC1: tmp1
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], challenges[args[i_args + 2]], tmp1[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 24: {
                                // OPERATION WITH DEST: tmp3 - SRC0: challenge - SRC1: public
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], challenges[args[i_args + 2]], publics[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 25: {
                                // OPERATION WITH DEST: tmp3 - SRC0: challenge - SRC1: number
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], challenges[args[i_args + 2]], numbers_[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 26: {
                                // OPERATION WITH DEST: tmp3 - SRC0: subproofValue - SRC1: commit1
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], subproofValues[args[i_args + 2]], bufferT_[nColsStagesAcc[args[i_args + 3]] + args[i_args + 4]]);
                                i_args += 5;
                                break;
                            }
                            case 27: {
                                // OPERATION WITH DEST: tmp3 - SRC0: subproofValue - SRC1: tmp1
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], subproofValues[args[i_args + 2]], tmp1[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 28: {
                                // OPERATION WITH DEST: tmp3 - SRC0: subproofValue - SRC1: public
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], subproofValues[args[i_args + 2]], publics[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 29: {
                                // OPERATION WITH DEST: tmp3 - SRC0: subproofValue - SRC1: number
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], subproofValues[args[i_args + 2]], numbers_[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 30: {
                                // COPY commit3 to tmp3
                                Goldilocks3::copy_avx512(tmp3[args[i_args]], (Goldilocks3::Element_avx512 &)bufferT_[nColsStagesAcc[args[i_args + 1]] + args[i_args + 2]]);
                                i_args += 3;
                                break;
                            }
                            case 31: {
                                // OPERATION WITH DEST: tmp3 - SRC0: commit3 - SRC1: commit3
                                Goldilocks3::op_avx512(args[i_args], tmp3[args[i_args + 1]], (Goldilocks3::Element_avx512 &)bufferT_[nColsStagesAcc[args[i_args + 2]] + args[i_args + 3]], (Goldilocks3::Element_avx512 &)bufferT_[nColsStagesAcc[args[i_args + 4]] + args[i_args + 5]]);
                                i_args += 6;
                                break;
                            }
                            case 32: {
                                // OPERATION WITH DEST: tmp3 - SRC0: commit3 - SRC1: tmp3
                                Goldilocks3::op_avx512(args[i_args], tmp3[args[i_args + 1]], (Goldilocks3::Element_avx512 &)bufferT_[nColsStagesAcc[args[i_args + 2]] + args[i_args + 3]], tmp3[args[i_args + 4]]);
                                i_args += 5;
                                break;
                            }
                            case 33: {
                                // OPERATION WITH DEST: tmp3 - SRC0: commit3 - SRC1: challenge
                                Goldilocks3::op_avx512(args[i_args], tmp3[args[i_args + 1]], (Goldilocks3::Element_avx512 &)bufferT_[nColsStagesAcc[args[i_args + 2]] + args[i_args + 3]], challenges[args[i_args + 4]]);
                                i_args += 5;
                                break;
                            }
                            case 34: {
                                // OPERATION WITH DEST: tmp3 - SRC0: commit3 - SRC1: subproofValue
                                Goldilocks3::op_avx512(args[i_args], tmp3[args[i_args + 1]], (Goldilocks3::Element_avx512 &)bufferT_[nColsStagesAcc[args[i_args + 2]] + args[i_args + 3]], subproofValues[args[i_args + 4]]);
                                i_args += 5;
                                break;
                            }
                            case 35: {
                                // COPY tmp3 to tmp3
                                Goldilocks3::copy_avx512(tmp3[args[i_args]], tmp3[args[i_args + 1]]);
                                i_args += 2;
                                break;
                            }
                            case 36: {
                                // OPERATION WITH DEST: tmp3 - SRC0: tmp3 - SRC1: tmp3
                                Goldilocks3::op_avx512(args[i_args], tmp3[args[i_args + 1]], tmp3[args[i_args + 2]], tmp3[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 37: {
                                // OPERATION WITH DEST: tmp3 - SRC0: tmp3 - SRC1: challenge
                                Goldilocks3::op_avx512(args[i_args], tmp3[args[i_args + 1]], tmp3[args[i_args + 2]], challenges[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 38: {
                                // OPERATION WITH DEST: tmp3 - SRC0: tmp3 - SRC1: subproofValue
                                Goldilocks3::op_avx512(args[i_args], tmp3[args[i_args + 1]], tmp3[args[i_args + 2]], subproofValues[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 39: {
                                // OPERATION WITH DEST: tmp3 - SRC0: challenge - SRC1: challenge
                                Goldilocks3::op_avx512(args[i_args], tmp3[args[i_args + 1]], challenges[args[i_args + 2]], challenges[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 40: {
                                // OPERATION WITH DEST: tmp3 - SRC0: challenge - SRC1: subproofValue
                                Goldilocks3::op_avx512(args[i_args], tmp3[args[i_args + 1]], challenges[args[i_args + 2]], subproofValues[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 41: {
                                // OPERATION WITH DEST: tmp3 - SRC0: subproofValue - SRC1: subproofValue
                                Goldilocks3::op_avx512(args[i_args], tmp3[args[i_args + 1]], subproofValues[args[i_args + 2]], subproofValues[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 42: {
                                // COPY eval to tmp3
                                Goldilocks3::copy_avx512(tmp3[args[i_args]], evals[args[i_args + 1]]);
                                i_args += 2;
                                break;
                            }
                            case 43: {
                                // OPERATION WITH DEST: tmp3 - SRC0: challenge - SRC1: eval
                                Goldilocks3::op_avx512(args[i_args], tmp3[args[i_args + 1]], challenges[args[i_args + 2]], evals[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 44: {
                                // OPERATION WITH DEST: tmp3 - SRC0: tmp3 - SRC1: eval
                                Goldilocks3::op_avx512(args[i_args], tmp3[args[i_args + 1]], tmp3[args[i_args + 2]], evals[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 45: {
                                // OPERATION WITH DEST: tmp3 - SRC0: eval - SRC1: commit1
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], evals[args[i_args + 2]], bufferT_[nColsStagesAcc[args[i_args + 3]] + args[i_args + 4]]);
                                i_args += 5;
                                break;
                            }
                            case 46: {
                                // OPERATION WITH DEST: tmp3 - SRC0: commit3 - SRC1: eval
                                Goldilocks3::op_avx512(args[i_args], tmp3[args[i_args + 1]], (Goldilocks3::Element_avx512 &)bufferT_[nColsStagesAcc[args[i_args + 2]] + args[i_args + 3]], evals[args[i_args + 4]]);
                                i_args += 5;
                                break;
                            }
                            case 47: {
                                // OPERATION WITH DEST: tmp3 - SRC0: eval - SRC1: eval
                                Goldilocks3::op_avx512(args[i_args], tmp3[args[i_args + 1]], evals[args[i_args + 2]], evals[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 48: {
                                // OPERATION WITH DEST: tmp3 - SRC0: eval - SRC1: public
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], evals[args[i_args + 2]], publics[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 49: {
                                // OPERATION WITH DEST: tmp3 - SRC0: eval - SRC1: number
                                Goldilocks3::op_31_avx512(args[i_args], tmp3[args[i_args + 1]], evals[args[i_args + 2]], numbers_[args[i_args + 3]]);
                                i_args += 4;
                                break;
                            }
                            case 50: {
                                // OPERATION WITH DEST: tmp3 - SRC0: subproofValue - SRC1: eval
                                Goldilocks3::op_avx512(args[i_args], tmp3[args[i_args + 1]], subproofValues[args[i_args + 2]], evals[args[i_args + 3]]);
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