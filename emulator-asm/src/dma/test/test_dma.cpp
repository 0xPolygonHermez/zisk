#include <stdint.h>
#include <stdio.h>
#include <unistd.h>
#include <string.h>
#include <random>
#include <vector>
#include <cstdint>

#include "test_dma_encode.hpp"
#include "test_dma_tools.hpp"
#include "test_mock.hpp"
#include "test_dma_memcmp_mops.hpp"
#include "test_dma_memcpy_mops.hpp"
#include "test_dma_memset_mops.hpp"
#include "test_dma_inputcpy_mops.hpp"
#include "test_dma_memcmp_mtrace.hpp"
#include "test_dma_memcpy_mtrace.hpp"
#include "test_dma_memset_mtrace.hpp"
#include "test_dma_inputcpy_mtrace.hpp"

int main () {
    test_dma_inputcpy_mops();
    test_dma_inputcpy_mtrace();
    test_dma_memcpy_mops();
    test_dma_memcmp_mops();
    test_dma_memset_mops();
    test_dma_memcpy_mtrace();
    test_dma_memcmp_mtrace();
    test_dma_memset_mtrace();
}


