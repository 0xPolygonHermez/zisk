#include <stdio.h>
#include <sys/time.h>
#include <stdlib.h>
#include <cstdint>
#include <cstring>
#include <clocale>
#include <cstdio>

#include "ec/ec.hpp"
#include "fcall/fcall.hpp"
#include "arith256/arith256.hpp"
#include "arith384/arith384.hpp"
#include "bn254/bn254.hpp"
#include "bls12_381/bls12_381.hpp"
#include "bigint/add256.hpp"
#include "ffiasm/fec.hpp"
#include "ffiasm/fnec.hpp"
#include "common/utils.hpp"

#define N_TESTS 1000000
#define MAX_TEST_SIZE_U64  36  // Enough for the largest test


void print_results(const char *name, uint64_t duration, double tp) {
    // static uint64_t line = 0;
    // const char *s_line = line % 2 ? "" : "\x1B[7m";
    // const char *e_line = line % 2 ? "" : "\x1B[0m";
    // printf("%s%-28s|%'15lu|%'15lu|%'15.4f%s\n", s_line, name, duration, (duration * 1000) / N_TESTS, tp, e_line);
    // line = line + 1;
    printf("%-28s|%'15lu|%'15lu|%'15.4f\n", name, duration, (duration * 1000) / N_TESTS, tp);
}

uint64_t TimeDiff(const struct timeval &startTime, const struct timeval &endTime)
{
    struct timeval diff;

    // Calculate the time difference
    diff.tv_sec = endTime.tv_sec - startTime.tv_sec;
    if (endTime.tv_usec >= startTime.tv_usec)
    {
        diff.tv_usec = endTime.tv_usec - startTime.tv_usec;
    }
    else if (diff.tv_sec > 0)
    {
        diff.tv_usec = 1000000 + endTime.tv_usec - startTime.tv_usec;
        diff.tv_sec--;
    }
    else
    {
        // gettimeofday() can go backwards under some circumstances: NTP, multithread...
        return 0;
    }

    // Return the total number of us
    return diff.tv_usec + 1000000 * diff.tv_sec;
}

uint64_t TimeDiff(const struct timeval &startTime)
{
    struct timeval endTime;
    gettimeofday(&endTime, NULL);
    return TimeDiff(startTime, endTime);
}

bool verbose = false;

void secp256k1_add_benchmark(uint64_t *data)
{
    try {
        const uint64_t TEST_SIZE_INPUT_U64 = 4 * 4;
        const uint64_t TEST_SIZE_OUTPUT_U64 = 2 * 4;
        const uint64_t TEST_SIZE_U64 = TEST_SIZE_INPUT_U64 + TEST_SIZE_OUTPUT_U64;
        for (uint64_t i = 0; i<N_TESTS; i++)
        {
            for (uint64_t j = 0; j < TEST_SIZE_INPUT_U64; j++) {
                data[i * TEST_SIZE_U64 + j] = (uint64_t)rand();
            }
            for (uint64_t j = 0; j < TEST_SIZE_OUTPUT_U64; j++) {
                data[i * TEST_SIZE_U64 + j + TEST_SIZE_INPUT_U64] = 0;
            }
        }

        struct timeval startTime;
        gettimeofday(&startTime, NULL);
        for (uint64_t i = 0; i<N_TESTS; i++)
        {
            uint64_t *test_data = data + i * TEST_SIZE_U64;
            AddPointEc(false, test_data, test_data + 4, test_data + 8, test_data + 12, test_data + 16, test_data + 20);
        }
        uint64_t duration = TimeDiff(startTime);
        double tp = duration == 0 ? 0 : double(N_TESTS)/duration;
        print_results("secp256k1 (add)", duration, tp);
    }
    catch (const std::exception & e) {
        printf("secp256k1 (add)             |Exception: %s\n", e.what());
    }
}

void secp256k1_dbl_benchmark(uint64_t *data)
{
    try {
        const uint64_t TEST_SIZE_POINT_U64 = 2 * 4;
        const uint64_t TEST_SIZE_INPUT_U64 = TEST_SIZE_POINT_U64 * 2;
        const uint64_t TEST_SIZE_OUTPUT_U64 = 2 * 4;
        const uint64_t TEST_SIZE_U64 = TEST_SIZE_INPUT_U64 + TEST_SIZE_OUTPUT_U64;
        for (uint64_t i = 0; i<N_TESTS; i++)
        {
            for (uint64_t j = 0; j < TEST_SIZE_POINT_U64; j++) {
                uint64_t value = (uint64_t)rand();
                data[i * TEST_SIZE_U64 + j] = value;
                data[i * TEST_SIZE_U64 + j + TEST_SIZE_POINT_U64] = value;
            }
            for (uint64_t j = 0; j < TEST_SIZE_OUTPUT_U64; j++) {
                data[i * TEST_SIZE_U64 + j + TEST_SIZE_INPUT_U64] = 0;
            }
        }

        struct timeval startTime;
        gettimeofday(&startTime, NULL);
        for (uint64_t i = 0; i<N_TESTS; i++)
        {
            uint64_t *test_data = data + i * TEST_SIZE_U64;
            AddPointEc(true, test_data, test_data + 4, test_data + 8, test_data + 12, test_data + 16, test_data + 20);
        }
        uint64_t duration = TimeDiff(startTime);
        double tp = duration == 0 ? 0 : double(N_TESTS)/duration;
        print_results("secp256k1 (dbl)", duration, tp);
    }
    catch (const std::exception & e) {
        printf("secp256k1 (dbl)             |Exception: %s\n", e.what());
    }
}
/*
void secp256k1_add_fe_benchmark(uint64_t *data)
{
    try {
        const uint64_t count = N_TESTS;
        RawFec::Element* x1 = new RawFec::Element[count];
        RawFec::Element* y1 = new RawFec::Element[count];
        RawFec::Element* x2 = new RawFec::Element[count];
        RawFec::Element* y2 = new RawFec::Element[count];
        RawFec::Element x3, y3;
        for (uint64_t i = 0; i<count; i++)
        {
            uint64_t _x1[4] = {(uint64_t)rand(),(uint64_t)rand(),(uint64_t)rand(),(uint64_t)rand()};
            uint64_t _y1[4] = {(uint64_t)rand(),(uint64_t)rand(),(uint64_t)rand(),(uint64_t)rand()};
            uint64_t _x2[4] = {(uint64_t)rand(),(uint64_t)rand(),(uint64_t)rand(),(uint64_t)rand()};
            uint64_t _y2[4] = {(uint64_t)rand(),(uint64_t)rand(),(uint64_t)rand(),(uint64_t)rand()};
            array2fe(_x1, x1[i]);
            array2fe(_y1, y1[i]);
            array2fe(_x2, x2[i]);
            array2fe(_y2, y2[i]);
        }
        struct timeval startTime;
        gettimeofday(&startTime, NULL);
        for (uint64_t i = 0; i<count; i++)
        {
            int result = AddPointEcFe(false, x1[i], y1[i], x2[i], y2[i], x3, y3);
        }
        uint64_t duration = TimeDiff(startTime);
        delete[] x1;
        delete[] y1;
        delete[] x2;
        delete[] y2;
        double tp = duration == 0 ? 0 : double(N_TESTS)/duration;
        print_results("secp256k1 (add) (direct fe)", duration, tp);
    }
    catch (const std::exception & e) {
        printf("secp256k1 (add) (direct fe) |Exception: %s\n", e.what());
    }
}

void secp256k1_dbl_fe_benchmark(uint64_t *data)
{
    try {
        const uint64_t count = N_TESTS;
        RawFec::Element* x1 = new RawFec::Element[count];
        RawFec::Element* y1 = new RawFec::Element[count];
        RawFec::Element x2, y2;
        RawFec::Element x3, y3;
        for (uint64_t i = 0; i<count; i++)
        {
            uint64_t _x1[4] = {(uint64_t)rand(),(uint64_t)rand(),(uint64_t)rand(),(uint64_t)rand()};
            uint64_t _y1[4] = {(uint64_t)rand(),(uint64_t)rand(),(uint64_t)rand(),(uint64_t)rand()};
            array2fe(_x1, x1[i]);
            array2fe(_y1, y1[i]);
        }
        struct timeval startTime;
        gettimeofday(&startTime, NULL);
        for (uint64_t i = 0; i<count; i++)
        {
            int result = AddPointEcFe(true, x1[i], y1[i], x1[i], y1[i], x3, y3);
        }
        uint64_t duration = TimeDiff(startTime);
        delete[] x1;
        delete[] y1;
        double tp = duration == 0 ? 0 : double(N_TESTS)/duration;
        print_results("secp256k1 (dbl) (direct fe)", duration, tp);
    }
    catch (const std::exception & e) {
        printf("secp256k1 (dbl) (direct fe) |Exception: %s\n", e.what());
    }
}
*/
void InverseFpEc_benchmark(uint64_t *data)
{
    try {
        const uint64_t TEST_SIZE_INPUT_U64 = 1 * 4;
        const uint64_t TEST_SIZE_OUTPUT_U64 = 1 * 4;
        const uint64_t TEST_SIZE_U64 = TEST_SIZE_INPUT_U64 + TEST_SIZE_OUTPUT_U64;
        for (uint64_t i = 0; i<N_TESTS; i++)
        {
            for (uint64_t j = 0; j < TEST_SIZE_INPUT_U64; j++) {
                uint64_t value = (uint64_t)rand();
                data[i * TEST_SIZE_U64 + j] = value;
            }
            for (uint64_t j = 0; j < TEST_SIZE_OUTPUT_U64; j++) {
                data[i * TEST_SIZE_U64 + j + TEST_SIZE_INPUT_U64] = 0;
            }
        }

        struct timeval startTime;
        gettimeofday(&startTime, NULL);
        for (uint64_t i = 0; i<1000000; i++)
        {
            uint64_t *test_data = data + i * TEST_SIZE_U64;
            int result = InverseFpEc(test_data, test_data + TEST_SIZE_INPUT_U64);
        }
        uint64_t duration = TimeDiff(startTime);
        double tp = duration == 0 ? 0 : double(N_TESTS)/duration;
        print_results("InverseFpEc", duration, tp);
    }
    catch (const std::exception & e) {
        printf("InverseFpEc                 |Exception: %s\n", e.what());
    } 
}

