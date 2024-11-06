#ifndef EXPRESSIONS_CTX_HPP
#define EXPRESSIONS_CTX_HPP
#include "expressions_bin.hpp"
#include "const_pols.hpp"
#include "stark_info.hpp"
#include "steps.hpp"
#include "setup_ctx.hpp"

struct Params {
    ParserParams parserParams;
    uint64_t stage;
    uint64_t stagePos;
    uint64_t polsMapId;
    bool inverse = false;
    uint64_t dim;
    opType op;
    uint64_t value;
    
    Params(ParserParams& params, bool inverse_ = false) : parserParams(params), inverse(inverse_) {
        dim = params.destDim;
        op = opType::tmp;
    }

    Params(PolMap& polMap, bool inverse_ = false, bool committed = true) : inverse(inverse_) {
        dim = polMap.dim;
        stage = polMap.stage;
        stagePos = polMap.stagePos;
        polsMapId = polMap.polsMapId;
        op = committed ? opType::cm : opType::const_;
    }

    Params(uint64_t value_, bool inverse_ = false) : inverse(inverse_) {
        dim = 1;
        op = opType::number;
        value = value_;
    }
};

struct Dest {
    Goldilocks::Element *dest = nullptr;
    uint64_t offset = 0;
    std::vector<Params> params;

    Dest(Goldilocks::Element *dest_, uint64_t offset_ = false) : dest(dest_), offset(offset_) {}

    void addParams(ParserParams& parserParams_, bool inverse_ = false) {
        params.push_back(Params(parserParams_, inverse_));
    }

    void addCmPol(PolMap& cmPol, bool inverse_ = false) {
        params.push_back(Params(cmPol, inverse_, true));
    }

    void addConstPol(PolMap& constPol, bool inverse_ = false) {
        params.push_back(Params(constPol, inverse_, false));
    }

    void addNumber(uint64_t value, bool inverse_ = false) {
        params.push_back(Params(value, inverse_));
    }
};

class ExpressionsCtx {
public:

    SetupCtx setupCtx;

    ExpressionsCtx(SetupCtx& _setupCtx) : setupCtx(_setupCtx) {};

    virtual ~ExpressionsCtx() {};
    
    virtual void calculateExpressions(StepsParams& params, ParserArgs &parserArgs, std::vector<Dest> dests, uint64_t domainSize) {};
 
    void calculateExpression(StepsParams& params, Goldilocks::Element* dest, uint64_t expressionId, bool inverse = false) {
        uint64_t domainSize;
        if(expressionId == setupCtx.starkInfo.cExpId || expressionId == setupCtx.starkInfo.friExpId) {
            setupCtx.expressionsBin.expressionsInfo[expressionId].destDim = 3;
            domainSize = 1 << setupCtx.starkInfo.starkStruct.nBitsExt;
        } else {
            domainSize = 1 << setupCtx.starkInfo.starkStruct.nBits;
        }
        Dest destStruct(dest);
        destStruct.addParams(setupCtx.expressionsBin.expressionsInfo[expressionId], inverse);
        std::vector<Dest> dests = {destStruct};
        calculateExpressions(params, setupCtx.expressionsBin.expressionsBinArgsExpressions, dests, domainSize);
    }

    void multiplyExpressions(StepsParams &params, Dest &dest) {
        uint64_t domainSize = 1 << setupCtx.starkInfo.starkStruct.nBits;
        std::vector<Dest> dests = {dest};
        calculateExpressions(params, setupCtx.expressionsBin.expressionsBinArgsExpressions, dests, domainSize);
    }
};

#endif