#ifndef EC_HPP
#define EC_HPP

int AddPointEc (
    unsigned long _dbl,
    const unsigned long * _x1,  // 4 x 64 bits
    const unsigned long * _y1,  // 4 x 64 bits
    const unsigned long * _x2,  // 4 x 64 bits
    const unsigned long * _y2,  // 4 x 64 bits
    unsigned long * _x3,  // 4 x 64 bits
    unsigned long * _y3  // 4 x 64 bits
);

int AddPointEcP (
    unsigned long _dbl,
    const unsigned long * _p1,  // 8 x 64 bits
    const unsigned long * _p2,  // 8 x 64 bits
    unsigned long * _p3  // 8 x 64 bits
);

#endif
