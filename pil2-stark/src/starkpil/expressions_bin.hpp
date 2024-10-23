#ifndef BINARY_HPP
#define BINARY_HPP

#include <string>
#include <map>
#include "binfile_utils.hpp"
#include "polinomial.hpp"
#include "goldilocks_base_field.hpp"
#include "goldilocks_base_field_avx.hpp"
#include "goldilocks_base_field_avx512.hpp"
#include "goldilocks_base_field_pack.hpp"
#include "goldilocks_cubic_extension.hpp"
#include "goldilocks_cubic_extension_pack.hpp"
#include "goldilocks_cubic_extension_avx.hpp"
#include "goldilocks_cubic_extension_avx512.hpp"
#include "stark_info.hpp"
#include <immintrin.h>
#include <cassert>

const int BINARY_EXPRESSIONS_SECTION = 2;
const int BINARY_CONSTRAINTS_SECTION = 3;
const int BINARY_HINTS_SECTION = 4;

const int GLOBAL_CONSTRAINTS_SECTION = 2;
const int GLOBAL_HINTS_SECTION = 3;

struct HintFieldValue {
    opType operand;
    uint64_t id;
    uint64_t dim;
    uint64_t value;
    string stringValue;
    std::vector<uint64_t> pos;
};

struct HintField {
    string name;
    std::vector<HintFieldValue> values;
};


struct Hint
{
    std::string name;
    std::vector<HintField> fields;
};

struct VecU64Result {
    uint64_t nElements;
    uint64_t* ids;
};

struct ParserParams
{
    uint32_t stage;
    uint32_t expId;
    uint32_t nTemp1;
    uint32_t nTemp3;
    uint32_t nOps;
    uint32_t opsOffset;
    uint32_t nArgs;
    uint32_t argsOffset;
    uint32_t nConstPolsUsed;
    uint32_t constPolsOffset;
    uint32_t nCmPolsUsed;
    uint32_t cmPolsOffset;
    uint32_t nChallengesUsed;
    uint32_t challengesOffset;
    uint32_t nPublicsUsed;
    uint32_t publicsOffset;
    uint32_t nAirgroupValuesUsed;
    uint32_t airgroupValuesOffset;
    uint32_t nAirValuesUsed;
    uint32_t airValuesOffset;
    uint32_t firstRow;
    uint32_t lastRow;
    uint32_t destDim;
    uint32_t destId;
    bool imPol;
    string line;
};

struct ParserArgs 
{
    uint8_t* ops;
    uint16_t* args;
    uint64_t* numbers;
    uint16_t* constPolsIds;
    uint16_t* cmPolsIds;
    uint16_t* challengesIds;
    uint16_t* publicsIds;
    uint16_t* airgroupValuesIds;
    uint16_t* airValuesIds;
    uint64_t nNumbers;
};

class ExpressionsBin
{
public:
    std::map<uint64_t, ParserParams> expressionsInfo;

    std::vector<ParserParams> constraintsInfoDebug;

    std::vector<Hint> hints;

    ParserArgs expressionsBinArgsConstraints;
    
    ParserArgs expressionsBinArgsExpressions;

    ~ExpressionsBin() {
        if (expressionsBinArgsExpressions.ops) delete[] expressionsBinArgsExpressions.ops;
        if (expressionsBinArgsExpressions.args) delete[] expressionsBinArgsExpressions.args;
        if (expressionsBinArgsExpressions.numbers) delete[] expressionsBinArgsExpressions.numbers;
        if (expressionsBinArgsExpressions.constPolsIds) delete[] expressionsBinArgsExpressions.constPolsIds;
        if (expressionsBinArgsExpressions.cmPolsIds) delete[] expressionsBinArgsExpressions.cmPolsIds;
        if (expressionsBinArgsExpressions.challengesIds) delete[] expressionsBinArgsExpressions.challengesIds;
        if (expressionsBinArgsExpressions.publicsIds) delete[] expressionsBinArgsExpressions.publicsIds;
        if (expressionsBinArgsExpressions.airgroupValuesIds) delete[] expressionsBinArgsExpressions.airgroupValuesIds;
        if (expressionsBinArgsExpressions.airValuesIds) delete[] expressionsBinArgsExpressions.airValuesIds;

        if (expressionsBinArgsConstraints.ops) delete[] expressionsBinArgsConstraints.ops;
        if (expressionsBinArgsConstraints.args) delete[] expressionsBinArgsConstraints.args;
        if (expressionsBinArgsConstraints.numbers) delete[] expressionsBinArgsConstraints.numbers;
        if (expressionsBinArgsConstraints.constPolsIds) delete[] expressionsBinArgsConstraints.constPolsIds;
        if (expressionsBinArgsConstraints.cmPolsIds) delete[] expressionsBinArgsConstraints.cmPolsIds;
        if (expressionsBinArgsConstraints.challengesIds) delete[] expressionsBinArgsConstraints.challengesIds;
        if (expressionsBinArgsConstraints.publicsIds) delete[] expressionsBinArgsConstraints.publicsIds;
        if (expressionsBinArgsConstraints.airgroupValuesIds) delete[] expressionsBinArgsConstraints.airgroupValuesIds;
        if (expressionsBinArgsConstraints.airValuesIds) delete[] expressionsBinArgsConstraints.airValuesIds;
    };

    /* Constructor */
    ExpressionsBin(string file, bool globalBin = false);

    void loadExpressionsBin(BinFileUtils::BinFile *expressionsBin);

    void loadGlobalBin(BinFileUtils::BinFile *globalBin);

    VecU64Result getHintIdsByName(std::string name);
};


#endif