void InverseFnEc_benchmark(uint64_t *data)
{
    try {
        const uint64_t TEST_SIZE_INPUT_U64 = 1 * 4;
        const uint64_t TEST_SIZE_OUTPUT_U64 = 1 * 4;
        const uint64_t TEST_SIZE_U64 = TEST_SIZE_INPUT_U64 + TEST_SIZE_OUTPUT_U64;
        for (uint64_t i = 0; i<N_TESTS; i++)
        {
            for (uint64_t j = 0; j < TEST_SIZE_INPUT_U64; j++) {
                uint64_t value = (uint64_t)rand();
                data[i * TEST_SIZE_U64 + j] = value;
            }
            for (uint64_t j = 0; j < TEST_SIZE_OUTPUT_U64; j++) {
                data[i * TEST_SIZE_U64 + j + TEST_SIZE_INPUT_U64] = 0;
            }
        }

        struct timeval startTime;
        gettimeofday(&startTime, NULL);
        for (uint64_t i = 0; i<1000000; i++)
        {
            uint64_t *test_data = data + i * TEST_SIZE_U64;
            int result = InverseFnEc(test_data, test_data + TEST_SIZE_INPUT_U64);
        }
        uint64_t duration = TimeDiff(startTime);
        double tp = duration == 0 ? 0 : double(N_TESTS)/duration;
        print_results("InverseFnEc", duration, tp);
    }
    catch (const std::exception & e) {
        printf("InverseFnEc                 |Exception: %s\n", e.what());
    }       
}

void SqrtFpEcParity_benchmark(uint64_t *data)
{
    try {
        const uint64_t TEST_SIZE_INPUT_U64 = 1 * 4;
        const uint64_t TEST_SIZE_OUTPUT_U64 = 1 * 4;
        const uint64_t TEST_SIZE_U64 = TEST_SIZE_INPUT_U64 + TEST_SIZE_OUTPUT_U64;
        for (uint64_t i = 0; i<N_TESTS; i++)
        {
            for (uint64_t j = 0; j < TEST_SIZE_INPUT_U64; j++) {
                uint64_t value = (uint64_t)rand();
                data[i * TEST_SIZE_U64 + j] = value;
            }
            for (uint64_t j = 0; j < TEST_SIZE_OUTPUT_U64; j++) {
                data[i * TEST_SIZE_U64 + j + TEST_SIZE_INPUT_U64] = 0;
            }
        }

        struct timeval startTime;
        gettimeofday(&startTime, NULL);
        for (uint64_t i = 0; i<1000000; i++)
        {
            uint64_t *test_data = data + i * TEST_SIZE_U64;
            int result = SqrtFpEcParity(test_data, i%2, test_data + TEST_SIZE_INPUT_U64);
        }
        uint64_t duration = TimeDiff(startTime);
        double tp = duration == 0 ? 0 : double(N_TESTS)/duration;
        print_results("SqrtFpEcParity", duration, tp);
    }
    catch (const std::exception & e) {
        printf("SqrtFpEcParity              |Exception: %s\n", e.what());
    }       
}

void BN254CurveAddP_benchmark(uint64_t *data) 
{
    try {
        const uint64_t TEST_SIZE_INPUT_U64 = 4 * 4;
        const uint64_t TEST_SIZE_OUTPUT_U64 = 2 * 4;
        const uint64_t TEST_SIZE_U64 = TEST_SIZE_INPUT_U64 + TEST_SIZE_OUTPUT_U64;
        for (uint64_t i = 0; i<N_TESTS; i++)
        {
            for (uint64_t j = 0; j < TEST_SIZE_INPUT_U64; j++) {
                uint64_t value = (uint64_t)rand();
                data[i * TEST_SIZE_U64 + j] = value;
            }
            for (uint64_t j = 0; j < TEST_SIZE_OUTPUT_U64; j++) {
                data[i * TEST_SIZE_U64 + j + TEST_SIZE_INPUT_U64] = 0;
            }
        }

        struct timeval startTime;
        gettimeofday(&startTime, NULL);
        for (uint64_t i = 0; i<1000000; i++)
        {
            uint64_t *test_data = data + i * TEST_SIZE_U64;
            int result = BN254CurveAddP(test_data, test_data + 8, test_data + 16);
        }
        uint64_t duration = TimeDiff(startTime);
        double tp = duration == 0 ? 0 : double(N_TESTS)/duration;
        print_results("BN254CurveAddP", duration, tp);
    }
    catch (const std::exception & e) {
        printf("BN254CurveAddP              |Exception: %s\n", e.what());
    }    
}

