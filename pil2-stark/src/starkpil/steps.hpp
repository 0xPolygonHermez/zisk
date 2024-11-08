#ifndef STEPS_HPP
#define STEPS_HPP

#pragma once 

struct StepsParams
{
    Goldilocks::Element *pols;
    Goldilocks::Element *publicInputs;
    Goldilocks::Element *challenges;
    Goldilocks::Element *airgroupValues;
    Goldilocks::Element *airValues;
    Goldilocks::Element *evals;
    Goldilocks::Element *xDivXSub;
    Goldilocks::Element *pConstPolsAddress;
    Goldilocks::Element *pConstPolsExtendedTreeAddress;
    Goldilocks::Element *customCommits[10];
};

#endif