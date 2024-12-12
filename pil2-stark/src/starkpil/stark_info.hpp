#ifndef STARK_INFO_HPP
#define STARK_INFO_HPP

#include <nlohmann/json.hpp>
#include <string>
#include <vector>
#include "zkassert.hpp"
#include "goldilocks_base_field.hpp"
#include "polinomial.hpp"
#include "merklehash_goldilocks.hpp"
#include "zklog.hpp"
#include "exit_process.hpp"

using json = nlohmann::json;
using namespace std;

/* StarkInfo class contains the contents of the file zkevm.starkinfo.json,
   which is parsed during the constructor */

typedef enum
{
    const_ = 0,
    cm = 1,
    tmp = 2,
    public_ = 3,
    airgroupvalue = 4,
    challenge = 5,
    number = 6,
    string_ = 7,
    airvalue = 8,
    proofvalue = 9,
    custom = 10,
} opType;


class CustomCommits
{
public:
    std::string name;
    vector<uint32_t> stageWidths;
};

class Boundary
{
public:
    std::string name;
    uint64_t offsetMin;
    uint64_t offsetMax;
};

class StepStruct
{
public:
    uint64_t nBits;
};

class StarkStruct
{
public:
    uint64_t nBits;
    uint64_t nBitsExt;
    uint64_t nQueries;
    bool hashCommits;
    string verificationHashType;
    uint64_t merkleTreeArity;
    bool merkleTreeCustom;
    vector<StepStruct> steps;
};

opType string2opType (const string s);

class PolMap
{
public:
    uint64_t stage;
    std::string name;
    vector<uint64_t> lengths;
    uint64_t dim;
    bool imPol;
    uint64_t stagePos;
    uint64_t stageId;
    uint64_t commitId;
    uint64_t expId;
    uint64_t polsMapId;
};

class EvMap
{
public:
    typedef enum
    {
        cm = 0,
        _const = 1,
        custom = 2,
    } eType;

    eType type;
    uint64_t id;
    int64_t prime;
    uint64_t commitId;
    uint64_t openingPos;

    void setType (string s)
    {
        if (s == "cm") type = cm;
        else if (s == "const") type = _const;
        else if (s == "custom") type = custom;
        else
        {
            zklog.error("EvMap::setType() found invalid type: " + s);
            exitProcess();
        }
    }
};

class StarkInfo
{
public:
    // Read from starkInfo file
    StarkStruct starkStruct;

    uint64_t airgroupId;
    uint64_t airId;

    uint64_t nPublics;
    uint64_t nConstants;
    
    uint64_t nStages;

    vector<CustomCommits> customCommits;

    vector<PolMap> cmPolsMap;
    vector<PolMap> constPolsMap;
    vector<PolMap> challengesMap;
    vector<PolMap> airgroupValuesMap;
    vector<PolMap> airValuesMap;
    vector<PolMap> proofValuesMap;
    vector<PolMap> publicsMap;
    vector<vector<PolMap>> customCommitsMap;

    vector<EvMap> evMap;
    
    vector<int64_t> openingPoints;
    vector<Boundary> boundaries;

    uint64_t qDeg;
    uint64_t qDim;

    uint64_t friExpId;
    uint64_t cExpId;

    std::map<std::string, uint64_t> mapSectionsN;

    // Precomputed
    std::map<std::pair<std::string, bool>, uint64_t> mapOffsets;
    
    uint64_t mapTotalN;

    std::map<std::string, uint64_t> mapTotalNcustomCommits;
    
    /* Constructor */
    StarkInfo(string file);

    /* Loads data from a json object */
    void load (json j);

    void setMapOffsets();

    /* Returns a polynomial specified by its ID */
    void getPolynomial(Polinomial &pol, Goldilocks::Element *pAddress, string type, PolMap& polInfo, bool domainExtended);
};

#endif