void BN254CurveDblP_benchmark(uint64_t *data) 
{
    try {
        const uint64_t TEST_SIZE_INPUT_U64 = 2 * 4;
        const uint64_t TEST_SIZE_OUTPUT_U64 = 2 * 4;
        const uint64_t TEST_SIZE_U64 = TEST_SIZE_INPUT_U64 + TEST_SIZE_OUTPUT_U64;
        for (uint64_t i = 0; i<N_TESTS; i++)
        {
            for (uint64_t j = 0; j < TEST_SIZE_INPUT_U64; j++) {
                uint64_t value = (uint64_t)rand();
                data[i * TEST_SIZE_U64 + j] = value;
            }
            for (uint64_t j = 0; j < TEST_SIZE_OUTPUT_U64; j++) {
                data[i * TEST_SIZE_U64 + j + TEST_SIZE_INPUT_U64] = 0;
            }
        }

        struct timeval startTime;
        gettimeofday(&startTime, NULL);
        for (uint64_t i = 0; i<N_TESTS; i++)
        {
            uint64_t *test_data = data + i * TEST_SIZE_U64;
            BN254CurveDblP(test_data, test_data + 8);
        }
        uint64_t duration = TimeDiff(startTime);
        double tp = duration == 0 ? 0 : double(N_TESTS)/duration;
        print_results("BN254CurveDblP", duration, tp);
    }
    catch (const std::exception & e) {
        printf("BN254CurveDblP              |Exception: %s\n", e.what());
    }    
}

void BN254FpInv_benchmark(uint64_t *data)
{
    try {
        const uint64_t TEST_SIZE_INPUT_U64 = 1 * 4;
        const uint64_t TEST_SIZE_OUTPUT_U64 = 1 * 4;
        const uint64_t TEST_SIZE_U64 = TEST_SIZE_INPUT_U64 + TEST_SIZE_OUTPUT_U64;
        for (uint64_t i = 0; i<N_TESTS; i++)
        {
            for (uint64_t j = 0; j < TEST_SIZE_INPUT_U64; j++) {
                uint64_t value = (uint64_t)rand();
                data[i * TEST_SIZE_U64 + j] = value;
            }
            for (uint64_t j = 0; j < TEST_SIZE_OUTPUT_U64; j++) {
                data[i * TEST_SIZE_U64 + j + TEST_SIZE_INPUT_U64] = 0;
            }
        }
        struct timeval startTime;
        gettimeofday(&startTime, NULL);
        for (uint64_t i = 0; i<N_TESTS; i++)
        {
            uint64_t *test_data = data + i * TEST_SIZE_U64;
            int result = BN254FpInv(test_data, test_data + 4);
        }
        uint64_t duration = TimeDiff(startTime);
        double tp = duration == 0 ? 0 : double(N_TESTS)/duration;
        print_results("BN254FpInv", duration, tp);
    }
    catch (const std::exception & e) {
        printf("BN254FpInv                  |Exception: %s\n", e.what());
    }            
}

void BN254ComplexAddP_benchmark(uint64_t *data)
{
    try {
        const uint64_t TEST_SIZE_INPUT_U64 = 2 * 8;
        const uint64_t TEST_SIZE_OUTPUT_U64 = 1 * 8;
        const uint64_t TEST_SIZE_U64 = TEST_SIZE_INPUT_U64 + TEST_SIZE_OUTPUT_U64;
        for (uint64_t i = 0; i<N_TESTS; i++)
        {
            for (uint64_t j = 0; j < TEST_SIZE_INPUT_U64; j++) {
                uint64_t value = (uint64_t)rand();
                data[i * TEST_SIZE_U64 + j] = value;
            }
            for (uint64_t j = 0; j < TEST_SIZE_OUTPUT_U64; j++) {
                data[i * TEST_SIZE_U64 + j + TEST_SIZE_INPUT_U64] = 0;
            }
        }
        struct timeval startTime;
        gettimeofday(&startTime, NULL);
        for (uint64_t i = 0; i<N_TESTS; i++)
        {
            uint64_t *test_data = data + i * TEST_SIZE_U64;
            int result = BN254ComplexAddP(test_data, test_data + 8, test_data + 16);
        }
        uint64_t duration = TimeDiff(startTime);
        double tp = duration == 0 ? 0 : double(N_TESTS)/duration;
        print_results("BN254ComplexAddP", duration, tp);
    }
    catch (const std::exception & e) {
        printf("BN254ComplexAddP            |Exception: %s\n", e.what());
    }            
}

void BN254ComplexSubP_benchmark(uint64_t *data)
{
    try {
        const uint64_t TEST_SIZE_INPUT_U64 = 2 * 8;
        const uint64_t TEST_SIZE_OUTPUT_U64 = 1 * 8;
        const uint64_t TEST_SIZE_U64 = TEST_SIZE_INPUT_U64 + TEST_SIZE_OUTPUT_U64;
        for (uint64_t i = 0; i<N_TESTS; i++)
        {
            for (uint64_t j = 0; j < TEST_SIZE_INPUT_U64; j++) {
                uint64_t value = (uint64_t)rand();
                data[i * TEST_SIZE_U64 + j] = value;
            }
            for (uint64_t j = 0; j < TEST_SIZE_OUTPUT_U64; j++) {
                data[i * TEST_SIZE_U64 + j + TEST_SIZE_INPUT_U64] = 0;
            }
        }
        struct timeval startTime;
        gettimeofday(&startTime, NULL);
        for (uint64_t i = 0; i<N_TESTS; i++)
        {
            uint64_t *test_data = data + i * TEST_SIZE_U64;
            int result = BN254ComplexSubP(test_data, test_data + 8, test_data + 16);
        }
        uint64_t duration = TimeDiff(startTime);
        double tp = duration == 0 ? 0 : double(N_TESTS)/duration;
        print_results("BN254ComplexSubP", duration, tp);
    }
    catch (const std::exception & e) {
        printf("BN254ComplexSubP            |Exception: %s\n", e.what());
    }            
}

void BN254ComplexMulP_benchmark(uint64_t *data)
{
    try {
        const uint64_t TEST_SIZE_INPUT_U64 = 2 * 8;
        const uint64_t TEST_SIZE_OUTPUT_U64 = 1 * 8;
        const uint64_t TEST_SIZE_U64 = TEST_SIZE_INPUT_U64 + TEST_SIZE_OUTPUT_U64;
        for (uint64_t i = 0; i<N_TESTS; i++)
        {
            for (uint64_t j = 0; j < TEST_SIZE_INPUT_U64; j++) {
                uint64_t value = (uint64_t)rand();
                data[i * TEST_SIZE_U64 + j] = value;
            }
            for (uint64_t j = 0; j < TEST_SIZE_OUTPUT_U64; j++) {
                data[i * TEST_SIZE_U64 + j + TEST_SIZE_INPUT_U64] = 0;
            }
        }
        struct timeval startTime;
        gettimeofday(&startTime, NULL);
        for (uint64_t i = 0; i<N_TESTS; i++)
        {
            uint64_t *test_data = data + i * TEST_SIZE_U64;
            int result = BN254ComplexMulP(test_data, test_data + 8, test_data + 16);
        }
        uint64_t duration = TimeDiff(startTime);
        double tp = duration == 0 ? 0 : double(N_TESTS)/duration;
        print_results("BN254ComplexMulP", duration, tp);
    }
    catch (const std::exception & e) {
        printf("BN254ComplexMulP            |Exception: %s\n", e.what());
    }            
}

