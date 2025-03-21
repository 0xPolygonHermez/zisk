#ifndef EC_HPP
#define EC_HPP

int AddPointEc (
    unsigned long dbl,
    const unsigned long * x1,  // 4 x 64 bits
    const unsigned long * y1,  // 4 x 64 bits
    const unsigned long * x2,  // 4 x 64 bits
    const unsigned long * y2,  // 4 x 64 bits
    unsigned long * x3,  // 4 x 64 bits
    unsigned long * y3  // 4 x 64 bits
);

int AddPointEcP (
    const unsigned long dbl,
    const unsigned long * p1,  // 8 x 64 bits
    const unsigned long * p2,  // 8 x 64 bits
    unsigned long * p3  // 8 x 64 bits
);

int InverseFpEc (
    const unsigned long * a,  // 8 x 64 bits
    unsigned long * r  // 8 x 64 bits
);

int InverseFnEc (
    const unsigned long * a,  // 8 x 64 bits
    unsigned long * r  // 8 x 64 bits
);

int SqrtFpEcParity (
    const unsigned long * a,  // 8 x 64 bits
    const unsigned long parity,
    unsigned long * r  // 8 x 64 bits
);

#endif
