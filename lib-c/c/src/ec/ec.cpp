
#include <gmpxx.h>
#include "ec.hpp"
#include "../ffiasm/fec.hpp"
#include "../ffiasm/fnec.hpp"
#include "../common/utils.hpp"
#include "../common/globals.hpp"
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

int inline AddPointEcFe (bool dbl, const RawFec::Element &x1, const RawFec::Element &y1, const RawFec::Element &x2, const RawFec::Element &y2, RawFec::Element &x3, RawFec::Element &y3)
{
    // Check if results are buffered
#ifdef ENABLE_EXPERIMENTAL_CODE
    if(ctx.ecRecoverPrecalcBuffer.filled == true){
        if(ctx.ecRecoverPrecalcBuffer.pos < 2){
            zklog.error("ecRecoverPrecalcBuffer.buffer buffer is not filled, but pos < 2 (pos=" + to_string(ctx.ecRecoverPrecalcBuffer.pos) + ")");
            exitProcess();
        }
        x3 = ctx.ecRecoverPrecalcBuffer.buffer[ctx.ecRecoverPrecalcBuffer.pos-2];
        y3 = ctx.ecRecoverPrecalcBuffer.buffer[ctx.ecRecoverPrecalcBuffer.pos-1];
        return ZKR_SUCCESS;
    }
#endif

    RawFec::Element aux1, aux2, s;

    if (dbl)
    {
        // s = 3*x1*x1/2*y1
        fec.mul(aux1, x1, x1);
        fec.fromUI(aux2, 3);
        fec.mul(aux1, aux1, aux2);
        fec.add(aux2, y1, y1);
        if (fec.isZero(aux2))
        {
            printf("AddPointEc() got denominator=0 1\n");
            return -1;
        }
        fec.div(s, aux1, aux2);

        // Required for x3 calculation
        fec.add(aux2, x1, x1);
    }
    else
    {
        // s = (y2-y1)/(x2-x1)
        fec.sub(aux1, y2, y1);
        fec.sub(aux2, x2, x1);
        if (fec.isZero(aux2))
        {
            printf("AddPointEc() got denominator=0 2\n");
            return -1;
        }
        fec.div(s, aux1, aux2);

        // Required for x3 calculation
        fec.add(aux2, x1, x2);
    }

    // x3 = s*s - (x1+x2)
    fec.mul(aux1, s, s);
    // aux2 was calculated before
    fec.sub(x3, aux1, aux2);

    // y3 = s*(x1-x3) - y1
    fec.sub(aux1, x1, x3);;
    fec.mul(aux1, aux1, s);
    fec.sub(y3, aux1, y1);

    return 0;
}

int inline AddPointEcDblFe (RawFec::Element &x1, RawFec::Element &y1)
{
    // Check if results are buffered
#ifdef ENABLE_EXPERIMENTAL_CODE
    if(ctx.ecRecoverPrecalcBuffer.filled == true){
        if(ctx.ecRecoverPrecalcBuffer.pos < 2){
            zklog.error("ecRecoverPrecalcBuffer.buffer buffer is not filled, but pos < 2 (pos=" + to_string(ctx.ecRecoverPrecalcBuffer.pos) + ")");
            exitProcess();
        }
        x3 = ctx.ecRecoverPrecalcBuffer.buffer[ctx.ecRecoverPrecalcBuffer.pos-2];
        y3 = ctx.ecRecoverPrecalcBuffer.buffer[ctx.ecRecoverPrecalcBuffer.pos-1];
        return ZKR_SUCCESS;
    }
#endif

    RawFec::Element aux1, aux2, aux3, s;

    // s = 3*x1*x1/2*y1
    fec.mul(aux1, x1, x1);
    fec.fromUI(aux2, 3);
    fec.mul(aux1, aux1, aux2);
    fec.add(aux2, y1, y1);
    if (fec.isZero(aux2))
    {
        printf("AddPointEc() got denominator=0 1\n");
        return -1;
    }
    fec.div(s, aux1, aux2);

    // Required for x3 calculation
    fec.add(aux2, x1, x1);

    // x3 = s*s - (x1+x2)
    fec.mul(aux1, s, s);
    // aux2 was calculated before
    
    fec.sub(aux3, aux1, aux2);

    // y3 = s*(x1-x3) - y1
    fec.sub(aux1, x1, aux3);
    x1 = aux3;
    fec.mul(aux1, aux1, s);
    fec.sub(y1, aux1, y1);

    return 0;
}

