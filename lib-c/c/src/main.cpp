#include <stdio.h>
#include <sys/time.h>
#include <stdlib.h>
#include <cstdint>
#include <cstring>
#include "ec/ec.hpp"
#include "fcall/fcall.hpp"
#include "arith256/arith256.hpp"
#include "bn254/bn254.hpp"

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

int main(int argc, char *argv[])
{
    if (argc > 1)
    {
        if (strcmp(argv[1], "-v") == 0)
        {
            verbose = true;
        } 
    }

    /*******************/
    /* secp256k1 curve */
    /*******************/

    printf("clib secp256k1:\n");

    {
        uint64_t dbl = 0;
        uint64_t x1[4] = {(uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand()};
        uint64_t y1[4] = {(uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand()};
        uint64_t x2[4] = {(uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand()};
        uint64_t y2[4] = {(uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand()};
        uint64_t x3[4] = {0, 0, 0, 0};
        uint64_t y3[4] = {0, 0, 0, 0};

        int result = AddPointEc(dbl, x1, y1, x2, y2, x3, y3);
        if (verbose)
        {
            printf("Called AddPointEc() result=%d x1=%lu:%lu:%lu:%lu y1=%lu:%lu:%lu:%lu x2=%lu:%lu:%lu:%lu y2=%lu:%lu:%lu:%lu x3=%lu:%lu:%lu:%lu y3=%lu:%lu:%lu:%lu\n",
                result,
                x1[3], x1[2], x1[1], x1[0],
                y1[3], y1[2], y1[1], y1[0],
                x2[3], x2[2], x2[1], x2[0],
                y2[3], y2[2], y2[1], y2[0],
                x3[3], x3[2], x3[1], x3[0],
                y3[3], y3[2], y3[1], y3[0]
            );
        }
   }
    
    {
        uint64_t dbl = 0;
        uint64_t p1[8] = {(uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand()};
        uint64_t p2[8] = {(uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand()};
        uint64_t p3[8] = {0, 0, 0, 0, 0, 0, 0, 0};

        int result = AddPointEcP(dbl, p1, p2, p3);
        if (verbose)
        {
            printf("Called AddPointEcP() result=%d x1=%lu:%lu:%lu:%lu y1=%lu:%lu:%lu:%lu x2=%lu:%lu:%lu:%lu y2=%lu:%lu:%lu:%lu x3=%lu:%lu:%lu:%lu y3=%lu:%lu:%lu:%lu\n",
                result,
                p1[3], p1[2], p1[1], p1[0],
                p1[7], p1[6], p1[5], p1[4],
                p2[3], p2[2], p2[1], p2[0],
                p2[7], p2[6], p2[5], p2[4],
                p3[3], p3[2], p3[1], p3[0],
                p3[7], p3[6], p3[5], p3[4]
            );
        }
    }

    {
        uint64_t dbl = 0;
        uint64_t x1[4] = {(uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand()};
        uint64_t y1[4] = {(uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand()};
        uint64_t x2[4] = {(uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand()};
        uint64_t y2[4] = {(uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand()};
        uint64_t x3[4] = {0, 0, 0, 0};
        uint64_t y3[4] = {0, 0, 0, 0};

        struct timeval startTime;
        gettimeofday(&startTime, NULL);
        for (uint64_t i = 0; i<1000000; i++)
        {
            int result = AddPointEc(dbl, x1, y1, x2, y2, x3, y3);
        }
        uint64_t duration = TimeDiff(startTime);
        double tp = duration == 0 ? 0 : double(1000000)/duration;
        printf("AddPointEc(dbl=0) duration=%lu us, average=%lu ns, TP = %f Mcalls/sec\n", duration, duration/1000, tp);
    }

    {
        uint64_t dbl = 1;
        uint64_t x1[4] = {(uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand()};
        uint64_t y1[4] = {(uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand()};
        uint64_t x2[4] = {(uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand()};
        uint64_t y2[4] = {(uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand()};
        uint64_t x3[4] = {0, 0, 0, 0};
        uint64_t y3[4] = {0, 0, 0, 0};

        struct timeval startTime;
        gettimeofday(&startTime, NULL);
        for (uint64_t i = 0; i<1000000; i++)
        {
            int result = AddPointEc(dbl, x1, y1, x2, y2, x3, y3);
        }
        uint64_t duration = TimeDiff(startTime);
        double tp = duration == 0 ? 0 : double(1000000)/duration;
        printf("AddPointEc(dbl=1) duration=%lu us, average=%lu ns, TP = %f Mcalls/sec\n", duration, duration/1000, tp);
    }

    {
        uint64_t a[4] = {(uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand()};
        uint64_t r[4] = {0, 0, 0, 0};

        struct timeval startTime;
        gettimeofday(&startTime, NULL);
        for (uint64_t i = 0; i<1000000; i++)
        {
            int result = InverseFpEc(a, r);
        }
        uint64_t duration = TimeDiff(startTime);
        double tp = duration == 0 ? 0 : double(1000000)/duration;
        printf("InverseFpEc() duration=%lu us, average=%lu ns, TP = %f Mcalls/sec\n", duration, duration/1000, tp);

    }

    {
        uint64_t a[4] = {(uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand()};
        uint64_t r[4] = {0, 0, 0, 0};

        struct timeval startTime;
        gettimeofday(&startTime, NULL);
        for (uint64_t i = 0; i<1000000; i++)
        {
            int result = InverseFnEc(a, r);
        }
        uint64_t duration = TimeDiff(startTime);
        double tp = duration == 0 ? 0 : double(1000000)/duration;
        printf("InverseFnEc() duration=%lu us, average=%lu ns, TP = %f Mcalls/sec\n", duration, duration/1000, tp);
    }

    {
        uint64_t a[4] = {(uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand()};
        uint64_t r[4] = {0, 0, 0, 0};
        uint64_t parity = 0;

        struct timeval startTime;
        gettimeofday(&startTime, NULL);
        for (uint64_t i = 0; i<1000000; i++)
        {
            int result = SqrtFpEcParity(a, parity, r);
        }
        uint64_t duration = TimeDiff(startTime);
        double tp = duration == 0 ? 0 : double(1000000)/duration;
        printf("SqrtFpEcParity() duration=%lu us, average=%lu ns, TP = %f Mcalls/sec\n", duration, duration/1000, tp);
    }

    printf("clib BN254:\n");

    /*******************/
    /* BN254 curve add */
    /*******************/

    {
        uint64_t p1[8] = {(uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand()};
        uint64_t p2[8] = {(uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand()};
        uint64_t p3[8] = {0, 0, 0, 0, 0, 0, 0, 0};

        struct timeval startTime;
        gettimeofday(&startTime, NULL);
        for (uint64_t i = 0; i<1000000; i++)
        {
            int result = BN254CurveAddP(p1, p2, p3);
        }
        uint64_t duration = TimeDiff(startTime);
        double tp = duration == 0 ? 0 : double(1000000)/duration;
        printf("BN254CurveAddP() duration=%lu us, average=%lu ns, TP = %f Mcalls/sec\n", duration, duration/1000, tp);
    }

    /**********************/
    /* BN254 curve double */
    /**********************/

    {
        uint64_t p1[8] = {(uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand()};
        uint64_t p2[8] = {0, 0, 0, 0, 0, 0, 0, 0};

        struct timeval startTime;
        gettimeofday(&startTime, NULL);
        for (uint64_t i = 0; i<1000000; i++)
        {
            int result = BN254CurveDblP(p1, p2);
        }
        uint64_t duration = TimeDiff(startTime);
        double tp = duration == 0 ? 0 : double(1000000)/duration;
        printf("BN254CurveDblP() duration=%lu us, average=%lu ns, TP = %f Mcalls/sec\n", duration, duration/1000, tp);
    }

    /**************/
    /* BN254FpInv */
    /**************/

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

    {
        uint64_t a[4] = {(uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand()};
        uint64_t r[4] = {0, 0, 0, 0};

        struct timeval startTime;
        gettimeofday(&startTime, NULL);
        for (uint64_t i = 0; i<1000000; i++)
        {
            int result = BN254FpInv(a, r);
        }
        uint64_t duration = TimeDiff(startTime);
        double tp = duration == 0 ? 0 : double(1000000)/duration;
        printf("BN254FpInv() duration=%lu us, average=%lu ns, TP = %f Mcalls/sec\n", duration, duration/1000, tp);

    }

    /*********************/
    /* BN254 complex add */
    /*********************/

    {
        uint64_t p1[8] = {(uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand()};
        uint64_t p2[8] = {(uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand()};
        uint64_t p3[8] = {0, 0, 0, 0, 0, 0, 0, 0};

        struct timeval startTime;
        gettimeofday(&startTime, NULL);
        for (uint64_t i = 0; i<1000000; i++)
        {
            int result = BN254ComplexAddP(p1, p2, p3);
        }
        uint64_t duration = TimeDiff(startTime);
        double tp = duration == 0 ? 0 : double(1000000)/duration;
        printf("BN254ComplexAddP() duration=%lu us, average=%lu ns, TP = %f Mcalls/sec\n", duration, duration/1000, tp);
    }

    /*********************/
    /* BN254 complex sub */
    /*********************/

    {
        uint64_t p1[8] = {(uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand()};
        uint64_t p2[8] = {(uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand()};
        uint64_t p3[8] = {0, 0, 0, 0, 0, 0, 0, 0};

        struct timeval startTime;
        gettimeofday(&startTime, NULL);
        for (uint64_t i = 0; i<1000000; i++)
        {
            int result = BN254ComplexSubP(p1, p2, p3);
        }
        uint64_t duration = TimeDiff(startTime);
        double tp = duration == 0 ? 0 : double(1000000)/duration;
        printf("BN254ComplexSubP() duration=%lu us, average=%lu ns, TP = %f Mcalls/sec\n", duration, duration/1000, tp);
    }

    /*********************/
    /* BN254 complex mul */
    /*********************/

    {
        uint64_t p1[8] = {(uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand()};
        uint64_t p2[8] = {(uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand()};
        uint64_t p3[8] = {0, 0, 0, 0, 0, 0, 0, 0};

        struct timeval startTime;
        gettimeofday(&startTime, NULL);
        for (uint64_t i = 0; i<1000000; i++)
        {
            int result = BN254ComplexMulP(p1, p2, p3);
        }
        uint64_t duration = TimeDiff(startTime);
        double tp = duration == 0 ? 0 : double(1000000)/duration;
        printf("BN254ComplexMulP() duration=%lu us, average=%lu ns, TP = %f Mcalls/sec\n", duration, duration/1000, tp);
    }

    /*******************/
    /* BN254ComplexInv */
    /*******************/

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

    {
        uint64_t a[8] = {(uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand()};
        uint64_t r[8] = {0, 0, 0, 0, 0, 0, 0, 0};

        struct timeval startTime;
        gettimeofday(&startTime, NULL);
        for (uint64_t i = 0; i<1000000; i++)
        {
            int result = BN254ComplexInv(a, r);
        }
        uint64_t duration = TimeDiff(startTime);
        double tp = duration == 0 ? 0 : double(1000000)/duration;   
        printf("BN254ComplexInv() duration=%lu us, average=%lu ns, TP = %f Mcalls/sec\n", duration, duration/1000, tp);
    }

    /***************************/
    /* BN254TwistAddLineCoeffs */
    /***************************/

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

    {
        uint64_t a[32];
        for (uint64_t i=0; i<32; i++) { a[i] = (uint64_t)rand(); }
        uint64_t r[16];
        for (uint64_t i=0; i<16; i++) { r[i] = 0; }

        struct timeval startTime;
        gettimeofday(&startTime, NULL);
        for (uint64_t i = 0; i<1000000; i++)
        {
            int result = BN254TwistAddLineCoeffs(a, r);
        }
        uint64_t duration = TimeDiff(startTime);
        double tp = duration == 0 ? 0 : double(1000000)/duration;
        printf("BN254TwistAddLineCoeffs() duration=%lu us, average=%lu ns, TP = %f Mcalls/sec\n", duration, duration/1000, tp);
    }

    /***************************/
    /* BN254TwistDblLineCoeffs */
    /***************************/

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

    {
        uint64_t a[32];
        for (uint64_t i=0; i<32; i++) { a[i] = (uint64_t)rand(); }
        uint64_t r[16];
        for (uint64_t i=0; i<16; i++) { r[i] = 0; }

        struct timeval startTime;
        gettimeofday(&startTime, NULL);
        for (uint64_t i = 0; i<1000000; i++)
        {
            int result = BN254TwistDblLineCoeffs(a, r);
        }
        uint64_t duration = TimeDiff(startTime);
        double tp = duration == 0 ? 0 : double(1000000)/duration;
        printf("BN254TwistDblLineCoeffs() duration=%lu us, average=%lu ns, TP = %f Mcalls/sec\n", duration, duration/1000, tp);
    }

    /************/
    /* Arith256 */
    /************/

    printf("clib arith:\n");

    {

        uint64_t a[4];
        for (uint64_t i=0; i<4; i++) { a[i] = (uint64_t)rand(); }
        uint64_t b[4];
        for (uint64_t i=0; i<4; i++) { b[i] = (uint64_t)rand(); }
        uint64_t c[4];
        for (uint64_t i=0; i<4; i++) { c[i] = (uint64_t)rand(); }
        uint64_t dl[4];
        uint64_t dh[4];

        struct timeval startTime;
        gettimeofday(&startTime, NULL);
        for (uint64_t i = 0; i<1000000; i++)
        {
            int result = Arith256(a, b, c, dl, dh);
        }
        uint64_t duration = TimeDiff(startTime);
        double tp = duration == 0 ? 0 : double(1000000)/duration;
        printf("Arith256()duration=%lu us, average=%lu ns, TP = %f Mcalls/sec\n", duration, duration/1000, tp);
    }

    /*******************/
    /* Arith256 module */
    /*******************/

    {
        uint64_t a[4];
        for (uint64_t i=0; i<4; i++) { a[i] = (uint64_t)rand(); }
        uint64_t b[4];
        for (uint64_t i=0; i<4; i++) { b[i] = (uint64_t)rand(); }
        uint64_t c[4];
        for (uint64_t i=0; i<4; i++) { c[i] = (uint64_t)rand(); }
        uint64_t module[4];
        for (uint64_t i=0; i<4; i++) { module[i] = (uint64_t)rand(); }
        uint64_t d[4];

        struct timeval startTime;
        gettimeofday(&startTime, NULL);
        for (uint64_t i = 0; i<1000000; i++)
        {
            int result = Arith256Mod(a, b, c, module, d);
        }
        uint64_t duration = TimeDiff(startTime);
        double tp = duration == 0 ? 0 : double(1000000)/duration;
        printf("Arith256Mod() duration=%lu us, average=%lu ns, TP = %f Mcalls/sec\n", duration, duration/1000, tp);
    }
}