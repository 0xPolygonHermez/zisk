//#include <stdio.h>
//#include <sys/time.h>
//#include <stdlib.h>
//#include <cstdint>
//#include <cstring>
#include "float/float.h"

// uint64_t TimeDiff(const struct timeval &startTime, const struct timeval &endTime)
// {
//     struct timeval diff;

//     // Calculate the time difference
//     diff.tv_sec = endTime.tv_sec - startTime.tv_sec;
//     if (endTime.tv_usec >= startTime.tv_usec)
//     {
//         diff.tv_usec = endTime.tv_usec - startTime.tv_usec;
//     }
//     else if (diff.tv_sec > 0)
//     {
//         diff.tv_usec = 1000000 + endTime.tv_usec - startTime.tv_usec;
//         diff.tv_sec--;
//     }
//     else
//     {
//         // gettimeofday() can go backwards under some circumstances: NTP, multithread...
//         return 0;
//     }

//     // Return the total number of us
//     return diff.tv_usec + 1000000 * diff.tv_sec;
// }

// uint64_t TimeDiff(const struct timeval &startTime)
// {
//     struct timeval endTime;
//     gettimeofday(&endTime, NULL);
//     return TimeDiff(startTime, endTime);
// }

//bool verbose = false;


int _zisk_main(int argc, char *argv[])
{
    // if (argc > 1)
    // {
    //     if (strcmp(argv[1], "-v") == 0)
    //     {
    //         verbose = true;
    //     } 
    // }

    /*********/
    /* float */
    /*********/

    //printf("float:\n");
    // uint64_t fregs[100];
    // *(double *)&fregs[1] = 1.7;
    //fregs[1] = (uint64_t)1.7;
    //*(double *)&fregs[2] = 2.4;
    //fregs[2] = (uint64_t)2.4;
    zisk_float();
    // printf("  f1 = %lf\n", *(double *)&fregs[1]);
    // printf("  f2 = %lf\n", *(double *)&fregs[2]);
    // printf("  f3 = %lf\n", *(double *)&fregs[3]);

    // To compile with rv64imacfd = rv64g
    // double a = 1.7;
    // double b = 2.4;
    // double c = a + b; // calls fadd.d
    // uint64_t d = *(uint64_t *)&c;
    // printf("d=%lx\n", d);

    return 0;
}