void BN254ComplexInv_benchmark(uint64_t *data)
{
    try {
        const uint64_t TEST_SIZE_INPUT_U64 = 1 * 4;
        const uint64_t TEST_SIZE_OUTPUT_U64 = 1 * 4;
        const uint64_t TEST_SIZE_U64 = TEST_SIZE_INPUT_U64 + TEST_SIZE_OUTPUT_U64;
        for (uint64_t i = 0; i<N_TESTS; i++)
        {
            for (uint64_t j = 0; j < TEST_SIZE_INPUT_U64; j++) {
                uint64_t value = (uint64_t)rand();
                data[i * TEST_SIZE_U64 + j] = value;
            }
            for (uint64_t j = 0; j < TEST_SIZE_OUTPUT_U64; j++) {
                data[i * TEST_SIZE_U64 + j + TEST_SIZE_INPUT_U64] = 0;
            }
        }
        struct timeval startTime;
        gettimeofday(&startTime, NULL);
        for (uint64_t i = 0; i<N_TESTS; i++)
        {
            uint64_t *test_data = data + i * TEST_SIZE_U64;
            int result = BN254ComplexInv(test_data, test_data + 4);
        }
        uint64_t duration = TimeDiff(startTime);
        double tp = duration == 0 ? 0 : double(N_TESTS)/duration;
        print_results("BN254ComplexInv", duration, tp);
    }
    catch (const std::exception & e) {
        printf("BN254ComplexInv             |Exception: %s\n", e.what());
    }            
} 

void BN254TwistAddLineCoeffs_benchmark(uint64_t *data)
{
        try {
        const uint64_t TEST_SIZE_INPUT_U64 = 1 * 4;
        const uint64_t TEST_SIZE_OUTPUT_U64 = 1 * 4;
        const uint64_t TEST_SIZE_U64 = TEST_SIZE_INPUT_U64 + TEST_SIZE_OUTPUT_U64;
        for (uint64_t i = 0; i<N_TESTS; i++)
        {
            for (uint64_t j = 0; j < TEST_SIZE_INPUT_U64; j++) {
                uint64_t value = (uint64_t)rand();
                data[i * TEST_SIZE_U64 + j] = value;
            }
            for (uint64_t j = 0; j < TEST_SIZE_OUTPUT_U64; j++) {
                data[i * TEST_SIZE_U64 + j + TEST_SIZE_INPUT_U64] = 0;
            }
        }
        struct timeval startTime;
        gettimeofday(&startTime, NULL);
        for (uint64_t i = 0; i<N_TESTS; i++)
        {
            uint64_t *test_data = data + i * TEST_SIZE_U64;
            int result = BN254TwistAddLineCoeffs(test_data, test_data + 4);
        }
        uint64_t duration = TimeDiff(startTime);
        double tp = duration == 0 ? 0 : double(N_TESTS)/duration;
        print_results("BN254TwistAddLineCoeffs", duration, tp);
    }
    catch (const std::exception & e) {
        printf("BN254TwistAddLineCoeffs     |Exception: %s\n", e.what());
    }
}

void BN254TwistDblLineCoeffs_benchmark(uint64_t *data)
{
        try {
        const uint64_t TEST_SIZE_INPUT_U64 = 1 * 4;
        const uint64_t TEST_SIZE_OUTPUT_U64 = 1 * 4;
        const uint64_t TEST_SIZE_U64 = TEST_SIZE_INPUT_U64 + TEST_SIZE_OUTPUT_U64;
        for (uint64_t i = 0; i<N_TESTS; i++)
        {
            for (uint64_t j = 0; j < TEST_SIZE_INPUT_U64; j++) {
                uint64_t value = (uint64_t)rand();
                data[i * TEST_SIZE_U64 + j] = value;
            }
            for (uint64_t j = 0; j < TEST_SIZE_OUTPUT_U64; j++) {
                data[i * TEST_SIZE_U64 + j + TEST_SIZE_INPUT_U64] = 0;
            }
        }
        struct timeval startTime;
        gettimeofday(&startTime, NULL);
        for (uint64_t i = 0; i<N_TESTS; i++)
        {
            uint64_t *test_data = data + i * TEST_SIZE_U64;
            int result = BN254TwistDblLineCoeffs(test_data, test_data + 4);
        }
        uint64_t duration = TimeDiff(startTime);
        double tp = duration == 0 ? 0 : double(N_TESTS)/duration;
        print_results("BN254TwistDblLineCoeffs", duration, tp);
    }
    catch (const std::exception & e) {
        printf("BN254TwistDblLineCoeffs     |Exception: %s\n", e.what());
    }
}

void Arith256_benchmark(uint64_t *data) {
    try {
        const uint64_t TEST_SIZE_INPUT_U64 = 3 * 4;
        const uint64_t TEST_SIZE_OUTPUT_U64 = 2 * 4;
        const uint64_t TEST_SIZE_U64 = TEST_SIZE_INPUT_U64 + TEST_SIZE_OUTPUT_U64;
        for (uint64_t i = 0; i<N_TESTS; i++)
        {
            for (uint64_t j = 0; j < TEST_SIZE_INPUT_U64; j++) {
                data[i * TEST_SIZE_U64 + j] = (uint64_t)rand();
            }
            for (uint64_t j = 0; j < TEST_SIZE_OUTPUT_U64; j++) {
                data[i * TEST_SIZE_U64 + j + TEST_SIZE_INPUT_U64] = 0;
            }
        }

        struct timeval startTime;
        gettimeofday(&startTime, NULL);
        for (uint64_t i = 0; i<N_TESTS; i++)
        {
            uint64_t *test_data = data + i * TEST_SIZE_U64;
            Arith256(test_data, test_data + 4, test_data + 8, test_data + 12, test_data + 16);
        }

        uint64_t duration = TimeDiff(startTime);
        double tp = duration == 0 ? 0 : double(N_TESTS)/duration;
        print_results("Arith256", duration, tp);
    }
    catch (const std::exception & e) {
        printf("Arith256                    |Exception: %s\n", e.what());
    }
    
}

