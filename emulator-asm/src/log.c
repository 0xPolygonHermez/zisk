#include <stdarg.h>
#include <stdio.h>
#include <time.h>
#include "log.hpp"
#include "globals.hpp"

void asm_printf(const char *format, ...)
{
    // Flush any previous error message
    fflush(stderr);

    // Determine the stream to use
    FILE *stream = stdio ? stderr : stdout;

    // Get current date and time
    struct timeval tv;
    gettimeofday(&tv, NULL);
    struct tm *tm_info = localtime(&tv.tv_sec);
    time_t now = time(NULL);
    
    // Custom format: YYYY-MM-DD HH:MM:SS
    char date_and_time[80];
    if (now == (time_t)-1)
    {
        // Fallback if time() fails
        snprintf(date_and_time, sizeof(date_and_time), "0000-00-00 00:00:00");
    }
    else
    {
        if (tm_info == NULL || strftime(date_and_time, sizeof(date_and_time), "%Y-%m-%d %H:%M:%S", tm_info) == 0)
        {
            // Fallback if localtime() fails or strftime() cannot format
            snprintf(date_and_time, sizeof(date_and_time), "0000-00-00 00:00:00");
        }
    }

    // Print the prefix first
    fprintf(stream, "[ASM %s %s.%06ld] ", log_name, date_and_time, tv.tv_usec);
    
    // Handle the variable arguments
    va_list args;
    va_start(args, format);
    vfprintf(stream, format, args);
    va_end(args);

    // Flush the output to ensure this message is printed immediately, in case we are exiting right
    // after this call
    fflush(stream);
}

void asm_raw_printf(const char *format, ...)
{
    // Determine the stream to use
    FILE *stream = stdio ? stderr : stdout;

    // Handle the variable arguments
    va_list args;
    va_start(args, format);
    vfprintf(stream, format, args);
    va_end(args);

    // Flush the output to ensure this message is printed immediately, in case we are exiting right
    // after this call
    fflush(stream);
}
