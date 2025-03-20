#include <stdio.h>
#include <stdlib.h>
#include <cstdint>
#include "ec/ec.hpp"

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
        printf("Called AddPointEc() result=%d x1=%llx:%llx:%llx:%llx y1=%llx:%llx:%llx:%llx x2=%llx:%llx:%llx:%llx y2=%llx:%llx:%llx:%llx x3=%llx:%llx:%llx:%llx y3=%llx:%llx:%llx:%llx\n",
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
        printf("Called AddPointEcP() result=%d x1=%llx:%llx:%llx:%llx y1=%llx:%llx:%llx:%llx x2=%llx:%llx:%llx:%llx y2=%llx:%llx:%llx:%llx x3=%llx:%llx:%llx:%llx y3=%llx:%llx:%llx:%llx\n",
            result,
            p1[3], p1[2], p1[1], p1[0],
            p1[7], p1[6], p1[5], p1[4],
            p2[3], p2[2], p2[1], p2[0],
            p2[7], p2[6], p2[5], p2[4],
            p3[3], p3[2], p3[1], p3[0],
            p3[7], p3[6], p3[5], p3[4]
        );
    }

    return 0;
}