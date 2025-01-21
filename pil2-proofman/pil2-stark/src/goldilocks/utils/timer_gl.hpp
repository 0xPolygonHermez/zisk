#ifndef TIMER_HPP
#define TIMER_HPP

#include <cstdint>
#include <sys/time.h>
#include <string>

// Returns the time difference in us
uint64_t TimeDiff_GL(const struct timeval &startTime, const struct timeval &endTime);
uint64_t TimeDiff_GL(const struct timeval &startTime); // End time is now

#define LOG_TIME
#ifdef LOG_TIME
#define TimerStart(name) struct timeval name##_start; gettimeofday(&name##_start,NULL); std::cout << "--> " << std::string(#name) << " starting..." << std::endl
#define TimerStop(name) struct timeval name##_stop; gettimeofday(&name##_stop,NULL); std::cout << "<-- " << std::string(#name) << " done" << std::endl
#define TimerLog(name) std::cout << std::string(#name) << ": " << TimeDiff_GL(name##_start, name##_stop)/1000000.0f << " s" << std::endl
#define TimerStopAndLog(name) struct timeval name##_stop; gettimeofday(&name##_stop,NULL); std::cout << "<-- " + std::string(#name) << " done: " << TimeDiff_GL(name##_start, name##_stop)/1000000.0f << " s" << std::endl
#else
#define TimerStart(name)
#define TimerStop(name)
#define TimerLog(name)
#define TimerStopAndLog(name)
#endif

#endif  // TIMER_HPP
