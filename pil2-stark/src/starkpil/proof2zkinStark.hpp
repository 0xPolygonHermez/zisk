#ifndef PROOF2ZKIN__STARK_HPP
#define PROOF2ZKIN__STARK_HPP

#include <nlohmann/json.hpp>
#include "proof_stark.hpp"

using ordered_json = nlohmann::ordered_json;
using json = nlohmann::json;


ordered_json proof2zkinStark(ordered_json &proof, StarkInfo &starkInfo);
ordered_json challenges2proof(json& globalInfo, Goldilocks::Element* challenges);
ordered_json challenges2zkin(json& globalInfo, Goldilocks::Element* challenges);
ordered_json joinzkin(ordered_json &zkin1, ordered_json &zkin2, ordered_json &verKey, StarkInfo &starkInfo);
ordered_json joinzkinfinal(json& globalInfo, Goldilocks::Element* publics, Goldilocks::Element *proofValues, Goldilocks::Element* challenges, void **zkin_vec, void **starkInfo_vec);
ordered_json joinzkinrecursive2(json& globalInfo, uint64_t airgroupId, Goldilocks::Element* publics, Goldilocks::Element* challenges, ordered_json &zkin1, ordered_json &zkin2, StarkInfo &starkInfo);

ordered_json publics2zkin(ordered_json &zkin, Goldilocks::Element* publics, json& globalInfo, uint64_t airgroupId);
ordered_json addRecursive2VerKey(ordered_json &zkin, Goldilocks::Element* recursive2VerKey);

#endif