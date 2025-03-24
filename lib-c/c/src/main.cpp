#include <stdio.h>
#include <sys/time.h>
#include <stdlib.h>
#include <cstdint>
#include "ec/ec.hpp"

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

int main(int argc, char *argv[])
{
    printf("clib main()\n");

    {
        uint64_t dbl = 0;
        uint64_t x1[4] = {(uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand()};
        uint64_t y1[4] = {(uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand()};
        uint64_t x2[4] = {(uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand()};
        uint64_t y2[4] = {(uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand()};
        uint64_t x3[4] = {0, 0, 0, 0};
        uint64_t y3[4] = {0, 0, 0, 0};

        int result = AddPointEc(dbl, x1, y1, x2, y2, x3, y3);
        printf("Called AddPointEc() result=%d x1=%lx:%lx:%lx:%lx y1=%lx:%lx:%lx:%lx x2=%lx:%lx:%lx:%lx y2=%lx:%lx:%lx:%lx x3=%lx:%lx:%lx:%lx y3=%lx:%lx:%lx:%lx\n",
            result,
            x1[3], x1[2], x1[1], x1[0],
            y1[3], y1[2], y1[1], y1[0],
            x2[3], x2[2], x2[1], x2[0],
            y2[3], y2[2], y2[1], y2[0],
            x3[3], x3[2], x3[1], x3[0],
            y3[3], y3[2], y3[1], y3[0]
        );
    }
    
    {
        uint64_t dbl = 0;
        uint64_t p1[8] = {(uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand()};
        uint64_t p2[8] = {(uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand(), (uint64_t)rand()};
        uint64_t p3[8] = {0, 0, 0, 0, 0, 0, 0, 0};

        int result = AddPointEcP(dbl, p1, p2, p3);
        printf("Called AddPointEcP() result=%d x1=%lx:%lx:%lx:%lx y1=%lx:%lx:%lx:%lx x2=%lx:%lx:%lx:%lx y2=%lx:%lx:%lx:%lx x3=%lx:%lx:%lx:%lx y3=%lx:%lx:%lx:%lx\n",
            result,
            p1[3], p1[2], p1[1], p1[0],
            p1[7], p1[6], p1[5], p1[4],
            p2[3], p2[2], p2[1], p2[0],
            p2[7], p2[6], p2[5], p2[4],
            p3[3], p3[2], p3[1], p3[0],
            p3[7], p3[6], p3[5], p3[4]
        );
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
        printf("AddPointEc(dbl=0) duration=%ld TP = %f Mcalls/sec\n", duration, tp);
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
        printf("AddPointEc(dbl=1) duration=%ld TP = %f Mcalls/sec\n", duration, tp);
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
        printf("InverseFpEc() duration=%ld TP = %f Mcalls/sec\n", duration, tp);
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
        printf("InverseFnEc() duration=%ld TP = %f Mcalls/sec\n", duration, tp);
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
        printf("SqrtFpEcParity() duration=%ld TP = %f Mcalls/sec\n", duration, tp);
    }

    return 0;
}