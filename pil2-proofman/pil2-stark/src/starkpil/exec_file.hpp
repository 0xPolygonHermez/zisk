#ifndef EXEC_FILE
#define EXEC_FILE

#include <iostream>
#include <fstream>
#include <sstream>
#include <iomanip>
#include <sys/stat.h>
#include <sys/mman.h>
#include <fcntl.h>
#include <unistd.h>
#include "utils.hpp"
#include "commit_pols_starks.hpp"


void getCommitedPols(Goldilocks::Element *circomWitness, std::string execFile, Goldilocks::Element *witness, Goldilocks::Element* publics, uint64_t sizeWitness, uint64_t N, uint64_t nPublics, uint64_t nCommitedPols)  {
    CommitPolsStarks commitPols((uint8_t *)witness, N, nCommitedPols);

    uint64_t nAdds;
    uint64_t nSMap;

    std::ifstream file(execFile, std::ios::binary);
    file.read(reinterpret_cast<char *>(&nAdds), sizeof(uint64_t));
    file.read(reinterpret_cast<char *>(&nSMap), sizeof(uint64_t));
    
    uint64_t *p_data = new uint64_t[2 + nAdds * 4 + nSMap * nCommitedPols];
    
    loadFileParallel(p_data, execFile, (2 + nAdds * 4 + nSMap * nCommitedPols) * sizeof(uint64_t));

    uint64_t *p_adds = &p_data[2];
    uint64_t *p_sMap = &p_data[2 + nAdds * 4];

    for(uint64_t i = 0; i < nPublics; ++i) {
        publics[i] = circomWitness[1 + i];
    }
        
    // #pragma omp parallel for
    for (uint64_t i = 0; i < nAdds; i++) {
        uint64_t idx_1 = p_adds[i * 4];
        uint64_t idx_2 = p_adds[i * 4 + 1];

        Goldilocks::Element c = circomWitness[idx_1] * Goldilocks::fromU64(p_adds[i * 4 + 2]);
        Goldilocks::Element d = circomWitness[idx_2] * Goldilocks::fromU64(p_adds[i * 4 + 3]);
        circomWitness[sizeWitness + i] = c + d;
    }

    // #pragma omp parallel for
    for (uint i = 0; i < N; i++) {
        for (uint j = 0; j < nCommitedPols; j++) {
            if (i < nSMap && p_sMap[nCommitedPols * i + j] != 0) {
                commitPols.Compressor.a[j][i] = circomWitness[p_sMap[nCommitedPols * i + j]];
            } else {
                commitPols.Compressor.a[j][i] = Goldilocks::zero();
            }
        }
    }
    delete[] p_data;
}

#endif
