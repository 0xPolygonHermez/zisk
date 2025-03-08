#ifndef EC_HPP
#define EC_HPP

#include <cstdint>
#include "../ffiasm/fec.hpp"

extern RawFec fec;

int AddPointEc (uint64_t _dbl, const uint64_t * _x1, const uint64_t * _y1, const uint64_t * _x2, const uint64_t * _y2, uint64_t * _x3, uint64_t * _y3);

#endif