void FastArith256_benchmark(uint64_t *data) {
    try {
        const uint64_t TEST_SIZE_INPUT_U64 = 3 * 4;
        const uint64_t TEST_SIZE_OUTPUT_U64 = 2 * 4;
        const uint64_t TEST_SIZE_U64 = TEST_SIZE_INPUT_U64 + TEST_SIZE_OUTPUT_U64;
        for (uint64_t i = 0; i<N_TESTS; i++)
        {
            for (uint64_t j = 0; j < TEST_SIZE_INPUT_U64; j++) {
                data[i * TEST_SIZE_U64 + j] = (uint64_t)rand();
            }
            for (uint64_t j = 0; j < TEST_SIZE_OUTPUT_U64; j++) {
                data[i * TEST_SIZE_U64 + j + TEST_SIZE_INPUT_U64] = 0;
            }
        }

        struct timeval startTime;
        gettimeofday(&startTime, NULL);
        for (uint64_t i = 0; i<N_TESTS; i++)
        {
            uint64_t *test_data = data + i * TEST_SIZE_U64;
            FastArith256(test_data, test_data + 4, test_data + 8, test_data + 12, test_data + 16);
        }

        uint64_t duration = TimeDiff(startTime);
        double tp = duration == 0 ? 0 : double(N_TESTS)/duration;
        print_results("Arith256 (fast)", duration, tp);
    }
    catch (const std::exception & e) {
        printf("Arith256 (fast)             |Exception: %s\n", e.what());
    }
}

void Arith256Mod_benchmark(uint64_t *data) {
    try {
        const uint64_t TEST_SIZE_INPUT_U64 = 4 * 4;
        const uint64_t TEST_SIZE_OUTPUT_U64 = 1 * 4;
        const uint64_t TEST_SIZE_U64 = TEST_SIZE_INPUT_U64 + TEST_SIZE_OUTPUT_U64;
        for (uint64_t i = 0; i<N_TESTS; i++)
        {
            for (uint64_t j = 0; j < TEST_SIZE_INPUT_U64; j++) {
                data[i * TEST_SIZE_U64 + j] = (uint64_t)rand();
            }
            for (uint64_t j = 0; j < TEST_SIZE_OUTPUT_U64; j++) {
                data[i * TEST_SIZE_U64 + j + TEST_SIZE_INPUT_U64] = 0;
            }
        }

        struct timeval startTime;
        gettimeofday(&startTime, NULL);
        for (uint64_t i = 0; i<N_TESTS; i++)
        {
            uint64_t *test_data = data + i * TEST_SIZE_U64;
            Arith256Mod(test_data, test_data + 4, test_data + 8, test_data + 12, test_data + 16);
        }
        uint64_t duration = TimeDiff(startTime);
        double tp = duration == 0 ? 0 : double(N_TESTS)/duration;
        print_results("Arith256Mod", duration, tp);
    }
    catch (const std::exception & e) {
        printf("Arith256Mod                 |Exception: %s\n", e.what());
    }
}

void Arith384_benchmark(uint64_t *data) {
    try {
        const uint64_t TEST_SIZE_INPUT_U64 = 3 * 6;
        const uint64_t TEST_SIZE_OUTPUT_U64 = 2 * 6;
        const uint64_t TEST_SIZE_U64 = TEST_SIZE_INPUT_U64 + TEST_SIZE_OUTPUT_U64;
        for (uint64_t i = 0; i<N_TESTS; i++)
        {
            for (uint64_t j = 0; j < TEST_SIZE_INPUT_U64; j++) {
                data[i * TEST_SIZE_U64 + j] = (uint64_t)rand();
            }
            for (uint64_t j = 0; j < TEST_SIZE_OUTPUT_U64; j++) {
                data[i * TEST_SIZE_U64 + j + TEST_SIZE_INPUT_U64] = 0;
            }
        }

        struct timeval startTime;
        gettimeofday(&startTime, NULL);
        for (uint64_t i = 0; i<N_TESTS; i++)
        {
            uint64_t *test_data = data + i * TEST_SIZE_U64;
            Arith384(test_data, test_data + 6, test_data + 12, test_data + 18, test_data + 24);
        }

        uint64_t duration = TimeDiff(startTime);
        double tp = duration == 0 ? 0 : double(N_TESTS)/duration;
        print_results("Arith384", duration, tp);
    }
    catch (const std::exception & e) {
        printf("Arith384                    |Exception: %s\n", e.what());
    }
    
}

void Arith384Mod_benchmark(uint64_t *data) {
    try {
        const uint64_t TEST_SIZE_INPUT_U64 = 4 * 6;
        const uint64_t TEST_SIZE_OUTPUT_U64 = 1 * 6;
        const uint64_t TEST_SIZE_U64 = TEST_SIZE_INPUT_U64 + TEST_SIZE_OUTPUT_U64;
        for (uint64_t i = 0; i<N_TESTS; i++)
        {
            for (uint64_t j = 0; j < TEST_SIZE_INPUT_U64; j++) {
                data[i * TEST_SIZE_U64 + j] = (uint64_t)rand();
            }
            for (uint64_t j = 0; j < TEST_SIZE_OUTPUT_U64; j++) {
                data[i * TEST_SIZE_U64 + j + TEST_SIZE_INPUT_U64] = 0;
            }
        }

        struct timeval startTime;
        gettimeofday(&startTime, NULL);
        for (uint64_t i = 0; i<N_TESTS; i++)
        {
            uint64_t *test_data = data + i * TEST_SIZE_U64;
            Arith384Mod(test_data, test_data + 6, test_data + 12, test_data + 18, test_data + 24);
        }
        uint64_t duration = TimeDiff(startTime);
        double tp = duration == 0 ? 0 : double(N_TESTS)/duration;
        print_results("Arith384Mod", duration, tp);
    }
    catch (const std::exception & e) {
        printf("Arith384Mod                 |Exception: %s\n", e.what());
    }
}

void BLS12_381CurveAddP_benchmark(uint64_t *data) 
{
    try {
        const uint64_t TEST_SIZE_INPUT_U64 = 4 * 6;
        const uint64_t TEST_SIZE_OUTPUT_U64 = 2 * 6;
        const uint64_t TEST_SIZE_U64 = TEST_SIZE_INPUT_U64 + TEST_SIZE_OUTPUT_U64;
        for (uint64_t i = 0; i<N_TESTS; i++)
        {
            for (uint64_t j = 0; j < TEST_SIZE_INPUT_U64; j++) {
                uint64_t value = (uint64_t)rand();
                data[i * TEST_SIZE_U64 + j] = value;
            }
            for (uint64_t j = 0; j < TEST_SIZE_OUTPUT_U64; j++) {
                data[i * TEST_SIZE_U64 + j + TEST_SIZE_INPUT_U64] = 0;
            }
        }

        struct timeval startTime;
        gettimeofday(&startTime, NULL);
        for (uint64_t i = 0; i<1000000; i++)
        {
            uint64_t *test_data = data + i * TEST_SIZE_U64;
            int result = BLS12_381CurveAddP(test_data, test_data + 12, test_data + 24);
        }
        uint64_t duration = TimeDiff(startTime);
        double tp = duration == 0 ? 0 : double(N_TESTS)/duration;
        print_results("BLS12_381CurveAddP", duration, tp);
    }
    catch (const std::exception & e) {
        printf("BLS12_381CurveAddP          |Exception: %s\n", e.what());
    }    
}

