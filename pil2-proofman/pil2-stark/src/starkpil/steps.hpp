#ifndef STEPS_HPP
#define STEPS_HPP

#pragma once 

struct StepsParams
{
    Goldilocks::Element *trace;
    Goldilocks::Element *aux_trace;
    Goldilocks::Element *publicInputs;
    Goldilocks::Element *proofValues;
    Goldilocks::Element *challenges;
    Goldilocks::Element *airgroupValues;
    Goldilocks::Element *airValues;
    Goldilocks::Element *evals;
    Goldilocks::Element *xDivXSub;
    Goldilocks::Element *pConstPolsAddress;
    Goldilocks::Element *pConstPolsExtendedTreeAddress;
    Goldilocks::Element *pCustomCommits[10];
    Goldilocks::Element *pCustomCommitsExtended[10];
};

#endif