int AddPointEc (uint64_t _dbl, const uint64_t * _x1, const uint64_t * _y1, const uint64_t * _x2, const uint64_t * _y2, uint64_t * _x3, uint64_t * _y3)
{
    bool dbl = _dbl;

    RawFec::Element x1, y1, x2, y2, x3, y3;
    array2fe(_x1, x1);
    array2fe(_y1, y1);
    if (!dbl)
    {
        array2fe(_x2, x2);
        array2fe(_y2, y2);
    }

    int result = AddPointEcFe (dbl, x1, y1, x2, y2, x3, y3);
    
    fe2array(x3, _x3);
    fe2array(y3, _y3);

    return result;
}

int AddPointEcDbl (uint64_t * _x1, uint64_t * _y1)
{
    RawFec::Element x1, y1;
    array2fe(_x1, x1);
    array2fe(_y1, y1);

    int result = AddPointEcDblFe (x1, y1);
    
    fe2array(x1, _x1);
    fe2array(y1, _y1);

    return result;
}

int AddPointEcP (uint64_t _dbl, const uint64_t * p1, const uint64_t * p2, uint64_t * p3)
{
    bool dbl = _dbl;

    RawFec::Element x1, y1, x2, y2, x3, y3;
    array2fe(p1, x1);
    array2fe(p1 + 4, y1);
    if (!dbl)
    {
        array2fe(p2, x2);
        array2fe(p2 + 4, y2);
    }

    // printf("AddPointEcP() x1=%s\n", fec.toString(x1, 16).c_str());
    // printf("AddPointEcP() y1=%s\n", fec.toString(y1, 16).c_str());
    // printf("AddPointEcP() x2=%s\n", fec.toString(x2, 16).c_str());
    // printf("AddPointEcP() y2=%s\n", fec.toString(y2, 16).c_str());

    int result = AddPointEcFe (dbl, x1, y1, x2, y2, x3, y3);

    fe2array(x3, p3);
    fe2array(y3, p3 + 4);

    return result;
}

uint64_t G[8] = {
    0x59F2815B16F81798,
    0x029BFCDB2DCE28D9,
    0x55A06295CE870B07,
    0x79BE667EF9DCBBAC,
    0x9C47D08FFB10D4B8,
    0xFD17B448A6855419,
    0x5DA4FBFC0E1108A8,
    0x483ADA7726A3C465,
};

int secp256k1_ecdsa_verify (
    const uint64_t * pk,     // 8 x 64 bits
    const uint64_t * _z,      // 4 x 64 bits
    const uint64_t * _r,      // 4 x 64 bits
    const uint64_t * _s,      // 4 x 64 bits
          uint64_t * result  // 8 x 64 bits
)
{
    // Convert z, r, s inputs to field elements
    RawFec::Element z, r, s;
    array2fe(_z, z);
    array2fe(_r, r);
    array2fe(_s, s);

    // Given the public key pk and the signature (r, s) over the message hash z:
    // 1. Computes s_inv = s⁻¹ mod n
    // 2. Computes u1 = z·s_inv mod n
    // 3. Computes u2 = r·s_inv mod n
    // 4. Computes and returns the curve point p = u1·G + u2·PK
    
    // s_inv = s⁻¹ mod n
    RawFec::Element s_inv;
    fec.inv(s_inv, s);

    // u1 = z·s_inv mod n
    RawFec::Element u1;
    fec.mul(u1, z, s_inv);

    // u2 = r·s_inv mod n
    RawFec::Element u2;
    fec.mul(u2, r, s_inv);

    uint64_t u1_array[4];
    uint64_t u2_array[4];
    fe2array(u1, u1_array);
    fe2array(u2, u2_array);

    secp256k1_curve_dbl_scalar_mul(u1_array, G, u2_array, pk, result);

    return 0;
}

int secp256k1_curve_dbl_scalar_mul(
    const uint64_t * k1, // 4 x 64 bits
    const uint64_t * p1, // 8 x 64 bits
    const uint64_t * k2, // 4 x 64 bits
    const uint64_t * p2, // 8 x 64 bits
    uint64_t * r // 8 x 64 bits
)
{
    for (uint64_t i = 0; i < 8; i++) {
        r[i] = 0;
    }

    for (int64_t ii=255; ii>=0; ii--) {
        uint64_t i = ii;

        // r = r + r
        AddPointEcDbl(r, r);

        // If k1[i] == 1 then r = r + p1
        uint64_t k1_bit = (k1[i / 64] >> (i % 64)) & 1;
        if (k1_bit == 1) {
            AddPointEcP(0, r, p1, r);
        }

        // If k2[i] == 1 then r = r + p2
        uint64_t k2_bit = (k2[i / 64] >> (i % 64)) & 1;
        if (k2_bit == 1) {
            AddPointEcP(0, r, p2, r);
        }
    }

    return 0;
}

#ifdef __cplusplus
} // extern "C"
#endif