void BLS12_381CurveDblP_benchmark(uint64_t *data) 
{
    try {
        const uint64_t TEST_SIZE_INPUT_U64 = 2 * 6;
        const uint64_t TEST_SIZE_OUTPUT_U64 = 2 * 6;
        const uint64_t TEST_SIZE_U64 = TEST_SIZE_INPUT_U64 + TEST_SIZE_OUTPUT_U64;
        for (uint64_t i = 0; i<N_TESTS; i++)
        {
            for (uint64_t j = 0; j < TEST_SIZE_INPUT_U64; j++) {
                uint64_t value = (uint64_t)rand();
                data[i * TEST_SIZE_U64 + j] = value;
            }
            for (uint64_t j = 0; j < TEST_SIZE_OUTPUT_U64; j++) {
                data[i * TEST_SIZE_U64 + j + TEST_SIZE_INPUT_U64] = 0;
            }
        }

        struct timeval startTime;
        gettimeofday(&startTime, NULL);
        for (uint64_t i = 0; i<N_TESTS; i++)
        {
            uint64_t *test_data = data + i * TEST_SIZE_U64;
            BLS12_381CurveDblP(test_data, test_data + 12);
        }
        uint64_t duration = TimeDiff(startTime);
        double tp = duration == 0 ? 0 : double(N_TESTS)/duration;
        print_results("BLS12_381CurveDblP", duration, tp);
    }
    catch (const std::exception & e) {
        printf("BLS12_381CurveDblP          |Exception: %s\n", e.what());
    }    
}

void BLS12_381ComplexAddP_benchmark(uint64_t *data)
{
    try {
        const uint64_t TEST_SIZE_INPUT_U64 = 2 * 12;
        const uint64_t TEST_SIZE_OUTPUT_U64 = 1 * 12;
        const uint64_t TEST_SIZE_U64 = TEST_SIZE_INPUT_U64 + TEST_SIZE_OUTPUT_U64;
        for (uint64_t i = 0; i<N_TESTS; i++)
        {
            for (uint64_t j = 0; j < TEST_SIZE_INPUT_U64; j++) {
                uint64_t value = (uint64_t)rand();
                data[i * TEST_SIZE_U64 + j] = value;
            }
            for (uint64_t j = 0; j < TEST_SIZE_OUTPUT_U64; j++) {
                data[i * TEST_SIZE_U64 + j + TEST_SIZE_INPUT_U64] = 0;
            }
        }
        struct timeval startTime;
        gettimeofday(&startTime, NULL);
        for (uint64_t i = 0; i<N_TESTS; i++)
        {
            uint64_t *test_data = data + i * TEST_SIZE_U64;
            int result = BLS12_381ComplexAddP(test_data, test_data + 12, test_data + 24);
        }
        uint64_t duration = TimeDiff(startTime);
        double tp = duration == 0 ? 0 : double(N_TESTS)/duration;
        print_results("BLS12_381ComplexAddP", duration, tp);
    }
    catch (const std::exception & e) {
        printf("BLS12_381ComplexAddP        |Exception: %s\n", e.what());
    }            
}

void BLS12_381ComplexSubP_benchmark(uint64_t *data)
{
    try {
        const uint64_t TEST_SIZE_INPUT_U64 = 2 * 12;
        const uint64_t TEST_SIZE_OUTPUT_U64 = 1 * 12;
        const uint64_t TEST_SIZE_U64 = TEST_SIZE_INPUT_U64 + TEST_SIZE_OUTPUT_U64;
        for (uint64_t i = 0; i<N_TESTS; i++)
        {
            for (uint64_t j = 0; j < TEST_SIZE_INPUT_U64; j++) {
                uint64_t value = (uint64_t)rand();
                data[i * TEST_SIZE_U64 + j] = value;
            }
            for (uint64_t j = 0; j < TEST_SIZE_OUTPUT_U64; j++) {
                data[i * TEST_SIZE_U64 + j + TEST_SIZE_INPUT_U64] = 0;
            }
        }
        struct timeval startTime;
        gettimeofday(&startTime, NULL);
        for (uint64_t i = 0; i<N_TESTS; i++)
        {
            uint64_t *test_data = data + i * TEST_SIZE_U64;
            int result = BLS12_381ComplexSubP(test_data, test_data + 12, test_data + 24);
        }
        uint64_t duration = TimeDiff(startTime);
        double tp = duration == 0 ? 0 : double(N_TESTS)/duration;
        print_results("BLS12_381ComplexSubP", duration, tp);
    }
    catch (const std::exception & e) {
        printf("BLS12_381ComplexSubP        |Exception: %s\n", e.what());
    }            
}

void BLS12_381ComplexMulP_benchmark(uint64_t *data)
{
    try {
        const uint64_t TEST_SIZE_INPUT_U64 = 2 * 12;
        const uint64_t TEST_SIZE_OUTPUT_U64 = 1 * 12;
        const uint64_t TEST_SIZE_U64 = TEST_SIZE_INPUT_U64 + TEST_SIZE_OUTPUT_U64;
        for (uint64_t i = 0; i<N_TESTS; i++)
        {
            for (uint64_t j = 0; j < TEST_SIZE_INPUT_U64; j++) {
                uint64_t value = (uint64_t)rand();
                data[i * TEST_SIZE_U64 + j] = value;
            }
            for (uint64_t j = 0; j < TEST_SIZE_OUTPUT_U64; j++) {
                data[i * TEST_SIZE_U64 + j + TEST_SIZE_INPUT_U64] = 0;
            }
        }
        struct timeval startTime;
        gettimeofday(&startTime, NULL);
        for (uint64_t i = 0; i<N_TESTS; i++)
        {
            uint64_t *test_data = data + i * TEST_SIZE_U64;
            int result = BLS12_381ComplexMulP(test_data, test_data + 12, test_data + 24);
        }
        uint64_t duration = TimeDiff(startTime);
        double tp = duration == 0 ? 0 : double(N_TESTS)/duration;
        print_results("BLS12_381ComplexMulP", duration, tp);
    }
    catch (const std::exception & e) {
        printf("BLS12_381ComplexMulP        |Exception: %s\n", e.what());
    }            
}

void Add256_benchmark(uint64_t *data) {

    try {
        const uint64_t TEST_SIZE_INPUT_U64 = 2 * 4;
        const uint64_t TEST_SIZE_OUTPUT_U64 = 1 * 4;
        const uint64_t TEST_SIZE_U64 = TEST_SIZE_INPUT_U64 + 1 + TEST_SIZE_OUTPUT_U64;
        for (uint64_t i = 0; i<N_TESTS; i++)
        {
            for (uint64_t j = 0; j < TEST_SIZE_INPUT_U64; j++) {
                data[i * TEST_SIZE_U64 + j] = (uint64_t)rand();
            }
            data[i * TEST_SIZE_U64 + TEST_SIZE_INPUT_U64] = (uint64_t)rand() % 2;
            for (uint64_t j = 0; j < TEST_SIZE_OUTPUT_U64; j++) {
                data[i * TEST_SIZE_U64 + j + TEST_SIZE_INPUT_U64 + 1] = 0;
            }
        }

        struct timeval startTime;
        gettimeofday(&startTime, NULL);
        for (uint64_t i = 0; i<N_TESTS; i++)
        {
            uint64_t *test_data = data + i * TEST_SIZE_U64;
            Add256(test_data, test_data + 4, test_data[8], test_data + 9);
        }

        uint64_t duration = TimeDiff(startTime);
        double tp = duration == 0 ? 0 : double(N_TESTS)/duration;
        print_results("Add256", duration, tp);
    }
    catch (const std::exception & e) {
        printf("Add256                      |Exception: %s\n", e.what());
    }
}


void BN254FpInv_test()
{
    {
        uint64_t x[4] = {1, 0, 0, 0};
        uint64_t expected_result[4] = {1, 0, 0, 0};
        uint64_t result[4] = {0, 0, 0, 0};
        int iresult = BN254FpInv (x, result);
        if ( (result[0] != expected_result[0]) ||
             (result[1] != expected_result[1]) ||
             (result[2] != expected_result[2]) ||
             (result[3] != expected_result[3]) )
        {
            printf("ERROR! BN254FpInv(1) returned unexpected result\n");
            printf("result =\n[%lu, %lu, %lu, %lu] = [0x%lx, 0x%lx, 0x%lx, 0x%lx]\n",
                result[0], result[1], result[2], result[3],
                result[0], result[1], result[2], result[3]);
            printf("expected_result =\n[%lu, %lu, %lu, %lu] = [0x%lx, 0x%lx, 0x%lx, 0x%lx]\n",
                expected_result[0], expected_result[1], expected_result[2], expected_result[3],
                expected_result[0], expected_result[1], expected_result[2], expected_result[3]);
        }
        else if (verbose)
        {
            printf("BN254FpInv(1) succeeded\n");
        }
    }

    {
        uint64_t x[4] = {0xf9ee4256a589409f, 0xa21a3985f17502d0, 0xb3eb393d00dc480c, 0x142def02c537eced};
        uint64_t expected_result[4] = {0x7258dab6e90d1680, 0x779f7ec5cad25c1d, 0xb9c114d250bcaa3c, 0x2525db1f6832d97d};
        uint64_t result[4] = {0, 0, 0, 0};
        int iresult = BN254FpInv (x, result);
        if ( (result[0] != expected_result[0]) ||
             (result[1] != expected_result[1]) ||
             (result[2] != expected_result[2]) ||
             (result[3] != expected_result[3]) )
        {
            printf("ERROR! BN254FpInv() returned unexpected result\n");
            printf("input =\n[%lu, %lu, %lu, %lu] = [0x%lx, 0x%lx, 0x%lx, 0x%lx]\n",
                x[0], x[1], x[2], x[3],
                x[0], x[1], x[2], x[3]);
            printf("result =\n[%lu, %lu, %lu, %lu] = [0x%lx, 0x%lx, 0x%lx, 0x%lx]\n",
                result[0], result[1], result[2], result[3],
                result[0], result[1], result[2], result[3]);
            printf("expected_result =\n[%lu, %lu, %lu, %lu] = [0x%lx, 0x%lx, 0x%lx, 0x%lx]\n",
                expected_result[0], expected_result[1], expected_result[2], expected_result[3],
                expected_result[0], expected_result[1], expected_result[2], expected_result[3]);
        }
        else if (verbose)
        {
            printf("BN254FpInv() succeeded\n");
        }
    }
}

void BN254ComplexInv_test()
{
    {
        uint64_t x[8] = {1, 0, 0, 0, 0, 0, 0, 0};
        uint64_t expected_result[8] = {1, 0, 0, 0, 0, 0, 0, 0};
        uint64_t result[8] = {0, 0, 0, 0, 0, 0, 0, 0};
        int iresult = BN254ComplexInv (x, result);
        if ( (result[0] != expected_result[0]) ||
             (result[1] != expected_result[1]) ||
             (result[2] != expected_result[2]) ||
             (result[3] != expected_result[3]) ||
             (result[4] != expected_result[4]) ||
             (result[5] != expected_result[5]) ||
             (result[6] != expected_result[6]) ||
             (result[7] != expected_result[7]) )
        {
            printf("ERROR! BN254ComplexInv(1) returned unexpected result\n");
            printf("result =\n[%lu, %lu, %lu, %lu, %lu, %lu, %lu, %lu] = [0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx]\n",
                result[0], result[1], result[2], result[3], result[4], result[5], result[6], result[7],
                result[0], result[1], result[2], result[3], result[4], result[5], result[6], result[7]);
            printf("expected_result =\n[%lu, %lu, %lu, %lu, %lu, %lu, %lu, %lu] = [0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx]\n",
                expected_result[0], expected_result[1], expected_result[2], expected_result[3], expected_result[4], expected_result[5], expected_result[6], expected_result[7],
                expected_result[0], expected_result[1], expected_result[2], expected_result[3], expected_result[4], expected_result[5], expected_result[6], expected_result[7]);
        }
        else if (verbose)
        {
            printf("BN254ComplexInv(1) succeeded\n");
        }
    }
    {
        uint64_t x[8] = {
            0xa4528921da9661b8,
            0xc13514a2f09d4f06,
            0x52406705a0d612b8,
            0x2b02b26b72efef38,
            0xb64cd3ecb5b08b28,
            0xe29c6143da89de45,
            0xdfa4f8b46115f7f6,
            0x17abb41fc8d1b2c7
        };
        uint64_t expected_result[8] = {
            0x163d11f5aa617bfc,
            0x825bc78934e518e5,
            0x31485988143cff2e,
            0x0551d3643b94a0ba,
            0xbd2738b4b0c67843,
            0xbed5ac50b31d3cef,
            0x516d2e7c293eef52,
            0x302d79e76ed154c1
        };
        uint64_t result[8] = {0, 0, 0, 0, 0, 0, 0, 0};
        int iresult = BN254ComplexInv (x, result);
        if ( (result[0] != expected_result[0]) ||
             (result[1] != expected_result[1]) ||
             (result[2] != expected_result[2]) ||
             (result[3] != expected_result[3]) ||
             (result[4] != expected_result[4]) ||
             (result[5] != expected_result[5]) ||
             (result[6] != expected_result[6]) ||
             (result[7] != expected_result[7]) )
        {
            printf("ERROR! BN254ComplexInv() returned unexpected result\n");
            printf("result =\n[%lu, %lu, %lu, %lu, %lu, %lu, %lu, %lu] = [0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx]\n",
                result[0], result[1], result[2], result[3], result[4], result[5], result[6], result[7],
                result[0], result[1], result[2], result[3], result[4], result[5], result[6], result[7]);
            printf("expected_result =\n[%lu, %lu, %lu, %lu, %lu, %lu, %lu, %lu] = [0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx]\n",
                expected_result[0], expected_result[1], expected_result[2], expected_result[3], expected_result[4], expected_result[5], expected_result[6], expected_result[7],
                expected_result[0], expected_result[1], expected_result[2], expected_result[3], expected_result[4], expected_result[5], expected_result[6], expected_result[7]);
        }
        else if (verbose)
        {
            printf("BN254ComplexInv() succeeded\n");
        }
    }
}

void BN254TwistAddLineCoeffs_test()
{
    {
        uint64_t input[32] = {
            // p
            0x66f0731159b54cd6,
            0xb630013739a5a053,
            0x31045e15f3a54bc2,
            0x214275f5c7d57155,
            0xfaf80b929d13179a,
            0xf63689aef8ecc6ff,
            0x26ffe67c5b2f3a49,
            0x04d4ad74230d1e83,
            0x46246b07a2ce41fd,
            0x65cd5922607deeee,
            0xe4ae145fac34c502,
            0x1e977a2280041e87,
            0x20ca11200df6b3c4,
            0x00bed9e88dfb7f8d,
            0x735adb5c7981edda,
            0x226adef094e4c626,
            
            // q
            0x4b70ada95bc43412,
            0x38f6cc990d30c020,
            0xca7d1f2becd3258a,
            0x2f9041da70888180,
            0x8d940679d41b2409,
            0xb28d0f4c5ea7672c,
            0xaa05b19dfad3217a,
            0x04ff3ef00c3f7d32,
            0x0cf3024d5172b33a,
            0xb3f5b354255ea1ee,
            0x70f37619880ce080,
            0x0e35dfd0b8edaa9c,
            0xf0e610b9d6ba7228,
            0x8d4202db12ceed20,
            0xdab0c37f22e05f42,
            0x172945c562cea2c7
        };
        uint64_t expected_result[16] = {
            // lambda
            0x70a3dd9659d4661d,
            0x272dad27777b65c9,
            0x0d3ed5d3d8417100,
            0x28b3fb64bf5e0593,
            0x84591f2f3fcbbf52,
            0x14fd5d4745900016,
            0xf620661dd1c5db97,
            0x0352e891aa056e3a,

            // mu
            0x7ae5d34cb3796d62,
            0x72e9885302380fda,
            0x90ba3e6a5edbad26,
            0x0da370e47b9854d6,
            0x337d5300e9a1f793,
            0x8e74c5f9836fb364,
            0x9207b0b313b312b5,
            0x263c38b6fef528c5
        };
        uint64_t result[16] = {0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0};

        BN254TwistAddLineCoeffs(input, result);

        bool failed = false;
        for (uint64_t i=0; i<16; i++)
        {
            if (result[i] != expected_result[i])
            {
                printf("ERROR! BN254TwistAddLineCoeffs() returned unexpected result\n");
                printf("result =\n[0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx]\n",
                    result[0], result[1], result[2], result[3], result[4], result[5], result[6], result[7],
                    result[8], result[9], result[10], result[11], result[12], result[13], result[14], result[15]);
                printf("expected_result =\n[0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx]\n",
                    expected_result[0], expected_result[1], expected_result[2], expected_result[3], expected_result[4], expected_result[5], expected_result[6], expected_result[7],
                    expected_result[8], expected_result[9], expected_result[10], expected_result[11], expected_result[12], expected_result[13], expected_result[14], expected_result[15]);
                failed = true;
                break;
            }
        }
        if ((!failed) && verbose) printf("BN254TwistAddLineCoeffs() succeeded\n");
    }
}

void BN254TwistDblLineCoeffs_test()
{   
    {
        uint64_t input[16] = {
            // p
            0x66f0731159b54cd6,
            0xb630013739a5a053,
            0x31045e15f3a54bc2,
            0x214275f5c7d57155,
            0xfaf80b929d13179a,
            0xf63689aef8ecc6ff,
            0x26ffe67c5b2f3a49,
            0x04d4ad74230d1e83,
            0x46246b07a2ce41fd,
            0x65cd5922607deeee,
            0xe4ae145fac34c502,
            0x1e977a2280041e87,
            0x20ca11200df6b3c4,
            0x00bed9e88dfb7f8d,
            0x735adb5c7981edda,
            0x226adef094e4c626
        };
        uint64_t expected_result[16] = {
            // lambda
            0xfa23df0596bf5ac0,
            0x1d60eabc30697e27,
            0xde847f8d09ff3261,
            0x0d2b35469ba57c1a,
            0x0e441461c8b02f6c,
            0x43ea3964b1f2af60,
            0x371d248d3d09e45f,
            0x260ac06e4d6faf7d,

            // mu
            0xf1e9e54da61ae409,
            0x1473486505dd6aeb,
            0xb6cb8f0ad3d51eea,
            0x2e6234f03865fd67,
            0xab884e4411c6b07b,
            0x0fafd74a66389f1c,
            0x9b91b3503e4834d0,
            0x0bb2e0552b697667,
        };
        uint64_t result[16] = {0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0};

        BN254TwistDblLineCoeffs(input, result);

        bool failed = false;
        for (uint64_t i=0; i<16; i++)
        {
            if (result[i] != expected_result[i])
            {
                printf("ERROR! BN254TwistDblLineCoeffs() returned unexpected result\n");
                printf("result =\n[0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx]\n",
                    result[0], result[1], result[2], result[3], result[4], result[5], result[6], result[7],
                    result[8], result[9], result[10], result[11], result[12], result[13], result[14], result[15]);
                printf("expected_result =\n[0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx, 0x%lx]\n",
                    expected_result[0], expected_result[1], expected_result[2], expected_result[3], expected_result[4], expected_result[5], expected_result[6], expected_result[7],
                    expected_result[8], expected_result[9], expected_result[10], expected_result[11], expected_result[12], expected_result[13], expected_result[14], expected_result[15]);
                failed = true;
                break;
            }
        }
        if ((!failed) && verbose) printf("BN254TwistDblLineCoeffs() succeeded\n");
    }
}

int main(int argc, char *argv[])
{
    if (argc > 1)
    {
        if (strcmp(argv[1], "-v") == 0)
        {
            verbose = true;
        } 
    }

    setlocale(LC_NUMERIC, "en_US.UTF-8");  // usa locale del sistema
    // o específico:    
    uint64_t *data = (uint64_t *)malloc(N_TESTS * MAX_TEST_SIZE_U64 * sizeof(uint64_t));
    printf("Test                        |duration   (us)|average    (ns)|TP (Mcalls/sec)\n");
    printf("----------------------------|---------------|---------------|---------------\n");

    secp256k1_add_benchmark(data);
    secp256k1_dbl_benchmark(data);
    // secp256k1_add_fe_benchmark(data);
    // secp256k1_dbl_fe_benchmark(data);
    InverseFpEc_benchmark(data);
    InverseFnEc_benchmark(data);
    SqrtFpEcParity_benchmark(data);
    BN254CurveAddP_benchmark(data);
    BN254CurveDblP_benchmark(data);
    BN254FpInv_benchmark(data);
    BN254ComplexAddP_benchmark(data);
    BN254ComplexSubP_benchmark(data);
    BN254ComplexMulP_benchmark(data);
    BN254ComplexInv_benchmark(data);
    BN254TwistAddLineCoeffs_benchmark(data);
    BN254TwistDblLineCoeffs_benchmark(data);
    Arith256_benchmark(data);
    FastArith256_benchmark(data);
    Arith256Mod_benchmark(data);
    Arith384_benchmark(data);
    Arith384Mod_benchmark(data);
    BLS12_381CurveAddP_benchmark(data);
    BLS12_381CurveDblP_benchmark(data);
    BLS12_381ComplexAddP_benchmark(data);
    BLS12_381ComplexSubP_benchmark(data);
    BLS12_381ComplexMulP_benchmark(data);
    Add256_benchmark(data);

    BN254FpInv_test();
    BN254ComplexInv_test();
    BN254TwistAddLineCoeffs_test();
    BN254TwistDblLineCoeffs_test();

    free(data);
}
